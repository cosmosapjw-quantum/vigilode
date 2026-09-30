//! WU-2 (audit F-036): `solve_gmres_givens` is a research candidate with no
//! production caller; production dispatch uses `solve_gmres`.  The two kernels
//! reach different Ok/Err outcomes on the same input.  This differential test
//! pins that difference on the audit's E-05 nonnormal operator so a change to
//! either kernel's success semantics is visible, and checks that every Ok
//! result satisfies the same recomputed true-residual threshold.

use rodas5p_core::{
    DenseMatrix, DenseOperator, JacobiPreconditioner, LinearSolveReport, WorkCounters, safe_l2,
};
use rodas5p_krylov::{GmresConfig, solve_gmres, solve_gmres_givens};

/// The E-05 harness generator (`harness/src/bin/e05_krylov_stress.rs`).
struct Lcg(u64);

impl Lcg {
    fn next_f64(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }

    fn uniform(&mut self) -> f64 {
        2.0 * self.next_f64() - 1.0
    }
}

/// Orthogonal Q as the explicit product of Householder reflectors, as in E-05.
fn random_orthogonal(n: usize, rng: &mut Lcg) -> DenseMatrix {
    let mut q = DenseMatrix::identity(n);
    for k in 0..n {
        let mut v: Vec<f64> = (0..n)
            .map(|i| if i < k { 0.0 } else { rng.uniform() })
            .collect();
        let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm == 0.0 {
            continue;
        }
        for x in &mut v {
            *x /= norm;
        }
        let mut qv = vec![0.0; n];
        for (i, value) in qv.iter_mut().enumerate() {
            for (j, vj) in v.iter().enumerate() {
                *value += q[(i, j)] * vj;
            }
        }
        for (i, qvi) in qv.iter().enumerate() {
            for (j, vj) in v.iter().enumerate() {
                q[(i, j)] -= 2.0 * qvi * vj;
            }
        }
    }
    q
}

/// `A = Q (D + s N) Q^T` with `D` log-spaced in `[-dmax, -1]`, as in E-05.
fn build_a(n: usize, s: f64, dmax: f64, rng: &mut Lcg) -> DenseMatrix {
    let q = random_orthogonal(n, rng);
    let mut inner = DenseMatrix::zeros(n, n);
    for i in 0..n {
        inner[(i, i)] = -(10f64).powf((i as f64) / ((n - 1) as f64) * dmax.log10());
        for j in (i + 1)..n {
            inner[(i, j)] = s * rng.uniform();
        }
    }
    q.matmul(&inner).unwrap().matmul(&q.transpose()).unwrap()
}

fn true_residual(a: &DenseMatrix, b: &[f64], x: &[f64]) -> f64 {
    let ax = a.matvec(x).unwrap();
    safe_l2(
        &b.iter()
            .zip(&ax)
            .map(|(bi, yi)| bi - yi)
            .collect::<Vec<_>>(),
    )
}

#[test]
fn candidate_and_production_outcomes_are_pinned_on_the_e05_nonnormal_jacobi_rows() {
    let n = 256;
    let rtol = 1.0e-10;
    let mut rng = Lcg(20_260_927);
    let b_unit: Vec<f64> = {
        let v: Vec<f64> = (0..n).map(|_| rng.uniform()).collect();
        let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        v.into_iter().map(|x| x / norm).collect()
    };
    // Bit-level anchor to the committed E-05 raw data (`e05_raw.json`, `b_unit`).
    assert_eq!(b_unit[0], -0.031_981_700_657_820_25);
    assert_eq!(b_unit[1], 0.095_957_793_725_194_96);
    assert_eq!(b_unit[2], -0.040_942_056_514_667_216);
    // E-05 builds s = 0 first; its draws precede the s = 1 operator.
    let _s0 = build_a(n, 0.0, 1.0e4, &mut rng);
    let a = build_a(n, 1.0, 1.0e4, &mut rng);
    let op = DenseOperator::new(a.clone()).unwrap();
    let pc = JacobiPreconditioner::from_matrix(&a).unwrap();
    let config = GmresConfig {
        restart: 40,
        max_arnoldi: 4000,
        rtol,
        atol: 0.0,
    };

    // (RHS scale, production Ok?, candidate Ok?) observed in E-05 at b3e8165.
    let pinned = [
        (1.0e-14, true, false),
        (1.0, false, true),
        (1.0e14, true, false),
    ];
    for (scale, production_ok, candidate_ok) in pinned {
        let b: Vec<f64> = b_unit.iter().map(|x| x * scale).collect();
        let threshold = rtol * safe_l2(&b);
        let mut work = WorkCounters::default();
        let production = solve_gmres(&op, &pc, &b, None, &config, &mut work);
        let mut work = WorkCounters::default();
        let candidate = solve_gmres_givens(&op, &pc, &b, None, &config, &mut work);
        let check = |label: &str, result: &Result<LinearSolveReport, _>, expected_ok: bool| {
            assert_eq!(
                result.is_ok(),
                expected_ok,
                "{label} at RHS scale {scale:e}: {:?}",
                result.as_ref().map(|report| report.iterations)
            );
            if let Ok(report) = result {
                assert!(report.converged);
                assert!(true_residual(&a, &b, &report.x) <= threshold);
            }
        };
        check("gmres", &production, production_ok);
        check("gmres-givens-candidate", &candidate, candidate_ok);
    }
}
