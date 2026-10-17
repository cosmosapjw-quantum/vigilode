//! Contract test suite and component exporter for Laguerre action error export (PY01).

use std::{
    alloc::{GlobalAlloc, Layout, System},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

use rodas5p_core::{
    CoreError, DenseMatrix, WorkCounters,
    directed::{Interval, add_up, mul_up},
    laguerre_adjoint::{EnvelopeKey, LAGUERRE_ADJOINT_DEPTH},
    polynomial_action::{
        JointPhiInput, PolynomialBasis, SymmetricNonpositiveOperator, joint_phi_action,
        joint_phi_action_laguerre_components, scale_bound_up,
    },
};
use serde_json::{Value, json};

// -----------------------------------------------------------------------------
// VG-1 Zero-Allocation Allocator
// -----------------------------------------------------------------------------

struct CountingAllocator;

thread_local! {
    static TRACK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() && TRACK.with(|t| t.get()) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
    }
}

fn count_allocations<T>(operation: impl FnOnce() -> T) -> (T, u64, u64) {
    ALLOCATIONS.store(0, Ordering::SeqCst);
    ALLOCATED_BYTES.store(0, Ordering::SeqCst);
    TRACK.with(|t| t.set(true));
    let result = operation();
    TRACK.with(|t| t.set(false));
    (
        result,
        ALLOCATIONS.load(Ordering::SeqCst),
        ALLOCATED_BYTES.load(Ordering::SeqCst),
    )
}

fn hex(v: f64) -> String {
    format!("{:016x}", v.to_bits())
}

fn f64_from_hex(s: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(s, 16).expect("valid 16-hex digit float"))
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

fn test_matrix(n: usize, rho: f64, seed: u64) -> DenseMatrix {
    let d: Vec<f64> = splitmix(seed, n)
        .iter()
        .map(|u| rho * (0.65 + 0.35 * u))
        .collect();
    let raw = splitmix(seed + 1000, n * n);
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        a[(i, i)] = -d[i];
        for j in 0..i {
            let cap = 0.2 * d[i].min(d[j]) / n as f64;
            let v = cap * raw[i * n + j];
            a[(i, j)] = v;
            a[(j, i)] = v;
        }
    }
    a
}

// -----------------------------------------------------------------------------
// Contract Tests
// -----------------------------------------------------------------------------

