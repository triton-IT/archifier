//! Tests bloquants W1b — export Markdown (T1-05).
//!
//! Contrat : `docs/w1b-contract.md` §1.2, §5.2, §5.5 à §5.8 et §6 ;
//! `technical-specifications.md` §12.2. Fixtures :
//! `fixtures/mvp2/export/dataset.json` (domaine `Paiements`, documentation à
//! trois liens `arch://` dont un doublon, diagramme à 2 nœuds / 1 arête, objet
//! archivé, domaine vide `Vide`) et `dataset-unreadable-diagram.json`.
//!
//! Les tests n'utilisent que `Repository::export_domain_markdown`,
//! `Repository::export_object_markdown` et, pour la coordonnée non finie (que
//! JSON ne peut pas porter), `archifier_core::export::export_domain`.

use archifier_core::export::export_domain;
use archifier_core::{Dataset, ObjectId};
use archifier_repo::{MemoryStore, Repository};
use std::collections::BTreeSet;
use std::rc::Rc;

const NOW_MS: u64 = 1_795_000_000_000; // 2026-11-18, horloge figée
const DATASET: &str = include_str!("../../../fixtures/mvp2/export/dataset.json");
const DATASET_UNREADABLE: &str =
    include_str!("../../../fixtures/mvp2/export/dataset-unreadable-diagram.json");

const DOMAIN: &str = "01K8ZP4N000000000000000001";
const ORDER: &str = "01K8ZP4N000000000000000002";
const PRICING: &str = "01K8ZP4N000000000000000003";
const ARCHIVED: &str = "01K8ZP4N00000000000000000B";
const DIAGRAM: &str = "01K8ZP4N00000000000000000N";

/// Bandeau imposé par le contrat §5.7.
const BANNER: &str = "> **Archivé** — consultation seule, dernière révision";

fn fixture(now_ms: u64) -> Repository {
    Repository::from_json(Rc::new(MemoryStore::new()), DATASET, now_ms, 42)
        .expect("fixture W1b lisible")
}

fn id(raw: &str) -> ObjectId {
    raw.parse().expect("ULID de fixture valide")
}

fn domain_id(repo: &Repository) -> ObjectId {
    repo.domains()
        .into_iter()
        .find(|d| d.slug == "paiements")
        .expect("domaine Paiements présent dans la fixture")
        .id
}

/// Exporte le domaine et aplatit le bundle en `(path, content)`.
fn domain_bundle(repo: &Repository) -> (String, Vec<(String, String)>) {
    let bundle = repo
        .export_domain_markdown(domain_id(repo))
        .expect("export du domaine");
    flatten(bundle)
}

/// Exporte l'objet et aplatit le bundle en `(path, content)`.
fn object_bundle(repo: &Repository, object: ObjectId) -> (String, Vec<(String, String)>) {
    let bundle = repo
        .export_object_markdown(object)
        .expect("export de l'objet");
    flatten(bundle)
}

fn flatten(bundle: archifier_core::export::ExportBundle) -> (String, Vec<(String, String)>) {
    (
        bundle.root,
        bundle
            .files
            .into_iter()
            .map(|file| (file.path, file.content))
            .collect(),
    )
}

fn content_of<'a>(files: &'a [(String, String)], path: &str) -> &'a str {
    files
        .iter()
        .find(|(candidate, _)| candidate == path)
        .map(|(_, content)| content.as_str())
        .unwrap_or_else(|| panic!("fichier {path} absent du bundle"))
}

fn has_file(files: &[(String, String)], path: &str) -> bool {
    files.iter().any(|(candidate, _)| candidate == path)
}

/// Cibles de tous les liens Markdown `[libellé](cible)`.
fn link_targets(markdown: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let mut rest = markdown;
    while let Some(start) = rest.find("](") {
        let after = &rest[start + 2..];
        let Some(end) = after.find(')') else { break };
        let raw = after[..end].trim();
        let target = raw
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_matches(|c| c == '<' || c == '>');
        if !target.is_empty() {
            targets.push(target.to_string());
        }
        rest = &after[end..];
    }
    targets
}

