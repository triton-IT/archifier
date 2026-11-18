//! Import et export de contrats OpenAPI (lot W1a).
//!
//! La preview est opposable : elle transporte le brut, le canonique, le hash et
//! la révision attendue. `apply_openapi_import` recalcule tout depuis le brut,
//! vérifie l'empreinte (`OAS-041` si périmée) puis écrit interface et version
//! dans un seul `mutate`.

use crate::types::{OpenApiImportOutcome, OpenApiImportPreview, OpenApiImportStatus};
use crate::{Inner, RepoError, RepoResult, Repository};
use archifier_core::contract_diff::{ContractChange, ContractDiff, DiffClass};
use archifier_core::openapi::{
    canonical_json_pretty, normalize_semver, parse_contract_with_warnings, ContractFormat,
    ParsedContract,
};
use archifier_core::{
    slugify, validate_interface, validate_interface_version, ContractType, Interface,
    InterfaceCategory, InterfaceVersion, Lifecycle, ObjectId, ObjectType, PublicationStatus,
    RepositoryObject,
};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use time::OffsetDateTime;

/// Générateur figé par le contrat W1a §10.5 (sans version de build).
pub const EXPORT_GENERATOR: &str = "archifier";

/// Résolution d'identité interne, avant écriture.
struct Resolution {
    status: OpenApiImportStatus,
    interface_id: Option<ObjectId>,
    base_version_id: Option<ObjectId>,
    target_version_id: Option<ObjectId>,
    diff: ContractDiff,
    warnings: Vec<String>,
    title: String,
    semantic_version: String,
}

impl Repository {
    /// Analyse un document OpenAPI sans écrire dans le référentiel.
    pub fn import_openapi_preview(
        &self,
        domain_id: ObjectId,
        raw: &str,
    ) -> RepoResult<OpenApiImportPreview> {
        let (contract, warnings) =
            parse_contract_with_warnings(raw).map_err(repo_error_from_contract)?;
        let resolution = {
            let inner = self.inner.borrow();
            if !inner.active_domain(domain_id) {
                return Err(RepoError::new(
                    "OBJ-003",
                    "Domaine responsable introuvable ou archivé.",
                ));
            }
            resolve_import(&inner, domain_id, &contract, warnings)?
        };

        let expected_interface_revision = resolution.interface_id.and_then(|interface_id| {
            self.inner
                .borrow()
                .find_interface(interface_id)
                .map(|interface| interface.object.revision)
        });

        Ok(OpenApiImportPreview {
            status: resolution.status,
            domain_id,
            interface_id: resolution.interface_id,
            base_version_id: resolution.base_version_id,
            target_version_id: resolution.target_version_id,
            interface_name: resolution.title,
            semantic_version: resolution.semantic_version,
            format: contract.format,
            raw: contract.raw,
            canonical: contract.canonical,
            hash: contract.hash,
            diff: resolution.diff,
            raw_bytes: raw.len(),
            warnings: resolution.warnings,
            expected_interface_revision,
        })
    }

    /// Applique une preview après recalcul complet et vérification d'actualité.
    pub fn apply_openapi_import(
        &self,
        preview: &OpenApiImportPreview,
        confirm_breaking: bool,
    ) -> RepoResult<OpenApiImportOutcome> {
        // 1. Recalcul depuis le brut, sans faire confiance à l'appelant.
        let (contract, _) =
            parse_contract_with_warnings(&preview.raw).map_err(repo_error_from_contract)?;

        // 2. Empreinte et canonique recalculés.
        if contract.hash != preview.hash
            || contract.canonical != preview.canonical
            || contract.format != preview.format
        {
            return Err(stale_preview());
        }

        // 3. Diff et identité recalculés ; 4. révision de l'interface vérifiée.
        let resolution = {
            let inner = self.inner.borrow();
            if !inner.active_domain(preview.domain_id) {
                return Err(RepoError::new(
                    "OBJ-003",
                    "Domaine responsable introuvable ou archivé.",
                ));
            }
            let resolution = resolve_import(&inner, preview.domain_id, &contract, vec![])?;
            if resolution.status != preview.status
                || resolution.interface_id != preview.interface_id
                || resolution.base_version_id != preview.base_version_id
                || resolution.target_version_id != preview.target_version_id
            {
                return Err(stale_preview());
            }
            if let Some(interface_id) = resolution.interface_id {
                let current = inner
                    .find_interface(interface_id)
                    .map(|interface| interface.object.revision);
                if preview.expected_interface_revision.is_some()
                    && current != preview.expected_interface_revision
                {
                    return Err(stale_preview());
                }
            }
            resolution
        };

        if resolution.status == OpenApiImportStatus::Unchanged {
            let interface_id = resolution.interface_id.ok_or_else(|| {
                RepoError::new("OAS-041", "Aperçu incohérent : interface introuvable.")
            })?;
            return Ok(OpenApiImportOutcome {
                status: OpenApiImportStatus::Unchanged,
                interface_id,
                version_id: resolution.base_version_id,
                written: false,
            });
        }

        // 5. Confirmation obligatoire pour rupture *et* inconnu.
        if resolution.diff.requires_confirmation && !confirm_breaking {
            return Err(RepoError::new(
                "OAS-040",
                "Rupture de contrat non confirmée : cochez « Confirmer la rupture » pour créer la version.",
            ));
        }

        // 6. Écriture interface + version dans un seul `mutate`.
        let breaking = resolution.diff.classification == DiffClass::Breaking && confirm_breaking;
        self.mutate(|inner| {
            apply_resolution(
                inner,
                self,
                preview.domain_id,
                &contract,
                &resolution,
                breaking,
            )
        })
    }

