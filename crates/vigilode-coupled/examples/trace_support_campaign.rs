//! Requested-support versus the frozen PR97 eager polynomial trace evaluator.
//! This is a complete query-call comparison, not an ODE trajectory benchmark.

use rodas5p_core::{CoreError, CoreResult, directed::Interval};
use serde_json::{Value, json};
use std::{hint::black_box, time::Instant};
use vigilode_coupled::trace_quotient::{
    QuarticParameters, TRACE_TARGET_ID, TraceSemantics, TraceWork, quartic_polynomial_trace,
};

const ALL: TraceSemantics = TraceSemantics::AllComplexRootsAlgebraicMultiplicity;
const REPEATS: usize = 7;
const ITERATIONS: usize = 128;
const ORDER_SEED: u64 = 20_261_028;
const BASELINE_SOURCE: &str = "87465835d9d21298d235917f036580b0b53ab645";
const POINTS: [(i128, i128); 16] = [
    (-1, -1),
    (2, 2),
    (2, 3),
    (2, 4),
    (3, 2),
    (3, 3),
    (3, 4),
    (4, 2),
    (4, 3),
    (4, 4),
    (5, 2),
    (5, 3),
    (5, 4),
    (2, 5),
    (3, 5),
    (8, 8),
];

#[derive(Clone, Copy)]
enum Route {
    Eager,
    Support,
}

impl Route {
    fn name(self) -> &'static str {
        match self {
            Self::Eager => "pr97_eager",
            Self::Support => "requested_support",
        }
    }
}

// Private helper used only with the finite exactly dyadic literals below.
const fn constant(value: f64) -> Interval {
    Interval {
        lo: value,
        hi: value,
    }
}

fn charge(counter: &mut u64) -> CoreResult<()> {
    *counter = counter
        .checked_add(1)
        .ok_or_else(|| CoreError::InvalidInput("trace work counter exhausted".into()))?;
    Ok(())
}

fn add(a: Interval, b: Interval, work: &mut TraceWork) -> CoreResult<Interval> {
    charge(&mut work.interval_additions)?;
    a.add(b)
}

fn mul(a: Interval, b: Interval, work: &mut TraceWork) -> CoreResult<Interval> {
    charge(&mut work.interval_multiplications)?;
    a.mul(b)
}

// Frozen operation order of PR97 quartic_polynomial_trace/all_moments and its
// even_moments dependency. There is one request boundary, as in the old API;
// calling today's public quartic_moments would add a second request boundary.
#[inline(never)]
fn eager_polynomial_trace(
    semantics: TraceSemantics,
    parameters: QuarticParameters,
    coefficients: &[Interval],
    work: &mut TraceWork,
) -> CoreResult<Interval> {
    charge(&mut work.requests_attempted)?;
    let result: CoreResult<Interval> = (|| {
        if semantics != ALL {
            charge(&mut work.semantics_rejections)?;
            return Err(CoreError::InvalidInput(
                "quartic trace requires all complex roots with algebraic multiplicity".into(),
            ));
        }
        charge(&mut work.polynomial_queries)?;
        if coefficients.len() > 7 {
            return Err(CoreError::InvalidInput(
                "quartic polynomial trace supports degrees zero through six".into(),
            ));
        }
        for coefficient in coefficients {
            Interval::new(coefficient.lo, coefficient.hi)?;
        }
        if coefficients.is_empty() {
            return Ok(constant(0.0));
        }
        if coefficients.len() <= 2 {
            return mul(coefficients[0], constant(4.0), work);
        }
        charge(&mut work.moment_batches)?;
        let a = parameters.a();
        let b = parameters.b();
        let a2 = mul(a, a, work)?;
        let s2 = mul(a, constant(4.0), work)?;
        let a2_plus_b = add(a2, b, work)?;
        let s4 = mul(a2_plus_b, constant(4.0), work)?;
        let a3 = mul(a2, a, work)?;
        let ab = mul(a, b, work)?;
        let four_a3 = mul(a3, constant(4.0), work)?;
        let twelve_ab = mul(ab, constant(12.0), work)?;
        let s6_ab = add(four_a3, twelve_ab, work)?;
        let s6 = add(s6_ab, constant(3.0), work)?;
        let s5 = mul(a, constant(-10.0), work)?;
        let moments = [constant(4.0), constant(0.0), s2, constant(-3.0), s4, s5, s6];
        let mut result = constant(0.0);
        for (&coefficient, &moment) in coefficients.iter().zip(&moments) {
            let term = mul(coefficient, moment, work)?;
            result = add(result, term, work)?;
        }
        Ok(result)
    })();
    if result.is_ok() {
        charge(&mut work.requests_completed)?;
    } else {
        charge(&mut work.requests_rejected)?;
    }
    result
}

#[inline(never)]
fn support_polynomial_trace(
    semantics: TraceSemantics,
    parameters: QuarticParameters,
    coefficients: &[Interval],
    work: &mut TraceWork,
) -> CoreResult<Interval> {
    quartic_polynomial_trace(semantics, parameters, coefficients, work)
}

