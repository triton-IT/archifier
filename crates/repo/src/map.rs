//! Persistance et commandes de la couche native v0.4.
//!
//! Le dépôt historique reste la source de vérité ; la carte est un champ
//! additif du snapshot (`Dataset::map`). Toute écriture passe par `mutate`,
//! donc par la sérialisation atomique et l'index reconstruit.

use crate::{RepoError, RepoResult, Repository};
use archifier_core::map::*;
use archifier_core::ObjectId;
use time::format_description::well_known::Rfc3339;

fn issue_error(issue: MapAnomaly) -> RepoError {
    RepoError::new(issue.code, issue.message)
}

fn require_name(name: &str) -> RepoResult<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(RepoError::new(
            "OBJ-001",
            "Le nom est obligatoire pour créer un objet de carte.",
        ));
    }
    if name.chars().count() > 120 {
        return Err(RepoError::new("OBJ-002", "Le nom dépasse 120 caractères."));
    }
    Ok(name.to_string())
}

// ---------------------------------------------------------------------------
// Entrées de commande
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct NewTeam {
    pub name: String,
    pub topology: TeamTopology,
    pub mission: String,
}

#[derive(Debug, Clone, Default)]
pub struct NewValueStream {
    pub name: String,
    pub value_proposition: String,
    pub stages: Vec<String>,
    pub owner_team_id: Option<ObjectId>,
}

#[derive(Debug, Clone, Default)]
pub struct NewProduct {
    pub name: String,
    pub kind: ProductKind,
    pub criticality: Criticality,
    pub lifecycle: LifecycleState,
    pub accountable_team_id: Option<ObjectId>,
    pub primary_value_stream_id: Option<ObjectId>,
    pub primary_portfolio_id: Option<ObjectId>,
    pub business_domain_ids: Vec<ObjectId>,
    pub application_ids: Vec<ObjectId>,
    pub capability_ids: Vec<ObjectId>,
    pub value_proposition: String,
}

#[derive(Debug, Clone, Default)]
pub struct NewCapability {
    pub name: String,
    pub parent_id: Option<ObjectId>,
    pub criticality: Criticality,
}

#[derive(Debug, Clone, Default)]
pub struct NewApplication {
    pub name: String,
    pub kind: ApplicationKind,
    pub criticality: Criticality,
    pub lifecycle: LifecycleState,
    pub operating_team_id: Option<ObjectId>,
    pub node_ids: Vec<ObjectId>,
}

#[derive(Debug, Clone)]
pub struct NewComponent {
    pub application_id: ObjectId,
    pub name: String,
    pub kind: ComponentKind,
}

#[derive(Debug, Clone)]
pub struct NewInterface {
    pub name: String,
    pub level: InterfaceLevel,
    pub kind: InterfaceKind,
    pub exposure: InterfaceExposure,
    pub criticality: Criticality,
    pub provider_object_id: ObjectId,
    pub provider_product_id: Option<ObjectId>,
    pub data_classification: Option<DataClassification>,
}

#[derive(Debug, Clone)]
pub struct NewVersion {
    pub interface_id: ObjectId,
    pub semantic_version: String,
    pub available_from: Option<String>,
    pub available_until: Option<String>,
    pub breaking_change: bool,
}

#[derive(Debug, Clone, Default)]
pub struct NewNode {
    pub name: String,
    pub kind: TechnologyNodeKind,
    pub environment: Environment,
    pub provider_team_id: Option<ObjectId>,
}

#[derive(Debug, Clone)]
pub struct NewStage {
    pub value_stream_id: ObjectId,
    pub name: String,
    pub value_item: String,
}

#[derive(Debug, Clone, Default)]
pub struct NewDomain {
    pub name: String,
    pub description: String,
    pub parent_id: Option<ObjectId>,
}

// ---------------------------------------------------------------------------
// Lecture
// ---------------------------------------------------------------------------

impl Repository {
    pub fn map_data(&self) -> MapDataset {
        self.inner.borrow().data.map.clone()
    }

    pub fn map_is_empty(&self) -> bool {
        self.inner.borrow().data.map.is_empty()
    }

    pub fn map_projection(&self, query: &MapQuery) -> MapProjection {
        project(&self.inner.borrow().data.map, query)
    }

    pub fn map_search(&self, needle: &str) -> Vec<MapSearchHit> {
        search(&self.inner.borrow().data.map, needle)
    }

    /// Objets natifs pour la Bibliothèque (nom, type, route).
    pub fn map_inventory(&self) -> Vec<MapSearchHit> {
        let data = self.inner.borrow();
        let mut out = Vec::new();
        for product in &data.data.map.products {
            let team = product
                .accountable_team_id
                .and_then(|id| data.data.map.team(id))
                .map(|team| team.name.clone())
                .unwrap_or_else(|| "Sans équipe".into());
            out.push(MapSearchHit {
                object_id: product.id,
                kind: MapObjectKind::Product,
                label: product.name.clone(),
                detail: format!(
                    "Produit · {} · {} · {} · {}",
                    product.kind.label(),
                    product.lifecycle.label(),
                    product.criticality.label(),
                    team
                ),
                route: format!("#/products/{}", product.id),
            });
        }
        for team in &data.data.map.teams {
            out.push(MapSearchHit {
                object_id: team.id,
                kind: MapObjectKind::Team,
                label: team.name.clone(),
                detail: format!("Équipe · {}", team.topology.label()),
                route: format!("#/teams/{}", team.id),
            });
        }
        for value_stream in &data.data.map.value_streams {
            let stages = data
                .data
                .map
                .stages
                .iter()
                .filter(|stage| stage.value_stream_id == value_stream.id)
                .count();
            out.push(MapSearchHit {
                object_id: value_stream.id,
                kind: MapObjectKind::Stage,
                label: value_stream.name.clone(),
                detail: format!("Value stream · {stages} étape(s)"),
                route: format!("#/value-streams/{}", value_stream.id),
            });
        }
        for application in &data.data.map.applications {
            let team = application
                .operating_team_id
                .and_then(|id| data.data.map.team(id))
                .map(|team| team.name.clone())
                .unwrap_or_else(|| "Sans équipe".into());
            out.push(MapSearchHit {
                object_id: application.id,
                kind: MapObjectKind::Application,
                label: application.name.clone(),
                detail: format!(
                    "Application · {} · {} · {}",
                    application.kind.label(),
                    application.lifecycle.label(),
                    team
                ),
                route: format!("#/applications/{}", application.id),
            });
        }
        for capability in &data.data.map.capabilities {
            out.push(MapSearchHit {
                object_id: capability.id,
                kind: MapObjectKind::Capability,
                label: capability.name.clone(),
                detail: format!("Capacité · {}", capability.criticality.label()),
                route: format!("#/capabilities/{}", capability.id),
            });
        }
        for node in &data.data.map.nodes {
            let environment = node
                .environment
                .map(|value| value.label().to_string())
                .unwrap_or_else(|| "Environnement non renseigné".into());
            out.push(MapSearchHit {
                object_id: node.id,
                kind: MapObjectKind::Node,
                label: node.name.clone(),
                detail: format!("Nœud · {} · {}", node.kind.label(), environment),
                route: format!("#/infrastructure/{}", node.id),
            });
        }
        for interface in &data.data.map.interfaces {
            out.push(MapSearchHit {
                object_id: interface.id,
                kind: MapObjectKind::Interface,
                label: interface.name.clone(),
                detail: format!(
                    "Interface · {} · {} · {}",
                    interface.level.label(),
                    interface.exposure.label(),
                    interface.criticality.label()
                ),
                route: format!("#/interfaces/{}", interface.id),
            });
        }
        out.sort_by(|a, b| a.label.cmp(&b.label));
        out
    }

