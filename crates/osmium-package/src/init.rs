//! Non-destructive authoring scaffold for a new package Source.
//!
//! `init` is the only operation in this crate that writes into a user-chosen
//! directory. It never overwrites an existing file, and it validates the
//! scaffold it produced with the same loader that reads third-party packages,
//! so a generated Source cannot be structurally invalid.

use crate::load_source;
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub const SCAFFOLD_PACKAGE_VERSION: &str = "0.1.0";
pub const SCAFFOLD_LANGUAGE: &str = "en";
/// Fallback package ID used when the target directory name has no usable
/// ASCII characters.
pub const FALLBACK_PACKAGE_ID: &str = "org.example/scaffold";
pub const FALLBACK_TITLE: &str = "Osmium package";
/// Longest string field accepted by the format 0.1 schema.
const MAX_STRING_CHARS: usize = 16_384;
/// Every path `init` may create, relative to the target directory.
pub const SCAFFOLD_FILES: [&str; 7] = [
    "osmium.json",
    "entities/concepts.json",
    "entities/objectives.json",
    "entities/curricula.json",
    "entities/resources.json",
    "entities/assessments.json",
    "content/lesson.md",
];

/// A structured `init` failure. `code` is stable so a caller can map it to an
/// exit status and to a diagnostic without matching on message text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitError {
    pub code: &'static str,
    pub message: String,
    pub path: PathBuf,
}

impl std::fmt::Display for InitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for InitError {}

/// A checked `init` request.
#[derive(Debug, Clone)]
pub struct InitRequest {
    pub directory: PathBuf,
    pub package_id: String,
    pub language: String,
    /// Optional learner-facing package title. `None` keeps the CLI's existing
    /// directory-derived default.
    pub title: Option<String>,
}

/// Validate a package ID against the manifest schema pattern and the same
/// length bound. A slash cannot escape the target directory because each side
/// is restricted to `[a-z0-9-]`.
pub(crate) fn validate_package_id(id: &str) -> Result<(), InitError> {
    let failure = |message: &str| InitError {
        code: "OSM_INIT_ID",
        message: message.to_owned(),
        path: PathBuf::from("package_id"),
    };
    if id.is_empty() || id.len() > 200 {
        return Err(failure("package ID must contain 1 to 200 characters"));
    }
    let (namespace, name) = id
        .split_once('/')
        .ok_or_else(|| failure("package ID must use the form org.example/name"))?;
    if name.contains('/') {
        return Err(failure("package ID must contain exactly one slash"));
    }
    let segment_ok = |segment: &str| {
        !segment.is_empty()
            && segment.split('.').all(|part| {
                let mut characters = part.chars();
                characters
                    .next()
                    .is_some_and(|first| first.is_ascii_lowercase())
                    && characters.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            })
    };
    if !namespace.contains('.') || name.contains('.') || !segment_ok(namespace) || !segment_ok(name)
    {
        return Err(failure(
            "package ID segments must be lowercase ASCII words separated by . or -",
        ));
    }
    Ok(())
}

const GENERATED_PACKAGE_NAMESPACE: &str = "org.osmium.generated";