#[test]
fn report_and_counter_parity_with_joint_phi_action() {
    // Fixture 1: 4x4 matrix, distinct vectors.
    let n = 4;
    let rho = 30.0;
    let a = test_matrix(n, rho, 12345);
    let op = SymmetricNonpositiveOperator::new(a.clone(), 0.0, 1.2 * rho * (1.0 + 1e-12), "test")
        .unwrap();
    let h = 0.05;
    let budget = 1e-8;

    let w1: [Vec<f64>; 5] = std::array::from_fn(|k| splitmix(100 + k as u64, n));
    let mut work_standard = WorkCounters::default();
    let report_standard = joint_phi_action(
        &op,
        h,
        JointPhiInput::Distinct(&w1),
        PolynomialBasis::Laguerre,
        budget,
        None,
        &mut work_standard,
    )
    .unwrap();

    let mut work_export = WorkCounters::default();
    let (report_export, _export) = joint_phi_action_laguerre_components(
        &op,
        h,
        JointPhiInput::Distinct(&w1),
        budget,
        &mut work_export,
    )
    .unwrap();

    assert_eq!(report_standard.degree, report_export.degree);
    assert_eq!(report_standard.laguerre_scale, report_export.laguerre_scale);
    assert_eq!(
        report_standard.normalization_shift,
        report_export.normalization_shift
    );
    assert_eq!(
        report_standard.laguerre_adjoint_total.map(|v| v.to_bits()),
        report_export.laguerre_adjoint_total.map(|v| v.to_bits())
    );
    assert_eq!(
        report_standard.fused_summation.to_bits(),
        report_export.fused_summation.to_bits()
    );
    assert_eq!(
        report_standard
            .fused
            .iter()
            .map(|x| x.to_bits())
            .collect::<Vec<_>>(),
        report_export
            .fused
            .iter()
            .map(|x| x.to_bits())
            .collect::<Vec<_>>()
    );
    assert_eq!(work_standard, work_export);

    // Fixture 2: 4x4 matrix, same-vector input with scales.
    let v2 = splitmix(9999, n);
    let scales2 = [1.0, -0.5, 0.25, 2.0, -1.0];
    let h2 = 0.02;
    let budget2 = 1e-9;

    let mut work_standard2 = WorkCounters::default();
    let report_standard2 = joint_phi_action(
        &op,
        h2,
        JointPhiInput::SameVector {
            vector: &v2,
            scales: scales2,
        },
        PolynomialBasis::Laguerre,
        budget2,
        None,
        &mut work_standard2,
    )
    .unwrap();

    let mut work_export2 = WorkCounters::default();
    let (report_export2, _export2) = joint_phi_action_laguerre_components(
        &op,
        h2,
        JointPhiInput::SameVector {
            vector: &v2,
            scales: scales2,
        },
        budget2,
        &mut work_export2,
    )
    .unwrap();

    assert_eq!(report_standard2.degree, report_export2.degree);
    assert_eq!(
        report_standard2.laguerre_scale,
        report_export2.laguerre_scale
    );
    assert_eq!(
        report_standard2.normalization_shift,
        report_export2.normalization_shift
    );
    assert_eq!(
        report_standard2.laguerre_adjoint_total.map(|v| v.to_bits()),
        report_export2.laguerre_adjoint_total.map(|v| v.to_bits())
    );
    assert_eq!(
        report_standard2.fused_summation.to_bits(),
        report_export2.fused_summation.to_bits()
    );
    assert_eq!(
        report_standard2
            .fused
            .iter()
            .map(|x| x.to_bits())
            .collect::<Vec<_>>(),
        report_export2
            .fused
            .iter()
            .map(|x| x.to_bits())
            .collect::<Vec<_>>()
    );
    assert_eq!(work_standard2, work_export2);
}

#[test]
fn identity_stamping_and_envelope_key_digest() {
    let n = 4;
    let rho = 20.0;
    let a = test_matrix(n, rho, 42);
    let op = SymmetricNonpositiveOperator::new(a.clone(), 0.0, 1.2 * rho * (1.0 + 1e-12), "test")
        .unwrap();
    let h = 0.05;
    let budget = 1e-8;
    let w: [Vec<f64>; 5] = std::array::from_fn(|k| splitmix(200 + k as u64, n));

    let mut work = WorkCounters::default();
    let (_report, export) = joint_phi_action_laguerre_components(
        &op,
        h,
        JointPhiInput::Distinct(&w),
        budget,
        &mut work,
    )
    .unwrap();

    let identity = &export.identity;
    let stamp = &identity.identity_sha256;
    assert_eq!(stamp, &identity.compute_sha256());

    // Check stamp propagation on every column term block.
    for col in &export.columns {
        assert_eq!(&col.identity_sha256, stamp);
    }
    // Check stamp propagation on assembly.
    assert_eq!(&export.assembly.identity_sha256, stamp);
    // Check stamp propagation on adjoint columns.
    for adj in &export.adjoint_columns {
        assert_eq!(&adj.identity_sha256, stamp);
    }

    // Verify envelope key digest matches (coefficients, extent, depth).
    let extent = f64_from_hex(&identity.laguerre_extent);
    for k in 0..5 {
        let coef_f64: Vec<f64> = export.adjoint_columns[k]
            .coefficients
            .iter()
            .map(|s| f64_from_hex(s))
            .collect();
        let expected_key_digest =
            EnvelopeKey::digest_from_f64(&coef_f64, extent, LAGUERRE_ADJOINT_DEPTH).unwrap();
        assert_eq!(
            identity.envelope_key_digests[k], expected_key_digest,
            "Column {k} envelope key digest mismatch"
        );
    }

    // Verify that modifying any identity field changes the recomputed SHA-256.
    let mut modified = identity.clone();
    modified.degree += 1;
    assert_ne!(modified.compute_sha256(), *stamp);

    let mut modified_h = identity.clone();
    modified_h.h = hex(h * 2.0);
    assert_ne!(modified_h.compute_sha256(), *stamp);

    let mut modified_fp = identity.clone();
    modified_fp.operator_fingerprint = "tampered_fingerprint".into();
    assert_ne!(modified_fp.compute_sha256(), *stamp);
}

