//! Command-line surface. Argument syntax only; no semantics live here.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "osmium",
    version,
    about = "Local-first learning runtime for portable packages",
    long_about = "Validate, inspect and query Osmium package Sources without \
                  network access. Every command writes one JSON document to \
                  stdout and human-readable lines to stderr."
)]
pub struct Cli {
    /// Local package and learning-state directory (otherwise OSMIUM_HOME or OS default).
    #[arg(long, global = true)]
    pub home: Option<PathBuf>,
    /// Machine-readable output. JSON is the only supported format in this
    /// phase; the flag exists so scripts can state the contract explicitly.
    #[arg(long, value_name = "FORMAT", default_value = "json")]
    pub output: OutputFormat,

    /// Explicit JSON output (also the default).
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    Json,
}

#[derive(Debug, clap::Args)]
pub struct LegacyOutput {
    /// JSON output; --json is the equivalent global flag.
    #[arg(long = "output", value_enum)]
    pub legacy_output: Option<OutputFormat>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Display installed curriculum, concepts, objectives, resources and assessments.
    Learn {
        package_id: String,
        #[arg(long)]
        package_version: Option<String>,
    },
    /// Read an installed Markdown resource by its stable ID.
    Read {
        package_id: String,
        resource_id: String,
        #[arg(long)]
        package_version: Option<String>,
    },
    /// Evaluate an answer and atomically append a learning event.
    Answer {
        package_id: String,
        assessment_id: String,
        #[arg(long)]
        response: String,
        #[arg(long)]
        package_version: Option<String>,
        #[arg(long)]
        request_id: Option<String>,
        #[arg(long)]
        duration_ms: Option<u64>,
        #[arg(long)]
        hints_used: Option<u64>,
    },
    /// Show observed progress derived from learning events.
    Progress {
        package_id: String,
        #[arg(long)]
        package_version: Option<String>,
    },
    /// List recorded attempts, newest first.
    History {
        package_id: String,
        #[arg(long)]
        package_version: Option<String>,
        #[arg(long, default_value_t = 16)]
        limit: usize,
        #[arg(long, default_value_t = 0)]
        offset: usize,
    },
    /// Recalculate all progress from the append-only event log.
    RebuildProgress,
    /// Export the complete event log as versioned JSONL without overwriting.
    ExportState {
        #[arg(long)]
        output: PathBuf,
    },
    /// Make a consistent SQLite backup without overwriting.
    BackupState {
        #[arg(long)]
        output: PathBuf,
    },
    /// Install a verified distribution into the local library.
    Install { path: PathBuf },
    /// List installed package versions after verifying their contents.
    Packages,
    /// Remove one installed package version while preserving learning history.
    Uninstall {
        package_id: String,
        #[arg(long = "package-version")]
        package_version: String,
    },
    /// Build a verified distribution directory or .osmium ZIP without overwriting.
    Build {
        source: PathBuf,
        #[arg(long = "output")]
        destination: PathBuf,
    },
    /// Create a minimal, already-valid package Source. Existing files are
    /// never overwritten.
    Init {
        #[command(flatten)]
        format: LegacyOutput,
        /// Target directory. It is created when absent.
        #[arg(default_value = ".")]
        directory: PathBuf,
        /// Package ID in `org.example/name` form. Derived from the directory
        /// name when omitted.
        #[arg(long, value_name = "ID")]
        package_id: Option<String>,
        /// BCP 47 language tag for the new package.
        #[arg(long, value_name = "TAG")]
        language: Option<String>,
    },
    /// Validate a Source: structure, semantics, safety and file existence.
    /// Nothing is written.
    Validate {
        #[command(flatten)]
        format: LegacyOutput,
        /// Package Source directory containing `osmium.json` or `osmium.yaml`.
        path: PathBuf,
    },
    /// Report structural coverage and metadata omissions, without quality scores.
    Lint {
        path: PathBuf,
        #[command(flatten)]
        format: LegacyOutput,
    },
    /// Report package metadata and entity counts.
    Inspect {
        #[command(flatten)]
        format: LegacyOutput,
        /// Package Source directory.
        path: PathBuf,
        /// Maximum number of prerequisite-ordered Concept IDs to report.
        #[arg(long, value_name = "N", default_value_t = 64)]
        limit: usize,
    },
    /// Page through one entity kind.
    Query {
        #[command(flatten)]
        format: LegacyOutput,
        /// Package Source directory.
        path: PathBuf,
        /// Entity kind: concept, objective, curriculum, resource, assessment.
        kind: String,
        /// Maximum number of entities to return.
        #[arg(long, value_name = "N", default_value_t = 32)]
        limit: usize,
        /// Number of entities to skip in document order.
        #[arg(long, value_name = "N", default_value_t = 0)]
        offset: usize,
    },
    /// Return the bounded neighborhood of one entity: its objectives,
    /// prerequisites, resources and assessments.
    Context {
        #[command(flatten)]
        format: LegacyOutput,
        /// Package Source directory.
        path: PathBuf,
        /// Stable entity ID.
        entity_id: String,
        /// Entity kind, required only when the ID is ambiguous.
        #[arg(long, value_name = "KIND")]
        kind: Option<String>,
        /// Maximum number of relation hops to follow.
        #[arg(long, value_name = "N", default_value_t = 2)]
        depth: usize,
        /// Maximum number of entities to return.
        #[arg(long, value_name = "N", default_value_t = 64)]
        limit: usize,
    },
    /// Register, attach and list learner-facing Reference records.
    Reference {
        // Boxed because the `add` option set is much larger than every other
        // variant and `Command` is parsed exactly once per process.
        #[command(subcommand)]
        action: Box<ReferenceAction>,
    },
}

