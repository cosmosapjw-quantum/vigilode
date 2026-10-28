//! Error-path work remains visible; a monitor alone authorizes output commit.
use rodas5p_core::{
    CoreError, CoreResult, LinearOperator, Preconditioner, PreconditionerSide, WorkCounters,
};
use rodas5p_krylov::{
    GmresConfig, GmresMonitorDecision, GmresWorkspace, solve_gmres_into_right_monitored,
};
use std::sync::atomic::{AtomicU64, Ordering};

struct Diag {
    n: usize,
    fail: u64,
    calls: AtomicU64,
}
impl LinearOperator for Diag {
    fn dimension(&self) -> usize {
        self.n
    }
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        let k = self.calls.fetch_add(1, Ordering::Relaxed) + 1;
        if k == self.fail {
            return Err(CoreError::NonFinite("injected operator failure".into()));
        }
        for (i, (a, b)) in y.iter_mut().zip(x).enumerate() {
            *a = (i + 1) as f64 * b;
        }
        Ok(())
    }
    fn token(&self) -> u64 {
        0
    }
}
struct Identity {
    n: usize,
    fail: u64,
    calls: AtomicU64,
}
impl Preconditioner for Identity {
    fn dimension(&self) -> usize {
        self.n
    }
    fn side(&self) -> PreconditionerSide {
        PreconditionerSide::Right
    }
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        let k = self.calls.fetch_add(1, Ordering::Relaxed) + 1;
        if k == self.fail {
            return Err(CoreError::LinearSolve(
                "injected preconditioner failure".into(),
            ));
        }
        y.copy_from_slice(x);
        Ok(())
    }
}
fn op(n: usize, fail: u64) -> Diag {
    Diag {
        n,
        fail,
        calls: AtomicU64::new(0),
    }
}
fn pc(n: usize, fail: u64) -> Identity {
    Identity {
        n,
        fail,
        calls: AtomicU64::new(0),
    }
}
fn config() -> GmresConfig {
    GmresConfig {
        restart: 3,
        max_arnoldi: 6,
        rtol: 0.,
        atol: 0.,
    }
}

#[test]
fn second_column_operator_and_preconditioner_errors_retain_first_column() {
    for (fail_op, fail_pc, expected_products) in [(2, 0, 2), (0, 3, 1)] {
        let op = op(3, fail_op);
        let pc = pc(3, fail_pc);
        let mut ws = GmresWorkspace::default();
        let mut counts = WorkCounters::default();
        let mut out = [99.; 3];
        let r = solve_gmres_into_right_monitored(
            &op,
            &pc,
            &[1.; 3],
            &[0.; 3],
            &config(),
            &mut out,
            &mut ws,
            &mut counts,
            &mut |_, _, _| Ok(GmresMonitorDecision::Continue),
        );
        assert!(r.is_err());
        assert_eq!(out, [99.; 3]);
        let p = ws.monitored_progress();
        assert_eq!(p.attempted_columns, 2);
        assert_eq!(p.completed_columns, 1);
        assert_eq!(p.column_operator_applications, expected_products);
        assert_eq!(p.cycles, 1);
        assert_eq!(p.least_squares_attempts, 1);
        assert_eq!(p.completed_least_squares_solves, 1);
        assert_eq!(counts.linear_iterations, 1);
        // Core counters count operator entries, including the application
        // whose following preconditioner attempt fails.
        assert_eq!(counts.linear_matvecs, 2);
        assert_eq!(counts.preconditioner_apps, if fail_pc == 0 { 3 } else { 2 });
        assert_eq!(counts.linear_solves, 0);
    }
}

#[test]
fn monitor_error_keeps_two_columns_and_next_call_resets_progress() {
    let op = op(3, 0);
    let pc = pc(3, 0);
    let mut ws = GmresWorkspace::default();
    let mut counts = WorkCounters::default();
    let mut out = [99.; 3];
    let r = solve_gmres_into_right_monitored(
        &op,
        &pc,
        &[1.; 3],
        &[0.; 3],
        &config(),
        &mut out,
        &mut ws,
        &mut counts,
        &mut |k, _, _| {
            if k == 2 {
                Err(CoreError::NonFinite("monitor failure".into()))
            } else {
                Ok(GmresMonitorDecision::Continue)
            }
        },
    );
    assert!(r.is_err());
    assert_eq!(out, [99.; 3]);
    let p = ws.monitored_progress();
    assert_eq!(p.attempted_columns, 2);
    assert_eq!(p.completed_columns, 2);
    assert_eq!(p.column_operator_applications, 2);
    assert_eq!(p.cycles, 1);
    assert_eq!(p.least_squares_attempts, 2);
    assert_eq!(p.completed_least_squares_solves, 2);
    assert_eq!(counts.linear_iterations, 2);
    assert_eq!(counts.linear_solves, 0);
    let bad = GmresConfig {
        restart: 0,
        ..config()
    };
    assert!(
        solve_gmres_into_right_monitored(
            &op,
            &pc,
            &[1.; 3],
            &[0.; 3],
            &bad,
            &mut out,
            &mut ws,
            &mut counts,
            &mut |_, _, _| Ok(GmresMonitorDecision::Accept)
        )
        .is_err()
    );
    assert_eq!(ws.monitored_progress(), Default::default());
    assert_eq!(out, [99.; 3]);
    assert_eq!(counts.linear_iterations, 2);
}

#[test]
fn happy_breakdown_continue_exhausts_without_false_error_or_output_commit() {
    let op = op(1, 0);
    let pc = pc(1, 0);
    let mut ws = GmresWorkspace::default();
    let mut counts = WorkCounters::default();
    let mut out = [99.];
    let r = solve_gmres_into_right_monitored(
        &op,
        &pc,
        &[1.],
        &[0.],
        &config(),
        &mut out,
        &mut ws,
        &mut counts,
        &mut |k, _, _| {
            assert_eq!(k, 1);
            Ok(GmresMonitorDecision::Continue)
        },
    )
    .unwrap();
    assert!(!r.accepted);
    assert_eq!(r.iterations, 1);
    assert_eq!(r.cycles, 1);
    assert_eq!(out, [99.]);
    assert_eq!(counts.linear_iterations, 1);
    assert_eq!(counts.linear_solves, 0);
    // An already exact initial iterate also has no direction to extend;
    // lack of monitor admission is a clean unaccepted result.
    let r = solve_gmres_into_right_monitored(
        &op,
        &pc,
        &[1.],
        &[1.],
        &config(),
        &mut out,
        &mut ws,
        &mut counts,
        &mut |_, _, _| panic!("no column exists"),
    )
    .unwrap();
    assert!(!r.accepted);
    assert_eq!(r.iterations, 0);
    assert_eq!(r.cycles, 0);
    assert_eq!(out, [99.]);
}