#[test]
fn refusal_on_declared_enclosure_and_degree_above_128() {
    let n = 4;
    let rho = 20.0;
    let a = test_matrix(n, rho, 77);
    let w: [Vec<f64>; 5] = std::array::from_fn(|k| splitmix(300 + k as u64, n));

    // Refusal 1: Declared (unverified) enclosure.
    let op_declared =
        SymmetricNonpositiveOperator::with_declared_enclosure(a.clone(), 0.0, rho, "declared")
            .unwrap();
    let mut work = WorkCounters::default();
    let err_declared = joint_phi_action_laguerre_components(
        &op_declared,
        0.05,
        JointPhiInput::Distinct(&w),
        1e-8,
        &mut work,
    );
    assert!(
        matches!(err_declared, Err(CoreError::InvalidInput(_))),
        "Expected InvalidInput error on declared enclosure, got: {err_declared:?}"
    );

    // Refusal 2: Degree > 128 (e.g. very large rho*h with tiny truncation budget).
    // rho = 2000.0, h = 1.0, budget = 1e-15 pushes degree way past 128.
    let op_large = SymmetricNonpositiveOperator::new(
        test_matrix(n, 2000.0, 88),
        0.0,
        1.2 * 2000.0 * (1.0 + 1e-12),
        "test",
    )
    .unwrap();
    let err_degree = joint_phi_action_laguerre_components(
        &op_large,
        1.0,
        JointPhiInput::Distinct(&w),
        1e-15,
        &mut work,
    );
    assert!(
        matches!(err_degree, Err(CoreError::InvalidInput(_))),
        "Expected InvalidInput error on degree > 128, got: {err_degree:?}"
    );
}

#[test]
fn vg1_zero_allocation_in_recurrence_and_directed_primitives() {
    // Directed interval arithmetic primitives: zero allocation.
    let i1 = Interval::point(1.2345).unwrap();
    let i2 = Interval::point(6.7890).unwrap();

    let (_, allocs, bytes) = count_allocations(|| {
        let mut val = i1;
        for _ in 0..100 {
            val = val.add(i2).unwrap();
            val = val.mul(i1).unwrap();
        }
        val
    });
    assert_eq!(allocs, 0, "Interval arithmetic allocated heap memory");
    assert_eq!(bytes, 0);

    // Directed f64 rounding primitives: zero allocation.
    let (_, allocs2, bytes2) = count_allocations(|| {
        let mut x = 1.0;
        for _ in 0..100 {
            x = add_up(x, 0.01).unwrap();
            x = mul_up(x, 1.001).unwrap();
        }
        x
    });
    assert_eq!(allocs2, 0, "Directed f64 primitives allocated heap memory");
    assert_eq!(bytes2, 0);

    // In-place vector step accumulation on pre-allocated buffers: zero allocation.
    let mut u = vec![1.0; 64];
    let v = vec![2.0; 64];
    let (_, allocs3, bytes3) = count_allocations(|| {
        for (ui, vi) in u.iter_mut().zip(&v) {
            *ui = add_up(*ui, *vi).unwrap();
        }
    });
    assert_eq!(
        allocs3, 0,
        "In-place vector accumulation allocated heap memory"
    );
    assert_eq!(bytes3, 0);
}

