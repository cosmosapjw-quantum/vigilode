//! Research node SP03 (`research/sp03_declared_structure_routing_20261010`):
//! routing by declared, validated problem structure.
//!
//! The problems transcribe the CLI's stiff benchmark
//! (`crates/rodas5p-cli/src/stiff_benchmark.rs`): the interleaved
//! Brusselator with its explicit and in-place Jacobians and the native band
//! fill of `brusselator_band`, HIRES with both Jacobian callbacks, Robertson
//! and van der Pol (mu = 1000) from this crate, the JVP-only Brusselator of
//! `brusselator_jvp_only_problem`, and the benchmark's adaptive
//! configuration (atol = rtol * atol_scale, initial step 1e-6, min step
//! 1e-14, max step = span, 1,000,000 attempts). The recorded profile binds
//! the CLI runs to these (attempts and final states; see the checker).
//!
//! Non-ignored tests: every registered malformed declaration is refused with
//! a typed error before any integration step, a correct declaration is
//! accepted, and the router equals its target drivers on quick cells.
//! `export_runs` (ignored; release; `SP03_RUNS`) is the recorded run.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use rodas5p_core::{CoreResult, DenseMatrix, WorkCounters};
use rodas5p_integrators::{
    AdaptiveStepConfig, BandedJacobian, BandedJacobianFn, FastLuPolicy, ObservedIntegrationResult,
    OdeProblem, OutputSchedule, ProblemStructure, Rodas5pFastOptions, RoutedDriver, RoutedRun,
    RoutingError, RoutingOptions, RoutingReason, StructureDeclaration, StructureError,
    integrate_rodas5p_fast_banded_observed, integrate_rodas5p_fast_observed,
    integrate_rodas5p_fast_observed_with_options, integrate_rodas5p_mf_fast_observed_gmres_into,
    integrate_rodas5p_routed_observed, robertson_problem, stiff_van_der_pol_problem,
};
use serde_json::{Value, json};

const SCHEMA: &str = "vigilode-sp03-runs-v1";
const INITIAL_STEP: f64 = 1.0e-6;
const MAX_ATTEMPTS: usize = 1_000_000;
const ERROR_FLOOR: f64 = 1.0e-10;
const REFERENCE_RTOL: f64 = 1.0e-13;
/// The registered tolerances and sizes.
const RTOLS: [f64; 2] = [1.0e-6, 1.0e-8];
const BRUSSELATOR_CELLS: [usize; 4] = [50, 160, 200, 500];
const SMALL_PROBLEMS: [&str; 3] = ["robertson", "hires", "van-der-pol-mu1000"];
/// The undeclared (matrix-free route) cell.
const UNDECLARED_CELLS: usize = 160;
const UNDECLARED_RTOL: f64 = 1.0e-6;

// ------------------------------------------------------------------ problems

type Counter = Option<Arc<AtomicUsize>>;
type Rhs = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;

fn bump(counter: &Counter) {
    if let Some(c) = counter {
        c.fetch_add(1, Ordering::Relaxed);
    }
}

fn brusselator_c(cells: usize) -> f64 {
    (cells as f64 + 1.0).powi(2) / 50.0
}

fn brusselator_rhs(cells: usize, counter: Counter) -> Rhs {
    let c = brusselator_c(cells);
    Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        bump(&counter);
        for i in 0..cells {
            let (u, v) = (y[2 * i], y[2 * i + 1]);
            let (ul, vl) = if i == 0 {
                (1.0, 3.0)
            } else {
                (y[2 * i - 2], y[2 * i - 1])
            };
            let (ur, vr) = if i + 1 == cells {
                (1.0, 3.0)
            } else {
                (y[2 * i + 2], y[2 * i + 3])
            };
            out[2 * i] = 1.0 + u * u * v - 4.0 * u + c * (ul - 2.0 * u + ur);
            out[2 * i + 1] = 3.0 * u - u * u * v + c * (vl - 2.0 * v + vr);
        }
        Ok(())
    })
}

