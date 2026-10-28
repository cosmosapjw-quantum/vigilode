//! Bounded native RVJ trace campaign. JSON on stdout preserves exact state bits.
//! Point/box enclosures are native. Whole-path Hermite/Bernstein certification
//! is performed separately on these same bits by the independent exact oracle.

use rodas5p_core::{CoreError, CoreResult, directed};
use serde::Serialize;
use serde_json::{Value, json};
use std::{hint::black_box, time::Instant};
use vigilode_coupled::trace_quotient::{
    QuarticInvariant, QuarticParameters, TRACE_TARGET_ID, TraceFieldRoute, TraceSemantics,
    TraceWork, quartic_field, quartic_field_enclosure, quartic_invariant, quartic_moments,
};

const ALL: TraceSemantics = TraceSemantics::AllComplexRootsAlgebraicMultiplicity;
const ROUTES: [TraceFieldRoute; 2] = [
    TraceFieldRoute::DirectMoments,
    TraceFieldRoute::SharedPolynomial,
];
// Resource choices for one registered campaign, not mathematical constants.
const SLABS: [usize; 5] = [16, 32, 64, 128, 256];
const REPEATS: usize = 7;
const TIMING_UNITS: usize = 16;
const TIMING_SLABS: usize = 128;

#[derive(Clone, Debug, Serialize)]
struct StateRecord {
    t: f64,
    t_bits: u64,
    state: [f64; 2],
    state_bits: [u64; 2],
}

fn state_record(t: f64, state: [f64; 2]) -> StateRecord {
    StateRecord {
        t,
        t_bits: t.to_bits(),
        state,
        state_bits: state.map(f64::to_bits),
    }
}

fn reference(t: f64) -> f64 {
    let t2 = t * t;
    let t4 = t2 * t2;
    3.0 / 4.0 + t / 8.0 + t4 * t / 64.0
}

fn reference_derivative(t: f64) -> f64 {
    let t2 = t * t;
    1.0 / 8.0 + 5.0 * (t2 * t2) / 64.0
}

fn rhs(
    t: f64,
    state: [f64; 2],
    route: TraceFieldRoute,
    work: &mut TraceWork,
) -> CoreResult<[f64; 2]> {
    let g = quartic_field(ALL, state, route, work)?;
    let p = reference(t);
    // Identical represented forcing for both candidate routes. Its direct
    // moment evaluation is charged on every RHS call, never precomputed away.
    let control = quartic_field(ALL, [p, p], TraceFieldRoute::DirectMoments, work)?;
    let dp = reference_derivative(t);
    Ok([dp + g[0] - control[0], dp + g[1] - control[1]])
}

fn shifted(state: [f64; 2], derivative: [f64; 2], factor: f64) -> [f64; 2] {
    [
        state[0] + factor * derivative[0],
        state[1] + factor * derivative[1],
    ]
}

fn integrate(
    slabs: usize,
    route: TraceFieldRoute,
    work: &mut TraceWork,
) -> CoreResult<Vec<StateRecord>> {
    if !SLABS.contains(&slabs) {
        return Err(CoreError::InvalidInput(
            "unregistered trace campaign grid".into(),
        ));
    }
    // Every node and midpoint is an exact dyadic on this finite grid.
    // The p/p' reference values here also fit exactly in binary64: on the
    // finest midpoint grid |t|=|k|/256 and p has denominator at most 2^46.
    let h = 2.0 / slabs as f64;
    let mut state = [39.0 / 64.0; 2];
    let mut states = Vec::with_capacity(slabs + 1);
    states.push(state_record(-1.0, state));
    for k in 0..slabs {
        let t = -1.0 + k as f64 * h;
        let k1 = rhs(t, state, route, work)?;
        let k2 = rhs(t + h / 2.0, shifted(state, k1, h / 2.0), route, work)?;
        let k3 = rhs(t + h / 2.0, shifted(state, k2, h / 2.0), route, work)?;
        let k4 = rhs(t + h, shifted(state, k3, h), route, work)?;
        for i in 0..2 {
            state[i] += h * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]) / 6.0;
        }
        if state.iter().any(|value| !value.is_finite()) {
            return Err(CoreError::NonFinite(
                "trace RK4 state became non-finite".into(),
            ));
        }
        states.push(state_record(-1.0 + (k + 1) as f64 * h, state));
    }
    Ok(states)
}

fn node_error(states: &[StateRecord]) -> (f64, f64) {
    let mut maximum: f64 = 0.0;
    let mut terminal = 0.0;
    for node in states {
        let exact = reference(node.t);
        terminal = node
            .state
            .iter()
            .map(|value| (value - exact).abs())
            .fold(0.0, f64::max);
        maximum = maximum.max(terminal);
    }
    (maximum, terminal)
}

