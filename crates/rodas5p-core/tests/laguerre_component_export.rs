//! Per-term export of the Laguerre total (research node
//! `research/py01_laguerre_component_export_20261011`, DAG node PY01).
//!
//! Contract tests (the DAG command, `cargo test -p rodas5p-core --test
//! laguerre_component_export`): report parity with `joint_phi_action` on two
//! fresh small fixtures, identity stamping, and the typed refusal (no export)
//! of a declared enclosure and of a degree above the adjoint limit.
//!
//! The ignored `export_components` re-executes the nine registered R-NEXT-04
//! cases from the published bits of `A`, `h` and `w` and writes
//! `PY01_COMPONENTS` (compact JSON) for `tools/py01_laguerre_component_check.py`.

use rodas5p_core::{
    CoreError, DenseMatrix, WorkCounters,
    directed::{add_up, mul_up},
    laguerre_adjoint::{
        LAGUERRE_ADJOINT_DEGREE_LIMIT, LAGUERRE_ADJOINT_DEPTH, laguerre_adjoint_envelopes,
        laguerre_adjoint_recurrence_bound, laguerre_adjoint_recurrence_bound_with_envelopes,
    },
    polynomial_action::{
        EnclosureEvidence, JointPhiInput, JointPhiReport, LAGUERRE_COMPONENTS_UNAVAILABLE,
        LaguerreComponentExport, PolynomialBasis, SymmetricNonpositiveOperator,
        TotalErrorAdmission, joint_phi_action, joint_phi_action_laguerre_components,
    },
};
use serde_json::{Value, json};

const SCALES: [f64; 5] = [1.0, -0.5, 0.25, 2.0, -1.0];
const PUBLISHED: &str = "research/rnext04_laguerre_admission_20261003/cases.json";
const PUBLISHED_SHA256: &str = "700726f0bd2d58d84fbd1e42bf4daa66efd80312057021bc1e69515f28a01b7d";
/// The nine registered executions: six gated, three controls.
const CASES: [&str; 9] = [
    "n6-rho400-h0.1-distinct",
    "n6-rho400-h0.1-same",
    "n12-rho400-h0.1-distinct",
    "n12-rho400-h0.1-same",
    "n20-rho400-h0.1-distinct",
    "n20-rho400-h0.1-same",
    "n6-rho400-h0.01-distinct",
    "subnormal-1e-310",
    "large-1e300",
];

fn hex(v: f64) -> String {
    format!("{:016x}", v.to_bits())
}

fn unhex(text: &str) -> f64 {
    assert_eq!(text.len(), 16, "{text}");
    f64::from_bits(u64::from_str_radix(text, 16).unwrap())
}

fn splitmix(seed: u64, n: usize) -> Vec<f64> {
    let mut state = seed;
    (0..n)
        .map(|_| {
            state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^= z >> 31;
            2.0 * ((z >> 11) as f64 / (1u64 << 53) as f64) - 1.0
        })
        .collect()
}

/// A fresh diagonally dominant symmetric nonpositive matrix (Gershgorin
/// discs in `[-1.2 rho, 0)`), as in the R-NEXT-04 source, other seeds.
fn fixture_matrix(n: usize, rho: f64, seed: u64) -> DenseMatrix {
    let d: Vec<f64> = splitmix(seed, n)
        .iter()
        .map(|u| rho * (0.65 + 0.35 * u))
        .collect();
    let raw = splitmix(seed + 1000, n * n);
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        a[(i, i)] = -d[i];
        for j in 0..i {
            let v = 0.2 * d[i].min(d[j]) / n as f64 * raw[i * n + j];
            a[(i, j)] = v;
            a[(j, i)] = v;
        }
    }
    a
}

/// The R-NEXT-04 operator rule.
fn operator(a: DenseMatrix, rho: f64) -> SymmetricNonpositiveOperator {
    SymmetricNonpositiveOperator::new(a, 0.0, 1.2 * rho * (1.0 + 1.0e-12), "py01").unwrap()
}