/// The CLI's `brusselator_problem`: explicit and in-place Jacobians, no JVP.
fn brusselator_full(cells: usize, counter: Counter) -> OdeProblem {
    let c = brusselator_c(cells);
    let n = 2 * cells;
    let fill = move |y: &[f64], j: &mut DenseMatrix| {
        for i in 0..cells {
            let (u, v) = (y[2 * i], y[2 * i + 1]);
            let (a, b) = (2 * i, 2 * i + 1);
            j[(a, a)] = 2.0 * u * v - 4.0 - 2.0 * c;
            j[(a, b)] = u * u;
            j[(b, a)] = 3.0 - 2.0 * u * v;
            j[(b, b)] = -u * u - 2.0 * c;
            if i > 0 {
                j[(a, a - 2)] = c;
                j[(b, b - 2)] = c;
            }
            if i + 1 < cells {
                j[(a, a + 2)] = c;
                j[(b, b + 2)] = c;
            }
        }
    };
    let jacobian = Arc::new(move |_t: f64, y: &[f64]| {
        let mut j = DenseMatrix::zeros(n, n);
        fill(y, &mut j);
        Ok(j)
    });
    let jacobian_into = Arc::new(move |_t: f64, y: &[f64], j: &mut DenseMatrix| {
        fill(y, j);
        Ok(())
    });
    OdeProblem::new(
        format!("brusselator-1d-{cells}"),
        n,
        brusselator_rhs(cells, counter),
        None,
        Some(jacobian),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap()
    .with_jacobian_into(jacobian_into)
}

/// The CLI's `brusselator_jvp_only_problem`.
fn brusselator_jvp_only(cells: usize, counter: Counter) -> OdeProblem {
    let c = brusselator_c(cells);
    let jvp = Arc::new(move |_t: f64, y: &[f64], w: &[f64], out: &mut [f64]| {
        for i in 0..cells {
            let (u, v) = (y[2 * i], y[2 * i + 1]);
            let (du, dv) = (w[2 * i], w[2 * i + 1]);
            let (dul, dvl) = if i == 0 {
                (0.0, 0.0)
            } else {
                (w[2 * i - 2], w[2 * i - 1])
            };
            let (dur, dvr) = if i + 1 == cells {
                (0.0, 0.0)
            } else {
                (w[2 * i + 2], w[2 * i + 3])
            };
            out[2 * i] = (2.0 * u * v - 4.0) * du + u * u * dv + c * (dul - 2.0 * du + dur);
            out[2 * i + 1] = (3.0 - 2.0 * u * v) * du - u * u * dv + c * (dvl - 2.0 * dv + dvr);
        }
        Ok(())
    });
    OdeProblem::new(
        format!("brusselator-1d-{cells}-jvp"),
        2 * cells,
        brusselator_rhs(cells, counter),
        None,
        None,
        Some(jvp),
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

fn brusselator_y0(cells: usize) -> Vec<f64> {
    (0..cells)
        .flat_map(|i| {
            let x = (i as f64 + 1.0) / (cells as f64 + 1.0);
            [1.0 + (2.0 * std::f64::consts::PI * x).sin(), 3.0]
        })
        .collect()
}

/// The CLI's `brusselator_band` (`lower = upper = 2`).
fn brusselator_band(cells: usize) -> BandedJacobian {
    let c = brusselator_c(cells);
    BandedJacobian {
        lower: 2,
        upper: 2,
        fill: Arc::new(move |_t: f64, y: &[f64], band: &mut [f64]| {
            for i in 0..cells {
                let (u, v) = (y[2 * i], y[2 * i + 1]);
                let (a, b) = (2 * i, 2 * i + 1);
                band[a * 5 + 2] = 2.0 * u * v - 4.0 - 2.0 * c;
                band[a * 5 + 3] = u * u;
                band[b * 5 + 1] = 3.0 - 2.0 * u * v;
                band[b * 5 + 2] = -u * u - 2.0 * c;
                if i > 0 {
                    band[a * 5] = c;
                    band[b * 5] = c;
                }
                if i + 1 < cells {
                    band[a * 5 + 4] = c;
                    band[b * 5 + 4] = c;
                }
            }
            Ok(())
        }),
    }
}

/// A malformed band (`lower = upper = 1`): the Brusselator's entries with
/// `|i - j| <= 1` only; the diffusion couplings at distance 2 are dropped.
fn brusselator_narrow_fill(cells: usize) -> BandedJacobianFn {
    Arc::new(move |_t: f64, y: &[f64], band: &mut [f64]| {
        for i in 0..cells {
            let (u, v) = (y[2 * i], y[2 * i + 1]);
            let (a, b) = (2 * i, 2 * i + 1);
            let c = brusselator_c(cells);
            band[a * 3 + 1] = 2.0 * u * v - 4.0 - 2.0 * c;
            band[a * 3 + 2] = u * u;
            band[b * 3] = 3.0 - 2.0 * u * v;
            band[b * 3 + 1] = -u * u - 2.0 * c;
        }
        Ok(())
    })
}

/// The CLI's `hires_problem`.
fn hires_full() -> (OdeProblem, Vec<f64>) {
    let rhs = Arc::new(|_t: f64, y: &[f64], out: &mut [f64]| {
        out[0] = -1.71 * y[0] + 0.43 * y[1] + 8.32 * y[2] + 0.0007;
        out[1] = 1.71 * y[0] - 8.75 * y[1];
        out[2] = -10.03 * y[2] + 0.43 * y[3] + 0.035 * y[4];
        out[3] = 8.32 * y[1] + 1.71 * y[2] - 1.12 * y[3];
        out[4] = -1.745 * y[4] + 0.43 * y[5] + 0.43 * y[6];
        out[5] = -280.0 * y[5] * y[7] + 0.69 * y[3] + 1.71 * y[4] - 0.43 * y[5] + 0.69 * y[6];
        out[6] = 280.0 * y[5] * y[7] - 1.81 * y[6];
        out[7] = -280.0 * y[5] * y[7] + 1.81 * y[6];
        Ok(())
    });
    fn hires_fill(y: &[f64], j: &mut DenseMatrix) {
        j[(0, 0)] = -1.71;
        j[(0, 1)] = 0.43;
        j[(0, 2)] = 8.32;
        j[(1, 0)] = 1.71;
        j[(1, 1)] = -8.75;
        j[(2, 2)] = -10.03;
        j[(2, 3)] = 0.43;
        j[(2, 4)] = 0.035;
        j[(3, 1)] = 8.32;
        j[(3, 2)] = 1.71;
        j[(3, 3)] = -1.12;
        j[(4, 4)] = -1.745;
        j[(4, 5)] = 0.43;
        j[(4, 6)] = 0.43;
        j[(5, 3)] = 0.69;
        j[(5, 4)] = 1.71;
        j[(5, 5)] = -280.0 * y[7] - 0.43;
        j[(5, 6)] = 0.69;
        j[(5, 7)] = -280.0 * y[5];
        j[(6, 5)] = 280.0 * y[7];
        j[(6, 6)] = -1.81;
        j[(6, 7)] = 280.0 * y[5];
        j[(7, 5)] = -280.0 * y[7];
        j[(7, 6)] = 1.81;
        j[(7, 7)] = -280.0 * y[5];
    }
    let jacobian = Arc::new(|_t: f64, y: &[f64]| {
        let mut j = DenseMatrix::zeros(8, 8);
        hires_fill(y, &mut j);
        Ok(j)
    });
    let jacobian_into = Arc::new(|_t: f64, y: &[f64], j: &mut DenseMatrix| {
        hires_fill(y, j);
        Ok(())
    });
    (
        OdeProblem::new(
            "hires",
            8,
            rhs,
            None,
            Some(jacobian),
            None,
            None,
            true,
            None,
            None,
        )
        .unwrap()
        .with_jacobian_into(jacobian_into),
        vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0057],
    )
}

/// One problem of the node as the CLI benchmark defines it.
struct Case {
    id: String,
    problem: OdeProblem,
    y0: Vec<f64>,
    t_span: (f64, f64),
    atol_scale: f64,
    cells: Option<usize>,
}

impl Case {
    fn adaptive(&self, rtol: f64) -> AdaptiveStepConfig {
        AdaptiveStepConfig {
            atol: rtol * self.atol_scale,
            rtol,
            initial_step: INITIAL_STEP,
            min_step: 1.0e-14,
            max_step: self.t_span.1 - self.t_span.0,
            max_attempts: MAX_ATTEMPTS,
            ..AdaptiveStepConfig::default()
        }
    }

    fn schedule(&self) -> OutputSchedule {
        OutputSchedule::new(vec![self.t_span.0, self.t_span.1]).unwrap()
    }

    fn dimension(&self) -> usize {
        self.problem.dimension
    }

    /// The registered declaration: the band on the Brusselators, `Dense`
    /// on the small problems.
    fn declared(&self) -> OdeProblem {
        let n = self.dimension();
        let declaration = match self.cells {
            Some(cells) => {
                let band = brusselator_band(cells);
                StructureDeclaration::banded(n, band.lower, band.upper, band.fill)
            }
            None => StructureDeclaration::dense(n),
        };
        self.problem
            .clone()
            .with_declared_structure(declaration)
            .unwrap()
    }

    fn jvp_only(&self) -> OdeProblem {
        brusselator_jvp_only(self.cells.expect("a Brusselator"), None)
    }
}

fn brusselator_case(cells: usize) -> Case {
    Case {
        id: format!("brusselator-1d-{cells}"),
        problem: brusselator_full(cells, None),
        y0: brusselator_y0(cells),
        t_span: (0.0, 10.0),
        atol_scale: 1.0,
        cells: Some(cells),
    }
}

fn small_case(id: &str) -> Case {
    let (problem, y0, t_span, atol_scale) = match id {
        "robertson" => {
            let (p, y0) = robertson_problem().unwrap();
            (p, y0, (0.0, 40.0), 1.0e-4)
        }
        "hires" => {
            let (p, y0) = hires_full();
            (p, y0, (0.0, 321.8122), 1.0e-4)
        }
        "van-der-pol-mu1000" => {
            let (p, y0) = stiff_van_der_pol_problem(1.0e3).unwrap();
            (p, y0, (0.0, 2000.0), 1.0)
        }
        other => panic!("unknown problem {other}"),
    };
    Case {
        id: id.into(),
        problem,
        y0,
        t_span,
        atol_scale,
        cells: None,
    }
}

// ------------------------------------------------------------------- records

fn hexes(v: &[f64]) -> Vec<String> {
    v.iter().map(|x| format!("{:016x}", x.to_bits())).collect()
}

/// The benchmark's endpoint error `max_i |y_i - r_i| / max(|r_i|, 1e-10)`.
fn endpoint_error(y: &[f64], reference: &[f64]) -> f64 {
    assert_eq!(y.len(), reference.len());
    y.iter()
        .zip(reference)
        .map(|(a, r)| (a - r).abs() / r.abs().max(ERROR_FLOOR))
        .fold(0.0, f64::max)
}

fn counters_value(c: &WorkCounters) -> Value {
    // Every field, zeros included, so the key set is fixed.
    let mut map = serde_json::to_value(c).unwrap();
    for key in [
        "jacobian_matvecs",
        "linear_matvec_vectors",
        "preconditioner_vectors",
        "merged_unknown_vector_calls",
        "phi_weight_underflows",
        "poly_block_products",
        "poly_vector_products",
        "poly_coefficient_setups",
        "poly_coefficient_reuses",
        "poly_block_allocations",
        "poly_fallbacks",
        "recycle_update_refreshes",
    ] {
        if map.get(key).is_none() {
            map[key] = json!(0);
        }
    }
    map
}

fn run_record(
    observed: &ObservedIntegrationResult,
    attempts: usize,
    accepted: usize,
    rejected: usize,
    driver: &str,
    reference: &[f64],
) -> Value {
    let y = observed.y.last().unwrap();
    json!({
        "ok": true,
        "success": observed.success,
        "message": observed.message,
        "driver": driver,
        "t": hexes(&observed.t),
        "y_last": hexes(y),
        "attempts": attempts,
        "accepted": accepted,
        "rejected": rejected,
        "internal_steps": observed.internal_steps,
        "output_clipped_steps": observed.output_clipped_steps,
        "counters": counters_value(&observed.counters),
        "error": endpoint_error(y, reference),
    })
}

fn failed(error: impl std::fmt::Display) -> Value {
    json!({"ok": false, "message": error.to_string()})
}

fn run_dense(case: &Case, rtol: f64, policy: FastLuPolicy, reference: &[f64]) -> Value {
    let options = Rodas5pFastOptions {
        lu_policy: policy,
        ..Rodas5pFastOptions::default()
    };
    match integrate_rodas5p_fast_observed_with_options(
        &case.problem,
        case.t_span,
        &case.y0,
        &case.adaptive(rtol),
        &case.schedule(),
        options,
    ) {
        Ok(r) => run_record(
            &r.observed,
            r.attempts,
            r.accepted_steps,
            r.rejected_steps,
            r.driver,
            reference,
        ),
        Err(e) => failed(e),
    }
}

fn run_banded(case: &Case, rtol: f64, reference: &[f64]) -> Value {
    let band = brusselator_band(case.cells.unwrap());
    match integrate_rodas5p_fast_banded_observed(
        &case.problem,
        &band,
        case.t_span,
        &case.y0,
        &case.adaptive(rtol),
        &case.schedule(),
    ) {
        Ok(r) => {
            let mut rec = run_record(
                &r.fast.observed,
                r.fast.attempts,
                r.fast.accepted_steps,
                r.fast.rejected_steps,
                r.fast.driver,
                reference,
            );
            rec["banded_work"] = serde_json::to_value(r.work).unwrap();
            rec
        }
        Err(e) => failed(e),
    }
}

fn run_legacy(case: &Case, problem: &OdeProblem, rtol: f64, reference: &[f64]) -> Value {
    match integrate_rodas5p_mf_fast_observed_gmres_into(
        problem,
        case.t_span,
        &case.y0,
        &RoutingOptions::default().matrix_free,
        &case.adaptive(rtol),
        &case.schedule(),
    ) {
        Ok(r) => run_record(
            &r.observed,
            r.attempts,
            r.accepted_steps,
            r.rejected_steps,
            r.driver,
            reference,
        ),
        Err(e) => failed(e),
    }
}

fn run_routed(case: &Case, problem: &OdeProblem, rtol: f64, reference: &[f64]) -> Value {
    match integrate_rodas5p_routed_observed(
        problem,
        case.t_span,
        &case.y0,
        &case.adaptive(rtol),
        &case.schedule(),
    ) {
        Ok(r) => {
            let mut rec = run_record(
                r.run.observed(),
                r.run.attempts(),
                r.run.accepted_steps(),
                r.run.rejected_steps(),
                r.routing.driver_id,
                reference,
            );
            rec["routing"] = serde_json::to_value(&r.routing).unwrap();
            if let Some(v) = &r.routing.verification {
                rec["routing"]["verification"]["charged"] = counters_value(&v.charged);
            }
            rec["driver_counters"] = counters_value(&r.driver_counters());
            if let RoutedRun::Banded(b) = &r.run {
                rec["banded_work"] = serde_json::to_value(b.work).unwrap();
            }
            rec
        }
        Err(e) => failed(e),
    }
}

// ---------------------------------------------------------------- references

fn workspace_path(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

const NATIVE_PATH: &str = "research/stiff_native_benchmark_20261001/NATIVE.json";

fn native_reference(problem: &str) -> Option<Vec<f64>> {
    let native: Value =
        serde_json::from_str(&std::fs::read_to_string(workspace_path(NATIVE_PATH)).unwrap())
            .unwrap();
    native["references"][problem]["final_state"]
        .as_array()
        .map(|v| v.iter().map(|x| x.as_f64().unwrap()).collect())
}

/// The reference of a case: NATIVE.json where it has one, else the dense
/// fast driver at rtol 1e-13 (recorded with its run).
fn reference(case: &Case) -> (Vec<f64>, Value) {
    if let Some(y) = native_reference(&case.id) {
        let record = json!({"kind": "native-json", "path": NATIVE_PATH, "y": y.clone()});
        return (y, record);
    }
    let run = integrate_rodas5p_fast_observed(
        &case.problem,
        case.t_span,
        &case.y0,
        &case.adaptive(REFERENCE_RTOL),
        &case.schedule(),
    )
    .unwrap();
    assert!(run.observed.success, "reference run of {} failed", case.id);
    let y = run.observed.y.last().unwrap().clone();
    let record = json!({
        "kind": "dense-fast-rtol-1e-13",
        "y": y.clone(),
        "y_hex": hexes(&y),
        "attempts": run.attempts,
        "accepted": run.accepted_steps,
    });
    (y, record)
}

// ---------------------------------------------------------------- validation

fn error_kind(error: &RoutingError) -> &'static str {
    match error {
        RoutingError::Structure(e) => structure_kind(e),
        RoutingError::Integration(_) => "Integration",
    }
}

fn structure_kind(error: &StructureError) -> &'static str {
    match error {
        StructureError::DimensionMismatch { .. } => "DimensionMismatch",
        StructureError::BandOutsideMatrix { .. } => "BandOutsideMatrix",
        StructureError::MissingBandCallback => "MissingBandCallback",
        StructureError::UnexpectedBandCallback => "UnexpectedBandCallback",
        StructureError::DenseWithoutExplicitJacobian => "DenseWithoutExplicitJacobian",
        StructureError::BandVerificationMismatch { .. } => "BandVerificationMismatch",
        StructureError::BandVerificationFailed(_) => "BandVerificationFailed",
    }
}

/// One validation case: declare, then (when the declaration is accepted)
/// route at rtol 1e-6, counting right-hand-side calls.
fn validation_case(
    name: &str,
    registered: bool,
    expected: &str,
    problem: OdeProblem,
    counter: Arc<AtomicUsize>,
    declaration: StructureDeclaration,
    case: &Case,
) -> Value {
    let declared = serde_json::to_value(declaration.structure).unwrap();
    let declared_dimension = declaration.dimension;
    let has_callback = declaration.band_jacobian.is_some();
    let base = json!({
        "case": name,
        "registered": registered,
        "problem": case.id,
        "dimension": problem.dimension,
        "declared": declared,
        "declared_dimension": declared_dimension,
        "band_callback": has_callback,
        "expected": expected,
    });
    let mut out = base;
    match problem.with_declared_structure(declaration) {
        Err(e) => {
            out["stage"] = json!("declaration");
            out["observed"] = json!(structure_kind(&e));
            out["message"] = json!(e.to_string());
            out["rhs_calls"] = json!(counter.load(Ordering::Relaxed));
            out["accepted_steps"] = json!(0);
        }
        Ok(declared) => {
            match integrate_rodas5p_routed_observed(
                &declared,
                case.t_span,
                &case.y0,
                &case.adaptive(1.0e-6),
                &case.schedule(),
            ) {
                Err(e) => {
                    out["stage"] = json!("verification");
                    out["observed"] = json!(error_kind(&e));
                    out["message"] = json!(e.to_string());
                    out["rhs_calls"] = json!(counter.load(Ordering::Relaxed));
                    out["accepted_steps"] = json!(0);
                }
                Ok(r) => {
                    out["stage"] = json!("integration");
                    out["observed"] = json!("accepted");
                    out["message"] = json!(r.run.observed().message);
                    out["rhs_calls"] = json!(counter.load(Ordering::Relaxed));
                    out["accepted_steps"] = json!(r.run.accepted_steps());
                    out["routing"] = serde_json::to_value(&r.routing).unwrap();
                    if let Some(v) = &r.routing.verification {
                        out["routing"]["verification"]["charged"] = counters_value(&v.charged);
                    }
                }
            }
        }
    }
    out
}

/// Every validation case (the registered malformed declarations first).
fn validation_cases(cells: usize) -> Vec<Value> {
    let case = brusselator_case(cells);
    let n = 2 * cells;
    let counted_full = || {
        let c = Arc::new(AtomicUsize::new(0));
        (brusselator_full(cells, Some(c.clone())), c)
    };
    let counted_jvp = || {
        let c = Arc::new(AtomicUsize::new(0));
        (brusselator_jvp_only(cells, Some(c.clone())), c)
    };
    let band = || brusselator_band(cells).fill;
    let mut rows = Vec::new();
    let (p, c) = counted_full();
    rows.push(validation_case(
        "band-outside-matrix-lower",
        true,
        "BandOutsideMatrix",
        p,
        c,
        StructureDeclaration::banded(n, n, 2, band()),
        &case,
    ));
    let (p, c) = counted_full();
    rows.push(validation_case(
        "band-outside-matrix-upper",
        true,
        "BandOutsideMatrix",
        p,
        c,
        StructureDeclaration::banded(n, 2, n, band()),
        &case,
    ));
    let (p, c) = counted_full();
    rows.push(validation_case(
        "narrow-band-dense-reference",
        true,
        "BandVerificationMismatch",
        p,
        c,
        StructureDeclaration::banded(n, 1, 1, brusselator_narrow_fill(cells)),
        &case,
    ));
    let (p, c) = counted_jvp();
    rows.push(validation_case(
        "narrow-band-jvp-reference",
        true,
        "BandVerificationMismatch",
        p,
        c,
        StructureDeclaration::banded(n, 1, 1, brusselator_narrow_fill(cells)),
        &case,
    ));
    let (p, c) = counted_full();
    rows.push(validation_case(
        "missing-band-callback",
        true,
        "MissingBandCallback",
        p,
        c,
        StructureDeclaration {
            dimension: n,
            structure: ProblemStructure::Banded { lower: 2, upper: 2 },
            band_jacobian: None,
        },
        &case,
    ));
    let (p, c) = counted_full();
    rows.push(validation_case(
        "dimension-mismatch-band",
        true,
        "DimensionMismatch",
        p,
        c,
        StructureDeclaration::banded(n + 2, 2, 2, band()),
        &case,
    ));
    let (p, c) = counted_full();
    rows.push(validation_case(
        "dimension-mismatch-dense",
        true,
        "DimensionMismatch",
        p,
        c,
        StructureDeclaration::dense(n - 1),
        &case,
    ));
    // Further malformed declarations (reported).
    let (p, c) = counted_jvp();
    rows.push(validation_case(
        "dense-without-explicit-jacobian",
        false,
        "DenseWithoutExplicitJacobian",
        p,
        c,
        StructureDeclaration::dense(n),
        &case,
    ));
    let (p, c) = counted_full();
    rows.push(validation_case(
        "callback-without-band",
        false,
        "UnexpectedBandCallback",
        p,
        c,
        StructureDeclaration {
            dimension: n,
            structure: ProblemStructure::Unstructured,
            band_jacobian: Some(band()),
        },
        &case,
    ));
    let (p, c) = counted_full();
    let wrong: BandedJacobianFn = {
        let fill = band();
        Arc::new(move |t: f64, y: &[f64], b: &mut [f64]| {
            fill(t, y, b)?;
            // One diffusion coupling off by a relative 1e-9.
            b[10 * 5 + 4] *= 1.0 + 1.0e-9;
            Ok(())
        })
    };
    rows.push(validation_case(
        "wrong-value-inside-band",
        false,
        "BandVerificationMismatch",
        p,
        c,
        StructureDeclaration::banded(n, 2, 2, wrong),
        &case,
    ));
    // Correct declarations.
    let (p, c) = counted_full();
    rows.push(validation_case(
        "correct-band-dense-reference",
        true,
        "accepted",
        p,
        c,
        StructureDeclaration::banded(n, 2, 2, band()),
        &case,
    ));
    let (p, c) = counted_jvp();
    rows.push(validation_case(
        "correct-band-jvp-reference",
        true,
        "accepted",
        p,
        c,
        StructureDeclaration::banded(n, 2, 2, band()),
        &case,
    ));
    let small = small_case("robertson");
    let counter = Arc::new(AtomicUsize::new(0));
    rows.push(validation_case(
        "correct-dense-small",
        true,
        "accepted",
        small.problem.clone(),
        counter,
        StructureDeclaration::dense(3),
        &small,
    ));
    rows
}

fn assert_validation(rows: &[Value]) {
    for row in rows {
        let name = row["case"].as_str().unwrap();
        assert_eq!(row["observed"], row["expected"], "{name}: {row}");
        if row["expected"] != "accepted" {
            assert_eq!(row["rhs_calls"], 0, "{name}: integration started");
            assert_eq!(row["accepted_steps"], 0, "{name}");
            assert_ne!(row["stage"], "integration", "{name}");
        } else {
            assert_eq!(row["message"], "success", "{name}");
        }
    }
}

// --------------------------------------------------------------------- tests

#[test]
fn every_malformed_declaration_is_refused_before_any_step() {
    let rows = validation_cases(20);
    assert_validation(&rows);
    let registered: Vec<&str> = rows
        .iter()
        .filter(|r| r["registered"] == true && r["expected"] != "accepted")
        .map(|r| r["observed"].as_str().unwrap())
        .collect();
    for kind in [
        "BandOutsideMatrix",
        "BandVerificationMismatch",
        "MissingBandCallback",
        "DimensionMismatch",
    ] {
        assert!(registered.contains(&kind), "{kind} not covered");
    }
}

#[test]
fn band_verification_is_charged_and_reported() {
    let case = brusselator_case(20);
    let declared = case.declared();
    let r = integrate_rodas5p_routed_observed(
        &declared,
        case.t_span,
        &case.y0,
        &case.adaptive(1.0e-4),
        &case.schedule(),
    )
    .unwrap();
    let v = r.routing.verification.as_ref().unwrap();
    assert_eq!(r.routing.reason, RoutingReason::DeclaredBand);
    assert_eq!((v.vectors, v.band_products, v.dense_products), (2, 2, 2));
    assert_eq!(v.charged.jacobian_matvecs, 4);
    assert_eq!(v.charged.jacobian_builds, 2);
    assert_eq!(
        r.run.observed().counters.jacobian_builds,
        r.driver_counters().jacobian_builds + 2
    );
}

/// Bitwise comparison of a routed record with its target's.
fn assert_parity(routed: &Value, target: &Value, label: &str) {
    for key in [
        "success",
        "t",
        "y_last",
        "attempts",
        "accepted",
        "rejected",
        "internal_steps",
        "output_clipped_steps",
        "driver",
    ] {
        assert_eq!(routed[key], target[key], "{label}: {key}");
    }
    assert_eq!(routed["driver_counters"], target["counters"], "{label}");
}

#[test]
fn routed_equals_its_target_driver_on_quick_cells() {
    // Band (and banded = dense), Brusselator n = 40.
    let case = brusselator_case(20);
    let reference = vec![1.0; case.dimension()];
    let routed = run_routed(&case, &case.declared(), 1.0e-4, &reference);
    let banded = run_banded(&case, 1.0e-4, &reference);
    let dense = run_dense(&case, 1.0e-4, FastLuPolicy::Legacy, &reference);
    assert_parity(&routed, &banded, "band");
    for key in ["t", "y_last", "attempts", "accepted", "rejected"] {
        assert_eq!(banded[key], dense[key], "banded vs dense: {key}");
    }
    assert_eq!(routed["routing"]["reason"], "declared-band");
    // Dense, Robertson.
    let small = small_case("robertson");
    let reference = native_reference("robertson").unwrap();
    let routed = run_routed(&small, &small.declared(), 1.0e-5, &reference);
    let dense = run_dense(&small, 1.0e-5, FastLuPolicy::Legacy, &reference);
    assert_parity(&routed, &dense, "dense");
    assert_eq!(routed["routing"]["reason"], "declared-dense-small");
    assert_eq!(routed["routing"]["verification"], Value::Null);
    // No declaration, JVP only: the matrix-free route.
    let mf = brusselator_case(10);
    let reference = vec![1.0; mf.dimension()];
    let jvp = mf.jvp_only();
    let routed = run_routed(&mf, &jvp, 1.0e-3, &reference);
    let legacy = run_legacy(&mf, &jvp, 1.0e-3, &reference);
    assert_parity(&routed, &legacy, "matrix-free");
    assert_eq!(routed["routing"]["reason"], "unstructured-jvp");
    assert_eq!(routed["routing"]["driver"], "matrix-free");
    // Unstructured with an explicit Jacobian: the dense driver, no fallback.
    let plain = integrate_rodas5p_routed_observed(
        &case.problem,
        case.t_span,
        &case.y0,
        &case.adaptive(1.0e-4),
        &case.schedule(),
    )
    .unwrap();
    assert_eq!(plain.routing.reason, RoutingReason::ExplicitJacobian);
    assert_eq!(plain.routing.driver, RoutedDriver::Dense);
    assert!(plain.routing.fallback.is_none());
}

#[test]
fn a_dense_declaration_above_eight_records_its_fallback() {
    let case = brusselator_case(10);
    let declared = case
        .problem
        .clone()
        .with_declared_structure(StructureDeclaration::dense(20))
        .unwrap();
    let r = integrate_rodas5p_routed_observed(
        &declared,
        case.t_span,
        &case.y0,
        &case.adaptive(1.0e-3),
        &case.schedule(),
    )
    .unwrap();
    assert_eq!(r.routing.reason, RoutingReason::ExplicitJacobian);
    assert!(r.routing.fallback.is_some());
    let direct = integrate_rodas5p_fast_observed(
        &case.problem,
        case.t_span,
        &case.y0,
        &case.adaptive(1.0e-3),
        &case.schedule(),
    )
    .unwrap();
    assert_eq!(r.run.observed().y, direct.observed.y);
    assert_eq!(r.run.observed().counters, direct.observed.counters);
}

#[test]
fn the_transcribed_problems_are_consistent() {
    // Band fill equals the dense Jacobian, and the JVP equals it to rounding.
    for cells in [10, 50] {
        let full = brusselator_full(cells, None);
        let jvp = brusselator_jvp_only(cells, None);
        let n = 2 * cells;
        let y: Vec<f64> = brusselator_y0(cells)
            .iter()
            .enumerate()
            .map(|(i, v)| v * (1.0 + 0.1 * ((i + 1) as f64).sin()))
            .collect();
        let mut counters = WorkCounters::default();
        let dense = full.dense_jacobian(0.0, &y, &mut counters).unwrap();
        let band = brusselator_band(cells);
        let mut values = vec![0.0; n * 5];
        (band.fill)(0.0, &y, &mut values).unwrap();
        for i in 0..n {
            for j in 0..n {
                let b = if j + 2 >= i && j <= i + 2 {
                    values[i * 5 + (j + 2 - i)]
                } else {
                    0.0
                };
                assert_eq!(dense[(i, j)].to_bits(), b.to_bits(), "({i}, {j})");
            }
        }
        let (mut a, mut b) = (vec![0.0; n], vec![0.0; n]);
        full.eval_rhs_into(0.0, &y, &mut a, &mut counters).unwrap();
        jvp.eval_rhs_into(0.0, &y, &mut b, &mut counters).unwrap();
        assert_eq!(a, b);
    }
}

// ---------------------------------------------------------------- recorded run

fn write_output(variable: &str, value: &Value) {
    if let Ok(path) = std::env::var(variable) {
        let path = workspace_path(&path);
        assert!(
            !path.exists(),
            "immutable output exists: {}",
            path.display()
        );
        std::fs::write(&path, serde_json::to_string_pretty(value).unwrap() + "\n").unwrap();
        println!("wrote {}", path.display());
    }
}

fn timed<T>(label: &str, f: impl FnOnce() -> T) -> T {
    let start = std::time::Instant::now();
    let value = f();
    println!("{label}: {:.1} s", start.elapsed().as_secs_f64());
    value
}

/// The recorded run: every registered cell and arm, the references and the
/// validation cases.
#[test]
#[ignore = "recorded run of research/sp03_declared_structure_routing_20261010; release build; set SP03_RUNS"]
fn export_runs() {
    if std::env::var("SP03_RUNS").is_err() {
        println!("SP03_RUNS is not set; export skipped");
        return;
    }
    let mut cells = Vec::new();
    let mut references = serde_json::Map::new();
    for &n_cells in &BRUSSELATOR_CELLS {
        let case = brusselator_case(n_cells);
        let (reference_y, reference_record) =
            timed(&format!("{} reference", case.id), || reference(&case));
        references.insert(case.id.clone(), reference_record);
        let declared = case.declared();
        let jvp = case.jvp_only();
        for &rtol in &RTOLS {
            let label = |arm: &str| format!("{} {rtol:e} {arm}", case.id);
            let arms = json!({
                "routed": timed(&label("routed"), || run_routed(&case, &declared, rtol, &reference_y)),
                "dense": timed(&label("dense"), || run_dense(&case, rtol, FastLuPolicy::Legacy, &reference_y)),
                "dense-colext64": timed(&label("dense-colext64"), || {
                    run_dense(&case, rtol, FastLuPolicy::ColumnExtentsAbove64, &reference_y)
                }),
                "banded": timed(&label("banded"), || run_banded(&case, rtol, &reference_y)),
                "legacy": timed(&label("legacy"), || run_legacy(&case, &jvp, rtol, &reference_y)),
            });
            cells.push(json!({
                "problem": case.id, "dimension": case.dimension(), "rtol": rtol,
                "atol": rtol * case.atol_scale, "declared": "banded", "arms": arms,
            }));
        }
        if n_cells == UNDECLARED_CELLS {
            let rtol = UNDECLARED_RTOL;
            let label = |arm: &str| format!("{} undeclared {rtol:e} {arm}", case.id);
            let arms = json!({
                "routed": timed(&label("routed"), || run_routed(&case, &jvp, rtol, &reference_y)),
                "legacy": timed(&label("legacy"), || run_legacy(&case, &jvp, rtol, &reference_y)),
            });
            cells.push(json!({
                "problem": case.id, "dimension": case.dimension(), "rtol": rtol,
                "atol": rtol * case.atol_scale, "declared": "none", "arms": arms,
            }));
        }
    }
    for id in SMALL_PROBLEMS {
        let case = small_case(id);
        let (reference_y, reference_record) = reference(&case);
        references.insert(case.id.clone(), reference_record);
        let declared = case.declared();
        for &rtol in &RTOLS {
            let arms = json!({
                "routed": run_routed(&case, &declared, rtol, &reference_y),
                "dense": run_dense(&case, rtol, FastLuPolicy::Legacy, &reference_y),
                "dense-colext64": run_dense(&case, rtol, FastLuPolicy::ColumnExtentsAbove64, &reference_y),
            });
            cells.push(json!({
                "problem": case.id, "dimension": case.dimension(), "rtol": rtol,
                "atol": rtol * case.atol_scale, "declared": "dense", "arms": arms,
            }));
        }
    }
    let validation = validation_cases(50);
    assert_validation(&validation);
    let out = json!({
        "schema": SCHEMA,
        "error_metric": "max_i |y_i - r_i| / max(|r_i|, 1e-10) against NATIVE.json (brusselator-1d-50/200, robertson, hires, van-der-pol-mu1000) or the dense fast driver at rtol 1e-13 (brusselator-1d-160/500)",
        "initial_step": INITIAL_STEP,
        "max_attempts": MAX_ATTEMPTS,
        "band_verification": {
            "vectors": rodas5p_integrators::BAND_VERIFICATION_VECTORS,
            "tolerance": rodas5p_integrators::BAND_VERIFICATION_TOLERANCE,
            "seed": rodas5p_integrators::BAND_VERIFICATION_SEED,
        },
        "references": references,
        "cells": cells,
        "validation": validation,
    });
    write_output("SP03_RUNS", &out);
}
