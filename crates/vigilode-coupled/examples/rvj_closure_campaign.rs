//! Registered bounded native RVJ closure campaign; JSON goes to stdout.
//! Run only after the execution sources and protocol have been published.
//! One warmup and seven interleaved timed repetitions are resource choices,
//! not accuracy constants. Coefficients and all endpoints are exported once
//! per arm outside the timing sample for an independent exact oracle.

use serde::Serialize;
use serde_json::{Value, json};
use std::{error::Error, hint::black_box, time::Instant};
use vigilode_coupled::cyclic_closure::{
    BRANCH_ID, Certifier, CyclicModel, CyclicWork, DyadicGrid, MODEL_ID, PATH_DEFINITION,
    PHYSICAL_CAP, PreparedEndpoint, RadiusRecipe, SlabCertificate, certify_slab, integrate_rk4,
    meets_physical_cap_bound,
};

const WARMUPS: usize = 1;
const REPEATS: usize = 7;
const ARMS: [(&str, Certifier, RadiusRecipe); 3] = [
    (
        "historical_power_legacy",
        Certifier::HistoricalPower,
        RadiusRecipe::Legacy,
    ),
    (
        "direct_bernstein_legacy",
        Certifier::DirectBernstein,
        RadiusRecipe::Legacy,
    ),
    (
        "direct_bernstein_tight",
        Certifier::DirectBernstein,
        RadiusRecipe::Tight,
    ),
];

#[derive(Clone, Debug, Serialize)]
struct Timing {
    repeat: usize,
    candidate_seconds: f64,
    preparation_seconds: f64,
    certificate_seconds: f64,
    total_seconds: f64,
}

struct Run {
    states: Vec<Vec<f64>>,
    slabs: Vec<SlabCertificate>,
    counts: CyclicWork,
    timing: Timing,
}

fn run_once(
    model: &CyclicModel,
    grid: &DyadicGrid,
    arm: usize,
    diagnostics: bool,
    repeat: usize,
) -> Result<Run, Box<dyn Error>> {
    let (_, certifier, radius) = ARMS[arm];
    let total_start = Instant::now();
    let mut counts = CyclicWork::default();
    let mut phase = "candidate";
    // Keep the work record outside the fallible closure so every returned
    // numerical error retains its actually attempted partial work.
    let result: Result<_, Box<dyn Error>> = (|| {
        let start = Instant::now();
        let states = integrate_rk4(model, grid, &mut counts)?;
        let candidate_seconds = start.elapsed().as_secs_f64();

        phase = "preparation";
        let start = Instant::now();
        let mut prepared: Vec<PreparedEndpoint> = Vec::new();
        match certifier {
            Certifier::HistoricalPower => {
                prepared.reserve(2 * grid.steps());
                for k in 0..grid.steps() {
                    prepared.push(model.prepare_reference_endpoint(&states[k], &mut counts)?);
                    prepared.push(model.prepare_reference_endpoint(&states[k + 1], &mut counts)?);
                }
            }
            Certifier::DirectBernstein => {
                prepared.reserve(grid.steps() + 1);
                for state in &states {
                    prepared.push(model.prepare_endpoint(state, &mut counts)?);
                }
            }
        }
        let preparation_seconds = start.elapsed().as_secs_f64();

        phase = "certificate";
        let start = Instant::now();
        let mut incoming = 0.0;
        let mut slabs = Vec::with_capacity(grid.steps());
        for k in 0..grid.steps() {
            let (left, right) = match certifier {
                Certifier::HistoricalPower => (&prepared[2 * k], &prepared[2 * k + 1]),
                Certifier::DirectBernstein => {
                    if k > 0 {
                        model.reuse_endpoint(&prepared[k], &states[k], &mut counts)?;
                    }
                    (&prepared[k], &prepared[k + 1])
                }
            };
            let receipt = certify_slab(
                model,
                left,
                right,
                grid.slab(k)?,
                incoming,
                certifier,
                radius,
                diagnostics,
                &mut counts,
            )?;
            incoming = receipt.ee;
            slabs.push(receipt);
        }
        black_box(incoming);
        let certificate_seconds = start.elapsed().as_secs_f64();
        phase = "workspace_cleanup";
        // Prepared intervals are private workspace, so their destruction is
        // part of complete online time. States/slabs are returned outputs;
        // their consumer-owned lifetime is outside this producer timer.
        drop(prepared);
        let total_seconds = total_start.elapsed().as_secs_f64();
        Ok((
            states,
            slabs,
            Timing {
                repeat,
                candidate_seconds,
                preparation_seconds,
                certificate_seconds,
                total_seconds,
            },
        ))
    })();
    match result {
        Ok((states, slabs, timing)) => Ok(Run {
            states,
            slabs,
            counts,
            timing,
        }),
        Err(error) => {
            let failure = json!({
                "schema": "vigilode.rvj_closure_attempt_failure.v1",
                "status": "failed", "name": ARMS[arm].0,
                "n": model.dimension(), "steps": grid.steps(),
                "phase": phase, "repeat": repeat, "diagnostics": diagnostics,
                "partial_counts": counts,
                "elapsed_attempt_seconds": total_start.elapsed().as_secs_f64(),
                "error": error.to_string()
            });
            eprintln!("{failure}");
            Err(error)
        }
    }
}

