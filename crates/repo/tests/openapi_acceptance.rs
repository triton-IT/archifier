//! Acceptation W1a — import et export OpenAPI.
//!
//! Ces tests suivent `docs/w1a-openapi-contract.md` et n'utilisent que l'API
//! publique : `parse_openapi`, `import_openapi_preview`, `apply_openapi_import`,
//! `export_openapi` et `diff_openapi_versions`. Aucun accès direct au stockage.
//!
//! Les dix tests bloquants du §8 sont présents, complétés par les cas des codes
//! séparés (§10.4), la canonicalisation YAML/JSON, l'avertissement
//! `OAS-WARN-001` et le diff entre deux versions stockées (§10.7).

use archifier_core::{
    ContractFormat, ContractType, DiffClass, InterfaceCategory, ObjectId, ParsedContract,
};
use archifier_repo::{MemoryStore, OpenApiImportStatus, Repository};
use serde_json::Value;
use std::rc::Rc;

const NOW_MS: u64 = 1_795_000_000_000; // octobre 2026, horloge figée

const PETSTORE_30: &str = include_str!("../../../fixtures/mvp2/openapi/petstore-3.0.yaml");
const PETSTORE_31: &str = include_str!("../../../fixtures/mvp2/openapi/petstore-3.1.json");
const PETSTORE_31_YAML: &str = include_str!("../../../fixtures/mvp2/openapi/petstore-3.1.yaml");
const PETSTORE_32: &str = include_str!("../../../fixtures/mvp2/openapi/petstore-3.2.yaml");
const PETSTORE_31_UNCHANGED: &str =
    include_str!("../../../fixtures/mvp2/openapi/petstore-3.1-unchanged.json");
const PETSTORE_31_BREAKING: &str =
    include_str!("../../../fixtures/mvp2/openapi/petstore-3.1-breaking.json");
const PETSTORE_31_WITH_IDS: &str =
    include_str!("../../../fixtures/mvp2/openapi/petstore-3.1-with-architecture-ids.json");
const PETSTORE_31_WITHOUT_OPERATION_ID: &str =
    include_str!("../../../fixtures/mvp2/openapi/petstore-3.1-without-operation-id.yaml");
const PETSTORE_31_DUPLICATE_OPERATION_ID: &str =
    include_str!("../../../fixtures/mvp2/openapi/petstore-3.1-duplicate-operation-id.json");
const PETSTORE_31_DUPLICATE_SEMVER: &str =
    include_str!("../../../fixtures/mvp2/openapi/petstore-3.1-duplicate-semver.json");
const PETSTORE_31_PARAMETER_REMOVED: &str =
    include_str!("../../../fixtures/mvp2/openapi/petstore-3.1-parameter-removed.json");
const PETSTORE_31_OPERATION_RENAMED: &str =
    include_str!("../../../fixtures/mvp2/openapi/petstore-3.1-operation-renamed.json");
const PETSTORE_31_SERVER_CHANGED: &str =
    include_str!("../../../fixtures/mvp2/openapi/petstore-3.1-server-changed.json");
const PETSTORE_31_NO_OPERATIONS: &str =
    include_str!("../../../fixtures/mvp2/openapi/petstore-3.1-no-operations.json");
const INVALID_DOCUMENT: &str = include_str!("../../../fixtures/mvp2/openapi/invalid-document.json");

fn repo() -> Repository {
    Repository::new(Rc::new(MemoryStore::new()), NOW_MS, 42)
}

fn domain(repo: &Repository) -> ObjectId {
    repo.create_domain("Contrats", "Domaine des contrats importés")
        .expect("création du domaine")
}

fn parsed(raw: &str) -> ParsedContract {
    archifier_core::openapi::parse_openapi(raw).unwrap_or_else(|_| panic!("document refusé"))
}

