//! docs/CONSTANTS_PROVENANCE.toml stays in sync with the source (audit F-046).

use std::collections::BTreeMap;

use rodas5p_integrators::g4_s5b0_policy_constants;

const TABLE: &str = include_str!("../../../docs/CONSTANTS_PROVENANCE.toml");
const ATLAS: &str = include_str!("../src/g4_s5b0_regime_atlas.rs");
const CLI: &str = include_str!("../../rodas5p-cli/src/main.rs");

const CLASSES: [&str; 5] = ["derived", "fitted", "declared", "cited", "unsourced"];
const STATUSES: [&str; 3] = ["active", "frozen", "retired"];
const ROLES: [&str; 3] = ["theory-bound", "calibration-only", "product-policy"];
const FIELDS: [&str; 13] = [
    "name",
    "role",
    "value",
    "location",
    "class",
    "source_artifact",
    "source_sha256",
    "fit_dataset",
    "n",
    "selection_criterion",
    "sensitivity",
    "valid_scope",
    "status",
];

/// The table uses a flat subset of TOML: `[[constant]]` headers and
/// `key = "string"` lines.
fn rows() -> Vec<BTreeMap<String, String>> {
    let mut rows = Vec::new();
    for line in TABLE.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "[[constant]]" {
            rows.push(BTreeMap::new());
            continue;
        }
        let (key, value) = line.split_once(" = ").expect("key = value line");
        let value = value
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .expect("string value");
        let row = rows.last_mut().expect("row header before fields");
        assert!(
            row.insert(key.to_string(), value.to_string()).is_none(),
            "duplicate {key}"
        );
    }
    rows
}

/// The literal of `const NAME: TYPE = LITERAL;` in `source`.
fn source_literal(source: &str, name: &str) -> String {
    let needle = format!("const {name}: ");
    let line = source
        .lines()
        .find(|line| line.contains(&needle))
        .unwrap_or_else(|| panic!("{name} not declared"));
    line.split_once(" = ")
        .and_then(|(_, rest)| rest.strip_suffix(';'))
        .unwrap_or_else(|| panic!("{name}: unexpected declaration {line}"))
        .trim()
        .to_string()
}

#[test]
fn constants_provenance_table_matches_source() {
    let rows = rows();
    let compiled = g4_s5b0_policy_constants()
        .into_iter()
        .collect::<BTreeMap<_, _>>();
    let mut names = Vec::new();
    for row in &rows {
        for field in FIELDS {
            assert!(row.contains_key(field), "{row:?} lacks {field}");
        }
        assert_eq!(row.len(), FIELDS.len(), "unknown field in {row:?}");
        let name = &row["name"];
        names.push(name.clone());
        assert!(CLASSES.contains(&row["class"].as_str()), "{name}: class");
        assert!(STATUSES.contains(&row["status"].as_str()), "{name}: status");
        assert!(ROLES.contains(&row["role"].as_str()), "{name}: role");
        if row["class"] == "unsourced" {
            assert_eq!(row["role"], "calibration-only", "{name}: unsourced role");
        }
        let value: f64 = row["value"].parse().expect("numeric value");
        let source = match row["location"].as_str() {
            "crates/rodas5p-integrators/src/g4_s5b0_regime_atlas.rs" => ATLAS,
            "crates/rodas5p-cli/src/main.rs" => CLI,
            other => panic!("{name}: unknown location {other}"),
        };
        let literal: f64 = source_literal(source, name).parse().unwrap();
        assert_eq!(value.to_bits(), literal.to_bits(), "{name}: source literal");
        if let Some(compiled) = compiled.get(name.as_str()) {
            assert_eq!(
                value.to_bits(),
                compiled.to_bits(),
                "{name}: compiled value"
            );
        }
        match row["class"].as_str() {
            "fitted" => {
                for field in ["source_artifact", "fit_dataset", "n", "selection_criterion"] {
                    assert!(!row[field].is_empty(), "{name}: fitted needs {field}");
                }
            }
            "unsourced" => assert!(row["source_artifact"].is_empty(), "{name}"),
            _ => {}
        }
        if !row["source_artifact"].is_empty() {
            assert_eq!(row["source_sha256"].len(), 64, "{name}: sha256");
        }
    }
    // Every versioned policy constant of the atlas has a row.
    for line in ATLAS.lines() {
        let declaration = line
            .trim_start()
            .trim_start_matches("pub ")
            .strip_prefix("const V");
        if let Some(rest) = declaration
            && rest.starts_with(|c: char| c.is_ascii_digit())
        {
            let name = format!("V{}", rest.split(':').next().unwrap());
            assert!(names.contains(&name), "{name} has no provenance row");
            assert!(
                compiled.contains_key(name.as_str()),
                "{name} not in registry"
            );
        }
    }
    assert_eq!(
        compiled.len(),
        names.iter().filter(|n| n.starts_with('V')).count()
    );
}

#[test]
fn source_artifact_hashes_are_current() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    for row in rows() {
        if row["source_artifact"].is_empty() {
            continue;
        }
        let bytes = std::fs::read(format!("{root}/{}", row["source_artifact"]))
            .unwrap_or_else(|e| panic!("{}: {e}", row["name"]));
        assert_eq!(
            rodas5p_core::sha256_hex(&bytes),
            row["source_sha256"],
            "{}",
            row["name"]
        );
    }
}
