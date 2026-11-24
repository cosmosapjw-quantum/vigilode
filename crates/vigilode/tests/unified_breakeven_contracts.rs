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
