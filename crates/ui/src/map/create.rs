//! Création inline depuis la carte : aucun modal, aucun changement de route
//! imposé. Les formulaires restent dans le flux de la carte (ADR-023 §7).

use crate::state::{input_value, parse_object_id, repo, App};
use archifier_core::map::*;
use archifier_core::ObjectId;
use archifier_repo::map::{
    NewApplication, NewCapability, NewComponent, NewDomain, NewInterface, NewNode, NewProduct,
    NewStage, NewTeam, NewValueStream, NewVersion,
};
use leptos::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreateKind {
    Product,
    Team,
    ValueStream,
    Application,
    Interface,
    Capability,
    Node,
}

impl CreateKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Product => "Produit",
            Self::Team => "Équipe",
            Self::ValueStream => "Value stream",
            Self::Application => "Application",
            Self::Interface => "Interface",
            Self::Capability => "Capacité",
            Self::Node => "Nœud",
        }
    }
}

pub const CREATE_KINDS: &[CreateKind] = &[
    CreateKind::Product,
    CreateKind::Team,
    CreateKind::ValueStream,
    CreateKind::Application,
    CreateKind::Interface,
    CreateKind::Capability,
    CreateKind::Node,
];

#[component]
pub fn CreateTray(app: App, open: RwSignal<bool>, kind: RwSignal<CreateKind>) -> impl IntoView {
    view! {
        {move || open.get().then(|| {
            let data = repo().map_data();
            let teams: Vec<(String, String)> = data
                .teams
                .iter()
                .map(|team| (team.id.to_string(), team.name.clone()))
                .collect();
            let value_streams: Vec<(String, String)> = data
                .value_streams
                .iter()
                .map(|stream| (stream.id.to_string(), stream.name.clone()))
                .collect();
            let nodes: Vec<(String, String)> = data
                .nodes
                .iter()
                .map(|node| (node.id.to_string(), node.name.clone()))
                .collect();
            let mut providers: Vec<(String, String)> = Vec::new();
            for product in &data.products {
                providers.push((product.id.to_string(), format!("Produit · {}", product.name)));
            }
            for application in &data.applications {
                providers.push((
                    application.id.to_string(),
                    format!("Application · {}", application.name),
                ));
            }
            for component in &data.components {
                providers.push((
                    component.id.to_string(),
                    format!("Composant · {}", component.name),
                ));
            }
            for node in &data.nodes {
                providers.push((node.id.to_string(), format!("Nœud · {}", node.name)));
            }

            view! {
                <section class="create-tray" aria-label="Création rapide depuis la carte">
                    <header class="create-tray-head">
                        <div>
                            <strong>"Nouvel objet"</strong>
                            <p class="muted small">
                                "Créez l'objet sans quitter la carte. Il apparaît dès sa création."
                            </p>
                        </div>
                        <button class="btn ghost small" on:click=move |_| open.set(false)>
                            "Fermer"
                        </button>
                    </header>
                    <nav class="tabs" aria-label="Type d'objet à créer">
                        {CREATE_KINDS.iter().copied().map(|value| {
                            let active = kind.get() == value;
                            view! {
                                <button
                                    class="tab"
                                    class:active=active
                                    type="button"
                                    on:click=move |_| kind.set(value)
                                >
                                    {value.label()}
                                </button>
                            }
                        }).collect_view()}
                    </nav>
                    <div class="create-body">
                        {match kind.get() {
                            CreateKind::Product => product_form(app, teams, value_streams, open, kind).into_any(),
                            CreateKind::Team => team_form(app, open).into_any(),
                            CreateKind::ValueStream => value_stream_form(app, teams, open).into_any(),
                            CreateKind::Application => application_form(app, teams, nodes, open).into_any(),
                            CreateKind::Interface => interface_form(app, providers, open).into_any(),
                            CreateKind::Capability => capability_form(app, open).into_any(),
                            CreateKind::Node => node_form(app, teams, open).into_any(),
                        }}
                    </div>
                </section>
            }
        })}
    }
}

fn team_form(app: App, open: RwSignal<bool>) -> AnyView {
    let name = RwSignal::new(String::new());
    let topology = RwSignal::new(TeamTopology::StreamAligned.as_str().to_string());
    let mission = RwSignal::new(String::new());
    let submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let input = NewTeam {
            name: name.get(),
            topology: TeamTopology::parse(&topology.get()).unwrap_or_default(),
            mission: mission.get(),
        };
        if app.run(repo().map_create_team(input)).is_some() {
            name.set(String::new());
            mission.set(String::new());
            app.notify("Équipe créée.");
            open.set(false);
        }
    };
    view! {
        <form class="form-grid" on:submit=submit>
            <label class="field">
                <span>"Nom"</span>
                <input placeholder="Équipe Paiements" prop:value=move || name.get()
                    on:input=move |ev| name.set(input_value(&ev)) />
            </label>
            <label class="field">
                <span>"Type d'équipe"</span>
                <select on:change=move |ev| topology.set(input_value(&ev))>
                    {TeamTopology::ALL.iter().map(|value| view! {
                        <option value=value.as_str() selected=*value == TeamTopology::StreamAligned>{value.label()}</option>
                    }).collect_view()}
                </select>
            </label>
            <label class="field field-wide">
                <span>"Mission"</span>
                <input placeholder="Rendre la souscription simple et rapide." prop:value=move || mission.get()
                    on:input=move |ev| mission.set(input_value(&ev)) />
            </label>
            <div class="actions field-wide">
                <button class="btn primary" type="submit">"Créer l'équipe"</button>
            </div>
        </form>
    }.into_any()
}

fn value_stream_form(app: App, teams: Vec<(String, String)>, open: RwSignal<bool>) -> AnyView {
    let name = RwSignal::new(String::new());
    let proposition = RwSignal::new(String::new());
    let stages = RwSignal::new(String::new());
    let owner = RwSignal::new(teams.first().map(|(id, _)| id.clone()).unwrap_or_default());
    let team_choices = teams.clone();
    let submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let input = NewValueStream {
            name: name.get(),
            value_proposition: proposition.get(),
            stages: stages
                .get()
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_string)
                .collect(),
            owner_team_id: crate::state::parse_object_id(&owner.get()),
        };
        if app.run(repo().map_create_value_stream(input)).is_some() {
            name.set(String::new());
            proposition.set(String::new());
            stages.set(String::new());
            app.notify("Value stream créé.");
            open.set(false);
        }
    };
    view! {
        <form class="form-grid" on:submit=submit>
            <label class="field">
                <span>"Nom"</span>
                <input placeholder="Souscrire un contrat" prop:value=move || name.get()
                    on:input=move |ev| name.set(input_value(&ev)) />
            </label>
            <label class="field">
                <span>"Équipe propriétaire"</span>
                <select on:change=move |ev| owner.set(input_value(&ev))>
                    <option value="">"Aucune"</option>
                    {team_choices.into_iter().map(|(id, label)| view! {
                        <option value=id>{label}</option>
                    }).collect_view()}
                </select>
            </label>
            <label class="field field-wide">
                <span>"Proposition de valeur"</span>
                <input placeholder="Un contrat adapté, souscrit en moins de dix minutes." prop:value=move || proposition.get()
                    on:input=move |ev| proposition.set(input_value(&ev)) />
            </label>
            <label class="field field-wide">
                <span>"Étapes, une par ligne"</span>
                <textarea rows="4" placeholder="Choisir une offre\nObtenir une tarification\nPayer la première prime"
                    prop:value=move || stages.get()
                    on:input=move |ev| stages.set(input_value(&ev))></textarea>
            </label>
            <div class="actions field-wide">
                <button class="btn primary" type="submit">"Créer le value stream"</button>
            </div>
        </form>
    }.into_any()
}

