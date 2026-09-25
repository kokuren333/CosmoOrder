//! Shared application operations for CLI and Desktop. Each operation session
//! holds the library lock; the UI must open a session per request, not per app.

use crate::{AttemptRequest, AttemptResult, ObjectiveProgress, Result, Store};
use osmium_package::{
    distribution::{Distribution, read_distribution},
    library::{InstallReport, InstalledPackage, Library},
};
use serde_json::Value;
use std::path::Path;

pub struct Runtime {
    store: Store,
    library: Library,
}

impl Runtime {
    pub fn lesson(&self, package_id: &str, version: Option<&str>) -> Result<Value> {
        let package = self.read(package_id, version)?;
        let documents = package.model.documents();
        Ok(
            serde_json::json!({"manifest":documents.manifest, "digest":package.digest, "concepts":documents.concepts, "objectives":documents.objectives, "curricula":documents.curricula, "resources":documents.resources, "assessments":documents.assessments}),
        )
    }

    pub fn resource(
        &self,
        package_id: &str,
        version: Option<&str>,
        resource_id: &str,
    ) -> Result<Value> {
        let package = self.read(package_id, version)?;
        let documents = package.model.documents();
        let resource = documents
            .resources
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == resource_id)
            .ok_or_else(|| crate::failure("OSM_RESOURCE", "resource does not exist"))?;
        let markdown = std::str::from_utf8(&package.files[resource["path"].as_str().unwrap()])
            .map_err(crate::db)?;
        let evidence: std::collections::BTreeSet<&str> =
            osmium_core::reference::evidence_ids(resource).collect();
        // Project only learner-safe, resource-linked Reference metadata. The
        // Authoring Workspace is never loaded by the package layer at all, and
        // this projection is the second boundary: even a record that survived
        // into a distribution is dropped here unless it is record-public.
        let references = osmium_core::reference::registry(&documents.manifest)
            .into_iter()
            .flatten()
            .filter(|record| osmium_core::reference::visibility(record).is_record_public())
            .filter(|record| {
                record["id"]
                    .as_str()
                    .is_some_and(|id| evidence.contains(id))
            })
            .map(|record| {
                let mut projected = serde_json::Map::new();
                for field in [
                    "id",
                    "title",
                    "citation",
                    "visibility",
                    "record_visibility",
                    "locator_visibility",
                    "type",
                    "publisher",
                    "authors",
                    "published_at",
                    "updated_at",
                    "accessed_at",
                    "version",
                    "edition",
                    "identifiers",
                ] {
                    if let Some(value) = record.get(field) {
                        projected.insert(field.to_owned(), value.clone());
                    }
                }
                // Only a public locator may reach a learner-facing DTO.
                if osmium_core::reference::visibility(record).is_locator_public()
                    && let Some(locator) = record.get("locator")
                {
                    projected.insert("locator".to_owned(), locator.clone());
                }
                Value::Object(projected)
            })
            .collect::<Vec<_>>();
        Ok(
            serde_json::json!({"resource":resource, "markdown":markdown, "references":references, "content_is_untrusted":true}),
        )
    }
    pub fn open(home: &Path) -> Result<Self> {
        let library = Library::open(home)?;
        let mut store = Store::open(&library)?;
        // Keep the session usable when one payload is damaged so its manifest
        // identity can still be selected for uninstall. Strict reads/listing
        // continue to surface the integrity diagnostics.
        store.sync_packages(&library.valid_packages()?)?;
        Ok(Self { store, library })
    }

    pub fn install(&mut self, path: &Path) -> Result<InstallReport> {
        let incoming = read_distribution(path)?;
        let manifest = &incoming.model.documents().manifest;
        self.store.check_identity(
            manifest["package_id"].as_str().unwrap(),
            manifest["package_version"].as_str().unwrap(),
            &incoming.digest,
        )?;
        let installed = self.library.install_distribution(incoming)?;
        self.store.sync_packages(&self.library.packages()?)?;
        Ok(installed)
    }

    /// Remove only the selected package payload. Store keeps its append-only
    /// learning events and digest-scoped progress projection for a later
    /// reinstall of the same version.
    pub fn uninstall(
        &mut self,
        package_id: &str,
        version: &str,
    ) -> Result<osmium_package::library::UninstallReport> {
        let report = self.library.uninstall(package_id, version)?;
        // Uninstall must remain usable when another installed payload is
        // damaged. The inventory can still show that payload for removal;
        // only fully verified packages participate in runtime availability.
        self.store.sync_packages(&self.library.valid_packages()?)?;
        Ok(report)
    }

    pub fn packages(&self) -> Result<Vec<InstalledPackage>> {
        self.library.packages()
    }
    pub fn package_inventory(&self) -> Result<Vec<osmium_package::library::PackageInventoryItem>> {
        self.library.inventory()
    }
    pub fn read(&self, package_id: &str, version: Option<&str>) -> Result<Distribution> {
        self.library.read(package_id, version)
    }
    /// Return a bounded Core-owned neighborhood within one installed Package.
    /// The Runtime selects/verifies the Package; Core defines entity and edge
    /// semantics. No Package or Store data is modified.
    pub fn context(
        &self,
        package_id: &str,
        version: Option<&str>,
        kind: &str,
        entity_id: &str,
        depth: usize,
        node_limit: usize,
    ) -> Result<osmium_core::query::ContextView> {
        let kind = osmium_core::query::EntityKind::parse(kind).ok_or_else(|| {
            crate::failure("OSM_UNKNOWN_ENTITY", format!("unknown entity kind: {kind}"))
        })?;
        let package = self.read(package_id, version)?;
        osmium_core::query::context(&package.model, kind, entity_id, depth, node_limit)
    }
    /// Search Concept titles within one installed Package using Core's
    /// bounded, deterministic read model.
    pub fn search_concepts(
        &self,
        package_id: &str,
        version: Option<&str>,
        needle: &str,
        limit: usize,
    ) -> Result<osmium_core::query::ConceptSearchView> {
        let package = self.read(package_id, version)?;
        osmium_core::query::search_concepts(&package.model, needle, limit)
    }
    pub fn answer(
        &mut self,
        package_id: &str,
        version: Option<&str>,
        request: &AttemptRequest,
    ) -> Result<AttemptResult> {
        let package = self.read(package_id, version)?;
        self.store.submit(&package, request)
    }
    pub fn progress(
        &self,
        package_id: &str,
        version: Option<&str>,
    ) -> Result<Vec<ObjectiveProgress>> {
        self.store.progress(&self.read(package_id, version)?)
    }
    pub fn history(
        &self,
        package_id: &str,
        version: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Value>> {
        self.store.history(package_id, version, limit, offset)
    }
    pub fn history_all(&self, limit: usize, offset: usize) -> Result<Vec<Value>> {
        self.store.history_all(limit, offset)
    }
    pub fn rebuild_progress(&mut self) -> Result<usize> {
        self.store.rebuild_progress()
    }
    pub fn export_state(&mut self, output: &Path) -> Result<usize> {
        self.store.export_state(output)
    }
    pub fn backup(&self, output: &Path) -> Result<()> {
        self.store.backup(output)
    }
    pub fn home(&self) -> &Path {
        self.library.home()
    }
    pub fn database_path(&self) -> &Path {
        self.store.path()
    }
}
