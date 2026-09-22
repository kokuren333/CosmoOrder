//! Structured failures crossing the IPC boundary.
//!
//! A command never returns an opaque string: the UI receives the same
//! diagnostic codes, paths and messages that Core and Package produce, so the
//! desktop cannot invent its own interpretation of a rejected package.

use osmium_core::schema::Diagnostic;
use serde::Serialize;

/// One failure as the frontend sees it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ErrorView {
    pub code: String,
    pub message: String,
    pub file: Option<String>,
    pub path: String,
    pub suggestions: Vec<String>,
}

/// A command failure: a non-empty list of diagnostics.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CommandError {
    pub diagnostics: Vec<ErrorView>,
}

impl CommandError {
    pub fn new(diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            diagnostics: diagnostics.into_iter().map(ErrorView::from).collect(),
        }
    }

    /// A failure raised by the desktop shell itself, for example when the IPC
    /// argument cannot be decoded. The code namespace stays distinct from the
    /// codes Core and Package own.
    pub fn shell(code: &str, message: impl ToString) -> Self {
        Self {
            diagnostics: vec![ErrorView {
                code: code.to_owned(),
                message: message.to_string(),
                file: None,
                path: String::new(),
                suggestions: Vec::new(),
            }],
        }
    }

    pub fn summary(&self) -> String {
        self.diagnostics
            .first()
            .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
            .unwrap_or_else(|| "unknown failure".to_owned())
    }
}

impl From<Vec<Diagnostic>> for CommandError {
    fn from(diagnostics: Vec<Diagnostic>) -> Self {
        Self::new(diagnostics)
    }
}

impl From<ErrorView> for Diagnostic {
    fn from(view: ErrorView) -> Self {
        Diagnostic {
            code: view.code,
            severity: "error".to_owned(),
            file: view.file,
            line: None,
            column: None,
            path: view.path,
            message: view.message,
            suggestions: view.suggestions,
        }
    }
}

impl From<Diagnostic> for ErrorView {
    fn from(diagnostic: Diagnostic) -> Self {
        Self {
            code: diagnostic.code,
            message: diagnostic.message,
            file: diagnostic.file,
            path: diagnostic.path,
            suggestions: diagnostic.suggestions,
        }
    }
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.summary())
    }
}

impl std::error::Error for CommandError {}

/// Commands return `Result<T, CommandError>`; the frontend receives the
/// serialized `CommandError` as the rejection value of `invoke`.
pub type CommandResult<T> = Result<T, CommandError>;
