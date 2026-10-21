//! Reproducible same-model, same-output, same-tolerance structured comparison.
//! All timings include solver/adapter/workspace setup; coefficient cache is
//! warmed equally. JSONL retains failures and measured allocator calls/bytes.
//! Run in release mode; this runner does not promote a speed claim.
use rodas5p_core::{CoreResult, DenseMatrix};
use rodas5p_integrators::collective_model::CollectiveModel;
use rodas5p_integrators::collective_rodas5p::integrate_collective_rodas5p_observed;
use rodas5p_integrators::{
    AdaptiveStepConfig, ObservedIntegrationResult, OdeProblem, OutputSchedule,
    integrate_rodas5p_fast_observed,
};
use serde_json::json;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::Arc;
use std::time::Instant;

struct Counting;
thread_local! {
    static CALLS: Cell<u64> = const { Cell::new(0) };
    static BYTES: Cell<u64> = const { Cell::new(0) };
}
fn count(n: usize) {
    CALLS.with(|v| v.set(v.get() + 1));
    BYTES.with(|v| v.set(v.get() + n as u64));
}
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        count(l.size());
        unsafe { System.alloc(l) }
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        count(l.size());
        unsafe { System.alloc_zeroed(l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        count(n);
        unsafe { System.realloc(p, l, n) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
}
#[global_allocator]
static COUNTING: Counting = Counting;

fn dense_problem(model: &CollectiveModel) -> CoreResult<OdeProblem> {
    let m = model.clone();
    Ok(model
        .to_ode_problem()?
        .with_jacobian_into(Arc::new(move |_, y, j: &mut DenseMatrix| {
            let (u, v) = m.mean(y);
            let chi = m.chi();
            let c = [
                -2.0 * chi * u * v,
                -chi * (u * u + 3.0 * v * v),
                chi * (3.0 * u * u + v * v),
                2.0 * chi * u * v,
            ];
            for row in 0..m.num_modes() {
                for col in 0..m.num_modes() {
                    for a in 0..2 {
                        for b in 0..2 {
                            j[(2 * row + a, 2 * col + b)] = c[2 * a + b] * m.weights()[col];
                        }
                    }
                }
                j[(2 * row, 2 * row)] -= m.kappa();
                j[(2 * row + 1, 2 * row + 1)] -= m.kappa();
                j[(2 * row, 2 * row + 1)] -= m.width() * m.xi()[row];
                j[(2 * row + 1, 2 * row)] += m.width() * m.xi()[row];
            }
            Ok(())
        })))
}

// Independent classical RK4 reference, with two resolutions and all requested
// output times. The refinement discrepancy is empirical, not an enclosure.
fn reference(model: &CollectiveModel, per_output: usize) -> CoreResult<Vec<Vec<f64>>> {
    let n = model.dimension();
    let mut y = vec![0.0; n];
    model.initial_state(&mut y)?;
    let mut out = vec![y.clone()];
    let mut q = vec![0.0; n];
    let (mut k1, mut k2, mut k3, mut k4) = (vec![0.0; n], vec![0.0; n], vec![0.0; n], vec![0.0; n]);
    let h = 0.1 / per_output as f64;
    for block in 0..10 {
        for step in 0..per_output {
            let t = 0.1 * block as f64 + h * step as f64;
            model.rhs(t, &y, &mut k1)?;
            for i in 0..n {
                q[i] = y[i] + 0.5 * h * k1[i];
            }
            model.rhs(t + 0.5 * h, &q, &mut k2)?;
            for i in 0..n {
                q[i] = y[i] + 0.5 * h * k2[i];
            }
            model.rhs(t + 0.5 * h, &q, &mut k3)?;
            for i in 0..n {
                q[i] = y[i] + h * k3[i];
            }
            model.rhs(t + h, &q, &mut k4)?;
            for i in 0..n {
                y[i] += h / 6.0 * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
            }
        }
        out.push(y.clone());
    }
    Ok(out)
}
fn discrepancy(a: &[Vec<f64>], b: &[Vec<f64>]) -> f64 {
    a.iter()
        .flatten()
        .zip(b.iter().flatten())
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
}

fn grid_wrms(a: &[Vec<f64>], reference: &[Vec<f64>], atol: f64, rtol: f64) -> f64 {
    a.iter()
        .zip(reference)
        .map(|(row, refrow)| {
            (row.iter()
                .zip(refrow)
                .map(|(x, r)| {
                    let z = (x - r) / (atol + rtol * r.abs());
                    z * z
                })
                .sum::<f64>()
                / row.len() as f64)
                .sqrt()
        })
        .fold(0.0, f64::max)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let modes = args
        .get(1)
        .map(|x| x.parse())
        .transpose()?
        .unwrap_or(32_usize);
    let repeats = args
        .get(2)
        .map(|x| x.parse())
        .transpose()?
        .unwrap_or(3_usize);
    if modes == 0 || !modes.is_power_of_two() || repeats == 0 {
        return Err("modes must be a positive power of two; repeats must be positive".into());
    }
    let model = CollectiveModel::new(
        (0..modes)
            .map(|j| -1.0 + 2.0 * j as f64 / modes as f64)
            .collect(),
        vec![1.0 / modes as f64; modes],
        (0..modes)
            .map(|j| (0.3 + 0.05 * (j as f64).sin(), 0.1 * (j as f64).cos()))
            .collect(),
        2.0,
        0.8,
        0.0,
        0.15,
    )?;
    let calibration_before = CALLS.with(Cell::get);
    let calibration_clone = std::hint::black_box(model.clone());
    std::hint::black_box(calibration_clone.xi().as_ptr());
    std::hint::black_box(calibration_clone.weights().as_ptr());
    std::hint::black_box(calibration_clone.initial_amplitudes().as_ptr());
    let allocation_calibration_calls = CALLS.with(Cell::get) - calibration_before;
    let allocation_calibrated = allocation_calibration_calls >= 3;
    drop(calibration_clone);
    let mut y = vec![0.0; model.dimension()];
    model.initial_state(&mut y)?;
    let output = OutputSchedule::uniform(0.0, 1.0, 0.1)?;
    let coarse = reference(&model, 512)?;
    let reference = reference(&model, 1024)?;
    let reference_delta = discrepancy(&coarse, &reference);
    rodas5p_core::rodas5p_coefficients()?;
    for rtol in [1e-6, 1e-8, 1e-10] {
        let config = AdaptiveStepConfig {
            atol: rtol * 1e-3,
            rtol,
            initial_step: 0.01,
            max_step: 0.2,
            ..Default::default()
        };
        let reference_refinement_wrms = grid_wrms(&coarse, &reference, config.atol, config.rtol);
        for repeat in 0..repeats {
            // Alternate order to reduce a fixed warm/cache ordering advantage.
            let arms = if repeat % 2 == 0 {
                ["structured", "dense_fast"]
            } else {
                ["dense_fast", "structured"]
            };
            for arm in arms {
                let calls = CALLS.with(Cell::get);
                let bytes = BYTES.with(Cell::get);
                let now = Instant::now();
                let mut structured_work = None;
                let result: CoreResult<ObservedIntegrationResult> = if arm == "structured" {
                    integrate_collective_rodas5p_observed(&model, (0.0, 1.0), &y, &config, &output)
                        .map(|r| {
                            structured_work = Some(r.linear_work);
                            r.observed
                        })
                } else {
                    dense_problem(&model).and_then(|p| {
                        integrate_rodas5p_fast_observed(&p, (0.0, 1.0), &y, &config, &output)
                            .map(|r| r.observed)
                    })
                };
                let nanos = now.elapsed().as_nanos();
                let alloc_calls = CALLS.with(Cell::get) - calls;
                let alloc_bytes = BYTES.with(Cell::get) - bytes;
                let common = json!({"schema":"collective-structured-comparison-v1","build_profile":if cfg!(debug_assertions) {"debug"} else {"release"},"arm":arm,"modes":modes,"repeat":repeat,
                    "rtol":rtol,"atol":config.atol,"elapsed_ns":nanos,"allocation_calls":allocation_calibrated.then_some(alloc_calls),"allocated_bytes":allocation_calibrated.then_some(alloc_bytes),
                    "allocation_calibration_valid":allocation_calibrated,"allocation_calibration_library_clone_calls":allocation_calibration_calls,
                    "allocation_raw_counter_calls":alloc_calls,"allocation_raw_counter_bytes":alloc_bytes,
                    "measurement_scope":"solver_and_adapter_setup_plus_integration_and_output","coefficient_cache":"warmed",
                    "reference":"independent_classical_rk4_10240_steps","reference_refinement_max_abs":reference_delta,"reference_refinement_max_grid_wrms":reference_refinement_wrms,"reference_authority":"empirical_two_resolution",
                    "promotion":"DIAGNOSTIC_ONLY"});
                let mut row = common;
                row["structured_linear_work"] = structured_work.map_or(serde_json::Value::Null, |w| {
                    json!({"arithmetic_kind":"conservative_structural_proxies_not_measured_flops",
                        "setup_ops":w.setup_ops,"setup_allocations_proxy":w.setup_allocations,
                        "setup_bytes_proxy":w.setup_bytes,"identity_check_ops":w.identity_check_ops,
                        "schur_certificate_ops":w.schur_certificate_ops,"inverse_certificate_ops":w.inverse_certificate_ops,
                        "rhs_ops":w.rhs_ops,"reconstruction_ops":w.reconstruction_ops,
                        "residual_witness_calls":w.residual_witness_calls})
                });
                row["stage_residual_certification"] = json!(
                    "disabled_for_this_numerical_baseline; opt_in_workspace_method_available"
                );
                match result {
                    Ok(r) => {
                        row["success"] = json!(r.success);
                        row["message"] = json!(r.message);
                        row["output_count"] = json!(r.t.len());
                        row["counters"] = json!(r.counters);
                        if r.success && r.t == output.times() && r.y.len() == reference.len() {
                            row["max_abs_reference_error"] = json!(discrepancy(&r.y, &reference));
                            let error_wrms = grid_wrms(&r.y, &reference, config.atol, config.rtol);
                            row["max_grid_wrms_reference_error"] = json!(error_wrms);
                            row["empirical_accuracy_gate_upper"] =
                                json!(error_wrms + reference_refinement_wrms);
                            row["empirical_accuracy_gate_pass"] =
                                json!(error_wrms + reference_refinement_wrms <= 1.0);
                        } else {
                            row["max_abs_reference_error"] = serde_json::Value::Null;
                            row["max_grid_wrms_reference_error"] = serde_json::Value::Null;
                            row["empirical_accuracy_gate_upper"] = serde_json::Value::Null;
                            row["empirical_accuracy_gate_pass"] = json!(false);
                        }
                    }
                    Err(e) => {
                        row["success"] = json!(false);
                        row["message"] = json!(e.to_string());
                        row["max_abs_reference_error"] = serde_json::Value::Null;
                        row["max_grid_wrms_reference_error"] = serde_json::Value::Null;
                        row["empirical_accuracy_gate_upper"] = serde_json::Value::Null;
                        row["empirical_accuracy_gate_pass"] = json!(false);
                    }
                }
                println!("{}", serde_json::to_string(&row)?);
            }
        }
    }
    Ok(())
}
