use osmium_core::parsing::{MAX_DOCUMENT_BYTES, parse_json};
use serde_json::json;

#[test]
fn duplicate_keys_are_rejected_including_escaped_equivalent_names() {
    for input in [
        r#"{"id":"first","id":"second"}"#,
        r#"{"nested":[{"id":1,"id":2}]}"#,
        r#"{"id":1,"\u0069d":2}"#,
    ] {
        let error = parse_json(input.as_bytes(), "test.json").unwrap_err();
        assert_eq!(error.code, "OSM_JSON");
        assert_eq!(error.file.as_deref(), Some("test.json"));
        assert!(error.message.contains("duplicate object key"));
        assert!(error.line.unwrap() >= 1);
        assert!(error.column.unwrap() > 0);
    }
}

#[test]
fn invalid_utf8_trailing_documents_and_excess_depth_fail() {
    for input in [b"\"\xff\"".as_slice(), b"{} {}", b"{", b"1e9999"] {
        assert!(parse_json(input, "bad.json").is_err());
    }
    let deep = format!("{}0{}", "[".repeat(200), "]".repeat(200));
    assert!(parse_json(deep.as_bytes(), "deep.json").is_err());
}

#[test]
fn document_byte_limit_is_inclusive() {
    let mut bytes = vec![b' '; MAX_DOCUMENT_BYTES];
    bytes[0] = b'0';
    assert_eq!(parse_json(&bytes, "limit.json").unwrap(), json!(0));
    bytes.push(b' ');
    assert_eq!(
        parse_json(&bytes, "limit.json").unwrap_err().code,
        "OSM_INPUT_LIMIT"
    );
}

#[test]
fn unknown_extension_values_preserve_their_json_types() {
    let value = json!({"org.example.future.v1":{"values":[null,true,-17,0.5,"教材",{"x":[]} ]}});
    let bytes = serde_json::to_vec(&value).unwrap();
    assert_eq!(parse_json(&bytes, "extensions.json").unwrap(), value);
}
