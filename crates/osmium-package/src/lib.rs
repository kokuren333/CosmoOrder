//! Bounded source-package I/O. No package code is executed.

pub mod distribution;
pub mod init;
pub mod library;

use osmium_core::parsing::{MAX_DOCUMENT_BYTES, parse_json};
use osmium_core::schema::{Diagnostic, DocumentKind, validate_document};
use osmium_core::validation::{
    PackageDocuments, PackageModel, validate_package, validate_relative_path,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, Metadata, OpenOptions};
use std::io::Read;
use std::path::Path;
use unicode_normalization::UnicodeNormalization;

pub const MAX_SOURCE_FILES: usize = 4096;
pub const MAX_SOURCE_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_SOURCE_DEPTH: usize = 32;

#[derive(Debug)]
pub struct LoadedSource {
    pub model: PackageModel,
    /// Exact bytes of manifest, entity files and resource files only.
    /// Unreferenced author notes are not distribution payloads.
    pub files: BTreeMap<String, Vec<u8>>,
}

fn diagnostic(file: &str, code: &str, message: impl Into<String>) -> Vec<Diagnostic> {
    vec![Diagnostic {
        code: code.into(),
        severity: "error".into(),
        entity_type: None,
        entity_id: None,
        file: Some(file.into()),
        line: None,
        column: None,
        path: String::new(),
        message: message.into(),
        suggestions: Vec::new(),
    }]
}

fn io_error(path: &Path, error: std::io::Error) -> Vec<Diagnostic> {
    diagnostic(&path.to_string_lossy(), "OSM_IO", error.to_string())
}

fn reject_link(meta: &Metadata, path: &Path) -> Result<(), Vec<Diagnostic>> {
    #[cfg(windows)]
    let is_reparse = {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & 0x400 != 0
    };
    #[cfg(not(windows))]
    let is_reparse = false;
    if meta.file_type().is_symlink() || is_reparse {
        return Err(diagnostic(
            &path.to_string_lossy(),
            "OSM_LINK",
            "symlinks and reparse points are not allowed",
        ));
    }
    Ok(())
}

/// Inventory first, without following links or reading file contents.
fn inventory(root: &Path) -> Result<BTreeMap<String, u64>, Vec<Diagnostic>> {
    let mut pending = vec![(root.to_path_buf(), 0)];
    let mut files = BTreeMap::new();
    let mut portable_names = BTreeSet::new();
    let mut total_bytes = 0u64;
    let mut total_entries = 0usize;
    while let Some((directory, depth)) = pending.pop() {
        if depth > MAX_SOURCE_DEPTH {
            return Err(diagnostic(
                "source",
                "OSM_INPUT_LIMIT",
                "source directory depth exceeds limit",
            ));
        }
        let meta = fs::symlink_metadata(&directory).map_err(|e| io_error(&directory, e))?;
        reject_link(&meta, &directory)?;
        for entry in fs::read_dir(&directory).map_err(|e| io_error(&directory, e))? {
            let entry = entry.map_err(|e| io_error(&directory, e))?;
            let path = entry.path();
            // A source may be a Git checkout. Repository internals are never
            // package payload, and are not traversed.
            if depth == 0 && entry.file_name() == ".git" {
                continue;
            }
            total_entries += 1;
            if total_entries > MAX_SOURCE_FILES {
                return Err(diagnostic(
                    "source",
                    "OSM_INPUT_LIMIT",
                    "source entry count exceeds limit",
                ));
            }
            let relative = path.strip_prefix(root).unwrap();
            let name = relative
                .to_str()
                .ok_or_else(|| diagnostic("source", "OSM_PATH", "non-UTF-8 path"))?
                .replace('\\', "/");
            validate_relative_path(&name)
                .map_err(|reason| diagnostic(&name, "OSM_PATH", reason))?;
            let portable = name.nfc().collect::<String>().to_lowercase();
            if !portable_names.insert(portable) {
                return Err(diagnostic(
                    &name,
                    "OSM_PATH_COLLISION",
                    "case or Unicode-equivalent source paths collide",
                ));
            }
            let meta = fs::symlink_metadata(&path).map_err(|e| io_error(&path, e))?;
            reject_link(&meta, &path)?;
            if meta.is_dir() {
                pending.push((path, depth + 1));
            } else if meta.is_file() {
                total_bytes = total_bytes.checked_add(meta.len()).ok_or_else(|| {
                    diagnostic("source", "OSM_INPUT_LIMIT", "source size overflow")
                })?;
                if total_bytes > MAX_SOURCE_BYTES {
                    return Err(diagnostic(
                        "source",
                        "OSM_INPUT_LIMIT",
                        "source byte count exceeds limit",
                    ));
                }
                files.insert(name, meta.len());
            } else {
                return Err(diagnostic(
                    &name,
                    "OSM_FILE_TYPE",
                    "only regular files and directories are allowed",
                ));
            }
        }
    }
    Ok(files)
}