/// Importe, applique sans confirmation et renvoie les identifiants créés.
/// Précondition commune des scénarios de mise à jour, de diff et d'export.
fn imported(repo: &Repository, domain_id: ObjectId, raw: &str) -> (ObjectId, ObjectId) {
    let preview = repo
        .import_openapi_preview(domain_id, raw)
        .expect("aperçu d'import");
    assert_eq!(
        preview.status,
        OpenApiImportStatus::New,
        "le premier import doit être un nouveau contrat"
    );
    assert!(
        !preview.diff.requires_confirmation,
        "un premier import n'exige pas de confirmation"
    );
    assert!(preview.base_version_id.is_none());
    assert!(preview.target_version_id.is_none());
    let outcome = repo
        .apply_openapi_import(&preview, false)
        .expect("application de l'import");
    (
        outcome.interface_id,
        outcome.version_id.expect("une version est créée"),
    )
}

/// Sérialise une version pour inspecter les champs du contrat (§10.3) sans
/// dépendre du type Rust exact de `contract_raw` / `contract_hash`.
fn version_as_json(repo: &Repository, interface_id: ObjectId, version_id: ObjectId) -> Value {
    let detail = repo
        .interface_detail(interface_id)
        .expect("interface présente");
    let summary = detail
        .versions
        .iter()
        .find(|v| v.version.id == version_id)
        .expect("version présente");
    serde_json::to_value(&summary.version).expect("version sérialisable")
}

fn exported_json(repo: &Repository, version_id: ObjectId) -> Value {
    let document = repo.export_openapi(version_id).expect("export OpenAPI");
    serde_json::from_str(&document).expect("l'export doit être un JSON OpenAPI valide")
}

// ---------------------------------------------------------------------------
// Tests bloquants du §8
// ---------------------------------------------------------------------------

#[test]
fn oas_import_3_0_3_1_3_2() {
    // `parse_openapi` est public (§9.9) : on vérifie le ParsedContract lui-même,
    // puis son intégration dans l'aperçu du dépôt.
    let repo = repo();
    let domain = domain(&repo);

    for (raw, title, format) in [
        (PETSTORE_30, "Petstore 3.0", ContractFormat::OpenApi30),
        (PETSTORE_31, "Petstore 3.1", ContractFormat::OpenApi31),
        (PETSTORE_32, "Petstore 3.2", ContractFormat::OpenApi32),
    ] {
        let contract = parsed(raw);
        assert_eq!(contract.format, format);
        assert_eq!(contract.title, title);
        assert_eq!(contract.version, "1.0.0");
        assert_eq!(contract.raw, raw, "le parseur conserve le texte d'origine");
        assert!(!contract.hash.is_empty());
        assert_eq!(contract.operations.len(), 4, "quatre opérations {title}");
        assert_eq!(contract.operations[0].path, "/pets", "opérations triées");
        assert_eq!(
            contract.operations[0].method, "GET",
            "méthode en majuscules"
        );
        assert!(!contract.servers.is_empty());
        assert!(contract.schemas.contains_key("Pet"));

        let preview = repo
            .import_openapi_preview(domain, raw)
            .unwrap_or_else(|e| panic!("famille {title} refusée : {e}"));
        assert_eq!(preview.format, format);
        assert_eq!(preview.status, OpenApiImportStatus::New);
        assert_eq!(preview.domain_id, domain);
        assert_eq!(preview.interface_name, title, "info.title mappé sur le nom");
        assert_eq!(preview.semantic_version, "1.0.0", "info.version mappé");
        assert_eq!(preview.raw_bytes, raw.len(), "taille du document brut");
        assert!(preview.interface_id.is_none());
        assert!(preview.target_version_id.is_none());

        let outcome = repo
            .apply_openapi_import(&preview, false)
            .unwrap_or_else(|e| panic!("application {title} refusée : {e}"));
        assert!(outcome.written, "l'application écrit {title}");
    }

    assert_eq!(repo.stats().interfaces, 3, "trois interfaces distinctes");
    let names: Vec<String> = repo
        .interfaces(&Default::default())
        .into_iter()
        .map(|i| i.name)
        .collect();
    for title in ["Petstore 3.0", "Petstore 3.1", "Petstore 3.2"] {
        assert!(
            names.iter().any(|n| n == title),
            "{title} absente : {names:?}"
        );
    }
}