fn node_field_enclosures(states: &[StateRecord], route: TraceFieldRoute) -> Value {
    let start = Instant::now();
    let mut work = TraceWork::default();
    let mut maximum_width: f64 = 0.0;
    let result: CoreResult<()> = (|| {
        for node in states {
            let parameters = QuarticParameters::point(node.state[0], node.state[1])?;
            let enclosed = quartic_field_enclosure(ALL, parameters, route, &mut work)?;
            for interval in enclosed {
                maximum_width = maximum_width.max(directed::sub_up(interval.hi, interval.lo)?);
            }
        }
        Ok(())
    })();
    json!({
        "status": if result.is_ok() { "completed" } else { "rejected" },
        "failure": result.err().map(|error| error.to_string()),
        "scope": "G at represented nodes; no whole-path or controlled-F residual claim",
        "seconds": start.elapsed().as_secs_f64(),
        "maximum_component_width": maximum_width,
        "work": work,
    })
}

fn interval_cases() -> CoreResult<Vec<Value>> {
    let mut result = Vec::new();
    for (name, a, b) in [
        ("ordinary", 0.5_f64, 0.75_f64),
        ("triple", 0.75, 0.75),
        ("complex", -0.25, -0.25),
        ("regular_branch_counterexample", 2.0, 2.0),
    ] {
        let parameters = QuarticParameters::point(a, b)?;
        let mut work = TraceWork::default();
        let moments = quartic_moments(ALL, parameters, &mut work)?;
        let shared = quartic_field_enclosure(ALL, parameters, ROUTES[1], &mut work)?;
        let direct = quartic_field_enclosure(ALL, parameters, ROUTES[0], &mut work)?;
        let constraint = quartic_invariant(
            ALL,
            parameters,
            QuarticInvariant::Q1SquaredPlusQ2,
            &mut work,
        )?;
        let product = quartic_invariant(ALL, parameters, QuarticInvariant::Q1Q2, &mut work)?;
        result.push(json!({
            "name": name,
            "a_bits": a.to_bits(), "b_bits": b.to_bits(),
            "moments": moments.map(|value| value.as_pair()),
            "field": shared.map(|value| value.as_pair()),
            "field_direct": direct.map(|value| value.as_pair()),
            "invariant_q1_squared_plus_q2": constraint.as_pair(),
            "invariant_q1_q2": product.as_pair(),
            "work": work,
        }));
    }
    Ok(result)
}

fn semantics_checks() -> CoreResult<Vec<Value>> {
    let parameters = QuarticParameters::point(2.0, 2.0)?;
    let mut result = Vec::new();
    for (query, semantics, expected) in [
        ("all_complex_multiplicity", ALL, "accepted"),
        ("selected_branch", TraceSemantics::SelectedReal, "rejected"),
        ("real_only", TraceSemantics::RealOnly, "rejected"),
    ] {
        let mut work = TraceWork::default();
        let value = quartic_invariant(semantics, parameters, QuarticInvariant::Q1, &mut work);
        let actual = if value.is_ok() {
            "accepted"
        } else {
            "rejected"
        };
        result.push(json!({
            "query": query, "expected": expected, "actual": actual,
            "value": value.as_ref().ok().map(|value| value.as_pair()),
            "failure": value.err().map(|error| error.to_string()),
            "selected_regular_branch_q1": 1,
            "work": work,
        }));
    }
    Ok(result)
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    CandidateField,
    IntervalField,
    CombinedField,
    CandidateTrajectory,
}

const PHASES: [Phase; 4] = [
    Phase::CandidateField,
    Phase::IntervalField,
    Phase::CombinedField,
    Phase::CandidateTrajectory,
];

fn timing_block(
    phase: Phase,
    route: TraceFieldRoute,
    states: &[StateRecord],
    work: &mut TraceWork,
) -> CoreResult<()> {
    for _ in 0..TIMING_UNITS {
        if matches!(phase, Phase::CandidateTrajectory) {
            black_box(integrate(TIMING_SLABS, route, work)?);
            continue;
        }
        for node in states {
            let state = black_box(node.state);
            if matches!(phase, Phase::CandidateField | Phase::CombinedField) {
                black_box(quartic_field(ALL, state, route, work)?);
            }
            if matches!(phase, Phase::IntervalField | Phase::CombinedField) {
                let parameters = QuarticParameters::point(state[0], state[1])?;
                black_box(quartic_field_enclosure(ALL, parameters, route, work)?);
            }
        }
    }
    Ok(())
}

