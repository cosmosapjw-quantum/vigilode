//! Optional model epoch for interior-mutable callbacks (research node
//! `research/rnext07_model_epoch_20261003`, remaining-only DAG node R-NEXT-07).
//! Writes its results to `RNEXT07_OUTPUT` when set.

use rodas5p_core::{CoreError, LinearMethod, LinearSolverConfig, WorkCounters};
use rodas5p_integrators::{KrylovState, OdeProblem, Rodas5pMfFastWorkspace};
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

const N: usize = 6;
const Q: f64 = 0.1;
const T0: f64 = 0.3;
const H: f64 = 0.05;

/// `y_i' = -p (y_i - sin t) + cos t + q y_(i+1) y_i` (cyclic), `p` read from
/// an atomic cell by every callback.
struct Model {
    p: Arc<AtomicU64>,
    epoch: Arc<AtomicU64>,
    fail: Arc<AtomicBool>,
}

impl Model {
    fn new(p: f64) -> Self {
        Self {
            p: Arc::new(AtomicU64::new(p.to_bits())),
            epoch: Arc::new(AtomicU64::new(1)),
            fail: Arc::new(AtomicBool::new(false)),
        }
    }
    fn set_p(&self, p: f64) {
        self.p.store(p.to_bits(), Ordering::SeqCst);
    }
    fn problem(&self, with_epoch: bool) -> OdeProblem {
        let (p1, p2, p3, fail) = (
            self.p.clone(),
            self.p.clone(),
            self.p.clone(),
            self.fail.clone(),
        );
        let problem = OdeProblem::new(
            "epoch-model",
            N,
            Arc::new(move |t, y, out| {
                if fail.swap(false, Ordering::SeqCst) {
                    out[0] = 1.0e300;
                    return Err(CoreError::NonFinite("one-shot RHS failure".into()));
                }
                let p = f64::from_bits(p1.load(Ordering::SeqCst));
                for i in 0..N {
                    out[i] = -p * (y[i] - t.sin()) + t.cos() + Q * y[(i + 1) % N] * y[i];
                }
                Ok(())
            }),
            None,
            None,
            Some(Arc::new(move |_t, y, v, out| {
                let p = f64::from_bits(p2.load(Ordering::SeqCst));
                for i in 0..N {
                    let next = (i + 1) % N;
                    out[i] = (-p + Q * y[next]) * v[i] + Q * y[i] * v[next];
                }
                Ok(())
            })),
            Some(Arc::new(move |t, _y, out| {
                let p = f64::from_bits(p3.load(Ordering::SeqCst));
                for value in out.iter_mut() {
                    *value = p * t.cos() - t.sin();
                }
                Ok(())
            })),
            false,
            None,
            None,
        )
        .unwrap();
        if with_epoch {
            let epoch = self.epoch.clone();
            problem.with_model_epoch(Arc::new(move || epoch.load(Ordering::SeqCst)))
        } else {
            problem
        }
    }
}

fn config() -> LinearSolverConfig {
    LinearSolverConfig {
        method: LinearMethod::Gcrodr,
        restart: 4,
        recycle_dim: 2,
        maxiter: 400,
        rtol: 1.0e-12,
        atol: 1.0e-14,
        ..Default::default()
    }
}

fn y0() -> Vec<f64> {
    (0..N).map(|i| 0.5 + 0.1 * i as f64).collect()
}

/// One workspace, its carried GCRO-DR recycle state and the counters of the
/// last attempt.
struct Lane {
    work: Rodas5pMfFastWorkspace,
    recycle: Option<KrylovState>,
}

#[derive(Debug, PartialEq)]
struct Outcome {
    y_new: Vec<u64>,
    error_norm: u64,
    stages: Vec<Vec<u64>>,
    counters: WorkCounters,
}

impl Lane {
    fn new(problem: &OdeProblem) -> Self {
        Self {
            work: Rodas5pMfFastWorkspace::new(problem, &config()).unwrap(),
            recycle: KrylovState::for_method(LinearMethod::Gcrodr),
        }
    }
    fn attempt(&mut self, problem: &OdeProblem, fresh: bool) -> Result<Outcome, CoreError> {
        let mut counters = WorkCounters::default();
        let error = self.work.attempt(
            problem,
            T0,
            &y0(),
            H,
            fresh,
            self.recycle.as_mut(),
            1.0e-8,
            1.0e-6,
            &mut counters,
        )?;
        Ok(Outcome {
            y_new: self.work.y_new().iter().map(|v| v.to_bits()).collect(),
            error_norm: error.to_bits(),
            stages: (0..8)
                .map(|i| self.work.stage(i).iter().map(|v| v.to_bits()).collect())
                .collect(),
            counters,
        })
    }
    fn recycle_rank(&self) -> usize {
        match &self.recycle {
            Some(KrylovState::Gcrodr(state)) => state.rank(),
            _ => 0,
        }
    }
}