fn same_endpoint_bits(a: &[Vec<f64>], b: &[Vec<f64>]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(left, right)| {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(x, y)| x.to_bits() == y.to_bits())
        })
}

fn bits(value: f64) -> String {
    value.to_bits().to_string()
}

fn slab_json(receipt: &SlabCertificate) -> Result<Value, Box<dyn Error>> {
    let d = receipt
        .diagnostics
        .as_ref()
        .ok_or("untimed diagnostic coefficients missing")?;
    Ok(json!({
        "index": receipt.index,
        "t0": receipt.t0, "t0_bits": bits(receipt.t0),
        "t1": receipt.t1, "t1_bits": bits(receipt.t1),
        "h": receipt.h, "h_bits": bits(receipt.h),
        "e0": receipt.e0, "e0_bits": bits(receipt.e0),
        "df": receipt.df, "df_bits": bits(receipt.df),
        "dg": receipt.dg, "dg_bits": bits(receipt.dg),
        "ew": receipt.ew, "ew_bits": bits(receipt.ew),
        "ee": receipt.ee, "ee_bits": bits(receipt.ee),
        "qw": receipt.qw, "qw_bits": bits(receipt.qw),
        "physical_budget_met": receipt.meets_physical_cap(),
        "path_bernstein": d.path_bernstein,
        "slow_bernstein": d.slow_bernstein,
        "algebraic_bernstein": d.algebraic_bernstein
    }))
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut configurations = Vec::new();
    for n in [4, 8, 16] {
        let model = CyclicModel::new(n, 0)?;
        for steps in [16, 32, 64, 128] {
            let grid = DyadicGrid::new(steps)?;
            let mut canonical_states: Option<Vec<Vec<f64>>> = None;
            for _ in 0..WARMUPS {
                for arm in 0..ARMS.len() {
                    let warmup = run_once(&model, &grid, arm, false, 0)?;
                    if let Some(states) = &canonical_states {
                        if !same_endpoint_bits(states, &warmup.states) {
                            return Err(
                                "warmup candidate endpoint bits changed between arms".into()
                            );
                        }
                    } else {
                        canonical_states = Some(warmup.states.clone());
                    }
                    black_box(&warmup.slabs);
                }
            }
            let canonical_states =
                canonical_states.ok_or("campaign requires at least one warmup")?;
            let mut timings: [Vec<Timing>; 3] =
                std::array::from_fn(|_| Vec::with_capacity(REPEATS));
            let mut timed_counts: [Option<CyclicWork>; 3] = std::array::from_fn(|_| None);
            let mut execution_order = Vec::with_capacity(REPEATS);
            for repeat in 0..REPEATS {
                let order: Vec<usize> = (0..ARMS.len())
                    .map(|offset| (repeat + offset) % ARMS.len())
                    .collect();
                execution_order.push(order.iter().map(|&arm| ARMS[arm].0).collect::<Vec<_>>());
                for arm in order {
                    let run = run_once(&model, &grid, arm, false, repeat)?;
                    // Endpoint equality and result retention happen outside
                    // the measured elapsed intervals.
                    if !same_endpoint_bits(&canonical_states, &run.states) {
                        return Err("timed candidate endpoint bits changed".into());
                    }
                    if let Some(previous) = &timed_counts[arm] {
                        if previous != &run.counts {
                            return Err(
                                "deterministic work counts changed between repetitions".into()
                            );
                        }
                    }
                    timed_counts[arm] = Some(run.counts);
                    timings[arm].push(run.timing);
                    black_box(run.slabs);
                }
            }

            let mut arms = Vec::with_capacity(ARMS.len());
            for arm in 0..ARMS.len() {
                // Independent complete diagnostic pass, explicitly outside
                // the timing cohort. Every slab is exported for the oracle.
                let run = run_once(&model, &grid, arm, true, 0)?;
                if !same_endpoint_bits(&canonical_states, &run.states) {
                    return Err("diagnostic endpoints differ from timed endpoints".into());
                }
                if timed_counts[arm].as_ref() != Some(&run.counts) {
                    return Err("diagnostics changed numerical work counters".into());
                }
                let max_df = run.slabs.iter().map(|c| c.df).fold(0.0_f64, f64::max);
                let max_dg = run.slabs.iter().map(|c| c.dg).fold(0.0_f64, f64::max);
                let max_ew = run.slabs.iter().map(|c| c.ew).fold(0.0_f64, f64::max);
                let max_qw = run.slabs.iter().map(|c| c.qw).fold(0.0_f64, f64::max);
                let slabs = run
                    .slabs
                    .iter()
                    .map(slab_json)
                    .collect::<Result<Vec<_>, _>>()?;
                arms.push(json!({
                    "name": ARMS[arm].0,
                    "certifier": ARMS[arm].1,
                    "radius_recipe": ARMS[arm].2,
                    "raw_timings": timings[arm],
                    "counts": run.counts,
                    "max_df": max_df, "max_dg": max_dg,
                    "max_ew": max_ew, "max_qw": max_qw,
                    "physical_budget_met": meets_physical_cap_bound(max_ew.max(max_qw)),
                    "slabs": slabs
                }));
            }
            let endpoint_bits: Vec<Vec<String>> = canonical_states
                .iter()
                .map(|state| state.iter().map(|x| bits(*x)).collect())
                .collect();
            configurations.push(json!({
                "n": n, "steps": steps, "h": grid.h(), "h_bits": bits(grid.h()),
                "endpoint_bits": endpoint_bits,
                "timed_execution_order": execution_order,
                "arms": arms
            }));
        }
    }
    let output = json!({
        "schema": "vigilode.rvj_closure_campaign.v1",
        "model_id": MODEL_ID, "branch_id": BRANCH_ID,
        "path_definition": PATH_DEFINITION,
        "physical_norm": "all-state-infinity",
        "physical_time_interval": [0,1], "cap": PHYSICAL_CAP,
        "cap_exact": "1/100000000",
        "cap_comparison": "directed upper product bound*100000000 <= 1",
        "warmups": WARMUPS, "repeats": REPEATS,
        "dimensions": [4,8,16], "resolutions": [16,32,64,128],
        "candidate": "same native compiled cyclic-lift dyadic RK4; all endpoint bits checked equal",
        "historical_reference": "native power polynomial convolution and Bernstein transform, same directed Interval primitives, 2N slopes-only endpoint preparations",
        "direct_certificate": {
            "preparation": "N+1 exact-slope and original-G endpoint enclosures; N-1 verified shared-endpoint reuses",
            "slow_identity": "degree3 endpoints exactly zero by the exact-field Hermite definition",
            "algebraic_identity": "degree6 beta1=beta0 and beta5=beta6 from G_q*v=f even off G=0",
            "df_bound": "(3/4)*max(abs(beta1),abs(beta2))",
            "dg_bound": "max(g,(7*g+25*m)/32), global g=max endpoint magnitudes and m=max three middle magnitudes"
        },
        "timing_scope": "candidate+preparation+certificate+in-run allocation+private workspace cleanup; excludes returned-output consumer lifetime, process startup, compilation, JSON, warmups and untimed diagnostic passes",
        "failure_accounting": "returned attempt errors emit structured stderr JSON with actual partial work counters, arm, n, N, phase and elapsed time; parent preserves stderr/exit status",
        "diagnostic_scope": "every slab of every arm; separate untimed pass uses identical endpoint bits and numerical work counts",
        "production_claim": "fixed-model opt-in certificate; no generic production promotion",
        "configurations": configurations
    });
    serde_json::to_writer(std::io::stdout().lock(), &output)?;
    println!();
    Ok(())
}
