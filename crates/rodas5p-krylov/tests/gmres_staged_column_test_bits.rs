//! Bits of the kernels that loop node V24 (owner decision OD-L1-11) must not
//! change, recorded BEFORE `gmres_staged.rs` was edited for that node.
//!
//! * The shipped restart policy of the matrix-free stage solve is
//!   [`solve_gmres_into_with_accounting`]: `restart` columns per cycle, the
//!   true residual tested once per cycle. V24 adds an adaptive policy next to
//!   it; the shipped policy must return the bits of the code before the edit.
//! * [`solve_staged_gmres`] lives in the edited file and serves the staged
//!   research arms; it must keep its bits as well.
//!
//! Every expected line below was printed by this file at the base commit of
//! the node (`V24_PRINT_BITS=1 cargo test ... -- --nocapture`), before any
//! edit of `gmres_staged.rs`.

use rodas5p_core::{DenseMatrix, DenseOperator, IdentityPreconditioner, WorkCounters};
use rodas5p_krylov::{
    GmresCapacity, GmresConfig, GmresIntoOptions, GmresWorkspace, ResidualAccounting,
    StagedGmresConfig, StagedGmresWorkspace, solve_gmres_into_with_accounting, solve_staged_gmres,
};

/// FNV-1a over the little-endian bytes of the values.
fn fingerprint(values: &[f64]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for value in values {
        for byte in value.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

/// One system `A x = b` with an optional start and its GMRES limits.
struct Case {
    name: &'static str,
    a: DenseMatrix,
    b: Vec<f64>,
    x0: Option<Vec<f64>>,
    restart: usize,
    budget: usize,
    rtol: f64,
    atol: f64,
}

fn tridiagonal(n: usize, sub: f64, diag: f64, sup: f64) -> DenseMatrix {
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        a[(i, i)] = diag;
        if i > 0 {
            a[(i, i - 1)] = sub;
        }
        if i + 1 < n {
            a[(i, i + 1)] = sup;
        }
    }
    a
}

/// Nonsymmetric, with structure in every row.
fn scattered(n: usize) -> DenseMatrix {
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        a[(i, i)] = 4.0 + 0.1 * i as f64;
        if i > 0 {
            a[(i, i - 1)] = -1.3;
        }
        if i + 1 < n {
            a[(i, i + 1)] = 0.7;
        }
        a[(i, (i * 7 + 3) % n)] += 0.2;
    }
    a
}

/// `I - s J` with the 1D diffusion-advection stencil `J`: the shape of a
/// stage system of the matrix-free driver.
fn shifted_stencil(n: usize, shift: f64) -> DenseMatrix {
    let mut a = DenseMatrix::identity(n);
    for i in 0..n {
        a[(i, i)] += shift * 2.0;
        if i > 0 {
            a[(i, i - 1)] -= shift * 1.25;
        }
        if i + 1 < n {
            a[(i, i + 1)] -= shift * 0.75;
        }
    }
    a
}

fn cases() -> Vec<Case> {
    let wave =
        |n: usize| -> Vec<f64> { (0..n).map(|i| ((i * 37 % 11) as f64 - 5.0) / 3.0).collect() };
    vec![
        Case {
            name: "tridiagonal-40",
            a: tridiagonal(40, -1.0, 10.0, -2.0),
            b: (0..40).map(|i| 1.0 + 0.1 * i as f64).collect(),
            x0: None,
            restart: 40,
            budget: 200,
            rtol: 1.0e-10,
            atol: 0.0,
        },
        Case {
            name: "scattered-60-restart-10",
            a: scattered(60),
            b: wave(60),
            x0: None,
            restart: 10,
            budget: 200,
            rtol: 1.0e-11,
            atol: 1.0e-14,
        },
        Case {
            name: "scattered-60-restart-10-start",
            a: scattered(60),
            b: wave(60),
            x0: Some((0..60).map(|i| 0.01 * ((i % 5) as f64 - 2.0)).collect()),
            restart: 10,
            budget: 200,
            rtol: 1.0e-11,
            atol: 1.0e-14,
        },
        Case {
            name: "shifted-stencil-90-restart-40",
            a: shifted_stencil(90, 4.0),
            b: (0..90).map(|i| (0.3 * i as f64).sin() + 0.2).collect(),
            x0: None,
            restart: 40,
            budget: 200,
            rtol: 1.0e-10,
            atol: 2.0e-15,
        },
        // Too stiff for the budget: the solve fails.
        Case {
            name: "shifted-stencil-90-stiff",
            a: shifted_stencil(90, 60.0),
            b: (0..90).map(|i| (0.3 * i as f64).sin() + 0.2).collect(),
            x0: None,
            restart: 40,
            budget: 200,
            rtol: 1.0e-10,
            atol: 2.0e-15,
        },
    ]
}