fn counters_json(c: &WorkCounters) -> Value {
    json!({
        "rhs_evaluations": c.rhs_evaluations, "ft_calls": c.ft_calls, "jvp_calls": c.jvp_calls,
        "recycle_cross_operator_refreshes": c.recycle_cross_operator_refreshes,
        "recycle_same_operator_uses": c.recycle_same_operator_uses,
        "recycle_refresh_matvecs": c.recycle_refresh_matvecs,
    })
}

/// Gate item 1: A bumped epoch after a parameter change: `fresh = false` equals a
/// `fresh = true` attempt on the changed model, recycle images refreshed.
fn epoch_change_invalidates() -> (bool, Value) {
    let model = Model::new(2.0);
    let problem = model.problem(true);
    let (mut stale_hint, mut fresh) = (Lane::new(&problem), Lane::new(&problem));
    let first_a = stale_hint.attempt(&problem, true).unwrap();
    let first_b = fresh.attempt(&problem, true).unwrap();
    let rank = stale_hint.recycle_rank();
    model.set_p(5.0);
    model.epoch.fetch_add(1, Ordering::SeqCst);
    let got = stale_hint.attempt(&problem, false).unwrap();
    let expected = fresh.attempt(&problem, true).unwrap();
    let refreshed = got.counters.ft_calls == 1
        && got.counters.rhs_evaluations == first_a.counters.rhs_evaluations
        && got.counters.recycle_cross_operator_refreshes > 0;
    let holds = first_a == first_b && rank > 0 && got == expected && refreshed;
    (
        holds,
        json!({"recycle_rank_before": rank, "bitwise_equal_to_fresh": got == expected,
               "counters": counters_json(&got.counters), "first_attempt": counters_json(&first_a.counters),
               "holds": holds}),
    )
}

/// Gate item 2: An unchanged epoch keeps the reuse, as without an epoch.
fn same_epoch_reuses() -> (bool, Value) {
    let with = Model::new(2.0);
    let without = Model::new(2.0);
    let (pw, po) = (with.problem(true), without.problem(false));
    let (mut lw, mut lo) = (Lane::new(&pw), Lane::new(&po));
    let a1 = lw.attempt(&pw, true).unwrap();
    let b1 = lo.attempt(&po, true).unwrap();
    let a2 = lw.attempt(&pw, false).unwrap();
    let b2 = lo.attempt(&po, false).unwrap();
    let reused = a2.counters.ft_calls == 0
        && a2.counters.rhs_evaluations + 1 == a1.counters.rhs_evaluations
        && a2.counters.recycle_cross_operator_refreshes == 0;
    let holds = a1 == b1 && a2 == b2 && reused;
    (
        holds,
        json!({"reused": reused, "equal_to_no_epoch": a2 == b2,
               "counters": counters_json(&a2.counters), "holds": holds}),
    )
}

