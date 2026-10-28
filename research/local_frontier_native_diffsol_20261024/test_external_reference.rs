//! Focused reference intake and ensemble validation controls.
//! Run: cargo test --offline -p vigilode --example local_bottleneck_suite -j 2
//! Synthetic reference states below exercise intake only; they are not evidence
//! of an independently validated scientific endpoint.
use super::*;

fn fixture(m: &Model) -> Value {
    json!({
        "schema": EXTERNAL_REFERENCE_SCHEMA,
        "model_descriptor": model_descriptor(m),
        "model_descriptor_sha256": "a".repeat(64),
        "norm": "max_component_absolute",
        "reference_state": m.exact(m.end).unwrap_or_else(|| m.initial()),
        "uncertainty": 1e-12,
        "provenance": {
            "source": "synthetic contract test",
            "method": "analytic endpoint or intake-only fixture",
            "evidence_sha256": "b".repeat(64)
        }
    })
}

fn parse(v: &Value, m: &Model) -> Result<EndpointReference, String> {
    parse_external_reference(
        &serde_json::to_vec(v).unwrap(),
        m,
        Some("c".repeat(64)),
        1e-4,
    )
}

fn outcome(m: &Model, state: Vec<f64>) -> Outcome {
    Outcome {
        success: true,
        state,
        time: m.end,
        stats: Value::Null,
        message: String::new(),
    }
}

#[test]
fn external_reference_preserves_identity_uncertainty_and_provenance() {
    let m = Model::new(1, true).unwrap();
    let f = fixture(&m);
    let r = parse(&f, &m).unwrap();
    assert_eq!(r.state, m.initial());
    assert_eq!(r.uncertainty, 1e-12);
    assert_eq!(r.descriptor_sha256, Some("a".repeat(64)));
    assert_eq!(r.file_sha256, Some("c".repeat(64)));
    assert_eq!(r.provenance, f["provenance"]);
    assert!(r.external);
}

#[test]
fn changed_case_size_dimension_time_initial_parameters_grid_boundary_or_probes_reject() {
    let m = Model::new(6, true).unwrap();
    let original = fixture(&m);
    for (pointer, bad) in [
        ("/case_id", json!(1)),
        ("/size", json!("standard")),
        ("/model_id", json!("other")),
        ("/t_start", json!(0.1)),
        ("/t_end", json!(m.end + 0.1)),
        ("/initial_state/0", json!(2.0)),
        ("/parameters/gamma0", json!(1.0)),
        ("/parameters/boundary", json!("Dirichlet")),
        ("/model_contract/dimension", json!(m.n + 1)),
        ("/model_contract/grid/0", json!(m.nx + 1)),
        ("/rhs_probes/1/rhs/0", json!(123.0)),
    ] {
        let mut v = original.clone();
        *v["model_descriptor"].pointer_mut(pointer).unwrap() = bad;
        let error = parse(&v, &m).err().unwrap();
        assert!(error.contains("model_descriptor"), "{pointer}: {error}");
    }
    let mut v = original;
    v["model_descriptor"]
        .as_object_mut()
        .unwrap()
        .remove("model_contract");
    assert!(parse(&v, &m).is_err());
}

#[test]
fn schemas_unknown_fields_missing_fields_and_duplicate_fields_reject() {
    let m = Model::new(1, true).unwrap();
    let f = fixture(&m);
    let mut v = f.clone();
    v["schema"] = json!("vigilode.local-bottleneck.external-reference.v2");
    assert!(parse(&v, &m).is_err());
    let mut v = f.clone();
    v["ignored_override"] = json!(true);
    assert!(parse(&v, &m).is_err());
    for key in f.as_object().unwrap().keys() {
        let mut v = f.clone();
        v.as_object_mut().unwrap().remove(key);
        assert!(parse(&v, &m).is_err(), "missing {key}");
    }
    let serialized = serde_json::to_string(&f).unwrap();
    let duplicate = format!("{{\"uncertainty\":0.0,{}", &serialized[1..]);
    assert!(parse_external_reference(duplicate.as_bytes(), &m, None, 1e-4).is_err());
}

#[test]
fn reference_requires_exact_norm_and_full_finite_numeric_state() {
    let m = Model::new(1, true).unwrap();
    for state in [
        json!([]),
        json!([1.0]),
        json!([1.0, 0.0, 0.0, 0.0]),
        json!([1.0, null, 0.0]),
        json!([1.0, "NaN", 0.0]),
    ] {
        let mut v = fixture(&m);
        v["reference_state"] = state;
        assert!(parse(&v, &m).is_err());
    }
    let mut v = fixture(&m);
    v["norm"] = json!("wrms");
    assert!(parse(&v, &m).is_err());
    let encoded = serde_json::to_string(&fixture(&m)).unwrap();
    let encoded = encoded.replace(
        "\"reference_state\":[1.0,0.0,0.0]",
        "\"reference_state\":[1e400,0.0,0.0]",
    );
    assert!(encoded.contains("1e400"));
    assert!(parse_external_reference(encoded.as_bytes(), &m, None, 1e-4).is_err());
}

#[test]
fn uncertainty_range_and_fixed_accuracy_cap_are_enforced() {
    let m = Model::new(1, true).unwrap();
    for uncertainty in [json!(-1e-12), json!(1.00001e-5), Value::Null, json!("NaN")] {
        let mut v = fixture(&m);
        v["uncertainty"] = uncertainty;
        assert!(parse(&v, &m).is_err());
    }
    for uncertainty in [0.0, 1e-5] {
        let mut v = fixture(&m);
        v["uncertainty"] = json!(uncertainty);
        assert!(parse(&v, &m).is_ok());
    }
    let bytes = serde_json::to_vec(&fixture(&m)).unwrap();
    for cap in [0.0, -1e-4, 1.00001e-4, f64::INFINITY, f64::NAN] {
        assert!(parse_external_reference(&bytes, &m, None, cap).is_err());
    }
    assert!(parse_external_reference(&bytes, &m, None, 1e-5).is_ok());
}

