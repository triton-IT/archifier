//! Géométrie de la carte : cadrage, zoom, déplacement, gestes.
//!
//! La carte garde un `viewBox` SVG stable et se déplace en modifiant ce
//! viewBox. Ce module ne touche ni au dépôt ni au DOM : il ne manipule que des
//! nombres, ce qui rend le zoom et le cadrage testables nativement.

use archifier_core::map::{MapLevel, MapProjection};
use archifier_core::ObjectId;

/// Fenêtre visible de la carte, exprimée dans les coordonnées de projection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapViewport {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl MapViewport {
    /// Largeur minimale : évite un zoom absurde sur un objet isolé.
    pub const MIN_W: f64 = 240.0;
    /// Largeur maximale : évite de perdre la carte dans un vide infini.
    pub const MAX_W: f64 = 16_000.0;

    /// Cadrage qui contient tous les quartiers et tous les objets, avec une
    /// marge lisible. C'est le « Ajuster » de la carte.
    pub fn content(projection: &MapProjection) -> Self {
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for district in &projection.districts {
            min_x = min_x.min(district.x);
            min_y = min_y.min(district.y);
            max_x = max_x.max(district.x + district.w);
            max_y = max_y.max(district.y + district.h);
        }
        for node in &projection.nodes {
            min_x = min_x.min(node.x);
            min_y = min_y.min(node.y);
            max_x = max_x.max(node.x + node.w);
            max_y = max_y.max(node.y + node.h);
        }

        if !min_x.is_finite()
            || !min_y.is_finite()
            || !max_x.is_finite()
            || !max_y.is_finite()
            || max_x <= min_x
            || max_y <= min_y
        {
            return Self {
                x: 0.0,
                y: 0.0,
                w: 900.0,
                h: 480.0,
            };
        }

        let pad = 48.0;
        Self {
            x: min_x - pad,
            y: min_y - pad,
            w: (max_x - min_x) + pad * 2.0,
            h: (max_y - min_y) + pad * 2.0,
        }
    }

    pub fn view_box(self) -> String {
        format!("{} {} {} {}", self.x, self.y, self.w, self.h)
    }

    /// Échelle écran → monde quand le SVG est rendu avec `xMidYMid meet`.
    pub fn scale_in(self, rect_w: f64, rect_h: f64) -> f64 {
        if self.w <= 0.0 || self.h <= 0.0 || rect_w <= 0.0 || rect_h <= 0.0 {
            return 1.0;
        }
        (rect_w / self.w).min(rect_h / self.h).max(0.0001)
    }

    /// Position relative (0..1) du curseur dans le contenu réellement rendu,
    /// letterboxing compris. Sert à zoomer sous le curseur.
    pub fn relative_in(
        self,
        rect_w: f64,
        rect_h: f64,
        client_x: f64,
        client_y: f64,
        left: f64,
        top: f64,
    ) -> (f64, f64) {
        let scale = self.scale_in(rect_w, rect_h);
        let rendered_w = (self.w * scale).max(0.0001);
        let rendered_h = (self.h * scale).max(0.0001);
        let offset_x = (rect_w - rendered_w) / 2.0;
        let offset_y = (rect_h - rendered_h) / 2.0;
        let rx = ((client_x - left - offset_x) / rendered_w).clamp(0.0, 1.0);
        let ry = ((client_y - top - offset_y) / rendered_h).clamp(0.0, 1.0);
        (rx, ry)
    }

    /// Position monde sous le curseur, letterboxing compris.
    pub fn world_at(
        self,
        rect_w: f64,
        rect_h: f64,
        client_x: f64,
        client_y: f64,
        left: f64,
        top: f64,
    ) -> (f64, f64) {
        let scale = self.scale_in(rect_w, rect_h);
        let rendered_w = (self.w * scale).max(0.0001);
        let rendered_h = (self.h * scale).max(0.0001);
        let offset_x = (rect_w - rendered_w) / 2.0;
        let offset_y = (rect_h - rendered_h) / 2.0;
        (
            self.x + (client_x - left - offset_x) / scale,
            self.y + (client_y - top - offset_y) / scale,
        )
    }

    /// Zoome en gardant le point relatif `(rel_x, rel_y)` immobile à l'écran.
    pub fn zoomed(self, factor: f64, rel_x: f64, rel_y: f64) -> Self {
        let factor = factor.clamp(0.2, 5.0);
        let w = (self.w / factor).clamp(Self::MIN_W, Self::MAX_W);
        let h = self.h * (w / self.w);
        Self {
            x: self.x + rel_x * (self.w - w),
            y: self.y + rel_y * (self.h - h),
            w,
            h,
        }
    }

    pub fn zoomed_centered(self, factor: f64) -> Self {
        self.zoomed(factor, 0.5, 0.5)
    }

    pub fn panned(self, dx: f64, dy: f64) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
            ..self
        }
    }

    pub fn centered_on(self, x: f64, y: f64) -> Self {
        Self {
            x: x - self.w / 2.0,
            y: y - self.h / 2.0,
            ..self
        }
    }

    /// Pourcentage affiché : 100 % correspond au cadrage complet.
    pub fn zoom_percent(self, content: Self) -> f64 {
        if self.w <= 0.0 {
            100.0
        } else {
            (content.w / self.w * 100.0).clamp(1.0, 999.0)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MapSelection {
    Node(String),
    Edge(String),
}

/// Source d'un geste « relier » : la prochaine carte cliquée devient la
/// consommatrice de l'interface.
#[derive(Clone, Debug, PartialEq)]
pub struct MapLinkSource {
    pub node_id: String,
    pub object_id: ObjectId,
    pub kind: archifier_core::map::MapObjectKind,
    pub level: MapLevel,
}

/// Déplacement d'un objet : l'objet suit le doigt, l'ancre n'est écrite qu'au
/// relâchement.
#[derive(Clone, Debug, PartialEq)]
pub struct MapNodeDrag {
    pub node_id: String,
    pub object_id: ObjectId,
    pub level: MapLevel,
    pub origin_x: f64,
    pub origin_y: f64,
    pub start_client_x: f64,
    pub start_client_y: f64,
    /// Pixels écran par unité de carte, figés au début du geste.
    pub scale: f64,
    pub x: f64,
    pub y: f64,
    pub moved: bool,
}

/// Déplacement du canevas. Aucune écriture métier : seule la fenêtre change.
#[derive(Clone, Debug, PartialEq)]
pub struct MapPan {
    pub start_client_x: f64,
    pub start_client_y: f64,
    pub origin_x: f64,
    pub origin_y: f64,
    pub scale: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MapGesture {
    Node(MapNodeDrag),
    Pan(MapPan),
}

#[cfg(test)]
mod tests {
    use super::MapViewport;

    fn viewport() -> MapViewport {
        MapViewport {
            x: 100.0,
            y: 50.0,
            w: 800.0,
            h: 400.0,
        }
    }

    #[test]
    fn zoom_keeps_the_relative_point_fixed() {
        let start = viewport();
        let rel_x = 0.25;
        let rel_y = 0.75;
        let zoomed = start.zoomed(2.0, rel_x, rel_y);

        assert_eq!(zoomed.w, 400.0);
        assert_eq!(zoomed.h, 200.0);
        assert!((zoomed.x - (100.0 + 0.25 * 400.0)).abs() < 1e-9);
        assert!((zoomed.y - (50.0 + 0.75 * 200.0)).abs() < 1e-9);
    }

    #[test]
    fn zoom_is_bounded() {
        let start = viewport();
        let huge = start.zoomed(1_000.0, 0.5, 0.5);
        let tiny = start.zoomed(0.000_1, 0.5, 0.5);
        assert!(huge.w >= MapViewport::MIN_W);
        assert!(tiny.w <= MapViewport::MAX_W);
        assert!(huge.h > 0.0);
        assert!(tiny.h > 0.0);
    }

    #[test]
    fn pan_moves_the_window_without_resizing_it() {
        let start = viewport();
        let moved = start.panned(-25.0, 40.0);
        assert_eq!(moved.w, start.w);
        assert_eq!(moved.h, start.h);
        assert_eq!(moved.x, 75.0);
        assert_eq!(moved.y, 90.0);
    }

    #[test]
    fn zoom_percent_is_relative_to_content() {
        let content = viewport();
        assert_eq!(content.zoom_percent(content), 100.0);
        assert_eq!(content.zoomed_centered(2.0).zoom_percent(content), 200.0);
        assert_eq!(content.zoomed_centered(0.5).zoom_percent(content), 50.0);
    }
}