#[test]
fn oas_raw_document_is_preserved() {
    let repo = repo();
    let domain = domain(&repo);
    let (interface_id, version_id) = imported(&repo, domain, PETSTORE_31);

    let version = version_as_json(&repo, interface_id, version_id);
    assert_eq!(
        version["contract_raw"].as_str(),
        Some(PETSTORE_31),
        "le document brut est conservé octet-à-octet"
    );

    let hash = version["contract_hash"]
        .as_str()
        .expect("contract_hash stocké");
    assert_eq!(hash.len(), 64, "empreinte SHA-256 hexadécimale : {hash}");
    assert!(
        hash.chars().all(|c| c.is_ascii_hexdigit()),
        "empreinte hexadécimale attendue : {hash}"
    );
}

#[test]
fn oas_import_is_idempotent_by_fingerprint() {
    // Les deux fichiers sont sémantiquement identiques mais byte-différents :
    // l'identité repose sur l'empreinte canonique, pas sur le texte brut.
    assert_ne!(
        PETSTORE_31, PETSTORE_31_UNCHANGED,
        "les fixtures doivent différer au niveau des octets"
    );

    let repo = repo();
    let domain = domain(&repo);
    let first = repo
        .import_openapi_preview(domain, PETSTORE_31)
        .expect("premier aperçu");
    assert_eq!(first.status, OpenApiImportStatus::New);
    let outcome = repo
        .apply_openapi_import(&first, false)
        .expect("application");
    assert_eq!(repo.stats().interfaces, 1);
    assert_eq!(repo.stats().versions, 1);
    let version_id = outcome.version_id.expect("version créée");

    let again = repo
        .import_openapi_preview(domain, PETSTORE_31_UNCHANGED)
        .expect("réimport du même contrat");
    assert_eq!(again.status, OpenApiImportStatus::Unchanged);
    assert_eq!(
        again.hash, first.hash,
        "même empreinte canonique malgré des octets différents"
    );
    assert_eq!(again.canonical, first.canonical);
    assert_eq!(again.interface_id, Some(outcome.interface_id));
    assert_eq!(again.target_version_id, Some(version_id));
    assert_eq!(repo.stats().interfaces, 1, "aucune seconde interface");
    assert_eq!(repo.stats().versions, 1, "aucune seconde version");

    let applied = repo
        .apply_openapi_import(&again, false)
        .expect("application idempotente");
    assert!(!applied.written, "un document inchangé n'écrit rien");
    assert_eq!(applied.version_id, Some(version_id));
    assert_eq!(repo.stats().interfaces, 1);
    assert_eq!(repo.stats().versions, 1);
}

#[test]
fn oas_import_creates_preview_without_write() {
    let repo = repo();
    let domain = domain(&repo);

    let preview = repo
        .import_openapi_preview(domain, PETSTORE_31)
        .expect("aperçu");
    assert_eq!(preview.status, OpenApiImportStatus::New);
    assert_eq!(preview.format, ContractFormat::OpenApi31);
    assert_eq!(preview.interface_name, "Petstore 3.1");
    assert_eq!(preview.semantic_version, "1.0.0");
    assert_eq!(preview.raw_bytes, PETSTORE_31.len());
    assert_eq!(preview.raw, PETSTORE_31);
    assert!(preview.interface_id.is_none());
    assert!(preview.base_version_id.is_none());
    assert!(preview.target_version_id.is_none());

    assert_eq!(repo.stats().interfaces, 0, "l'aperçu ne crée rien");
    assert_eq!(repo.stats().versions, 0, "l'aperçu ne crée aucune version");
}