/// Bitwise report equality: serde_json writes the shortest round-trip form
/// of every finite binary64 (with the sign of zero), so equal text is equal
/// bits.
fn same_report(a: &JointPhiReport, b: &JointPhiReport) -> bool {
    serde_json::to_string(a).unwrap() == serde_json::to_string(b).unwrap()
        && a.fused
            .iter()
            .map(|x| x.to_bits())
            .eq(b.fused.iter().map(|x| x.to_bits()))
        && a.columns.iter().flatten().map(|x| x.to_bits()).eq(b
            .columns
            .iter()
            .flatten()
            .map(|x| x.to_bits()))
}

enum Input {
    Distinct([Vec<f64>; 5]),
    Same(Vec<f64>),
}

impl Input {
    fn as_input(&self) -> JointPhiInput<'_> {
        match self {
            Input::Distinct(w) => JointPhiInput::Distinct(w),
            Input::Same(v) => JointPhiInput::SameVector {
                vector: v,
                scales: SCALES,
            },
        }
    }
}

/// The export together with `joint_phi_action` on fresh counters; asserts
/// parity of the report and of the counters.
fn run_with_parity(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: &Input,
    budget: f64,
) -> (JointPhiReport, LaguerreComponentExport, WorkCounters) {
    let mut work = WorkCounters::default();
    let (report, export) =
        joint_phi_action_laguerre_components(op, h, input.as_input(), budget, &mut work).unwrap();
    let mut base_work = WorkCounters::default();
    let base = joint_phi_action(
        op,
        h,
        input.as_input(),
        PolynomialBasis::Laguerre,
        budget,
        None,
        &mut base_work,
    )
    .unwrap();
    assert!(
        same_report(&report, &base),
        "report differs from joint_phi_action"
    );
    assert_eq!(work, base_work, "work counters differ");
    (report, export, work)
}

/// Every stamp equals the recomputed identity digest.
fn stamps_hold(export: &LaguerreComponentExport) -> bool {
    let digest = export.identity.sha256();
    export.identity_sha256 == digest
        && export.assembly.identity_sha256 == digest
        && export.terms.iter().all(|t| t.identity_sha256 == digest)
        && export.adjoint.iter().all(|a| a.identity_sha256 == digest)
}

/// The exported adjoint sub-decomposition reproduces the exported
/// `recurrence_adjoint` (in the window; out of it the 2^shift scaling is
/// left to the checker).
fn adjoint_binds(export: &LaguerreComponentExport) -> bool {
    export.adjoint.iter().zip(&export.terms).all(|(a, t)| {
        let beta: Vec<f64> = a.envelopes.iter().map(|x| unhex(x)).collect();
        let eps: Vec<f64> = a.residuals.iter().map(|x| unhex(x)).collect();
        let mut sum = 0.0;
        for (b, e) in beta[1..].iter().zip(&eps) {
            sum = add_up(sum, mul_up(*b, *e).unwrap()).unwrap();
        }
        let value = mul_up(unhex(&a.scale_abs), sum).unwrap();
        export.identity.normalization_shift != 0 || hex(value) == t.recurrence_adjoint
    })
}

