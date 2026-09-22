//! Output envelope and exit statuses.
//!
//! The envelope shape is part of the machine contract, so it is versioned
//! separately from the package format.

use osmium_core::schema::Diagnostic;
use serde::Serialize;
use serde_json::Value;

/// Version of the `output_version` envelope, not of the package format.
pub const OUTPUT_VERSION: &str = "0.1";

#[derive(Debug, Serialize)]
pub struct Envelope {
    pub output_version: &'static str,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    /// Always present, possibly empty, so consumers need no null check.
    pub diagnostics: Vec<Diagnostic>,
}

impl Envelope {
    pub fn success(data: &impl Serialize) -> Self {
        Self {
            output_version: OUTPUT_VERSION,
            ok: true,
            data: serde_json::to_value(data).ok(),
            diagnostics: Vec::new(),
        }
    }

    pub fn failure(diagnostics: &[Diagnostic]) -> Self {
        Self {
            output_version: OUTPUT_VERSION,
            ok: false,
            data: None,
            diagnostics: diagnostics.to_vec(),
        }
    }
}

/// Process exit statuses fixed by the CLI contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    Success,
    /// The target package is invalid or unusable.
    Invalid,
    /// The invocation itself is wrong: bad arguments, missing path, existing
    /// file that `init` refuses to overwrite.
    Usage,
    /// I/O or internal failure: the tool could not complete the request.
    Internal,
    /// The package requires a schema version or capability this build lacks.
    Incompatible,
}

impl Exit {
    pub fn code(self) -> u8 {
        match self {
            Self::Success => 0,
            Self::Invalid => 1,
            Self::Usage => 2,
            Self::Internal => 3,
            Self::Incompatible => 4,
        }
    }
}