#[test]
fn oas_apply_creates_interface_and_version() {
    let repo = repo();
    let domain = domain(&repo);

    let preview = repo
        .import_openapi_preview(domain, PETSTORE_30)
        .expect("aperçu");
    let outcome = repo
        .apply_openapi_import(&preview, false)
        .expect("application");
    assert_eq!(outcome.status, OpenApiImportStatus::New);
    assert!(outcome.written);
    let version_id = outcome.version_id.expect("version créée");

    let detail = repo
        .interface_detail(outcome.interface_id)
        .expect("interface créée");
    assert_eq!(detail.interface.name, "Petstore 3.0");
    assert_eq!(detail.interface.category, InterfaceCategory::Application);
    assert_eq!(
        detail.interface.contract_type,
        ContractType::OpenApi,
        "le type de contrat est renseigné par l'import"
    );
    assert_eq!(detail.versions.len(), 1);
    assert_eq!(detail.versions[0].version.id, version_id);
    assert_eq!(detail.versions[0].version.semantic_version, "1.0.0");
    assert_eq!(
        detail.versions[0].version.publication_status,
        archifier_core::PublicationStatus::Draft,
        "une version importée est DRAFT"
    );

    assert_eq!(repo.stats().interfaces, 1);
    assert_eq!(repo.stats().versions, 1);
}

#[test]
fn oas_diff_classifies_breaking_change() {
    let repo = repo();
    let domain = domain(&repo);
    let (interface_id, base_version) = imported(&repo, domain, PETSTORE_31);

    let preview = repo
        .import_openapi_preview(domain, PETSTORE_31_BREAKING)
        .expect("aperçu de l'évolution");
    assert_eq!(preview.status, OpenApiImportStatus::UpdateProposed);
    assert_eq!(
        preview.interface_id,
        Some(interface_id),
        "le rapprochement se fait sur la clé logique du titre"
    );
    assert_eq!(
        preview.base_version_id,
        Some(base_version),
        "le diff porte sur la version existante"
    );
    assert_eq!(
        preview.target_version_id, None,
        "une nouvelle version sera créée, pas une réécriture"
    );
    assert_eq!(preview.semantic_version, "2.0.0");
    assert_eq!(preview.diff.classification, DiffClass::Breaking);
    assert!(
        preview.diff.requires_confirmation,
        "une rupture exige une confirmation"
    );

    for code in ["OAS-010", "OAS-011", "OAS-012", "OAS-013"] {
        assert!(
            preview.diff.changes.iter().any(|c| c.code == code),
            "changement {code} attendu : {:?}",
            preview.diff.changes
        );
    }
    assert!(
        preview
            .diff
            .changes
            .iter()
            .filter(|c| c.code.starts_with("OAS-01"))
            .all(|c| c.class == DiffClass::Breaking),
        "les suppressions et restrictions sont des ruptures : {:?}",
        preview.diff.changes
    );

    assert_eq!(
        repo.stats().versions,
        1,
        "un aperçu de rupture n'écrit aucune version"
    );
}

#[test]
fn oas_breaking_change_requires_confirmation() {
    let repo = repo();
    let domain = domain(&repo);
    let (interface_id, _) = imported(&repo, domain, PETSTORE_31);
    let preview = repo
        .import_openapi_preview(domain, PETSTORE_31_BREAKING)
        .expect("aperçu de rupture");

    let error = repo
        .apply_openapi_import(&preview, false)
        .expect_err("une rupture non confirmée est refusée");
    assert!(
        error.is("OAS-040"),
        "code OAS-040 attendu, obtenu : {error}"
    );
    assert_eq!(
        repo.stats().versions,
        1,
        "aucune écriture sans confirmation"
    );

    let outcome = repo
        .apply_openapi_import(&preview, true)
        .expect("rupture confirmée");
    assert_eq!(outcome.status, OpenApiImportStatus::UpdateProposed);
    assert!(outcome.written);
    assert_eq!(
        outcome.interface_id, interface_id,
        "une nouvelle version de la même interface"
    );
    let version_id = outcome.version_id.expect("version de rupture créée");

    assert_eq!(repo.stats().interfaces, 1);
    assert_eq!(repo.stats().versions, 2);
    let detail = repo.interface_detail(interface_id).expect("interface");
    let created = detail
        .versions
        .iter()
        .find(|v| v.version.id == version_id)
        .expect("nouvelle version");
    assert_eq!(created.version.semantic_version, "2.0.0");
    assert!(
        created.version.breaking_change,
        "la version de rupture est marquée breaking_change"
    );
}