// -----------------------------------------------------------------------------
// Published Cases Loader
// -----------------------------------------------------------------------------

fn load_published_case(target_label: &str) -> (DenseMatrix, [Vec<f64>; 5], f64, f64, f64, Value) {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let cases_path = repo_root.join("research/rnext04_laguerre_admission_20261003/cases.json");
    assert!(
        cases_path.exists(),
        "cases.json must exist at {cases_path:?}"
    );

    let cases_bytes = std::fs::read(&cases_path).expect("read cases.json");
    let cases_val: Value = serde_json::from_slice(&cases_bytes).expect("parse cases.json");
    let all_cases = cases_val["cases"].as_array().expect("cases array");
    let pub_case = all_cases
        .iter()
        .find(|c| c["label"].as_str() == Some(target_label))
        .unwrap_or_else(|| panic!("Target case {target_label} not found in cases.json"));

    let n = pub_case["n"].as_u64().unwrap() as usize;
    let h_bits = pub_case["h"].as_str().unwrap();
    let h = f64_from_hex(h_bits);

    // Rebuild A matrix from hex bits.
    let a_json = pub_case["a"].as_array().unwrap();
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        let row = a_json[i].as_array().unwrap();
        for j in 0..n {
            a[(i, j)] = f64_from_hex(row[j].as_str().unwrap());
        }
    }

    // Rebuild w vectors from hex bits.
    let w_json = pub_case["w"].as_array().unwrap();
    let mut w: [Vec<f64>; 5] = std::array::from_fn(|_| Vec::with_capacity(n));
    for k in 0..5 {
        let col = w_json[k].as_array().unwrap();
        for item in col {
            w[k].push(f64_from_hex(item.as_str().unwrap()));
        }
    }

    // Determine nominal rho and input style.
    let (rho, trunc_budget) = if target_label == "subnormal-1e-310" {
        (50.0, 1.0e-10 * 1.0e-310)
    } else if target_label == "large-1e300" {
        (50.0, 1.0e-10 * 1.0e300)
    } else {
        (400.0, 1.0e-10)
    };

    (a, w, h, rho, trunc_budget, pub_case.clone())
}

// -----------------------------------------------------------------------------
// VG-2 Outward Rebuild and Monotonicity Tests
// -----------------------------------------------------------------------------