/// Create a user-facing course beneath a selected course-library directory.
/// The stable Package ID is generated here, in the package authoring layer,
/// rather than by a frontend. Existing CLI `init` keeps its directory-derived
/// ID behavior through `init_source`.
pub fn init_source_with_generated_id(
    parent_directory: impl AsRef<Path>,
    title: &str,
    language: &str,
) -> Result<PathBuf, InitError> {
    let parent = parent_directory.as_ref();
    let title = title.trim();
    if title.is_empty() {
        return Err(failure(
            "OSM_INIT_TITLE",
            "title must not be empty".to_owned(),
            PathBuf::from("title"),
        ));
    }

    let mut slug = String::new();
    let mut separator = false;
    for character in title.to_lowercase().chars() {
        if character.is_ascii_lowercase() || character.is_ascii_digit() {
            slug.push(character);
            separator = false;
        } else if !separator && !slug.is_empty() {
            slug.push('-');
            separator = true;
        }
        if slug.len() >= 72 {
            break;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() || !slug.starts_with(|c: char| c.is_ascii_lowercase()) {
        slug = if slug.is_empty() {
            "course".to_owned()
        } else {
            format!("course-{slug}")
        };
    }

    // A friendly source folder name is helpful, but the suffix makes its path
    // independent of title uniqueness and keeps repeated titles non-destructive.
    let directory = (1u32..=10_000)
        .find_map(|index| {
            let name = if index == 1 {
                slug.clone()
            } else {
                format!("{slug}-{index}")
            };
            let candidate = parent.join(name);
            match fs::symlink_metadata(&candidate) {
                Ok(_) => None,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Some(Ok(candidate)),
                Err(error) => Some(Err(failure("OSM_INIT_IO", error.to_string(), candidate))),
            }
        })
        .ok_or_else(|| {
            failure(
                "OSM_INIT_EXISTS",
                "could not find an unused source folder name".to_owned(),
                parent.to_path_buf(),
            )
        })??;

    let package_id = format!(
        "{GENERATED_PACKAGE_NAMESPACE}/{slug}-{}",
        Uuid::new_v4().simple()
    );
    let request = InitRequest {
        directory: directory.clone(),
        package_id,
        language: language.to_owned(),
        title: Some(title.to_owned()),
    };
    init_source(&request)?;
    Ok(directory)
}

/// Entity IDs generated by the scaffold, checked against the schema grammar so
/// an internal mistake is caught before any file is written.
fn valid_entity_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        && id.chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_' || c == '-'
        })
}

fn clamp(value: String) -> String {
    let mut clamped: String = value.chars().take(MAX_STRING_CHARS).collect();
    while clamped.ends_with(' ') || clamped.ends_with('\t') {
        clamped.pop();
    }
    clamped
}

/// Derive a valid package ID and title from the target directory name.
fn derive_identity(directory: &Path) -> (String, String) {
    let raw = directory
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let title = if raw.trim().is_empty() {
        FALLBACK_TITLE.to_owned()
    } else {
        raw.to_owned()
    };
    let lowered = raw.to_lowercase();
    let mut name = String::with_capacity(lowered.len());
    let mut previous_separator = false;
    for character in lowered.chars() {
        if character.is_ascii_lowercase() || character.is_ascii_digit() {
            name.push(character);
            previous_separator = false;
        } else if !previous_separator && !name.is_empty() {
            name.push('-');
            previous_separator = true;
        }
    }
    while name.ends_with('-') {
        name.pop();
    }
    if name.is_empty() || !name.starts_with(|c: char| c.is_ascii_lowercase()) {
        return (FALLBACK_PACKAGE_ID.to_owned(), clamp(title));
    }
    let candidate = format!("org.example/{name}");
    if validate_package_id(&candidate).is_ok() {
        (candidate, clamp(title))
    } else {
        (FALLBACK_PACKAGE_ID.to_owned(), clamp(title))
    }
}

fn json(value: &serde_json::Value) -> String {
    let mut text = serde_json::to_string_pretty(value).expect("scaffold values are serializable");
    text.push('\n');
    text
}

