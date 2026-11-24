//! Contract tests of the forward break-even pilots (FBP) of the unified router: research
//! loop L1, node V27_ROUTER_HYPERPARAMETER_FREE, phase 1, owner decision OD-L1-22. The
//! design is `research/production_loop_20261116/router_hyperparameter_free/DESIGN_RECOMMENDED.md`
//! (sections 3 and 4) and the problems are those of `PREDECLARED.json` in that directory.
//!
//! Part 1 holds the failing-first tests of `PREDECLARED.json` (`order_of_work`, phase 1).
//! They use only names that exist both in the API of the base commit `1ffbda20` and in the
//! API of this node, and were run against the base sources first
//! (`router_hyperparameter_free/logs/failing_first_base.log`).
//!
//! Every test prints one `V27_RECORD {json}` line with the numbers it measured before it
//! asserts.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Value, json};
use vigilode::prelude::*;

// ---------------------------------------------------------------------------- records

fn record(test: &str, fields: Value) {
    println!("V27_RECORD {}", json!({ "test": test, "fields": fields }));
}

/// Callback entries counted inside the user closures.
#[derive(Clone, Default)]
struct Calls {
    rhs: Arc<AtomicU64>,
    fatal: Arc<AtomicU64>,
}

impl Calls {
    fn rhs(&self) -> u64 {
        self.rhs.load(Ordering::Relaxed)
    }
    fn fatal(&self) -> u64 {
        self.fatal.load(Ordering::Relaxed)
    }
}

fn bits(state: &[f64]) -> Vec<u64> {
    state.iter().map(|v| v.to_bits()).collect()
}

fn has_rodas5p_leg(solution: &VigilodeSolution) -> bool {
    solution
        .legs
        .iter()
        .any(|leg| leg.tier == SelectedTier::TierProtectedReference)
}

fn leg_summary(solution: &VigilodeSolution) -> Value {
    solution
        .legs
        .iter()
        .map(|leg| {
            json!({
                "tier": format!("{:?}", leg.tier),
                "driver": leg.driver_id,
                "t_start": leg.t_start,
                "t_end": leg.t_end,
                "attempts": leg.attempts,
                "rhs_calls": leg.counters.rhs_calls,
                "success": leg.success,
            })
        })
        .collect()
}

// ------------------------------------------------------------------------------ costs

/// Break-even length of DESIGN_RECOMMENDED.md 3.2, `ceil((2 n^2 + 54 n + 384) / 156)`,
/// restated here so that the tests do not depend on the implementation.
fn break_even(n: u64) -> u64 {
    (2 * n * n + 54 * n + 384).div_ceil(156)
}

/// Three times the operation count of `PREDECLARED.json` (`measures`): 52 n per DOPRI5
/// attempt, 128 n per RODAS5P attempt, 2 n^3 / 3 per factorization, 2 n^2 per solve pair
/// and per finite-difference build. Integers throughout.
fn ops3(
    n: u64,
    dopri5_attempts: u64,
    rodas5p_attempts: u64,
    factorizations: u64,
    solves: u64,
    builds: u64,
) -> u128 {
    let n = u128::from(n);
    156 * n * u128::from(dopri5_attempts)
        + 384 * n * u128::from(rodas5p_attempts)
        + 2 * n.pow(3) * u128::from(factorizations)
        + 6 * n * n * (u128::from(solves) + u128::from(builds))
}

/// Charged callbacks and operations of a run, with its attempts by method.
#[derive(Clone, Copy, Debug)]
struct Cost {
    callbacks: u64,
    ops3: u128,
    dopri5_attempts: u64,
    rodas5p_attempts: u64,
}

impl Cost {
    fn json(&self) -> Value {
        json!({
            "callbacks": self.callbacks,
            "operations": (self.ops3 as f64) / 3.0,
            "dopri5_attempts": self.dopri5_attempts,
            "rodas5p_attempts": self.rodas5p_attempts,
        })
    }
}

/// The cost of a unified solve: DOPRI5 attempts are those of its DOPRI5 legs, every other
/// attempt of the solve (RODAS5P legs and pilots) is a RODAS5P attempt.
fn unified_cost(n: usize, solution: &VigilodeSolution) -> Cost {
    let dopri5_attempts: u64 = solution
        .legs
        .iter()
        .filter(|leg| leg.tier == SelectedTier::Tier1Explicit)
        .map(|leg| leg.attempts as u64)
        .sum();
    let rodas5p_attempts = (solution.stats.attempts as u64).saturating_sub(dopri5_attempts);
    let c = &solution.stats.counters;
    Cost {
        callbacks: c.rhs_calls + c.ft_calls + c.jvp_calls,
        ops3: ops3(
            n as u64,
            dopri5_attempts,
            rodas5p_attempts,
            c.direct_factorizations,
            c.linear_solves,
            c.jacobian_builds,
        ),
        dopri5_attempts,
        rodas5p_attempts,
    }
}

fn dopri5_cost(n: usize, solution: &ExplicitSolution) -> Cost {
    let attempts = solution.stats.attempts as u64;
    Cost {
        callbacks: solution.stats.counters.rhs_calls,
        ops3: ops3(n as u64, attempts, 0, 0, 0, 0),
        dopri5_attempts: attempts,
        rodas5p_attempts: 0,
    }
}

fn rodas5p_cost(n: usize, solution: &Solution) -> Cost {
    let c = &solution.stats.counters;
    let attempts = solution.stats.attempts as u64;
    Cost {
        callbacks: c.rhs_calls + c.ft_calls + c.jvp_calls,
        ops3: ops3(
            n as u64,
            0,
            attempts,
            c.direct_factorizations,
            c.linear_solves,
            c.jacobian_builds,
        ),
        dopri5_attempts: 0,
        rodas5p_attempts: attempts,
    }
}