    /// Exporte une version en document OpenAPI 3.1.0 déterministe.
    pub fn export_openapi(&self, version_id: ObjectId) -> RepoResult<String> {
        let inner = self.inner.borrow();
        let version = inner
            .data
            .interface_versions
            .iter()
            .find(|version| version.id == version_id)
            .cloned()
            .ok_or_else(|| RepoError::not_found("Version d'interface"))?;
        let interface = inner
            .find_interface(version.interface_id)
            .ok_or_else(|| RepoError::not_found("Interface"))?;

        if interface.category != InterfaceCategory::Application
            || interface.contract_type != ContractType::OpenApi
        {
            return Err(RepoError::new(
                "OAS-060",
                "Export OpenAPI refusé : seules les interfaces APPLICATIVES de type OpenAPI sont exportables.",
            ));
        }

        let has_operations = version
            .contract_snapshot
            .get("operations")
            .and_then(Value::as_array)
            .map(|operations| !operations.is_empty())
            .unwrap_or(false);
        if !version.contract_snapshot.is_object()
            || version
                .contract_snapshot
                .as_object()
                .map(|snapshot| snapshot.is_empty())
                .unwrap_or(true)
            || !has_operations
        {
            return Err(RepoError::new(
                "OAS-061",
                "Export OpenAPI refusé : la version ne porte aucun contrat normalisé avec opération.",
            ));
        }

        let mut document = render_openapi(interface, &version);
        document.push('\n');
        Ok(document)
    }

    /// Diff entre deux versions stockées (T1-03).
    pub fn diff_openapi_versions(
        &self,
        from_version_id: ObjectId,
        to_version_id: ObjectId,
    ) -> RepoResult<ContractDiff> {
        let inner = self.inner.borrow();
        let find = |version_id: ObjectId| {
            inner
                .data
                .interface_versions
                .iter()
                .find(|version| version.id == version_id)
                .ok_or_else(|| RepoError::not_found("Version d'interface"))
        };
        let from = find(from_version_id)?;
        let to = find(to_version_id)?;
        let has_snapshot = |version: &InterfaceVersion| {
            version
                .contract_snapshot
                .as_object()
                .map(|snapshot| !snapshot.is_empty())
                .unwrap_or(false)
        };
        if !has_snapshot(from) || !has_snapshot(to) {
            return Err(RepoError::new(
                "OAS-061",
                "Diff OpenAPI impossible : une version ne porte aucun contrat normalisé.",
            ));
        }
        Ok(archifier_core::diff_snapshots(
            &from.contract_snapshot,
            &to.contract_snapshot,
        ))
    }
}

fn stale_preview() -> RepoError {
    RepoError::new(
        "OAS-041",
        "Aperçu périmé : le document ou l'interface a changé depuis l'analyse. Relancez l'analyse.",
    )
}

// ---------------------------------------------------------------------------
// Résolution d'identité (§10.2)
// ---------------------------------------------------------------------------

fn repo_error_from_contract(error: archifier_core::ContractError) -> RepoError {
    RepoError::new(error.code(), error.detail())
}

