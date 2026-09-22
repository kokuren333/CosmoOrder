//! Reproducible distribution packages with verified payloads. Archives are
//! inspected in memory, never extracted using archive-supplied filesystem paths.

use crate::{
    MAX_SOURCE_BYTES, MAX_SOURCE_FILES, diagnostic, inventory, io_error, load_source, read_checked,
    reject_link,
};
use osmium_core::parsing::{MAX_DOCUMENT_BYTES, parse_json};
use osmium_core::schema::Diagnostic;
use osmium_core::validation::{
    PackageDocuments, PackageModel, validate_package, validate_relative_path,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Cursor, Read, Write},
    path::Path,
};
use unicode_normalization::UnicodeNormalization;
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

pub const MAX_ARCHIVE_BYTES: u64 = MAX_SOURCE_BYTES + 4 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileRecord {
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DistributionManifest {
    pub distribution_version: String,
    pub canonicalization: String,
    pub package: Value,
    pub files: BTreeMap<String, FileRecord>,
}

#[derive(Debug)]
pub struct Distribution {
    pub model: PackageModel,
    pub digest: String,
    pub files: BTreeMap<String, Vec<u8>>,
}

#[derive(Debug, Serialize)]
pub struct BuildReport {
    pub package_id: String,
    pub package_version: String,
    pub digest: String,
    pub files: usize,
    pub output: String,
    pub archive_sha256: Option<String>,
}

pub fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// osmium-json-0.1: recursively sorted UTF-8 keys, compact serde_json values,
/// one final LF. This is a versioned build profile, not an RFC 8785 claim.
pub fn canonical_json(value: &Value) -> Vec<u8> {
    fn sorted(value: &Value) -> Value {
        match value {
            Value::Object(map) => {
                let mut pairs: Vec<_> = map.iter().collect();
                pairs.sort_by(|a, b| a.0.cmp(b.0));
                Value::Object(
                    pairs
                        .into_iter()
                        .map(|(k, v)| (k.clone(), sorted(v)))
                        .collect(),
                )
            }
            Value::Array(items) => Value::Array(items.iter().map(sorted).collect()),
            other => other.clone(),
        }
    }
    let mut bytes = serde_json::to_vec(&sorted(value)).expect("JSON values serialize");
    bytes.push(b'\n');
    bytes
}

fn invalid(message: impl Into<String>) -> Vec<Diagnostic> {
    diagnostic("manifest.json", "OSM_DISTRIBUTION", message)
}

pub(crate) fn verify_files(
    files: BTreeMap<String, Vec<u8>>,
) -> Result<Distribution, Vec<Diagnostic>> {
    if files.len() > MAX_SOURCE_FILES
        || files.values().map(|b| b.len() as u64).sum::<u64>() > MAX_SOURCE_BYTES
    {
        return Err(diagnostic(
            "distribution",
            "OSM_INPUT_LIMIT",
            "distribution exceeds file/byte limits",
        ));
    }
    let mut names = BTreeSet::new();
    for (name, bytes) in &files {
        validate_relative_path(name).map_err(|e| diagnostic(name, "OSM_PATH", e))?;
        if name.split('/').count() > 33 || bytes.len() > MAX_DOCUMENT_BYTES {
            return Err(diagnostic(
                name,
                "OSM_INPUT_LIMIT",
                "distribution file exceeds depth/byte limit",
            ));
        }
        if !names.insert(name.nfc().collect::<String>().to_lowercase()) {
            return Err(diagnostic(
                name,
                "OSM_PATH_COLLISION",
                "portable paths collide",
            ));
        }
    }
    let bytes = files
        .get("manifest.json")
        .ok_or_else(|| invalid("missing manifest.json"))?;
    let value = parse_json(bytes, "manifest.json").map_err(|e| vec![*e])?;
    let manifest: DistributionManifest =
        serde_json::from_value(value.clone()).map_err(|e| invalid(e.to_string()))?;
    if manifest.distribution_version != "0.1" || manifest.canonicalization != "osmium-json-0.1" {
        return Err(diagnostic(
            "manifest.json",
            "OSM_SCHEMA_VERSION",
            "unsupported distribution profile",
        ));
    }
    if manifest
        .package
        .get("schema_version")
        .and_then(Value::as_str)
        .is_some_and(|v| v != "0.1")
    {
        return Err(diagnostic(
            "manifest.json",
            "OSM_SCHEMA_VERSION",
            "unsupported package schema",
        ));
    }
    let errors = osmium_core::schema::validate_document(
        osmium_core::schema::DocumentKind::DistributionManifest,
        &value,
    );
    if !errors.is_empty() {
        return Err(errors);
    }
    if canonical_json(&value) != *bytes {
        return Err(invalid("distribution manifest is not canonical"));
    }
    if manifest.files.len() + 1 != files.len() || manifest.files.contains_key("manifest.json") {
        return Err(invalid("payload inventory does not match files"));
    }
    for (name, record) in &manifest.files {
        let payload = files
            .get(name)
            .ok_or_else(|| invalid(format!("missing payload: {name}")))?;
        if record.size != payload.len() as u64 || record.sha256 != sha256(payload) {
            return Err(diagnostic(
                name,
                "OSM_HASH",
                "payload size or digest mismatch",
            ));
        }
    }
    let source_manifest = &manifest.package;
    let errors = osmium_core::schema::validate_document(
        osmium_core::schema::DocumentKind::Manifest,
        source_manifest,
    );
    if !errors.is_empty() {
        return Err(errors);
    }
    let mut used = BTreeSet::from(["manifest.json".to_owned()]);
    let mut documents = BTreeMap::new();
    for (kind, path) in source_manifest["entities"].as_object().unwrap() {
        let path = path.as_str().unwrap();
        if !path.ends_with(".json") || path == "manifest.json" {
            return Err(invalid("invalid entity document path"));
        }
        let bytes = files
            .get(path)
            .ok_or_else(|| invalid(format!("missing entity document: {path}")))?;
        let value = parse_json(bytes, path).map_err(|e| vec![*e])?;
        if canonical_json(&value) != *bytes {
            return Err(invalid(format!("entity document is not canonical: {path}")));
        }
        used.insert(path.to_owned());
        documents.insert(kind.as_str(), value);
    }
    let model = validate_package(PackageDocuments {
        manifest: source_manifest.clone(),
        concepts: documents["concepts"].clone(),
        objectives: documents["objectives"].clone(),
        curricula: documents["curricula"].clone(),
        resources: documents["resources"].clone(),
        assessments: documents["assessments"].clone(),
    })?;
    for resource in model.documents().resources.as_array().unwrap() {
        let path = resource["path"].as_str().unwrap();
        if !path.ends_with(".md") {
            return Err(invalid("unsupported resource extension"));
        }
        let bytes = files
            .get(path)
            .ok_or_else(|| invalid(format!("missing resource: {path}")))?;
        let text = std::str::from_utf8(bytes).map_err(|_| invalid("resource must be UTF-8"))?;
        if text.contains('\r') {
            return Err(invalid("distribution Markdown must use LF newlines"));
        }
        used.insert(path.to_owned());
    }
    if used.len() != files.len() {
        return Err(invalid("unreferenced file in distribution"));
    }
    let digest = sha256(bytes);
    Ok(Distribution {
        model,
        digest,
        files,
    })
}

pub fn compile_source(source: &Path) -> Result<Distribution, Vec<Diagnostic>> {
    let loaded = load_source(source)?;
    let mut files = BTreeMap::new();
    for (name, bytes) in &loaded.files {
        if matches!(name.as_str(), "osmium.json" | "osmium.yaml") {
            continue;
        }
        let bytes = if name.ends_with(".json") {
            canonical_json(&parse_json(bytes, name).map_err(|e| vec![*e])?)
        } else {
            std::str::from_utf8(bytes)
                .map_err(|_| invalid("resource must be UTF-8"))?
                .replace("\r\n", "\n")
                .replace('\r', "\n")
                .into_bytes()
        };
        files.insert(name.clone(), bytes);
    }
    let manifest = DistributionManifest {
        distribution_version: "0.1".into(),
        canonicalization: "osmium-json-0.1".into(),
        package: loaded.model.documents().manifest.clone(),
        files: files
            .iter()
            .map(|(name, bytes)| {
                (
                    name.clone(),
                    FileRecord {
                        size: bytes.len() as u64,
                        sha256: sha256(bytes),
                    },
                )
            })
            .collect(),
    };
    if files.contains_key("manifest.json") {
        return Err(invalid(
            "manifest.json is reserved for distribution metadata",
        ));
    }
    files.insert(
        "manifest.json".into(),
        canonical_json(&serde_json::to_value(manifest).unwrap()),
    );
    verify_files(files)
}

pub fn archive_bytes(distribution: &Distribution) -> Result<Vec<u8>, Vec<Diagnostic>> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .last_modified_time(zip::DateTime::default())
        .unix_permissions(0o644);
    for (name, bytes) in &distribution.files {
        writer
            .start_file(name, options)
            .map_err(|e| invalid(e.to_string()))?;
        writer
            .write_all(bytes)
            .map_err(|e| invalid(e.to_string()))?;
    }
    Ok(writer
        .finish()
        .map_err(|e| invalid(e.to_string()))?
        .into_inner())
}