/// Largest number of pilots after `dopri5_attempts` DOPRI5 attempts:
/// `1 + floor(log2(A_D / b))` when `A_D >= b`, else 0 (pilot k needs `A_D >= b 2^k`).
fn max_pilots(dopri5_attempts: u64, b: u64) -> u64 {
    let ratio = dopri5_attempts / b;
    if ratio == 0 {
        0
    } else {
        1 + u64::from(63 - ratio.leading_zeros())
    }
}

// --------------------------------------------------------------------------- problems

/// F2: `y' = -y` whose right-hand side fails fatally whenever `y > 1`.
fn f2(calls: &Calls) -> Problem {
    let (rhs, fatal) = (calls.rhs.clone(), calls.fatal.clone());
    Problem::new(1, move |_t, y: &[f64], dy: &mut [f64]| {
        rhs.fetch_add(1, Ordering::Relaxed);
        if y[0] > 1.0 {
            fatal.fetch_add(1, Ordering::Relaxed);
            return Err(CallbackError::Fatal("F2: the state left [0, 1]".into()));
        }
        dy[0] = -y[0];
        Ok(())
    })
    .autonomous()
}

/// Lorenz-96 with forcing 8: `y_i' = (y_{i+1} - y_{i-2}) y_{i-1} - y_i + 8`, cyclic.
fn lorenz96(n: usize, calls: &Calls) -> Problem {
    let rhs = calls.rhs.clone();
    Problem::new(n, move |_t, y: &[f64], dy: &mut [f64]| {
        rhs.fetch_add(1, Ordering::Relaxed);
        for i in 0..n {
            dy[i] = (y[(i + 1) % n] - y[(i + n - 2) % n]) * y[(i + n - 1) % n] - y[i] + 8.0;
        }
        Ok::<(), CallbackError>(())
    })
    .autonomous()
}

fn lorenz96_y0(n: usize) -> Vec<f64> {
    let mut y0 = vec![8.0; n];
    y0[19] += 0.01;
    y0
}