#[test]
fn vg2_outward_rebuild_and_nonzero_normalization_shift() {
    let test_cases = ["n6-rho400-h0.1-distinct", "subnormal-1e-310", "large-1e300"];
    for label in test_cases {
        let (a, w, h, rho, trunc_budget, _) = load_published_case(label);
        let op =
            SymmetricNonpositiveOperator::new(a, 0.0, 1.2 * rho * (1.0 + 1e-12), "test").unwrap();
        let mut work = WorkCounters::default();
        let (_report, export) = joint_phi_action_laguerre_components(
            &op,
            h,
            JointPhiInput::Distinct(&w),
            trunc_budget,
            &mut work,
        )
        .unwrap();

        let shift = export.identity.normalization_shift;
        if label == "n6-rho400-h0.1-distinct" {
            assert_eq!(shift, 0, "n6 case must be in-window (shift == 0)");
        } else {
            assert_ne!(
                shift, 0,
                "subnormal/large case must have nonzero normalization_shift"
            );
        }

        // Using ONLY the exported hex fields, rebuild each column's adjoint bound.
        for k in 0..5 {
            let adj_col = &export.adjoint_columns[k];
            let abs_scale = f64_from_hex(&adj_col.abs_scale);
            let envelopes: Vec<f64> = adj_col.envelopes.iter().map(|s| f64_from_hex(s)).collect();
            let residuals: Vec<f64> = adj_col
                .local_residuals
                .iter()
                .map(|s| f64_from_hex(s))
                .collect();

            let m = export.identity.degree;
            let mut col_sum = 0.0;
            for j in 1..=m {
                let term = mul_up(envelopes[j], residuals[j - 1]).unwrap();
                col_sum = add_up(col_sum, term).unwrap();
            }
            let mut col_bound = mul_up(abs_scale, col_sum).unwrap();
            if shift != 0 {
                col_bound = scale_bound_up(col_bound, shift).unwrap();
            }

            let exported_adj = f64_from_hex(&export.columns[k].recurrence_adjoint);
            assert_eq!(
                col_bound.to_bits(),
                exported_adj.to_bits(),
                "case {label} column {k} bit-for-bit rebuild mismatch: {col_bound} vs {exported_adj}"
            );
        }

        // Verify upward assembly sum against assembly.total_upper.
        let fused_sum = f64_from_hex(&export.assembly.fused_summation);
        let mut upward_sum = fused_sum;
        for col in &export.columns {
            upward_sum = add_up(upward_sum, f64_from_hex(&col.truncation)).unwrap();
            upward_sum = add_up(upward_sum, f64_from_hex(&col.coefficient)).unwrap();
            upward_sum = add_up(upward_sum, f64_from_hex(&col.summation)).unwrap();
        }
        for col in &export.columns {
            upward_sum = add_up(upward_sum, f64_from_hex(&col.recurrence_adjoint)).unwrap();
        }
        for col in &export.columns {
            upward_sum = add_up(upward_sum, f64_from_hex(&col.normalization)).unwrap();
        }

        let total_upper = f64_from_hex(&export.assembly.total_upper);
        assert!(
            upward_sum >= total_upper,
            "case {label}: upward sum {upward_sum} must enclose total_upper {total_upper}"
        );
        if shift == 0 {
            assert_eq!(
                upward_sum.to_bits(),
                total_upper.to_bits(),
                "case {label}: in-window upward sum must match total_upper bit-for-bit"
            );
        } else {
            let rel_diff = (upward_sum - total_upper) / total_upper;
            assert!(
                rel_diff < 0.05,
                "case {label}: out-of-window upward sum {upward_sum} exceeds total_upper {total_upper} by {rel_diff}"
            );
        }
    }
}

#[test]
fn vg2_degree_budget_sweep_monotone_truncation() {
    let (a, w, h, rho, _, _) = load_published_case("n6-rho400-h0.1-distinct");
    let op = SymmetricNonpositiveOperator::new(a, 0.0, 1.2 * rho * (1.0 + 1e-12), "test").unwrap();

    let budgets = [1.0e-3, 1.0e-5, 1.0e-7, 1.0e-9, 1.0e-10];
    let mut prev_degree = 0;
    let mut prev_truncations = [f64::INFINITY; 5];

    for &budget in &budgets {
        let mut work = WorkCounters::default();
        let (_report, export) = joint_phi_action_laguerre_components(
            &op,
            h,
            JointPhiInput::Distinct(&w),
            budget,
            &mut work,
        )
        .unwrap();

        let m = export.identity.degree;
        assert!(
            m > prev_degree,
            "Degree must strictly increase as budget tightens: got {m} <= prev {prev_degree}"
        );
        prev_degree = m;

        for (k, prev_trunc) in prev_truncations.iter_mut().enumerate() {
            let trunc = f64_from_hex(&export.columns[k].truncation);
            assert!(
                trunc < *prev_trunc,
                "Per-column truncation bound must decrease monotonically with increasing degree: col {k} had {trunc} >= prev {prev_trunc}",
            );
            *prev_trunc = trunc;
        }
    }
}

// -----------------------------------------------------------------------------
// Exporter: Rebuilds exactly the 9 registered cases from cases.json
// -----------------------------------------------------------------------------