    pub fn map_anomalies(&self) -> Vec<MapAnomaly> {
        dataset_anomalies(&self.inner.borrow().data.map)
    }
}

// ---------------------------------------------------------------------------
// Écritures
// ---------------------------------------------------------------------------

impl Repository {
    pub fn map_create_team(&self, input: NewTeam) -> RepoResult<ObjectId> {
        let name = require_name(&input.name)?;
        let id = self.new_id();
        self.mutate(|inner| {
            inner.data.map.teams.push(Team {
                id,
                name,
                topology: input.topology,
                mission: input.mission,
                interaction_modes: vec![],
                revision: 1,
            });
            inner.data.map.revision += 1;
            Ok(id)
        })
    }

    pub fn map_create_value_stream(&self, input: NewValueStream) -> RepoResult<ObjectId> {
        let name = require_name(&input.name)?;
        let id = self.new_id();
        let labels: Vec<String> = input
            .stages
            .iter()
            .map(|label| label.trim().to_string())
            .filter(|label| !label.is_empty())
            .collect();
        let stage_ids: Vec<ObjectId> = (0..labels.len()).map(|_| self.new_id()).collect();
        self.mutate(|inner| {
            if let Some(team_id) = input.owner_team_id {
                if inner.data.map.team(team_id).is_none() {
                    return Err(RepoError::new(
                        "TEAM-404",
                        "L'équipe propriétaire du value stream n'existe pas.",
                    ));
                }
            }
            let mut stages = Vec::new();
            for (index, (stage_id, label)) in stage_ids.iter().zip(labels.iter()).enumerate() {
                stages.push(ValueStreamStage {
                    id: *stage_id,
                    value_stream_id: id,
                    name: label.clone(),
                    order: index as u32 + 1,
                    value_item: String::new(),
                    capability_ids: vec![],
                    product_ids: vec![],
                    process_ids: vec![],
                    revision: 1,
                });
            }
            inner.data.map.stages.extend(stages);
            inner.data.map.value_streams.push(ValueStream {
                id,
                name,
                value_proposition: input.value_proposition,
                stage_ids,
                owner_team_id: input.owner_team_id,
                revision: 1,
            });
            inner.data.map.revision += 1;
            Ok(id)
        })
    }

    /// Ajoute une étape à la fin d'un value stream existant.
    pub fn map_create_stage(&self, input: NewStage) -> RepoResult<ObjectId> {
        let name = require_name(&input.name)?;
        let id = self.new_id();
        self.mutate(|inner| {
            let data = &mut inner.data.map;
            let Some(stream) = data.value_stream(input.value_stream_id) else {
                return Err(RepoError::new("VS-404", "Value stream introuvable."));
            };
            let order = data
                .stages
                .iter()
                .filter(|stage| stage.value_stream_id == stream.id)
                .map(|stage| stage.order)
                .max()
                .unwrap_or(0)
                + 1;
            data.stages.push(ValueStreamStage {
                id,
                value_stream_id: input.value_stream_id,
                name,
                order,
                value_item: input.value_item,
                capability_ids: vec![],
                product_ids: vec![],
                process_ids: vec![],
                revision: 1,
            });
            if let Some(stream) = data
                .value_streams
                .iter_mut()
                .find(|stream| stream.id == input.value_stream_id)
            {
                stream.stage_ids.push(id);
                stream.revision += 1;
            }
            data.revision += 1;
            Ok(id)
        })
    }

    /// Rattache un produit à une étape de value stream.
    pub fn map_add_product_to_stage(
        &self,
        stage_id: ObjectId,
        product_id: ObjectId,
    ) -> RepoResult<()> {
        self.mutate(|inner| {
            let data = &mut inner.data.map;
            if data.product(product_id).is_none() {
                return Err(RepoError::new("PRD-404", "Produit introuvable."));
            }
            let stage = data
                .stages
                .iter_mut()
                .find(|stage| stage.id == stage_id)
                .ok_or_else(|| RepoError::new("VS-404", "Étape introuvable."))?;
            if !stage.product_ids.contains(&product_id) {
                stage.product_ids.push(product_id);
                stage.revision += 1;
            }
            data.revision += 1;
            Ok(())
        })
    }

    /// Rattache une capacité à une étape de value stream.
    pub fn map_add_capability_to_stage(
        &self,
        stage_id: ObjectId,
        capability_id: ObjectId,
    ) -> RepoResult<()> {
        self.mutate(|inner| {
            let data = &mut inner.data.map;
            if data.capability(capability_id).is_none() {
                return Err(RepoError::new("CAP-404", "Capacité introuvable."));
            }
            let stage = data
                .stages
                .iter_mut()
                .find(|stage| stage.id == stage_id)
                .ok_or_else(|| RepoError::new("VS-404", "Étape introuvable."))?;
            if !stage.capability_ids.contains(&capability_id) {
                stage.capability_ids.push(capability_id);
                stage.revision += 1;
            }
            data.revision += 1;
            Ok(())
        })
    }

    /// Crée un domaine métier, éventuellement enfant d'un domaine existant.
    pub fn map_create_business_domain(&self, input: NewDomain) -> RepoResult<ObjectId> {
        let name = require_name(&input.name)?;
        let id = self.new_id();
        self.mutate(|inner| {
            let data = &mut inner.data.map;
            if let Some(parent_id) = input.parent_id {
                if data.domain(parent_id).is_none() {
                    return Err(RepoError::new("DOM-404", "Domaine parent introuvable."));
                }
            }
            data.domains.push(BusinessDomain {
                id,
                name,
                description: input.description,
                parent_id: input.parent_id,
                revision: 1,
            });
            data.revision += 1;
            Ok(id)
        })
    }

    /// Classe un produit dans un domaine métier. Le domaine ne possède jamais
    /// le produit : il ne fait que le classer.
    pub fn map_add_product_to_domain(
        &self,
        product_id: ObjectId,
        domain_id: ObjectId,
    ) -> RepoResult<()> {
        self.mutate(|inner| {
            let data = &mut inner.data.map;
            if data.domain(domain_id).is_none() {
                return Err(RepoError::new("DOM-404", "Domaine métier introuvable."));
            }
            let product = data
                .products
                .iter_mut()
                .find(|product| product.id == product_id)
                .ok_or_else(|| RepoError::new("PRD-404", "Produit introuvable."))?;
            if !product.business_domain_ids.contains(&domain_id) {
                product.business_domain_ids.push(domain_id);
                product.revision += 1;
            }
            data.revision += 1;
            Ok(())
        })
    }