/// Reference authoring is the only part of the CLI that writes to a Source.
///
/// `Add` carries many optional bibliographic fields, so it is much larger than
/// the other variants. The enum is built once per process from the command line
/// and never stored in a collection, so the layout difference costs nothing.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Subcommand)]
pub enum ReferenceAction {
    /// Register one Reference in the manifest registry. Existing files are
    /// never overwritten and an already-registered ID is a no-op.
    Add {
        #[command(flatten)]
        format: LegacyOutput,
        /// Package Source directory.
        path: PathBuf,
        /// Stable Reference ID.
        #[arg(long, value_name = "ID")]
        id: String,
        /// Record kind: url, doi, isbn, citation, local_file, package_asset or manual.
        #[arg(long, value_name = "KIND")]
        kind: String,
        #[arg(long, value_name = "TEXT")]
        title: Option<String>,
        /// URL or package-relative path reaching the Reference.
        #[arg(long, value_name = "LOCATOR")]
        locator: Option<String>,
        /// Human-readable citation for a learner bibliography.
        #[arg(long, value_name = "TEXT")]
        citation: Option<String>,
        /// Material type: webpage, article, book, guideline, dataset, document, other.
        #[arg(long = "type", value_name = "TYPE")]
        reference_type: Option<String>,
        #[arg(long, value_name = "NAME")]
        publisher: Option<String>,
        /// Repeatable author name, in the order it should be displayed.
        #[arg(long = "author", value_name = "NAME")]
        authors: Vec<String>,
        #[arg(long = "published-at", value_name = "DATE")]
        published_at: Option<String>,
        #[arg(long = "updated-at", value_name = "DATE")]
        updated_at: Option<String>,
        #[arg(long = "accessed-at", value_name = "DATE")]
        accessed_at: Option<String>,
        #[arg(long, value_name = "TEXT")]
        edition: Option<String>,
        #[arg(long, value_name = "TEXT")]
        version: Option<String>,
        /// public, attribution_only or private.
        #[arg(long, value_name = "VISIBILITY", default_value = "public")]
        visibility: String,
    },
    /// Attach an existing Reference to a Resource or an Assessment as Evidence.
    Attach {
        #[command(flatten)]
        format: LegacyOutput,
        /// Package Source directory.
        path: PathBuf,
        /// Stable Reference ID to attach.
        reference_id: String,
        /// Target Resource ID.
        #[arg(long, value_name = "ID", conflicts_with = "assessment")]
        resource: Option<String>,
        /// Target Assessment ID.
        #[arg(long, value_name = "ID")]
        assessment: Option<String>,
    },
    /// List registered References without writing anything.
    List {
        #[command(flatten)]
        format: LegacyOutput,
        /// Package Source directory.
        path: PathBuf,
    },
}