#[test]
#[ignore]
fn export_components() {
    let target_labels = [
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

    let scales_same = [1.0, -0.5, 0.25, 2.0, -1.0];
    let mut exported_cases = Vec::new();

    for &target_label in &target_labels {
        let (a, w, h, rho, trunc_budget, pub_case) = load_published_case(target_label);
        let n = a.nrows();

        let op =
            SymmetricNonpositiveOperator::new(a.clone(), 0.0, 1.2 * rho * (1.0 + 1e-12), "test")
                .unwrap();

        let is_same = target_label.ends_with("-same");
        let input = if is_same {
            JointPhiInput::SameVector {
                vector: &w[0],
                scales: scales_same,
            }
        } else {
            JointPhiInput::Distinct(&w)
        };

        let mut work = WorkCounters::default();
        let (report, export) =
            joint_phi_action_laguerre_components(&op, h, input, trunc_budget, &mut work).unwrap();

        // Verify exact bitwise parity with published case in cases.json.
        let pub_degree = pub_case["degree"].as_u64().unwrap() as usize;
        let pub_scale = pub_case["laguerre_scale"].as_f64().unwrap();
        let pub_shift = pub_case["normalization_shift"].as_i64().unwrap();
        let pub_fused: Vec<String> = pub_case["fused"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        let pub_total_f64 = pub_case["coverage"]["total"].as_f64().unwrap();

        assert_eq!(
            report.degree, pub_degree,
            "Degree mismatch on {target_label}"
        );
        assert_eq!(
            report.laguerre_scale.unwrap(),
            pub_scale,
            "Laguerre scale mismatch on {target_label}"
        );
        assert_eq!(
            report.normalization_shift, pub_shift,
            "Normalization shift mismatch on {target_label}"
        );

        let report_fused_hex: Vec<String> = report.fused.iter().map(|&x| hex(x)).collect();
        assert_eq!(
            report_fused_hex, pub_fused,
            "Fused vector bits mismatch on {target_label}"
        );

        let report_total_hex = hex(report.laguerre_adjoint_total.unwrap());
        let pub_total_hex = hex(pub_total_f64);
        assert_eq!(
            report_total_hex, pub_total_hex,
            "Laguerre adjoint total bits mismatch on {target_label}"
        );

        exported_cases.push(json!({
            "label": target_label,
            "n": n,
            "a": (0..n).map(|i| (0..n).map(|j| hex(a[(i, j)])).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "h": hex(h),
            "w": w.iter().map(|v| v.iter().map(|&x| hex(x)).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "fused": report_fused_hex,
            "degree": report.degree,
            "laguerre_scale": pub_scale,
            "normalization_shift": report.normalization_shift,
            "laguerre_adjoint_total": report_total_hex,
            "export": export,
        }));
    }

    assert_eq!(exported_cases.len(), 9);

    let output_rel = std::env::var("PY01_COMPONENTS").unwrap_or_else(|_| {
        "research/py01_laguerre_component_export_20261011/COMPONENTS.json".into()
    });
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let output_path = if Path::new(&output_rel).is_absolute() {
        std::path::PathBuf::from(output_rel)
    } else {
        repo_root.join(output_rel)
    };

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }

    let commit_out = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_root)
        .output()
        .expect("git rev-parse HEAD");
    let source_commit = String::from_utf8(commit_out.stdout)
        .expect("utf8")
        .trim()
        .to_string();
    let status_out = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_root)
        .output()
        .expect("git status --porcelain");
    let tree_clean = status_out.stdout.is_empty();

    let doc = json!({
        "schema": "vigilode-py01-laguerre-components-v1",
        "node": "research/py01_laguerre_component_export_20261011",
        "source_commit": source_commit,
        "tree_clean": tree_clean,
        "cases": exported_cases,
    });

    std::fs::write(
        &output_path,
        serde_json::to_string_pretty(&doc).unwrap() + "\n",
    )
    .expect("write COMPONENTS.json");
    println!("Exported 9 cases to {output_path:?}");
}
