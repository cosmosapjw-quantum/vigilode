//! The Markdown report is a view of the JSON: every number it prints appears
//! in the JSON (as a number formatted by the report's formatter, or inside a
//! JSON string copied verbatim), and no row is dropped.

use std::collections::BTreeSet;

use serde_json::Value;
use vigilode_bench::{corpus::Corpus, harness, report};

const TINY: &str = r#"{
  "schema": "vigilode-bench.corpus.v1",
  "id": "test-tiny",
  "grid_points": 11,
  "rtols": [1e-4, 1e-6],
  "warmups": 0,
  "repetitions": 2,
  "error_rtol": 1.0,
  "error_levels": [1e-3, 1e-4, 1e-5, 1e-6, 1e-7],
  "reference": {
    "arm": "vigilode", "rtol": 1e-11, "max_steps": 1000000,
    "crosscheck_arm": "diffsol-bdf-dense", "crosscheck_rtols": [1e-11, 1e-10],
    "dispute_threshold": 1e-8,
    "informational_arms": ["radau5"]
  },
  "arms": ["vigilode", "vigilode-banded", "diffsol-bdf-dense", "cvode-bdf-dense", "cvode-bdf-band", "dvode-mf21", "radau5", "rodas", "internal-radau-iia3"],
  "problems": [
    {"id": "linear", "kind": "linear-exact", "n": 2, "t_end": 2.0, "atol_scale": [1e-2, 1e-2], "reference_atol": 1e-13, "banded": true},
    {"id": "rober", "kind": "robertson", "n": 3, "t_end": 1.0, "atol_scale": [1e-4, 1e-4, 1e-4], "reference_atol": 1e-16, "banded": false}
  ]
}"#;

fn collect(v: &Value, out: &mut BTreeSet<String>) {
    match v {
        Value::Number(n) => {
            out.insert(report::fmt_number(n));
        }
        Value::String(s) => out.extend(report::numeric_tokens(s)),
        Value::Array(a) => a.iter().for_each(|x| collect(x, out)),
        Value::Object(m) => m.iter().for_each(|(k, x)| {
            out.extend(report::numeric_tokens(k));
            collect(x, out)
        }),
        _ => {}
    }
}

#[test]
fn every_markdown_number_appears_in_the_json_and_no_row_is_dropped() {
    let corpus = Corpus::from_json(TINY).unwrap();
    let mut log = |_: &str| {};
    let mut doc = harness::run_corpus(&corpus, &mut log).unwrap();
    doc["provenance"] = serde_json::json!({"git_commit": "test", "host_logical_cpus": 2});
    // Round-trip through text, as the CLI does.
    let doc: Value = serde_json::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
    let md = report::render(&doc).unwrap();
    let mut allowed = BTreeSet::new();
    collect(&doc, &mut allowed);
    let missing: Vec<String> = report::numeric_tokens(&md)
        .into_iter()
        .filter(|t| !allowed.contains(t))
        .collect();
    assert!(missing.is_empty(), "numbers not in the JSON: {missing:?}");

    let rows = doc["rows"].as_array().unwrap();
    // linear (banded): 9 arms; rober (dense only): 7 arms; 2 rtols each.
    assert_eq!(rows.len(), (9 + 7) * 2);
    for r in rows {
        assert_eq!(r["repetitions"].as_array().unwrap().len(), 2);
        assert!(md.contains(&report::cell(&r["wall_ms_median"])));
    }
    assert!(doc["matched"].as_array().unwrap().iter().all(|m| {
        let s = m["status"].as_str().unwrap();
        (s == "MATCHED") == m["time_ratio"].is_number()
    }));
    assert!(
        doc["matched"]
            .as_array()
            .unwrap()
            .iter()
            .all(|m| m["arm"] != "internal-radau-iia3")
    );
}

#[test]
fn numeric_tokens_see_numbers_and_skip_identifiers() {
    let t = report::numeric_tokens("| radau5 | 1.234e-5 | 42 | L-0139 | (3) | x2 | 7. |");
    assert_eq!(t, vec!["1.234e-5", "42", "3", "7"]);
}