/// The complete scaffold as relative portable path → exact bytes.
///
/// Paths use `/` for the package format and are converted to the platform
/// separator only when a filesystem call is made.
pub fn scaffold_files(request: &InitRequest) -> BTreeMap<String, Vec<u8>> {
    let (derived_id, derived_title) = derive_identity(&request.directory);
    let title = request.title.as_deref().unwrap_or(&derived_title);
    let package_id = if request.package_id.is_empty() {
        derived_id
    } else {
        request.package_id.clone()
    };
    let concept = "example.concept";
    let objective = "example.objective";
    let resource = "example.lesson";
    let assessment = "example.assessment";
    let curriculum = "example.curriculum";
    for id in [concept, objective, resource, assessment, curriculum] {
        debug_assert!(valid_entity_id(id), "scaffold entity ID is invalid: {id}");
    }
    let manifest = serde_json::json!({
        "schema_version": osmium_core::schema::SCHEMA_VERSION,
        "package_id": package_id,
        "package_version": SCAFFOLD_PACKAGE_VERSION,
        "title": title,
        "language": request.language,
        "capabilities": { "required": [], "optional": [] },
        "entities": {
            "concepts": "entities/concepts.json",
            "objectives": "entities/objectives.json",
            "curricula": "entities/curricula.json",
            "resources": "entities/resources.json",
            "assessments": "entities/assessments.json"
        },
        "extensions": {}
    });
    let concepts = serde_json::json!([
        {
            "id": concept,
            "title": "Example concept",
            "requires": []
        }
    ]);
    let objectives = serde_json::json!([
        {
            "id": objective,
            "concept": concept,
            "description": "Describe what a learner can do after this objective."
        }
    ]);
    let curricula = serde_json::json!([
        {
            "id": curriculum,
            "title": "Example curriculum",
            "objectives": [objective]
        }
    ]);
    let resources = serde_json::json!([
        {
            "id": resource,
            "type": "markdown",
            "title": "Example lesson",
            "path": "content/lesson.md",
            "teaches": [objective]
        }
    ]);
    let assessments = serde_json::json!([
        {
            "id": assessment,
            "revision": "1",
            "measures": [objective],
            "stimulus": { "markdown": "Replace this stimulus with a real question." },
            "response": {
                "type": "single_select",
                "options": [
                    { "id": "a", "text": "First option" },
                    { "id": "b", "text": "Second option" }
                ]
            },
            "evaluation": { "type": "exact", "answer": "a" },
            "feedback": { "markdown": "Replace this feedback with an explanation." }
        }
    ]);
    let mut files = BTreeMap::new();
    files.insert("osmium.json".to_owned(), json(&manifest).into_bytes());
    files.insert(
        "entities/concepts.json".to_owned(),
        json(&concepts).into_bytes(),
    );
    files.insert(
        "entities/objectives.json".to_owned(),
        json(&objectives).into_bytes(),
    );
    files.insert(
        "entities/curricula.json".to_owned(),
        json(&curricula).into_bytes(),
    );
    files.insert(
        "entities/resources.json".to_owned(),
        json(&resources).into_bytes(),
    );
    files.insert(
        "entities/assessments.json".to_owned(),
        json(&assessments).into_bytes(),
    );
    files.insert(
        "content/lesson.md".to_owned(),
        b"# Example lesson\n\nReplace this Markdown with the lesson body.\n\nRaw HTML and scripts are not executed by Osmium.\n"
            .to_vec(),
    );
    files
}

fn failure(code: &'static str, message: String, path: PathBuf) -> InitError {
    InitError {
        code,
        message,
        path,
    }
}