fn resolve_import(
    inner: &Inner,
    domain_id: ObjectId,
    contract: &ParsedContract,
    mut warnings: Vec<String>,
) -> RepoResult<Resolution> {
    let mut title = contract.title.trim().to_string();
    if title.is_empty() {
        title = "Contrat OpenAPI".to_string();
        warnings
            .push("info.title absent ; « Contrat OpenAPI » utilisé comme nom d'interface.".into());
    }
    let semantic_version = normalize_semver(&contract.version).map_err(repo_error_from_contract)?;
    let hash = contract.hash.as_str();
    let format = contract.format;

    // 2. x-architecture-version-id connu.
    if let Some(version_id) = contract
        .extension("x-architecture-version-id")
        .and_then(|value| value.parse::<ObjectId>().ok())
    {
        if let Some(version) = inner
            .data
            .interface_versions
            .iter()
            .find(|version| version.id == version_id)
        {
            interface_for(inner, version.interface_id)?;
            if version.contract_format == Some(format)
                && version.contract_hash.as_deref() == Some(hash)
            {
                return Ok(unchanged(
                    version.interface_id,
                    version.id,
                    warnings,
                    title,
                    semantic_version,
                ));
            }
            let diff =
                archifier_core::diff_snapshots(&version.contract_snapshot, &contract.snapshot());
            if version.publication_status == PublicationStatus::Draft {
                ensure_semver_available(
                    inner,
                    version.interface_id,
                    &semantic_version,
                    Some(version.id),
                )?;
                return Ok(Resolution {
                    status: OpenApiImportStatus::UpdateProposed,
                    interface_id: Some(version.interface_id),
                    base_version_id: Some(version.id),
                    target_version_id: Some(version.id),
                    diff,
                    warnings,
                    title,
                    semantic_version,
                });
            }
            ensure_semver_available(inner, version.interface_id, &semantic_version, None)?;
            return Ok(Resolution {
                status: OpenApiImportStatus::UpdateProposed,
                interface_id: Some(version.interface_id),
                base_version_id: Some(version.id),
                target_version_id: None,
                diff,
                warnings,
                title,
                semantic_version,
            });
        }
    }

    // 3. x-architecture-interface-id connu.
    if let Some(interface_id) = contract
        .extension("x-architecture-interface-id")
        .and_then(|value| value.parse::<ObjectId>().ok())
    {
        if let Some(interface) = inner.find_interface(interface_id) {
            return proposal_for_interface(
                inner,
                interface,
                contract,
                hash,
                format,
                warnings,
                title,
                semantic_version,
            );
        }
    }

    // 4. hash déjà présent sur n'importe quelle version : index (format, hash).
    if let Some(version_id) = inner
        .index
        .contract_by_hash
        .get(&(format, hash.to_string()))
        .copied()
    {
        if let Some(version) = inner
            .data
            .interface_versions
            .iter()
            .find(|version| version.id == version_id)
        {
            return Ok(unchanged(
                version.interface_id,
                version.id,
                warnings,
                title,
                semantic_version,
            ));
        }
    }

    // 5. clé logique dérivée du titre, dans le même domaine seulement.
    let logical_key = logical_key_for(&title, hash);
    if let Some(interface_id) = inner.index.interface_by_key.get(&logical_key).copied() {
        if let Some(interface) = inner.find_interface(interface_id) {
            if interface.provider_domain_id != domain_id {
                return Err(RepoError::new(
                    "OAS-005",
                    format!(
                        "La clé logique « {logical_key} » appartient déjà à une interface d'un autre domaine."
                    ),
                ));
            }
            return proposal_for_interface(
                inner,
                interface,
                contract,
                hash,
                format,
                warnings,
                title,
                semantic_version,
            );
        }
    }

    // 6. nouvelle interface proposée.
    Ok(Resolution {
        status: OpenApiImportStatus::New,
        interface_id: None,
        base_version_id: None,
        target_version_id: None,
        diff: ContractDiff::unchanged(),
        warnings,
        title,
        semantic_version,
    })
}

#[allow(clippy::too_many_arguments)]
fn proposal_for_interface(
    inner: &Inner,
    interface: &Interface,
    contract: &ParsedContract,
    hash: &str,
    format: ContractFormat,
    warnings: Vec<String>,
    title: String,
    semantic_version: String,
) -> RepoResult<Resolution> {
    if interface.category != InterfaceCategory::Application {
        return Err(RepoError::new(
            "OAS-007",
            format!(
                "L'interface « {} » n'est pas de catégorie APPLICATION : import OpenAPI refusé.",
                interface.object.name
            ),
        ));
    }

    if let Some(version) = inner.data.interface_versions.iter().find(|version| {
        version.interface_id == interface.object.id
            && version.contract_format == Some(format)
            && version.contract_hash.as_deref() == Some(hash)
    }) {
        return Ok(unchanged(
            version.interface_id,
            version.id,
            warnings,
            title,
            semantic_version,
        ));
    }

    ensure_semver_available(inner, interface.object.id, &semantic_version, None)?;
    let baseline = latest_version(inner, interface.object.id).and_then(|version_id| {
        inner
            .data
            .interface_versions
            .iter()
            .find(|version| version.id == version_id)
    });
    let diff = match baseline {
        Some(version) => {
            archifier_core::diff_snapshots(&version.contract_snapshot, &contract.snapshot())
        }
        None => ContractDiff::unchanged(),
    };
    Ok(Resolution {
        status: OpenApiImportStatus::UpdateProposed,
        interface_id: Some(interface.object.id),
        base_version_id: baseline.map(|version| version.id),
        target_version_id: None,
        diff,
        warnings,
        title,
        semantic_version,
    })
}

fn unchanged(
    interface_id: ObjectId,
    version_id: ObjectId,
    warnings: Vec<String>,
    title: String,
    semantic_version: String,
) -> Resolution {
    Resolution {
        status: OpenApiImportStatus::Unchanged,
        interface_id: Some(interface_id),
        base_version_id: Some(version_id),
        // §9.6 : un aperçu `Unchanged` pointe sur la version existante.
        target_version_id: Some(version_id),
        diff: idempotent_diff(),
        warnings,
        title,
        semantic_version,
    }
}

fn idempotent_diff() -> ContractDiff {
    ContractDiff::from_changes(vec![ContractChange::new(
        "OAS-050",
        "/",
        "Empreinte déjà importée : aucune écriture.",
        DiffClass::Compatible,
    )])
}

fn interface_for(inner: &Inner, interface_id: ObjectId) -> RepoResult<&Interface> {
    let interface = inner
        .find_interface(interface_id)
        .ok_or_else(|| RepoError::not_found("Interface"))?;
    if interface.category != InterfaceCategory::Application {
        return Err(RepoError::new(
            "OAS-007",
            format!(
                "L'interface « {} » n'est pas de catégorie APPLICATION : import OpenAPI refusé.",
                interface.object.name
            ),
        ));
    }
    Ok(interface)
}