fn product_form(
    app: App,
    teams: Vec<(String, String)>,
    value_streams: Vec<(String, String)>,
    open: RwSignal<bool>,
    kind_signal: RwSignal<CreateKind>,
) -> AnyView {
    let name = RwSignal::new(String::new());
    let kind = RwSignal::new(ProductKind::Customer.as_str().to_string());
    let criticality = RwSignal::new(Criticality::Medium.as_str().to_string());
    let value_stream = RwSignal::new(
        value_streams
            .first()
            .map(|(id, _)| id.clone())
            .unwrap_or_default(),
    );
    let proposition = RwSignal::new(String::new());
    if teams.is_empty() {
        return view! {
            <div class="callout">
                <strong>"Une équipe accountable est requise."</strong>
                <p>"Un produit a exactement une équipe responsable (PRD-001). Créez d'abord une équipe."</p>
                <button
                    class="btn small primary"
                    type="button"
                    on:click=move |_| kind_signal.set(CreateKind::Team)
                >
                    "Créer une équipe"
                </button>
            </div>
        }
        .into_any();
    }
    let team = RwSignal::new(teams[0].0.clone());
    let submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let input = NewProduct {
            name: name.get(),
            kind: ProductKind::parse(&kind.get()).unwrap_or_default(),
            criticality: Criticality::parse(&criticality.get()).unwrap_or_default(),
            lifecycle: LifecycleState::Discovery,
            accountable_team_id: crate::state::parse_object_id(&team.get()),
            primary_value_stream_id: crate::state::parse_object_id(&value_stream.get()),
            primary_portfolio_id: None,
            business_domain_ids: vec![],
            application_ids: vec![],
            capability_ids: vec![],
            value_proposition: proposition.get(),
        };
        if app.run(repo().map_create_product(input)).is_some() {
            name.set(String::new());
            proposition.set(String::new());
            app.notify("Produit créé.");
            open.set(false);
        }
    };
    view! {
        <form class="form-grid" on:submit=submit>
            <label class="field">
                <span>"Nom"</span>
                <input placeholder="Parcours Souscription" prop:value=move || name.get()
                    on:input=move |ev| name.set(input_value(&ev)) />
            </label>
            <label class="field">
                <span>"Type de produit"</span>
                <select on:change=move |ev| kind.set(input_value(&ev))>
                    {ProductKind::ALL.iter().map(|value| view! {
                        <option value=value.as_str()>{value.label()}</option>
                    }).collect_view()}
                </select>
            </label>
            <label class="field">
                <span>"Équipe accountable"</span>
                <select on:change=move |ev| team.set(input_value(&ev))>
                    {teams.clone().into_iter().map(|(id, label)| view! {
                        <option value=id>{label}</option>
                    }).collect_view()}
                </select>
            </label>
            <label class="field">
                <span>"Value stream principal"</span>
                <select prop:value=move || value_stream.get() on:change=move |ev| value_stream.set(input_value(&ev))>
                    <option value="">"Non rattaché"</option>
                    {value_streams.into_iter().map(|(id, label)| view! {
                        <option value=id>{label}</option>
                    }).collect_view()}
                </select>
            </label>
            <label class="field">
                <span>"Criticité"</span>
                <select on:change=move |ev| criticality.set(input_value(&ev))>
                    {Criticality::ALL.iter().map(|value| view! {
                        <option value=value.as_str() selected=*value == Criticality::Medium>{value.label()}</option>
                    }).collect_view()}
                </select>
            </label>
            <label class="field field-wide">
                <span>"Proposition de valeur"</span>
                <input placeholder="Souscrire sans friction, en moins de dix minutes." prop:value=move || proposition.get()
                    on:input=move |ev| proposition.set(input_value(&ev)) />
            </label>
            <div class="actions field-wide">
                <button class="btn primary" type="submit">"Créer le produit"</button>
            </div>
        </form>
    }.into_any()
}

fn application_form(
    app: App,
    teams: Vec<(String, String)>,
    nodes: Vec<(String, String)>,
    open: RwSignal<bool>,
) -> AnyView {
    let name = RwSignal::new(String::new());
    let kind = RwSignal::new(ApplicationKind::Internal.as_str().to_string());
    let criticality = RwSignal::new(Criticality::Medium.as_str().to_string());
    let node = RwSignal::new(String::new());
    if teams.is_empty() {
        return view! {
            <div class="callout">"Une application a exactement une équipe opératrice (PRD-009). Créez d'abord une équipe."</div>
        }
        .into_any();
    }
    let team = RwSignal::new(teams[0].0.clone());
    let submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let node_id = crate::state::parse_object_id(&node.get());
        let input = NewApplication {
            name: name.get(),
            kind: ApplicationKind::parse(&kind.get()).unwrap_or_default(),
            criticality: Criticality::parse(&criticality.get()).unwrap_or_default(),
            lifecycle: LifecycleState::Run,
            operating_team_id: crate::state::parse_object_id(&team.get()),
            node_ids: node_id.into_iter().collect(),
        };
        if app.run(repo().map_create_application(input)).is_some() {
            name.set(String::new());
            app.notify("Application créée.");
            open.set(false);
        }
    };
    view! {
        <form class="form-grid" on:submit=submit>
            <label class="field">
                <span>"Nom"</span>
                <input placeholder="Payment Hub" prop:value=move || name.get()
                    on:input=move |ev| name.set(input_value(&ev)) />
            </label>
            <label class="field">
                <span>"Type"</span>
                <select on:change=move |ev| kind.set(input_value(&ev))>
                    {ApplicationKind::ALL.iter().map(|value| view! {
                        <option value=value.as_str()>{value.label()}</option>
                    }).collect_view()}
                </select>
            </label>
            <label class="field">
                <span>"Équipe opératrice"</span>
                <select on:change=move |ev| team.set(input_value(&ev))>
                    {teams.into_iter().map(|(id, label)| view! {
                        <option value=id>{label}</option>
                    }).collect_view()}
                </select>
            </label>
            <label class="field">
                <span>"Nœud de déploiement"</span>
                <select on:change=move |ev| node.set(input_value(&ev))>
                    <option value="">"Aucun (anomalie PRD-022)"</option>
                    {nodes.into_iter().map(|(id, label)| view! {
                        <option value=id>{label}</option>
                    }).collect_view()}
                </select>
            </label>
            <label class="field">
                <span>"Criticité"</span>
                <select on:change=move |ev| criticality.set(input_value(&ev))>
                    {Criticality::ALL.iter().map(|value| view! {
                        <option value=value.as_str() selected=*value == Criticality::Medium>{value.label()}</option>
                    }).collect_view()}
                </select>
            </label>
            <div class="actions field-wide">
                <button class="btn primary" type="submit">"Créer l'application"</button>
            </div>
        </form>
    }.into_any()
}