#[test]
fn oas_export_is_byte_reproducible() {
    let repo = repo();
    let domain = domain(&repo);
    let (_, version_id) = imported(&repo, domain, PETSTORE_31);

    let first = repo.export_openapi(version_id).expect("premier export");
    let second = repo.export_openapi(version_id).expect("second export");
    assert_eq!(first, second, "deux exports successifs sont identiques");
    assert!(first.ends_with('\n'), "saut de ligne final attendu");
    assert!(
        !first.contains("x-architecture-snapshot-at"),
        "aucun horodatage volatile"
    );

    let document: Value = serde_json::from_str(&first).expect("JSON canonique attendu");
    assert_eq!(document["openapi"], "3.1.0");
    assert_eq!(document["info"]["title"], "Petstore 3.1");
}

#[test]
fn oas_export_refuses_non_http_interface() {
    let repo = repo();
    let domain = domain(&repo);
    let interface = repo
        .create_interface(
            domain,
            "Parcours client",
            "parcours-client",
            InterfaceCategory::Business,
        )
        .expect("interface métier");
    let version = repo
        .create_interface_version(interface, "1.0.0", None, None, false, false)
        .expect("version");

    let error = repo
        .export_openapi(version)
        .expect_err("une interface métier ne s'exporte pas en OpenAPI");
    assert!(
        error.is("OAS-060"),
        "code OAS-060 attendu, obtenu : {error}"
    );
}

#[test]
fn oas_export_injects_architecture_extensions() {
    let repo = repo();
    let domain = domain(&repo);
    let (interface_id, version_id) = imported(&repo, domain, PETSTORE_31);

    let document = exported_json(&repo, version_id);
    assert_eq!(document["openapi"], "3.1.0");
    let interface_text = interface_id.to_string();
    let version_text = version_id.to_string();
    let domain_text = domain.to_string();
    assert_eq!(
        document["x-architecture-interface-id"].as_str(),
        Some(interface_text.as_str())
    );
    assert_eq!(
        document["x-architecture-version-id"].as_str(),
        Some(version_text.as_str())
    );
    assert_eq!(
        document["x-architecture-domain-id"].as_str(),
        Some(domain_text.as_str())
    );
    assert_eq!(
        document["x-architecture-logical-key"].as_str(),
        Some("petstore-3-1")
    );
    assert_eq!(document["x-architecture-version"].as_str(), Some("1.0.0"));

    // §6 et §9.5 disent `archifier/0.2.0`, §10.5 dit `archifier` (constante
    // sans version de build). On accepte les deux formes, le point est signalé.
    let generated = document["x-architecture-generated-by"]
        .as_str()
        .expect("x-architecture-generated-by présent");
    assert!(
        generated == "archifier" || generated.starts_with("archifier/"),
        "signature du générateur inattendue : {generated}"
    );
}

#[test]
fn oas_architecture_extensions_do_not_change_fingerprint() {
    // Le même document, augmenté de deux x-architecture-* inconnus, doit être
    // reconnu comme inchangé : ces extensions sont hors empreinte (§2, §10.2).
    let repo = repo();
    let domain = domain(&repo);
    let (interface_id, _) = imported(&repo, domain, PETSTORE_31);

    let preview = repo
        .import_openapi_preview(domain, PETSTORE_31_WITH_IDS)
        .expect("réimport avec extensions");
    assert_eq!(
        preview.status,
        OpenApiImportStatus::Unchanged,
        "les x-architecture-* n'entrent pas dans l'empreinte"
    );
    assert_eq!(preview.interface_id, Some(interface_id));
    assert_eq!(repo.stats().interfaces, 1);
    assert_eq!(repo.stats().versions, 1);
}

