use osmium_core::yaml::parse_yaml;
use serde_json::json;

#[test]
fn authoring_yaml_preserves_json_types_and_quoted_strings() {
    let value = parse_yaml("schema_version: \"0.1\"\ntext: 日本語\nitems: [true, null, 1, 0.5, 'true']\nblock: |\n  hello\n".as_bytes(), "osmium.yaml").unwrap();
    assert_eq!(value["schema_version"], "0.1");
    assert_eq!(value["items"], json!([true, null, 1, 0.5, "true"]));
    assert_eq!(value["block"], "hello\n");
}

#[test]
fn ambiguous_or_executable_yaml_features_fail() {
    for input in [
        "a: 1\na: 2",
        "a: 1\n'a': 2",
        "a: &x [1]\nb: *x",
        "a: !custom value",
        "a: !!str 1",
        "<<: {a: 1}",
        "1: value",
        "? [a,b]\n: x",
        "---\na: 1\n---\na: 2",
        "a: [",
    ] {
        assert!(parse_yaml(input.as_bytes(), "bad.yaml").is_err(), "{input}");
    }
}

#[test]
fn depth_utf8_and_byte_limits_are_enforced() {
    let deep = format!("{}0{}", "[".repeat(80), "]".repeat(80));
    assert!(parse_yaml(deep.as_bytes(), "deep.yaml").is_err());
    assert!(parse_yaml(&[0xff], "bad.yaml").is_err());
    let oversized = vec![b' '; 4 * 1024 * 1024 + 1];
    assert_eq!(
        parse_yaml(&oversized, "big.yaml").unwrap_err().code,
        "OSM_INPUT_LIMIT"
    );
}

#[test]
fn duplicate_key_coordinates_are_real() {
    let error = parse_yaml(b"a: 1\na: 2\n", "bad.yaml").unwrap_err();
    assert_eq!(error.line, Some(2));
    assert_eq!(error.column, Some(1));
}