fn interface_form(app: App, providers: Vec<(String, String)>, open: RwSignal<bool>) -> AnyView {
    let name = RwSignal::new(String::new());
    let level = RwSignal::new(InterfaceLevel::Application.as_str().to_string());
    let kind = RwSignal::new(InterfaceKind::Api.as_str().to_string());
    let exposure = RwSignal::new(InterfaceExposure::Product.as_str().to_string());
    let criticality = RwSignal::new(Criticality::High.as_str().to_string());
    let provider = RwSignal::new(
        providers
            .first()
            .map(|(id, _)| id.clone())
            .unwrap_or_default(),
    );
    let version = RwSignal::new("1.0.0".to_string());
    if providers.is_empty() {
        return view! {
            <div class="callout">"Une interface a un fournisseur : créez d'abord un produit, une application ou un nœud."</div>
        }
        .into_any();
    }
    let submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let provider_id = match crate::state::parse_object_id(&provider.get()) {
            Some(id) => id,
            None => {
                app.notify("Choisissez un fournisseur.");
                return;
            }
        };
        let provider_product_id = repo()
            .map_data()
            .products
            .iter()
            .find(|product| product.id == provider_id)
            .map(|product| product.id);
        let input = NewInterface {
            name: name.get(),
            level: InterfaceLevel::parse(&level.get()).unwrap_or_default(),
            kind: InterfaceKind::parse(&kind.get()).unwrap_or_default(),
            exposure: InterfaceExposure::parse(&exposure.get()).unwrap_or_default(),
            criticality: Criticality::parse(&criticality.get()).unwrap_or_default(),
            provider_object_id: provider_id,
            provider_product_id,
            data_classification: Some(DataClassification::Internal),
        };
        if let Some(id) = app.run(repo().map_create_interface(input)) {
            let version_text = version.get();
            if !version_text.trim().is_empty() {
                app.run(repo().map_create_interface_version(NewVersion {
                    interface_id: id,
                    semantic_version: version_text,
                    available_from: Some("2026-01-01".into()),
                    available_until: None,
                    breaking_change: false,
                }));
            }
            name.set(String::new());
            app.notify("Interface créée avec sa première version.");
            open.set(false);
        }
    };
    view! {
        <form class="form-grid" on:submit=submit>
            <label class="field">
                <span>"Nom"</span>
                <input placeholder="Payment API" prop:value=move || name.get()
                    on:input=move |ev| name.set(input_value(&ev)) />
            </label>
            <label class="field">
                <span>"Fournisseur"</span>
                <select on:change=move |ev| provider.set(input_value(&ev))>
                    {providers.into_iter().map(|(id, label)| view! {
                        <option value=id>{label}</option>
                    }).collect_view()}
                </select>
            </label>
            <label class="field">
                <span>"Niveau"</span>
                <select on:change=move |ev| level.set(input_value(&ev))>
                    {InterfaceLevel::ALL.iter().map(|value| view! {
                        <option value=value.as_str() selected=*value == InterfaceLevel::Application>{value.label()}</option>
                    }).collect_view()}
                </select>
            </label>
            <label class="field">
                <span>"Type"</span>
                <select on:change=move |ev| kind.set(input_value(&ev))>
                    {InterfaceKind::ALL.iter().map(|value| view! {
                        <option value=value.as_str()>{value.label()}</option>
                    }).collect_view()}
                </select>
            </label>
            <label class="field">
                <span>"Exposition"</span>
                <select on:change=move |ev| exposure.set(input_value(&ev))>
                    {InterfaceExposure::ALL.iter().map(|value| view! {
                        <option value=value.as_str() selected=*value == InterfaceExposure::Product>{value.label()}</option>
                    }).collect_view()}
                </select>
            </label>
            <label class="field">
                <span>"Version initiale"</span>
                <input placeholder="1.0.0" prop:value=move || version.get()
                    on:input=move |ev| version.set(input_value(&ev)) />
            </label>
            <label class="field">
                <span>"Criticité"</span>
                <select on:change=move |ev| criticality.set(input_value(&ev))>
                    {Criticality::ALL.iter().map(|value| view! {
                        <option value=value.as_str() selected=*value == Criticality::High>{value.label()}</option>
                    }).collect_view()}
                </select>
            </label>
            <div class="actions field-wide">
                <button class="btn primary" type="submit">"Créer l'interface"</button>
            </div>
        </form>
    }.into_any()
}

fn capability_form(app: App, open: RwSignal<bool>) -> AnyView {
    let name = RwSignal::new(String::new());
    let criticality = RwSignal::new(Criticality::Medium.as_str().to_string());
    let submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let input = NewCapability {
            name: name.get(),
            parent_id: None,
            criticality: Criticality::parse(&criticality.get()).unwrap_or_default(),
        };
        if app.run(repo().map_create_capability(input)).is_some() {
            name.set(String::new());
            app.notify("Capacité créée.");
            open.set(false);
        }
    };
    view! {
        <form class="form-grid" on:submit=submit>
            <label class="field">
                <span>"Nom"</span>
                <input placeholder="Tarification" prop:value=move || name.get()
                    on:input=move |ev| name.set(input_value(&ev)) />
            </label>
            <label class="field">
                <span>"Criticité"</span>
                <select on:change=move |ev| criticality.set(input_value(&ev))>
                    {Criticality::ALL.iter().map(|value| view! {
                        <option value=value.as_str() selected=*value == Criticality::Medium>{value.label()}</option>
                    }).collect_view()}
                </select>
            </label>
            <div class="actions field-wide">
                <button class="btn primary" type="submit">"Créer la capacité"</button>
            </div>
        </form>
    }.into_any()
}

