//! Failure classification.
//!
//! A failure carries the diagnostics a user needs to find the cause and the
//! exit status the CLI contract assigns to it. Classification is by
//! diagnostic code, never by matching on message text.

use crate::envelope::Exit;
use osmium_core::schema::Diagnostic;

/// Codes that mean "this build cannot read the package at all", which is a
/// compatibility failure rather than a content mistake.
const INCOMPATIBLE_CODES: [&str; 3] = [
    "OSM_SCHEMA_VERSION",
    "OSM_CAPABILITY",
    "OSM_CAPABILITY_CONFLICT",
];
/// Codes that mean "the request could not be completed", which is an I/O or
/// internal failure rather than an invalid package.
const INTERNAL_CODES: [&str; 4] = [
    "OSM_IO",
    "OSM_SOURCE_CHANGED",
    "OSM_INIT_IO",
    "OSM_INIT_INTERNAL",
];

/// The exit status for a set of diagnostics.
///
/// Compatibility outranks content validity: a package that needs an
/// unimplemented capability is reported as incompatible even when other
/// problems exist.
pub fn exit_for(diagnostics: &[Diagnostic]) -> Exit {
    if diagnostics
        .iter()
        .any(|diagnostic| INCOMPATIBLE_CODES.contains(&diagnostic.code.as_str()))
    {
        Exit::Incompatible
    } else if diagnostics
        .iter()
        .any(|diagnostic| INTERNAL_CODES.contains(&diagnostic.code.as_str()))
    {
        Exit::Internal
    } else {
        Exit::Invalid
    }
}

pub struct Failure {
    diagnostics: Vec<Diagnostic>,
    exit: Exit,
}

impl Failure {
    /// The status of the failure itself, not of its diagnostics.
    pub fn new(diagnostics: Vec<Diagnostic>, exit: Exit) -> Self {
        Self { diagnostics, exit }
    }

    /// Classify by diagnostic code.
    pub fn from_diagnostics(diagnostics: Vec<Diagnostic>) -> Self {
        let exit = exit_for(&diagnostics);
        Self { diagnostics, exit }
    }

    /// An invocation mistake, such as a path that does not exist.
    pub fn usage(code: &str, message: impl Into<String>) -> Self {
        Self::new(vec![diagnostic(code, message)], Exit::Usage)
    }

    /// A failure this build caused, such as a scaffold that cannot be written.
    pub fn internal(code: &str, message: impl Into<String>) -> Self {
        Self::new(vec![diagnostic(code, message)], Exit::Internal)
    }

    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        self.diagnostics.clone()
    }

    pub fn exit(&self) -> Exit {
        self.exit
    }
}

/// A diagnostic for a failure that has no package location.
pub fn diagnostic(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        code: code.to_owned(),
        severity: "error".to_owned(),
        file: None,
        line: None,
        column: None,
        path: String::new(),
        message: message.into(),
        suggestions: Vec::new(),
    }
}

/// A diagnostic for a rejected command line. `clap` already printed usage, so
/// the message is kept as one line.
pub fn usage_diagnostic(message: String) -> Diagnostic {
    diagnostic("OSM_USAGE", message.trim().to_owned())
}

/// A diagnostic for a package path that is not a readable directory.
pub fn target_diagnostic(path: &std::path::Path, message: &str) -> Diagnostic {
    let mut value = diagnostic("OSM_INVOCATION", message);
    value.file = Some(path.to_string_lossy().into_owned());
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compatibility_outranks_content_validity() {
        let diagnostics = vec![
            diagnostic("OSM_SCHEMA", "broken field"),
            diagnostic("OSM_CAPABILITY", "unsupported capability"),
        ];
        assert_eq!(exit_for(&diagnostics), Exit::Incompatible);
    }

    #[test]
    fn io_failure_outranks_content_validity() {
        let diagnostics = vec![
            diagnostic("OSM_SCHEMA", "broken field"),
            diagnostic("OSM_IO", "gone"),
        ];
        assert_eq!(exit_for(&diagnostics), Exit::Internal);
    }

    #[test]
    fn ordinary_validation_is_an_invalid_package() {
        assert_eq!(
            exit_for(&[diagnostic("OSM_REFERENCE", "unknown ID")]),
            Exit::Invalid
        );
        assert_eq!(Exit::Usage.code(), 2);
        assert_eq!(Exit::Incompatible.code(), 4);
    }
}