// ---------------------------------------------------------------------------
// Canonicalisation et avertissements
// ---------------------------------------------------------------------------

#[test]
fn yaml_and_json_share_fingerprint() {
    // `petstore-3.1.yaml` et `petstore-3.1.json` portent le même document dans
    // deux syntaxes : l'empreinte et la forme canonique doivent coïncider.
    let repo = repo();
    let domain = domain(&repo);

    let from_json = repo
        .import_openapi_preview(domain, PETSTORE_31)
        .expect("aperçu JSON");
    let from_yaml = repo
        .import_openapi_preview(domain, PETSTORE_31_YAML)
        .expect("aperçu YAML");

    assert_eq!(from_json.format, ContractFormat::OpenApi31);
    assert_eq!(from_yaml.format, ContractFormat::OpenApi31);
    assert_eq!(
        from_json.hash, from_yaml.hash,
        "YAML et JSON partagent la même empreinte"
    );
    assert_eq!(from_json.canonical, from_yaml.canonical);

    let outcome = repo
        .apply_openapi_import(&from_json, false)
        .expect("application JSON");
    let again = repo
        .import_openapi_preview(domain, PETSTORE_31_YAML)
        .expect("réimport YAML");
    assert_eq!(again.status, OpenApiImportStatus::Unchanged);
    assert_eq!(again.interface_id, Some(outcome.interface_id));
    assert_eq!(repo.stats().interfaces, 1);
    assert_eq!(repo.stats().versions, 1);
}

#[test]
fn oas_warn_001_generated_operation_id() {
    let repo = repo();
    let domain = domain(&repo);

    let preview = repo
        .import_openapi_preview(domain, PETSTORE_31_WITHOUT_OPERATION_ID)
        .expect("document sans operationId accepté");
    assert_eq!(preview.status, OpenApiImportStatus::New);
    assert!(!preview.warnings.is_empty(), "un avertissement est remonté");
    assert!(
        preview.warnings.iter().any(|w| w.contains("OAS-WARN-001")),
        "OAS-WARN-001 attendu : {:?}",
        preview.warnings
    );

    let outcome = repo
        .apply_openapi_import(&preview, false)
        .expect("application");
    assert!(outcome.written);
    let detail = repo
        .interface_detail(outcome.interface_id)
        .expect("interface créée");
    assert_eq!(detail.versions.len(), 1);
    let operations = detail.versions[0].version.contract_snapshot["operations"]
        .as_array()
        .expect("contract_snapshot.operations");
    assert_eq!(operations.len(), 4);
    assert!(
        operations
            .iter()
            .all(|op| op["operation_id"].as_str().is_some_and(|id| !id.is_empty())),
        "un identifiant est généré pour chaque opération : {operations:?}"
    );
}

// ---------------------------------------------------------------------------
// Erreurs typées (§10.4)
// ---------------------------------------------------------------------------

#[test]
fn oas_002_invalid_document() {
    let repo = repo();
    let domain = domain(&repo);

    let error = repo
        .import_openapi_preview(domain, INVALID_DOCUMENT)
        .expect_err("un document illisible est refusé");
    assert!(
        error.is("OAS-002"),
        "code OAS-002 attendu, obtenu : {error}"
    );
    assert_eq!(repo.stats().interfaces, 0);
    assert_eq!(repo.stats().versions, 0);
}