fn node_form(app: App, teams: Vec<(String, String)>, open: RwSignal<bool>) -> AnyView {
    let name = RwSignal::new(String::new());
    let kind = RwSignal::new(TechnologyNodeKind::Cluster.as_str().to_string());
    let environment = RwSignal::new(Environment::Prod.as_str().to_string());
    let team = RwSignal::new(teams.first().map(|(id, _)| id.clone()).unwrap_or_default());
    let submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let input = NewNode {
            name: name.get(),
            kind: TechnologyNodeKind::parse(&kind.get()).unwrap_or_default(),
            environment: Environment::parse(&environment.get()).unwrap_or_default(),
            provider_team_id: crate::state::parse_object_id(&team.get()),
        };
        if app.run(repo().map_create_node(input)).is_some() {
            name.set(String::new());
            app.notify("Nœud créé.");
            open.set(false);
        }
    };
    view! {
        <form class="form-grid" on:submit=submit>
            <label class="field">
                <span>"Nom"</span>
                <input placeholder="Kubernetes Production" prop:value=move || name.get()
                    on:input=move |ev| name.set(input_value(&ev)) />
            </label>
            <label class="field">
                <span>"Type"</span>
                <select on:change=move |ev| kind.set(input_value(&ev))>
                    {TechnologyNodeKind::ALL.iter().map(|value| view! {
                        <option value=value.as_str() selected=*value == TechnologyNodeKind::Cluster>{value.label()}</option>
                    }).collect_view()}
                </select>
            </label>
            <label class="field">
                <span>"Environnement"</span>
                <select on:change=move |ev| environment.set(input_value(&ev))>
                    {Environment::ALL.iter().map(|value| view! {
                        <option value=value.as_str() selected=*value == Environment::Prod>{value.label()}</option>
                    }).collect_view()}
                </select>
            </label>
            <label class="field">
                <span>"Équipe fournisseur"</span>
                <select on:change=move |ev| team.set(input_value(&ev))>
                    <option value="">"Aucune"</option>
                    {teams.into_iter().map(|(id, label)| view! {
                        <option value=id>{label}</option>
                    }).collect_view()}
                </select>
            </label>
            <div class="actions field-wide">
                <button class="btn primary" type="submit">"Créer le nœud"</button>
            </div>
        </form>
    }.into_any()
}

/// Regroupe les créations liées dans une seule carte, repliée par défaut.
///
/// Une fiche ne doit pas ressembler à un formulaire géant : les gestes de
/// création sont proposés comme des commandes, et le formulaire n'apparaît
/// qu'après un choix explicite.
#[component]
pub fn CreateMenu(children: Children) -> impl IntoView {
    view! {
        <section class="card create-menu" aria-labelledby="create-menu-title">
            <header class="card-head create-menu-head">
                <div>
                    <h2 id="create-menu-title">"Créer et lier"</h2>
                    <p class="muted small">
                        "Ajoutez un objet directement rattaché à cette fiche."
                    </p>
                </div>
            </header>
            <div class="card-body create-menu-body">
                {children()}
            </div>
        </section>
    }
}

/// Une commande de création repliée. Le résumé est le bouton ; le formulaire
/// n'occupe de la place qu'une fois ouvert.
#[component]
pub fn CreateSection(
    title: String,
    #[prop(optional)] description: Option<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <details class="create-section">
            <summary>
                <span class="create-section-icon" aria-hidden="true">"+"</span>
                <span class="create-section-label">{title}</span>
                <span class="create-section-chevron" aria-hidden="true">"▾"</span>
            </summary>
            <div class="create-section-body">
                {description.map(|description| view! {
                    <p class="create-section-help">{description}</p>
                })}
                {children()}
            </div>
        </details>
    }
}

fn team_choices() -> Vec<(String, String)> {
    repo()
        .map_data()
        .teams
        .iter()
        .map(|team| (team.id.to_string(), team.name.clone()))
        .collect()
}

fn value_stream_choices() -> Vec<(String, String)> {
    repo()
        .map_data()
        .value_streams
        .iter()
        .map(|stream| (stream.id.to_string(), stream.name.clone()))
        .collect()
}

/// Crée une équipe, puis la rattache éventuellement à l'objet courant.
///
/// Le geste reste le même partout : une équipe est la seule source de
/// responsabilité, donc sa création doit pouvoir se faire depuis la fiche qui
/// en a besoin.
#[component]
pub fn TeamCreate(
    app: App,
    #[prop(optional)] product: Option<ObjectId>,
    #[prop(optional)] application: Option<ObjectId>,
    #[prop(optional)] value_stream: Option<ObjectId>,
    #[prop(optional)] node: Option<ObjectId>,
) -> impl IntoView {
    let name = RwSignal::new(String::new());
    let topology = RwSignal::new(TeamTopology::StreamAligned.as_str().to_string());
    let mission = RwSignal::new(String::new());
    let submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let input = NewTeam {
            name: name.get(),
            topology: TeamTopology::parse(&topology.get()).unwrap_or_default(),
            mission: mission.get(),
        };
        let Some(team_id) = app.run(repo().map_create_team(input)) else {
            return;
        };
        let mut linked = false;
        if let Some(product_id) = product {
            linked |= app
                .run(repo().map_set_product_accountable_team(product_id, Some(team_id)))
                .is_some();
        }
        if let Some(application_id) = application {
            linked |= app
                .run(repo().map_set_application_operating_team(application_id, Some(team_id)))
                .is_some();
        }
        if let Some(value_stream_id) = value_stream {
            linked |= app
                .run(repo().map_set_value_stream_owner(value_stream_id, Some(team_id)))
                .is_some();
        }
        if let Some(node_id) = node {
            linked |= app
                .run(repo().map_set_node_provider_team(node_id, Some(team_id)))
                .is_some();
        }
        name.set(String::new());
        mission.set(String::new());
        app.notify(if linked {
            "Équipe créée et rattachée."
        } else {
            "Équipe créée."
        });
    };
    view! {
        <form class="form-grid" on:submit=submit>
            <label class="field">
                <span>"Nom"</span>
                <input placeholder="Équipe Paiements" prop:value=move || name.get()
                    on:input=move |ev| name.set(input_value(&ev)) />
            </label>
            <label class="field">
                <span>"Type d'équipe"</span>
                <select on:change=move |ev| topology.set(input_value(&ev))>
                    {TeamTopology::ALL.iter().map(|value| view! {
                        <option value=value.as_str() selected=*value == TeamTopology::StreamAligned>{value.label()}</option>
                    }).collect_view()}
                </select>
            </label>
            <label class="field field-wide">
                <span>"Mission"</span>
                <input placeholder="Rendre la souscription simple et rapide." prop:value=move || mission.get()
                    on:input=move |ev| mission.set(input_value(&ev)) />
            </label>
            <div class="actions field-wide">
                <button class="btn primary" type="submit">"Créer l'équipe"</button>
            </div>
        </form>
    }
}

