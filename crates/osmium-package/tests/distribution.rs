use osmium_package::distribution::{
    archive_bytes, build, canonical_json, compile_source, read_distribution, sha256,
};
use serde_json::json;
use std::{
    fs,
    io::{Cursor, Write},
    path::{Path, PathBuf},
};
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

fn source() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/arithmetic")
}

#[test]
fn canonical_profile_and_hash_have_fixed_vectors() {
    assert_eq!(
        sha256(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        canonical_json(&json!({"z":0,"a":["日本",true,null],"nested":{"b":2,"a":1}})),
        "{\"a\":[\"日本\",true,null],\"nested\":{\"a\":1,\"b\":2},\"z\":0}\n".as_bytes()
    );
    assert_eq!(
        canonical_json(&json!([0, 1.0, -0.0, 1.25, u64::MAX, "\n\t\u{0001}/\\\""])),
        b"[0,1.0,-0.0,1.25,18446744073709551615,\"\\n\\t\\u0001/\\\\\\\"\"]\n"
    );
}

#[test]
fn source_whitespace_is_normalized_and_optional_payload_survives() {
    let temp = tempfile::tempdir().unwrap();
    let loaded = osmium_package::load_source(source()).unwrap();
    for (name, bytes) in loaded.files {
        let path = temp.path().join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    let manifest_path = temp.path().join("osmium.json");
    let mut manifest = loaded.model.documents().manifest.clone();
    manifest["extensions"] = json!({"org.example.notes.v1":{"text":"保持", "decimal":1.2345678901234567, "small":9.999e-100, "list":[true,null]}});
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let first = compile_source(temp.path()).unwrap();
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let path = temp.path().join("content/introduction.md");
    let content = fs::read_to_string(&path).unwrap();
    fs::write(path, content.replace("\r\n", "\n").replace('\n', "\r\n")).unwrap();
    let second = compile_source(temp.path()).unwrap();
    assert_eq!(first.digest, second.digest);
    assert_eq!(
        second.model.documents().manifest["extensions"],
        manifest["extensions"]
    );
}

#[test]
fn future_distribution_and_nested_package_versions_are_explicitly_rejected() {
    for field in ["distribution_version", "canonicalization", "schema_version"] {
        let temp = tempfile::tempdir().unwrap();
        let folder = temp.path().join("built");
        build(&source(), &folder).unwrap();
        let path = folder.join("manifest.json");
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        if field == "schema_version" {
            value["package"][field] = json!("99");
        } else {
            value[field] = json!("99");
        }
        fs::write(path, canonical_json(&value)).unwrap();
        assert_eq!(
            read_distribution(&folder).unwrap_err()[0].code,
            "OSM_SCHEMA_VERSION"
        );
    }
}

#[test]
fn directory_and_zip_are_identical_and_build_is_reproducible() {
    let dir = tempfile::tempdir().unwrap();
    let original = fs::read(source().join("osmium.json")).unwrap();
    let first = compile_source(&source()).unwrap();
    let second = compile_source(&source()).unwrap();
    assert_eq!(
        first.digest,
        "c6a3d3c41accec038421326dfca988125352ef71586890c66c21cd3d149159ba"
    );
    assert_eq!(
        sha256(&archive_bytes(&first).unwrap()),
        "12ae2b3b0bcb3251f80762bb0c7d570182e85d0ea08a69aed19ba3e1029d1ecd"
    );
    assert_eq!(first.digest, second.digest);
    assert_eq!(
        archive_bytes(&first).unwrap(),
        archive_bytes(&second).unwrap()
    );
    let folder = dir.path().join("built");
    let zip = dir.path().join("built.osmium");
    let a = build(&source(), &folder).unwrap();
    let b = build(&source(), &zip).unwrap();
    assert_eq!(a.digest, b.digest);
    assert!(b.archive_sha256.is_some());
    let plain = read_distribution(&folder).unwrap();
    let packed = read_distribution(&zip).unwrap();
    assert_eq!(plain.files, packed.files);
    assert_eq!(
        plain.model.documents().manifest,
        packed.model.documents().manifest
    );
    assert_eq!(fs::read(source().join("osmium.json")).unwrap(), original);
    assert!(!folder.join("osmium.json").exists());
}

#[test]
fn tampered_extra_missing_and_noncanonical_files_fail() {
    for mode in ["tamper", "extra", "missing", "manifest"] {
        let temp = tempfile::tempdir().unwrap();
        let folder = temp.path().join("built");
        build(&source(), &folder).unwrap();
        match mode {
            "tamper" => fs::write(folder.join("content/introduction.md"), "tampered").unwrap(),
            "extra" => fs::write(folder.join("extra.md"), "extra").unwrap(),
            "missing" => fs::remove_file(folder.join("content/introduction.md")).unwrap(),
            _ => {
                let path = folder.join("manifest.json");
                let mut bytes = fs::read(&path).unwrap();
                bytes.push(b' ');
                fs::write(path, bytes).unwrap();
            }
        }
        assert!(read_distribution(&folder).is_err(), "{mode}");
    }
}

#[test]
fn existing_outputs_and_invalid_sources_are_preserved() {
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("keep.osmium");
    fs::write(&output, "keep").unwrap();
    assert!(build(&source(), &output).is_err());
    assert_eq!(fs::read_to_string(&output).unwrap(), "keep");
    let empty = temp.path().join("empty");
    fs::create_dir(&empty).unwrap();
    let missing = temp.path().join("missing");
    assert!(build(&empty, &missing).is_err());
    assert!(!missing.exists());
}

fn zip_entries(entries: &[(&str, Vec<u8>)], method: CompressionMethod) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in entries {
        writer
            .start_file(
                *name,
                SimpleFileOptions::default().compression_method(method),
            )
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[test]
fn unsafe_archive_names_and_expansion_are_rejected_without_extracting() {
    for name in [
        "../escape.md",
        "/absolute.md",
        "C:/escape.md",
        "a\\b.md",
        "a/CON.md",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("bad.osmium");
        fs::write(
            &file,
            zip_entries(&[(name, b"bad".to_vec())], CompressionMethod::Stored),
        )
        .unwrap();
        assert!(read_distribution(&file).is_err(), "{name}");
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
    }
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("bomb.osmium");
    fs::write(
        &file,
        zip_entries(
            &[("big.md", vec![b'0'; 4 * 1024 * 1024 + 1])],
            CompressionMethod::Deflated,
        ),
    )
    .unwrap();
    assert!(
        read_distribution(&file)
            .unwrap_err()
            .iter()
            .any(|e| e.code == "OSM_INPUT_LIMIT")
    );
}

#[test]
fn duplicate_zip_directory_names_cannot_be_hidden_by_zip_index() {
    // Create equal-length names, then replace both occurrences of b.md by a.md.
    let mut bytes = zip_entries(
        &[("a.md", b"one".to_vec()), ("b.md", b"two".to_vec())],
        CompressionMethod::Stored,
    );
    for i in 0..bytes.len() - 3 {
        if &bytes[i..i + 4] == b"b.md" {
            bytes[i] = b'a';
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("duplicate.osmium");
    fs::write(&file, &bytes).unwrap();
    assert!(read_distribution(&file).is_err());
    // Forging the footer count must not hide duplicates either.
    let end = bytes.len() - 22;
    bytes[end + 8..end + 10].copy_from_slice(&1u16.to_le_bytes());
    bytes[end + 10..end + 12].copy_from_slice(&1u16.to_le_bytes());
    fs::write(&file, bytes).unwrap();
    assert!(read_distribution(&file).is_err());
}
