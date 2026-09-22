//! Framework-independent Osmium package contracts.
//!
//! Schema validation is structural. Reference, path and graph validation must
//! also succeed before a package can be installed or used for learning.

pub mod lint;
pub mod parsing;
pub mod query;
pub mod schema;
pub mod validation;
pub mod yaml;
