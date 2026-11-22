//! Commandes d'écriture de la carte.
//!
//! Les gestes du canevas ne touchent jamais directement au layout authored :
//! ils passent par ces commandes, qui valident et persistent. Le reste de la
//! carte est une projection.

use crate::map::create::CreateKind;
use crate::state::{repo, App};
use archifier_core::map::*;
use archifier_core::ObjectId;
use archifier_repo::map::{NewApplication, NewCapability, NewNode, NewProduct, NewTeam};

/// Déplace un objet sur la carte et persiste son ancre.
pub fn move_anchor(app: App, object_id: ObjectId, level: MapLevel, x: f64, y: f64) {
    match repo().map_set_anchor(object_id, level, x, y) {
        Ok(()) => {
            app.refresh();
            app.publish_write();
            app.notify("Position enregistrée.");
        }
        Err(error) => app.fail(&error),
    }
}

/// Retire l'ancre manuelle : l'objet retrouve le placement automatique.
pub fn clear_anchor(app: App, object_id: ObjectId, level: MapLevel) {
    match repo().map_clear_anchor(object_id, level) {
        Ok(()) => {
            app.refresh();
            app.publish_write();
            app.notify("Placement automatique rétabli pour cet objet.");
        }
        Err(error) => app.fail(&error),
    }
}

/// Renomme un objet natif depuis le panneau de sélection.
pub fn rename_object(app: App, object_id: ObjectId, name: &str) {
    match repo().map_rename_object(object_id, name) {
        Ok(()) => {
            app.refresh();
            app.publish_write();
        }
        Err(error) => app.fail(&error),
    }
}

/// Crée une interface réelle entre deux objets et sa première version
/// brouillon. Une route n'est jamais une flèche anonyme.
pub fn link_objects(app: App, provider: ObjectId, consumer: ObjectId, level: InterfaceLevel) {
    match repo().map_create_interface_between(provider, consumer, level) {
        Ok(_) => {
            app.refresh();
            app.publish_write();
            app.notify("Interface créée en brouillon — qualifiez-la dans la route.");
        }
        Err(error) => app.fail(&error),
    }
}

/// Crée un objet au point demandé, puis le ancre au niveau courant.
///
/// `x` et `y` sont le coin supérieur gauche de la carte dans les coordonnées
/// de projection. Les types qui ne s'affichent pas à ce niveau (équipe,
/// capacité) sont créés quand même : ils apparaissent dans les fiches et la
/// recherche.
pub fn create_at(app: App, kind: CreateKind, query: &MapQuery, x: f64, y: f64) -> Option<ObjectId> {
    let level = query.level;
    let data = repo().map_data();
    let team = data.teams.first().map(|team| team.id);
    let needs_team = matches!(kind, CreateKind::Product | CreateKind::Application);

    if needs_team && team.is_none() {
        app.notify("Créez d'abord une équipe, puis posez un produit.");
        return None;
    }

    let result = match kind {
        CreateKind::Team => repo().map_create_team(NewTeam {
            name: "Nouvelle équipe".into(),
            topology: TeamTopology::StreamAligned,
            mission: String::new(),
        }),
        CreateKind::Product => repo().map_create_product(NewProduct {
            name: "Nouveau produit".into(),
            accountable_team_id: team,
            ..Default::default()
        }),
        CreateKind::Application => repo().map_create_application(NewApplication {
            name: "Nouvelle application".into(),
            operating_team_id: team,
            ..Default::default()
        }),
        CreateKind::Capability => repo().map_create_capability(NewCapability {
            name: "Nouvelle capacité".into(),
            ..Default::default()
        }),
        CreateKind::Node => repo().map_create_node(NewNode {
            name: "Nouveau nœud".into(),
            provider_team_id: team,
            ..Default::default()
        }),
        CreateKind::ValueStream | CreateKind::Interface => {
            app.notify("Ce type se crée depuis le panneau Créer.");
            return None;
        }
    };

    match result {
        Ok(id) => {
            // Au niveau produit, une application n'apparaît sur la carte que si
            // elle est rattachée au produit courant.
            if matches!(kind, CreateKind::Application)
                && level == MapLevel::Product
                && query.scope.kind == MapScopeKind::Product
            {
                if let Some(product_id) = query.scope.id {
                    if let Err(error) = repo().map_add_application_to_product(product_id, id) {
                        app.fail(&error);
                    }
                }
            }
            if matches!(
                kind,
                CreateKind::Product | CreateKind::Application | CreateKind::Node
            ) {
                if let Err(error) = repo().map_set_anchor(id, level, x, y) {
                    app.fail(&error);
                }
            }
            app.refresh();
            app.publish_write();
            app.notify(format!("{} créé — nommez-le.", kind.label()));
            Some(id)
        }
        Err(error) => {
            app.fail(&error);
            None
        }
    }
}