fn read_checked(
    root: &Path,
    name: &str,
    inventory: &BTreeMap<String, u64>,
) -> Result<Vec<u8>, Vec<Diagnostic>> {
    validate_relative_path(name).map_err(|reason| diagnostic(name, "OSM_PATH", reason))?;
    let expected = inventory.get(name).ok_or_else(|| {
        diagnostic(
            name,
            "OSM_MISSING_FILE",
            "referenced file does not exist with this exact spelling",
        )
    })?;
    if *expected > MAX_DOCUMENT_BYTES as u64 {
        return Err(diagnostic(
            name,
            "OSM_INPUT_LIMIT",
            "individual source file exceeds 4 MiB",
        ));
    }
    let mut path = root.to_path_buf();
    for component in name.split('/') {
        path.push(component);
        let meta = fs::symlink_metadata(&path).map_err(|e| io_error(&path, e))?;
        reject_link(&meta, &path)?;
    }
    let resolved = path.canonicalize().map_err(|e| io_error(&path, e))?;
    if !resolved.starts_with(root) {
        return Err(diagnostic(
            name,
            "OSM_PATH",
            "referenced file escapes source root",
        ));
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // Inspect a reparse point itself, rather than following a replaced leaf.
        options.custom_flags(0x00200000);
    }
    let file = options.open(&path).map_err(|e| io_error(&path, e))?;
    let meta = file.metadata().map_err(|e| io_error(&path, e))?;
    reject_link(&meta, &path)?;
    if !meta.is_file() {
        return Err(diagnostic(
            name,
            "OSM_FILE_TYPE",
            "referenced input is not a regular file",
        ));
    }
    let mut bytes = Vec::new();
    file.take(MAX_DOCUMENT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| io_error(&path, e))?;
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err(diagnostic(
            name,
            "OSM_INPUT_LIMIT",
            "file grew beyond the 4 MiB limit",
        ));
    }
    if bytes.len() as u64 != *expected
        || path.canonicalize().map_err(|e| io_error(&path, e))? != resolved
    {
        return Err(diagnostic(
            name,
            "OSM_SOURCE_CHANGED",
            "source changed during loading; retry with a stable source",
        ));
    }
    Ok(bytes)
}

/// Load a JSON or YAML source manifest and its declared files without mutation.
pub fn load_source(path: impl AsRef<Path>) -> Result<LoadedSource, Vec<Diagnostic>> {
    let input = path.as_ref();
    let meta = fs::symlink_metadata(input).map_err(|e| io_error(input, e))?;
    reject_link(&meta, input)?;
    if !meta.is_dir() {
        return Err(diagnostic(
            "source",
            "OSM_FILE_TYPE",
            "source must be a directory",
        ));
    }
    let root = input.canonicalize().map_err(|e| io_error(input, e))?;
    let entries = inventory(&root)?;
    if entries.contains_key("osmium.yaml") && entries.contains_key("osmium.json") {
        return Err(diagnostic(
            "source",
            "OSM_MANIFEST",
            "source must contain exactly one manifest",
        ));
    }
    let manifest_name = if entries.contains_key("osmium.yaml") {
        "osmium.yaml"
    } else {
        "osmium.json"
    };
    let manifest_bytes = read_checked(&root, manifest_name, &entries)?;
    let manifest = if manifest_name.ends_with(".yaml") {
        osmium_core::yaml::parse_yaml(&manifest_bytes, manifest_name)
    } else {
        parse_json(&manifest_bytes, manifest_name)
    }
    .map_err(|e| vec![*e])?;
    if manifest
        .get("schema_version")
        .is_some_and(|v| v != osmium_core::schema::SCHEMA_VERSION)
    {
        let mut errors = diagnostic(
            manifest_name,
            "OSM_SCHEMA_VERSION",
            "unsupported schema version",
        );
        errors[0].path = "/schema_version".into();
        return Err(errors);
    }
    let mut diagnostics = validate_document(DocumentKind::Manifest, &manifest);
    if !diagnostics.is_empty() {
        for error in &mut diagnostics {
            error.file = Some(manifest_name.into());
        }
        return Err(diagnostics);
    }
    let mut files = BTreeMap::from([(manifest_name.to_owned(), manifest_bytes)]);
    let mut documents = BTreeMap::new();
    for (kind, path) in manifest["entities"].as_object().unwrap() {
        let name = path.as_str().unwrap();
        if !name.ends_with(".json") {
            return Err(diagnostic(
                name,
                "OSM_FILE_TYPE",
                "entity documents must be JSON",
            ));
        }
        let bytes = read_checked(&root, name, &entries)?;
        let value = parse_json(&bytes, name).map_err(|e| vec![*e])?;
        files.insert(name.into(), bytes);
        documents.insert(kind.as_str(), value);
    }
    let take = |key: &str| -> Value { documents[key].clone() };
    let model = validate_package(PackageDocuments {
        manifest: manifest.clone(),
        concepts: take("concepts"),
        objectives: take("objectives"),
        curricula: take("curricula"),
        resources: take("resources"),
        assessments: take("assessments"),
    })
    .map_err(|mut errors| {
        for error in &mut errors {
            if let Some(kind) = error.file.as_deref() {
                error.file = Some(if kind == "manifest" {
                    manifest_name.into()
                } else {
                    manifest["entities"][kind].as_str().unwrap_or(kind).into()
                });
            }
        }
        errors
    })?;
    for resource in model.documents().resources.as_array().unwrap() {
        let name = resource["path"].as_str().unwrap();
        if !name.ends_with(".md") {
            return Err(diagnostic(
                name,
                "OSM_FILE_TYPE",
                "Markdown resources must use .md files",
            ));
        }
        if !files.contains_key(name) {
            let bytes = read_checked(&root, name, &entries)?;
            std::str::from_utf8(&bytes)
                .map_err(|_| diagnostic(name, "OSM_UTF8", "Markdown must be UTF-8"))?;
            files.insert(name.into(), bytes);
        }
    }
    Ok(LoadedSource { model, files })
}