fn check_directory_ancestors(path: &Path) -> Result<(), InitError> {
    for ancestor in path.ancestors().filter(|p| !p.as_os_str().is_empty()) {
        match fs::symlink_metadata(ancestor) {
            Ok(meta) => {
                if !meta.is_dir() || super::reject_link(&meta, ancestor).is_err() {
                    return Err(failure(
                        "OSM_INIT_TARGET",
                        "target ancestors must be regular directories without links".into(),
                        ancestor.into(),
                    ));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(failure("OSM_INIT_IO", e.to_string(), ancestor.into())),
        }
    }
    Ok(())
}

/// Create a minimal, already-valid package Source.
///
/// An empty `package_id` asks for one derived from the target directory name,
/// so that rule lives only here. Every target file must be absent first:
/// existing content is never overwritten, and files created before a write
/// failure are removed.
pub fn init_source(request: &InitRequest) -> Result<Vec<String>, InitError> {
    let package_id = if request.package_id.is_empty() {
        derive_identity(&request.directory).0
    } else {
        request.package_id.clone()
    };
    validate_package_id(&package_id)?;
    if request.language.trim().is_empty() {
        return Err(failure(
            "OSM_INIT_LANGUAGE",
            "language must not be empty".to_owned(),
            PathBuf::from("language"),
        ));
    }
    if language_tags::LanguageTag::parse(&request.language).is_err() {
        return Err(failure(
            "OSM_INIT_LANGUAGE",
            format!(
                "language must be a syntactically well-formed BCP 47 tag: {}",
                request.language
            ),
            PathBuf::from("language"),
        ));
    }
    let files = scaffold_files(request);
    if files.len() != SCAFFOLD_FILES.len() {
        return Err(failure(
            "OSM_INIT_INTERNAL",
            "the scaffold file list is inconsistent".to_owned(),
            request.directory.clone(),
        ));
    }
    for name in files.keys() {
        if !SCAFFOLD_FILES.contains(&name.as_str()) {
            return Err(failure(
                "OSM_INIT_INTERNAL",
                format!("unexpected scaffold path: {name}"),
                PathBuf::from(name),
            ));
        }
    }
    let directory = &request.directory;
    check_directory_ancestors(directory)?;
    if fs::symlink_metadata(directory.join("osmium.yaml")).is_ok() {
        return Err(failure(
            "OSM_INIT_EXISTS",
            "a YAML manifest already exists".into(),
            directory.join("osmium.yaml"),
        ));
    }
    if let Ok(meta) = fs::symlink_metadata(directory) {
        if meta.file_type().is_symlink() || !meta.is_dir() {
            return Err(failure(
                "OSM_INIT_TARGET",
                "target exists and is not a regular directory".to_owned(),
                directory.clone(),
            ));
        }
    }
    // Check every path before writing anything, so a refused `init` leaves the
    // target exactly as it was.
    let mut paths = Vec::new();
    for name in files.keys() {
        let path = directory.join(name.replace('/', std::path::MAIN_SEPARATOR_STR));
        check_directory_ancestors(path.parent().unwrap())?;
        if fs::symlink_metadata(&path).is_ok() {
            return Err(failure(
                "OSM_INIT_EXISTS",
                format!("refusing to overwrite existing file: {name}"),
                path,
            ));
        }
        paths.push(path);
    }
    let mut created: Vec<PathBuf> = Vec::new();
    let result = (|| -> std::io::Result<()> {
        for (name, path) in files.keys().zip(&paths) {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)?;
            created.push(path.clone());
            file.write_all(&files[name])?;
        }
        Ok(())
    })();
    if let Err(error) = result {
        for path in &created {
            let _ = fs::remove_file(path);
        }
        return Err(failure(
            "OSM_INIT_IO",
            format!("could not write the scaffold: {error}"),
            directory.clone(),
        ));
    }
    // The scaffold must satisfy the same loader that reads third-party
    // packages. A failure here is an internal defect, not a user error.
    load_source(directory).map_err(|diagnostics| {
        for path in &created {
            let _ = fs::remove_file(path);
        }
        failure(
            "OSM_INIT_INVALID",
            format!(
                "the generated scaffold did not validate: {}",
                diagnostics
                    .first()
                    .map(|diagnostic| diagnostic.message.clone())
                    .unwrap_or_else(|| "no diagnostics".to_owned())
            ),
            directory.clone(),
        )
    })?;
    Ok(SCAFFOLD_FILES
        .iter()
        .map(|name| (*name).to_owned())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_id_grammar_matches_schema_and_yaml_is_not_overwritten() {
        for id in ["org/name", "org.example/name.with.dots"] {
            assert!(validate_package_id(id).is_err());
        }
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("osmium.yaml"), "keep: true").unwrap();
        assert_eq!(
            init_source(&request(dir.path())).unwrap_err().code,
            "OSM_INIT_EXISTS"
        );
        assert!(!dir.path().join("osmium.json").exists());
    }

    #[test]
    fn init_rejects_linked_parent_without_writing_outside() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let link = dir.path().join("entities");
        #[cfg(windows)]
        assert!(
            std::process::Command::new("cmd")
                .args(["/c", "mklink", "/J"])
                .arg(&link)
                .arg(outside.path())
                .output()
                .unwrap()
                .status
                .success()
        );
        #[cfg(unix)]
        std::os::unix::fs::symlink(outside.path(), &link).unwrap();
        assert_eq!(
            init_source(&request(dir.path())).unwrap_err().code,
            "OSM_INIT_TARGET"
        );
        assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
        assert!(!dir.path().join("osmium.json").exists());
        #[cfg(windows)]
        fs::remove_dir(link).unwrap();
        #[cfg(unix)]
        fs::remove_file(link).unwrap();
    }

    fn request(directory: &Path) -> InitRequest {
        InitRequest {
            directory: directory.to_path_buf(),
            package_id: "org.example/scaffold".to_owned(),
            language: "en".to_owned(),
            title: None,
        }
    }

    #[test]
    fn scaffold_is_valid_and_uses_only_declared_files() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("arithmetic-basics");
        let created = init_source(&request(&target)).unwrap();
        assert_eq!(created, SCAFFOLD_FILES);
        let loaded = load_source(&target).unwrap();
        assert_eq!(
            loaded.model.documents().manifest["package_id"],
            "org.example/scaffold"
        );
        assert_eq!(
            loaded.model.documents().manifest["title"],
            "arithmetic-basics"
        );
        assert_eq!(loaded.model.prerequisite_order(), ["example.concept"]);
    }

    #[test]
    fn init_refuses_to_overwrite_any_existing_file() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("lesson");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("osmium.json"), "{}").unwrap();
        let error = init_source(&request(&target)).unwrap_err();
        assert_eq!(error.code, "OSM_INIT_EXISTS");
        assert_eq!(
            fs::read_to_string(target.join("osmium.json")).unwrap(),
            "{}"
        );
        assert!(!target.join("entities").exists());
    }

    #[test]
    fn init_rejects_a_non_directory_target() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("file");
        fs::write(&target, "not a directory").unwrap();
        assert_eq!(
            init_source(&request(&target)).unwrap_err().code,
            "OSM_INIT_TARGET"
        );
    }

    #[test]
    fn init_rejects_invalid_package_ids_without_creating_files() {
        for id in [
            "scaffold",
            "org.example/",
            "/name",
            "org.example/Name",
            "org.example/name/extra",
            "org.example/.name",
            "org.example/na me",
        ] {
            assert!(validate_package_id(id).is_err(), "{id} should be rejected");
        }
        for id in [
            "org.example/scaffold",
            "a.b.c/name",
            "org.example/my-lesson",
        ] {
            assert!(validate_package_id(id).is_ok(), "{id} should be accepted");
        }
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("lesson");
        let mut bad = request(&target);
        bad.package_id = "org.example/Name".to_owned();
        assert_eq!(init_source(&bad).unwrap_err().code, "OSM_INIT_ID");
        assert!(!target.exists());
    }

    #[test]
    fn an_empty_package_id_is_derived_from_the_target_directory() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("My Lesson");
        let mut derived = request(&target);
        derived.package_id = String::new();
        init_source(&derived).unwrap();
        let loaded = load_source(&target).unwrap();
        assert_eq!(
            loaded.model.documents().manifest["package_id"],
            "org.example/my-lesson"
        );
    }

    #[test]
    fn generated_authoring_ids_are_valid_unique_and_paths_are_non_destructive() {
        let directory = tempfile::tempdir().unwrap();
        let first =
            init_source_with_generated_id(directory.path(), "Created Course", "en").unwrap();
        let second =
            init_source_with_generated_id(directory.path(), "Created Course", "en").unwrap();
        let first_package = load_source(&first).unwrap();
        let second_package = load_source(&second).unwrap();
        let first_id = first_package.model.documents().manifest["package_id"]
            .as_str()
            .unwrap();
        let second_id = second_package.model.documents().manifest["package_id"]
            .as_str()
            .unwrap();

        assert!(validate_package_id(first_id).is_ok());
        assert!(validate_package_id(second_id).is_ok());
        assert_ne!(first_id, second_id);
        assert_eq!(first.file_name().unwrap(), "created-course");
        assert_eq!(second.file_name().unwrap(), "created-course-2");
        assert_eq!(
            first_package.model.documents().manifest["title"],
            "Created Course"
        );
        assert_eq!(
            second_package.model.documents().manifest["title"],
            "Created Course"
        );
    }

    #[test]
    fn init_rejects_a_malformed_language_before_writing() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("lesson");
        let mut bad = request(&target);
        bad.language = "not a tag".to_owned();
        assert_eq!(init_source(&bad).unwrap_err().code, "OSM_INIT_LANGUAGE");
        assert!(!target.exists());
    }

    #[test]
    fn derived_identity_stays_inside_the_target_directory() {
        let (id, title) = derive_identity(Path::new("D:/work/Basics 101"));
        assert_eq!(id, "org.example/basics-101");
        assert_eq!(title, "Basics 101");
        let (id, title) = derive_identity(Path::new("D:/work/日本語"));
        assert_eq!(id, FALLBACK_PACKAGE_ID);
        assert_eq!(title, "日本語");
    }

    #[test]
    fn valid_entity_ids_match_the_schema_grammar() {
        assert!(valid_entity_id("example.concept"));
        assert!(valid_entity_id("addition_01"));
        assert!(!valid_entity_id("Example"));
        assert!(!valid_entity_id(".hidden"));
    }
}