pub fn read_distribution(path: &Path) -> Result<Distribution, Vec<Diagnostic>> {
    let meta = fs::symlink_metadata(path).map_err(|e| io_error(path, e))?;
    reject_link(&meta, path)?;
    let mut files = BTreeMap::new();
    if meta.is_dir() {
        let root = path.canonicalize().map_err(|e| io_error(path, e))?;
        if fs::symlink_metadata(root.join(".git")).is_ok() {
            return Err(invalid("distribution cannot contain .git"));
        }
        let entries = inventory(&root)?;
        for name in entries.keys() {
            files.insert(name.clone(), read_checked(&root, name, &entries)?);
        }
    } else {
        if !meta.is_file() || meta.len() > MAX_ARCHIVE_BYTES {
            return Err(diagnostic(
                "archive",
                "OSM_INPUT_LIMIT",
                "archive exceeds size limit or is not regular",
            ));
        }
        let mut data = Vec::new();
        fs::File::open(path)
            .map_err(|e| io_error(path, e))?
            .take(MAX_ARCHIVE_BYTES + 1)
            .read_to_end(&mut data)
            .map_err(|e| io_error(path, e))?;
        if data.len() as u64 > MAX_ARCHIVE_BYTES {
            return Err(diagnostic(
                "archive",
                "OSM_INPUT_LIMIT",
                "archive grew beyond limit",
            ));
        }
        let declared_entries = zip_entry_count(&data)?;
        let mut archive = ZipArchive::new(Cursor::new(data)).map_err(|e| invalid(e.to_string()))?;
        if archive.len() != declared_entries {
            return Err(diagnostic(
                "archive",
                "OSM_PATH_COLLISION",
                "duplicate or inconsistent ZIP entries",
            ));
        }
        if archive.len() > MAX_SOURCE_FILES {
            return Err(diagnostic(
                "archive",
                "OSM_INPUT_LIMIT",
                "too many ZIP entries",
            ));
        }
        let mut total = 0u64;
        for index in 0..archive.len() {
            let mut entry = archive
                .by_index(index)
                .map_err(|e| invalid(e.to_string()))?;
            let name = std::str::from_utf8(entry.name_raw())
                .map_err(|_| invalid("ZIP names must be UTF-8"))?
                .to_owned();
            validate_relative_path(&name).map_err(|e| diagnostic(&name, "OSM_PATH", e))?;
            if entry.is_dir()
                || entry.is_symlink()
                || entry.encrypted()
                || entry
                    .unix_mode()
                    .is_some_and(|m| m & 0o170000 != 0 && m & 0o170000 != 0o100000)
            {
                return Err(diagnostic(
                    &name,
                    "OSM_FILE_TYPE",
                    "ZIP entries must be unencrypted regular files",
                ));
            }
            if entry.size() > MAX_DOCUMENT_BYTES as u64 {
                return Err(diagnostic(
                    &name,
                    "OSM_INPUT_LIMIT",
                    "ZIP entry exceeds limit",
                ));
            }
            let mut bytes = Vec::new();
            (&mut entry)
                .take(MAX_DOCUMENT_BYTES as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| invalid(e.to_string()))?;
            total += bytes.len() as u64;
            if bytes.len() > MAX_DOCUMENT_BYTES || total > MAX_SOURCE_BYTES {
                return Err(diagnostic(
                    &name,
                    "OSM_INPUT_LIMIT",
                    "expanded ZIP exceeds limit",
                ));
            }
            if files.insert(name.clone(), bytes).is_some() {
                return Err(diagnostic(
                    &name,
                    "OSM_PATH_COLLISION",
                    "duplicate ZIP entry",
                ));
            }
        }
    }
    verify_files(files)
}