/// Crée un produit et le rattache directement au contexte de la fiche :
/// équipe, value stream, capacité, application, domaine ou étape.
#[component]
pub fn ProductCreate(
    app: App,
    #[prop(optional)] team: Option<ObjectId>,
    #[prop(optional)] value_stream: Option<ObjectId>,
    #[prop(optional)] capability: Option<ObjectId>,
    #[prop(optional)] application: Option<ObjectId>,
    #[prop(optional)] domain: Option<ObjectId>,
    #[prop(optional)] stage: Option<ObjectId>,
    #[prop(optional)] consume_version: Option<ObjectId>,
) -> impl IntoView {
    let teams = team_choices();
    if teams.is_empty() {
        return view! {
            <div class="callout">
                <strong>"Une équipe accountable est requise."</strong>
                <p>"Un produit a exactement une équipe responsable (PRD-001). Créez-la ici, puis le produit."</p>
                <TeamCreate app=app />
            </div>
        }
        .into_any();
    }
    let default_team = team
        .map(|id| id.to_string())
        .unwrap_or_else(|| teams[0].0.clone());
    let streams = value_stream_choices();
    let default_stream = value_stream.map(|id| id.to_string()).unwrap_or_default();
    let name = RwSignal::new(String::new());
    let kind = RwSignal::new(ProductKind::Customer.as_str().to_string());
    let criticality = RwSignal::new(Criticality::Medium.as_str().to_string());
    let team_choice = RwSignal::new(default_team);
    let stream_choice = RwSignal::new(default_stream);
    let proposition = RwSignal::new(String::new());
    let submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let input = NewProduct {
            name: name.get(),
            kind: ProductKind::parse(&kind.get()).unwrap_or_default(),
            criticality: Criticality::parse(&criticality.get()).unwrap_or_default(),
            lifecycle: LifecycleState::Discovery,
            accountable_team_id: parse_object_id(&team_choice.get()).or(team),
            primary_value_stream_id: parse_object_id(&stream_choice.get()).or(value_stream),
            primary_portfolio_id: None,
            business_domain_ids: domain.into_iter().collect(),
            application_ids: application.into_iter().collect(),
            capability_ids: capability.into_iter().collect(),
            value_proposition: proposition.get(),
        };
        let Some(product_id) = app.run(repo().map_create_product(input)) else {
            return;
        };
        if let Some(stage_id) = stage {
            app.run(repo().map_add_product_to_stage(stage_id, product_id));
        }
        if let Some(version_id) = consume_version {
            app.run(repo().map_add_interface_consumption(
                version_id,
                product_id,
                Criticality::Medium,
            ));
        }
        name.set(String::new());
        proposition.set(String::new());
        app.notify("Produit créé et rattaché.");
    };
    view! {
        <form class="form-grid" on:submit=submit>
            <label class="field">
                <span>"Nom"</span>
                <input placeholder="Parcours Souscription" prop:value=move || name.get()
                    on:input=move |ev| name.set(input_value(&ev)) />
            </label>
            <label class="field">
                <span>"Type de produit"</span>
                <select on:change=move |ev| kind.set(input_value(&ev))>
                    {ProductKind::ALL.iter().map(|value| view! {
                        <option value=value.as_str()>{value.label()}</option>
                    }).collect_view()}
                </select>
            </label>
            <label class="field">
                <span>"Équipe accountable"</span>
                <select prop:value=move || team_choice.get() on:change=move |ev| team_choice.set(input_value(&ev))>
                    {teams.into_iter().map(|(id, label)| view! {
                        <option value=id>{label}</option>
                    }).collect_view()}
                </select>
            </label>
            {value_stream.is_none().then(move || view! {
                <label class="field">
                    <span>"Value stream principal"</span>
                    <select prop:value=move || stream_choice.get() on:change=move |ev| stream_choice.set(input_value(&ev))>
                        <option value="">"- Hors value stream -"</option>
                        {streams.into_iter().map(|(id, label)| view! {
                            <option value=id>{label}</option>
                        }).collect_view()}
                    </select>
                </label>
            })}
            <label class="field">
                <span>"Criticité"</span>
                <select on:change=move |ev| criticality.set(input_value(&ev))>
                    {Criticality::ALL.iter().map(|value| view! {
                        <option value=value.as_str() selected=*value == Criticality::Medium>{value.label()}</option>
                    }).collect_view()}
                </select>
            </label>
            <label class="field field-wide">
                <span>"Proposition de valeur"</span>
                <input placeholder="Souscrire sans friction." prop:value=move || proposition.get()
                    on:input=move |ev| proposition.set(input_value(&ev)) />
            </label>
            <div class="actions field-wide">
                <button class="btn primary" type="submit">"Créer le produit lié"</button>
            </div>
        </form>
    }
    .into_any()
}

/// Crée une application, puis la rattache au produit, au nœud ou à l'interface
/// depuis lesquels l'utilisateur agit.
#[component]
pub fn ApplicationCreate(
    app: App,
    #[prop(optional)] team: Option<ObjectId>,
    #[prop(optional)] product: Option<ObjectId>,
    #[prop(optional)] node: Option<ObjectId>,
    #[prop(optional)] consume_version: Option<ObjectId>,
) -> impl IntoView {
    let teams = team_choices();
    if teams.is_empty() {
        return view! {
            <div class="callout">
                <strong>"Une équipe opératrice est requise."</strong>
                <p>"Une application a exactement une équipe opératrice (PRD-009). Créez-la ici, puis l'application."</p>
                <TeamCreate app=app />
            </div>
        }
        .into_any();
    }
    let default_team = team
        .map(|id| id.to_string())
        .unwrap_or_else(|| teams[0].0.clone());
    let nodes: Vec<(String, String)> = repo()
        .map_data()
        .nodes
        .iter()
        .map(|item| (item.id.to_string(), item.name.clone()))
        .collect();
    let default_node = node.map(|id| id.to_string()).unwrap_or_default();
    let name = RwSignal::new(String::new());
    let kind = RwSignal::new(ApplicationKind::Internal.as_str().to_string());
    let criticality = RwSignal::new(Criticality::Medium.as_str().to_string());
    let team_choice = RwSignal::new(default_team);
    let node_choice = RwSignal::new(default_node);
    let submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let input = NewApplication {
            name: name.get(),
            kind: ApplicationKind::parse(&kind.get()).unwrap_or_default(),
            criticality: Criticality::parse(&criticality.get()).unwrap_or_default(),
            lifecycle: LifecycleState::Run,
            operating_team_id: parse_object_id(&team_choice.get()).or(team),
            node_ids: parse_object_id(&node_choice.get())
                .or(node)
                .into_iter()
                .collect(),
        };
        let Some(application_id) = app.run(repo().map_create_application(input)) else {
            return;
        };
        if let Some(product_id) = product {
            app.run(repo().map_add_application_to_product(product_id, application_id));
        }
        if let Some(version_id) = consume_version {
            app.run(repo().map_add_interface_consumption(
                version_id,
                application_id,
                Criticality::Medium,
            ));
        }
        name.set(String::new());
        app.notify("Application créée et rattachée.");
    };
    view! {
        <form class="form-grid" on:submit=submit>
            <label class="field">
                <span>"Nom"</span>
                <input placeholder="Payment Hub" prop:value=move || name.get()
                    on:input=move |ev| name.set(input_value(&ev)) />
            </label>
            <label class="field">
                <span>"Type"</span>
                <select on:change=move |ev| kind.set(input_value(&ev))>
                    {ApplicationKind::ALL.iter().map(|value| view! {
                        <option value=value.as_str()>{value.label()}</option>
                    }).collect_view()}
                </select>
            </label>
            <label class="field">
                <span>"Équipe opératrice"</span>
                <select prop:value=move || team_choice.get() on:change=move |ev| team_choice.set(input_value(&ev))>
                    {teams.into_iter().map(|(id, label)| view! {
                        <option value=id>{label}</option>
                    }).collect_view()}
                </select>
            </label>
            {node.is_none().then(move || view! {
                <label class="field">
                    <span>"Nœud de déploiement"</span>
                    <select prop:value=move || node_choice.get() on:change=move |ev| node_choice.set(input_value(&ev))>
                        <option value="">"Aucun (anomalie PRD-022)"</option>
                        {nodes.into_iter().map(|(id, label)| view! {
                            <option value=id>{label}</option>
                        }).collect_view()}
                    </select>
                </label>
            })}
            <label class="field">
                <span>"Criticité"</span>
                <select on:change=move |ev| criticality.set(input_value(&ev))>
                    {Criticality::ALL.iter().map(|value| view! {
                        <option value=value.as_str() selected=*value == Criticality::Medium>{value.label()}</option>
                    }).collect_view()}
                </select>
            </label>
            <div class="actions field-wide">
                <button class="btn primary" type="submit">"Créer l'application liée"</button>
            </div>
        </form>
    }.into_any()
}