#[test]
fn hashes_and_provenance_are_required_but_are_not_claimed_as_locally_hashed() {
    let m = Model::new(1, true).unwrap();
    for digest in ["", "abc", &"A".repeat(64), &"g".repeat(64)] {
        let mut v = fixture(&m);
        v["model_descriptor_sha256"] = json!(digest);
        assert!(parse(&v, &m).is_err());
    }
    let bytes = serde_json::to_vec(&fixture(&m)).unwrap();
    assert!(parse_external_reference(&bytes, &m, Some("invalid".into()), 1e-4).is_err());
    for provenance in [
        Value::Null,
        json!("source"),
        json!({}),
        json!({"source":"", "method":"m", "evidence_sha256":"b".repeat(64)}),
        json!({"source":"s", "method":" ", "evidence_sha256":"b".repeat(64)}),
        json!({"source":"s", "method":"m", "evidence_sha256":"bad"}),
    ] {
        let mut v = fixture(&m);
        v["provenance"] = provenance;
        assert!(parse(&v, &m).is_err());
    }
}

#[test]
fn external_reference_cannot_replace_a_conflicting_builtin() {
    let m = Model::new(15, true).unwrap();
    let mut v = fixture(&m);
    assert!(parse(&v, &m).is_ok());
    let exact = v["reference_state"][0].as_f64().unwrap();
    v["reference_state"][0] = json!(exact + 1e-3);
    v["uncertainty"] = json!(1e-5);
    assert!(parse(&v, &m).err().unwrap().contains("conflicts"));
}

#[test]
fn inaccurate_nonfirst_member_prevents_pass_and_reports_its_index() {
    let m = Model::new(1, true).unwrap();
    let mut f = fixture(&m);
    f["uncertainty"] = json!(1e-5);
    let r = parse(&f, &m).unwrap();
    let mut late = r.state.clone();
    late[1] += 9.5e-5;
    let results = vec![
        outcome(&m, r.state.clone()),
        outcome(&m, r.state.clone()),
        outcome(&m, late),
    ];
    let a = assess_members(&m, &results, Some(&r), 1e-4);
    assert!(a.success);
    assert!(!a.pass);
    assert_eq!(a.status, "ACCURACY_FAILED");
    assert_eq!(a.maximum_error_member_index, Some(2));
    assert_eq!(a.difference, Some(9.5e-5));
    assert_eq!(a.error, Some(9.5e-5 + 1e-5));
}

#[test]
fn every_member_must_be_complete_finite_and_full_length() {
    let m = Model::new(1, true).unwrap();
    let r = parse(&fixture(&m), &m).unwrap();
    for fault in 0..6 {
        let good = outcome(&m, r.state.clone());
        let mut bad = outcome(&m, r.state.clone());
        match fault {
            0 => bad.success = false,
            1 => bad.time = m.end / 2.0,
            2 => {
                bad.state.pop();
            }
            3 => bad.state.push(0.0),
            4 => bad.state[1] = f64::NAN,
            _ => bad.time = f64::NAN,
        }
        let a = assess_members(&m, &[good, bad], Some(&r), 1e-4);
        assert!(!a.success, "fault {fault}");
        assert!(!a.pass, "fault {fault}");
        assert_eq!(a.status, "SOLVER_FAILED");
    }
    assert!(!assess_members(&m, &[], Some(&r), 1e-4).pass);
}

#[test]
fn no_reference_remains_unvalidated_and_zero_error_ties_select_first_member() {
    let m = Model::new(1, true).unwrap();
    let r = parse(&fixture(&m), &m).unwrap();
    let results = vec![outcome(&m, r.state.clone()), outcome(&m, r.state.clone())];
    let absent = assess_members(&m, &results, None, 1e-4);
    assert!(absent.success);
    assert!(!absent.pass);
    assert_eq!(absent.status, "UNVALIDATED_REFERENCE");
    assert!(absent.error.is_none());
    let valid = assess_members(&m, &results, Some(&r), 1e-4);
    assert!(valid.pass);
    assert_eq!(valid.maximum_error_member_index, Some(0));
}

#[test]
fn reference_intake_and_assessment_preserve_solver_state_and_work_counters() {
    let m = Model::new(15, true).unwrap();
    let opts = Options {
        rtol: 1e-6,
        atol: Atol::Scalar(1e-9),
        h0: 1e-4,
        max_steps: 100000,
        ..Default::default()
    };
    let plain_counts = Arc::new(Counters::default());
    let plain = solve(&m, "dopri5", &opts, plain_counts.clone(), None, false);
    let r = parse(&fixture(&m), &m).unwrap();
    let ref_counts = Arc::new(Counters::default());
    let with_ref = solve(&m, "dopri5", &opts, ref_counts.clone(), None, false);
    assert_eq!(plain_counts.counts(), ref_counts.counts());
    assert_eq!(plain.stats, with_ref.stats);
    assert_eq!(plain.time.to_bits(), with_ref.time.to_bits());
    assert_eq!(
        plain.state.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
        with_ref
            .state
            .iter()
            .map(|x| x.to_bits())
            .collect::<Vec<_>>()
    );
    let before = ref_counts.counts();
    assert!(assess_members(&m, &[with_ref], Some(&r), 1e-4).pass);
    assert_eq!(before, ref_counts.counts());
    let builtin = builtin_reference(&m).unwrap();
    assert!(assess_members(&m, &[plain], Some(&builtin), 1e-4).pass);
}
