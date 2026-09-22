//! `osmium` command-line adapter.
//!
//! The CLI owns argument parsing, the output envelope and exit statuses. It
//! contains no validation logic: every semantic decision comes from
//! `osmium-core` and every filesystem decision comes from `osmium-package`.
//!
//! Machine contract for this phase:
//!
//! - stdout carries exactly one JSON envelope document (or clap's own help and
//!   version text).
//! - stderr carries a human-readable line per diagnostic.
//! - exit status is 0 success, 1 invalid package, 2 invalid invocation,
//!   3 I/O or internal failure, 4 schema or capability incompatibility.

mod args;
mod commands;
mod envelope;
mod error;

use args::Cli;
use clap::Parser;
use envelope::Envelope;
use std::ffi::OsString;
use std::io::Write;

pub use envelope::{Exit, OUTPUT_VERSION};

/// Argument vector used by [`run`] and by the integration tests.
pub type Args = Vec<OsString>;

/// Run one command against the process streams.
pub fn run(arguments: Args) -> Exit {
    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();
    run_from(arguments, &mut stdout, &mut stderr)
}

/// Parse arguments, run one command and write its result.
///
/// `stderr` receives human-readable lines only; no envelope field is written
/// there, so a caller can pipe stdout without losing the machine contract.
pub fn run_from<O, E>(arguments: Args, stdout: &mut O, stderr: &mut E) -> Exit
where
    O: Write,
    E: Write,
{
    let cli = match Cli::try_parse_from(arguments) {
        Ok(cli) => cli,
        Err(error) => {
            // Help and version are requested output, not a failed invocation.
            let requested = matches!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            );
            // Render through the caller's writer rather than the process
            // stream, so the whole contract stays observable and testable.
            let _ = write!(stderr, "{error}");
            if requested {
                return Exit::Success;
            }
            let diagnostic = error::usage_diagnostic(error.to_string());
            let envelope = Envelope::failure(std::slice::from_ref(&diagnostic));
            let _ = write_envelope(stdout, &envelope);
            let _ = writeln!(stderr, "error: {}", diagnostic.message);
            return Exit::Usage;
        }
    };
    match commands::execute(&cli) {
        Ok((data, diagnostics)) => {
            let mut envelope = Envelope::success(&data);
            envelope.diagnostics = diagnostics;
            for diagnostic in &envelope.diagnostics {
                let _ = writeln!(
                    stderr,
                    "{}: {}: {}",
                    diagnostic.severity, diagnostic.code, diagnostic.message
                );
            }
            if write_envelope(stdout, &envelope).is_err() {
                return Exit::Internal;
            }
            Exit::Success
        }
        Err(failure) => {
            let diagnostics = failure.diagnostics();
            let envelope = Envelope::failure(&diagnostics);
            if write_envelope(stdout, &envelope).is_err() {
                return Exit::Internal;
            }
            for diagnostic in &diagnostics {
                let _ = writeln!(
                    stderr,
                    "{}: {}: {}",
                    diagnostic.severity, diagnostic.code, diagnostic.message
                );
            }
            failure.exit()
        }
    }
}

fn write_envelope<O: Write>(stdout: &mut O, envelope: &Envelope) -> std::io::Result<()> {
    serde_json::to_writer(&mut *stdout, envelope).map_err(std::io::Error::other)?;
    stdout.write_all(b"\n")?;
    stdout.flush()
}