/// Crée une capacité et la rattache au produit, à l'étape ou à la capacité
/// parente depuis laquelle elle est créée.
#[component]
pub fn CapabilityCreate(
    app: App,
    #[prop(optional)] product: Option<ObjectId>,
    #[prop(optional)] stage: Option<ObjectId>,
    #[prop(optional)] parent: Option<ObjectId>,
    #[prop(optional)] value_stream: Option<ObjectId>,
) -> impl IntoView {
    let capabilities: Vec<(String, String)> = repo()
        .map_data()
        .capabilities
        .iter()
        .map(|capability| (capability.id.to_string(), capability.name.clone()))
        .collect();
    let stages: Vec<(String, String)> = value_stream
        .map(|stream_id| {
            repo()
                .map_data()
                .stages_of(stream_id)
                .into_iter()
                .map(|stage| (stage.id.to_string(), stage.name.clone()))
                .collect()
        })
        .unwrap_or_default();
    let default_parent = parent.map(|id| id.to_string()).unwrap_or_default();
    let default_stage = stage
        .map(|id| id.to_string())
        .unwrap_or_else(|| stages.first().map(|(id, _)| id.clone()).unwrap_or_default());
    let name = RwSignal::new(String::new());
    let criticality = RwSignal::new(Criticality::Medium.as_str().to_string());
    let parent_choice = RwSignal::new(default_parent);
    let stage_choice = RwSignal::new(default_stage);
    let submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let input = NewCapability {
            name: name.get(),
            parent_id: parse_object_id(&parent_choice.get()).or(parent),
            criticality: Criticality::parse(&criticality.get()).unwrap_or_default(),
        };
        let Some(capability_id) = app.run(repo().map_create_capability(input)) else {
            return;
        };
        if let Some(product_id) = product {
            app.run(repo().map_add_capability_to_product(product_id, capability_id));
        }
        let linked_stage = stage.or_else(|| parse_object_id(&stage_choice.get()));
        if let Some(stage_id) = linked_stage {
            app.run(repo().map_add_capability_to_stage(stage_id, capability_id));
        }
        name.set(String::new());
        app.notify("Capacité créée et rattachée.");
    };
    view! {
        <form class="form-grid" on:submit=submit>
            <label class="field">
                <span>"Nom"</span>
                <input placeholder="Tarification" prop:value=move || name.get()
                    on:input=move |ev| name.set(input_value(&ev)) />
            </label>
            <label class="field">
                <span>"Criticité"</span>
                <select on:change=move |ev| criticality.set(input_value(&ev))>
                    {Criticality::ALL.iter().map(|value| view! {
                        <option value=value.as_str() selected=*value == Criticality::Medium>{value.label()}</option>
                    }).collect_view()}
                </select>
            </label>
            {(parent.is_none() && !capabilities.is_empty()).then(move || view! {
                <label class="field">
                    <span>"Capacité parente"</span>
                    <select prop:value=move || parent_choice.get() on:change=move |ev| parent_choice.set(input_value(&ev))>
                        <option value="">"Aucune (niveau 1)"</option>
                        {capabilities.into_iter().map(|(id, label)| view! {
                            <option value=id>{label}</option>
                        }).collect_view()}
                    </select>
                </label>
            })}
            {(stage.is_none() && !stages.is_empty()).then(move || view! {
                <label class="field">
                    <span>"Étape du value stream"</span>
                    <select prop:value=move || stage_choice.get() on:change=move |ev| stage_choice.set(input_value(&ev))>
                        {stages.into_iter().map(|(id, label)| view! {
                            <option value=id>{label}</option>
                        }).collect_view()}
                    </select>
                </label>
            })}
            <div class="actions field-wide">
                <button class="btn primary" type="submit">"Créer la capacité liée"</button>
            </div>
        </form>
    }
}