fn ensure_semver_available(
    inner: &Inner,
    interface_id: ObjectId,
    semantic_version: &str,
    except_version_id: Option<ObjectId>,
) -> RepoResult<()> {
    let duplicated = inner.data.interface_versions.iter().any(|version| {
        version.interface_id == interface_id
            && version.semantic_version == semantic_version
            && Some(version.id) != except_version_id
    });
    if duplicated {
        return Err(RepoError::new(
            "OAS-004",
            format!(
                "La version {semantic_version} existe déjà sur cette interface : aucune écriture."
            ),
        ));
    }
    Ok(())
}

fn latest_version(inner: &Inner, interface_id: ObjectId) -> Option<ObjectId> {
    inner
        .data
        .interface_versions
        .iter()
        .filter(|version| version.interface_id == interface_id)
        .max_by(|a, b| (a.created_at, a.id).cmp(&(b.created_at, b.id)))
        .map(|version| version.id)
}

fn logical_key_for(title: &str, hash: &str) -> String {
    let key = slugify(title);
    let key = if key.len() < 3 {
        format!("openapi-{}", short_hash(hash))
    } else if key.len() > 80 {
        key.chars()
            .take(80)
            .collect::<String>()
            .trim_matches('-')
            .to_string()
    } else {
        key
    };
    if key.len() < 3 {
        format!("openapi-{}", short_hash(hash))
    } else {
        key
    }
}

fn short_hash(hash: &str) -> String {
    hash.get(..8).unwrap_or(hash).to_string()
}

fn unique_logical_key(inner: &Inner, title: &str, hash: &str) -> String {
    let base = logical_key_for(title, hash);
    if !inner.index.interface_by_key.contains_key(&base) {
        return base;
    }
    for counter in 2..100 {
        let suffix = format!("-{counter}");
        let stem_len = 80usize.saturating_sub(suffix.len());
        let stem = base
            .chars()
            .take(stem_len)
            .collect::<String>()
            .trim_matches('-')
            .to_string();
        let candidate = format!("{stem}{suffix}");
        if !inner.index.interface_by_key.contains_key(&candidate) {
            return candidate;
        }
    }
    format!("openapi-{}", short_hash(hash))
}

// ---------------------------------------------------------------------------
// Écriture (un seul mutate)
// ---------------------------------------------------------------------------

fn apply_resolution(
    inner: &mut Inner,
    repository: &Repository,
    domain_id: ObjectId,
    contract: &ParsedContract,
    resolution: &Resolution,
    breaking: bool,
) -> RepoResult<OpenApiImportOutcome> {
    let now = repository.now();
    let user = repository.current_user();
    let snapshot = contract.snapshot();
    let format = Some(contract.format);
    let raw = Some(contract.raw.clone());
    let hash = Some(contract.hash.clone());

    match resolution.status {
        OpenApiImportStatus::Unchanged => {
            let interface_id = resolution
                .interface_id
                .ok_or_else(|| RepoError::new("OAS-041", "Aperçu incohérent."))?;
            Ok(OpenApiImportOutcome {
                status: OpenApiImportStatus::Unchanged,
                interface_id,
                version_id: resolution.base_version_id,
                written: false,
            })
        }
        OpenApiImportStatus::New => {
            if !inner.active_domain(domain_id) {
                return Err(RepoError::new(
                    "OBJ-003",
                    "Domaine responsable introuvable ou archivé.",
                ));
            }
            let interface_id = repository.new_id();
            let version_id = repository.new_id();
            let key = unique_logical_key(inner, &resolution.title, &contract.hash);
            let object = RepositoryObject {
                id: interface_id,
                object_type: ObjectType::Interface,
                name: resolution.title.clone(),
                responsible_domain_id: domain_id,
                lifecycle: Lifecycle::Active,
                revision: 1,
                created_at: now,
                created_by: user.clone(),
                updated_at: now,
                updated_by: user.clone(),
            };
            let interface = Interface {
                object,
                logical_interface_key: key,
                category: InterfaceCategory::Application,
                provider_domain_id: domain_id,
                contract_type: ContractType::OpenApi,
                notation_bindings: vec![],
            };
            validate_interface(&interface).map_err(RepoError::from_validation)?;
            inner.data.interfaces.push(interface);

            let version = build_version(
                version_id,
                interface_id,
                &resolution.semantic_version,
                breaking,
                None,
                format,
                raw,
                hash,
                snapshot,
                now,
                &user,
            );
            validate_interface_version(&version).map_err(RepoError::from_validation)?;
            inner.data.interface_versions.push(version);

            inner.record(
                interface_id,
                format!("Import OpenAPI : interface « {} »", resolution.title),
                now,
                &user,
            );
            inner.record(
                version_id,
                format!("Import OpenAPI : version {}", resolution.semantic_version),
                now,
                &user,
            );
            Ok(OpenApiImportOutcome {
                status: OpenApiImportStatus::New,
                interface_id,
                version_id: Some(version_id),
                written: true,
            })
        }
        OpenApiImportStatus::UpdateProposed => {
            let interface_id = resolution
                .interface_id
                .ok_or_else(|| RepoError::new("OAS-041", "Aperçu incohérent."))?;
            {
                let interface = inner
                    .find_interface(interface_id)
                    .ok_or_else(|| RepoError::not_found("Interface"))?;
                if interface.category != InterfaceCategory::Application {
                    return Err(RepoError::new(
                        "OAS-007",
                        "L'interface cible n'est pas de catégorie APPLICATION.",
                    ));
                }
                if !interface.object.is_active() {
                    return Err(RepoError::new(
                        "OBJ-007",
                        format!("« {} » est archivé : lecture seule.", interface.object.name),
                    ));
                }
            }

            if let Some(version_id) = resolution.target_version_id {
                {
                    let version = inner
                        .data
                        .interface_versions
                        .iter_mut()
                        .find(|version| version.id == version_id)
                        .ok_or_else(|| RepoError::new("OAS-041", "Version cible disparue."))?;
                    if version.publication_status != PublicationStatus::Draft {
                        return Err(stale_preview());
                    }
                    version.semantic_version = resolution.semantic_version.clone();
                    version.revision += 1;
                    version.breaking_change = version.breaking_change || breaking;
                    version.contract_snapshot = snapshot;
                    version.contract_format = format;
                    version.contract_raw = raw;
                    version.contract_hash = hash;
                }
                set_contract_type(inner, interface_id);
                inner.record(
                    version_id,
                    format!(
                        "Mise à jour OpenAPI : version {}",
                        resolution.semantic_version
                    ),
                    now,
                    &user,
                );
                inner.record(
                    interface_id,
                    format!("Mise à jour OpenAPI de {}", resolution.semantic_version),
                    now,
                    &user,
                );
                return Ok(OpenApiImportOutcome {
                    status: OpenApiImportStatus::UpdateProposed,
                    interface_id,
                    version_id: Some(version_id),
                    written: true,
                });
            }

            let version_id = repository.new_id();
            let version = build_version(
                version_id,
                interface_id,
                &resolution.semantic_version,
                breaking,
                resolution.base_version_id,
                format,
                raw,
                hash,
                snapshot,
                now,
                &user,
            );
            validate_interface_version(&version).map_err(RepoError::from_validation)?;
            inner.data.interface_versions.push(version);
            set_contract_type(inner, interface_id);
            inner.record(
                version_id,
                format!("Nouvelle version OpenAPI {}", resolution.semantic_version),
                now,
                &user,
            );
            inner.record(
                interface_id,
                format!("Nouvelle version OpenAPI {}", resolution.semantic_version),
                now,
                &user,
            );
            Ok(OpenApiImportOutcome {
                status: OpenApiImportStatus::UpdateProposed,
                interface_id,
                version_id: Some(version_id),
                written: true,
            })
        }
    }
}