#[test]
fn oas_003_duplicate_operation_id() {
    let repo = repo();
    let domain = domain(&repo);

    let error = repo
        .import_openapi_preview(domain, PETSTORE_31_DUPLICATE_OPERATION_ID)
        .expect_err("un operationId dupliqué est refusé");
    assert!(
        error.is("OAS-003"),
        "code OAS-003 attendu, obtenu : {error}"
    );
    assert_eq!(repo.stats().interfaces, 0);
}

#[test]
fn oas_004_duplicate_semver() {
    let repo = repo();
    let domain = domain(&repo);
    let (interface_id, _) = imported(&repo, domain, PETSTORE_31);

    // Le refus peut survenir à l'aperçu ou à l'application : on observe les
    // deux étapes d'un seul geste, puis l'absence d'écriture.
    let result = repo
        .import_openapi_preview(domain, PETSTORE_31_DUPLICATE_SEMVER)
        .and_then(|preview| repo.apply_openapi_import(&preview, false));
    let error = result.expect_err("une SemVer déjà utilisée est refusée");
    assert!(
        error.is("OAS-004"),
        "code OAS-004 attendu, obtenu : {error}"
    );
    assert_eq!(repo.stats().interfaces, 1);
    assert_eq!(repo.stats().versions, 1, "aucune écriture");
    assert_eq!(
        repo.interface_detail(interface_id)
            .expect("interface")
            .versions
            .len(),
        1,
        "l'interface ne gagne aucune version"
    );
}

#[test]
fn oas_061_version_without_contract() {
    // Document valide mais sans aucune opération : la version existe, son
    // contrat normalisé ne contient rien d'exportable.
    let repo = repo();
    let domain = domain(&repo);
    let preview = repo
        .import_openapi_preview(domain, PETSTORE_31_NO_OPERATIONS)
        .expect("aperçu");
    assert_eq!(preview.status, OpenApiImportStatus::New);
    let outcome = repo
        .apply_openapi_import(&preview, true)
        .expect("application");
    let version_id = outcome.version_id.expect("version créée");

    let error = repo
        .export_openapi(version_id)
        .expect_err("une version sans opération ne s'exporte pas");
    assert!(
        error.is("OAS-061"),
        "code OAS-061 attendu, obtenu : {error}"
    );
}

// ---------------------------------------------------------------------------
// Changements de diff (§10.4)
// ---------------------------------------------------------------------------

#[test]
fn oas_015_parameter_removed() {
    let repo = repo();
    let domain = domain(&repo);
    let (interface_id, base_version) = imported(&repo, domain, PETSTORE_31);

    let preview = repo
        .import_openapi_preview(domain, PETSTORE_31_PARAMETER_REMOVED)
        .expect("aperçu");
    assert_eq!(preview.status, OpenApiImportStatus::UpdateProposed);
    assert_eq!(preview.interface_id, Some(interface_id));
    assert_eq!(preview.base_version_id, Some(base_version));
    assert!(
        preview.diff.changes.iter().any(|c| c.code == "OAS-015"),
        "OAS-015 attendu : {:?}",
        preview.diff.changes
    );
    assert_eq!(
        preview.diff.classification,
        DiffClass::Breaking,
        "suppression de paramètre = rupture (spec technique §11.3)"
    );
    assert_eq!(repo.stats().versions, 1, "aucune écriture");
}

#[test]
fn oas_016_operation_id_renamed() {
    let repo = repo();
    let domain = domain(&repo);
    let (interface_id, base_version) = imported(&repo, domain, PETSTORE_31);

    let preview = repo
        .import_openapi_preview(domain, PETSTORE_31_OPERATION_RENAMED)
        .expect("aperçu");
    assert_eq!(preview.status, OpenApiImportStatus::UpdateProposed);
    assert_eq!(preview.interface_id, Some(interface_id));
    assert_eq!(preview.base_version_id, Some(base_version));
    assert!(
        preview.diff.changes.iter().any(|c| c.code == "OAS-016"),
        "OAS-016 attendu : {:?}",
        preview.diff.changes
    );
    assert!(
        !preview.diff.changes.iter().any(|c| c.code == "OAS-010"),
        "un renommage n'est pas une suppression d'opération : {:?}",
        preview.diff.changes
    );
    assert_eq!(preview.diff.classification, DiffClass::Breaking);
    assert_eq!(repo.stats().versions, 1, "aucune écriture");
}