    pub fn map_create_product(&self, input: NewProduct) -> RepoResult<ObjectId> {
        let name = require_name(&input.name)?;
        let id = self.new_id();
        self.mutate(|inner| {
            let data = &mut inner.data.map;
            if let Some(team_id) = input.accountable_team_id {
                if data.team(team_id).is_none() {
                    return Err(RepoError::new(
                        "PRD-001",
                        "L'équipe accountable doit exister dans le référentiel.",
                    ));
                }
            }
            if let Some(value_stream_id) = input.primary_value_stream_id {
                if data.value_stream(value_stream_id).is_none() {
                    return Err(RepoError::new(
                        "PRD-033",
                        "Le value stream principal doit exister.",
                    ));
                }
            }
            if let Some(portfolio_id) = input.primary_portfolio_id {
                if data.portfolio(portfolio_id).is_none() {
                    return Err(RepoError::new(
                        "PRD-033",
                        "Le portefeuille de repli doit exister.",
                    ));
                }
            }
            for application_id in &input.application_ids {
                if data.application(*application_id).is_none() {
                    return Err(RepoError::new(
                        "PRD-005",
                        "Un produit utilise des applications existantes ; il ne les possède pas.",
                    ));
                }
            }
            let product = Product {
                id,
                name,
                kind: input.kind,
                lifecycle: input.lifecycle,
                criticality: input.criticality,
                parent_product_id: None,
                portfolio_ids: input.primary_portfolio_id.into_iter().collect(),
                accountable_team_id: input.accountable_team_id,
                contributing_team_ids: vec![],
                primary_value_stream_id: input.primary_value_stream_id,
                primary_portfolio_id: input.primary_portfolio_id,
                business_domain_ids: input.business_domain_ids,
                value_proposition: input.value_proposition,
                capability_ids: input.capability_ids,
                application_ids: input.application_ids,
                revision: 1,
            };
            validate_product(data, &product).map_err(issue_error)?;
            data.products.push(product);
            data.revision += 1;
            Ok(id)
        })
    }

    /// Rattache une application existante à un produit : relation d'usage,
    /// jamais de possession (`PRD-005`).
    pub fn map_add_application_to_product(
        &self,
        product_id: ObjectId,
        application_id: ObjectId,
    ) -> RepoResult<()> {
        self.mutate(|inner| {
            let data = &mut inner.data.map;
            if data.application(application_id).is_none() {
                return Err(RepoError::new("APP-404", "Application introuvable."));
            }
            let product = data
                .products
                .iter_mut()
                .find(|product| product.id == product_id)
                .ok_or_else(|| RepoError::new("PRD-404", "Produit introuvable."))?;
            if !product.application_ids.contains(&application_id) {
                product.application_ids.push(application_id);
                product.revision += 1;
            }
            data.revision += 1;
            Ok(())
        })
    }

    /// Ajoute une capacité réalisée par un produit (`PRD-016`).
    pub fn map_add_capability_to_product(
        &self,
        product_id: ObjectId,
        capability_id: ObjectId,
    ) -> RepoResult<()> {
        self.mutate(|inner| {
            let data = &mut inner.data.map;
            if data.capability(capability_id).is_none() {
                return Err(RepoError::new("CAP-404", "Capacité introuvable."));
            }
            let product = data
                .products
                .iter_mut()
                .find(|product| product.id == product_id)
                .ok_or_else(|| RepoError::new("PRD-404", "Produit introuvable."))?;
            if !product.capability_ids.contains(&capability_id) {
                product.capability_ids.push(capability_id);
                product.revision += 1;
            }
            data.revision += 1;
            Ok(())
        })
    }

    /// Rattache ou détache un produit de son value stream principal (`PRD-033`).
    pub fn map_set_product_value_stream(
        &self,
        product_id: ObjectId,
        value_stream_id: Option<ObjectId>,
    ) -> RepoResult<()> {
        self.mutate(|inner| {
            let data = &mut inner.data.map;
            if let Some(value_stream_id) = value_stream_id {
                if data.value_stream(value_stream_id).is_none() {
                    return Err(RepoError::new("VS-404", "Value stream introuvable."));
                }
            }
            let product = data
                .products
                .iter_mut()
                .find(|product| product.id == product_id)
                .ok_or_else(|| RepoError::new("PRD-404", "Produit introuvable."))?;
            product.primary_value_stream_id = value_stream_id;
            product.revision += 1;
            data.revision += 1;
            Ok(())
        })
    }

    /// Définit l'équipe accountable d'un produit (`PRD-001`).
    pub fn map_set_product_accountable_team(
        &self,
        product_id: ObjectId,
        team_id: Option<ObjectId>,
    ) -> RepoResult<()> {
        self.mutate(|inner| {
            let data = &mut inner.data.map;
            if let Some(team_id) = team_id {
                if data.team(team_id).is_none() {
                    return Err(RepoError::new("TEAM-404", "Équipe introuvable."));
                }
            }
            let product = data
                .products
                .iter_mut()
                .find(|product| product.id == product_id)
                .ok_or_else(|| RepoError::new("PRD-404", "Produit introuvable."))?;
            product.accountable_team_id = team_id;
            product.revision += 1;
            data.revision += 1;
            Ok(())
        })
    }

    /// Définit l'équipe opératrice d'une application (`PRD-009`).
    pub fn map_set_application_operating_team(
        &self,
        application_id: ObjectId,
        team_id: Option<ObjectId>,
    ) -> RepoResult<()> {
        self.mutate(|inner| {
            let data = &mut inner.data.map;
            if let Some(team_id) = team_id {
                if data.team(team_id).is_none() {
                    return Err(RepoError::new("TEAM-404", "Équipe introuvable."));
                }
            }
            let application = data
                .applications
                .iter_mut()
                .find(|application| application.id == application_id)
                .ok_or_else(|| RepoError::new("APP-404", "Application introuvable."))?;
            application.operating_team_id = team_id;
            application.revision += 1;
            data.revision += 1;
            Ok(())
        })
    }

    /// Définit l'équipe propriétaire d'un value stream.
    pub fn map_set_value_stream_owner(
        &self,
        value_stream_id: ObjectId,
        team_id: Option<ObjectId>,
    ) -> RepoResult<()> {
        self.mutate(|inner| {
            let data = &mut inner.data.map;
            if let Some(team_id) = team_id {
                if data.team(team_id).is_none() {
                    return Err(RepoError::new("TEAM-404", "Équipe introuvable."));
                }
            }
            let stream = data
                .value_streams
                .iter_mut()
                .find(|stream| stream.id == value_stream_id)
                .ok_or_else(|| RepoError::new("VS-404", "Value stream introuvable."))?;
            stream.owner_team_id = team_id;
            stream.revision += 1;
            data.revision += 1;
            Ok(())
        })
    }

    /// Définit l'équipe fournisseur d'un nœud d'infrastructure.
    pub fn map_set_node_provider_team(
        &self,
        node_id: ObjectId,
        team_id: Option<ObjectId>,
    ) -> RepoResult<()> {
        self.mutate(|inner| {
            let data = &mut inner.data.map;
            if let Some(team_id) = team_id {
                if data.team(team_id).is_none() {
                    return Err(RepoError::new("TEAM-404", "Équipe introuvable."));
                }
            }
            let node = data
                .nodes
                .iter_mut()
                .find(|node| node.id == node_id)
                .ok_or_else(|| RepoError::new("NODE-404", "Nœud introuvable."))?;
            node.provider_team_id = team_id;
            node.revision += 1;
            data.revision += 1;
            Ok(())
        })
    }