#[derive(Clone)]
struct Query {
    name: &'static str,
    state: [f64; 2],
    coefficients: [f64; 7],
    length: usize,
}

fn evaluate(query: &Query, route: Route, work: &mut TraceWork) -> CoreResult<Interval> {
    // Parameter and coefficient construction are identical and inside every
    // timed call. No compiled-support plan or heap allocation is hidden here.
    let parameters = QuarticParameters::point(query.state[0], query.state[1])?;
    let mut coefficients = [constant(0.0); 7];
    for (destination, &value) in coefficients.iter_mut().zip(&query.coefficients) {
        *destination = Interval::point(value)?;
    }
    match route {
        Route::Eager => {
            eager_polynomial_trace(ALL, parameters, &coefficients[..query.length], work)
        }
        Route::Support => {
            support_polynomial_trace(ALL, parameters, &coefficients[..query.length], work)
        }
    }
}

// Exact integer matrix oracle for a=aq/4, b=bq/4, as an independent quotient
// representation. All subsequent small dyadic sums fit exactly in binary64.
fn companion_moments(aq: i128, bq: i128) -> [f64; 7] {
    let matrix = [
        [0, 0, 0, 4 * bq - aq * aq],
        [16, 0, 0, -16],
        [0, 16, 0, 8 * aq],
        [0, 0, 16, 0],
    ];
    let mut power = [[0_i128; 4]; 4];
    for (i, row) in power.iter_mut().enumerate() {
        row[i] = 1;
    }
    let mut moments = [0.0; 7];
    for (k, moment) in moments.iter_mut().enumerate() {
        let numerator: i128 = (0..4).map(|i| power[i][i]).sum();
        assert!(numerator.abs() < (1_i128 << 53));
        *moment = numerator as f64 / 16_i128.pow(k as u32) as f64;
        let mut next = [[0_i128; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                next[i][j] = (0..4).map(|l| power[i][l] * matrix[l][j]).sum();
            }
        }
        power = next;
    }
    moments
}

fn case(query: &Query, exact: f64, require_baseline: bool) -> (Value, bool) {
    let mut arms = Vec::new();
    let mut support_pass = false;
    let mut baseline_pass = false;
    let mut support_singleton = false;
    let mut baseline_singleton = false;
    let mut sound = true;
    for route in [Route::Eager, Route::Support] {
        let mut work = TraceWork::default();
        let value = evaluate(query, route, &mut work);
        let encloses = value.as_ref().map(|v| v.contains(exact));
        let singleton = value.as_ref().map(|v| v.lo == exact && v.hi == exact);
        sound &= match &encloses {
            Ok(passed) => *passed,
            Err(_) => true,
        };
        if matches!(route, Route::Support) {
            support_pass = encloses.as_ref().is_ok_and(|&passed| passed);
            support_singleton = singleton.as_ref().is_ok_and(|&passed| passed);
        } else {
            baseline_pass = encloses.as_ref().is_ok_and(|&passed| passed);
            baseline_singleton = singleton.as_ref().is_ok_and(|&passed| passed);
        }
        arms.push(json!({
            "route": route.name(),
            "status": if value.is_ok() { "accepted" } else { "refused" },
            "enclosure": value.as_ref().ok().map(|v| v.as_pair()),
            "encloses_exact": encloses.ok(),
            "singleton_equals_exact": singleton.ok(),
            "failure": value.err().map(|error| error.to_string()),
            "work": work,
        }));
    }
    (
        json!({
            "name": query.name, "state_bits": query.state.map(f64::to_bits),
            "coefficient_bits": query.coefficients.map(f64::to_bits), "length": query.length,
            "exact_reference_bits": exact.to_bits(), "arms": arms,
        }),
        sound
            && support_pass
            && support_singleton
            && (!require_baseline || (baseline_pass && baseline_singleton)),
    )
}

fn family(name: &'static str) -> (usize, [f64; 7]) {
    match name {
        "square" => (3, [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0]),
        "cubic_padded" => (7, [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0]),
        "fifth_padded" => (7, [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0]),
        "dense_six" => (7, [1.0, -2.0, 3.0, -4.0, 5.0, -6.0, 7.0]),
        "mixed_six" => (7, [5.0, 0.0, -2.0, 1.0, 0.0, 0.0, 0.25]),
        _ => unreachable!("registered family"),
    }
}

const FAMILIES: [&str; 5] = [
    "square",
    "cubic_padded",
    "fifth_padded",
    "dense_six",
    "mixed_six",
];

