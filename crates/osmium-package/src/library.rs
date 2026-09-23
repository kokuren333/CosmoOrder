//! Local immutable package files. The lock coordinates Osmium processes;
//! it does not defend against a hostile process controlling the user's home.

use crate::distribution::{Distribution, read_distribution};
use crate::{diagnostic, io_error, reject_link};
use osmium_core::schema::Diagnostic;
use serde::Serialize;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub const MAX_INSTALLED_PACKAGES: usize = 1024;

#[derive(Debug, Clone, Serialize)]
pub struct InstalledPackage {
    pub package_id: String,
    pub package_version: String,
    pub schema_version: String,
    pub title: String,
    /// Pure Core summary of the validated package contents.
    pub entity_counts: std::collections::BTreeMap<String, usize>,
    pub digest: String,
    pub path: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct InstallReport {
    pub package: InstalledPackage,
    pub already_installed: bool,
}

#[derive(Debug)]
pub struct Library {
    home: PathBuf,
    // Closing the handle releases the OS advisory lock, including on crashes.
    _lock: fs::File,
}

fn error(code: &str, message: &str) -> Vec<Diagnostic> {
    diagnostic("library", code, message)
}

pub fn default_home() -> Result<PathBuf, Vec<Diagnostic>> {
    if let Some(value) = std::env::var_os("OSMIUM_HOME") {
        if value.is_empty() {
            return Err(error("OSM_IO", "OSMIUM_HOME must not be empty"));
        }
        return Ok(PathBuf::from(value));
    }
    #[cfg(windows)]
    if let Some(value) = std::env::var_os("LOCALAPPDATA").filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(value).join("Osmium"));
    }
    #[cfg(not(windows))]
    {
        if let Some(value) = std::env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
            let path = PathBuf::from(value);
            if !path.is_absolute() {
                return Err(error("OSM_IO", "XDG_DATA_HOME must be absolute"));
            }
            return Ok(path.join("osmium"));
        }
        if let Some(value) = std::env::var_os("HOME").filter(|v| !v.is_empty()) {
            return Ok(PathBuf::from(value).join(".local/share/osmium"));
        }
    }
    Err(error(
        "OSM_IO",
        "cannot determine data directory; specify --home or OSMIUM_HOME",
    ))
}

