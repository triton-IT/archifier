//! Réglages : diagnostic de stockage, export/import, couverture des métamodèles.

use crate::components::{Card, Empty, PageHeader};
use crate::state::{download_text, input_value, repo, storage_label, App};
use archifier_core::LinkStatus;
use leptos::prelude::*;

#[component]
pub fn Settings(app: App) -> impl IntoView {
    let stats = move || {
        let _ = app.version.get();
        repo().stats()
    };
    let invalid_links = move || {
        let _ = app.version.get();
        repo().broken_links()
    };
    let (draft, set_draft) = signal(String::new());
    let (show_import, set_show_import) = signal(false);

    let export = move || {
        let json = repo().export_json();
        download_text("archifier-snapshot.json", &json);
        app.notify("Sauvegarde téléchargée.");
    };

    let import = move || {
        let payload = draft.get();
        match repo().load_json(&payload) {
            Ok(()) => {
                app.refresh();
                app.publish_write();
                set_draft.set(String::new());
                set_show_import.set(false);
                app.notify("Sauvegarde importée.");
            }
            Err(error) => app.fail(&error),
        }
    };

    let coverage = repo()
        .metamodel()
        .coverage()
        .into_iter()
        .collect::<Vec<_>>();

    let reset = move || {
        match repo().reset_storage() {
            Ok(()) => {
                crate::state::request_empty_start();
                // Publier avant le rechargement : les autres onglets voient le
                // référentiel remis à zéro et peuvent recharger à leur tour.
                app.publish_write();
                // Le rechargement ferme la connexion courante, ce qui débloque
                // la suppression ; le drapeau garantit un démarrage vide.
                if let Some(window) = web_sys::window() {
                    let _ = window.location().reload();
                }
            }
            Err(error) => app.fail(&error),
        }
    };

    view! {
        <main class="page">
            <PageHeader
                title="Réglages".into()
                subtitle="Diagnostic du profil local, sauvegarde et couverture des métamodèles.".into()
            />

            <div class="grid two-cols">
                <Card title="Stockage".into()>
                    <dl class="props">
                        <div><dt>"Moteur"</dt><dd>{move || storage_label()}</dd></div>
                        <div><dt>"Taille du référentiel"</dt><dd>{move || format!("{} Ko", stats().persisted_bytes / 1024)}</dd></div>
                        <div><dt>"Utilisateur"</dt><dd>{move || repo().current_user()}</dd></div>
                        <div><dt>"Date de référence"</dt><dd>{move || crate::state::fmt_date(repo().now())}</dd></div>
                    </dl>
                    {move || repo().persist_error().map(|error| view! {
                        <p class="banner error inline">{error}</p>
                    })}
                    <p class="muted small">
                        "Historique conservé intégralement. IndexedDB n'est pas soumis à la limite de 5 Mo de localStorage."
                    </p>
                </Card>

                <Card title="Sauvegarde".into()>
                    <p class="muted small">
                        "Le fichier produit reprend la forme du schéma cible : il sera importable tel quel dans la version serveur."
                    </p>
                    <div class="actions">
                        <button class="btn primary" on:click=move |_| export()>"Exporter le référentiel"</button>
                        <button class="btn" on:click=move |_| set_show_import.update(|v| *v = !*v)>"Importer"</button>
                    </div>
                    {move || show_import.get().then(|| view! {
                        <div class="editor">
                            <textarea
                                rows="8"
                                placeholder="Collez ici un contenu .archifier.json"
                                prop:value=move || draft.get()
                                on:input=move |ev| set_draft.set(input_value(&ev))
                            ></textarea>
                            <div class="actions">
                                <button class="btn primary" on:click=move |_| import()>"Remplacer le référentiel"</button>
                                <button class="btn ghost" on:click=move |_| set_show_import.set(false)>"Annuler"</button>
                            </div>
                        </div>
                    })}
                </Card>
            </div>

            <Card title="Contenu du référentiel".into()>
                <div class="stat-strip">
                    {move || {
                        let s = stats();
                        view! {
                            <>
                                <span><strong>{s.domains}</strong>" domaines"</span>
                                <span><strong>{s.elements}</strong>" éléments"</span>
                                <span><strong>{s.relationships}</strong>" relations"</span>
                                <span><strong>{s.interfaces}</strong>" interfaces"</span>
                                <span><strong>{s.versions}</strong>" versions"</span>
                                <span><strong>{s.diagrams}</strong>" vues"</span>
                                <span><strong>{s.stories}</strong>" stories"</span>
                                <span><strong>{s.reviews}</strong>" revues"</span>
                                <span><strong>{s.documentations}</strong>" documents"</span>
                            </>
                        }
                    }}
                </div>
            </Card>

            <Card title="Liens invalides".into()>
                <p class="muted small">
                    "Diagnostic en lecture seule : aucun lien n'est corrigé ni supprimé automatiquement."
                </p>
                {move || {
                    let links = invalid_links();
                    if links.is_empty() {
                        return view! {
                            <Empty text="Aucun lien invalide détecté.".into() />
                        }.into_any();
                    }
                    view! {
                        <table class="table">
                            <thead>
                                <tr>
                                    <th>"Code"</th>
                                    <th>"Statut"</th>
                                    <th>"Source"</th>
                                    <th>"Cible"</th>
                                    <th>"Contexte"</th>
                                    <th>"Propriétaire"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {links.into_iter().map(|diagnostic| {
                                    let (label, badge) = match diagnostic.link.status {
                                        LinkStatus::Broken => ("Cassé", "badge expired"),
                                        LinkStatus::Archived => ("Archivé", "badge archived"),
                                        LinkStatus::Ok => ("Valide", "badge ok"),
                                    };
                                    let target_uri = diagnostic.link.target_uri.clone();
                                    let target_view = match diagnostic.target.clone() {
                                        Some(target) => view! {
                                            <a href=target.href>{target.name.clone()}</a>
                                        }.into_any(),
                                        None => view! {
                                            <span class="muted">"objet absent"</span>
                                        }.into_any(),
                                    };
                                    let owner_view = match diagnostic.owner.clone() {
                                        Some(owner) => view! {
                                            <a href=owner.href>{owner.name.clone()}</a>
                                        }.into_any(),
                                        None => view! { <span class="muted">"—"</span> }.into_any(),
                                    };
                                    view! {
                                        <tr>
                                            <td class="mono">{diagnostic.code.clone()}</td>
                                            <td><span class=badge>{label}</span></td>
                                            <td>
                                                <a href=diagnostic.source.href.clone()>
                                                    {diagnostic.source.name.clone()}
                                                </a>
                                            </td>
                                            <td>
                                                {target_view}
                                                <div class="muted small mono">{target_uri}</div>
                                            </td>
                                            <td class="mono small">{diagnostic.context.clone()}</td>
                                            <td>{owner_view}</td>
                                        </tr>
                                    }
                                }).collect_view()}
                            </tbody>
                        </table>
                    }.into_any()
                }}
            </Card>

            <Card title="Couverture des métamodèles".into()>
                <table class="table">
                    <thead>
                        <tr><th>"Profil"</th><th>"Classes"</th><th>"Relations"</th><th>"Paires auto-autorisées"</th></tr>
                    </thead>
                    <tbody>
                        {coverage.into_iter().map(|(id, elements, relations, pairs)| view! {
                            <tr>
                                <td class="mono">{id}</td>
                                <td>{elements}</td>
                                <td>{relations}</td>
                                <td>{pairs}</td>
                            </tr>
                        }).collect_view()}
                    </tbody>
                </table>
                <p class="muted small">
                    "Sous-ensemble annoncé : toute classe ou relation hors profil est refusée par le moteur avec un code explicite."
                </p>
            </Card>

            <Card title="Limites assumées du profil local".into()>
                <ul class="link-list">
                    <li>"Aucune authentification réelle : les identités sont simulées, toutes les capacités sont identiques."</li>
                    <li>"Aucune IA générative : la clé d'API ne doit jamais vivre dans le navigateur."</li>
                    <li>"Pas de recherche sémantique : seuls les index lexicaux et la tolérance aux fautes sont actifs."</li>
                    <li>"Les données sont propres à ce navigateur et à cette origine."</li>
                </ul>
            </Card>

            <Card title="Réinitialisation".into()>
                <p class="muted small">
                    "Efface définitivement le référentiel local et redémarre sur une application vierge. Exportez une sauvegarde avant si nécessaire."
                </p>
                <div class="actions">
                    <button class="btn danger" on:click=move |_| reset()>
                        "Effacer toutes les données"
                    </button>
                </div>
            </Card>
        </main>
    }
}
