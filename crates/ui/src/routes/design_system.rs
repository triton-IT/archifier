//! `/_design` — galerie du design system (VD-23).

use crate::i18n::t;
use crate::router::Route;
use archifier_core::ViewIntent;
use leptos::prelude::*;

#[component]
pub fn DesignSystem() -> impl IntoView {
    view! {
        <main class="page design-system">
            <header class="page-head">
                <h1>{t("design_system.title")}</h1>
                <p class="muted">{t("design_system.subtitle")}</p>
            </header>

            <section class="card">
                <h2>{t("design_system.layers")}</h2>
                <div class="gallery-layers">
                    {[
                        ("business", "layer.business"),
                        ("application", "layer.application"),
                        ("technology", "layer.technology"),
                        ("physical", "layer.physical"),
                        ("motivation", "layer.motivation"),
                        ("implementation", "layer.implementation"),
                    ].into_iter().map(|(tone, key)| view! {
                        <div class=format!("layer-sample layer-{tone}")>{t(key)}</div>
                    }).collect_view()}
                </div>
            </section>

            <section class="card">
                <h2>{t("design_system.zones")}</h2>
                <svg class="gallery-canvas" viewBox="0 0 640 220" role="img" aria-label=t("design_system.zones")>
                    <g class="zone zone-domain">
                        <rect x="10" y="10" width="280" height="180" rx="8"></rect>
                        <text x="22" y="32" class="zone-label">"Paiements"</text>
                    </g>
                    <g class="zone zone-group">
                        <rect x="320" y="10" width="300" height="180" rx="8" stroke-dasharray="6 4"></rect>
                        <text x="332" y="32" class="zone-label">"Couloir client"</text>
                    </g>
                    <g class="node">
                        <rect x="60" y="70" width="160" height="72" rx="8" class="node-box"></rect>
                        <text x="70" y="98" class="node-title">"Pricing"</text>
                        <text x="70" y="118" class="node-type">"Service applicatif"</text>
                    </g>
                    <g class="port">
                        <circle cx="290" cy="106" r="8"></circle>
                        <text x="302" y="110" class="port-version">"v2"</text>
                    </g>
                </svg>
            </section>

            <section class="card">
                <h2>{t("design_system.temporal")}</h2>
                <div class="gallery-row">
                    <div class="temporal-demo temporal-active">
                        <strong>"Active"</strong><span>"trait plein"</span>
                    </div>
                    <div class="temporal-demo temporal-scheduled">
                        <strong>"Planifiée"</strong><span>"pointillé 6 4"</span>
                    </div>
                    <div class="temporal-demo temporal-expired">
                        <strong>"Expirée"</strong><span>"fantôme barré"</span>
                    </div>
                </div>
            </section>

            <section class="card">
                <h2>{t("design_system.statuses")}</h2>
                <div class="gallery-row">
                    <span class="badge draft">"Croquis"</span>
                    <span class="badge warn">"Brouillon"</span>
                    <span class="badge ok">"Validé"</span>
                    <span class="badge accent">"Publié"</span>
                </div>
                <div class="gallery-row">
                    <div class="draft-demo">"Forme au trait pointillé"</div>
                    <div class="valid-demo">"Forme pleine validée"</div>
                </div>
            </section>

            <section class="card">
                <h2>{t("design_system.buttons")}</h2>
                <div class="gallery-row">
                    <button class="btn primary">"Action principale"</button>
                    <button class="btn">"Action"</button>
                    <button class="btn ghost">"Discrète"</button>
                    <button class="btn danger">"Danger"</button>
                </div>
                <div class="gallery-row">
                    <label class="field"><span>"Nom"</span><input placeholder="Pricing" /></label>
                    <label class="field"><span>"Type"</span><select><option>"Service applicatif"</option></select></label>
                </div>
            </section>

            <section class="card">
                <h2>{t("design_system.empty")}</h2>
                <p class="callout">{t("intent.application_landscape.empty")}</p>
                <button class="btn primary">{t("atelier.create")}</button>
            </section>

            <section class="card">
                <h2>{t("design_system.intents")}</h2>
                <ul class="link-list">
                    {ViewIntent::ALL.into_iter().map(|intent| view! {
                        <li>
                            <a href=Route::ViewNew { intent: Some(intent) }.to_hash()>
                                {t(intent.label_key())}
                            </a>
                            <span class="muted small">{t(&intent.label_key().replace(".label", ".tagline"))}</span>
                        </li>
                    }).collect_view()}
                </ul>
            </section>
        </main>
    }
}