/// Résout `target` depuis le dossier `base_dir`, séparateur `/`.
fn resolve(base_dir: &str, target: &str) -> String {
    let target = target.split('#').next().unwrap_or("");
    let mut parts: Vec<&str> = if target.starts_with('/') {
        Vec::new()
    } else {
        base_dir.split('/').filter(|s| !s.is_empty()).collect()
    };
    for segment in target.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

/// §5.2 : aucun lien généré ne pointe encore vers `arch://`.
/// Les notes de lien cassé peuvent en citer un dans le texte, hors lien.
fn assert_no_arch_link(files: &[(String, String)]) {
    for (path, content) in files {
        assert!(
            !content.contains("](arch://"),
            "{path} contient un lien Markdown arch://"
        );
        for target in link_targets(content) {
            assert!(
                !target.contains("arch://"),
                "lien {target} de {path} n'a pas été converti"
            );
        }
    }
}

/// Tout lien relatif pointe vers un fichier réellement présent dans le bundle.
fn assert_links_resolve(files: &[(String, String)]) {
    let known: BTreeSet<&str> = files.iter().map(|(path, _)| path.as_str()).collect();
    for (path, content) in files {
        let base_dir = match path.rsplit_once('/') {
            Some((dir, _)) => dir,
            None => "",
        };
        for target in link_targets(content) {
            if target.starts_with('#') {
                continue; // ancre interne
            }
            assert!(
                !target.starts_with("http://")
                    && !target.starts_with("https://")
                    && !target.starts_with("mailto:"),
                "ressource externe interdite dans l'export : {target} ({path})"
            );
            assert!(
                !target.starts_with('/'),
                "lien absolu interdit dans {path} : {target}"
            );
            let resolved = resolve(base_dir, &target);
            assert!(
                known.contains(resolved.as_str()),
                "lien {target} de {path} pointe vers {resolved}, absent du bundle"
            );
        }
    }
}

/// §5.6 : chemins relatifs à `root`, `\n`, saut de ligne final, UTF-8 sans BOM.
fn assert_canonical_files(files: &[(String, String)], root: &str) {
    let root_prefix = format!("{root}/");
    for (path, content) in files {
        assert!(!path.is_empty(), "chemin vide");
        assert!(!path.starts_with('/'), "chemin absolu : {path}");
        assert!(!path.contains('\\'), "séparateur Windows : {path}");
        assert!(!path.contains(".."), "chemin remontant : {path}");
        assert!(
            !path.starts_with(&root_prefix),
            "path est relatif à root, pas préfixé par lui : {path}"
        );
        assert!(!content.starts_with('\u{feff}'), "BOM UTF-8 dans {path}");
        assert!(!content.contains('\r'), "CRLF dans {path}");
        assert!(
            content.ends_with('\n'),
            "saut de ligne final absent : {path}"
        );
    }
}

#[test]
fn markdown_export_has_valid_relative_links() {
    let repo = fixture(NOW_MS);
    let (root, files) = domain_bundle(&repo);

    assert_eq!(root, "paiements", "racine = slug du domaine");
    assert!(has_file(&files, "index.md"), "index.md présent");
    let order_page = format!("elements/{ORDER}.md");
    assert!(has_file(&files, &order_page), "page de l'élément attendue");
    assert_canonical_files(&files, &root);
    assert_no_arch_link(&files);
    assert_links_resolve(&files);

    // La documentation est convertie en liens relatifs, pas supprimée.
    let order = content_of(&files, &order_page);
    assert!(order.contains("Order Service v2"), "{order}");
    assert!(order.contains("Paiements"), "domaine affiché : {order}");
    assert!(order.contains("Pricing Service"), "voisin cité : {order}");
    let order_dir = order_page
        .rsplit_once('/')
        .map(|(dir, _)| dir)
        .unwrap_or("");
    let order_targets = link_targets(order);
    let pricing_page = format!("elements/{PRICING}.md");
    assert!(
        order_targets
            .iter()
            .any(|target| resolve(order_dir, target) == pricing_page),
        "le lien arch:// valide devient un lien vers {pricing_page} : {order_targets:?}"
    );
    assert!(
        order.to_lowercase().contains("lien cassé"),
        "le lien cassé reste lisible en texte : {order}"
    );

    // L'index : titre, sections utiles, liens triés par ObjectId.
    let index = content_of(&files, "index.md");
    assert!(index.starts_with("# Paiements"), "{index}");
    assert!(index.contains("## Éléments"), "section Éléments : {index}");
    assert!(
        index.contains("## Diagrammes"),
        "section Diagrammes : {index}"
    );
    // W1c : les pages Story ne sont pas exigées par le §6 ; si l'index les
    // liste, `assert_links_resolve` impose que leurs pages existent.
    assert!(
        !index.contains("## Interfaces"),
        "section vide omise : {index}"
    );
    let element_ids: Vec<String> = link_targets(index)
        .into_iter()
        .map(|target| resolve("", &target))
        .filter(|path| path.starts_with("elements/") && path.ends_with(".md"))
        .map(|path| path["elements/".len()..path.len() - ".md".len()].to_string())
        .collect();
    let mut sorted_ids = element_ids.clone();
    sorted_ids.sort();
    assert_eq!(element_ids, sorted_ids, "index trié par ObjectId");
    assert!(
        element_ids.contains(&ORDER.to_string()),
        "index.md liste {order_page}"
    );

    // Objet archivé : page présente, bandeau « archivé ».
    let archived_page = format!("elements/{ARCHIVED}.md");
    assert!(has_file(&files, &archived_page), "objet archivé exporté");
}

#[test]
fn markdown_export_is_reproducible() {
    // Même dataset, deux horloges : le bundle doit être identique.
    let early = fixture(NOW_MS);
    let late = fixture(NOW_MS + 365 * 24 * 3600 * 1000);
    let (root_early, files_early) = domain_bundle(&early);
    let (root_late, files_late) = domain_bundle(&late);

    assert_eq!(root_early, root_late);
    assert_eq!(
        files_early, files_late,
        "aucune horloge d'export : même dataset ⇒ même bundle"
    );

    // Deux exports successifs du même dépôt sont identiques.
    let (_, again) = domain_bundle(&early);
    assert_eq!(files_early, again);

    // Fichiers triés par path (octets), chemins uniques.
    let paths: Vec<&str> = files_early.iter().map(|(path, _)| path.as_str()).collect();
    let mut sorted = paths.clone();
    sorted.sort_unstable();
    assert_eq!(paths, sorted, "fichiers triés par path");
    let unique: BTreeSet<&str> = paths.iter().copied().collect();
    assert_eq!(unique.len(), paths.len(), "chemins uniques");

    assert_canonical_files(&files_early, &root_early);

    // Aucune date de génération (l'horloge du dépôt ne fuit pas).
    for (path, content) in &files_early {
        assert!(
            !content.contains("2026-11-18") && !content.contains("2027-11-18"),
            "{path} contient une date d'export"
        );
    }
}

#[test]
fn markdown_export_includes_svg_and_mermaid() {
    let repo = fixture(NOW_MS);
    let (root, files) = domain_bundle(&repo);

    let svg_path = format!("diagrams/{DIAGRAM}.svg");
    let mmd_path = format!("diagrams/{DIAGRAM}.mmd");
    let page_path = format!("diagrams/{DIAGRAM}.md");
    assert!(has_file(&files, &page_path), "page de diagramme");
    assert!(has_file(&files, &svg_path), "SVG du diagramme");
    assert!(has_file(&files, &mmd_path), "complément Mermaid");
    assert_canonical_files(&files, &root);

    let svg = content_of(&files, &svg_path);
    // §5.7 : bbox = union des nœuds + padding 8, formaté en {:.1}.
    assert!(
        svg.contains(r#"viewBox="32.0 32.0 476.0 96.0""#),
        "viewBox calculé depuis la bbox : {svg}"
    );
    assert!(svg.contains("<rect"), "un rectangle par nœud : {svg}");
    assert!(
        svg.contains("<line") || svg.contains("<polyline") || svg.contains("<path"),
        "une ligne par arête : {svg}"
    );
    assert!(svg.contains("Order Service v2"), "libellé du nœud 1");
    assert!(svg.contains("Pricing Service"), "libellé du nœud 2");
    let lowered = svg.to_lowercase();
    for forbidden in ["<script", "javascript:", "http://", "https://", "src="] {
        assert!(
            !lowered.contains(forbidden),
            "SVG autonome : « {forbidden} » interdit"
        );
    }

    // §5.7 : flowchart LR, ids n0/n1 dans l'ordre du tableau.
    let mmd = content_of(&files, &mmd_path);
    assert_eq!(
        mmd.lines()
            .find(|line| !line.trim().is_empty())
            .map(str::trim),
        Some("flowchart LR"),
        "Mermaid = flowchart LR"
    );
    assert!(mmd.contains("n0"), "nœud n0 : {mmd}");
    assert!(mmd.contains("n1"), "nœud n1 : {mmd}");
    assert!(
        mmd.contains("-->|serving|"),
        "arête étiquetée par relation_type : {mmd}"
    );
    assert_eq!(
        mmd.matches("-->").count(),
        1,
        "une arête par DiagramEdge : {mmd}"
    );

    // La page de diagramme renvoie vers ses deux représentations.
    let page = content_of(&files, &page_path);
    assert!(
        page.contains(&format!("{DIAGRAM}.svg")) && page.contains(&format!("{DIAGRAM}.mmd")),
        "page de diagramme liée à SVG et Mermaid : {page}"
    );
}

#[test]
fn markdown_export_includes_neighbors_for_object() {
    let repo = fixture(NOW_MS);
    let object = id(ORDER);
    let (root, files) = object_bundle(&repo, object);
    let domain_size = domain_bundle(&repo).1.len();

    assert_eq!(root, "order-service-v2", "racine = slugify(name)");
    let order_page = format!("elements/{ORDER}.md");
    let pricing_page = format!("elements/{PRICING}.md");
    assert!(has_file(&files, &order_page), "page de l'objet exporté");
    assert!(
        has_file(&files, &pricing_page),
        "voisin direct inclus pour que le lien résolve"
    );

    assert_canonical_files(&files, &root);
    assert_no_arch_link(&files);
    assert_links_resolve(&files);

    let order = content_of(&files, &order_page);
    assert!(order.contains("Pricing Service"));

    // Le bundle d'objet reste réduit aux voisins directs.
    assert!(
        files.len() < domain_size,
        "bundle d'objet ({}) plus petit que le domaine ({domain_size})",
        files.len()
    );
}

#[test]
fn markdown_export_empty_domain_has_index() {
    let repo = fixture(NOW_MS);
    let empty = repo
        .domains()
        .into_iter()
        .find(|d| d.slug == "vide")
        .expect("domaine vide présent dans la fixture")
        .id;
    let bundle = repo
        .export_domain_markdown(empty)
        .expect("un domaine vide s'exporte");
    let (root, files) = flatten(bundle);

    assert_eq!(root, "vide");
    assert!(has_file(&files, "index.md"), "index.md présent même vide");
    assert!(
        !files.iter().any(|(path, _)| path.starts_with("elements/")),
        "aucune page d'élément pour un domaine vide"
    );
    assert_canonical_files(&files, &root);

    let index = content_of(&files, "index.md");
    assert!(index.starts_with("# Vide"), "{index}");
    assert!(index.contains("vide"), "slug affiché : {index}");
    assert!(index.contains(&empty.to_string()), "id affiché : {index}");
    assert!(
        !index.contains("## Éléments"),
        "aucune section vide : {index}"
    );
    assert_links_resolve(&files);
    assert_no_arch_link(&files);
}

#[test]
fn markdown_export_archived_object_has_banner() {
    let repo = fixture(NOW_MS);
    let (_, files) = domain_bundle(&repo);
    let archived_page = format!("elements/{ARCHIVED}.md");
    let archived = content_of(&files, &archived_page);

    assert!(archived.contains("Legacy Billing"), "{archived}");
    // §5.7 : le bandeau est la première ligne après le titre.
    let mut lines = archived.lines();
    let title = lines
        .find(|line| line.starts_with("# "))
        .expect("titre de page en première position");
    assert!(title.contains("Legacy Billing"), "{title}");
    let after_title = lines
        .find(|line| !line.trim().is_empty())
        .expect("bandeau après le titre");
    assert_eq!(after_title.trim(), BANNER, "{archived}");
}

#[test]
fn markdown_export_non_finite_coordinate_is_export_002() {
    // Coordonnée non finie : impossible à représenter en JSON, construite en
    // mémoire et soumise à la fonction pure du noyau.
    let mut dataset: Dataset = serde_json::from_str(DATASET).expect("dataset lisible");
    let diagram = dataset
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.object.id == id(DIAGRAM))
        .expect("diagramme de la fixture");
    diagram.layout.nodes[0].x = f64::NAN;

    let error = export_domain(&dataset, id(DOMAIN)).expect_err("coordonnée non finie refusée");
    let debug = format!("{error:?}");
    assert!(
        debug.contains("UnreadableDiagram"),
        "ExportError::UnreadableDiagram attendu : {debug}"
    );

    // Même branche côté dépôt : largeur nulle dans une fixture sur disque.
    let repo = Repository::from_json(Rc::new(MemoryStore::new()), DATASET_UNREADABLE, NOW_MS, 42)
        .expect("fixture EXPORT-002 lisible");
    let bad_domain = repo
        .domains()
        .into_iter()
        .find(|domain| domain.slug == "invalide")
        .expect("domaine invalide")
        .id;
    let error = repo.export_domain_markdown(bad_domain).unwrap_err();
    assert!(error.is("EXPORT-002"), "{error}");
}