/// Crée un value stream et le rattache à un produit ou à une équipe
/// propriétaire.
#[component]
pub fn ValueStreamCreate(
    app: App,
    #[prop(optional)] product: Option<ObjectId>,
    #[prop(optional)] team: Option<ObjectId>,
) -> impl IntoView {
    let teams = team_choices();
    let default_team = team
        .or_else(|| {
            product.and_then(|product_id| {
                repo()
                    .map_data()
                    .product(product_id)
                    .and_then(|product| product.accountable_team_id)
            })
        })
        .map(|id| id.to_string())
        .unwrap_or_default();
    let name = RwSignal::new(String::new());
    let proposition = RwSignal::new(String::new());
    let stages = RwSignal::new(String::new());
    let owner = RwSignal::new(default_team);
    let submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let input = NewValueStream {
            name: name.get(),
            value_proposition: proposition.get(),
            stages: stages
                .get()
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_string)
                .collect(),
            owner_team_id: parse_object_id(&owner.get()).or(team),
        };
        let Some(stream_id) = app.run(repo().map_create_value_stream(input)) else {
            return;
        };
        if let Some(product_id) = product {
            app.run(repo().map_set_product_value_stream(product_id, Some(stream_id)));
        }
        name.set(String::new());
        proposition.set(String::new());
        stages.set(String::new());
        app.notify("Value stream créé et rattaché.");
    };
    view! {
        <form class="form-grid" on:submit=submit>
            <label class="field">
                <span>"Nom"</span>
                <input placeholder="Souscrire un contrat" prop:value=move || name.get()
                    on:input=move |ev| name.set(input_value(&ev)) />
            </label>
            {team.is_none().then(move || view! {
                <label class="field">
                    <span>"Équipe propriétaire"</span>
                    <select prop:value=move || owner.get() on:change=move |ev| owner.set(input_value(&ev))>
                        <option value="">"Aucune"</option>
                        {teams.into_iter().map(|(id, label)| view! {
                            <option value=id>{label}</option>
                        }).collect_view()}
                    </select>
                </label>
            })}
            <label class="field field-wide">
                <span>"Proposition de valeur"</span>
                <input placeholder="Un contrat en moins de dix minutes." prop:value=move || proposition.get()
                    on:input=move |ev| proposition.set(input_value(&ev)) />
            </label>
            <label class="field field-wide">
                <span>"Étapes, une par ligne"</span>
                <textarea rows="3" prop:value=move || stages.get()
                    on:input=move |ev| stages.set(input_value(&ev))></textarea>
            </label>
            <div class="actions field-wide">
                <button class="btn primary" type="submit">"Créer le value stream lié"</button>
            </div>
        </form>
    }
}

/// Ajoute une étape au value stream courant.
#[component]
pub fn StageCreate(app: App, value_stream: ObjectId) -> impl IntoView {
    let name = RwSignal::new(String::new());
    let value_item = RwSignal::new(String::new());
    let submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        if app
            .run(repo().map_create_stage(NewStage {
                value_stream_id: value_stream,
                name: name.get(),
                value_item: value_item.get(),
            }))
            .is_some()
        {
            name.set(String::new());
            value_item.set(String::new());
            app.notify("Étape ajoutée au value stream.");
        }
    };
    view! {
        <form class="form-grid" on:submit=submit>
            <label class="field">
                <span>"Nom de l'étape"</span>
                <input placeholder="Obtenir une tarification" prop:value=move || name.get()
                    on:input=move |ev| name.set(input_value(&ev)) />
            </label>
            <label class="field">
                <span>"Valeur produite"</span>
                <input placeholder="Un prix juste" prop:value=move || value_item.get()
                    on:input=move |ev| value_item.set(input_value(&ev)) />
            </label>
            <div class="actions field-wide">
                <button class="btn primary" type="submit">"Ajouter l'étape"</button>
            </div>
        </form>
    }
}

/// Crée une interface fournie par l'objet courant, avec sa première version.
#[component]
pub fn InterfaceCreate(app: App, provider: ObjectId, level: InterfaceLevel) -> impl IntoView {
    let name = RwSignal::new(String::new());
    let kind = RwSignal::new(InterfaceKind::Api.as_str().to_string());
    let exposure = RwSignal::new(
        if level == InterfaceLevel::Infrastructure {
            InterfaceExposure::Internal.as_str()
        } else {
            InterfaceExposure::Product.as_str()
        }
        .to_string(),
    );
    let criticality = RwSignal::new(Criticality::High.as_str().to_string());
    let version = RwSignal::new("1.0.0".to_string());
    let submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let provider_product_id = repo()
            .map_data()
            .product(provider)
            .map(|product| product.id);
        let input = NewInterface {
            name: name.get(),
            level,
            kind: InterfaceKind::parse(&kind.get()).unwrap_or_default(),
            exposure: InterfaceExposure::parse(&exposure.get()).unwrap_or_default(),
            criticality: Criticality::parse(&criticality.get()).unwrap_or_default(),
            provider_object_id: provider,
            provider_product_id,
            data_classification: Some(DataClassification::Internal),
        };
        let Some(interface_id) = app.run(repo().map_create_interface(input)) else {
            return;
        };
        let version_text = version.get();
        if !version_text.trim().is_empty() {
            app.run(repo().map_create_interface_version(NewVersion {
                interface_id,
                semantic_version: version_text,
                available_from: Some("2026-01-01".into()),
                available_until: None,
                breaking_change: false,
            }));
        }
        name.set(String::new());
        app.notify("Interface créée et rattachée avec sa première version.");
    };
    let exposure_values: Vec<InterfaceExposure> = if level == InterfaceLevel::Infrastructure {
        vec![InterfaceExposure::Internal]
    } else {
        InterfaceExposure::ALL.to_vec()
    };
    view! {
        <form class="form-grid" on:submit=submit>
            <label class="field">
                <span>"Nom"</span>
                <input placeholder="Payment API" prop:value=move || name.get()
                    on:input=move |ev| name.set(input_value(&ev)) />
            </label>
            <label class="field">
                <span>"Type"</span>
                <select on:change=move |ev| kind.set(input_value(&ev))>
                    {InterfaceKind::ALL.iter().map(|value| view! {
                        <option value=value.as_str()>{value.label()}</option>
                    }).collect_view()}
                </select>
            </label>
            <label class="field">
                <span>"Exposition"</span>
                <select on:change=move |ev| exposure.set(input_value(&ev))>
                    {exposure_values.into_iter().map(|value| view! {
                        <option value=value.as_str() selected=value.as_str() == exposure.get_untracked()>{value.label()}</option>
                    }).collect_view()}
                </select>
            </label>
            <label class="field">
                <span>"Version initiale"</span>
                <input placeholder="1.0.0" prop:value=move || version.get()
                    on:input=move |ev| version.set(input_value(&ev)) />
            </label>
            <label class="field">
                <span>"Criticité"</span>
                <select on:change=move |ev| criticality.set(input_value(&ev))>
                    {Criticality::ALL.iter().map(|value| view! {
                        <option value=value.as_str() selected=*value == Criticality::High>{value.label()}</option>
                    }).collect_view()}
                </select>
            </label>
            <div class="actions field-wide">
                <button class="btn primary" type="submit">"Créer l'interface liée"</button>
            </div>
        </form>
    }
}