    pub fn map_create_capability(&self, input: NewCapability) -> RepoResult<ObjectId> {
        let name = require_name(&input.name)?;
        let id = self.new_id();
        self.mutate(|inner| {
            let data = &mut inner.data.map;
            let level = input
                .parent_id
                .and_then(|parent_id| data.capability(parent_id))
                .map(|parent| parent.level + 1)
                .unwrap_or(1);
            if level > 3 {
                return Err(RepoError::new(
                    "PRD-017",
                    "La profondeur utile d'une capacité s'arrête à trois niveaux.",
                ));
            }
            data.capabilities.push(Capability {
                id,
                name,
                parent_id: input.parent_id,
                level,
                maturity: None,
                criticality: input.criticality,
                revision: 1,
            });
            data.revision += 1;
            Ok(id)
        })
    }

    pub fn map_create_application(&self, input: NewApplication) -> RepoResult<ObjectId> {
        let name = require_name(&input.name)?;
        let id = self.new_id();
        self.mutate(|inner| {
            let data = &mut inner.data.map;
            if let Some(team_id) = input.operating_team_id {
                if data.team(team_id).is_none() {
                    return Err(RepoError::new(
                        "PRD-009",
                        "L'équipe opératrice doit exister dans le référentiel.",
                    ));
                }
            }
            for node_id in &input.node_ids {
                if data.node(*node_id).is_none() {
                    return Err(RepoError::new(
                        "PRD-022",
                        "Le nœud de déploiement doit exister.",
                    ));
                }
            }
            let application = Application {
                id,
                name,
                kind: input.kind,
                lifecycle: input.lifecycle,
                criticality: input.criticality,
                operating_team_id: input.operating_team_id,
                component_ids: vec![],
                data_object_ids: vec![],
                node_ids: input.node_ids,
                revision: 1,
            };
            validate_application(data, &application).map_err(issue_error)?;
            data.applications.push(application);
            data.revision += 1;
            Ok(id)
        })
    }

    pub fn map_create_component(&self, input: NewComponent) -> RepoResult<ObjectId> {
        let name = require_name(&input.name)?;
        let id = self.new_id();
        self.mutate(|inner| {
            let data = &mut inner.data.map;
            if data.application(input.application_id).is_none() {
                return Err(RepoError::new("APP-404", "Application introuvable."));
            }
            data.components.push(ApplicationComponent {
                id,
                application_id: input.application_id,
                name,
                kind: input.kind,
                interface_ids: vec![],
                data_object_ids: vec![],
                revision: 1,
            });
            if let Some(application) = data
                .applications
                .iter_mut()
                .find(|application| application.id == input.application_id)
            {
                application.component_ids.push(id);
                application.revision += 1;
            }
            data.revision += 1;
            Ok(id)
        })
    }

    pub fn map_create_interface(&self, input: NewInterface) -> RepoResult<ObjectId> {
        let name = require_name(&input.name)?;
        if input.provider_object_id.as_str().is_empty() {
            return Err(RepoError::new("PRD-024", "Une interface a un fournisseur."));
        }
        let id = self.new_id();
        self.mutate(|inner| {
            let data = &mut inner.data.map;
            if data.object_label(input.provider_object_id).is_none() {
                return Err(RepoError::new(
                    "PRD-024",
                    "Le fournisseur de l'interface doit exister.",
                ));
            }
            let interface = MapInterface {
                id,
                name,
                level: input.level,
                category: match input.level {
                    InterfaceLevel::Business => InterfaceCategory::Business,
                    InterfaceLevel::Application => InterfaceCategory::Application,
                    InterfaceLevel::Infrastructure => InterfaceCategory::Technical,
                },
                kind: input.kind,
                provider_object_id: input.provider_object_id,
                provider_product_id: input.provider_product_id,
                exposure: input.exposure,
                criticality: input.criticality,
                data_classification: input.data_classification,
                revision: 1,
            };
            validate_interface(data, &interface, false).map_err(issue_error)?;
            data.interfaces.push(interface);
            // Une interface fournie par un composant doit apparaître dans son
            // application : la liste des ports du composant reste la source de
            // vérité de la frontière applicative.
            if let Some(component) = data
                .components
                .iter_mut()
                .find(|component| component.id == input.provider_object_id)
            {
                if !component.interface_ids.contains(&id) {
                    component.interface_ids.push(id);
                    component.revision += 1;
                }
            }
            data.revision += 1;
            Ok(id)
        })
    }

    pub fn map_create_interface_version(&self, input: NewVersion) -> RepoResult<ObjectId> {
        let version = input.semantic_version.trim();
        if !valid_semver(version) {
            return Err(RepoError::new(
                "PRD-025",
                "La version sémantique doit respecter MAJEUR.MINEUR.CORRECTIF.",
            ));
        }
        let id = self.new_id();
        self.mutate(|inner| {
            let data = &mut inner.data.map;
            let Some(interface) = data.interface(input.interface_id) else {
                return Err(RepoError::new("INT-404", "Interface introuvable."));
            };
            if !version_after_existing(data, interface.id, version) {
                return Err(RepoError::new(
                    "PRD-030",
                    "Une nouvelle version doit être supérieure à la version existante.",
                ));
            }
            data.versions.push(MapInterfaceVersion {
                id,
                interface_id: input.interface_id,
                semantic_version: version.to_string(),
                publication_status: PublicationState::Published,
                available_from: input.available_from,
                available_until: input.available_until,
                breaking_change: input.breaking_change,
                contract_type: None,
                revision: 1,
            });
            data.revision += 1;
            Ok(id)
        })
    }

    pub fn map_add_interface_consumption(
        &self,
        version_id: ObjectId,
        consumer_object_id: ObjectId,
        criticality: Criticality,
    ) -> RepoResult<ObjectId> {
        let id = self.new_id();
        self.mutate(|inner| {
            let data = &mut inner.data.map;
            validate_consumption(data, version_id, consumer_object_id).map_err(issue_error)?;
            let already = data.consumptions.iter().any(|consumption| {
                consumption.interface_version_id == version_id
                    && consumption.consumer_object_id == consumer_object_id
            });
            if already {
                return Err(RepoError::new(
                    "PRD-026",
                    "Ce consommateur référence déjà cette version.",
                ));
            }
            data.consumptions.push(InterfaceConsumption {
                id,
                interface_version_id: version_id,
                consumer_object_id,
                criticality,
                since: None,
                until: None,
                revision: 1,
            });
            data.revision += 1;
            Ok(id)
        })
    }

    pub fn map_create_node(&self, input: NewNode) -> RepoResult<ObjectId> {
        let name = require_name(&input.name)?;
        let id = self.new_id();
        self.mutate(|inner| {
            let data = &mut inner.data.map;
            if let Some(team_id) = input.provider_team_id {
                if data.team(team_id).is_none() {
                    return Err(RepoError::new("TEAM-404", "Équipe introuvable."));
                }
            }
            data.nodes.push(TechnologyNode {
                id,
                name,
                kind: input.kind,
                environment: Some(input.environment),
                provider_team_id: input.provider_team_id,
                parent_node_id: None,
                application_ids: vec![],
                lifecycle: LifecycleState::Run,
                revision: 1,
            });
            data.revision += 1;
            Ok(id)
        })
    }

