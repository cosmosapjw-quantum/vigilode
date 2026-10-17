//! Contract test suite and stage-1 plan-only generator for Laguerre adjoint bound (PY02).

use std::{
    alloc::{GlobalAlloc, Layout, System},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

use rodas5p_core::{
    DenseMatrix, WorkCounters,
    directed::{Interval, add_up, mul_up},
    plan_joint_phi_local,
    polynomial_action::{
        JointPhiInput, PolynomialBasis, SymmetricNonpositiveOperator, joint_phi_action,
        scalar_phi_enclosure,
    },
};
use serde_json::json;

// -----------------------------------------------------------------------------
// Counting Allocator for VG-1
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

fn matrix(n: usize, rho: f64, seed: u64) -> DenseMatrix {
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

fn operator(a: DenseMatrix, rho: f64) -> SymmetricNonpositiveOperator {
    SymmetricNonpositiveOperator::new(a, 0.0, 1.2 * rho * (1.0 + 1.0e-12), "test").unwrap()
}

fn distinct(n: usize, seed: u64, amplitude: f64) -> [Vec<f64>; 5] {
    std::array::from_fn(|k| {
        splitmix(seed + 77 * k as u64, n)
            .iter()
            .map(|v| amplitude * v)
            .collect()
    })
}

fn same_vector(n: usize, seed: u64) -> ([Vec<f64>; 5], Vec<f64>, [f64; 5]) {
    let v = splitmix(seed, n);
    let scales = [1.0, -0.5, 0.25, 2.0, -1.0];
    let w = std::array::from_fn(|k| v.iter().map(|x| scales[k] * x).collect());
    (w, v, scales)
}

/// Jacobi eigenvalue decomposition of real symmetric matrix A.
/// Returns (eigenvalues, eigenvectors V) where A = V * diag(eigenvalues) * V^T.
fn jacobi_evd(mut a: DenseMatrix) -> (Vec<f64>, DenseMatrix) {
    let n = a.nrows();
    let mut v = DenseMatrix::zeros(n, n);
    for i in 0..n {
        v[(i, i)] = 1.0;
    }
    for _ in 0..(100 * n * n) {
        let mut max_off = 0.0;
        let mut p = 0;
        let mut q = 1;
        for i in 0..n {
            for j in (i + 1)..n {
                let off = a[(i, j)].abs();
                if off > max_off {
                    max_off = off;
                    p = i;
                    q = j;
                }
            }
        }
        if max_off < 1.0e-15 {
            break;
        }
        let app = a[(p, p)];
        let aqq = a[(q, q)];
        let apq = a[(p, q)];
        let theta = 0.5 * (aqq - app) / apq;
        let t = if theta >= 0.0 {
            1.0 / (theta + (theta * theta + 1.0).sqrt())
        } else {
            -1.0 / (-theta + (theta * theta + 1.0).sqrt())
        };
        let c = 1.0 / (t * t + 1.0).sqrt();
        let s = t * c;
        let tau = s / (1.0 + c);

        a[(p, p)] -= t * apq;
        a[(q, q)] += t * apq;
        a[(p, q)] = 0.0;
        a[(q, p)] = 0.0;

        for j in 0..n {
            if j != p && j != q {
                let ajp = a[(j, p)];
                let ajq = a[(j, q)];
                a[(j, p)] = ajp - s * (ajq + tau * ajp);
                a[(p, j)] = a[(j, p)];
                a[(j, q)] = ajq + s * (ajp - tau * ajq);
                a[(q, j)] = a[(j, q)];
            }
        }
        for j in 0..n {
            let vjp = v[(j, p)];
            let vjq = v[(j, q)];
            v[(j, p)] = vjp - s * (vjq + tau * vjp);
            v[(j, q)] = vjq + s * (vjp - tau * vjq);
        }
    }
    let eigenvalues = (0..n).map(|i| a[(i, i)]).collect();
    (eigenvalues, v)
}

/// Compute exact reference F = sum_{k=0..4} phi_k(h A) w_k using Jacobi EVD and certified scalar phi.
fn compute_reference_f(a: &DenseMatrix, h: f64, w: &[Vec<f64>; 5]) -> (Vec<f64>, f64) {
    let n = a.nrows();
    let (evals, v) = jacobi_evd(a.clone());

    let mut f_total = vec![0.0; n];
    for k in 0..5 {
        let mut wk_trans = vec![0.0; n];
        for i in 0..n {
            let mut dot = 0.0;
            for j in 0..n {
                dot += v[(j, i)] * w[k][j];
            }
            wk_trans[i] = dot;
        }

        let mut yk_trans = vec![0.0; n];
        for i in 0..n {
            let z = h * evals[i];
            let phi_k = if z > 0.0 {
                // Nonpositive operators have nonpositive eigenvalues
                1.0
            } else {
                let enc = scalar_phi_enclosure(Interval::point(z).unwrap()).unwrap();
                0.5 * (enc[k].lo + enc[k].hi)
            };
            yk_trans[i] = phi_k * wk_trans[i];
        }

        for i in 0..n {
            let mut dot = 0.0;
            for j in 0..n {
                dot += v[(i, j)] * yk_trans[j];
            }
            f_total[i] += dot;
        }
    }

    let norm_f = f_total.iter().map(|x| x * x).sum::<f64>().sqrt();
    (f_total, norm_f)
}

// -----------------------------------------------------------------------------
// Scientific Gate Tests
// -----------------------------------------------------------------------------

#[test]
fn vg1_zero_allocation_in_transform_selection_primitives() {
    // Verify that directed rounding and scalar math primitives allocate zero heap memory.
    let (_, allocs1, bytes1) = count_allocations(|| {
        let mut x = 1.0;
        for _ in 0..100 {
            x = add_up(x, 0.01).unwrap();
            x = mul_up(x, 1.001).unwrap();
        }
        x
    });
    assert_eq!(allocs1, 0, "Directed f64 primitives allocated memory");
    assert_eq!(bytes1, 0);

    let i1 = Interval::point(1.2345).unwrap();
    let i2 = Interval::point(6.7890).unwrap();
    let (_, allocs2, bytes2) = count_allocations(|| {
        let mut val = i1;
        for _ in 0..100 {
            val = val.add(i2).unwrap();
            val = val.mul(i1).unwrap();
        }
        val
    });
    assert_eq!(allocs2, 0, "Interval arithmetic allocated memory");
    assert_eq!(bytes2, 0);
}

#[test]
fn vg2_monotone_degree_and_truncation_bound_with_tightening_budget() {
    let n = 7;
    let rho = 100.0;
    let a = matrix(n, rho, 9001);
    let op = operator(a, rho);
    let h = 0.08;
    let w = distinct(n, 9001 + 9, 1.0);

    let budgets = [1.0e-3, 1.0e-5, 1.0e-7, 1.0e-9, 1.0e-11];

    for basis in [PolynomialBasis::Chebyshev, PolynomialBasis::Laguerre] {
        let mut prev_degree = 0;
        for &budget in &budgets {
            let plan =
                plan_joint_phi_local(&op, h, JointPhiInput::Distinct(&w), basis, budget).unwrap();
            assert!(
                plan.degree >= prev_degree,
                "Degree must be monotonically non-decreasing as budget tightens for {:?}: got {} < prev {}",
                basis,
                plan.degree,
                prev_degree
            );
            assert!(
                plan.truncation_bound <= budget * (1.0 + 1.0e-12),
                "Truncation bound {} must be <= budget {} for {:?}",
                plan.truncation_bound,
                budget,
                basis
            );
            prev_degree = plan.degree;
        }
    }
}

#[test]
fn vg3_plan_local_matches_joint_phi_action_degree_and_scale_bitwise() {
    let n = 4;
    let rho = 40.0;
    let a = matrix(n, rho, 12345);
    let op = operator(a, rho);
    let h = 0.05;
    let w = distinct(n, 12345 + 9, 1.0);
    let budget = 1.0e-8;

    for basis in [PolynomialBasis::Chebyshev, PolynomialBasis::Laguerre] {
        let plan =
            plan_joint_phi_local(&op, h, JointPhiInput::Distinct(&w), basis, budget).unwrap();

        let mut work = WorkCounters::default();
        let report = joint_phi_action(
            &op,
            h,
            JointPhiInput::Distinct(&w),
            basis,
            budget,
            None,
            &mut work,
        )
        .unwrap();

        assert_eq!(
            plan.degree, report.degree,
            "degree mismatch for {:?}",
            basis
        );
        assert_eq!(
            plan.laguerre_scale, report.laguerre_scale,
            "laguerre_scale mismatch for {:?}",
            basis
        );
        assert_eq!(
            plan.normalization_shift as i64, report.normalization_shift,
            "normalization_shift mismatch for {:?}",
            basis
        );
        assert_eq!(
            plan.truncation_bound.to_bits(),
            report.truncation_bound_exact_arithmetic.to_bits(),
            "truncation_bound mismatch for {:?}",
            basis
        );
    }
}

#[test]
fn refusal_contracts() {
    let n = 4;
    let rho = 40.0;
    let a = matrix(n, rho, 12345);
    let op = operator(a, rho);
    let h = 0.05;
    let w = distinct(n, 12345 + 9, 1.0);
    let budget = 1.0e-8;

    // Invalid h < 0
    assert!(
        plan_joint_phi_local(
            &op,
            -0.05,
            JointPhiInput::Distinct(&w),
            PolynomialBasis::Chebyshev,
            budget,
        )
        .is_err()
    );

    // Invalid budget <= 0 or NaN
    assert!(
        plan_joint_phi_local(
            &op,
            h,
            JointPhiInput::Distinct(&w),
            PolynomialBasis::Chebyshev,
            0.0,
        )
        .is_err()
    );
    assert!(
        plan_joint_phi_local(
            &op,
            h,
            JointPhiInput::Distinct(&w),
            PolynomialBasis::Chebyshev,
            -1.0,
        )
        .is_err()
    );
    assert!(
        plan_joint_phi_local(
            &op,
            h,
            JointPhiInput::Distinct(&w),
            PolynomialBasis::Chebyshev,
            f64::NAN,
        )
        .is_err()
    );

    // Dimension mismatch
    let wrong_w = distinct(n + 1, 12345 + 9, 1.0);
    assert!(
        plan_joint_phi_local(
            &op,
            h,
            JointPhiInput::Distinct(&wrong_w),
            PolynomialBasis::Chebyshev,
            budget,
        )
        .is_err()
    );

    // Degree > 128 refusal for Laguerre
    let huge_a = matrix(n, 5000.0, 9999);
    let huge_op = operator(huge_a, 5000.0);
    let huge_w = distinct(n, 9999 + 9, 1.0);
    let res = plan_joint_phi_local(
        &huge_op,
        0.5,
        JointPhiInput::Distinct(&huge_w),
        PolynomialBasis::Laguerre,
        1.0e-15,
    );
    assert!(res.is_err(), "Must refuse Laguerre degree > 128");
}

// -----------------------------------------------------------------------------
// Stage-1 Plan-Only Exporter
// -----------------------------------------------------------------------------

#[test]
#[ignore]
fn plan_only() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    let output_rel = std::env::var("PY02_PLANS")
        .unwrap_or_else(|_| "research/py02_laguerre_adjoint_bound_20261011/PLANS.json".into());
    let output_path = if Path::new(&output_rel).is_absolute() {
        std::path::PathBuf::from(output_rel)
    } else {
        repo_root.join(output_rel)
    };

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
    let status_str = String::from_utf8_lossy(&status_out.stdout);
    let tree_clean = status_str.lines().all(|l| {
        let t = l.trim();
        t.is_empty() || t.ends_with("PLANS.json") || t.ends_with("ADMISSION.json")
    });

    let mut cases = Vec::new();
    let mut seed = 9000;

    for n in [7, 14] {
        for rho in [100.0, 300.0] {
            for h in [0.08, 0.16] {
                seed += 1;
                let a = matrix(n, rho, seed);
                let op = operator(a.clone(), rho);

                for same in [false, true] {
                    let (w, v, scales) = if same {
                        same_vector(n, seed + 5)
                    } else {
                        (distinct(n, seed + 9, 1.0), Vec::new(), [0.0; 5])
                    };
                    let input = if same {
                        JointPhiInput::SameVector { vector: &v, scales }
                    } else {
                        JointPhiInput::Distinct(&w)
                    };

                    let (_f_ref, norm_f) = compute_reference_f(&a, h, &w);

                    for budget_factor in [1.0e-6, 1.0e-8] {
                        let total_budget = budget_factor * norm_f.max(1.0);
                        let truncation_budget = 0.25 * total_budget;

                        let cheb_plan = plan_joint_phi_local(
                            &op,
                            h,
                            input,
                            PolynomialBasis::Chebyshev,
                            truncation_budget,
                        )
                        .expect("Chebyshev plan");

                        let lag_plan = plan_joint_phi_local(
                            &op,
                            h,
                            input,
                            PolynomialBasis::Laguerre,
                            truncation_budget,
                        )
                        .expect("Laguerre plan");

                        let label = format!(
                            "n{n}-rho{:.0}-h{h}-{}-{:.0e}",
                            rho,
                            if same { "same" } else { "distinct" },
                            budget_factor
                        );

                        let a_hex: Vec<Vec<String>> = (0..n)
                            .map(|i| (0..n).map(|j| hex(a[(i, j)])).collect())
                            .collect();
                        let w_hex: Vec<Vec<String>> = w
                            .iter()
                            .map(|col| col.iter().map(|&x| hex(x)).collect())
                            .collect();

                        cases.push(json!({
                            "label": label,
                            "n": n,
                            "rho": rho,
                            "rho_bits": hex(rho),
                            "h": h,
                            "h_bits": hex(h),
                            "seed": seed,
                            "same": same,
                            "budget_factor": budget_factor,
                            "norm_f": norm_f,
                            "norm_f_bits": hex(norm_f),
                            "total_budget": total_budget,
                            "total_budget_bits": hex(total_budget),
                            "truncation_budget": truncation_budget,
                            "truncation_budget_bits": hex(truncation_budget),
                            "a": a_hex,
                            "w": w_hex,
                            "chebyshev_plan": cheb_plan,
                            "laguerre_plan": lag_plan,
                        }));
                    }
                }
            }
        }
    }

    assert_eq!(cases.len(), 32, "Must generate exactly 32 stage-2 cases");

    let doc = json!({
        "schema": "vigilode-py02-laguerre-plans-v1",
        "node": "research/py02_laguerre_adjoint_bound_20261011",
        "source_commit": source_commit,
        "tree_clean": tree_clean,
        "cases": cases,
    });

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(
        &output_path,
        serde_json::to_string_pretty(&doc).unwrap() + "\n",
    )
    .expect("write PLANS.json");

    println!("Wrote 32 plans to {output_path:?}");
}