#[test]
fn report_parity_on_fresh_fixtures() {
    // Fixture 1: distinct vectors inside the normalization window.
    let (n, rho, h) = (7, 80.0, 0.05);
    let op = operator(fixture_matrix(n, rho, 4101), rho);
    assert!(matches!(
        op.enclosure().evidence,
        EnclosureEvidence::Gershgorin
    ));
    let input = Input::Distinct(std::array::from_fn(|k| splitmix(4200 + 31 * k as u64, n)));
    let (report, export, work) = run_with_parity(&op, h, &input, 1.0e-10);
    assert_eq!(report.normalization_shift, 0);
    assert!(report.degree > 0 && report.degree <= LAGUERRE_ADJOINT_DEGREE_LIMIT);
    assert!(work.poly_block_products > 0);
    assert_eq!(export.terms.len(), 5);
    assert_eq!(export.adjoint.len(), 5);
    assert_eq!(
        export.assembly.laguerre_adjoint_total,
        hex(report.laguerre_adjoint_total.unwrap())
    );
    assert_eq!(export.assembly.fused_summation, hex(report.fused_summation));
    for (k, (t, c)) in export.terms.iter().zip(&report.column_errors).enumerate() {
        assert_eq!(t.column, k);
        assert_eq!(t.truncation, hex(c.truncation));
        assert_eq!(t.coefficient, hex(c.coefficient));
        assert_eq!(t.recurrence_adjoint, hex(c.recurrence_adjoint.unwrap()));
        assert_eq!(t.summation, hex(c.summation));
        assert_eq!(t.normalization, hex(c.normalization));
    }
    for a in &export.adjoint {
        assert_eq!(a.coefficients.len(), report.degree + 1);
        assert_eq!(a.envelopes.len(), report.degree + 1);
        assert_eq!(a.residuals.len(), report.degree);
        // The exported envelopes are those of the exported coefficients.
        let stored: Vec<f64> = a.coefficients.iter().map(|x| unhex(x)).collect();
        let extent = unhex(&export.identity.laguerre_extent);
        let beta = laguerre_adjoint_envelopes(&stored, extent, LAGUERRE_ADJOINT_DEPTH).unwrap();
        assert_eq!(
            beta.iter().map(|x| hex(*x)).collect::<Vec<_>>(),
            a.envelopes
        );
        let eps: Vec<f64> = a.residuals.iter().map(|x| unhex(x)).collect();
        let (with, _) = laguerre_adjoint_recurrence_bound_with_envelopes(
            &stored,
            extent,
            &eps,
            LAGUERRE_ADJOINT_DEPTH,
        )
        .unwrap();
        let plain =
            laguerre_adjoint_recurrence_bound(&stored, extent, &eps, LAGUERRE_ADJOINT_DEPTH)
                .unwrap();
        assert_eq!(with.to_bits(), plain.to_bits());
    }
    assert!(adjoint_binds(&export));

    // Fixture 2: one vector with the R-NEXT-04 scales, far below the
    // normalization window (the out-of-window path).
    let (n, rho, h) = (5, 30.0, 0.02);
    let op = operator(fixture_matrix(n, rho, 4301), rho);
    let amplitude = 1.0e-305;
    let v: Vec<f64> = splitmix(4400, n).iter().map(|x| amplitude * x).collect();
    let input = Input::Same(v);
    let (report, export, _) = run_with_parity(&op, h, &input, 1.0e-10 * amplitude);
    assert!(
        report.normalization_shift < -500,
        "{}",
        report.normalization_shift
    );
    assert_eq!(
        export.identity.normalization_shift,
        report.normalization_shift
    );
    assert_eq!(export.identity.input, "same-vector");
    for (k, a) in export.adjoint.iter().enumerate() {
        assert_eq!(unhex(&a.scale_abs), SCALES[k].abs());
    }
    // Same-vector columns share one recurrence, hence one residual list.
    assert!(
        export
            .adjoint
            .iter()
            .all(|a| a.residuals == export.adjoint[0].residuals)
    );
}

#[test]
fn identity_stamping() {
    let (n, rho) = (6, 60.0);
    let op = operator(fixture_matrix(n, rho, 4501), rho);
    let v = splitmix(4600, n);
    let w: [Vec<f64>; 5] = std::array::from_fn(|k| v.iter().map(|x| SCALES[k] * x).collect());
    let same = Input::Same(v.clone());
    let distinct = Input::Distinct(w);
    let (_, a, _) = run_with_parity(&op, 0.03, &same, 1.0e-10);
    let (_, b, _) = run_with_parity(&op, 0.03, &distinct, 1.0e-10);
    let (_, c, _) = run_with_parity(&op, 0.04, &same, 1.0e-10);
    let (_, a2, _) = run_with_parity(&op, 0.03, &same, 1.0e-10);
    for export in [&a, &b, &c] {
        assert!(stamps_hold(export));
        assert_eq!(export.identity_sha256.len(), 64);
        assert_eq!(export.identity.adjoint_depth, LAGUERRE_ADJOINT_DEPTH);
        assert_eq!(export.identity.envelope_key_digests.len(), 5);
        for (digest, a) in export
            .identity
            .envelope_key_digests
            .iter()
            .zip(&export.adjoint)
        {
            assert_eq!(digest, &a.envelope_key.coefficients_sha256);
            assert_eq!(a.envelope_key.extent_bits, export.identity.laguerre_extent);
            assert_eq!(a.envelope_key.depth, LAGUERRE_ADJOINT_DEPTH);
            assert_eq!(a.envelope_key.degree, export.identity.degree);
        }
    }
    // The input kind and h are part of the identity; a rerun is the same.
    assert_ne!(a.identity_sha256, b.identity_sha256);
    assert_ne!(a.identity_sha256, c.identity_sha256);
    assert_eq!(a, a2);
    // A changed identity field no longer matches the stamps.
    let mut tampered = a.clone();
    tampered.identity.degree += 1;
    assert!(!stamps_hold(&tampered));
    let mut restamped = a.clone();
    restamped.terms[2].identity_sha256 = b.identity_sha256.clone();
    assert!(!stamps_hold(&restamped));
    // The canonical encoding is sorted compact JSON.
    let encoding = a.identity.canonical_encoding();
    let parsed: Value = serde_json::from_str(&encoding).unwrap();
    let keys: Vec<&String> = parsed.as_object().unwrap().keys().collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted);
    assert!(!encoding.contains(' ') && !encoding.contains('\n'));
}