fn ensure_directory(path: &Path) -> Result<(), Vec<Diagnostic>> {
    for ancestor in path.ancestors().filter(|p| !p.as_os_str().is_empty()) {
        match fs::symlink_metadata(ancestor) {
            Ok(meta) => {
                reject_link(&meta, ancestor)?;
                if !meta.is_dir() {
                    return Err(error(
                        "OSM_IO",
                        "data directory ancestor is not a directory",
                    ));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(io_error(ancestor, e)),
        }
    }
    fs::create_dir_all(path).map_err(|e| io_error(path, e))?;
    reject_link(
        &fs::symlink_metadata(path).map_err(|e| io_error(path, e))?,
        path,
    )
}

fn is_digest(name: &str) -> bool {
    name.len() == 64
        && name
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn record(package: &Distribution, path: PathBuf) -> InstalledPackage {
    let manifest = &package.model.documents().manifest;
    InstalledPackage {
        package_id: manifest["package_id"].as_str().unwrap().into(),
        package_version: manifest["package_version"].as_str().unwrap().into(),
        schema_version: manifest["schema_version"].as_str().unwrap().into(),
        title: manifest["title"].as_str().unwrap().into(),
        entity_counts: osmium_core::query::inspect(&package.model, 0)
            .manifest
            .entity_counts,
        digest: package.digest.clone(),
        path,
    }
}

impl Library {
    pub fn open(home: &Path) -> Result<Self, Vec<Diagnostic>> {
        if home.as_os_str().is_empty() {
            return Err(error("OSM_IO", "data directory must not be empty"));
        }
        ensure_directory(home)?;
        let home = home.canonicalize().map_err(|e| io_error(home, e))?;
        let lock_path = home.join(".library.lock");
        match fs::symlink_metadata(&lock_path) {
            Ok(meta) => {
                reject_link(&meta, &lock_path)?;
                if !meta.is_file() {
                    return Err(error("OSM_IO", "library lock is not a regular file"));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(io_error(&lock_path, e)),
        }
        let mut options = fs::OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.custom_flags(0x00200000); // FILE_FLAG_OPEN_REPARSE_POINT
        }
        let lock = options
            .open(&lock_path)
            .map_err(|e| io_error(&lock_path, e))?;
        reject_link(
            &lock.metadata().map_err(|e| io_error(&lock_path, e))?,
            &lock_path,
        )?;
        fs2::FileExt::try_lock_exclusive(&lock).map_err(|e| io_error(&lock_path, e))?;
        ensure_directory(&home.join("library"))?;
        ensure_directory(&home.join("staging"))?;
        Ok(Self { home, _lock: lock })
    }

    pub fn home(&self) -> &Path {
        &self.home
    }

    /// Check a Runtime-owned file before handing its path to a storage adapter.
    pub fn checked_data_file(&self, name: &str) -> Result<PathBuf, Vec<Diagnostic>> {
        if name.contains('/') || osmium_core::validation::validate_relative_path(name).is_err() {
            return Err(error(
                "OSM_PATH",
                "data file must have a portable single-component name",
            ));
        }
        let path = self.home.join(name);
        match fs::symlink_metadata(&path) {
            Ok(meta) => {
                reject_link(&meta, &path)?;
                if !meta.is_file() {
                    return Err(error("OSM_IO", "data file is not regular"));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(io_error(&path, e)),
        }
        Ok(path)
    }

    /// Fully revalidate files before exposing metadata, also discovering any
    /// complete package renamed before a later database registration failed.
    pub fn packages(&self) -> Result<Vec<InstalledPackage>, Vec<Diagnostic>> {
        let root = self.home.join("library");
        let mut packages = Vec::new();
        let mut identities = std::collections::BTreeSet::new();
        for entry in fs::read_dir(&root).map_err(|e| io_error(&root, e))? {
            let entry = entry.map_err(|e| io_error(&root, e))?;
            if packages.len() >= MAX_INSTALLED_PACKAGES {
                return Err(error("OSM_INPUT_LIMIT", "too many installed packages"));
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            let meta =
                fs::symlink_metadata(entry.path()).map_err(|e| io_error(&entry.path(), e))?;
            reject_link(&meta, &entry.path())?;
            if !meta.is_dir() {
                return Err(error(
                    "OSM_LIBRARY",
                    "installed package must be a directory",
                ));
            }
            if !is_digest(&name) {
                return Err(error("OSM_LIBRARY", "unexpected entry in package library"));
            }
            let package = read_distribution(&entry.path())?;
            if package.digest != name {
                return Err(error(
                    "OSM_HASH",
                    "installed package directory digest mismatch",
                ));
            }
            let record = record(&package, entry.path());
            if !identities.insert((record.package_id.clone(), record.package_version.clone())) {
                return Err(error(
                    "OSM_VERSION_CONFLICT",
                    "multiple contents installed for one ID/version",
                ));
            }
            packages.push(record);
        }
        packages.sort_by(|a, b| {
            (&a.package_id, &a.package_version).cmp(&(&b.package_id, &b.package_version))
        });
        Ok(packages)
    }

    pub fn read(
        &self,
        package_id: &str,
        version: Option<&str>,
    ) -> Result<Distribution, Vec<Diagnostic>> {
        let mut matches = self.packages()?.into_iter().filter(|p| {
            p.package_id == package_id && version.is_none_or(|v| p.package_version == v)
        });
        let selected = matches
            .next()
            .ok_or_else(|| error("OSM_NOT_INSTALLED", "requested package is not installed"))?;
        if matches.next().is_some() {
            return Err(error(
                "OSM_VERSION_REQUIRED",
                "multiple versions installed; specify --package-version",
            ));
        }
        read_distribution(&selected.path)
    }

    pub fn install(&mut self, input: &Path) -> Result<InstallReport, Vec<Diagnostic>> {
        let incoming = read_distribution(input)?;
        self.install_distribution(incoming)
    }

    /// Install exactly this in-memory snapshot; revalidate public file values.
    pub fn install_distribution(
        &mut self,
        incoming: Distribution,
    ) -> Result<InstallReport, Vec<Diagnostic>> {
        let incoming = crate::distribution::verify_files(incoming.files)?;
        let destination = self.home.join("library").join(&incoming.digest);
        let incoming_record = record(&incoming, destination.clone());
        let packages = self.packages()?;
        for package in &packages {
            if package.package_id == incoming_record.package_id
                && package.package_version == incoming_record.package_version
            {
                if package.digest != incoming_record.digest {
                    return Err(error(
                        "OSM_VERSION_CONFLICT",
                        "same package ID/version has different content; publish a new version",
                    ));
                }
                return Ok(InstallReport {
                    package: package.clone(),
                    already_installed: true,
                });
            }
        }
        if packages.len() >= MAX_INSTALLED_PACKAGES {
            return Err(error("OSM_INPUT_LIMIT", "too many installed packages"));
        }
        let staging = self.home.join("staging");
        let temporary = tempfile::tempdir_in(&staging).map_err(|e| io_error(&staging, e))?;
        for (name, bytes) in &incoming.files {
            let path = temporary.path().join(name);
            fs::create_dir_all(path.parent().unwrap()).map_err(|e| io_error(&path, e))?;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .map_err(|e| io_error(&path, e))?;
            file.write_all(bytes).map_err(|e| io_error(&path, e))?;
            file.sync_all().map_err(|e| io_error(&path, e))?;
        }
        let verified = read_distribution(temporary.path())?;
        if verified.digest != incoming.digest {
            return Err(error("OSM_HASH", "staging content changed"));
        }
        if fs::symlink_metadata(&destination).is_ok() {
            return Err(error(
                "OSM_OUTPUT_EXISTS",
                "install destination already exists",
            ));
        }
        fs::rename(temporary.path(), &destination).map_err(|e| io_error(&destination, e))?;
        Ok(InstallReport {
            package: incoming_record,
            already_installed: false,
        })
    }
}