    pub fn map_deploy_application(
        &self,
        application_id: ObjectId,
        node_id: ObjectId,
    ) -> RepoResult<()> {
        self.mutate(|inner| {
            let data = &mut inner.data.map;
            if data.node(node_id).is_none() {
                return Err(RepoError::new("PRD-022", "Nœud introuvable."));
            }
            let application = data
                .applications
                .iter_mut()
                .find(|application| application.id == application_id)
                .ok_or_else(|| RepoError::new("APP-404", "Application introuvable."))?;
            if !application.node_ids.contains(&node_id) {
                application.node_ids.push(node_id);
                application.revision += 1;
            }
            if let Some(node) = data.nodes.iter_mut().find(|node| node.id == node_id) {
                if !node.application_ids.contains(&application_id) {
                    node.application_ids.push(application_id);
                    node.revision += 1;
                }
            }
            data.revision += 1;
            Ok(())
        })
    }

    /// Crée une interface réelle entre deux objets de la carte, avec sa
    /// première version brouillon et la consommation du côté cible.
    ///
    /// C'est le geste « relier » : une route n'est jamais une flèche anonyme.
    pub fn map_create_interface_between(
        &self,
        provider_object_id: ObjectId,
        consumer_object_id: ObjectId,
        level: InterfaceLevel,
    ) -> RepoResult<ObjectId> {
        let id = self.new_id();
        let version_id = self.new_id();
        let consumption_id = self.new_id();
        self.mutate(|inner| {
            let data = &mut inner.data.map;
            let (_, provider_label) = data.object_label(provider_object_id).ok_or_else(|| {
                RepoError::new("PRD-024", "Le fournisseur de l'interface doit exister.")
            })?;
            let (_, consumer_label) = data.object_label(consumer_object_id).ok_or_else(|| {
                RepoError::new("PRD-026", "Le consommateur de l'interface doit exister.")
            })?;
            let provider_is_product = data.product(provider_object_id).is_some();
            let interface = MapInterface {
                id,
                name: format!("{provider_label} → {consumer_label}"),
                level,
                category: match level {
                    InterfaceLevel::Business => InterfaceCategory::Business,
                    InterfaceLevel::Application => InterfaceCategory::Application,
                    InterfaceLevel::Infrastructure => InterfaceCategory::Technical,
                },
                kind: InterfaceKind::Api,
                provider_object_id,
                provider_product_id: provider_is_product.then_some(provider_object_id),
                exposure: InterfaceExposure::Internal,
                criticality: Criticality::Medium,
                data_classification: None,
                revision: 1,
            };
            validate_interface(data, &interface, false).map_err(issue_error)?;
            data.interfaces.push(interface);
            data.versions.push(MapInterfaceVersion {
                id: version_id,
                interface_id: id,
                semantic_version: "0.1.0".into(),
                publication_status: PublicationState::Draft,
                available_from: None,
                available_until: None,
                breaking_change: false,
                contract_type: None,
                revision: 1,
            });
            data.consumptions.push(InterfaceConsumption {
                id: consumption_id,
                interface_version_id: version_id,
                consumer_object_id,
                criticality: Criticality::Medium,
                since: None,
                until: None,
                revision: 1,
            });
            data.revision += 1;
            Ok(id)
        })
    }

    /// Renomme un objet natif de la carte sans changer son identité.
    ///
    /// Le renommage est volontairement polymorphe : la carte peut éditer un
    /// produit, une équipe, une application, une capacité, un nœud ou une
    /// interface depuis le même panneau.
    pub fn map_rename_object(&self, object_id: ObjectId, name: &str) -> RepoResult<()> {
        let name = require_name(name)?;
        self.mutate(|inner| {
            let data = &mut inner.data.map;
            let mut renamed = false;

            macro_rules! try_rename {
                ($items:expr) => {
                    if !renamed {
                        if let Some(item) = $items.iter_mut().find(|item| item.id == object_id) {
                            item.name = name.clone();
                            item.revision += 1;
                            renamed = true;
                        }
                    }
                };
            }

            try_rename!(data.products);
            try_rename!(data.teams);
            try_rename!(data.value_streams);
            try_rename!(data.stages);
            try_rename!(data.capabilities);
            try_rename!(data.applications);
            try_rename!(data.components);
            try_rename!(data.interfaces);
            try_rename!(data.nodes);
            try_rename!(data.domains);
            try_rename!(data.portfolios);
            try_rename!(data.initiatives);

            if !renamed {
                return Err(RepoError::new("MAP-404", "Objet de carte introuvable."));
            }
            data.revision += 1;
            Ok(())
        })
    }

    /// Déplace un objet sur la carte : crée ou met à jour une ancre
    /// `pinned=true` (`MAP-022`). Le layout authored n'est jamais touché.
    pub fn map_set_anchor(
        &self,
        object_id: ObjectId,
        level: MapLevel,
        x: f64,
        y: f64,
    ) -> RepoResult<()> {
        let updated_at = self
            .now()
            .format(&Rfc3339)
            .unwrap_or_else(|_| String::new());
        self.mutate(|inner| {
            let data = &mut inner.data.map;
            if data.object_label(object_id).is_none() {
                return Err(RepoError::new("MAP-404", "Objet de carte introuvable."));
            }
            if let Some(anchor) = data
                .anchors
                .iter_mut()
                .find(|anchor| anchor.object_id == object_id && anchor.level == level)
            {
                anchor.x = x;
                anchor.y = y;
                anchor.pinned = true;
                anchor.updated_at = updated_at;
                anchor.revision += 1;
            } else {
                data.anchors.push(MapAnchor {
                    object_id,
                    level,
                    district_id: None,
                    x,
                    y,
                    pinned: true,
                    updated_at,
                    revision: 1,
                });
            }
            data.revision += 1;
            Ok(())
        })
    }

    pub fn map_clear_anchor(&self, object_id: ObjectId, level: MapLevel) -> RepoResult<()> {
        self.mutate(|inner| {
            let data = &mut inner.data.map;
            data.anchors
                .retain(|anchor| !(anchor.object_id == object_id && anchor.level == level));
            data.revision += 1;
            Ok(())
        })
    }

    /// Charge un référentiel de démonstration lisible. Sans effet si la carte
    /// contient déjà des objets : aucune donnée utilisateur n'est écrasée.
    pub fn load_demo_map(&self) -> RepoResult<bool> {
        if !self.map_is_empty() {
            return Ok(false);
        }
        let dataset = self.demo_dataset();
        self.mutate(|inner| {
            inner.data.map = dataset;
            inner.data.map.revision += 1;
            Ok(true)
        })?;
        Ok(true)
    }