fn refused(result: Result<(JointPhiReport, LaguerreComponentExport), CoreError>) -> bool {
    matches!(result, Err(CoreError::InvalidInput(ref m)) if m.starts_with(LAGUERRE_COMPONENTS_UNAVAILABLE))
}

#[test]
fn refusal_without_a_complete_adjoint_total() {
    // A declared enclosure: the largest diagonal magnitude contains the
    // diagonal but not the Gershgorin discs.
    let (n, rho) = (6, 50.0);
    let a = fixture_matrix(n, rho, 4701);
    let diagonal_max = (0..n).map(|i| a[(i, i)].abs()).fold(0.0, f64::max);
    let declared = SymmetricNonpositiveOperator::new(a, 0.0, diagonal_max, "declared").unwrap();
    assert!(matches!(
        declared.enclosure().evidence,
        EnclosureEvidence::Declared { .. }
    ));
    let w: [Vec<f64>; 5] = std::array::from_fn(|k| splitmix(4800 + k as u64, n));
    let mut work = WorkCounters::default();
    assert!(refused(joint_phi_action_laguerre_components(
        &declared,
        0.01,
        JointPhiInput::Distinct(&w),
        1.0e-10,
        &mut work,
    )));

    // A degree above the adjoint limit: a tight budget at a large h rho.
    let (n, rho) = (4, 400.0);
    let op = operator(fixture_matrix(n, rho, 4901), rho);
    let w: [Vec<f64>; 5] = std::array::from_fn(|k| splitmix(5000 + k as u64, n));
    let (h, budget) = (0.2, 1.0e-13);
    let report = joint_phi_action(
        &op,
        h,
        JointPhiInput::Distinct(&w),
        PolynomialBasis::Laguerre,
        budget,
        None,
        &mut WorkCounters::default(),
    )
    .unwrap();
    assert!(
        report.degree > LAGUERRE_ADJOINT_DEGREE_LIMIT,
        "degree {}",
        report.degree
    );
    assert!(report.laguerre_adjoint_total.is_none());
    assert!(refused(joint_phi_action_laguerre_components(
        &op,
        h,
        JointPhiInput::Distinct(&w),
        budget,
        &mut WorkCounters::default(),
    )));

    // The scalar branch (A = -2 I) has no recurrence and no adjoint total.
    let mut scalar = DenseMatrix::zeros(3, 3);
    for i in 0..3 {
        scalar[(i, i)] = -2.0;
    }
    let op = SymmetricNonpositiveOperator::new(scalar, 2.0, 2.0, "scalar").unwrap();
    let w: [Vec<f64>; 5] = std::array::from_fn(|k| splitmix(5100 + k as u64, 3));
    assert!(refused(joint_phi_action_laguerre_components(
        &op,
        0.1,
        JointPhiInput::Distinct(&w),
        1.0e-10,
        &mut WorkCounters::default(),
    )));
}

// ---------------------------------------------------------------------------
// The recorded export (registered command 2)
// ---------------------------------------------------------------------------