fn timing_block(queries: &[Query], route: Route) -> Value {
    let mut work = TraceWork::default();
    let mut failures = 0_u64;
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        for query in queries {
            match evaluate(black_box(query), route, &mut work) {
                Ok(value) => {
                    black_box(value.as_pair());
                }
                Err(error) => {
                    black_box(error);
                    failures += 1;
                }
            }
        }
    }
    let seconds = start.elapsed().as_secs_f64();
    json!({"route": route.name(), "seconds": seconds, "calls": queries.len() * ITERATIONS,
           "failures": failures, "work": work})
}

fn main() {
    let mut validation = Vec::new();
    let mut validation_pass = true;
    for (aq, bq) in POINTS {
        let moments = companion_moments(aq, bq);
        for name in FAMILIES {
            let (length, coefficients) = family(name);
            let query = Query {
                name,
                state: [aq as f64 / 4.0, bq as f64 / 4.0],
                coefficients,
                length,
            };
            let exact = coefficients
                .iter()
                .zip(moments)
                .map(|(c, m)| c * m)
                .sum::<f64>();
            let (record, passed) = case(&query, exact, true);
            validation.push(record);
            validation_pass &= passed;
        }
    }
    let mut finite_scope = Vec::new();
    for (name, degree, a, b, exact) in [
        (
            "huge_square",
            2,
            2.0_f64.powi(600),
            f64::MAX,
            2.0_f64.powi(602),
        ),
        ("huge_cubic", 3, f64::MAX, f64::MAX, -3.0),
        ("huge_fourth", 4, 2.0_f64.powi(400), 0.0, 2.0_f64.powi(802)),
        (
            "huge_fifth",
            5,
            2.0_f64.powi(600),
            f64::MAX,
            -10.0 * 2.0_f64.powi(600),
        ),
    ] {
        let mut coefficients = [0.0; 7];
        coefficients[degree] = 1.0;
        let query = Query {
            name,
            state: [a, b],
            coefficients,
            length: 7,
        };
        let (record, passed) = case(&query, exact, false);
        finite_scope.push(record);
        validation_pass &= passed;
    }
    let setup_start = Instant::now();
    let workloads: Vec<_> = FAMILIES
        .into_iter()
        .map(|name| {
            let (length, coefficients) = family(name);
            (
                name,
                POINTS
                    .into_iter()
                    .map(|(aq, bq)| Query {
                        name,
                        state: [aq as f64 / 4.0, bq as f64 / 4.0],
                        coefficients,
                        length,
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    let workload_setup_seconds = setup_start.elapsed().as_secs_f64();
    let mut order_state = ORDER_SEED;
    let mut timing = Vec::new();
    for (name, queries) in &workloads {
        let warmups: Vec<_> = [Route::Eager, Route::Support]
            .into_iter()
            .map(|route| timing_block(queries, route))
            .collect();
        let mut repeats = Vec::new();
        for repeat in 0..REPEATS {
            order_state = order_state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let order = if order_state >> 63 == 0 {
                [Route::Eager, Route::Support, Route::Support, Route::Eager]
            } else {
                [Route::Support, Route::Eager, Route::Eager, Route::Support]
            };
            let blocks: Vec<_> = order
                .into_iter()
                .map(|route| timing_block(queries, route))
                .collect();
            repeats.push(json!({"repeat": repeat, "blocks": blocks}));
        }
        timing.push(json!({"family": name, "warmups": warmups, "repeats": repeats}));
    }
    let all_timed_calls_completed = timing.iter().all(|family| {
        family["warmups"]
            .as_array()
            .unwrap()
            .iter()
            .chain(
                family["repeats"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .flat_map(|repeat| repeat["blocks"].as_array().unwrap().iter()),
            )
            .all(|block| block["failures"] == 0)
    });
    let report = json!({
        "schema": "vigilode-trace-support-campaign-v1", "target": TRACE_TARGET_ID,
        "baseline_source": BASELINE_SOURCE,
        "baseline": "frozen old eager quartic_polynomial_trace operation order; same binary",
        "semantics": "all complex roots with algebraic multiplicity",
        "matched_accuracy_gate": "every timed input: both enclosures must be the exact singleton companion trace",
        "scope": "polynomial query calls, not an ODE integration or certified trajectory speedup",
        "validation": validation, "large_finite_scope_cases": finite_scope,
        "validation_pass": validation_pass, "all_timed_calls_completed": all_timed_calls_completed,
        "timing_protocol": {"repeats": REPEATS, "iterations": ITERATIONS, "queries_per_iteration": 16,
            "order": "seeded random ABBA/BAAB per repeat", "seed": ORDER_SEED,
            "call_boundary": "legacy evaluator and candidate wrapper are inline(never); candidate keeps the public cross-crate call, so full public-call overhead is measured",
            "included": "parameter/coefficient construction, validation, evaluation, counter updates and output black_box",
            "excluded": "one shared workload construction, JSON serialization, stored-result destruction",
            "workload_setup_seconds": workload_setup_seconds},
        "timing": timing,
    });
    println!("{}", serde_json::to_string(&report).unwrap());
    if !validation_pass || !all_timed_calls_completed {
        std::process::exit(1);
    }
}
