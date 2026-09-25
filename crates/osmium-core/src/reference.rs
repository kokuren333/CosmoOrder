//! Reference and Evidence vocabulary shared by validation, lint, distribution
//! and the runtime projector.
//!
//! Format 0.1 grew two names for the same relation. The canonical name is now
//! *Reference*: `manifest.references[]` holds learner-facing knowledge
//! resources, and `evidence_reference_ids[]` on a Resource or Assessment is the
//! Evidence relation to them. The earlier `sources` / `source_ids` spelling is
//! still read so that existing packages keep working, but nothing writes it.
//!
//! Visibility used to be one closed enum that mixed two independent questions.
//! It is modelled here as two axes:
//!
//! | legacy `visibility` | `record_visibility` | `locator_visibility` |
//! | --- | --- | --- |
//! | `public`            | `public`            | `public`             |
//! | `attribution_only`  | `public`            | `hidden`             |
//! | `private`           | `private`           | *absent*             |
//!
//! Private records are Authoring Provenance that leaked into a Package and are
//! removed from every distribution; the supported home for authoring input is
//! the `.osmium/` Authoring Workspace, which is never part of a Package.

use serde_json::Value;

/// Canonical manifest field holding the Reference registry.
pub const REFERENCES: &str = "references";
/// Legacy manifest field holding the same registry.
pub const LEGACY_REFERENCES: &str = "sources";
/// Canonical Evidence field on a Resource or Assessment.
pub const EVIDENCE: &str = "evidence_reference_ids";
/// Legacy Evidence field on a Resource or Assessment.
pub const LEGACY_EVIDENCE: &str = "source_ids";

/// Name of the registry field this manifest actually uses, if it has one.
pub fn registry_field(manifest: &Value) -> Option<&'static str> {
    if manifest.get(REFERENCES).is_some() {
        Some(REFERENCES)
    } else if manifest.get(LEGACY_REFERENCES).is_some() {
        Some(LEGACY_REFERENCES)
    } else {
        None
    }
}

/// The Reference registry of a manifest, whichever name it uses.
pub fn registry(manifest: &Value) -> Option<&Vec<Value>> {
    registry_field(manifest).and_then(|field| manifest[field].as_array())
}

/// Name of the Evidence field this entity actually uses.
///
/// The canonical name wins when both are present so that a half-migrated file
/// cannot silently fall back to stale IDs.
pub fn evidence_field(entity: &Value) -> &'static str {
    if entity.get(EVIDENCE).is_some() {
        EVIDENCE
    } else {
        LEGACY_EVIDENCE
    }
}

/// Name of the Evidence field a writer should use for this entity.
///
/// Reading prefers whichever name the entity already chose; writing prefers the
/// canonical name and only stays on the legacy name for an entity that already
/// uses it, so a migration is never silently reverted.
pub fn evidence_field_for_write(entity: &Value) -> &'static str {
    if entity.get(LEGACY_EVIDENCE).is_some() && entity.get(EVIDENCE).is_none() {
        LEGACY_EVIDENCE
    } else {
        EVIDENCE
    }
}

/// Declared Evidence Reference IDs, ignoring a `null` or wrong-typed field.
pub fn evidence_ids(entity: &Value) -> impl Iterator<Item = &str> {
    entity[evidence_field(entity)]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
}

/// Whether the entity declares any Evidence relation at all.
pub fn has_evidence(entity: &Value) -> bool {
    entity
        .get(EVIDENCE)
        .or_else(|| entity.get(LEGACY_EVIDENCE))
        .is_some_and(|value| !value.as_array().is_some_and(Vec::is_empty))
}

/// Two-axis visibility of one Reference record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// Distributable record with a distributable locator.
    Public,
    /// Distributable record whose locator stays authoring-only.
    AttributionOnly,
    /// Authoring Provenance; the record itself is removed from a distribution.
    Private,
}

impl Visibility {
    /// Whether the record may appear in a learner-facing distribution.
    pub fn is_record_public(self) -> bool {
        !matches!(self, Visibility::Private)
    }

    /// Whether the locator may appear in a learner-facing distribution.
    pub fn is_locator_public(self) -> bool {
        matches!(self, Visibility::Public)
    }
}

/// Resolve the two axes conservatively when legacy and split fields disagree.
/// Any private record declaration wins; otherwise any hidden-locator declaration
/// wins. This prevents a contradictory legacy field from weakening privacy.
pub fn visibility(reference: &Value) -> Visibility {
    if reference["record_visibility"] == "private" || reference["visibility"] == "private" {
        return Visibility::Private;
    }
    if reference["locator_visibility"] == "hidden" || reference["visibility"] == "attribution_only"
    {
        return Visibility::AttributionOnly;
    }
    Visibility::Public
}

/// A license is meaningful only when the author states a known reuse status and
/// names an actual license or reuse policy. Placeholder prose that merely fills
/// the field is not metadata, and neither is `license_status: unknown`.
pub fn license_is_known(resource: &Value) -> bool {
    if resource.get("license_status").and_then(Value::as_str) != Some("known") {
        return false;
    }
    let text = resource
        .get("license")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    let normalized = text.to_ascii_lowercase();
    let normalized = normalized.split_whitespace().collect::<Vec<_>>().join(" ");
    !normalized.is_empty()
        && !normalized.contains("未設定")
        && !normalized.contains("未定")
        && !normalized.contains("unknown")
        && !normalized.contains("tbd")
        && !normalized.contains("n/a")
        && !normalized.contains("to be determined")
}

/// Query keys that make a URL a credential or a signed temporary locator.
///
/// This is a heuristic, not a proof: it recognises well-known signing
/// parameters and rejects them, while ordinary publication queries such as
/// `?id=123`, `?lang=ja` or `?article=foo` stay valid.
pub fn sensitive_query_key(locator: &str) -> Option<String> {
    if !locator.contains('?') {
        return None;
    }
    let query = locator.split_once('?')?.1;
    let query = query.split('#').next().unwrap_or_default();
    query.split('&').find_map(|part| {
        let raw = part.split('=').next().unwrap_or_default();
        let key = percent_decode(raw).to_ascii_lowercase();
        let key = key.trim();
        let sensitive = matches!(
            key,
            "token"
                | "access_token"
                | "refresh_token"
                | "id_token"
                | "api_key"
                | "apikey"
                | "key"
                | "secret"
                | "client_secret"
                | "password"
                | "passwd"
                | "pwd"
                | "auth"
                | "authorization"
                | "credential"
                | "credentials"
                | "signature"
                | "sig"
                | "sas"
                | "x-amz-signature"
                | "x-amz-credential"
                | "x-amz-security-token"
        ) || key.starts_with("x-amz-")
            || key.ends_with("_token")
            || key.ends_with("-token")
            || key.ends_with("_secret")
            || key.ends_with("_key");
        sensitive.then(|| key.to_owned())
    })
}

/// Minimal percent-decoding for query keys, so `access%5Ftoken` is still seen.
fn percent_decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = String::with_capacity(raw.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[index + 1..index + 3])
                .ok()
                .and_then(|hex| u8::from_str_radix(hex, 16).ok());
            if let Some(byte) = hex {
                out.push(byte as char);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index] as char);
        index += 1;
    }
    out
}

/// A URL carrying an authority with embedded userinfo (`user:pass@host`).
pub fn url_has_credentials(locator: &str) -> bool {
    locator
        .split_once("://")
        .and_then(|(_, rest)| rest.split(['/', '?', '#']).next())
        .is_some_and(|authority| authority.contains('@'))
}