fn git(args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?} failed");
    String::from_utf8(out.stdout).unwrap()
}

fn hex_matrix(rows: &Value) -> Vec<Vec<f64>> {
    rows.as_array()
        .unwrap()
        .iter()
        .map(|row| {
            row.as_array()
                .unwrap()
                .iter()
                .map(|x| unhex(x.as_str().unwrap()))
                .collect()
        })
        .collect()
}

/// The registered operator rule, input kind and truncation budget of a
/// label (the R-NEXT-04 source: rho 50 and budget 1e-10 x amplitude for the
/// two scale controls, rho 400 and 1e-10 otherwise).
fn registered(label: &str) -> (f64, f64, bool) {
    let (rho, amplitude) = match label {
        "subnormal-1e-310" => (50.0, 1.0e-310),
        "large-1e300" => (50.0, 1.0e300),
        _ => (400.0, 1.0),
    };
    (rho, 1.0e-10 * amplitude, label.ends_with("-same"))
}

/// Execute one case through the export and the plain action and return its
/// COMPONENTS record.
fn case_record(
    label: &str,
    a: &DenseMatrix,
    h: f64,
    w: &[Vec<f64>; 5],
    admission_budget: f64,
) -> Value {
    let n = a.nrows();
    let (rho, truncation_budget, same) = registered(label);
    let input = if same {
        let v = w[0].clone();
        for k in 0..5 {
            assert!(
                w[k].iter()
                    .zip(&v)
                    .all(|(x, y)| x.to_bits() == (SCALES[k] * y).to_bits()),
                "{label}: w_{k} is not scales[{k}] w_0"
            );
        }
        Input::Same(v)
    } else {
        Input::Distinct(w.clone())
    };
    let op = operator(a.clone(), rho);
    let mut work = WorkCounters::default();
    let (report, export) = joint_phi_action_laguerre_components(
        &op,
        h,
        input.as_input(),
        truncation_budget,
        &mut work,
    )
    .unwrap();
    // Parity with the plain action on fresh counters (also a contract test).
    let mut base_work = WorkCounters::default();
    let base = joint_phi_action(
        &op,
        h,
        input.as_input(),
        PolynomialBasis::Laguerre,
        truncation_budget,
        None,
        &mut base_work,
    )
    .unwrap();
    let admission = match report.admit_laguerre_total(admission_budget) {
        TotalErrorAdmission::Admitted { bound, .. } => {
            json!({"admitted": true, "bound": hex(bound)})
        }
        TotalErrorAdmission::Rejected { reason } => json!({"admitted": false, "reason": reason}),
    };
    println!(
        "{label}: degree {} shift {} total {:e}",
        report.degree,
        report.normalization_shift,
        report.laguerre_adjoint_total.unwrap()
    );
    json!({
        "label": label,
        "n": n,
        "a": (0..n).map(|i| (0..n).map(|j| hex(a[(i, j)])).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "h": hex(h),
        "w": w.iter().map(|v| v.iter().map(|x| hex(*x)).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "input": if same { "same-vector" } else { "distinct" },
        "scales": if same { SCALES.iter().map(|x| hex(*x)).collect::<Vec<_>>() } else { vec![hex(1.0); 5] },
        "rho_nominal": hex(rho),
        "admission_budget": hex(admission_budget),
        "report": {
            "degree": report.degree,
            "laguerre_scale": hex(report.laguerre_scale.unwrap()),
            "normalization_shift": report.normalization_shift,
            "fused": report.fused.iter().map(|x| hex(*x)).collect::<Vec<_>>(),
            "laguerre_adjoint_total": hex(report.laguerre_adjoint_total.unwrap()),
            "truncation_bound_exact_arithmetic": hex(report.truncation_bound_exact_arithmetic),
            "evidence": format!("{:?}", report.evidence),
            "admission": admission,
        },
        "parity": {
            "report_bitwise_equal": same_report(&report, &base),
            "counters_equal": work == base_work,
            "counters": serde_json::to_value(work).unwrap(),
        },
        "export": serde_json::to_value(&export).unwrap(),
    })
}

/// Writes the document (compact JSON) to `path` when it is set (a relative
/// path is taken from the workspace root; an existing file is never
/// overwritten). Unset (e.g. the CI run of every ignored test), nothing is
/// written and the parity of every case is asserted instead.
fn write_document(path: Option<String>, cases: Vec<Value>) {
    let Some(path) = path else {
        for case in &cases {
            assert_eq!(
                case["parity"]["report_bitwise_equal"], true,
                "{}",
                case["label"]
            );
            assert_eq!(case["parity"]["counters_equal"], true, "{}", case["label"]);
        }
        println!("no output path set; nothing written");
        return;
    };
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out_path = root.join(&path);
    assert!(!out_path.exists(), "immutable output exists: {path}");
    let source_commit = git(&["rev-parse", "HEAD"]).trim().to_string();
    let tree_status = git(&["status", "--porcelain"]);
    let document = json!({
        "schema": "vigilode-py01-laguerre-components-v1",
        "node": "research/py01_laguerre_component_export_20261011",
        "source_commit": source_commit,
        "tree_status": tree_status,
        "published": {"path": PUBLISHED, "sha256": PUBLISHED_SHA256},
        "cases": cases,
    });
    std::fs::write(&out_path, serde_json::to_string(&document).unwrap() + "\n").unwrap();
    println!("wrote {path}");
}

#[test]
#[ignore = "recorded run of research node PY01; writes PY01_COMPONENTS when set"]
fn export_components() {
    let path = std::env::var("PY01_COMPONENTS").ok();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bytes = std::fs::read(root.join(PUBLISHED)).unwrap();
    assert_eq!(
        rodas5p_core::sha256_hex(&bytes),
        PUBLISHED_SHA256,
        "published cases changed"
    );
    let published: Value = serde_json::from_slice(&bytes).unwrap();
    let published = published["cases"].as_array().unwrap();
    let mut cases = Vec::new();
    for label in CASES {
        let rows: Vec<&Value> = published.iter().filter(|c| c["label"] == label).collect();
        assert_eq!(rows.len(), 1, "{label}");
        let case = rows[0];
        let n = case["n"].as_u64().unwrap() as usize;
        let a_rows = hex_matrix(&case["a"]);
        let mut a = DenseMatrix::zeros(n, n);
        for i in 0..n {
            for j in 0..n {
                a[(i, j)] = a_rows[i][j];
            }
        }
        let h = unhex(case["h"].as_str().unwrap());
        let w_rows = hex_matrix(&case["w"]);
        let w: [Vec<f64>; 5] = std::array::from_fn(|k| w_rows[k].clone());
        cases.push(case_record(
            label,
            &a,
            h,
            &w,
            case["budget"].as_f64().unwrap(),
        ));
    }
    write_document(path, cases);
}

/// Not a registered command: the same record format on fresh fixtures under
/// the nine labels (fresh matrices and vectors, h = 0.01), written to
/// `PY01_FIXTURE_COMPONENTS` to check the checker's schema before the
/// recorded run. No registered case is executed.
#[test]
#[ignore = "format check on fresh fixtures; writes PY01_FIXTURE_COMPONENTS when set"]
fn export_fresh_fixture_records() {
    let path = std::env::var("PY01_FIXTURE_COMPONENTS").ok();
    let mut cases = Vec::new();
    for (index, label) in CASES.iter().enumerate() {
        let (rho, _, same) = registered(label);
        let n = 3;
        let a = fixture_matrix(n, rho, 6000 + 10 * index as u64);
        let amplitude = match *label {
            "subnormal-1e-310" => 1.0e-310,
            "large-1e300" => 1.0e300,
            _ => 1.0,
        };
        let w: [Vec<f64>; 5] = if same {
            let v = splitmix(6500 + index as u64, n);
            std::array::from_fn(|k| v.iter().map(|x| SCALES[k] * x).collect())
        } else {
            std::array::from_fn(|k| {
                splitmix(6600 + 10 * index as u64 + k as u64, n)
                    .iter()
                    .map(|x| amplitude * x)
                    .collect()
            })
        };
        cases.push(case_record(label, &a, 0.01, &w, 1.0e-6 * amplitude));
    }
    write_document(path, cases);
}
