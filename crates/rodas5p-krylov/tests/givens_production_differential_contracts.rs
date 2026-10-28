//! WU-2 (audit F-036): `solve_gmres_givens` is a research candidate with no
//! production caller; production dispatch uses `solve_gmres`.  The two kernels
//! reach different Ok/Err outcomes on the same input.  This differential test
//! pins that difference on the audit's E-05 nonnormal operator so a change to
//! either kernel's success semantics is visible, and checks that every Ok
//! result satisfies the same recomputed true-residual threshold.

use std::{fs::File, io::Write, path::PathBuf};

use rodas5p_core::{
    DenseMatrix, DenseOperator, ExactPreconditionerIdentity, JacobiPreconditioner,
    LinearSolveReport, Preconditioner, WorkCounters, safe_l2,
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

/// Independent row accumulation and norm: do not call the operator, its matvec,
/// or use the report's residual. The exported exact inputs additionally permit
/// a high-precision oracle outside this binary64 diagnostic.
fn true_residual(a: &DenseMatrix, b: &[f64], x: &[f64]) -> f64 {
    assert_eq!(a.nrows(), b.len());
    assert_eq!(a.ncols(), x.len());
    let mut norm = 0.0_f64;
    for (row, right) in b.iter().enumerate() {
        let mut sum = 0.0;
        let mut correction = 0.0;
        for (column, value) in x.iter().enumerate() {
            let product = a[(row, column)] * value;
            let next = sum + product;
            correction += if sum.abs() >= product.abs() {
                (sum - next) + product
            } else {
                (product - next) + sum
            };
            sum = next;
        }
        norm = norm.hypot((right - sum) - correction);
    }
    norm
}

fn check_true_residual(a: &DenseMatrix, b: &[f64], report: &LinearSolveReport, threshold: f64) {
    let residual = true_residual(a, b, &report.x);
    assert!(
        residual.is_finite() && residual <= threshold,
        "independent true residual {residual:e} exceeds {threshold:e}"
    );
    assert!(report.converged);
}

/// Optional create-only export. Each hexadecimal word is the exact binary64
/// payload, never a decimal reconstruction. The caller records build/profile
/// identity next to this fixture and performs the separate precision check.
fn oracle_directory() -> Option<PathBuf> {
    std::env::var_os("Q_GIVENS_ORACLE_DIR").map(PathBuf::from)
}

fn write_bits(file: &mut File, name: &str, values: &[f64]) {
    write!(file, "{name} {}", values.len()).unwrap();
    for value in values {
        write!(file, " {:016x}", value.to_bits()).unwrap();
    }
    writeln!(file).unwrap();
}

fn export_fixture(
    a: &DenseMatrix,
    b_unit: &[f64],
    pc: &JacobiPreconditioner,
    config: &GmresConfig,
) {
    if let Some(directory) = oracle_directory() {
        let mut file = File::create_new(directory.join("fixture.bits")).unwrap();
        writeln!(file, "q-givens-exact-bits-v1").unwrap();
        writeln!(file, "dimension {}", a.nrows()).unwrap();
        writeln!(file, "restart {}", config.restart).unwrap();
        writeln!(file, "max_arnoldi {}", config.max_arnoldi).unwrap();
        write_bits(&mut file, "rtol", &[config.rtol]);
        write_bits(&mut file, "atol", &[config.atol]);
        write_bits(&mut file, "a_row_major", a.as_slice());
        write_bits(&mut file, "b_unit", b_unit);
        let Some(ExactPreconditionerIdentity::Jacobi {
            inverse_diagonal_bits,
        }) = pc.exact_identity()
        else {
            panic!("the fixture must retain the actual Jacobi inverse diagonal");
        };
        let inverse: Vec<_> = inverse_diagonal_bits
            .into_iter()
            .map(f64::from_bits)
            .collect();
        write_bits(&mut file, "jacobi_inverse_diagonal", &inverse);
    }
}

fn export_result(
    label: &str,
    scale: f64,
    b: &[f64],
    threshold: f64,
    result: &Result<LinearSolveReport, rodas5p_core::CoreError>,
) {
    if let Some(directory) = oracle_directory() {
        let mut file = File::create_new(directory.join(format!("{label}-{scale:e}.bits"))).unwrap();
        writeln!(file, "q-givens-result-bits-v1").unwrap();
        write_bits(&mut file, "scale", &[scale]);
        write_bits(&mut file, "b", b);
        write_bits(&mut file, "binary64_threshold", &[threshold]);
        match result {
            Ok(report) => {
                writeln!(file, "status ok").unwrap();
                writeln!(file, "iterations {}", report.iterations).unwrap();
                write_bits(&mut file, "x", &report.x);
                write_bits(&mut file, "reported_residual", &[report.residual_norm]);
            }
            Err(error) => writeln!(file, "status error {error}").unwrap(),
        }
    }
}

fn run_exact_oracle(scale: f64) {
    if let Some(script) = std::env::var_os("Q_GIVENS_ORACLE_SCRIPT") {
        let directory = oracle_directory().expect("the exact oracle requires Q_GIVENS_ORACLE_DIR");
        let status = std::process::Command::new("python3")
            .arg(script)
            .arg(directory.join("fixture.bits"))
            .arg(directory.join(format!("gmres-{scale:e}.bits")))
            .arg(directory.join(format!("gmres-givens-candidate-{scale:e}.bits")))
            .arg("--output")
            .arg(directory.join(format!("oracle-{scale:e}.json")))
            .status()
            .expect("start independent exact-rational residual oracle");
        assert!(status.success(), "independent exact residual oracle failed");
    }
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

    export_fixture(&a, &b_unit, &pc, &config);

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
        export_result("gmres", scale, &b, threshold, &production);
        export_result("gmres-givens-candidate", scale, &b, threshold, &candidate);
        run_exact_oracle(scale);
        // Check both returned solutions before any historical status assertion.
        for report in [&production, &candidate].into_iter().flatten() {
            check_true_residual(&a, &b, report, threshold);
        }
        let check = |label: &str, result: &Result<LinearSolveReport, _>, expected_ok: bool| {
            assert_eq!(
                result.is_ok(),
                expected_ok,
                "{label} at RHS scale {scale:e}: {:?}",
                result.as_ref().map(|report| report.iterations)
            );
        };
        check("gmres", &production, production_ok);
        check("gmres-givens-candidate", &candidate, candidate_ok);
    }
}

#[test]
#[should_panic(expected = "independent true residual")]
fn a_corrupted_ok_solution_cannot_certify_itself_with_its_reported_residual() {
    let a = DenseMatrix::identity(2);
    let report = LinearSolveReport {
        x: vec![1.0, 9.0],
        converged: true,
        info: 0,
        residual_norm: 0.0,
        relative_residual: 0.0,
        iterations: 1,
        matvecs: 1,
        preconditioner_apps: 0,
        method: "corrupted-negative-control".into(),
    };
    check_true_residual(&a, &[1.0, 2.0], &report, 1.0e-10);
}