fn counters_line(counters: &WorkCounters) -> String {
    format!(
        "mv={} dg={} li={} ip={} vu={} pa={} ls={}",
        counters.linear_matvecs,
        counters.diagnostic_matvecs,
        counters.linear_iterations,
        counters.orthogonalization_inner_products,
        counters.orthogonalization_vector_updates,
        counters.preconditioner_apps,
        counters.linear_solves
    )
}

/// The shipped kernel on `case` under `accounting`: one line of bits.
fn shipped_line(case: &Case, accounting: ResidualAccounting) -> String {
    let n = case.b.len();
    let op = DenseOperator::new(case.a.clone()).unwrap();
    let pc = IdentityPreconditioner::new(n);
    let config = GmresConfig {
        restart: case.restart,
        max_arnoldi: case.budget,
        rtol: case.rtol,
        atol: case.atol,
    };
    let mut x = vec![f64::NAN; n];
    let mut counters = WorkCounters::default();
    let outcome = solve_gmres_into_with_accounting(
        &op,
        &pc,
        &case.b,
        case.x0.as_deref(),
        &config,
        None,
        &mut x,
        &mut GmresWorkspace::default(),
        GmresCapacity::unbounded(),
        GmresIntoOptions::default(),
        accounting,
        &mut counters,
    );
    let report = match outcome {
        Ok(report) => report,
        // A failed solve leaves the output untouched.
        Err(error) => {
            assert!(x.iter().all(|value| value.is_nan()));
            return format!("err=[{error}] {}", counters_line(&counters));
        }
    };
    format!(
        "x={:016x} it={} cy={} res={:016x} rel={:016x} ini={:016x} {}",
        fingerprint(&x),
        report.iterations,
        report.cycles,
        report.residual_norm.to_bits(),
        report.relative_residual.to_bits(),
        report.initial_residual_norm.to_bits(),
        counters_line(&counters)
    )
}

/// The staged solver on `case` (zero start): one line of bits.
fn staged_line(case: &Case, guards: bool) -> String {
    let n = case.b.len();
    let op = DenseOperator::new(case.a.clone()).unwrap();
    let mut config = StagedGmresConfig::new(case.restart, case.budget, case.rtol, case.atol);
    if guards {
        config.stall_rule = true;
        config.nu_guard = true;
        config.floor_at_confirmations = true;
    }
    let mut x = vec![f64::NAN; n];
    let mut counters = WorkCounters::default();
    let report = solve_staged_gmres(
        &op,
        &case.b,
        &config,
        None,
        &mut x,
        &mut StagedGmresWorkspace::default(),
        &mut counters,
    )
    .unwrap();
    format!(
        "x={:016x} out={:?} col={} cy={} tr={} cf={} fc={} res={:016x} prj={:016x} thr={:016x} {}",
        fingerprint(&x),
        report.outcome,
        report.columns,
        report.cycles,
        report.true_residuals,
        report.confirmations,
        report.failed_confirmations,
        report.residual_norm.to_bits(),
        report.projected_residual.to_bits(),
        report.final_threshold.to_bits(),
        counters_line(&counters)
    )
}