fn robertson_fd(calls: &Calls) -> Problem {
    let rhs = calls.rhs.clone();
    Problem::new(3, move |_t, y: &[f64], dy: &mut [f64]| {
        rhs.fetch_add(1, Ordering::Relaxed);
        dy[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
        dy[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
        dy[2] = 3.0e7 * y[1] * y[1];
        Ok::<(), CallbackError>(())
    })
    .autonomous()
}

fn oscillator(calls: &Calls) -> Problem {
    let rhs = calls.rhs.clone();
    Problem::new(2, move |_t, y: &[f64], dy: &mut [f64]| {
        rhs.fetch_add(1, Ordering::Relaxed);
        dy[0] = y[1];
        dy[1] = -y[0];
        Ok::<(), CallbackError>(())
    })
    .autonomous()
}

/// The Arenstorf orbit (restricted three-body problem), first-order form.
fn arenstorf(calls: &Calls) -> Problem {
    let rhs = calls.rhs.clone();
    let mu = 0.012277471;
    let mup = 1.0 - mu;
    Problem::new(4, move |_t, y: &[f64], dy: &mut [f64]| {
        rhs.fetch_add(1, Ordering::Relaxed);
        let d1 = ((y[0] + mu).powi(2) + y[1] * y[1]).powf(1.5);
        let d2 = ((y[0] - mup).powi(2) + y[1] * y[1]).powf(1.5);
        dy[0] = y[2];
        dy[1] = y[3];
        dy[2] = y[0] + 2.0 * y[3] - mup * (y[0] + mu) / d1 - mu * (y[0] - mup) / d2;
        dy[3] = y[1] - 2.0 * y[2] - mup * y[1] / d1 - mu * y[1] / d2;
        Ok::<(), CallbackError>(())
    })
    .autonomous()
}

/// `y0` and one period of the orbit; the f64 values nearest to
/// -2.00158510637908252240537862224 and 17.0652165601579625588917206249.
const ARENSTORF_Y0: [f64; 4] = [0.994, 0.0, 0.0, -2.001_585_106_379_082_4];
const ARENSTORF_PERIOD: f64 = 17.065_216_560_157_964;

// ======================================================================================
// Part 1: failing-first tests (public API of the base commit and of this node)
// ======================================================================================

#[test]
fn f2_fatal_above_one_completes_under_auto() {
    let test = "f2_fatal_above_one_completes_under_auto";
    let span = (0.0, 1.0);
    let direct_calls = Calls::default();
    let direct = Dopri5::solve(&f2(&direct_calls), span, &[1.0], &Options::default())
        .expect("direct DOPRI5 call");
    let calls = Calls::default();
    let solved = VigilODE::solve(&f2(&calls), span, &[1.0], SolverStrategy::Auto);
    record(
        test,
        json!({
            "outcome": match &solved {
                Ok(solution) => format!("{:?}", solution.status),
                Err(error) => format!("Err: {error}"),
            },
            "legs": solved.as_ref().map(leg_summary).unwrap_or(Value::Null),
            "final_time": solved.as_ref().map(|s| s.final_time).ok(),
            "fatal_entries": calls.fatal(),
            "rhs_entries": calls.rhs(),
            "direct": { "success": direct.success, "rhs_entries": direct_calls.rhs(),
                        "fatal_entries": direct_calls.fatal() },
        }),
    );
    assert!(
        direct.success,
        "Dopri5::solve completes F2: {}",
        direct.message
    );
    let solution = solved.expect("F2 is a valid request");
    assert!(
        solution.success(),
        "F2 must complete under Auto as it does under Dopri5::solve, got {:?}",
        solution.status
    );
    assert_eq!(
        calls.fatal(),
        0,
        "a callback was made at a state above 1, off the trajectory"
    );
}

#[test]
fn lorenz96_40_has_no_rodas5p_leg_and_costs_at_most_twice_dopri5() {
    let test = "lorenz96_40_has_no_rodas5p_leg_and_costs_at_most_twice_dopri5";
    let n = 40;
    let span = (0.0, 10.0);
    let y0 = lorenz96_y0(n);
    let direct_calls = Calls::default();
    let direct = Dopri5::solve(&lorenz96(n, &direct_calls), span, &y0, &Options::default())
        .expect("direct DOPRI5 call");
    let calls = Calls::default();
    let unified = VigilODE::solve(&lorenz96(n, &calls), span, &y0, SolverStrategy::Auto)
        .expect("unified solve");
    let (cost, reference) = (unified_cost(n, &unified), dopri5_cost(n, &direct));
    record(
        test,
        json!({
            "legs": leg_summary(&unified),
            "status": format!("{:?}", unified.status),
            "unified": cost.json(),
            "dopri5": reference.json(),
            "rhs_entries": calls.rhs(),
            "callback_ratio": cost.callbacks as f64 / reference.callbacks as f64,
            "operation_ratio": cost.ops3 as f64 / reference.ops3 as f64,
        }),
    );
    assert!(direct.success && unified.success());
    assert!(
        !has_rodas5p_leg(&unified),
        "Lorenz-96 n = 40 is not stiff, yet a RODAS5P leg ran: {}",
        leg_summary(&unified)
    );
    assert!(
        cost.callbacks <= 2 * reference.callbacks,
        "{} charged callbacks, more than twice the {} of Dopri5::solve",
        cost.callbacks,
        reference.callbacks
    );
    assert!(
        cost.ops3 <= 2 * reference.ops3,
        "{} operations, more than twice the {} of Dopri5::solve",
        cost.ops3 as f64 / 3.0,
        reference.ops3 as f64 / 3.0
    );
}

#[test]
fn robertson_40_charges_at_most_two_direct_rodas5p_plus_window_and_pilots() {
    let test = "robertson_40_charges_at_most_two_direct_rodas5p_plus_window_and_pilots";
    let n = 3usize;
    let span = (0.0, 40.0);
    let y0 = [1.0, 0.0, 0.0];
    let direct_calls = Calls::default();
    let direct = Rodas5p::solve(&robertson_fd(&direct_calls), span, &y0, &Options::default())
        .expect("direct RODAS5P call");
    let calls = Calls::default();
    let unified = VigilODE::solve(&robertson_fd(&calls), span, &y0, SolverStrategy::Auto)
        .expect("unified solve");
    let (cost, reference) = (unified_cost(n, &unified), rodas5p_cost(n, &direct));
    let nn = n as u64;
    let b = break_even(nn);
    // one DOPRI5 attempt: 6 callbacks and 52 n operations; one fresh finite-difference
    // RODAS5P attempt of an autonomous problem: n + 9 callbacks, (2n^3 + 54n^2 + 384n) / 3
    let pilots = max_pilots(cost.dopri5_attempts, b);
    let bound_callbacks = 2 * reference.callbacks + 2 * b * 6 + pilots * (nn + 9);
    let bound_ops3 = 2 * reference.ops3
        + u128::from(2 * b * 156 * nn)
        + u128::from(pilots) * u128::from(2 * nn.pow(3) + 54 * nn * nn + 384 * nn);
    record(
        test,
        json!({
            "legs": leg_summary(&unified),
            "status": format!("{:?}", unified.status),
            "unified": cost.json(),
            "rodas5p": reference.json(),
            "b": b,
            "max_pilots": pilots,
            "bound_callbacks": bound_callbacks,
            "bound_operations": bound_ops3 as f64 / 3.0,
            "rhs_entries": calls.rhs(),
        }),
    );
    assert!(direct.is_success() && unified.success());
    assert!(
        cost.callbacks <= bound_callbacks,
        "{} charged callbacks above 2 x {} (direct RODAS5P) + 2 b c_D + {} pilots = {}",
        cost.callbacks,
        reference.callbacks,
        pilots,
        bound_callbacks
    );
    assert!(
        cost.ops3 <= bound_ops3,
        "{} operations above the bound {}",
        cost.ops3 as f64 / 3.0,
        bound_ops3 as f64 / 3.0
    );
}

#[test]
fn oscillator_1000_has_no_rodas5p_leg() {
    let test = "oscillator_1000_has_no_rodas5p_leg";
    let calls = Calls::default();
    let unified = VigilODE::solve(
        &oscillator(&calls),
        (0.0, 1000.0),
        &[1.0, 0.0],
        SolverStrategy::Auto,
    )
    .expect("unified solve");
    let cost = unified_cost(2, &unified);
    record(
        test,
        json!({
            "legs": leg_summary(&unified),
            "status": format!("{:?}", unified.status),
            "unified": cost.json(),
            "final_state_bits": bits(&unified.final_state),
        }),
    );
    assert!(unified.success());
    assert!(
        !has_rodas5p_leg(&unified),
        "the harmonic oscillator is not stiff, yet a RODAS5P leg ran: {}",
        leg_summary(&unified)
    );
}

#[test]
fn arenstorf_has_no_rodas5p_leg() {
    let test = "arenstorf_has_no_rodas5p_leg";
    let calls = Calls::default();
    let unified = VigilODE::solve(
        &arenstorf(&calls),
        (0.0, ARENSTORF_PERIOD),
        &ARENSTORF_Y0,
        SolverStrategy::Auto,
    )
    .expect("unified solve");
    let cost = unified_cost(4, &unified);
    record(
        test,
        json!({
            "legs": leg_summary(&unified),
            "status": format!("{:?}", unified.status),
            "unified": cost.json(),
            "final_state": unified.final_state,
        }),
    );
    assert!(unified.success());
    assert!(
        !has_rodas5p_leg(&unified),
        "the Arenstorf orbit is not stiff, yet a RODAS5P leg ran: {}",
        leg_summary(&unified)
    );
}

// ======================================================================================
// Part 2: the DOPRI5 session that the break-even route drives
// ======================================================================================

fn lotka_volterra(calls: &Calls) -> Problem {
    let rhs = calls.rhs.clone();
    Problem::new(2, move |_t, y: &[f64], dy: &mut [f64]| {
        rhs.fetch_add(1, Ordering::Relaxed);
        dy[0] = 1.5 * y[0] - y[0] * y[1];
        dy[1] = -3.0 * y[1] + y[0] * y[1];
        Ok::<(), CallbackError>(())
    })
    .autonomous()
}

/// One DOPRI5 run through the public session with a quota of one attempt per call, as the
/// break-even route drives it, and the direct `Dopri5::solve` with the same options.
fn quota_one_and_direct(
    build: &dyn Fn(&Calls) -> Problem,
    span: (f64, f64),
    y0: &[f64],
    options: &Options,
) -> (ExplicitSolution, ExplicitSolution, usize, u64, u64) {
    let direct_calls = Calls::default();
    let direct = Dopri5::solve(&build(&direct_calls), span, y0, options).expect("direct DOPRI5");
    let calls = Calls::default();
    let problem = build(&calls);
    let (advances, session) = Dopri5::with_session(
        &problem,
        span,
        y0,
        options,
        &[span.0, span.1],
        vigilode::SessionEpochs::default(),
        |session| {
            let mut advances = 0usize;
            loop {
                let before = session.checkpoint().attempts;
                let outcome = session
                    .advance_to(span.1, 1, vigilode::SessionEpochs::default())
                    .expect("a registered target");
                advances += 1;
                assert!(session.checkpoint().attempts <= before + 1, "quota of one");
                if outcome != vigilode::SessionAdvance::Yield {
                    break advances;
                }
            }
        },
    )
    .expect("session run");
    (session, direct, advances, calls.rhs(), direct_calls.rhs())
}

#[test]
fn session_quota_one_advances_keep_the_bits_and_counters_of_dopri5_solve() {
    let test = "session_quota_one_advances_keep_the_bits_and_counters_of_dopri5_solve";
    type Case = (
        &'static str,
        Box<dyn Fn(&Calls) -> Problem>,
        (f64, f64),
        Vec<f64>,
        usize,
    );
    let cases: Vec<Case> = vec![
        (
            "oscillator",
            Box::new(oscillator),
            (0.0, 10.0),
            vec![1.0, 0.0],
            100_000,
        ),
        (
            "lotka-volterra",
            Box::new(lotka_volterra),
            (0.0, 100.0),
            vec![1.0, 1.0],
            100_000,
        ),
        ("f2", Box::new(f2), (0.0, 1.0), vec![1.0], 100_000),
        // the session's own attempt budget ends the run
        (
            "robertson-budget-300",
            Box::new(robertson_fd),
            (0.0, 40.0),
            vec![1.0, 0.0, 0.0],
            300,
        ),
    ];
    let mut rows = Vec::new();
    for (name, build, span, y0, max_steps) in &cases {
        let options = Options {
            max_steps: *max_steps,
            ..Options::default()
        };
        let (session, direct, advances, entries, direct_entries) =
            quota_one_and_direct(&**build, *span, y0, &options);
        let equal_stats = serde_json::to_value(&session.stats).unwrap()
            == serde_json::to_value(&direct.stats).unwrap();
        rows.push(json!({
            "case": name, "advances": advances, "attempts": session.stats.attempts,
            "direct_attempts": direct.stats.attempts, "success": session.success,
            "stop_reason": format!("{:?}", session.stop_reason),
            "final_state_bits_equal": bits(&session.final_state) == bits(&direct.final_state),
            "stats_equal": equal_stats, "rhs_entries": [entries, direct_entries],
        }));
        assert_eq!(session.success, direct.success, "{name}");
        assert_eq!(session.stop_reason, direct.stop_reason, "{name}");
        assert_eq!(
            session.final_time.to_bits(),
            direct.final_time.to_bits(),
            "{name}"
        );
        assert_eq!(
            bits(&session.final_state),
            bits(&direct.final_state),
            "{name}"
        );
        assert!(equal_stats, "{name}: the work record differs");
        assert_eq!(entries, direct_entries, "{name}: callback entries differ");
    }
    record(test, json!({ "cases": rows }));
}

// ======================================================================================
// Part 3: the properties U1-U5 of DESIGN_RECOMMENDED.md 4.1 and owner decision D1
// ======================================================================================

use std::sync::Mutex;

/// Every right-hand-side entry `(t, y)` in call order.
type Log = Arc<Mutex<Vec<(f64, Vec<f64>)>>>;

/// A problem whose right-hand side `f` records every entry in `log` and counts it.
fn logged(
    n: usize,
    calls: &Calls,
    log: &Log,
    f: impl Fn(&[f64], &mut [f64]) + Send + Sync + 'static,
) -> Problem {
    let (rhs, log) = (calls.rhs.clone(), log.clone());
    Problem::new(n, move |t: f64, y: &[f64], dy: &mut [f64]| {
        rhs.fetch_add(1, Ordering::Relaxed);
        log.lock().unwrap().push((t, y.to_vec()));
        f(y, dy);
        Ok::<(), CallbackError>(())
    })
    .autonomous()
}

fn damped_oscillator(y: &[f64], dy: &mut [f64]) {
    dy[0] = y[1];
    dy[1] = -1.0e4 * y[0] - 0.2 * y[1];
}

fn nonnormal(y: &[f64], dy: &mut [f64]) {
    dy[0] = -y[0] + 1.0e3 * y[1];
    dy[1] = -2.0 * y[1];
}

fn robertson_rhs(y: &[f64], dy: &mut [f64]) {
    dy[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
    dy[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
    dy[2] = 3.0e7 * y[1] * y[1];
}

type Rhs = fn(&[f64], &mut [f64]);

/// A named problem: dimension, right-hand side, initial state, span.
type NamedCase = (&'static str, usize, Rhs, Vec<f64>, (f64, f64));

/// The non-stiff problems of PREDECLARED.json used for U1 (no handover expected).
fn non_stiff_cases() -> Vec<NamedCase> {
    fn osc(y: &[f64], dy: &mut [f64]) {
        dy[0] = y[1];
        dy[1] = -y[0];
    }
    fn lv(y: &[f64], dy: &mut [f64]) {
        dy[0] = 1.5 * y[0] - y[0] * y[1];
        dy[1] = -3.0 * y[1] + y[0] * y[1];
    }
    fn l96(y: &[f64], dy: &mut [f64]) {
        let n = y.len();
        for i in 0..n {
            dy[i] = (y[(i + 1) % n] - y[(i + n - 2) % n]) * y[(i + n - 1) % n] - y[i] + 8.0;
        }
    }
    fn aren(y: &[f64], dy: &mut [f64]) {
        let mu = 0.012277471;
        let mup = 1.0 - mu;
        let d1 = ((y[0] + mu).powi(2) + y[1] * y[1]).powf(1.5);
        let d2 = ((y[0] - mup).powi(2) + y[1] * y[1]).powf(1.5);
        dy[0] = y[2];
        dy[1] = y[3];
        dy[2] = y[0] + 2.0 * y[3] - mup * (y[0] + mu) / d1 - mu * (y[0] - mup) / d2;
        dy[3] = y[1] - 2.0 * y[2] - mup * y[1] / d1 - mu * y[1] / d2;
    }
    vec![
        ("osc_1", 2, osc, vec![1.0, 0.0], (0.0, 1.0)),
        ("osc_10", 2, osc, vec![1.0, 0.0], (0.0, 10.0)),
        ("osc_1000", 2, osc, vec![1.0, 0.0], (0.0, 1000.0)),
        ("lotka_volterra", 2, lv, vec![1.0, 1.0], (0.0, 100.0)),
        (
            "arenstorf",
            4,
            aren,
            ARENSTORF_Y0.to_vec(),
            (0.0, ARENSTORF_PERIOD),
        ),
        ("lorenz96_40", 40, l96, lorenz96_y0(40), (0.0, 10.0)),
        (
            "damped_osc",
            2,
            damped_oscillator,
            vec![1.0, 0.0],
            (0.0, 10.0),
        ),
        ("nonnormal", 2, nonnormal, vec![1.0, 1.0], (0.0, 10.0)),
    ]
}

fn pilot_sums(solution: &VigilodeSolution) -> (usize, u64, f64) {
    solution.pilots.iter().fold((0, 0, 0.0), |(a, c, o), p| {
        (
            a + p.pilot_attempts,
            c + p.pilot_callbacks,
            o + p.pilot_operations,
        )
    })
}

#[test]
fn break_even_length_is_the_table_of_the_design() {
    let n = [1usize, 2, 3, 4, 8, 10, 28, 40, 64, 100, 1000];
    let b: Vec<usize> = n.iter().map(|&n| vigilode::break_even_length(n)).collect();
    record(
        "break_even_length_is_the_table_of_the_design",
        json!({ "n": n, "b": b }),
    );
    assert_eq!(b, [3, 4, 4, 5, 7, 8, 23, 37, 78, 166, 13170]);
    for n in 1..=2000u64 {
        assert_eq!(
            vigilode::break_even_length(n as usize) as u64,
            break_even(n)
        );
        // the operation unit binds: c_R <= b c_D in callbacks (n + 11 <= 6 b) as well
        assert!(n + 11 <= 6 * break_even(n));
    }
}

#[test]
fn u1_a_run_without_handover_is_dopri5_solve_plus_its_pilots_within_twice_its_cost() {
    let test = "u1_a_run_without_handover_is_dopri5_solve_plus_its_pilots_within_twice_its_cost";
    let mut rows = Vec::new();
    let mut lost_pilots = 0usize;
    for (name, n, f, y0, span) in non_stiff_cases() {
        let log: Log = Arc::default();
        let direct_calls = Calls::default();
        let direct = Dopri5::solve(
            &logged(n, &direct_calls, &log, f),
            span,
            &y0,
            &Options::default(),
        )
        .expect("direct DOPRI5");
        let calls = Calls::default();
        let unified = VigilODE::solve(&logged(n, &calls, &log, f), span, &y0, SolverStrategy::Auto)
            .expect("unified solve");
        let (cost, reference) = (unified_cost(n, &unified), dopri5_cost(n, &direct));
        let (pilot_attempts, pilot_callbacks, pilot_operations) = pilot_sums(&unified);
        let b = vigilode::break_even_length(n);
        let explicit = unified
            .explicit_solution
            .as_ref()
            .expect("the DOPRI5 record");
        let equal_record = serde_json::to_value(&explicit.stats).unwrap()
            == serde_json::to_value(&direct.stats).unwrap();
        rows.push(json!({
            "case": name, "b": b, "pilots": unified.pilots.len(),
            "verdicts": unified.pilots.iter().map(|p| format!("{:?}", p.verdict)).collect::<Vec<_>>(),
            "unified": cost.json(), "dopri5": reference.json(),
            "pilot_callbacks": pilot_callbacks, "pilot_operations": pilot_operations,
            "bits_equal": bits(&unified.final_state) == bits(&direct.final_state),
            "dopri5_record_equal": equal_record,
        }));
        assert!(unified.success() && direct.success, "{name}");
        assert!(!has_rodas5p_leg(&unified), "{name}: handed over");
        lost_pilots += unified.pilots.len();
        // the trajectory, the bits and the DOPRI5 record of Dopri5::solve
        assert_eq!(
            bits(&unified.final_state),
            bits(&direct.final_state),
            "{name}"
        );
        assert!(
            equal_record,
            "{name}: the DOPRI5 record differs from Dopri5::solve"
        );
        // plus exactly the pilots' work
        assert_eq!(
            unified.stats.attempts,
            direct.stats.attempts + pilot_attempts,
            "{name}"
        );
        assert_eq!(
            cost.callbacks,
            reference.callbacks + pilot_callbacks,
            "{name}"
        );
        assert_eq!(calls.rhs(), cost.callbacks, "{name}: uncharged callbacks");
        assert!(
            unified
                .pilots
                .iter()
                .all(|p| p.verdict == vigilode::PilotVerdict::Lost),
            "{name}"
        );
        // at most twice the cost of Dopri5::solve in each unit (U1), and the pilots'
        // number and spend of E10
        assert!(cost.callbacks <= 2 * reference.callbacks, "{name}");
        assert!(cost.ops3 <= 2 * reference.ops3, "{name}");
        let a_d = direct.stats.attempts as u64;
        assert!(
            unified.pilots.len() as u64 <= max_pilots(a_d, b as u64),
            "{name}: {} pilots after {a_d} DOPRI5 attempts",
            unified.pilots.len()
        );
        assert!(pilot_callbacks <= reference.callbacks, "{name}");
        assert!(pilot_operations <= reference.ops3 as f64 / 3.0, "{name}");
    }
    record(test, json!({ "cases": rows, "lost_pilots": lost_pilots }));
    assert!(
        lost_pilots > 0,
        "no case made a pilot: the test shows nothing"
    );
}

#[test]
fn u2_dopri5_pilots_and_rodas5p_share_one_attempt_budget() {
    let test = "u2_dopri5_pilots_and_rodas5p_share_one_attempt_budget";
    let mut rows = Vec::new();
    for max_steps in [100_000usize, 500, 50, 36, 35, 20, 9, 8, 5, 1] {
        let calls = Calls::default();
        let unified = VigilODE::solve(
            &robertson_fd(&calls),
            (0.0, 40.0),
            &[1.0, 0.0, 0.0],
            VigilodeOptions::default().max_steps(max_steps),
        )
        .expect("unified solve");
        let counters = &unified.stats.counters;
        let in_counters = counters.accepted_steps + counters.rejected_steps;
        let legs: usize = unified.legs.iter().map(|leg| leg.attempts).sum();
        let (pilot_attempts, _, _) = pilot_sums(&unified);
        let accepted_pilot = unified
            .pilots
            .last()
            .filter(|p| p.verdict == vigilode::PilotVerdict::Accepted)
            .map_or(0, |p| p.pilot_attempts);
        rows.push(json!({
            "max_steps": max_steps, "success": unified.success(),
            "status": format!("{:?}", unified.status), "attempts": unified.stats.attempts,
            "leg_attempts": legs, "pilot_attempts": pilot_attempts, "pilots": unified.pilots.len(),
            "legs": leg_summary(&unified),
        }));
        assert!(unified.stats.attempts <= max_steps, "{max_steps}");
        assert_eq!(unified.stats.attempts as u64, in_counters, "{max_steps}");
        // every attempt is in a leg or in a pilot that is not part of a leg
        assert_eq!(
            unified.stats.attempts,
            legs + pilot_attempts - accepted_pilot,
            "{max_steps}"
        );
        assert_eq!(counters.rhs_calls, calls.rhs(), "{max_steps}");
        if !unified.success() {
            assert_eq!(unified.stats.attempts, max_steps, "{max_steps}");
            assert_eq!(
                unified.status,
                Status::Failed(FailureReason::MaxStepsReached { max_steps }),
                "{max_steps}"
            );
        }
    }
    record(test, json!({ "cases": rows }));
}

/// Check the entry sequence of a unified run against the one of `Dopri5::solve`: the
/// unified sequence is DOPRI5's with the pilots' blocks inserted, up to the handover; after
/// an accepted pilot every entry is at or after the pilot's end. Returns the number of
/// entries in pilot blocks.
fn check_entries(
    name: &str,
    direct: &[(f64, Vec<f64>)],
    unified: &[(f64, Vec<f64>)],
    solution: &VigilodeSolution,
) -> usize {
    let same = |a: &(f64, Vec<f64>), b: &(f64, Vec<f64>)| {
        a.0.to_bits() == b.0.to_bits() && bits(&a.1) == bits(&b.1)
    };
    let (t0, y0) = (solution.stats.t_span.0, &solution.stats.y0);
    assert!(
        same(&unified[0], &(t0, y0.clone())),
        "{name}: the first entry is not DOPRI5's first stage at (t0, y0)"
    );
    let at_t0: Vec<_> = unified.iter().filter(|(t, _)| *t == t0).collect();
    assert!(
        at_t0.iter().all(|(_, y)| bits(y) == bits(y0)),
        "{name}: an entry at t0 that is not DOPRI5's first stage"
    );
    let (mut i, mut j, mut in_pilots) = (0usize, 0usize, 0usize);
    let mut pilots = solution.pilots.iter();
    while i < unified.len() {
        if j < direct.len() && same(&unified[i], &direct[j]) {
            i += 1;
            j += 1;
            continue;
        }
        let pilot = pilots
            .next()
            .unwrap_or_else(|| panic!("{name}: entry {i} is neither DOPRI5's nor a pilot's"));
        let block = &unified[i..i + pilot.pilot_callbacks as usize];
        assert!(
            block
                .iter()
                .all(|(t, _)| *t >= pilot.t && *t <= pilot.t + pilot.h),
            "{name}: a pilot entry outside [t_D, t_D + H]"
        );
        assert!(
            block
                .iter()
                .any(|(t, y)| t.to_bits() == pilot.t.to_bits() && y.len() == y0.len()),
            "{name}: the pilot made no call at its start t_D"
        );
        in_pilots += block.len();
        i += block.len();
        if pilot.verdict == vigilode::PilotVerdict::Accepted {
            let end = pilot.t + pilot.h;
            assert!(
                unified[i..].iter().all(|(t, _)| *t >= end * (1.0 - 1e-15)),
                "{name}: an entry before the end of the accepted pilot"
            );
            return in_pilots;
        }
    }
    assert_eq!(j, direct.len(), "{name}: DOPRI5 entries are missing");
    assert!(pilots.next().is_none(), "{name}: a pilot made no entry");
    in_pilots
}

#[test]
fn u3_no_callback_before_dopri5s_first_stage_and_every_other_entry_is_a_pilots() {
    let test = "u3_no_callback_before_dopri5s_first_stage_and_every_other_entry_is_a_pilots";
    let mut cases = non_stiff_cases();
    cases.push((
        "robertson_40",
        3,
        robertson_rhs,
        vec![1.0, 0.0, 0.0],
        (0.0, 40.0),
    ));
    let mut rows = Vec::new();
    for (name, n, f, y0, span) in cases {
        let (direct_log, log): (Log, Log) = (Arc::default(), Arc::default());
        Dopri5::solve(
            &logged(n, &Calls::default(), &direct_log, f),
            span,
            &y0,
            &Options::default(),
        )
        .expect("direct DOPRI5");
        let calls = Calls::default();
        let unified = VigilODE::solve(&logged(n, &calls, &log, f), span, &y0, SolverStrategy::Auto)
            .expect("unified solve");
        let (direct_log, log) = (direct_log.lock().unwrap(), log.lock().unwrap());
        let in_pilots = check_entries(name, &direct_log, &log, &unified);
        rows.push(json!({
            "case": name, "entries": log.len(), "in_pilot_blocks": in_pilots,
            "pilots": unified.pilots.len(), "charged": unified.stats.counters.rhs_calls,
        }));
        assert_eq!(
            log.len() as u64,
            unified.stats.counters.rhs_calls,
            "{name}: uncharged"
        );
    }
    // F2: no entry above 1, so no fatal error
    let calls = Calls::default();
    let solution = VigilODE::solve(&f2(&calls), (0.0, 1.0), &[1.0], SolverStrategy::Auto).unwrap();
    rows.push(
        json!({ "case": "f2", "fatal_entries": calls.fatal(), "success": solution.success() }),
    );
    record(test, json!({ "cases": rows }));
    assert!(solution.success() && calls.fatal() == 0);
}

fn heat_fd(n: usize, calls: &Calls) -> Problem {
    let rhs = calls.rhs.clone();
    Problem::new(n, move |_t, y: &[f64], dy: &mut [f64]| {
        rhs.fetch_add(1, Ordering::Relaxed);
        let inv_dx2 = ((n + 1) as f64).powi(2);
        for i in 0..n {
            let left = if i == 0 { 0.0 } else { y[i - 1] };
            let right = if i + 1 == n { 0.0 } else { y[i + 1] };
            dy[i] = (left - 2.0 * y[i] + right) * inv_dx2;
        }
        Ok::<(), CallbackError>(())
    })
    .autonomous()
}

fn heat_sine(n: usize) -> Vec<f64> {
    (1..=n)
        .map(|i| (std::f64::consts::PI * i as f64 / (n + 1) as f64).sin())
        .collect()
}

#[test]
fn u4_a_handover_follows_an_accepted_pilot_and_is_recorded() {
    let test = "u4_a_handover_follows_an_accepted_pilot_and_is_recorded";
    type Case = (
        &'static str,
        usize,
        Box<dyn Fn(&Calls) -> Problem>,
        Vec<f64>,
        (f64, f64),
    );
    let cases: Vec<Case> = vec![
        (
            "robertson_40",
            3,
            Box::new(robertson_fd),
            vec![1.0, 0.0, 0.0],
            (0.0, 40.0),
        ),
        (
            "heat_fd_8",
            8,
            Box::new(|c| heat_fd(8, c)),
            heat_sine(8),
            (0.0, 100.0),
        ),
    ];
    let mut rows = Vec::new();
    for (name, n, build, y0, span) in cases {
        let calls = Calls::default();
        let unified = VigilODE::solve(&build(&calls), span, &y0, SolverStrategy::Auto).unwrap();
        let pilot = unified.pilots.last().expect("a pilot").clone();
        let b = vigilode::break_even_length(n);
        let inner = unified.inner_solution.as_ref().expect("the RODAS5P run");
        rows.push(json!({
            "case": name, "pilots": serde_json::to_value(&unified.pilots).unwrap(),
            "legs": leg_summary(&unified),
            "history": serde_json::to_value(&unified.route_history).unwrap(),
            "fallback_steps": unified.stats.counters.fallback_steps,
            "callbacks": unified.stats.counters.rhs_calls,
        }));
        assert!(unified.success(), "{name}");
        assert_eq!(pilot.verdict, vigilode::PilotVerdict::Accepted, "{name}");
        assert!(
            unified.pilots[..unified.pilots.len() - 1]
                .iter()
                .all(|p| p.verdict == vigilode::PilotVerdict::Lost)
        );
        // the evidence: one RODAS5P attempt covered H after b DOPRI5 attempts that cost at
        // least as much in every unit
        assert_eq!(pilot.window_attempts, b, "{name}");
        assert_eq!(pilot.pilot_attempts, 1, "{name}");
        assert!(pilot.h > 0.0 && pilot.t > span.0, "{name}");
        assert!(pilot.window_callbacks >= pilot.pilot_callbacks, "{name}");
        assert!(pilot.window_operations >= pilot.pilot_operations, "{name}");
        assert!(pilot.dopri5_attempts >= b, "{name}");
        // the legs: DOPRI5 to t_D, then RODAS5P from t_D starting with the pilot's step
        assert_eq!(unified.legs.len(), 2, "{name}");
        let (dopri5, rodas5p) = (&unified.legs[0], &unified.legs[1]);
        assert_eq!(dopri5.tier, SelectedTier::Tier1Explicit);
        assert_eq!(dopri5.t_end.to_bits(), pilot.t.to_bits(), "{name}");
        assert_eq!(dopri5.attempts, pilot.dopri5_attempts, "{name}");
        assert_eq!(rodas5p.tier, SelectedTier::TierProtectedReference);
        assert_eq!(rodas5p.t_start.to_bits(), pilot.t.to_bits(), "{name}");
        assert!(inner.t[0] > pilot.t && inner.t[0] <= pilot.t + pilot.h * (1.0 + 1e-15));
        assert_eq!(
            inner.stats.options.h0.to_bits(),
            pilot.h.to_bits(),
            "{name}: h0 = H"
        );
        // the route history records the handover
        assert_eq!(unified.route_history.len(), 2, "{name}");
        let entry = &unified.route_history[1];
        assert_eq!(entry.time.to_bits(), pilot.t.to_bits(), "{name}");
        assert!(entry.reason.starts_with("handover"), "{}", entry.reason);
        assert_eq!(unified.stats.counters.fallback_steps, 1, "{name}");
        assert_eq!(unified.stats.counters.rhs_calls, calls.rhs(), "{name}");
    }
    record(test, json!({ "cases": rows }));
}

#[test]
fn u5_same_inputs_give_the_same_route_counters_and_bits() {
    let test = "u5_same_inputs_give_the_same_route_counters_and_bits";
    let digest = |solution: &VigilodeSolution| {
        json!({
            "legs": serde_json::to_value(&solution.legs).unwrap(),
            "pilots": serde_json::to_value(&solution.pilots).unwrap(),
            "history": serde_json::to_value(&solution.route_history).unwrap(),
            "counters": serde_json::to_value(solution.stats.counters).unwrap(),
            "status": serde_json::to_value(&solution.status).unwrap(),
            "final_time": solution.final_time.to_bits(),
            "final_state": bits(&solution.final_state),
        })
    };
    let mut rows = Vec::new();
    for (name, n, f, y0, span) in non_stiff_cases().into_iter().chain([(
        "robertson_40",
        3,
        robertson_rhs as Rhs,
        vec![1.0, 0.0, 0.0],
        (0.0, 40.0),
    )]) {
        let run = || {
            let log: Log = Arc::default();
            let solution = VigilODE::solve(
                &logged(n, &Calls::default(), &log, f),
                span,
                &y0,
                SolverStrategy::Auto,
            )
            .unwrap();
            digest(&solution)
        };
        let (first, second) = (run(), run());
        rows.push(json!({ "case": name, "equal": first == second }));
        assert_eq!(first, second, "{name}");
    }
    record(test, json!({ "cases": rows }));
}

#[test]
fn d1_a_fatal_callback_inside_a_pilot_ends_the_solve_with_the_accepted_prefix() {
    let test = "d1_a_fatal_callback_inside_a_pilot_ends_the_solve_with_the_accepted_prefix";
    // N2: y1' = 0, y2' = -1000 y2, y0 = (1, 1); fatal whenever y1 > 1, which only a
    // finite-difference column reaches
    let calls = Calls::default();
    let (rhs, fatal) = (calls.rhs.clone(), calls.fatal.clone());
    let problem = Problem::new(2, move |_t, y: &[f64], dy: &mut [f64]| {
        rhs.fetch_add(1, Ordering::Relaxed);
        if y[0] > 1.0 {
            fatal.fetch_add(1, Ordering::Relaxed);
            return Err(CallbackError::Fatal("N2: y1 left [0, 1]".into()));
        }
        dy[0] = 0.0;
        dy[1] = -1000.0 * y[1];
        Ok(())
    })
    .autonomous();
    let unified = VigilODE::solve(&problem, (0.0, 1.0), &[1.0, 1.0], SolverStrategy::Auto).unwrap();
    record(
        test,
        json!({
            "status": format!("{:?}", unified.status),
            "stop": serde_json::to_value(&unified.stop).unwrap(),
            "pilots": serde_json::to_value(&unified.pilots).unwrap(),
            "legs": leg_summary(&unified),
            "final_time": unified.final_time,
            "fatal_entries": calls.fatal(),
            "rhs_entries": calls.rhs(),
            "rhs_charged": unified.stats.counters.rhs_calls,
        }),
    );
    assert!(!unified.success());
    let pilot = unified.pilots.last().expect("a pilot");
    assert_eq!(unified.pilots.len(), 1, "the first pilot ends the solve");
    assert_eq!(pilot.verdict, vigilode::PilotVerdict::Fatal);
    match &unified.status {
        Status::Failed(FailureReason::CallbackFailed {
            callback, message, ..
        }) => {
            assert_eq!(*callback, vigilode::Callback::Rhs);
            assert!(message.contains("N2"), "{message}");
        }
        other => panic!("expected the callback failure, got {other:?}"),
    }
    // the accepted prefix: DOPRI5 up to the pilot's start
    assert_eq!(unified.legs.len(), 1);
    assert_eq!(unified.final_time.to_bits(), pilot.t.to_bits());
    assert!(unified.final_time > 0.0);
    assert_eq!(calls.fatal(), 1, "a fatal callback is not called again");
    assert_eq!(
        unified.stats.counters.rhs_calls,
        calls.rhs(),
        "the failing call is charged"
    );
}
