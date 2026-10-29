//! N03 source-conditional physical error guard attached to N02 opt-in GMRES.
use rodas5p_core::{
    CoreError, CoreResult, LinearOperator, Preconditioner, PreconditionerSide, WorkCounters,
};
use rodas5p_krylov::{
    CertifiedInterval, GmresCapacity, GmresConfig, GmresMonitorDecision, GmresWorkspace,
    N03SourceStamp, StageIntervalBox, solve_gmres_into_right_flexible_triggered_monitored,
};
use std::cell::Cell;

struct ShiftedScalar;
impl LinearOperator for ShiftedScalar {
    fn dimension(&self) -> usize {
        1
    }
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        y[0] = 1.5 * x[0];
        Ok(())
    }
    fn token(&self) -> u64 {
        0
    }
}
struct RightIdentity;
impl Preconditioner for RightIdentity {
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
fn snapshot() -> N03SourceStamp {
    N03SourceStamp {
        model: "toy-J".into(),
        source: "fixed".into(),
        clock: "t".into(),
        norm: "physical-weighted".into(),
        epoch: 3,
    }
}
fn box_stage() -> StageIntervalBox {
    StageIntervalBox::new(
        1,
        vec![CertifiedInterval::point(-2.).unwrap()],
        vec![CertifiedInterval::point(3.).unwrap()],
        vec![1.],
        vec![1.],
        0.25,
        snapshot(),
    )
    .unwrap()
}
fn conf() -> GmresConfig {
    GmresConfig {
        restart: 1,
        max_arnoldi: 1,
        rtol: 0.,
        atol: 0.,
    }
}

#[test]
fn n02_opt_in_accepts_only_n03_original_residual_and_error_gate() {
    let stage = box_stage();
    let key = snapshot();
    let mut out = [77.];
    let mut guard_out = [0.];
    let mut work = WorkCounters::default();
    let mut ws = GmresWorkspace::default();
    let report = solve_gmres_into_right_flexible_triggered_monitored(
        &ShiftedScalar,
        &RightIdentity,
        &[3.],
        &[0.],
        &conf(),
        0.0,
        GmresCapacity::unbounded(),
        &mut out,
        &mut ws,
        &mut work,
        &mut |_, candidate, _| {
            let v = stage
                .commit_if_admitted(&key, || snapshot(), candidate, &mut guard_out, 1e-11, 1e-11)
                .map_err(CoreError::LinearSolve)?;
            Ok(if v.admitted_within_given_box {
                GmresMonitorDecision::Accept
            } else {
                GmresMonitorDecision::Continue
            })
        },
    )
    .unwrap();
    assert!(report.accepted);
    assert!((out[0] - 2.0).abs() < 1e-11);
    assert_eq!(out, guard_out);
    assert_eq!(work.linear_solves, 1);
}

#[test]
fn stale_source_before_monitor_commit_refuses_transactionally() {
    let stage = box_stage();
    let key = snapshot();
    let count = Cell::new(0usize);
    let mut out = [77.];
    let mut guard_out = [88.];
    let mut work = WorkCounters::default();
    let mut ws = GmresWorkspace::default();
    let failure = solve_gmres_into_right_flexible_triggered_monitored(
        &ShiftedScalar,
        &RightIdentity,
        &[3.],
        &[0.],
        &conf(),
        0.0,
        GmresCapacity::unbounded(),
        &mut out,
        &mut ws,
        &mut work,
        &mut |_, candidate, _| {
            stage
                .commit_if_admitted(
                    &key,
                    || {
                        count.set(count.get() + 1);
                        let mut s = snapshot();
                        if count.get() > 1 {
                            s.epoch += 1;
                        }
                        s
                    },
                    candidate,
                    &mut guard_out,
                    1e-11,
                    1e-11,
                )
                .map(|_| GmresMonitorDecision::Accept)
                .map_err(CoreError::LinearSolve)
        },
    );
    assert!(failure.is_err());
    assert_eq!(out, [77.]);
    assert_eq!(guard_out, [88.]);
    assert_eq!(work.linear_solves, 0);
}