fn check(kind: &str, name: &str, line: &str, expected: &[(&str, &str)]) {
    if std::env::var_os("V24_PRINT_BITS").is_some() {
        println!("V24_BITS {kind} (\"{name}\", \"{line}\"),");
        return;
    }
    let want = expected
        .iter()
        .find(|(case, _)| *case == name)
        .unwrap_or_else(|| panic!("{kind}: no recorded bits for {name}"))
        .1;
    assert_eq!(line, want, "{kind} {name}: the bits changed");
}

/// Printed at the base commit 892132e5 (before any edit of the node).
const SHIPPED_RECOMPUTE_FINAL: [(&str, &str); 5] = [
    (
        "tridiagonal-40",
        "x=748cf679ac4eb2aa it=40 cy=1 res=3d1205548b4ba5f2 rel=3cccc871e40344b5 ini=403408f3c1a7aaec mv=41 dg=1 li=40 ip=1640 vu=1641 pa=41 ls=1",
    ),
    (
        "scattered-60-restart-10",
        "x=e0b750ed37f80863 it=30 cy=3 res=3d9a73db81bee2ce rel=3d69e5840e925d12 ini=402057f1dbf8cb80 mv=33 dg=1 li=30 ip=330 vu=333 pa=33 ls=1",
    ),
    (
        "scattered-60-restart-10-start",
        "x=f892f943d649f616 it=30 cy=3 res=3d98e9d58630d863 rel=3d6863c651a8b9bf ini=4020622ca75987b1 mv=34 dg=1 li=30 ip=330 vu=333 pa=33 ls=1",
    ),
    (
        "shifted-stencil-90-restart-40",
        "x=f0dfe67700683496 it=80 cy=2 res=3db75a25ffea96b5 rel=3d8a63e106d31184 ini=401c50f1cab30160 mv=82 dg=1 li=80 ip=3280 vu=3282 pa=82 ls=1",
    ),
    (
        "shifted-stencil-90-stiff",
        "err=[linear solve failed: GMRES exhausted 200 Arnoldi vectors] mv=205 dg=0 li=200 ip=8200 vu=8205 pa=205 ls=0",
    ),
];

const SHIPPED_REUSE_CONFIRMED: [(&str, &str); 5] = [
    (
        "tridiagonal-40",
        "x=748cf679ac4eb2aa it=40 cy=1 res=3d1205548b4ba5f2 rel=3cccc871e40344b5 ini=403408f3c1a7aaec mv=41 dg=0 li=40 ip=1640 vu=1641 pa=41 ls=1",
    ),
    (
        "scattered-60-restart-10",
        "x=e0b750ed37f80863 it=30 cy=3 res=3d9a73db81bee2ce rel=3d69e5840e925d12 ini=402057f1dbf8cb80 mv=33 dg=0 li=30 ip=330 vu=333 pa=33 ls=1",
    ),
    (
        "scattered-60-restart-10-start",
        "x=f892f943d649f616 it=30 cy=3 res=3d98e9d58630d863 rel=3d6863c651a8b9bf ini=4020622ca75987b1 mv=34 dg=0 li=30 ip=330 vu=333 pa=33 ls=1",
    ),
    (
        "shifted-stencil-90-restart-40",
        "x=f0dfe67700683496 it=80 cy=2 res=3db75a25ffea96b5 rel=3d8a63e106d31184 ini=401c50f1cab30160 mv=82 dg=0 li=80 ip=3280 vu=3282 pa=82 ls=1",
    ),
    (
        "shifted-stencil-90-stiff",
        "err=[linear solve failed: GMRES exhausted 200 Arnoldi vectors] mv=205 dg=0 li=200 ip=8200 vu=8205 pa=205 ls=0",
    ),
];