// Inspect declared count before indexing, and detect names hidden by the ZIP
// library's name-keyed map. Small packages do not require ZIP64 or split ZIP.
fn zip_entry_count(data: &[u8]) -> Result<usize, Vec<Diagnostic>> {
    if data.len() < 22 {
        return Err(invalid("truncated ZIP"));
    }
    let start = data.len().saturating_sub(65557);
    let end = (start..=data.len() - 22)
        .rev()
        .find(|i| {
            data[*i..].starts_with(b"PK\x05\x06")
                && *i + 22 + u16::from_le_bytes([data[*i + 20], data[*i + 21]]) as usize
                    == data.len()
        })
        .ok_or_else(|| invalid("ZIP footer missing or trailing bytes present"))?;
    let word = |offset| u16::from_le_bytes([data[end + offset], data[end + offset + 1]]) as usize;
    let count = word(10);
    if word(4) != 0
        || word(6) != 0
        || word(8) != count
        || count == 65535
        || count > MAX_SOURCE_FILES
    {
        return Err(diagnostic(
            "archive",
            "OSM_INPUT_LIMIT",
            "split/ZIP64/oversized archives are unsupported",
        ));
    }
    let dword = |offset| {
        u32::from_le_bytes(data[end + offset..end + offset + 4].try_into().unwrap()) as usize
    };
    let directory_size = dword(12);
    let mut cursor = dword(16);
    if cursor.checked_add(directory_size) != Some(end) {
        return Err(invalid("ZIP directory extent is inconsistent or ZIP64"));
    }
    let mut actual = 0;
    let mut names = BTreeSet::new();
    while cursor < end {
        if end - cursor < 46 || &data[cursor..cursor + 4] != b"PK\x01\x02" {
            return Err(invalid("invalid ZIP directory entry"));
        }
        let length = |offset| {
            u16::from_le_bytes([data[cursor + offset], data[cursor + offset + 1]]) as usize
        };
        let name_len = length(28);
        let next = cursor + 46 + name_len + length(30) + length(32);
        if next > end || !names.insert(&data[cursor + 46..cursor + 46 + name_len]) {
            return Err(invalid("duplicate or truncated ZIP directory entry"));
        }
        cursor = next;
        actual += 1;
        if actual > count {
            return Err(invalid("ZIP entry count is inconsistent"));
        }
    }
    if actual != count {
        return Err(invalid("ZIP entry count is inconsistent"));
    }
    Ok(count)
}

