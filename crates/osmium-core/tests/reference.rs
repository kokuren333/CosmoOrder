//! The Reference/Evidence vocabulary and its compatibility mapping are shared by
//! validation, lint, distribution and the runtime projector, so the mapping is
//! pinned here rather than re-derived in each caller.

use osmium_core::reference::{
    self, EVIDENCE, LEGACY_EVIDENCE, LEGACY_REFERENCES, REFERENCES, Visibility, registry,
    registry_field,
};
use serde_json::json;

#[test]
fn the_two_visibility_axes_reproduce_the_legacy_enum() {
    // public: record and locator both distribute.
    let public = json!({"visibility":"public"});
    assert_eq!(reference::visibility(&public), Visibility::Public);
    assert!(reference::visibility(&public).is_record_public());
    assert!(reference::visibility(&public).is_locator_public());

    // attribution_only: attributable record, authoring-only locator.
    let attribution_only = json!({"visibility":"attribution_only"});
    assert_eq!(
        reference::visibility(&attribution_only),
        Visibility::AttributionOnly
    );
    assert!(reference::visibility(&attribution_only).is_record_public());
    assert!(!reference::visibility(&attribution_only).is_locator_public());

    // private: Authoring Provenance that leaked into a Package.
    let private = json!({"visibility":"private"});
    assert_eq!(reference::visibility(&private), Visibility::Private);
    assert!(!reference::visibility(&private).is_record_public());
    assert!(!reference::visibility(&private).is_locator_public());

    // The split spelling is equivalent on both axes.
    assert_eq!(
        reference::visibility(&json!({"record_visibility":"public","locator_visibility":"public"})),
        Visibility::Public
    );
    assert_eq!(
        reference::visibility(&json!({"record_visibility":"public","locator_visibility":"hidden"})),
        Visibility::AttributionOnly
    );
    assert_eq!(
        reference::visibility(&json!({"record_visibility":"private"})),
        Visibility::Private
    );

    // A contradictory record is treated as the most restrictive reading: if any
    // axis says private, the whole record is private.
    assert_eq!(
        reference::visibility(&json!({
            "visibility":"public",
            "record_visibility":"private",
            "locator_visibility":"public"
        })),
        Visibility::Private
    );
    // A legacy private declaration also wins over permissive split fields.
    assert_eq!(
        reference::visibility(&json!({
            "visibility":"private",
            "record_visibility":"public",
            "locator_visibility":"public"
        })),
        Visibility::Private
    );
    // An absent visibility defaults to public, matching the schema default the
    // loader applies for legacy records.
    assert_eq!(reference::visibility(&json!({})), Visibility::Public);
}

#[test]
fn registry_and_evidence_names_prefer_the_canonical_spelling() {
    assert_eq!(
        registry_field(&json!({"sources":[]})),
        Some(LEGACY_REFERENCES)
    );
    assert_eq!(registry_field(&json!({"references":[]})), Some(REFERENCES));
    assert_eq!(
        registry_field(&json!({"references":[],"sources":[]})),
        Some(REFERENCES)
    );
    assert_eq!(registry_field(&json!({})), None);
    assert!(registry(&json!({})).is_none());
    assert_eq!(
        registry(&json!({"references":[{"id":"a"}]})).unwrap().len(),
        1
    );

    assert_eq!(
        reference::evidence_field(&json!({"source_ids":[]})),
        LEGACY_EVIDENCE
    );
    assert_eq!(
        reference::evidence_field(&json!({"evidence_reference_ids":[]})),
        EVIDENCE
    );
    assert_eq!(
        reference::evidence_field(&json!({"evidence_reference_ids":[],"source_ids":[]})),
        EVIDENCE
    );
    // A half-migrated file must not fall back to stale legacy IDs.
    assert_eq!(
        reference::evidence_ids(&json!({"source_ids":["old"],"evidence_reference_ids":["new"]}))
            .collect::<Vec<_>>(),
        ["new"]
    );
    assert!(reference::has_evidence(
        &json!({"evidence_reference_ids":["a"]})
    ));
    assert!(!reference::has_evidence(
        &json!({"evidence_reference_ids":[]})
    ));
    assert!(!reference::has_evidence(&json!({})));
}

#[test]
fn a_license_is_meaningful_only_when_the_status_is_known_and_the_text_says_something() {
    assert!(!reference::license_is_known(&json!({})));
    assert!(!reference::license_is_known(&json!({"license":"CC0-1.0"})));
    assert!(!reference::license_is_known(
        &json!({"license":"CC0-1.0","license_status":"unknown"})
    ));
    assert!(!reference::license_is_known(
        &json!({"license":"CC0-1.0","license_status":"unspecified"})
    ));
    // Placeholder prose that only fills the field is not metadata.
    assert!(!reference::license_is_known(
        &json!({"license":"再利用条件は未設定。","license_status":"known"})
    ));
    assert!(!reference::license_is_known(
        &json!({"license":"unknown","license_status":"known"})
    ));
    assert!(!reference::license_is_known(
        &json!({"license":"TBD","license_status":"known"})
    ));
    assert!(!reference::license_is_known(
        &json!({"license":"   ","license_status":"known"})
    ));
    assert!(reference::license_is_known(
        &json!({"license":"CC BY 4.0","license_status":"known"})
    ));
    assert!(reference::license_is_known(
        &json!({"license":"Original text; reuse permitted with attribution.","license_status":"known"})
    ));
}

#[test]
fn sensitive_query_detection_is_a_heuristic_that_names_the_key_it_rejected() {
    assert_eq!(
        reference::sensitive_query_key("https://a.example/x?id=1"),
        None
    );
    assert_eq!(reference::sensitive_query_key("https://a.example/x"), None);
    assert_eq!(
        reference::sensitive_query_key("https://a.example/x?token=abc").as_deref(),
        Some("token")
    );
    assert_eq!(
        reference::sensitive_query_key("https://a.example/x?X-Amz-Signature=abc").as_deref(),
        Some("x-amz-signature")
    );
    assert_eq!(
        reference::sensitive_query_key("https://a.example/x?session_token=abc").as_deref(),
        Some("session_token")
    );
    assert_eq!(
        reference::sensitive_query_key("https://a.example/x?key=abc").as_deref(),
        Some("key")
    );
    // The heuristic is not a proof: an opaque single-letter parameter that
    // happens to be a secret is not detected, and that is documented.
    assert_eq!(
        reference::sensitive_query_key("https://a.example/x?s=opaque"),
        None
    );

    assert!(reference::url_has_credentials(
        "https://user:secret@example.org/g"
    ));
    assert!(reference::url_has_credentials("https://user@example.org/g"));
    assert!(!reference::url_has_credentials("https://example.org/g"));
    assert!(!reference::url_has_credentials("https://example.org/a@b"));
    assert!(!reference::url_has_credentials(
        "https://example.org?email=person@example.net"
    ));
}