/// Crée un nœud, puis le lie à une application (déploiement) ou à une
/// consommation d'interface.
#[component]
pub fn NodeCreate(
    app: App,
    #[prop(optional)] team: Option<ObjectId>,
    #[prop(optional)] application: Option<ObjectId>,
    #[prop(optional)] consume_version: Option<ObjectId>,
) -> impl IntoView {
    let teams = team_choices();
    let default_team = team
        .map(|id| id.to_string())
        .unwrap_or_else(|| teams.first().map(|(id, _)| id.clone()).unwrap_or_default());
    let name = RwSignal::new(String::new());
    let kind = RwSignal::new(TechnologyNodeKind::Cluster.as_str().to_string());
    let environment = RwSignal::new(Environment::Prod.as_str().to_string());
    let team_choice = RwSignal::new(default_team);
    let submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let input = NewNode {
            name: name.get(),
            kind: TechnologyNodeKind::parse(&kind.get()).unwrap_or_default(),
            environment: Environment::parse(&environment.get()).unwrap_or_default(),
            provider_team_id: parse_object_id(&team_choice.get()).or(team),
        };
        let Some(node_id) = app.run(repo().map_create_node(input)) else {
            return;
        };
        if let Some(application_id) = application {
            app.run(repo().map_deploy_application(application_id, node_id));
        }
        if let Some(version_id) = consume_version {
            app.run(repo().map_add_interface_consumption(version_id, node_id, Criticality::Medium));
        }
        name.set(String::new());
        app.notify("Nœud créé et rattaché.");
    };
    view! {
        <form class="form-grid" on:submit=submit>
            <label class="field">
                <span>"Nom"</span>
                <input placeholder="Kubernetes Production" prop:value=move || name.get()
                    on:input=move |ev| name.set(input_value(&ev)) />
            </label>
            <label class="field">
                <span>"Type"</span>
                <select on:change=move |ev| kind.set(input_value(&ev))>
                    {TechnologyNodeKind::ALL.iter().map(|value| view! {
                        <option value=value.as_str() selected=*value == TechnologyNodeKind::Cluster>{value.label()}</option>
                    }).collect_view()}
                </select>
            </label>
            <label class="field">
                <span>"Environnement"</span>
                <select on:change=move |ev| environment.set(input_value(&ev))>
                    {Environment::ALL.iter().map(|value| view! {
                        <option value=value.as_str() selected=*value == Environment::Prod>{value.label()}</option>
                    }).collect_view()}
                </select>
            </label>
            <label class="field">
                <span>"Équipe fournisseur"</span>
                <select prop:value=move || team_choice.get() on:change=move |ev| team_choice.set(input_value(&ev))>
                    <option value="">"Aucune"</option>
                    {teams.into_iter().map(|(id, label)| view! {
                        <option value=id>{label}</option>
                    }).collect_view()}
                </select>
            </label>
            <div class="actions field-wide">
                <button class="btn primary" type="submit">"Créer le nœud lié"</button>
            </div>
        </form>
    }
}

/// Crée un domaine métier et classe éventuellement le produit courant dedans.
#[component]
pub fn BusinessDomainCreate(
    app: App,
    #[prop(optional)] product: Option<ObjectId>,
    #[prop(optional)] parent: Option<ObjectId>,
) -> impl IntoView {
    let name = RwSignal::new(String::new());
    let description = RwSignal::new(String::new());
    let submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let Some(domain_id) = app.run(repo().map_create_business_domain(NewDomain {
            name: name.get(),
            description: description.get(),
            parent_id: parent,
        })) else {
            return;
        };
        if let Some(product_id) = product {
            app.run(repo().map_add_product_to_domain(product_id, domain_id));
        }
        name.set(String::new());
        description.set(String::new());
        app.notify("Domaine métier créé et rattaché.");
    };
    view! {
        <form class="form-grid" on:submit=submit>
            <label class="field">
                <span>"Nom"</span>
                <input placeholder="Tarification" prop:value=move || name.get()
                    on:input=move |ev| name.set(input_value(&ev)) />
            </label>
            <label class="field field-wide">
                <span>"Description"</span>
                <input placeholder="Connaissance du risque et du prix." prop:value=move || description.get()
                    on:input=move |ev| description.set(input_value(&ev)) />
            </label>
            <div class="actions field-wide">
                <button class="btn primary" type="submit">"Créer le domaine lié"</button>
            </div>
        </form>
    }
}

/// Composant utilitaire pour la création d'un composant depuis la page
/// application, la fiche composant ou une interface (consommateur).
#[component]
pub fn ComponentCreate(
    app: App,
    #[prop(optional)] application: Option<ObjectId>,
    #[prop(optional)] consume_version: Option<ObjectId>,
) -> impl IntoView {
    let applications: Vec<(String, String)> = repo()
        .map_data()
        .applications
        .iter()
        .map(|item| (item.id.to_string(), item.name.clone()))
        .collect();
    if application.is_none() && applications.is_empty() {
        return view! {
            <div class="callout">
                <strong>"Une application est requise."</strong>
                <p>"Un composant appartient à une application. Créez d'abord une application, puis son premier composant."</p>
                <a class="btn small primary" href="#/map">"Créer une application depuis la carte"</a>
            </div>
        }
        .into_any();
    }
    let default_application = application.map(|id| id.to_string()).unwrap_or_else(|| {
        applications
            .first()
            .map(|(id, _)| id.clone())
            .unwrap_or_default()
    });
    let name = RwSignal::new(String::new());
    let kind = RwSignal::new(ComponentKind::Component.as_str().to_string());
    let application_choice = RwSignal::new(default_application);
    let submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let Some(application_id) = parse_object_id(&application_choice.get()).or(application)
        else {
            app.notify("Choisissez une application.");
            return;
        };
        let input = NewComponent {
            application_id,
            name: name.get(),
            kind: ComponentKind::parse(&kind.get()).unwrap_or_default(),
        };
        let Some(component_id) = app.run(repo().map_create_component(input)) else {
            return;
        };
        if let Some(version_id) = consume_version {
            app.run(repo().map_add_interface_consumption(
                version_id,
                component_id,
                Criticality::Medium,
            ));
        }
        name.set(String::new());
        app.notify("Composant créé et rattaché.");
    };
    view! {
        <form class="form-grid" on:submit=submit>
            <label class="field">
                <span>"Nom du composant"</span>
                <input placeholder="Paiement carte" prop:value=move || name.get()
                    on:input=move |ev| name.set(input_value(&ev)) />
            </label>
            <label class="field">
                <span>"Type de composant"</span>
                <select on:change=move |ev| kind.set(input_value(&ev))>
                    {ComponentKind::ALL.iter().map(|value| view! {
                        <option value=value.as_str()>{value.label()}</option>
                    }).collect_view()}
                </select>
            </label>
            {application.is_none().then(move || view! {
                <label class="field">
                    <span>"Application"</span>
                    <select prop:value=move || application_choice.get() on:change=move |ev| application_choice.set(input_value(&ev))>
                        {applications.into_iter().map(|(id, label)| view! {
                            <option value=id>{label}</option>
                        }).collect_view()}
                    </select>
                </label>
            })}
            <div class="actions field-wide">
                <button class="btn primary" type="submit">"Créer le composant"</button>
            </div>
        </form>
    }.into_any()
}
