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
    pub severity: String,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub path: String,
    pub suggestions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
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
                severity: "error".to_owned(),
                message: message.to_string(),
                file: None,
                line: None,
                column: None,
                path: String::new(),
                suggestions: Vec::new(),
                entity_type: None,
                entity_id: None,
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
            severity: view.severity,
            file: view.file,
            line: view.line,
            column: view.column,
            path: view.path,
            message: view.message,
            suggestions: view.suggestions,
            entity_type: view.entity_type,
            entity_id: view.entity_id,
        }
    }
}

impl From<Diagnostic> for ErrorView {
    fn from(diagnostic: Diagnostic) -> Self {
        Self {
            code: diagnostic.code,
            severity: diagnostic.severity,
            message: diagnostic.message,
            file: diagnostic.file,
            line: diagnostic.line,
            column: diagnostic.column,
            path: diagnostic.path,
            suggestions: diagnostic.suggestions,
            entity_type: diagnostic.entity_type,
            entity_id: diagnostic.entity_id,
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

#[cfg(test)]
mod tests {
    use super::{CommandError, ErrorView};
    use osmium_core::schema::Diagnostic;

    #[test]
    fn diagnostic_severity_survives_the_desktop_ipc_projection() {
        let warning = Diagnostic {
            code: "OSM_LINT_LICENSE_UNKNOWN".into(),
            severity: "warning".into(),
            entity_type: Some("resource".into()),
            entity_id: Some("lesson".into()),
            file: Some("entities/resources.json".into()),
            line: Some(4),
            column: Some(7),
            path: "/0/license".into(),
            message: "license status is unknown".into(),
            suggestions: vec!["confirm reuse conditions".into()],
        };
        let error = CommandError::new(vec![warning]);
        let payload = serde_json::to_value(&error).expect("serializable diagnostics");
        assert_eq!(payload["diagnostics"][0]["severity"], "warning");
        assert_eq!(payload["diagnostics"][0]["path"], "/0/license");
        assert_eq!(payload["diagnostics"][0]["line"], 4);
        assert_eq!(payload["diagnostics"][0]["column"], 7);

        let view: ErrorView = error.diagnostics.into_iter().next().unwrap();
        let round_trip: Diagnostic = view.into();
        assert_eq!(round_trip.severity, "warning");
        assert_eq!(round_trip.line, Some(4));
        assert_eq!(round_trip.column, Some(7));
        assert_eq!(round_trip.entity_id.as_deref(), Some("lesson"));
    }

    #[test]
    fn shell_failures_are_errors_at_the_ipc_boundary() {
        let error = CommandError::shell("OSM_SHELL", "failed");
        assert_eq!(error.diagnostics[0].severity, "error");
    }
}
