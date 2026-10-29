//! N02 additive native path tests; full repo execution is an independent gate.
use rodas5p_core::{
    CoreError, CoreResult, LinearOperator, Preconditioner, PreconditionerSide, WorkCounters,
};
use rodas5p_krylov::{
    GmresCapacity, GmresConfig, GmresMonitorDecision, GmresWorkspace,
    solve_gmres_into_right_flexible_triggered_monitored,
};
use std::sync::atomic::{AtomicUsize, Ordering};
struct Identity {
    n: usize,
}
impl LinearOperator for Identity {
    fn dimension(&self) -> usize {
        self.n
    }
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        y.copy_from_slice(x);
        Ok(())
    }
    fn token(&self) -> u64 {
        0
    }
}
struct VariableRight {
    n: usize,
    calls: AtomicUsize,
}
impl Preconditioner for VariableRight {
    fn dimension(&self) -> usize {
        self.n
    }
    fn side(&self) -> PreconditionerSide {
        PreconditionerSide::Right
    }
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        let column = self.calls.fetch_add(1, Ordering::Relaxed);
        for i in 0..self.n {
            y[i] = if column == 0 && i == 0 {
                x[i]
            } else {
                2.0 * x[i]
            };
        }
        Ok(())
    }
}
fn config(n: usize) -> GmresConfig {
    GmresConfig {
        restart: n,
        max_arnoldi: n,
        rtol: 0.0,
        atol: 0.0,
    }
}
#[test]
fn variable_right_preconditioner_stores_z_and_recovers_original() {
    let op = Identity { n: 2 };
    let pc = VariableRight {
        n: 2,
        calls: AtomicUsize::new(0),
    };
    let mut out = [77.0, 77.0];
    let mut ws = GmresWorkspace::default();
    let mut work = WorkCounters::default();
    let r = solve_gmres_into_right_flexible_triggered_monitored(
        &op,
        &pc,
        &[1., 1.],
        &[0., 0.],
        &config(2),
        0.0,
        GmresCapacity::unbounded(),
        &mut out,
        &mut ws,
        &mut work,
        &mut |_, x, _| {
            let residual = (1. - x[0]).abs().max((1. - x[1]).abs());
            Ok(if residual < 1e-12 {
                GmresMonitorDecision::Accept
            } else {
                GmresMonitorDecision::Continue
            })
        },
    )
    .unwrap();
    assert!(r.accepted);
    assert_eq!(r.iterations, 2);
    assert!((out[0] - 1.0).abs() < 1e-12);
    assert!((out[1] - 1.0).abs() < 1e-12);
    assert_eq!(pc.calls.load(Ordering::Relaxed), 2);
    assert_eq!(work.linear_solves, 1);
}
#[test]
fn zero_trigger_versus_always_build_gives_same_final_candidate() {
    struct Diagonal;
    impl LinearOperator for Diagonal {
        fn dimension(&self) -> usize {
            8
        }
        fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
            for i in 0..8 {
                y[i] = (1.0 + i as f64 / 10.0) * x[i];
            }
            Ok(())
        }
        fn token(&self) -> u64 {
            0
        }
    }
    struct Right;
    impl Preconditioner for Right {
        fn dimension(&self) -> usize {
            8
        }
        fn side(&self) -> PreconditionerSide {
            PreconditionerSide::Right
        }
        fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
            y.copy_from_slice(x);
            Ok(())
        }
    }
    let mut outputs = Vec::new();
    let mut least_squares = Vec::new();
    for trigger in [0.0, f64::MAX] {
        let mut out = [5.0; 8];
        let mut ws = GmresWorkspace::default();
        let mut work = WorkCounters::default();
        let r = solve_gmres_into_right_flexible_triggered_monitored(
            &Diagonal,
            &Right,
            &[1.; 8],
            &[0.; 8],
            &config(8),
            trigger,
            GmresCapacity::unbounded(),
            &mut out,
            &mut ws,
            &mut work,
            &mut |k, x, _| {
                let norm = (0..8)
                    .map(|i| (1.0 - (1.0 + i as f64 / 10.0) * x[i]).abs())
                    .fold(0.0f64, f64::max);
                Ok(if k == 8 && norm < 1e-12 {
                    GmresMonitorDecision::Accept
                } else {
                    GmresMonitorDecision::Continue
                })
            },
        )
        .unwrap();
        assert!(r.accepted);
        outputs.push(out.map(f64::to_bits));
        least_squares.push(ws.monitored_progress().completed_least_squares_solves);
    }
    assert_eq!(outputs[0], outputs[1]);
    assert_eq!(least_squares, vec![1, 8]);
}
#[test]
fn a_monitor_error_cannot_mutate_output() {
    struct Right;
    impl Preconditioner for Right {
        fn dimension(&self) -> usize {
            1
        }
        fn side(&self) -> PreconditionerSide {
            PreconditionerSide::Right
        }
        fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
            y.copy_from_slice(x);
            Ok(())
        }
    }
    let mut out = [55.0];
    let mut ws = GmresWorkspace::default();
    let mut counts = WorkCounters::default();
    let e = solve_gmres_into_right_flexible_triggered_monitored(
        &Identity { n: 1 },
        &Right,
        &[1.],
        &[0.],
        &config(1),
        0.0,
        GmresCapacity::unbounded(),
        &mut out,
        &mut ws,
        &mut counts,
        &mut |_, _, _| Err(CoreError::NonFinite("stale epoch".into())),
    );
    assert!(e.is_err());
    assert_eq!(out, [55.]);
    assert_eq!(counts.linear_solves, 0);
}