/// Build into a new path. All content is compiled and verified before writes;
/// output parents must already exist. Existing outputs are never overwritten.
pub fn build(source: &Path, output: &Path) -> Result<BuildReport, Vec<Diagnostic>> {
    let distribution = compile_source(source)?;
    if fs::symlink_metadata(output).is_ok() {
        return Err(diagnostic(
            &output.to_string_lossy(),
            "OSM_OUTPUT_EXISTS",
            "output already exists",
        ));
    }
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    for ancestor in parent.ancestors().filter(|p| !p.as_os_str().is_empty()) {
        let meta = fs::symlink_metadata(ancestor).map_err(|e| io_error(ancestor, e))?;
        reject_link(&meta, ancestor)?;
    }
    let archive = output.extension().is_some_and(|e| e == "osmium");
    let mut archive_sha256 = None;
    if archive {
        let bytes = archive_bytes(&distribution)?;
        archive_sha256 = Some(sha256(&bytes));
        let mut temporary =
            tempfile::NamedTempFile::new_in(parent).map_err(|e| io_error(parent, e))?;
        temporary
            .write_all(&bytes)
            .map_err(|e| io_error(output, e))?;
        temporary
            .as_file()
            .sync_all()
            .map_err(|e| io_error(output, e))?;
        read_distribution(temporary.path())?;
        temporary
            .persist_noclobber(output)
            .map_err(|e| io_error(output, e.error))?;
    } else {
        let temporary = tempfile::tempdir_in(parent).map_err(|e| io_error(parent, e))?;
        for (name, bytes) in &distribution.files {
            let path = temporary.path().join(name);
            fs::create_dir_all(path.parent().unwrap()).map_err(|e| io_error(&path, e))?;
            fs::write(&path, bytes).map_err(|e| io_error(&path, e))?;
        }
        read_distribution(temporary.path())?;
        // A concurrently created destination is not silently replaced on Windows.
        // Avoid POSIX rename replacing an empty directory by checking again.
        if output.exists() {
            return Err(diagnostic(
                &output.to_string_lossy(),
                "OSM_OUTPUT_EXISTS",
                "output appeared during build",
            ));
        }
        fs::rename(temporary.path(), output).map_err(|e| io_error(output, e))?;
    }
    Ok(BuildReport {
        package_id: distribution.model.documents().manifest["package_id"]
            .as_str()
            .unwrap()
            .into(),
        package_version: distribution.model.documents().manifest["package_version"]
            .as_str()
            .unwrap()
            .into(),
        digest: distribution.digest,
        files: distribution.files.len(),
        output: output.to_string_lossy().into(),
        archive_sha256,
    })
}