#[test]
fn oas_032_server_or_security_changed() {
    let repo = repo();
    let domain = domain(&repo);
    let (interface_id, base_version) = imported(&repo, domain, PETSTORE_31);

    let preview = repo
        .import_openapi_preview(domain, PETSTORE_31_SERVER_CHANGED)
        .expect("aperçu");
    assert_eq!(preview.status, OpenApiImportStatus::UpdateProposed);
    assert_eq!(preview.interface_id, Some(interface_id));
    assert_eq!(preview.base_version_id, Some(base_version));
    assert!(
        preview.diff.changes.iter().any(|c| c.code == "OAS-032"),
        "OAS-032 attendu : {:?}",
        preview.diff.changes
    );
    assert_eq!(
        preview.diff.classification,
        DiffClass::Unknown,
        "serveur/sécurité modifié = inconnu"
    );
    assert!(
        preview.diff.requires_confirmation,
        "un changement inconnu exige aussi une confirmation (§10.4)"
    );
    assert_eq!(repo.stats().versions, 1, "aucune écriture");
}

#[test]
fn oas_unknown_change_requires_confirmation() {
    let repo = repo();
    let domain = domain(&repo);
    let (interface_id, _) = imported(&repo, domain, PETSTORE_31);
    let preview = repo
        .import_openapi_preview(domain, PETSTORE_31_SERVER_CHANGED)
        .expect("aperçu");

    assert_eq!(preview.diff.classification, DiffClass::Unknown);
    assert!(preview.diff.requires_confirmation);

    let error = repo
        .apply_openapi_import(&preview, false)
        .expect_err("un changement inconnu non confirmé est refusé");
    assert!(
        error.is("OAS-040"),
        "code OAS-040 attendu, obtenu : {error}"
    );
    assert_eq!(repo.stats().versions, 1);

    let outcome = repo
        .apply_openapi_import(&preview, true)
        .expect("changement inconnu confirmé");
    assert!(outcome.written);
    assert_eq!(outcome.interface_id, interface_id);
    let version_id = outcome.version_id.expect("version créée");
    assert_eq!(repo.stats().interfaces, 1);
    assert_eq!(repo.stats().versions, 2);

    let detail = repo.interface_detail(interface_id).expect("interface");
    let created = detail
        .versions
        .iter()
        .find(|v| v.version.id == version_id)
        .expect("nouvelle version");
    assert_eq!(created.version.semantic_version, "1.1.0");
    assert!(
        !created.version.breaking_change,
        "inconnu confirmé n'est pas une rupture (§10.4)"
    );
}

// ---------------------------------------------------------------------------
// Diff entre versions stockées (§10.7)
// ---------------------------------------------------------------------------

#[test]
fn oas_diff_two_stored_versions() {
    let repo = repo();
    let domain = domain(&repo);
    let (_, base_version) = imported(&repo, domain, PETSTORE_31);

    let preview = repo
        .import_openapi_preview(domain, PETSTORE_31_BREAKING)
        .expect("aperçu de rupture");
    let outcome = repo
        .apply_openapi_import(&preview, true)
        .expect("rupture confirmée");
    let target_version = outcome.version_id.expect("version de rupture");

    let diff = repo
        .diff_openapi_versions(base_version, target_version)
        .expect("diff entre deux versions stockées");
    assert_eq!(diff.classification, DiffClass::Breaking);
    assert!(diff.requires_confirmation);
    for code in ["OAS-010", "OAS-011", "OAS-012", "OAS-013"] {
        assert!(
            diff.changes.iter().any(|c| c.code == code),
            "changement {code} attendu entre les versions stockées : {:?}",
            diff.changes
        );
    }
}
