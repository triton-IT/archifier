//! Cadrage d'une Story : zoom, focus et atténuation (T1-10).
//!
//! Le helper est volontairement sans I/O ni dépendance UI : il transforme un
//! `DiagramLayout` et un `DiagramFraming` en nœuds prêts à rendre. Le layout
//! source n'est jamais modifié ; le zoom s'applique uniquement aux coordonnées
//! et dimensions produites.

use crate::ids::ObjectId;
use crate::model::{DiagramFraming, DiagramLayout};
use std::collections::HashSet;

/// Bornes de zoom imposées par la spécification.
pub const MIN_ZOOM: f32 = 0.5;
pub const MAX_ZOOM: f32 = 3.0;
pub const DEFAULT_ZOOM: f32 = 1.0;

/// Opacité d'un nœud explicitement atténué.
pub const DIMMED_OPACITY: f32 = 0.35;

/// Nœud de diagramme cadré pour un bloc Story.
#[derive(Debug, Clone, PartialEq)]
pub struct FramedNode {
    pub object_id: ObjectId,
    /// Vrai quand l'élément appartient à `focus_element_ids`.
    pub highlighted: bool,
    /// Opacité de rendu : `1.0` pour un nœud normal ou focalisé,
    /// [`DIMMED_OPACITY`] pour un nœud atténué.
    pub opacity: f32,
    pub node_id: String,
    /// Coordonnées et dimensions après application du zoom.
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    /// Zoom effectif, borné à [`MIN_ZOOM`, `MAX_ZOOM`].
    pub zoom: f32,
    pub dimmed: bool,
}

impl FramedNode {
    pub fn is_visible(&self) -> bool {
        self.opacity > 0.0
    }
}

/// Résultat complet d'un cadrage : échelle, nœuds et légende.
#[derive(Debug, Clone, PartialEq)]
pub struct FramedDiagram {
    pub zoom: f32,
    /// Nœuds dans l'ordre de rendu (focus en dernier).
    pub nodes: Vec<FramedNode>,
    pub caption: Option<String>,
}

/// Zoom effectif d'un cadrage : `1.0` par défaut, borné à `0.5..=3.0`.
pub fn effective_zoom(framing: &DiagramFraming) -> f32 {
    framing
        .zoom
        .filter(|zoom| zoom.is_finite())
        .unwrap_or(DEFAULT_ZOOM)
        .clamp(MIN_ZOOM, MAX_ZOOM)
}

/// Projette le layout dans le cadrage demandé.
///
/// - le zoom multiplie `x`, `y`, `w` et `h` ;
/// - les identifiants de `focus_element_ids` sont marqués `highlighted` et
///   rendus en dernier (tri stable), pour passer au-dessus des autres nœuds ;
/// - les identifiants de `dimmed_element_ids` reçoivent l'opacité atténuée ;
/// - un nœud à la fois focalisé et atténué reste focalisé.
pub fn frame_layout(layout: &DiagramLayout, framing: &DiagramFraming) -> Vec<FramedNode> {
    let zoom = effective_zoom(framing);
    let focus: HashSet<ObjectId> = framing.focus_element_ids.iter().copied().collect();
    let dimmed: HashSet<ObjectId> = framing.dimmed_element_ids.iter().copied().collect();
    let scale = f64::from(zoom);

    let mut nodes: Vec<FramedNode> = layout
        .nodes
        .iter()
        .map(|node| {
            let highlighted = focus.contains(&node.object_id);
            let is_dimmed = !highlighted && dimmed.contains(&node.object_id);
            FramedNode {
                object_id: node.object_id,
                highlighted,
                opacity: if is_dimmed { DIMMED_OPACITY } else { 1.0 },
                node_id: node.id.clone(),
                x: node.x * scale,
                y: node.y * scale,
                w: node.w * scale,
                h: node.h * scale,
                zoom,
                dimmed: is_dimmed,
            }
        })
        .collect();

    // Tri stable : l'ordre relatif du layout est conservé, seuls les nœuds
    // focalisés passent en fin de liste pour être dessinés au-dessus.
    nodes.sort_by_key(|node| u8::from(node.highlighted));
    nodes
}

