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
        let source_ids = resource["source_ids"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        // Project only learner-safe, resource-linked source metadata. Private
        // source records are removed here as a second boundary after build.
        let sources = documents.manifest["sources"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|source| {
                source["visibility"] == "public" || source["visibility"] == "attribution_only"
            })
            .filter(|source| {
                source["id"]
                    .as_str()
                    .is_some_and(|id| source_ids.contains(id))
            })
            .map(|source| {
                let mut projected = serde_json::Map::new();
                for field in ["id", "title", "citation", "visibility"] {
                    if let Some(value) = source.get(field) {
                        projected.insert(field.to_owned(), value.clone());
                    }
                }
                if source["visibility"] == "public"
                    && let Some(locator) = source.get("locator")
                {
                    projected.insert("locator".to_owned(), locator.clone());
                }
                Value::Object(projected)
            })
            .collect::<Vec<_>>();
        Ok(
            serde_json::json!({"resource":resource, "markdown":markdown, "sources":sources, "content_is_untrusted":true}),
        )
    }
    pub fn open(home: &Path) -> Result<Self> {
        let library = Library::open(home)?;
        let mut store = Store::open(&library)?;
        store.sync_packages(&library.packages()?)?;
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

    pub fn packages(&self) -> Result<Vec<InstalledPackage>> {
        self.library.packages()
    }
    pub fn read(&self, package_id: &str, version: Option<&str>) -> Result<Distribution> {
        self.library.read(package_id, version)
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
