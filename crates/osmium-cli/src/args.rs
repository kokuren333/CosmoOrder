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
    /// Install a verified distribution into the local library.
    Install { path: PathBuf },
    /// List installed package versions after verifying their contents.
    Packages,
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
}