/// Cadre un layout pour un bloc Story : zoom, focus, atténuation et légende.
pub fn apply_framing(framing: &DiagramFraming, layout: &DiagramLayout) -> FramedDiagram {
    FramedDiagram {
        zoom: effective_zoom(framing),
        nodes: frame_layout(layout, framing),
        caption: framing.caption.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DiagramNode, NodeKind};

    fn node(id: &str, object_id: ObjectId, x: f64, y: f64) -> DiagramNode {
        DiagramNode {
            id: id.into(),
            object_id,
            kind: NodeKind::Element,
            version_id: None,
            x,
            y,
            w: 100.0,
            h: 50.0,
            layer_id: None,
            zone_id: None,
            collapsed: false,
            port: None,
            style_override: Default::default(),
        }
    }

    fn layout() -> DiagramLayout {
        DiagramLayout {
            schema: 1,
            viewport: Default::default(),
            nodes: vec![
                node("n1", ObjectId::from_parts(1, 1), 10.0, 20.0),
                node("n2", ObjectId::from_parts(1, 2), 30.0, 40.0),
                node("n3", ObjectId::from_parts(1, 3), 50.0, 60.0),
            ],
            edges: vec![],
            annotations: vec![],
            ..Default::default()
        }
    }

    #[test]
    fn story_framing_applies_zoom_focus_dim() {
        let source = layout();
        let untouched = source.clone();
        let framing = DiagramFraming {
            zoom: Some(1.5),
            focus_element_ids: vec![ObjectId::from_parts(1, 2)],
            dimmed_element_ids: vec![ObjectId::from_parts(1, 1)],
            caption: Some("Étape 1".into()),
        };

        let framed = apply_framing(&framing, &source);
        assert_eq!(source, untouched, "le layout source reste intact");
        assert_eq!(framed.zoom, 1.5);
        assert_eq!(framed.caption.as_deref(), Some("Étape 1"));
        assert_eq!(framed.nodes.len(), 3);

        let focused = framed
            .nodes
            .iter()
            .find(|node| node.object_id == ObjectId::from_parts(1, 2))
            .expect("nœud focalisé");
        assert!(focused.highlighted);
        assert!(!focused.dimmed);
        assert_eq!(focused.opacity, 1.0);
        assert_eq!(focused.zoom, 1.5);
        assert_eq!(focused.x, 45.0);
        assert_eq!(focused.y, 60.0);
        assert_eq!(focused.w, 150.0);
        assert_eq!(focused.h, 75.0);

        let dimmed = framed
            .nodes
            .iter()
            .find(|node| node.object_id == ObjectId::from_parts(1, 1))
            .expect("nœud atténué");
        assert!(dimmed.dimmed);
        assert!(!dimmed.highlighted);
        assert!(dimmed.opacity < 1.0);

        assert!(framed
            .nodes
            .last()
            .map(|node| node.highlighted)
            .unwrap_or(false));
    }

    #[test]
    fn zoom_defaults_and_clamps() {
        assert_eq!(effective_zoom(&DiagramFraming::default()), DEFAULT_ZOOM);
        assert_eq!(
            effective_zoom(&DiagramFraming {
                zoom: Some(12.0),
                ..Default::default()
            }),
            MAX_ZOOM
        );
        assert_eq!(
            effective_zoom(&DiagramFraming {
                zoom: Some(0.1),
                ..Default::default()
            }),
            MIN_ZOOM
        );
        assert_eq!(
            effective_zoom(&DiagramFraming {
                zoom: Some(f32::NAN),
                ..Default::default()
            }),
            DEFAULT_ZOOM
        );
    }

    #[test]
    fn focus_wins_over_dim() {
        let source = layout();
        let shared = ObjectId::from_parts(1, 3);
        let framing = DiagramFraming {
            zoom: None,
            focus_element_ids: vec![shared],
            dimmed_element_ids: vec![shared],
            caption: None,
        };
        let framed = apply_framing(&framing, &source);
        let node = framed
            .nodes
            .iter()
            .find(|node| node.object_id == shared)
            .expect("nœud partagé");
        assert!(node.highlighted);
        assert!(!node.dimmed);
        assert_eq!(node.opacity, 1.0);
    }

    #[test]
    fn default_framing_leaves_every_node_visible() {
        let framed = apply_framing(&DiagramFraming::default(), &layout());
        assert_eq!(framed.zoom, 1.0);
        assert!(framed.caption.is_none());
        assert!(framed
            .nodes
            .iter()
            .all(|node| !node.highlighted && node.opacity == 1.0));
    }
}