const STAGED_PLAIN: [(&str, &str); 4] = [
    (
        "tridiagonal-40",
        "x=a18f243bb85a8430 out=Converged col=14 cy=1 tr=1 cf=1 fc=0 res=3e1831d2b8ef938f prj=3e1831d48903be5d thr=3e2135bc82ba575f mv=15 dg=0 li=14 ip=210 vu=211 pa=0 ls=1",
    ),
    (
        "scattered-60-restart-10",
        "x=099c20385d3e09c9 out=Converged col=28 cy=3 tr=3 cf=1 fc=0 res=3dc47623bffbc1ab prj=3dc47622f3da5965 thr=3dd6765eb17d5429 mv=31 dg=0 li=28 ip=292 vu=295 pa=0 ls=1",
    ),
    (
        "shifted-stencil-90-restart-40",
        "x=6646bb022611c76c out=Converged col=69 cy=2 tr=2 cf=1 fc=0 res=3e07a00914dd6e3d prj=3e07a009139dcfbb thr=3e0852cbc0d14ac0 mv=71 dg=0 li=69 ip=2510 vu=2512 pa=0 ls=1",
    ),
    (
        "shifted-stencil-90-stiff",
        "x=5d9fa03df5d86095 out=Failed col=200 cy=5 tr=5 cf=0 fc=0 res=3eea1a5da764d945 prj=7ff8000000000000 thr=3e0852cbc0d14ac0 mv=205 dg=0 li=200 ip=8200 vu=8205 pa=0 ls=0",
    ),
];

const STAGED_GUARDS: [(&str, &str); 4] = [
    (
        "tridiagonal-40",
        "x=a18f243bb85a8430 out=Converged col=14 cy=1 tr=1 cf=1 fc=0 res=3e1831d2b8ef938f prj=3e1831d48903be5d thr=3e2135bc82ba575f mv=15 dg=0 li=14 ip=210 vu=211 pa=0 ls=1",
    ),
    (
        "scattered-60-restart-10",
        "x=099c20385d3e09c9 out=Converged col=28 cy=3 tr=3 cf=1 fc=0 res=3dc47623bffbc1ab prj=3dc47622f3da5965 thr=3dd6765eb17d5429 mv=31 dg=0 li=28 ip=292 vu=295 pa=0 ls=1",
    ),
    (
        "shifted-stencil-90-restart-40",
        "x=6646bb022611c76c out=Converged col=69 cy=2 tr=2 cf=1 fc=0 res=3e07a00914dd6e3d prj=3e07a009139dcfbb thr=3e0852cbc0d14ac0 mv=71 dg=0 li=69 ip=2510 vu=2512 pa=0 ls=1",
    ),
    (
        "shifted-stencil-90-stiff",
        "x=5d9fa03df5d86095 out=Failed col=200 cy=5 tr=5 cf=0 fc=0 res=3eea1a5da764d945 prj=7ff8000000000000 thr=3e0852cbc0d14ac0 mv=205 dg=0 li=200 ip=8200 vu=8205 pa=0 ls=0",
    ),
];

#[test]
fn the_shipped_policy_returns_the_bits_of_the_code_before_the_edit() {
    for case in cases() {
        check(
            "SHIPPED_RECOMPUTE_FINAL",
            case.name,
            &shipped_line(&case, ResidualAccounting::RecomputeFinal),
            &SHIPPED_RECOMPUTE_FINAL,
        );
        check(
            "SHIPPED_REUSE_CONFIRMED",
            case.name,
            &shipped_line(&case, ResidualAccounting::ReuseConfirmed),
            &SHIPPED_REUSE_CONFIRMED,
        );
    }
}

#[test]
fn the_staged_solver_returns_the_bits_of_the_code_before_the_edit() {
    for case in cases().iter().filter(|case| case.x0.is_none()) {
        check(
            "STAGED_PLAIN",
            case.name,
            &staged_line(case, false),
            &STAGED_PLAIN,
        );
        check(
            "STAGED_GUARDS",
            case.name,
            &staged_line(case, true),
            &STAGED_GUARDS,
        );
    }
}
