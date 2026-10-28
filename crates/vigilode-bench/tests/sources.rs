//! SOURCES.json and native/SHA256SUMS name the same files with the same hashes.

use std::collections::BTreeMap;

#[test]
fn sources_json_matches_sha256sums() {
    let root = env!("CARGO_MANIFEST_DIR");
    let sums = std::fs::read_to_string(format!("{root}/native/SHA256SUMS")).unwrap();
    let sums: BTreeMap<String, String> = sums
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let (h, p) = l.split_once("  ").unwrap();
            (p.to_string(), h.to_string())
        })
        .collect();
    let json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(format!("{root}/native/SOURCES.json")).unwrap(),
    )
    .unwrap();
    let files: BTreeMap<String, String> = json["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["path"].as_str().unwrap().to_string(),
                f["sha256"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    assert_eq!(sums, files);
    assert!(
        json["files"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["modified"] == false)
    );
}