fn set_contract_type(inner: &mut Inner, interface_id: ObjectId) {
    if let Some(interface) = inner
        .data
        .interfaces
        .iter_mut()
        .find(|interface| interface.object.id == interface_id)
    {
        if interface.contract_type != ContractType::OpenApi {
            interface.contract_type = ContractType::OpenApi;
            interface.object.revision += 1;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn build_version(
    id: ObjectId,
    interface_id: ObjectId,
    semantic_version: &str,
    breaking_change: bool,
    supersedes_version_id: Option<ObjectId>,
    contract_format: Option<ContractFormat>,
    contract_raw: Option<String>,
    contract_hash: Option<String>,
    contract_snapshot: Value,
    now: OffsetDateTime,
    user: &str,
) -> InterfaceVersion {
    InterfaceVersion {
        id,
        interface_id,
        semantic_version: semantic_version.to_string(),
        revision: 1,
        publication_status: PublicationStatus::Draft,
        created_at: now,
        created_by: user.to_string(),
        published_at: None,
        available_from: None,
        available_until: None,
        breaking_change,
        supersedes_version_id,
        contract_snapshot,
        contract_format,
        contract_raw,
        contract_hash,
    }
}

// ---------------------------------------------------------------------------
// Export
// ---------------------------------------------------------------------------

fn render_openapi(interface: &Interface, version: &InterfaceVersion) -> String {
    let snapshot = &version.contract_snapshot;

    let mut info = Map::new();
    info.insert("title".into(), Value::String(interface.object.name.clone()));
    info.insert(
        "version".into(),
        Value::String(version.semantic_version.clone()),
    );

    let mut root = Map::new();
    root.insert("openapi".into(), Value::String("3.1.0".into()));
    root.insert("info".into(), Value::Object(info));

    if let Some(servers) = snapshot.get("servers").and_then(Value::as_array) {
        let servers: Vec<Value> = servers
            .iter()
            .filter_map(Value::as_str)
            .map(|url| json!({ "url": url }))
            .collect();
        if !servers.is_empty() {
            root.insert("servers".into(), Value::Array(servers));
        }
    }

    let paths = render_paths(snapshot);
    if !paths.is_empty() {
        let mut path_object = Map::new();
        for (path, methods) in paths {
            path_object.insert(path, Value::Object(methods));
        }
        root.insert("paths".into(), Value::Object(path_object));
    }

    if let Some(schemas) = snapshot.get("schemas").and_then(Value::as_object) {
        if !schemas.is_empty() {
            let mut components = Map::new();
            components.insert("schemas".into(), Value::Object(schemas.clone()));
            root.insert("components".into(), Value::Object(components));
        }
    }

    if let Some(names) = snapshot.get("security_schemes").and_then(Value::as_array) {
        let requirements: Vec<Value> = names
            .iter()
            .filter_map(Value::as_str)
            .map(|name| {
                let mut requirement = Map::new();
                requirement.insert(name.to_string(), Value::Array(vec![]));
                Value::Object(requirement)
            })
            .collect();
        if !requirements.is_empty() {
            root.insert("security".into(), Value::Array(requirements));
        }
    }

    root.insert(
        "x-architecture-interface-id".into(),
        Value::String(interface.object.id.to_string()),
    );
    root.insert(
        "x-architecture-version-id".into(),
        Value::String(version.id.to_string()),
    );
    root.insert(
        "x-architecture-domain-id".into(),
        Value::String(interface.provider_domain_id.to_string()),
    );
    root.insert(
        "x-architecture-logical-key".into(),
        Value::String(interface.logical_interface_key.clone()),
    );
    root.insert(
        "x-architecture-version".into(),
        Value::String(version.semantic_version.clone()),
    );
    root.insert(
        "x-architecture-generated-by".into(),
        Value::String(EXPORT_GENERATOR.to_string()),
    );

    canonical_json_pretty(&Value::Object(root))
}

fn render_paths(snapshot: &Value) -> BTreeMap<String, Map<String, Value>> {
    let mut paths: BTreeMap<String, Map<String, Value>> = BTreeMap::new();
    let Some(operations) = snapshot.get("operations").and_then(Value::as_array) else {
        return paths;
    };

    for operation in operations {
        let path = operation.get("path").and_then(Value::as_str).unwrap_or("");
        let method = operation
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        if path.is_empty() || method.is_empty() {
            continue;
        }

        let mut body = Map::new();
        if let Some(operation_id) = operation.get("operation_id").and_then(Value::as_str) {
            if !operation_id.is_empty() {
                body.insert(
                    "operationId".into(),
                    Value::String(operation_id.to_string()),
                );
            }
        }
        if let Some(parameters) = operation.get("parameters").and_then(Value::as_array) {
            if !parameters.is_empty() {
                body.insert("parameters".into(), Value::Array(parameters.clone()));
            }
        }
        if let Some(request_body) = operation.get("request_body").filter(|body| !body.is_null()) {
            body.insert("requestBody".into(), request_body.clone());
        }
        body.insert("responses".into(), render_responses(operation));

        paths
            .entry(path.to_string())
            .or_default()
            .insert(method, Value::Object(body));
    }
    paths
}

fn render_responses(operation: &Value) -> Value {
    let mut responses = Map::new();
    if let Some(list) = operation.get("responses").and_then(Value::as_array) {
        for response in list {
            let status = response
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            if status.is_empty() {
                continue;
            }
            let schema_ref = response.get("schema_ref").and_then(Value::as_str);
            let mut entry = Map::new();
            match schema_ref {
                Some(reference) => {
                    entry.insert("description".into(), Value::String(String::new()));
                    entry.insert(
                        "content".into(),
                        json!({
                            "application/json": { "schema": { "$ref": reference } }
                        }),
                    );
                }
                None => {
                    entry.insert("description".into(), Value::String(String::new()));
                }
            }
            responses.insert(status, Value::Object(entry));
        }
    }
    if responses.is_empty() {
        responses.insert(
            "default".into(),
            json!({ "description": "Aucune réponse documentée." }),
        );
    }
    Value::Object(responses)
}

// ---------------------------------------------------------------------------
// Tests unitaires
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::InterfaceFilter;
    use crate::{MemoryStore, Repository};
    use archifier_core::openapi::parse_contract_with_warnings as parse;
    use archifier_core::DiffClass;
    use std::rc::Rc;

    const DOC_V1: &str = r##"{
      "openapi": "3.1.0",
      "info": { "title": "Customer API", "version": "1.0.0", "description": "Clients" },
      "servers": [{ "url": "https://api.example.com/v1" }],
      "paths": {
        "/customers": {
          "get": {
            "operationId": "listCustomers",
            "responses": {
              "200": { "description": "ok", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Customer" } } } }
            }
          }
        },
        "/customers/{id}": {
          "get": {
            "operationId": "getCustomer",
            "parameters": [{ "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }],
            "responses": { "200": { "description": "ok" } }
          }
        }
      },
      "components": { "schemas": { "Customer": { "type": "object", "properties": { "id": { "type": "string" } } } } }
    }"##;

    const DOC_V2: &str = r##"{
      "openapi": "3.1.0",
      "info": { "title": "Customer API", "version": "2.0.0", "description": "Clients (v2)" },
      "paths": {
        "/customers": {
          "get": {
            "operationId": "listCustomers",
            "responses": { "200": { "description": "ok" } }
          }
        }
      },
      "components": { "schemas": { "Customer": { "type": "object", "properties": { "id": { "type": "string" } } } } }
    }"##;

    fn repository() -> Repository {
        Repository::new(Rc::new(MemoryStore::new()), 1_700_000_000_000, 7)
    }

    fn seeded() -> (Repository, ObjectId) {
        let repository = repository();
        let domain_id = repository
            .create_domain("Paiements", "Domaine de test")
            .unwrap();
        (repository, domain_id)
    }

    #[test]
    fn preview_creates_nothing() {
        let (repository, domain_id) = seeded();
        let preview = repository
            .import_openapi_preview(domain_id, DOC_V1)
            .expect("aperçu");
        assert_eq!(preview.status, OpenApiImportStatus::New);
        assert!(preview.interface_id.is_none());
        assert!(preview.base_version_id.is_none());
        assert!(preview.target_version_id.is_none());
        assert_eq!(preview.hash.len(), 64);
        assert_eq!(preview.raw, DOC_V1);
        assert!(preview.warnings.is_empty());
        assert!(repository
            .interfaces(&InterfaceFilter::default())
            .is_empty());
    }

    #[test]
    fn apply_creates_application_interface_and_version() {
        let (repository, domain_id) = seeded();
        let preview = repository
            .import_openapi_preview(domain_id, DOC_V1)
            .unwrap();
        let outcome = repository.apply_openapi_import(&preview, false).unwrap();
        assert!(outcome.written);
        assert_eq!(outcome.status, OpenApiImportStatus::New);
        let version_id = outcome.version_id.expect("version créée");

        let interfaces = repository.interfaces(&InterfaceFilter::default());
        assert_eq!(interfaces.len(), 1);
        assert_eq!(interfaces[0].name, "Customer API");
        assert_eq!(interfaces[0].category, InterfaceCategory::Application);

        let detail = repository.interface_detail(outcome.interface_id).unwrap();
        assert_eq!(detail.interface.contract_type, ContractType::OpenApi);
        let version = detail
            .versions
            .iter()
            .find(|summary| summary.version.id == version_id)
            .expect("version présente")
            .version
            .clone();
        assert_eq!(version.contract_raw.as_deref(), Some(DOC_V1));
        assert_eq!(
            version.contract_hash.as_deref(),
            Some(parse(DOC_V1).unwrap().0.hash.as_str())
        );
        assert_eq!(version.contract_format, Some(ContractFormat::OpenApi31));
        assert_eq!(version.publication_status, PublicationStatus::Draft);
        assert!(version.available_from.is_none());
        assert_eq!(version.revision, 1);
        assert_eq!(version.contract_snapshot["openapi_version"], "3.1.0");
        assert_eq!(version.semantic_version, "1.0.0");
    }

    #[test]
    fn import_is_idempotent_by_fingerprint() {
        let (repository, domain_id) = seeded();
        let preview = repository
            .import_openapi_preview(domain_id, DOC_V1)
            .unwrap();
        let outcome = repository.apply_openapi_import(&preview, false).unwrap();

        let second = repository
            .import_openapi_preview(domain_id, DOC_V1)
            .unwrap();
        assert_eq!(second.status, OpenApiImportStatus::Unchanged);
        assert_eq!(second.interface_id, Some(outcome.interface_id));
        assert_eq!(second.base_version_id, outcome.version_id);
        assert_eq!(second.target_version_id, outcome.version_id);
        assert_eq!(second.diff.classification, DiffClass::Compatible);

        let unchanged = repository.apply_openapi_import(&second, false).unwrap();
        assert!(!unchanged.written);
        assert_eq!(unchanged.version_id, outcome.version_id);
        assert_eq!(repository.interfaces(&InterfaceFilter::default()).len(), 1);
    }

    #[test]
    fn breaking_change_requires_confirmation() {
        let (repository, domain_id) = seeded();
        let first = repository
            .import_openapi_preview(domain_id, DOC_V1)
            .unwrap();
        let first_outcome = repository.apply_openapi_import(&first, false).unwrap();

        let second = repository
            .import_openapi_preview(domain_id, DOC_V2)
            .unwrap();
        assert_eq!(second.status, OpenApiImportStatus::UpdateProposed);
        assert_eq!(second.diff.classification, DiffClass::Breaking);
        assert!(second.target_version_id.is_none());

        let refused = repository.apply_openapi_import(&second, false).unwrap_err();
        assert_eq!(refused.code, "OAS-040");

        let accepted = repository.apply_openapi_import(&second, true).unwrap();
        assert!(accepted.written);
        let detail = repository.interface_detail(accepted.interface_id).unwrap();
        let version = detail
            .versions
            .iter()
            .find(|summary| Some(summary.version.id) == accepted.version_id)
            .expect("nouvelle version")
            .version
            .clone();
        assert!(version.breaking_change);
        assert_eq!(version.supersedes_version_id, first_outcome.version_id);
        assert_eq!(detail.versions.len(), 2);
    }

    #[test]
    fn draft_version_is_updated_in_place_by_extension() {
        let (repository, domain_id) = seeded();
        let first = repository
            .import_openapi_preview(domain_id, DOC_V1)
            .unwrap();
        let outcome = repository.apply_openapi_import(&first, false).unwrap();
        let version_id = outcome.version_id.expect("version");

        let with_extension = DOC_V1
            .replace(
                "\"description\": \"Clients\"",
                "\"description\": \"Clients v1.1\"",
            )
            .replace(
                "\"openapi\": \"3.1.0\",",
                &format!(
                    "\"openapi\": \"3.1.0\", \"x-architecture-version-id\": \"{version_id}\","
                ),
            );
        let second = repository
            .import_openapi_preview(domain_id, &with_extension)
            .unwrap();
        assert_eq!(second.status, OpenApiImportStatus::UpdateProposed);
        assert_eq!(second.target_version_id, Some(version_id));
        assert_eq!(second.base_version_id, Some(version_id));
        let applied = repository.apply_openapi_import(&second, true).unwrap();
        assert_eq!(applied.version_id, Some(version_id));
        let detail = repository.interface_detail(outcome.interface_id).unwrap();
        assert_eq!(detail.versions.len(), 1);
    }

    #[test]
    fn published_version_is_never_rewritten() {
        let (repository, domain_id) = seeded();
        let first = repository
            .import_openapi_preview(domain_id, DOC_V1)
            .unwrap();
        let outcome = repository.apply_openapi_import(&first, false).unwrap();
        let version_id = outcome.version_id.expect("version");
        repository
            .publish_version(
                version_id,
                time::macros::datetime!(2026-01-01 00:00 UTC),
                time::macros::datetime!(2027-01-01 00:00 UTC),
                false,
            )
            .unwrap();

        let with_extension = DOC_V1
            .replace("\"version\": \"1.0.0\"", "\"version\": \"1.0.1\"")
            .replace(
                "\"description\": \"Clients\"",
                "\"description\": \"Clients v1.1\"",
            )
            .replace(
                "\"openapi\": \"3.1.0\",",
                &format!(
                    "\"openapi\": \"3.1.0\", \"x-architecture-version-id\": \"{version_id}\","
                ),
            );
        let second = repository
            .import_openapi_preview(domain_id, &with_extension)
            .unwrap();
        assert_eq!(second.status, OpenApiImportStatus::UpdateProposed);
        assert!(second.target_version_id.is_none());
        assert_eq!(second.base_version_id, Some(version_id));
    }

    #[test]
    fn stale_document_is_refused() {
        let (repository, domain_id) = seeded();
        let mut preview = repository
            .import_openapi_preview(domain_id, DOC_V1)
            .unwrap();
        preview.hash = "0".repeat(64);
        let error = repository
            .apply_openapi_import(&preview, false)
            .unwrap_err();
        assert_eq!(error.code, "OAS-041");
    }

    #[test]
    fn duplicated_semver_is_refused() {
        let (repository, domain_id) = seeded();
        let first = repository
            .import_openapi_preview(domain_id, DOC_V1)
            .unwrap();
        repository.apply_openapi_import(&first, false).unwrap();

        let changed = DOC_V1.replace("\"description\": \"Clients\"", "\"description\": \"Autre\"");
        let second = repository
            .import_openapi_preview(domain_id, &changed)
            .unwrap_err();
        assert_eq!(second.code, "OAS-004");
    }

    #[test]
    fn export_is_byte_reproducible_and_injects_extensions() {
        let (repository, domain_id) = seeded();
        let preview = repository
            .import_openapi_preview(domain_id, DOC_V1)
            .unwrap();
        let outcome = repository.apply_openapi_import(&preview, false).unwrap();
        let version_id = outcome.version_id.expect("version");

        let first = repository.export_openapi(version_id).unwrap();
        let second = repository.export_openapi(version_id).unwrap();
        assert_eq!(first, second);
        assert!(first.ends_with('\n'));
        assert!(!first.contains("x-architecture-snapshot-at"));

        let document: Value = serde_json::from_str(&first).unwrap();
        assert_eq!(document["openapi"], "3.1.0");
        assert_eq!(
            document["x-architecture-interface-id"],
            outcome.interface_id.to_string()
        );
        assert_eq!(
            document["x-architecture-version-id"],
            version_id.to_string()
        );
        assert_eq!(document["x-architecture-domain-id"], domain_id.to_string());
        assert_eq!(document["x-architecture-logical-key"], "customer-api");
        assert_eq!(document["x-architecture-version"], "1.0.0");
        assert_eq!(document["x-architecture-generated-by"], "archifier");
        assert!(document["paths"]["/customers"]["get"].is_object());
    }

    #[test]
    fn export_refuses_non_http_interface() {
        let (repository, domain_id) = seeded();
        let interface_id = repository
            .create_interface(
                domain_id,
                "Interface métier",
                "interface-metier",
                InterfaceCategory::Business,
            )
            .unwrap();
        let version_id = repository
            .create_interface_version(interface_id, "1.0.0", None, None, false, false)
            .unwrap();
        let error = repository.export_openapi(version_id).unwrap_err();
        assert_eq!(error.code, "OAS-060");
    }

    #[test]
    fn export_refuses_version_without_contract() {
        let (repository, domain_id) = seeded();
        let preview = repository
            .import_openapi_preview(domain_id, DOC_V1)
            .unwrap();
        let outcome = repository.apply_openapi_import(&preview, false).unwrap();
        let empty_version = repository
            .create_interface_version(outcome.interface_id, "9.9.9", None, None, false, false)
            .unwrap();
        let error = repository.export_openapi(empty_version).unwrap_err();
        assert_eq!(error.code, "OAS-061");
    }

    #[test]
    fn diff_between_stored_versions_is_available() {
        let (repository, domain_id) = seeded();
        let first = repository
            .import_openapi_preview(domain_id, DOC_V1)
            .unwrap();
        let first_outcome = repository.apply_openapi_import(&first, false).unwrap();
        let second = repository
            .import_openapi_preview(domain_id, DOC_V2)
            .unwrap();
        let second_outcome = repository.apply_openapi_import(&second, true).unwrap();

        let diff = repository
            .diff_openapi_versions(
                first_outcome.version_id.unwrap(),
                second_outcome.version_id.unwrap(),
            )
            .unwrap();
        assert_eq!(diff.classification, DiffClass::Breaking);
    }
}