    fn demo_dataset(&self) -> MapDataset {
        let mut data = MapDataset::default();
        let id = || self.new_id();

        // Équipes
        let team_sub = id();
        let team_claim = id();
        let team_pay = id();
        let team_data = id();
        let team_risk = id();
        let teams = [
            (
                team_sub,
                "Équipe Souscription",
                TeamTopology::StreamAligned,
                "Rendre la souscription simple et rapide.",
            ),
            (
                team_claim,
                "Équipe Sinistres",
                TeamTopology::StreamAligned,
                "Traiter les sinistres en moins de 48 h.",
            ),
            (
                team_pay,
                "Équipe Paiements",
                TeamTopology::Platform,
                "Fournir un encaissement fiable à tous les produits.",
            ),
            (
                team_data,
                "Équipe Données Client",
                TeamTopology::Platform,
                "Exposer une connaissance client unique.",
            ),
            (
                team_risk,
                "Équipe Conformité",
                TeamTopology::Enabling,
                "Outiller la conformité sans opérer d'application.",
            ),
        ];
        for (team_id, name, topology, mission) in teams {
            data.teams.push(Team {
                id: team_id,
                name: name.into(),
                topology,
                mission: mission.into(),
                interaction_modes: vec![TeamInteractionMode::XAsAService],
                revision: 1,
            });
        }

        // Value streams et étapes
        let vs_subscribe = id();
        let vs_claim = id();
        let vs_pay = id();
        let stage_choose = id();
        let stage_quote = id();
        let stage_pay = id();
        let stage_declare = id();
        let stage_assess = id();
        let stage_indemnify = id();
        data.value_streams.push(ValueStream {
            id: vs_subscribe,
            name: "Souscrire un contrat".into(),
            value_proposition: "Un contrat adapté, souscrit en moins de dix minutes.".into(),
            stage_ids: vec![stage_choose, stage_quote, stage_pay],
            owner_team_id: Some(team_sub),
            revision: 1,
        });
        data.value_streams.push(ValueStream {
            id: vs_claim,
            name: "Gérer un sinistre".into(),
            value_proposition: "Un sinistre déclaré, évalué et indemnisé sans rupture.".into(),
            stage_ids: vec![stage_declare, stage_assess, stage_indemnify],
            owner_team_id: Some(team_claim),
            revision: 1,
        });
        data.value_streams.push(ValueStream {
            id: vs_pay,
            name: "Encaisser un paiement".into(),
            value_proposition: "Un paiement encaissé, traçable et réconcilié.".into(),
            stage_ids: vec![stage_pay],
            owner_team_id: Some(team_pay),
            revision: 1,
        });
        for (stage_id, vs, name, order, value) in [
            (
                stage_choose,
                vs_subscribe,
                "Choisir une offre",
                1,
                "Une offre comprise",
            ),
            (
                stage_quote,
                vs_subscribe,
                "Obtenir une tarification",
                2,
                "Un prix juste",
            ),
            (
                stage_pay,
                vs_subscribe,
                "Payer la première prime",
                3,
                "Un contrat actif",
            ),
            (
                stage_declare,
                vs_claim,
                "Déclarer le sinistre",
                1,
                "Un dossier ouvert",
            ),
            (
                stage_assess,
                vs_claim,
                "Évaluer le dommage",
                2,
                "Une expertise fiable",
            ),
            (
                stage_indemnify,
                vs_claim,
                "Indemniser",
                3,
                "Un client remboursé",
            ),
        ] {
            data.stages.push(ValueStreamStage {
                id: stage_id,
                value_stream_id: vs,
                name: name.into(),
                order,
                value_item: value.into(),
                capability_ids: vec![],
                product_ids: vec![],
                process_ids: vec![],
                revision: 1,
            });
        }
        data.stages.push(ValueStreamStage {
            id: id(),
            value_stream_id: vs_pay,
            name: "Encaisser".into(),
            order: 1,
            value_item: "Un paiement réconcilié".into(),
            capability_ids: vec![],
            product_ids: vec![],
            process_ids: vec![],
            revision: 1,
        });

        // Capacités
        let cap_pricing = id();
        let cap_claim = id();
        let cap_payment = id();
        let cap_customer = id();
        for (cap_id, name, criticality, maturity) in [
            (cap_pricing, "Tarification", Criticality::High, 4),
            (cap_claim, "Gestion de sinistre", Criticality::Critical, 3),
            (cap_payment, "Encaissement", Criticality::Critical, 4),
            (cap_customer, "Connaissance client", Criticality::High, 2),
        ] {
            data.capabilities.push(Capability {
                id: cap_id,
                name: name.into(),
                parent_id: None,
                level: 1,
                maturity: Some(maturity),
                criticality,
                revision: 1,
            });
        }

        // Applications
        let app_portal = id();
        let app_pricing = id();
        let app_policy = id();
        let app_claims = id();
        let app_pay = id();
        let app_crm = id();
        let app_data = id();
        let app_archive = id();
        for (app_id, name, kind, lifecycle, criticality, team) in [
            (
                app_portal,
                "Portail Client",
                ApplicationKind::Internal,
                LifecycleState::Run,
                Criticality::Critical,
                team_sub,
            ),
            (
                app_pricing,
                "Moteur de Tarification",
                ApplicationKind::Internal,
                LifecycleState::Run,
                Criticality::High,
                team_sub,
            ),
            (
                app_policy,
                "Core Contrat",
                ApplicationKind::Legacy,
                LifecycleState::Run,
                Criticality::Critical,
                team_sub,
            ),
            (
                app_claims,
                "Gestion Sinistres",
                ApplicationKind::Internal,
                LifecycleState::Run,
                Criticality::Critical,
                team_claim,
            ),
            (
                app_pay,
                "Payment Hub",
                ApplicationKind::Platform,
                LifecycleState::Run,
                Criticality::Critical,
                team_pay,
            ),
            (
                app_crm,
                "CRM Client",
                ApplicationKind::Saas,
                LifecycleState::Run,
                Criticality::High,
                team_data,
            ),
            (
                app_data,
                "Data Platform",
                ApplicationKind::Platform,
                LifecycleState::Run,
                Criticality::High,
                team_data,
            ),
            (
                app_archive,
                "Archive Contrats",
                ApplicationKind::Cots,
                LifecycleState::Sunset,
                Criticality::Low,
                team_sub,
            ),
        ] {
            data.applications.push(Application {
                id: app_id,
                name: name.into(),
                kind,
                lifecycle,
                criticality,
                operating_team_id: Some(team),
                component_ids: vec![],
                data_object_ids: vec![],
                node_ids: vec![],
                revision: 1,
            });
        }

        // Produits
        let prod_sub = id();
        let prod_claim = id();
        let prod_pricing = id();
        let prod_pay = id();
        let prod_client = id();
        let domain_auto = id();
        let domain_home = id();
        let domain_pay = id();
        data.products.push(Product {
            id: prod_sub,
            name: "Parcours Souscription".into(),
            kind: ProductKind::Customer,
            lifecycle: LifecycleState::Run,
            criticality: Criticality::Critical,
            parent_product_id: None,
            portfolio_ids: vec![],
            accountable_team_id: Some(team_sub),
            contributing_team_ids: vec![team_pay, team_data],
            primary_value_stream_id: Some(vs_subscribe),
            primary_portfolio_id: None,
            business_domain_ids: vec![domain_auto],
            value_proposition:
                "Souscrire une assurance auto ou habitation sans friction, en moins de dix minutes."
                    .into(),
            capability_ids: vec![cap_pricing],
            application_ids: vec![app_portal, app_pricing, app_policy, app_archive, app_crm],
            revision: 1,
        });
        data.products.push(Product {
            id: prod_claim,
            name: "Parcours Sinistre".into(),
            kind: ProductKind::Customer,
            lifecycle: LifecycleState::Run,
            criticality: Criticality::Critical,
            parent_product_id: None,
            portfolio_ids: vec![],
            accountable_team_id: Some(team_claim),
            contributing_team_ids: vec![team_pay, team_data, team_risk],
            primary_value_stream_id: Some(vs_claim),
            primary_portfolio_id: None,
            business_domain_ids: vec![domain_home],
            value_proposition: "Déclarer un sinistre en trois minutes et suivre son indemnisation."
                .into(),
            capability_ids: vec![cap_claim, cap_customer],
            application_ids: vec![app_claims, app_crm, app_policy, app_portal],
            revision: 1,
        });
        data.products.push(Product {
            id: prod_pricing,
            name: "Tarification".into(),
            kind: ProductKind::Platform,
            lifecycle: LifecycleState::Run,
            criticality: Criticality::High,
            parent_product_id: None,
            portfolio_ids: vec![],
            accountable_team_id: Some(team_sub),
            contributing_team_ids: vec![],
            primary_value_stream_id: None,
            primary_portfolio_id: None,
            business_domain_ids: vec![domain_auto],
            value_proposition: "Fournir un prix cohérent à tous les parcours de vente.".into(),
            capability_ids: vec![cap_pricing],
            application_ids: vec![app_pricing, app_data],
            revision: 1,
        });
        data.products.push(Product {
            id: prod_pay,
            name: "Encaissement".into(),
            kind: ProductKind::Platform,
            lifecycle: LifecycleState::Run,
            criticality: Criticality::Critical,
            parent_product_id: None,
            portfolio_ids: vec![],
            accountable_team_id: Some(team_pay),
            contributing_team_ids: vec![],
            primary_value_stream_id: None,
            primary_portfolio_id: None,
            business_domain_ids: vec![domain_pay],
            value_proposition: "Encaisser et réconcilier tous les paiements du groupe.".into(),
            capability_ids: vec![cap_payment],
            application_ids: vec![app_pay],
            revision: 1,
        });
        data.products.push(Product {
            id: prod_client,
            name: "Connaissance Client".into(),
            kind: ProductKind::Data,
            lifecycle: LifecycleState::Build,
            criticality: Criticality::High,
            parent_product_id: None,
            portfolio_ids: vec![],
            accountable_team_id: Some(team_data),
            contributing_team_ids: vec![team_risk],
            primary_value_stream_id: Some(vs_subscribe),
            primary_portfolio_id: None,
            business_domain_ids: vec![domain_auto, domain_pay],
            value_proposition: "Une vue client unique, consentie et exploitable.".into(),
            capability_ids: vec![cap_customer],
            application_ids: vec![app_crm, app_data],
            revision: 1,
        });

        // Contributions des produits aux étapes : un produit contribue à une
        // étape, il n'est jamais dupliqué dans le value stream.
        for stage in &mut data.stages {
            match stage.value_stream_id {
                vs if vs == vs_subscribe => {
                    stage.product_ids = vec![prod_sub, prod_client];
                    stage.capability_ids = vec![cap_pricing, cap_customer];
                }
                vs if vs == vs_claim => {
                    stage.product_ids = vec![prod_claim];
                    stage.capability_ids = vec![cap_claim, cap_customer];
                }
                vs if vs == vs_pay => {
                    stage.product_ids = vec![prod_pay, prod_sub];
                    stage.capability_ids = vec![cap_payment];
                }
                _ => {}
            }
        }

        // Nœuds déclarés avant les interfaces techniques.
        let node_k8s = id();
        let node_main = id();
        let node_saas = id();
        let node_data = id();

        // Interfaces et versions
        let int_portal = id();
        let int_portal_version = id();
        let int_pricing = id();
        let int_pay = id();
        let int_client = id();
        let int_claims = id();
        let int_network = id();
        let int_network_version = id();
        let add_interface =
            |data: &mut MapDataset, interface: MapInterface, version: MapInterfaceVersion| {
                data.interfaces.push(interface);
                data.versions.push(version);
            };
        add_interface(
            &mut data,
            MapInterface {
                id: int_portal,
                name: "Portail Souscription API".into(),
                level: InterfaceLevel::Business,
                category: InterfaceCategory::Business,
                kind: InterfaceKind::Api,
                provider_object_id: prod_sub,
                provider_product_id: Some(prod_sub),
                exposure: InterfaceExposure::Product,
                criticality: Criticality::Critical,
                data_classification: Some(DataClassification::Confidential),
                revision: 1,
            },
            MapInterfaceVersion {
                id: int_portal_version,
                interface_id: int_portal,
                semantic_version: "2.1.0".into(),
                publication_status: PublicationState::Published,
                available_from: Some("2025-09-01".into()),
                available_until: None,
                breaking_change: false,
                contract_type: Some(ContractType::OpenApi),
                revision: 1,
            },
        );
        add_interface(
            &mut data,
            MapInterface {
                id: int_pricing,
                name: "Tarification API".into(),
                level: InterfaceLevel::Business,
                category: InterfaceCategory::Business,
                kind: InterfaceKind::Api,
                provider_object_id: prod_pricing,
                provider_product_id: Some(prod_pricing),
                exposure: InterfaceExposure::Product,
                criticality: Criticality::High,
                data_classification: Some(DataClassification::Internal),
                revision: 1,
            },
            MapInterfaceVersion {
                id: id(),
                interface_id: int_pricing,
                semantic_version: "1.4.0".into(),
                publication_status: PublicationState::Published,
                available_from: Some("2025-03-15".into()),
                available_until: None,
                breaking_change: false,
                contract_type: Some(ContractType::OpenApi),
                revision: 1,
            },
        );
        add_interface(
            &mut data,
            MapInterface {
                id: int_pay,
                name: "Payment API".into(),
                level: InterfaceLevel::Application,
                category: InterfaceCategory::Application,
                kind: InterfaceKind::Api,
                provider_object_id: app_pay,
                provider_product_id: Some(prod_pay),
                exposure: InterfaceExposure::Product,
                criticality: Criticality::Critical,
                data_classification: Some(DataClassification::Confidential),
                revision: 1,
            },
            MapInterfaceVersion {
                id: id(),
                interface_id: int_pay,
                semantic_version: "2.0.0".into(),
                publication_status: PublicationState::Published,
                available_from: Some("2026-01-15".into()),
                available_until: None,
                breaking_change: true,
                contract_type: Some(ContractType::OpenApi),
                revision: 1,
            },
        );
        add_interface(
            &mut data,
            MapInterface {
                id: int_client,
                name: "Client 360 API".into(),
                level: InterfaceLevel::Application,
                category: InterfaceCategory::Application,
                kind: InterfaceKind::Api,
                provider_object_id: app_data,
                provider_product_id: Some(prod_client),
                exposure: InterfaceExposure::Product,
                criticality: Criticality::High,
                data_classification: Some(DataClassification::Confidential),
                revision: 1,
            },
            MapInterfaceVersion {
                id: id(),
                interface_id: int_client,
                semantic_version: "1.0.0".into(),
                publication_status: PublicationState::Published,
                available_from: Some("2026-02-01".into()),
                available_until: None,
                breaking_change: false,
                contract_type: Some(ContractType::OpenApi),
                revision: 1,
            },
        );
        add_interface(
            &mut data,
            MapInterface {
                id: int_claims,
                name: "Sinistre API".into(),
                level: InterfaceLevel::Business,
                category: InterfaceCategory::Business,
                kind: InterfaceKind::Api,
                provider_object_id: prod_claim,
                provider_product_id: Some(prod_claim),
                exposure: InterfaceExposure::Product,
                criticality: Criticality::Critical,
                data_classification: Some(DataClassification::Confidential),
                revision: 1,
            },
            MapInterfaceVersion {
                id: id(),
                interface_id: int_claims,
                semantic_version: "1.2.0".into(),
                publication_status: PublicationState::Published,
                available_from: Some("2025-11-01".into()),
                available_until: None,
                breaking_change: false,
                contract_type: Some(ContractType::OpenApi),
                revision: 1,
            },
        );

        add_interface(
            &mut data,
            MapInterface {
                id: int_network,
                name: "Réseau Inter-DC".into(),
                level: InterfaceLevel::Infrastructure,
                category: InterfaceCategory::Technical,
                kind: InterfaceKind::Network,
                provider_object_id: node_k8s,
                provider_product_id: None,
                exposure: InterfaceExposure::Internal,
                criticality: Criticality::High,
                data_classification: Some(DataClassification::Confidential),
                revision: 1,
            },
            MapInterfaceVersion {
                id: int_network_version,
                interface_id: int_network,
                semantic_version: "1.0.0".into(),
                publication_status: PublicationState::Published,
                available_from: Some("2025-01-01".into()),
                available_until: None,
                breaking_change: false,
                contract_type: Some(ContractType::Other),
                revision: 1,
            },
        );

        // Consommations versionnées : le parcours souscription appelle le
        // paiement et la tarification ; le sinistre lit la connaissance client.
        let version_of = |data: &MapDataset, interface_id: ObjectId| {
            data.versions
                .iter()
                .find(|version| version.interface_id == interface_id)
                .map(|version| (version.id, version.semantic_version.clone()))
        };
        for (interface_id, consumer, criticality) in [
            (int_pay, app_portal, Criticality::Critical),
            (int_pricing, app_portal, Criticality::High),
            (int_client, app_claims, Criticality::High),
            (int_portal, app_claims, Criticality::Medium),
            (int_pay, prod_claim, Criticality::Critical),
        ] {
            if let Some((version_id, _)) = version_of(&data, interface_id) {
                data.consumptions.push(InterfaceConsumption {
                    id: id(),
                    interface_version_id: version_id,
                    consumer_object_id: consumer,
                    criticality,
                    since: Some("2026-03-01".into()),
                    until: None,
                    revision: 1,
                });
            }
        }

        // Nœuds et déploiement
        for (node_id, name, kind, environment) in [
            (
                node_k8s,
                "Kubernetes Production",
                TechnologyNodeKind::Cluster,
                Environment::Prod,
            ),
            (
                node_main,
                "Mainframe Assurance",
                TechnologyNodeKind::Server,
                Environment::Prod,
            ),
            (
                node_saas,
                "Salesforce Europe",
                TechnologyNodeKind::Platform,
                Environment::Prod,
            ),
            (
                node_data,
                "Lakehouse Production",
                TechnologyNodeKind::Cluster,
                Environment::Prod,
            ),
        ] {
            data.nodes.push(TechnologyNode {
                id: node_id,
                name: name.into(),
                kind,
                environment: Some(environment),
                provider_team_id: Some(team_data),
                parent_node_id: None,
                application_ids: vec![],
                lifecycle: LifecycleState::Run,
                revision: 1,
            });
        }
        for (app_id, node_id) in [
            (app_portal, node_k8s),
            (app_pricing, node_k8s),
            (app_policy, node_main),
            (app_claims, node_k8s),
            (app_pay, node_k8s),
            (app_crm, node_saas),
            (app_data, node_data),
            (app_archive, node_main),
        ] {
            if let Some(application) = data.applications.iter_mut().find(|a| a.id == app_id) {
                application.node_ids.push(node_id);
            }
            if let Some(node) = data.nodes.iter_mut().find(|n| n.id == node_id) {
                node.application_ids.push(app_id);
            }
        }

        // Consommation technique : le mainframe dépend du réseau inter-DC.
        data.consumptions.push(InterfaceConsumption {
            id: id(),
            interface_version_id: int_network_version,
            consumer_object_id: node_main,
            criticality: Criticality::High,
            since: Some("2025-01-01".into()),
            until: None,
            revision: 1,
        });

        // Composants de l'application de tarification.
        let comp_rules = id();
        let comp_api = id();
        data.components.push(ApplicationComponent {
            id: comp_rules,
            application_id: app_pricing,
            name: "Règles tarifaires".into(),
            kind: ComponentKind::Service,
            interface_ids: vec![],
            data_object_ids: vec![],
            revision: 1,
        });
        data.components.push(ApplicationComponent {
            id: comp_api,
            application_id: app_pricing,
            name: "API Tarification".into(),
            kind: ComponentKind::Adapter,
            interface_ids: vec![int_pricing],
            data_object_ids: vec![],
            revision: 1,
        });
        if let Some(application) = data.applications.iter_mut().find(|a| a.id == app_pricing) {
            application.component_ids = vec![comp_rules, comp_api];
        }

        // Domaines métier : classification seule, jamais propriété.
        for (domain_id, name, description) in [
            (
                domain_auto,
                "Assurance Auto",
                "Classification de connaissance du risque auto.",
            ),
            (
                domain_home,
                "Assurance Habitation",
                "Classification de connaissance du risque habitation.",
            ),
            (
                domain_pay,
                "Paiements",
                "Moyens de paiement et réconciliation.",
            ),
        ] {
            data.domains.push(BusinessDomain {
                id: domain_id,
                name: name.into(),
                description: description.into(),
                parent_id: None,
                revision: 1,
            });
        }

        data.revision = 1;
        data
    }
}

fn valid_semver(value: &str) -> bool {
    let mut parts = value.split('.');
    let (Some(major), Some(minor), Some(patch)) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    if parts.next().is_some() {
        return false;
    }
    [major, minor, patch]
        .iter()
        .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
}

fn version_after_existing(data: &MapDataset, interface_id: ObjectId, candidate: &str) -> bool {
    data.versions
        .iter()
        .filter(|version| version.interface_id == interface_id)
        .all(|version| compare_semver(candidate, &version.semantic_version).is_gt())
}

fn compare_semver(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |value: &str| -> (u64, u64, u64) {
        let mut parts = value.split('.');
        (
            parts.next().and_then(|p| p.parse().ok()).unwrap_or(0),
            parts.next().and_then(|p| p.parse().ok()).unwrap_or(0),
            parts.next().and_then(|p| p.parse().ok()).unwrap_or(0),
        )
    };
    parse(a).cmp(&parse(b))
}