/// Gate item 3: Without an epoch a parameter change is not detected (manual
/// contract). The operator calls the JVP callback live, so `fresh = false`
/// mixes a stale `f(t, y)`, `f_t` and recycle images with a live operator.
fn no_epoch_keeps_manual_contract() -> (bool, Value) {
    // GCRO-DR: stale recycle images under the live operator.
    let model = Model::new(2.0);
    let p = model.problem(false);
    let mut lane = Lane::new(&p);
    lane.attempt(&p, true).unwrap();
    model.set_p(5.0);
    let stale = lane.attempt(&p, false);
    let changed = Lane::new(&p).attempt(&p, true).unwrap();
    let gcrodr_undetected = match &stale {
        Err(_) => true,
        Ok(outcome) => outcome.y_new != changed.y_new,
    };
    let gcrodr_outcome = match &stale {
        Err(error) => json!({"error": error.to_string()}),
        Ok(outcome) => {
            json!({"completed": true, "equals_changed_model": outcome.y_new == changed.y_new})
        }
    };

    // GMRES (no recycle state): completes with stale f and a live operator.
    let gmres = LinearSolverConfig {
        method: LinearMethod::Gmres,
        ..config()
    };
    let run = |problem: &OdeProblem, work: &mut Rodas5pMfFastWorkspace, fresh: bool| {
        work.attempt(
            problem,
            T0,
            &y0(),
            H,
            fresh,
            None,
            1.0e-8,
            1.0e-6,
            &mut WorkCounters::default(),
        )
        .map(|_| work.y_new().iter().map(|v| v.to_bits()).collect::<Vec<_>>())
    };
    let model = Model::new(2.0);
    let p = model.problem(false);
    let mut work = Rodas5pMfFastWorkspace::new(&p, &gmres).unwrap();
    let old_model = run(&p, &mut work, true).unwrap();
    model.set_p(5.0);
    let stale = run(&p, &mut work, false);
    let changed = run(
        &p,
        &mut Rodas5pMfFastWorkspace::new(&p, &gmres).unwrap(),
        true,
    )
    .unwrap();
    let fresh_again = run(&p, &mut work, true).unwrap();
    let gmres_mixed = stale
        .as_ref()
        .is_ok_and(|y| *y != old_model && *y != changed);
    let holds = gcrodr_undetected && gmres_mixed && fresh_again == changed;
    (
        holds,
        json!({"gcrodr": gcrodr_outcome, "gcrodr_change_undetected": gcrodr_undetected,
               "gmres_completes_and_differs_from_both_models": gmres_mixed,
               "fresh_true_gives_changed_model": fresh_again == changed, "holds": holds}),
    )
}

/// Gate item 4: A smaller epoch invalidates; a failed refresh leaves nothing reusable.
/// Lane `a` uses `fresh = false`, its twin `b` (same history) `fresh = true`.
fn any_change_and_failure() -> (bool, Value) {
    let model = Model::new(2.0);
    model.epoch.store(5, Ordering::SeqCst);
    let problem = model.problem(true);
    let (mut a, mut b) = (Lane::new(&problem), Lane::new(&problem));
    a.attempt(&problem, true).unwrap();
    b.attempt(&problem, true).unwrap();
    model.set_p(3.0);
    model.epoch.store(3, Ordering::SeqCst);
    let got = a.attempt(&problem, false).unwrap();
    let expected = b.attempt(&problem, true).unwrap();
    let smaller_invalidates = got == expected && got.counters.ft_calls == 1;

    // Failure: epoch 3 -> 4 with p = 4, the refresh fails before any solve;
    // then p = 3 and epoch 3 again. The data frozen under epoch 3 were dropped
    // by the failed refresh, so the next `fresh = false` attempt refreshes.
    model.set_p(4.0);
    model.epoch.store(4, Ordering::SeqCst);
    model.fail.store(true, Ordering::SeqCst);
    let failed = a.attempt(&problem, false).is_err();
    model.set_p(3.0);
    model.epoch.store(3, Ordering::SeqCst);
    let after = a.attempt(&problem, false).unwrap();
    let twin = b.attempt(&problem, true).unwrap();
    let refreshed_after_failure = after == twin && after.counters.ft_calls == 1;
    let holds = smaller_invalidates && failed && refreshed_after_failure;
    (
        holds,
        json!({"smaller_epoch_invalidates": smaller_invalidates, "refresh_failed": failed,
               "refreshed_after_failure": refreshed_after_failure,
               "counters_after_failure": counters_json(&after.counters), "holds": holds}),
    )
}

#[test]
fn model_epoch_contract() {
    let (g1, r1) = epoch_change_invalidates();
    let (g2, r2) = same_epoch_reuses();
    let (g3, r3) = no_epoch_keeps_manual_contract();
    let (g4, r4) = any_change_and_failure();
    let report = json!({
        "schema": "vigilode-rnext07-model-epoch-v1",
        "node": "research/rnext07_model_epoch_20261003",
        "epoch_change_invalidates": r1,
        "same_epoch_reuses": r2,
        "no_epoch_manual_contract": r3,
        "any_change_and_failure": r4,
        "gate_items_1_to_4": {"1": g1, "2": g2, "3": g3, "4": g4},
        "note": "item 5 (existing MF workspace contract tests) is recorded from their own test runs",
    });
    if let Ok(path) = std::env::var("RNEXT07_OUTPUT") {
        // Relative to the workspace root, not the crate directory.
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path);
        assert!(!path.exists(), "immutable output exists");
        std::fs::write(&path, serde_json::to_string_pretty(&report).unwrap() + "\n").unwrap();
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&report["gate_items_1_to_4"]).unwrap()
    );
    assert!(g1 && g2 && g3 && g4, "{report:#}");
}