fn timings(states: &[StateRecord]) -> Value {
    let mut warmups = Vec::new();
    for phase in PHASES {
        for route in ROUTES {
            let mut work = TraceWork::default();
            let start = Instant::now();
            let result = timing_block(phase, route, states, &mut work);
            warmups.push(json!({
                "phase": phase, "route": route, "seconds": start.elapsed().as_secs_f64(),
                "status": if result.is_ok() { "completed" } else { "rejected" },
                "failure": result.err().map(|error| error.to_string()), "work": work,
            }));
        }
    }
    let mut samples = Vec::new();
    for repeat in 0..REPEATS {
        let order = if repeat % 2 == 0 {
            ROUTES
        } else {
            [ROUTES[1], ROUTES[0]]
        };
        for phase in PHASES {
            for (order_position, route) in order.iter().copied().enumerate() {
                let mut work = TraceWork::default();
                let start = Instant::now();
                let result = timing_block(phase, route, states, &mut work);
                samples.push(json!({
                    "repeat": repeat, "order_position": order_position,
                    "phase": phase, "route": route,
                    "seconds": start.elapsed().as_secs_f64(),
                    "status": if result.is_ok() { "completed" } else { "rejected" },
                    "failure": result.err().map(|error| error.to_string()),
                    "work": work,
                }));
            }
        }
    }
    json!({
        "warmup_blocks_per_phase_route": 1,
        "repeat_count": REPEATS,
        "units_per_block": TIMING_UNITS,
        "trajectory_slabs": TIMING_SLABS,
        "field_states_per_unit": states.len(),
        "field_dataset": "direct_moments_N128.states (same represented inputs for both routes)",
        "order": "alternate direct/shared and shared/direct by repeat; same order in each phase",
        "resource_choices_are_mathematical_constants": false,
        "includes": "API semantics and finite checks, actual arithmetic, caller-owned counters; interval phases include parameter construction; trajectory includes forcing, RK4 updates, state/bit allocation and destruction",
        "excludes": "diagnostic node oracle, JSON serialization, retained input dataset construction, independent exact whole-path oracle",
        "excludes_exact_oracle": true,
        "root_enumeration_baseline_exists": false,
        "warmups": warmups, "samples": samples,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();
    let cases = interval_cases()?;
    let semantics = semantics_checks()?;
    let mut runs = Vec::new();
    let mut timing_states = None;
    let mut all_completed = true;
    for slabs in SLABS {
        for route in ROUTES {
            let mut work = TraceWork::default();
            let begin = Instant::now();
            let result = integrate(slabs, route, &mut work);
            let integration_seconds = begin.elapsed().as_secs_f64();
            match result {
                Ok(states) => {
                    let (maximum, terminal) = node_error(&states);
                    let point_certificate = node_field_enclosures(&states, route);
                    if point_certificate["status"] != "completed" {
                        all_completed = false;
                    }
                    if slabs == TIMING_SLABS && route == TraceFieldRoute::DirectMoments {
                        timing_states = Some(states.clone());
                    }
                    runs.push(json!({
                        "status": "completed", "route": route, "slabs": slabs,
                        "states": states,
                        "max_node_error_inf": maximum, "terminal_error_inf": terminal,
                        "rhs_calls": 4 * slabs,
                        "forcing_route": "direct_moments for both state routes",
                        "work": work, "integration_seconds": integration_seconds,
                        "node_field_enclosure": point_certificate,
                        "whole_path_certified_natively": false,
                    }));
                }
                Err(error) => {
                    all_completed = false;
                    runs.push(json!({
                        "status": "rejected", "route": route, "slabs": slabs,
                        "failure": error.to_string(), "work": work,
                        "integration_seconds": integration_seconds,
                    }));
                }
            }
        }
    }
    let preparation_seconds = start.elapsed().as_secs_f64();
    let timing = match timing_states {
        Some(states) => timings(&states),
        None => json!({"status": "not_run", "reason": "the registered timing dataset failed"}),
    };
    let output = json!({
        "schema_version": 1, "task_id": "RVJ-TRACE-20261028",
        "target_id": TRACE_TARGET_ID,
        "semantics": "all_complex_roots_algebraic_multiplicity",
        "all_requested_trajectories_completed": all_completed,
        "grid": SLABS,
        "clock": "t=-1+2*k/N; exact dyadic nodes/midpoints on registered grids",
        "reference": "p(t)=3/4+t/8+t^5/64; exact node reference on registered dyadic grids",
        "native_certificate_scope": "point or parameter-box all-root observable/aggregate field only",
        "whole_path_scope": "separate independent exact-rational Hermite/Bernstein oracle on the exported native state bits",
        "production_solver_admission": false,
        "interval_cases": cases, "semantics_checks": semantics, "runs": runs,
        "diagnostic_preparation_seconds": preparation_seconds,
        "timing": timing,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
