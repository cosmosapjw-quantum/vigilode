//! Work-precision benchmark of RODAS5P against the in-repository BDF and
//! Radau IIA integrators on four standard stiff problems (research node
//! `research/stiff_bdf_radau_benchmark_20261001`). SciPy's BDF and Radau run
//! the same equations from `tools/stiff_benchmark_scipy.py`, which also
//! checks that its right-hand sides match the samples exported here.
//!
//! Wall seconds are diagnostics on a shared host: the statistical authority
//! of timing is on hold (`rodas5p_fair_ab::timing_authority_registry`), so
//! nothing here is a speed promotion.

use std::{sync::Arc, time::Instant};

use anyhow::Result;
use rodas5p_core::{CoreResult, DenseMatrix, LinearMethod, LinearSolverConfig, WorkCounters};
use rodas5p_integrators::{
    AdaptiveObservedIntegrationResult, AdaptiveRunDiagnostics, AdaptiveStepConfig, BandedJacobian,
    BandedKernel, BandedWork, BatchProblem, BdfConfig, FastLuPolicy, IntegrationMethod,
    NewtonTolerancePolicy, OdeProblem, OutputSchedule, RadauConfig, Rodas5pFastOptions,
    Rodas5pFastSmallOptions, Rodas5pFastSmallResult, SmallBatchMember, SmallProblem,
    integrate_adaptive_observed_with_config, integrate_bdf_adaptive_observed,
    integrate_radau_adaptive_observed, integrate_rodas5p_fast_banded_observed_with_kernel,
    integrate_rodas5p_fast_observed, integrate_rodas5p_fast_observed_with_options,
    integrate_rodas5p_fast_small_batch, integrate_rodas5p_fast_small_observed,
    integrate_rodas5p_fast_small_observed_with_options,
    integrate_rodas5p_fast_small_observed_with_small_options, robertson_problem,
    stiff_van_der_pol_problem,
};
use serde_json::{Value, json};

pub const SCHEMA: &str = "vigilode-stiff-bdf-radau-benchmark-v1";
pub const TOLERANCES: [f64; 7] = [1.0e-3, 1.0e-4, 1.0e-5, 1.0e-6, 1.0e-7, 1.0e-8, 1.0e-9];
/// The lean RODAS5P driver (`research/stiff_rodas5p_fast_20261002`). Not in
/// [`ARMS`], so the default selection of the earlier nodes is unchanged.
pub const FAST_ARM: &str = "rodas5p-fast";

fn known_arm(arm: &str) -> bool {
    ARMS.contains(&arm)
        || arm == FAST_ARM
        || arm == SMALL_ARM
        || OPTION_ARMS.contains(&arm)
        || small_static_arm(arm).is_some()
        || banded_kernel_arm(arm).is_some()
}

/// The INT-03 banded pipeline on the Brusselators (speed research node
/// SPD03, `research/spd03_banded_arm_instructions_20261005`): the indexed
/// kernel and the slices kernel. Not in [`ARMS`].
pub const BANDED_ARM: &str = "rodas5p-fast-banded";
pub const BANDED_SLICES_ARM: &str = "rodas5p-fast-banded-slices";

pub fn banded_kernel_arm(arm: &str) -> Option<BandedKernel> {
    match arm {
        BANDED_ARM => Some(BandedKernel::Indexed),
        BANDED_SLICES_ARM => Some(BandedKernel::Slices),
        _ => None,
    }
}

/// The number of cells of a `brusselator-1d-<cells>` benchmark problem.
fn brusselator_cells(id: &str) -> Option<usize> {
    id.strip_prefix("brusselator-1d-")?.parse().ok()
}

/// The band (`lower = upper = 2`) of the interleaved Brusselator, writing the
/// same expressions as the dense fill of [`brusselator_problem`] at band
/// offsets `j + 2 - i`: row `2i` holds columns `2i - 2, 2i, 2i + 1, 2i + 2`
/// at offsets 0, 2, 3, 4 and row `2i + 1` columns `2i - 1, 2i, 2i + 1,
/// 2i + 3` at offsets 0, 1, 2, 4.
fn brusselator_band(cells: usize) -> BandedJacobian {
    let c = (cells as f64 + 1.0).powi(2) / 50.0;
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

/// The banded pipeline on a Brusselator benchmark problem, with its counted
/// banded work.
fn run_banded(
    problem: &BenchmarkProblem,
    rtol: f64,
    kernel: BandedKernel,
) -> CoreResult<(AdaptiveObservedIntegrationResult, BandedWork)> {
    let cells = brusselator_cells(problem.id).ok_or_else(|| {
        rodas5p_core::CoreError::InvalidInput(format!(
            "the banded arms cover the Brusselators, not {}",
            problem.id
        ))
    })?;
    let adaptive = adaptive_config(problem, rtol);
    let output = OutputSchedule::new(vec![problem.t_span.0, problem.t_span.1])?;
    let run = integrate_rodas5p_fast_banded_observed_with_kernel(
        &problem.problem,
        &brusselator_band(cells),
        problem.t_span,
        &problem.y0,
        &adaptive,
        &output,
        kernel,
    )?;
    Ok((
        AdaptiveObservedIntegrationResult {
            observed: run.fast.observed,
            diagnostics: AdaptiveRunDiagnostics {
                attempts: run.fast.attempts,
                accepted_macro_steps: run.fast.accepted_steps,
                rejected_macro_steps: run.fast.rejected_steps,
                ..AdaptiveRunDiagnostics::default()
            },
        },
        run.work,
    ))
}

/// The opt-in option arms of speed research node SPD01
/// (`research/spd01_fast_driver_overhead_20261005`): the dense fast driver
/// and the small driver with a prevalidated controller (`-val`), a fused
/// landing (`-land`) or both (`-ovh`). Not in [`ARMS`] and not in any
/// default selection. Returns `(small driver, options)`.
pub fn option_arm(arm: &str) -> Option<(bool, Rodas5pFastOptions)> {
    let (small, suffix) = if let Some(rest) = arm.strip_prefix("rodas5p-fast-small-") {
        (true, rest)
    } else if let Some(rest) = arm.strip_prefix("rodas5p-fast-") {
        (false, rest)
    } else {
        return None;
    };
    let options = match suffix {
        "val" => Rodas5pFastOptions {
            prevalidated_controller: true,
            ..Rodas5pFastOptions::default()
        },
        "land" => Rodas5pFastOptions {
            fused_landing: true,
            ..Rodas5pFastOptions::default()
        },
        "ovh" => Rodas5pFastOptions {
            prevalidated_controller: true,
            fused_landing: true,
            ..Rodas5pFastOptions::default()
        },
        // Speed research node SPD02: the column-extent LU of the dense
        // driver (no small variant).
        "colext" if !small => Rodas5pFastOptions {
            lu_policy: FastLuPolicy::ColumnExtents,
            ..Rodas5pFastOptions::default()
        },
        // Speed research node SPD09 (`research/spd09_colext_threshold_20261007`):
        // the column-extent LU above `RODAS5P_FAST_SMALL_LU_MAX` only.
        "colext64" if !small => Rodas5pFastOptions {
            lu_policy: FastLuPolicy::ColumnExtentsAbove64,
            ..Rodas5pFastOptions::default()
        },
        _ => return None,
    };
    Some((small, options))
}

/// Every option arm: SPD01's six (dense then small), SPD02's one and
/// SPD09's one.
pub const OPTION_ARMS: [&str; 8] = [
    "rodas5p-fast-val",
    "rodas5p-fast-land",
    "rodas5p-fast-ovh",
    "rodas5p-fast-small-val",
    "rodas5p-fast-small-land",
    "rodas5p-fast-small-ovh",
    "rodas5p-fast-colext",
    "rodas5p-fast-colext64",
];

/// The arms of speed research node SPD06
/// (`research/spd06_small_static_stages_20261007`): the small driver with
/// the fixed stage structure and index-loop solves (the gated arm), the fixed
/// stages only, and the gated arm with SPD01's two options. Not in [`ARMS`]
/// and not in [`OPTION_ARMS`] (they take [`Rodas5pFastSmallOptions`]).
pub const SMALL_STATIC_ARMS: [&str; 3] = [
    "rodas5p-fast-small-static",
    "rodas5p-fast-small-static-stages",
    "rodas5p-fast-small-static-ovh",
];

/// The small-driver options of an SPD06 arm.
pub fn small_static_arm(arm: &str) -> Option<Rodas5pFastSmallOptions> {
    let fixed = Rodas5pFastSmallOptions {
        static_stages: true,
        static_solves: true,
        ..Rodas5pFastSmallOptions::default()
    };
    match arm {
        "rodas5p-fast-small-static" => Some(fixed),
        "rodas5p-fast-small-static-stages" => Some(Rodas5pFastSmallOptions {
            static_solves: false,
            ..fixed
        }),
        "rodas5p-fast-small-static-ovh" => Some(Rodas5pFastSmallOptions {
            fast: Rodas5pFastOptions {
                prevalidated_controller: true,
                fused_landing: true,
                ..Rodas5pFastOptions::default()
            },
            ..fixed
        }),
        _ => None,
    }
}

pub const ARMS: [&str; 5] = [
    "rodas5p",
    "repo-bdf2",
    "repo-bdf2-tier-a",
    "repo-radau5",
    "repo-radau5-tier-a",
];
const INITIAL_STEP: f64 = 1.0e-6;
const MAX_ATTEMPTS: usize = 1_000_000;

pub struct BenchmarkProblem {
    pub id: &'static str,
    pub problem: OdeProblem,
    pub y0: Vec<f64>,
    pub t_span: (f64, f64),
    /// atol = rtol * atol_scale.
    pub atol_scale: f64,
}

fn hires_problem() -> CoreResult<(OdeProblem, Vec<f64>)> {
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
    // One fill for both Jacobian callbacks: same values, fixed pattern.
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
    Ok((
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
        )?
        .with_jacobian_into(jacobian_into),
        vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0057],
    ))
}

/// The one-dimensional Brusselator of Hairer and Wanner (II, IV.1) with
/// `cells` interior points, interleaved as `u_1, v_1, u_2, v_2, ...`.
fn brusselator_problem(cells: usize) -> CoreResult<(OdeProblem, Vec<f64>)> {
    let c = (cells as f64 + 1.0).powi(2) / 50.0;
    let rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
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
    });
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
    let y0 = (0..cells)
        .flat_map(|i| {
            let x = (i as f64 + 1.0) / (cells as f64 + 1.0);
            [1.0 + (2.0 * std::f64::consts::PI * x).sin(), 3.0]
        })
        .collect();
    Ok((
        OdeProblem::new(
            format!("brusselator-1d-{cells}"),
            n,
            rhs,
            None,
            Some(jacobian),
            None,
            None,
            true,
            None,
            None,
        )?
        .with_jacobian_into(jacobian_into),
        y0,
    ))
}

/// The four problems of the first benchmark (the default selection).
pub const DEFAULT_PROBLEMS: [&str; 4] = [
    "robertson",
    "hires",
    "van-der-pol-mu1000",
    "brusselator-1d-50",
];

/// Every problem the benchmark can run: the defaults and the 400-component
/// Brusselator of the native comparison.
pub fn benchmark_problems() -> CoreResult<Vec<BenchmarkProblem>> {
    let mut problems = default_problems()?;
    let (large, large_y0) = brusselator_problem(200)?;
    problems.push(BenchmarkProblem {
        id: "brusselator-1d-200",
        problem: large,
        y0: large_y0,
        t_span: (0.0, 10.0),
        atol_scale: 1.0,
    });
    Ok(problems)
}

/// [`benchmark_problems`] plus `brusselator-1d-500` (n = 1000), for the
/// slope of the banded arms of speed research node SPD03, and
/// `brusselator-1d-30` (n = 60) and `brusselator-1d-40` (n = 80), on either
/// side of the column-extent threshold of speed research node SPD09
/// (`research/spd09_colext_threshold_20261007`). Only `stiff-profile-run`
/// and the SPD03/SPD09 tests use it, so the benchmark, its tests and the
/// SPD01/SPD02 identity exports keep their problem set.
pub fn profile_problems() -> CoreResult<Vec<BenchmarkProblem>> {
    let mut problems = benchmark_problems()?;
    for (id, cells) in [
        ("brusselator-1d-500", 500),
        ("brusselator-1d-30", 30),
        ("brusselator-1d-40", 40),
    ] {
        let (problem, y0) = brusselator_problem(cells)?;
        problems.push(BenchmarkProblem {
            id,
            problem,
            y0,
            t_span: (0.0, 10.0),
            atol_scale: 1.0,
        });
    }
    Ok(problems)
}

fn default_problems() -> CoreResult<Vec<BenchmarkProblem>> {
    let (robertson, robertson_y0) = robertson_problem()?;
    let (hires, hires_y0) = hires_problem()?;
    let (vdp, vdp_y0) = stiff_van_der_pol_problem(1.0e3)?;
    let (brusselator, brusselator_y0) = brusselator_problem(50)?;
    Ok(vec![
        BenchmarkProblem {
            id: "robertson",
            problem: robertson,
            y0: robertson_y0,
            t_span: (0.0, 40.0),
            atol_scale: 1.0e-4,
        },
        BenchmarkProblem {
            id: "hires",
            problem: hires,
            y0: hires_y0,
            t_span: (0.0, 321.8122),
            atol_scale: 1.0e-4,
        },
        BenchmarkProblem {
            id: "van-der-pol-mu1000",
            problem: vdp,
            y0: vdp_y0,
            t_span: (0.0, 2000.0),
            atol_scale: 1.0,
        },
        BenchmarkProblem {
            id: "brusselator-1d-50",
            problem: brusselator,
            y0: brusselator_y0,
            t_span: (0.0, 10.0),
            atol_scale: 1.0,
        },
    ])
}

fn adaptive_config(problem: &BenchmarkProblem, rtol: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: rtol * problem.atol_scale,
        rtol,
        initial_step: INITIAL_STEP,
        min_step: 1.0e-14,
        max_step: problem.t_span.1 - problem.t_span.0,
        max_attempts: MAX_ATTEMPTS,
        ..AdaptiveStepConfig::default()
    }
}

pub fn run_arm(
    arm: &str,
    problem: &BenchmarkProblem,
    rtol: f64,
) -> CoreResult<AdaptiveObservedIntegrationResult> {
    let adaptive = adaptive_config(problem, rtol);
    let output = OutputSchedule::new(vec![problem.t_span.0, problem.t_span.1])?;
    let (p, span, y0) = (&problem.problem, problem.t_span, problem.y0.as_slice());
    let tier_a = NewtonTolerancePolicy::ScaledToOuterTolerance;
    match arm {
        "rodas5p" => integrate_adaptive_observed_with_config(
            p,
            span,
            y0,
            IntegrationMethod::Sequential,
            Some(&LinearSolverConfig {
                method: LinearMethod::Direct,
                ..LinearSolverConfig::default()
            }),
            None,
            &adaptive,
            &output,
        ),
        FAST_ARM => {
            // Only the counts of the per-attempt diagnostics are filled; the
            // fast driver keeps no per-attempt vectors.
            let fast = integrate_rodas5p_fast_observed(p, span, y0, &adaptive, &output)?;
            Ok(AdaptiveObservedIntegrationResult {
                observed: fast.observed,
                diagnostics: AdaptiveRunDiagnostics {
                    attempts: fast.attempts,
                    accepted_macro_steps: fast.accepted_steps,
                    rejected_macro_steps: fast.rejected_steps,
                    ..AdaptiveRunDiagnostics::default()
                },
            })
        }
        SMALL_ARM => run_small(problem, &adaptive, &output, Rodas5pFastOptions::default()),
        "repo-bdf2" => {
            integrate_bdf_adaptive_observed(p, span, y0, &BdfConfig::default(), &adaptive, &output)
        }
        "repo-bdf2-tier-a" => integrate_bdf_adaptive_observed(
            p,
            span,
            y0,
            &BdfConfig {
                newton_tolerance: tier_a,
                ..BdfConfig::default()
            },
            &adaptive,
            &output,
        ),
        "repo-radau5" => integrate_radau_adaptive_observed(
            p,
            span,
            y0,
            &RadauConfig::default(),
            &adaptive,
            &output,
        ),
        "repo-radau5-tier-a" => integrate_radau_adaptive_observed(
            p,
            span,
            y0,
            &RadauConfig {
                newton_tolerance: tier_a,
                reuse_stage_lu_for_error_estimate: true,
                ..RadauConfig::default()
            },
            &adaptive,
            &output,
        ),
        other if banded_kernel_arm(other).is_some() => {
            run_banded(problem, rtol, banded_kernel_arm(other).unwrap()).map(|(r, _)| r)
        }
        other if small_static_arm(other).is_some() => run_small_static_full(
            problem,
            &adaptive,
            &output,
            small_static_arm(other).unwrap(),
        )
        .map(small_result),
        other => match option_arm(other) {
            Some((true, options)) => run_small(problem, &adaptive, &output, options),
            Some((false, options)) => {
                let fast = integrate_rodas5p_fast_observed_with_options(
                    p, span, y0, &adaptive, &output, options,
                )?;
                Ok(AdaptiveObservedIntegrationResult {
                    observed: fast.observed,
                    diagnostics: AdaptiveRunDiagnostics {
                        attempts: fast.attempts,
                        accepted_macro_steps: fast.accepted_steps,
                        rejected_macro_steps: fast.rejected_steps,
                        ..AdaptiveRunDiagnostics::default()
                    },
                })
            }
            None => Err(rodas5p_core::CoreError::InvalidInput(format!(
                "unknown benchmark arm {other}"
            ))),
        },
    }
}

fn counters_json(c: &WorkCounters) -> Value {
    json!({
        "rhs_evaluations": c.rhs_evaluations,
        "jacobian_builds": c.jacobian_builds,
        "direct_factorizations": c.direct_factorizations,
        "linear_solves": c.linear_solves,
        "direct_solve_calls": c.direct_solve_calls,
        "nonlinear_iterations": c.nonlinear_iterations,
    })
}

/// Deterministic sample states for the right-hand-side parity check.
fn parity_states(y0: &[f64]) -> Vec<Vec<f64>> {
    let perturbed = y0
        .iter()
        .enumerate()
        .map(|(i, &y)| y * (1.0 + 0.1 * ((i + 1) as f64).sin()) + 0.01 * ((i + 1) as f64).cos())
        .collect();
    vec![y0.to_vec(), perturbed]
}

fn parity_samples(problem: &BenchmarkProblem) -> Result<Value> {
    let p = &problem.problem;
    let mut samples = Vec::new();
    for state in parity_states(&problem.y0) {
        let mut counters = WorkCounters::default();
        let f = p.eval_rhs(problem.t_span.0, &state, &mut counters)?;
        let jac = p.dense_jacobian(problem.t_span.0, &state, &mut counters)?;
        let rows = (0..p.dimension)
            .map(|i| (0..p.dimension).map(|j| jac[(i, j)]).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        samples.push(json!({ "t": problem.t_span.0, "y": state, "f": f, "jacobian": rows }));
    }
    Ok(json!(samples))
}

/// Every arm, problem and tolerance: `warmups` untimed runs, then
/// `repetitions` timed runs, which must all end in the same state.
pub fn stiff_benchmark(
    repetitions: usize,
    warmups: usize,
    problem_ids: &[String],
    arms: &[String],
) -> Result<Value> {
    anyhow::ensure!(repetitions >= 1, "at least one timed repetition");
    for arm in arms {
        anyhow::ensure!(known_arm(arm), "unknown arm {arm}");
    }
    let mut problems = Vec::new();
    for problem in benchmark_problems()? {
        if problem_ids.iter().any(|id| id == problem.id) {
            problems.push(problem);
        }
    }
    anyhow::ensure!(
        problems.len() == problem_ids.len(),
        "unknown problem in {problem_ids:?}"
    );
    let mut rows = Vec::new();
    let mut parity = serde_json::Map::new();
    for problem in &problems {
        parity.insert(problem.id.into(), parity_samples(problem)?);
        for arm in arms.iter().map(String::as_str) {
            for &rtol in &TOLERANCES {
                for _ in 0..warmups {
                    let _ = run_arm(arm, problem, rtol);
                }
                let mut walls = Vec::with_capacity(repetitions);
                let mut first: Option<CoreResult<AdaptiveObservedIntegrationResult>> = None;
                let mut deterministic = true;
                for _ in 0..repetitions {
                    let started = Instant::now();
                    let result = run_arm(arm, problem, rtol);
                    walls.push(started.elapsed().as_secs_f64());
                    match (&first, &result) {
                        (None, _) => {}
                        (Some(Ok(a)), Ok(b)) => {
                            deterministic &= a.observed.y == b.observed.y
                                && a.observed.counters == b.observed.counters;
                        }
                        (Some(Err(_)), Err(_)) => {}
                        _ => deterministic = false,
                    }
                    if first.is_none() {
                        first = Some(result);
                    }
                }
                let mut sorted = walls.clone();
                sorted.sort_by(f64::total_cmp);
                let median = sorted[sorted.len() / 2];
                let row = match first.expect("one repetition") {
                    Ok(result) if result.observed.success => {
                        let d = &result.diagnostics;
                        json!({
                            "problem": problem.id, "arm": arm, "rtol": rtol,
                            "atol": rtol * problem.atol_scale,
                            "status": "completed",
                            "final_state": result.observed.y.last(),
                            "accepted_steps": d.accepted_macro_steps,
                            "rejected_steps": d.rejected_macro_steps,
                            "counters": counters_json(&result.observed.counters),
                            "wall_seconds": walls, "wall_median": median,
                            "deterministic": deterministic,
                        })
                    }
                    Ok(result) => json!({
                        "problem": problem.id, "arm": arm, "rtol": rtol,
                        "atol": rtol * problem.atol_scale,
                        "status": "failed", "message": result.observed.message,
                        "counters": counters_json(&result.observed.counters),
                        "wall_seconds": walls, "wall_median": median,
                        "deterministic": deterministic,
                    }),
                    Err(error) => json!({
                        "problem": problem.id, "arm": arm, "rtol": rtol,
                        "atol": rtol * problem.atol_scale,
                        "status": "failed", "message": error.to_string(),
                        "wall_seconds": walls, "wall_median": median,
                        "deterministic": deterministic,
                    }),
                };
                eprintln!(
                    "{} {} rtol={rtol:e}: {} median {:.4e} s",
                    problem.id, arm, row["status"], median
                );
                rows.push(row);
            }
        }
    }
    Ok(json!({
        "schema": SCHEMA,
        "timing_status": "DIAGNOSTIC: wall seconds on a shared host; statistical timing authority is on hold",
        "repetitions": repetitions,
        "warmups": warmups,
        "initial_step": INITIAL_STEP,
        "max_attempts": MAX_ATTEMPTS,
        "problems": problems.iter().map(|p| json!({
            "id": p.id, "dimension": p.problem.dimension, "t_span": [p.t_span.0, p.t_span.1],
            "y0": p.y0, "atol_scale": p.atol_scale,
        })).collect::<Vec<_>>(),
        "parity_samples": parity,
        "lu_microbench": lu_microbench(&[50, 200], 51)?,
        "rows": rows,
    }))
}

/// Run one arm on one problem `repetitions` times and nothing else: the
/// workload of the profiling node (`research/stiff_rodas5p_profile_20261002`).
/// Reports the work of one run and checks that every repetition ends in the
/// same state.
pub fn profile_run(problem_id: &str, arm: &str, rtol: f64, repetitions: usize) -> Result<Value> {
    anyhow::ensure!(repetitions >= 1, "at least one repetition");
    anyhow::ensure!(known_arm(arm), "unknown arm {arm}");
    let problem = profile_problems()?
        .into_iter()
        .find(|p| p.id == problem_id)
        .ok_or_else(|| anyhow::anyhow!("unknown problem {problem_id}"))?;
    let kernel = banded_kernel_arm(arm);
    let run = |problem: &BenchmarkProblem| -> CoreResult<(
        AdaptiveObservedIntegrationResult,
        Option<BandedWork>,
    )> {
        match kernel {
            Some(kernel) => run_banded(problem, rtol, kernel).map(|(r, w)| (r, Some(w))),
            None => run_arm(arm, problem, rtol).map(|r| (r, None)),
        }
    };
    let (first, work) = run(&problem)?;
    let mut deterministic = true;
    for _ in 1..repetitions {
        let (again, _) = run(&problem)?;
        deterministic &= again.observed.y == first.observed.y;
    }
    let d = &first.diagnostics;
    let mut out = json!({
        "problem": problem_id, "arm": arm, "rtol": rtol, "repetitions": repetitions,
        "success": first.observed.success,
        "attempts": d.attempts,
        "accepted_steps": d.accepted_macro_steps,
        "rejected_steps": d.rejected_macro_steps,
        "counters": counters_json(&first.observed.counters),
        "final_state": first.observed.y.last(),
        "deterministic": deterministic,
    });
    if let Some(work) = work {
        out["banded_work"] = json!({
            "factor_operations": work.factor_operations,
            "solve_operations": work.solve_operations,
            "stored_slots": work.stored_slots,
        });
    }
    Ok(out)
}

/// Median seconds of one dense LU factorization (faer partial pivoting, as
/// used by the RODAS5P arm) of two `2 cells` square matrices: the Brusselator
/// iteration matrix `I - 0.05 J(y0)` ("banded") and the full matrix
/// `1/(1 + |i - j|) + n delta_ij`, the same pair the native driver times.
fn lu_microbench(cells: &[usize], repetitions: usize) -> Result<Value> {
    let mut out = serde_json::Map::new();
    for &c in cells {
        let (problem, y0) = brusselator_problem(c)?;
        let n = problem.dimension;
        let mut counters = WorkCounters::default();
        let jac = problem.dense_jacobian(0.0, &y0, &mut counters)?;
        let mut banded = DenseMatrix::zeros(n, n);
        let mut full = DenseMatrix::zeros(n, n);
        for i in 0..n {
            for j in 0..n {
                let delta = f64::from(u8::from(i == j));
                banded[(i, j)] = delta - 0.05 * jac[(i, j)];
                full[(i, j)] = 1.0 / (1.0 + i.abs_diff(j) as f64) + n as f64 * delta;
            }
        }
        let mut entry = serde_json::Map::new();
        for (label, matrix) in [("banded", &banded), ("full", &full)] {
            let mut walls = Vec::with_capacity(repetitions);
            for _ in 0..repetitions {
                let started = Instant::now();
                let lu = rodas5p_core::LuFactorization::new(matrix)?;
                walls.push(started.elapsed().as_secs_f64());
                std::hint::black_box(&lu);
            }
            walls.sort_by(f64::total_cmp);
            entry.insert(
                label.into(),
                json!({ "faer-partial-pivot-lu": walls[walls.len() / 2] }),
            );
        }
        out.insert(n.to_string(), Value::Object(entry));
    }
    Ok(Value::Object(out))
}

/// The small-n specialization of the fast driver
/// (`research/thread_transfer_smalln_cost_20261002`): van der Pol, Robertson
/// and HIRES with compile-time dimension and static dispatch. Not in
/// [`ARMS`].
pub const SMALL_ARM: &str = "rodas5p-fast-small";

/// van der Pol with the same operations as `stiff_van_der_pol_problem`.
struct SmallVanDerPol {
    mu: f64,
}

impl SmallProblem<2> for SmallVanDerPol {
    fn rhs(&self, y: &[f64; 2], out: &mut [f64; 2]) {
        let mu = self.mu;
        out[0] = y[1];
        out[1] = mu * (1.0 - y[0] * y[0]) * y[1] - y[0];
    }
    fn jacobian(&self, y: &[f64; 2], out: &mut [[f64; 2]; 2]) {
        let mu = self.mu;
        out[0][0] = 0.0;
        out[0][1] = 1.0;
        out[1][0] = -2.0 * mu * y[0] * y[1] - 1.0;
        out[1][1] = mu * (1.0 - y[0] * y[0]);
    }
}

/// [`SmallVanDerPol`] on `B` lanes with the scalar expression order (speed
/// research node `research/spd08_small_ensemble_lanes_20261007`); the lane
/// data is `mu` per lane.
impl<const B: usize> BatchProblem<2, B> for SmallVanDerPol {
    type Lanes = [f64; B];
    fn lanes(member: &Self) -> [f64; B] {
        [member.mu; B]
    }
    fn set_lane(lanes: &mut [f64; B], lane: usize, member: &Self) {
        lanes[lane] = member.mu;
    }
    fn rhs_batch(mu: &[f64; B], y: &[[f64; B]; 2], out: &mut [[f64; B]; 2]) {
        let [y0, y1] = y;
        let [out0, out1] = out;
        for l in 0..B {
            let mu = mu[l];
            out0[l] = y1[l];
            out1[l] = mu * (1.0 - y0[l] * y0[l]) * y1[l] - y0[l];
        }
    }
}

/// Robertson with the same operations as `robertson_problem`.
struct SmallRobertson;

impl SmallProblem<3> for SmallRobertson {
    fn rhs(&self, y: &[f64; 3], out: &mut [f64; 3]) {
        out[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
        out[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
        out[2] = 3.0e7 * y[1] * y[1];
    }
    fn jacobian(&self, y: &[f64; 3], out: &mut [[f64; 3]; 3]) {
        out[0][0] = -0.04;
        out[0][1] = 1.0e4 * y[2];
        out[0][2] = 1.0e4 * y[1];
        out[1][0] = 0.04;
        out[1][1] = -1.0e4 * y[2] - 6.0e7 * y[1];
        out[1][2] = -1.0e4 * y[1];
        out[2][1] = 6.0e7 * y[1];
    }
}

/// HIRES with the same operations as [`hires_problem`].
struct SmallHires;

impl SmallProblem<8> for SmallHires {
    fn rhs(&self, y: &[f64; 8], out: &mut [f64; 8]) {
        out[0] = -1.71 * y[0] + 0.43 * y[1] + 8.32 * y[2] + 0.0007;
        out[1] = 1.71 * y[0] - 8.75 * y[1];
        out[2] = -10.03 * y[2] + 0.43 * y[3] + 0.035 * y[4];
        out[3] = 8.32 * y[1] + 1.71 * y[2] - 1.12 * y[3];
        out[4] = -1.745 * y[4] + 0.43 * y[5] + 0.43 * y[6];
        out[5] = -280.0 * y[5] * y[7] + 0.69 * y[3] + 1.71 * y[4] - 0.43 * y[5] + 0.69 * y[6];
        out[6] = 280.0 * y[5] * y[7] - 1.81 * y[6];
        out[7] = -280.0 * y[5] * y[7] + 1.81 * y[6];
    }
    fn jacobian(&self, y: &[f64; 8], j: &mut [[f64; 8]; 8]) {
        j[0][0] = -1.71;
        j[0][1] = 0.43;
        j[0][2] = 8.32;
        j[1][0] = 1.71;
        j[1][1] = -8.75;
        j[2][2] = -10.03;
        j[2][3] = 0.43;
        j[2][4] = 0.035;
        j[3][1] = 8.32;
        j[3][2] = 1.71;
        j[3][3] = -1.12;
        j[4][4] = -1.745;
        j[4][5] = 0.43;
        j[4][6] = 0.43;
        j[5][3] = 0.69;
        j[5][4] = 1.71;
        j[5][5] = -280.0 * y[7] - 0.43;
        j[5][6] = 0.69;
        j[5][7] = -280.0 * y[5];
        j[6][5] = 280.0 * y[7];
        j[6][6] = -1.81;
        j[6][7] = 280.0 * y[5];
        j[7][5] = -280.0 * y[7];
        j[7][6] = 1.81;
        j[7][7] = -280.0 * y[5];
    }
}

fn small_result(
    run: rodas5p_integrators::Rodas5pFastSmallResult,
) -> AdaptiveObservedIntegrationResult {
    AdaptiveObservedIntegrationResult {
        observed: run.observed,
        diagnostics: AdaptiveRunDiagnostics {
            attempts: run.attempts,
            accepted_macro_steps: run.accepted_steps,
            rejected_macro_steps: run.rejected_steps,
            ..AdaptiveRunDiagnostics::default()
        },
    }
}

fn fixed<const N: usize>(y0: &[f64]) -> CoreResult<[f64; N]> {
    y0.try_into().map_err(|_| {
        rodas5p_core::CoreError::Dimension(format!("the small arm expects dimension {N}"))
    })
}

/// The small arm on one benchmark problem; `mu` overrides van der Pol's
/// stiffness (the ensemble).
fn run_small(
    problem: &BenchmarkProblem,
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    options: Rodas5pFastOptions,
) -> CoreResult<AdaptiveObservedIntegrationResult> {
    run_small_full(problem, adaptive, output, options).map(small_result)
}

/// The small driver with the SPD06 options on one benchmark problem, with
/// the full result.
fn run_small_static_full(
    problem: &BenchmarkProblem,
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    options: Rodas5pFastSmallOptions,
) -> CoreResult<rodas5p_integrators::Rodas5pFastSmallResult> {
    let span = problem.t_span;
    match problem.id {
        "van-der-pol-mu1000" => integrate_rodas5p_fast_small_observed_with_small_options(
            &SmallVanDerPol { mu: 1000.0 },
            span,
            &fixed::<2>(&problem.y0)?,
            adaptive,
            output,
            options,
        ),
        "robertson" => integrate_rodas5p_fast_small_observed_with_small_options(
            &SmallRobertson,
            span,
            &fixed::<3>(&problem.y0)?,
            adaptive,
            output,
            options,
        ),
        "hires" => integrate_rodas5p_fast_small_observed_with_small_options(
            &SmallHires,
            span,
            &fixed::<8>(&problem.y0)?,
            adaptive,
            output,
            options,
        ),
        other => Err(rodas5p_core::CoreError::InvalidInput(format!(
            "the small arm covers van der Pol, Robertson and HIRES, not {other}"
        ))),
    }
}

/// The small driver on one benchmark problem with the full result.
fn run_small_full(
    problem: &BenchmarkProblem,
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    options: Rodas5pFastOptions,
) -> CoreResult<rodas5p_integrators::Rodas5pFastSmallResult> {
    let span = problem.t_span;
    match problem.id {
        "van-der-pol-mu1000" => integrate_rodas5p_fast_small_observed_with_options(
            &SmallVanDerPol { mu: 1000.0 },
            span,
            &fixed::<2>(&problem.y0)?,
            adaptive,
            output,
            options,
        ),
        "robertson" => integrate_rodas5p_fast_small_observed_with_options(
            &SmallRobertson,
            span,
            &fixed::<3>(&problem.y0)?,
            adaptive,
            output,
            options,
        ),
        "hires" => integrate_rodas5p_fast_small_observed_with_options(
            &SmallHires,
            span,
            &fixed::<8>(&problem.y0)?,
            adaptive,
            output,
            options,
        ),
        other => Err(rodas5p_core::CoreError::InvalidInput(format!(
            "the small arm covers van der Pol, Robertson and HIRES, not {other}"
        ))),
    }
}

/// The lane-batched ensemble arms of speed research node SPD08
/// (`research/spd08_small_ensemble_lanes_20261007`): the small driver's
/// arithmetic on 4 or 8 lanes. Only in `stiff-ensemble-run`.
pub const SMALL_BATCH4_ARM: &str = "rodas5p-fast-small-batch4";
pub const SMALL_BATCH8_ARM: &str = "rodas5p-fast-small-batch8";

/// The member results of one ensemble run with `arm`, in member order.
fn ensemble_members(
    arm: &str,
    members: usize,
    base: &BenchmarkProblem,
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
) -> Result<Vec<Rodas5pFastSmallResult>> {
    let mu = |k: usize| 1000.0 * (1.0 + k as f64 / members as f64);
    if arm == SMALL_BATCH4_ARM || arm == SMALL_BATCH8_ARM {
        let y0 = fixed::<2>(&base.y0)?;
        let batch: Vec<_> = (0..members)
            .map(|k| SmallBatchMember {
                problem: SmallVanDerPol { mu: mu(k) },
                t_span: base.t_span,
                y0,
            })
            .collect();
        let runs = if arm == SMALL_BATCH4_ARM {
            integrate_rodas5p_fast_small_batch::<2, 4, _>(&batch, adaptive, output)
        } else {
            integrate_rodas5p_fast_small_batch::<2, 8, _>(&batch, adaptive, output)
        };
        return Ok(runs.into_iter().collect::<CoreResult<Vec<_>>>()?);
    }
    // Speed research node SPD06: the gated static arm runs the ensemble too.
    let small_static = (arm == SMALL_STATIC_ARMS[0]).then(|| small_static_arm(arm).unwrap());
    let mut runs = Vec::with_capacity(members);
    for k in 0..members {
        let mu = mu(k);
        let run = if let Some(options) = small_static {
            integrate_rodas5p_fast_small_observed_with_small_options(
                &SmallVanDerPol { mu },
                base.t_span,
                &fixed::<2>(&base.y0)?,
                adaptive,
                output,
                options,
            )?
        } else if arm == SMALL_ARM {
            integrate_rodas5p_fast_small_observed(
                &SmallVanDerPol { mu },
                base.t_span,
                &fixed::<2>(&base.y0)?,
                adaptive,
                output,
            )?
        } else {
            let (problem, _) = stiff_van_der_pol_problem(mu)?;
            let fast =
                integrate_rodas5p_fast_observed(&problem, base.t_span, &base.y0, adaptive, output)?;
            Rodas5pFastSmallResult {
                observed: fast.observed,
                attempts: fast.attempts,
                accepted_steps: fast.accepted_steps,
                rejected_steps: fast.rejected_steps,
                jacobian_reuses: fast.jacobian_reuses,
                driver: fast.driver,
            }
        };
        runs.push(run);
    }
    Ok(runs)
}

fn f64_bits_hex(v: f64) -> String {
    format!("{:016x}", v.to_bits())
}

/// One member of a `--dump-members` file: everything the SPD08 identity gate
/// compares, floats as IEEE bit patterns.
fn member_dump(k: usize, run: &Rodas5pFastSmallResult) -> Value {
    let observed = &run.observed;
    json!({
        "member": k,
        "success": observed.success,
        "attempts": run.attempts,
        "accepted": run.accepted_steps,
        "rejected": run.rejected_steps,
        "reuses": run.jacobian_reuses,
        "counters": observed.counters,
        "internal_steps": observed.internal_steps,
        "output_clipped_steps": observed.output_clipped_steps,
        "output_times_bits": observed.t.iter().map(|t| f64_bits_hex(*t)).collect::<Vec<_>>(),
        "output_states_bits": observed
            .y
            .iter()
            .map(|y| y.iter().map(|v| f64_bits_hex(*v)).collect::<Vec<_>>())
            .collect::<Vec<_>>(),
        "final_state_bits": observed
            .y
            .last()
            .map(|y| y.iter().map(|v| f64_bits_hex(*v)).collect::<Vec<_>>()),
    })
}

/// An ensemble of `members` van der Pol trajectories,
/// `mu = 1000 (1 + k / members)`, run back to back with `arm`
/// (`rodas5p-fast`, `rodas5p-fast-small` or SPD06's
/// `rodas5p-fast-small-static`) or on lanes
/// (`rodas5p-fast-small-batch4`, `-batch8`, SPD08): the summed attempts and a
/// checksum of the final states, accumulated in member order after all
/// members finished. The whole ensemble runs `repetitions` times (SPD08's
/// repetition protocol); every repetition must give the same attempts and
/// checksum bits, and the report is one repetition's. `dump` receives the
/// per-member results of the first repetition.
pub fn ensemble_run(
    arm: &str,
    members: usize,
    rtol: f64,
    repetitions: usize,
    dump: Option<&std::path::Path>,
) -> Result<Value> {
    anyhow::ensure!(members >= 1, "at least one member");
    anyhow::ensure!(repetitions >= 1, "at least one repetition");
    anyhow::ensure!(
        arm == FAST_ARM
            || arm == SMALL_ARM
            || arm == SMALL_STATIC_ARMS[0]
            || arm == SMALL_BATCH4_ARM
            || arm == SMALL_BATCH8_ARM,
        "ensemble arms: {FAST_ARM}, {SMALL_ARM}, {}, {SMALL_BATCH4_ARM}, {SMALL_BATCH8_ARM}",
        SMALL_STATIC_ARMS[0]
    );
    // The setup is per process; a repetition integrates the whole ensemble.
    let base = benchmark_problems()?
        .into_iter()
        .find(|p| p.id == "van-der-pol-mu1000")
        .expect("van der Pol is a benchmark problem");
    let adaptive = adaptive_config(&base, rtol);
    let output = OutputSchedule::new(vec![base.t_span.0, base.t_span.1])?;
    let mut report: Option<(usize, f64)> = None;
    let mut first_runs = None;
    for _ in 0..repetitions {
        let runs = ensemble_members(arm, members, &base, &adaptive, &output)?;
        let (mut attempts, mut checksum) = (0_usize, 0.0_f64);
        for (k, run) in runs.iter().enumerate() {
            anyhow::ensure!(run.observed.success, "member {k} failed");
            attempts += run.attempts;
            checksum += run.observed.y.last().unwrap().iter().sum::<f64>();
        }
        match report {
            None => report = Some((attempts, checksum)),
            Some((a, c)) => anyhow::ensure!(
                a == attempts && c.to_bits() == checksum.to_bits(),
                "repetitions differ: attempts {a} vs {attempts}, checksum {c} vs {checksum}"
            ),
        }
        if dump.is_some() && first_runs.is_none() {
            first_runs = Some(runs);
        }
    }
    let (attempts, checksum) = report.expect("at least one repetition");
    if let (Some(path), Some(runs)) = (dump, first_runs) {
        let doc = json!({
            "schema": "vigilode-spd08-ensemble-members-v1",
            "arm": arm,
            "members": members,
            "rtol": rtol,
            "results": runs.iter().enumerate().map(|(k, run)| member_dump(k, run)).collect::<Vec<_>>(),
        });
        std::fs::write(path, serde_json::to_string_pretty(&doc)? + "\n")?;
    }
    Ok(json!({
        "arm": arm,
        "members": members,
        "rtol": rtol,
        "repetitions": repetitions,
        "attempts": attempts,
        "checksum": checksum,
        "checksum_bits": f64_bits_hex(checksum),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analytic_jacobians_match_finite_differences() {
        for problem in benchmark_problems().unwrap() {
            if problem.problem.dimension > 100 {
                continue;
            }
            let p = &problem.problem;
            let n = p.dimension;
            for state in parity_states(&problem.y0) {
                let mut counters = WorkCounters::default();
                let jac = p.dense_jacobian(0.0, &state, &mut counters).unwrap();
                // Central differences are exact for the quadratic terms
                // (Robertson at y2 = 0 has a zero Jacobian entry whose
                // one-sided difference is -3e7 h).
                for j in 0..n {
                    let h = 1.0e-5 * state[j].abs().max(1.0);
                    let (mut up, mut down) = (state.clone(), state.clone());
                    up[j] += h;
                    down[j] -= h;
                    let fu = p.eval_rhs(0.0, &up, &mut counters).unwrap();
                    let fdn = p.eval_rhs(0.0, &down, &mut counters).unwrap();
                    for i in 0..n {
                        let fd = (fu[i] - fdn[i]) / (2.0 * h);
                        let scale = jac[(i, j)].abs().max(1.0);
                        assert!(
                            (fd - jac[(i, j)]).abs() <= 1.0e-6 * scale,
                            "{} J[{i},{j}] = {} vs {fd}",
                            problem.id,
                            jac[(i, j)]
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn in_place_jacobians_equal_the_explicit_ones() {
        for problem in benchmark_problems().unwrap() {
            let p = &problem.problem;
            let n = p.dimension;
            let mut buffer = DenseMatrix::zeros(n, n);
            let mut counters = WorkCounters::default();
            for state in parity_states(&problem.y0) {
                // The buffer keeps the previous Jacobian, as the contract allows.
                p.dense_jacobian_into(0.0, &state, &mut buffer, &mut counters)
                    .unwrap();
                let explicit = p.dense_jacobian(0.0, &state, &mut counters).unwrap();
                assert_eq!(buffer.as_slice(), explicit.as_slice(), "{}", problem.id);
            }
        }
    }

    #[test]
    fn every_arm_completes_a_loose_hires_run() {
        let problems = benchmark_problems().unwrap();
        let hires = problems.iter().find(|p| p.id == "hires").unwrap();
        for arm in ARMS.into_iter().chain([FAST_ARM]) {
            let result = run_arm(arm, hires, 1.0e-3).unwrap();
            assert!(
                result.observed.success,
                "{arm}: {}",
                result.observed.message
            );
        }
    }
}

/// Identity export of speed research node SPD01
/// (`research/spd01_fast_driver_overhead_20261005`): every option set of the
/// dense and small fast drivers against the legacy driver on the benchmark
/// problems at the seven tolerances, as IEEE bits.
#[cfg(test)]
mod spd01 {
    use super::*;

    fn hx(v: f64) -> String {
        format!("{:016x}", v.to_bits())
    }

    fn row(
        observed: &rodas5p_integrators::ObservedIntegrationResult,
        attempts: usize,
        accepted: usize,
        rejected: usize,
        reuses: usize,
        driver: &str,
    ) -> Value {
        json!({
            "driver": driver, "success": observed.success, "message": observed.message,
            "t": observed.t.iter().map(|v| hx(*v)).collect::<Vec<_>>(),
            "y": observed.y.iter().map(|s| s.iter().map(|v| hx(*v)).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "attempts": attempts, "accepted_steps": accepted, "rejected_steps": rejected,
            "jacobian_reuses": reuses, "internal_steps": observed.internal_steps,
            "output_clipped_steps": observed.output_clipped_steps,
            "counters": serde_json::to_value(observed.counters).unwrap(),
        })
    }

    const SETS: [(&str, Rodas5pFastOptions); 4] = [
        (
            "legacy",
            Rodas5pFastOptions {
                prevalidated_controller: false,
                fused_landing: false,
                lu_policy: FastLuPolicy::Legacy,
            },
        ),
        (
            "val",
            Rodas5pFastOptions {
                prevalidated_controller: true,
                fused_landing: false,
                lu_policy: FastLuPolicy::Legacy,
            },
        ),
        (
            "land",
            Rodas5pFastOptions {
                prevalidated_controller: false,
                fused_landing: true,
                lu_policy: FastLuPolicy::Legacy,
            },
        ),
        (
            "ovh",
            Rodas5pFastOptions {
                prevalidated_controller: true,
                fused_landing: true,
                lu_policy: FastLuPolicy::Legacy,
            },
        ),
    ];

    #[test]
    #[ignore = "identity export of research/spd01_fast_driver_overhead_20261005; release build; set SPD01_IDENTITY"]
    fn spd01_identity_export() {
        let Ok(path) = std::env::var("SPD01_IDENTITY") else {
            println!("SPD01_IDENTITY not set: nothing written");
            return;
        };
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path);
        assert!(
            !path.exists(),
            "immutable output exists: {}",
            path.display()
        );
        let mut rows = Vec::new();
        for problem in benchmark_problems().unwrap() {
            for rtol in TOLERANCES {
                let adaptive = adaptive_config(&problem, rtol);
                let output = OutputSchedule::new(vec![problem.t_span.0, problem.t_span.1]).unwrap();
                let mut entry =
                    json!({"problem": problem.id, "rtol": rtol, "dense": {}, "small": {}});
                for (name, options) in SETS {
                    let fast = integrate_rodas5p_fast_observed_with_options(
                        &problem.problem,
                        problem.t_span,
                        &problem.y0,
                        &adaptive,
                        &output,
                        options,
                    )
                    .unwrap();
                    entry["dense"][name] = row(
                        &fast.observed,
                        fast.attempts,
                        fast.accepted_steps,
                        fast.rejected_steps,
                        fast.jacobian_reuses,
                        fast.driver,
                    );
                    if ["van-der-pol-mu1000", "robertson", "hires"].contains(&problem.id) {
                        let small = run_small_full(&problem, &adaptive, &output, options).unwrap();
                        entry["small"][name] = row(
                            &small.observed,
                            small.attempts,
                            small.accepted_steps,
                            small.rejected_steps,
                            small.jacobian_reuses,
                            small.driver,
                        );
                    }
                }
                let same = |sets: &serde_json::Map<String, Value>| -> bool {
                    sets.is_empty()
                        || sets.values().all(|v| {
                            let mut a = v.clone();
                            let mut b = sets["legacy"].clone();
                            a["driver"] = Value::Null;
                            b["driver"] = Value::Null;
                            a == b
                        })
                };
                let dense_same = same(entry["dense"].as_object().unwrap());
                let small_same = same(entry["small"].as_object().unwrap());
                entry["dense_identical"] = json!(dense_same);
                entry["small_identical"] = json!(small_same);
                println!(
                    "{} {:e}: dense identical {}, small identical {}",
                    problem.id, rtol, entry["dense_identical"], entry["small_identical"]
                );
                rows.push(entry);
            }
        }
        let out = json!({"schema": "vigilode-spd01-identity-v1", "rows": rows});
        std::fs::write(&path, serde_json::to_string(&out).unwrap() + "\n").unwrap();
        println!("wrote {}", path.display());
    }

    /// The option arms refuse an invalid configuration exactly as the legacy
    /// arm does (the entry validation is kept).
    #[test]
    fn option_arms_keep_the_entry_validation() {
        let problem = benchmark_problems()
            .unwrap()
            .into_iter()
            .find(|p| p.id == "robertson")
            .unwrap();
        let output = OutputSchedule::new(vec![problem.t_span.0, problem.t_span.1]).unwrap();
        let mut bad = adaptive_config(&problem, 1.0e-6);
        bad.safety = f64::NAN;
        for (_, options) in SETS {
            let dense = integrate_rodas5p_fast_observed_with_options(
                &problem.problem,
                problem.t_span,
                &problem.y0,
                &bad,
                &output,
                options,
            );
            let small = run_small_full(&problem, &bad, &output, options);
            assert!(dense.is_err() && small.is_err());
        }
        for arm in OPTION_ARMS {
            assert!(known_arm(arm));
            assert!(!ARMS.contains(&arm));
        }
        assert!(option_arm("rodas5p-fast").is_none());
        assert!(option_arm("rodas5p-fast-small").is_none());
    }

    /// Identity export of speed research node SPD02
    /// (`research/spd02_lu_column_extents_20261005`): the column-extent LU
    /// against the legacy dense driver on the five benchmark problems at
    /// the seven tolerances.
    #[test]
    #[ignore = "identity export of research/spd02_lu_column_extents_20261005; release build; set SPD02_IDENTITY"]
    fn spd02_identity_export() {
        let Ok(path) = std::env::var("SPD02_IDENTITY") else {
            println!("SPD02_IDENTITY not set: nothing written");
            return;
        };
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path);
        assert!(
            !path.exists(),
            "immutable output exists: {}",
            path.display()
        );
        let colext = Rodas5pFastOptions {
            lu_policy: FastLuPolicy::ColumnExtents,
            ..Rodas5pFastOptions::default()
        };
        let mut rows = Vec::new();
        for problem in benchmark_problems().unwrap() {
            for rtol in TOLERANCES {
                let adaptive = adaptive_config(&problem, rtol);
                let output = OutputSchedule::new(vec![problem.t_span.0, problem.t_span.1]).unwrap();
                let legacy = integrate_rodas5p_fast_observed(
                    &problem.problem,
                    problem.t_span,
                    &problem.y0,
                    &adaptive,
                    &output,
                )
                .unwrap();
                let fast = integrate_rodas5p_fast_observed_with_options(
                    &problem.problem,
                    problem.t_span,
                    &problem.y0,
                    &adaptive,
                    &output,
                    colext,
                )
                .unwrap();
                let a = row(
                    &legacy.observed,
                    legacy.attempts,
                    legacy.accepted_steps,
                    legacy.rejected_steps,
                    legacy.jacobian_reuses,
                    legacy.driver,
                );
                let b = row(
                    &fast.observed,
                    fast.attempts,
                    fast.accepted_steps,
                    fast.rejected_steps,
                    fast.jacobian_reuses,
                    fast.driver,
                );
                let (mut a0, mut b0) = (a.clone(), b.clone());
                a0["driver"] = Value::Null;
                b0["driver"] = Value::Null;
                let identical = a0 == b0;
                println!(
                    "{} {:e}: identical {identical}, lu {:?} / {:?}",
                    problem.id, rtol, legacy.lu, fast.lu
                );
                rows.push(
                    json!({"problem": problem.id, "rtol": rtol, "identical": identical,
                                 "legacy_lu": format!("{:?}", legacy.lu),
                                 "colext_lu": format!("{:?}", fast.lu),
                                 "legacy": a, "colext": b}),
                );
            }
        }
        let out = json!({"schema": "vigilode-spd02-identity-v1", "rows": rows});
        std::fs::write(&path, serde_json::to_string(&out).unwrap() + "\n").unwrap();
        println!("wrote {}", path.display());
    }

    /// Speed research node SPD03: the native band fill of the Brusselator
    /// equals the dense fill entry for entry on the parity states.
    #[test]
    fn spd03_band_fill_equals_dense_fill() {
        for problem in profile_problems().unwrap() {
            let Some(cells) = brusselator_cells(problem.id) else {
                continue;
            };
            let band = brusselator_band(cells);
            let n = problem.problem.dimension;
            let mut counters = WorkCounters::default();
            for state in parity_states(&problem.y0) {
                let mut dense = DenseMatrix::zeros(n, n);
                problem
                    .problem
                    .dense_jacobian_into(0.0, &state, &mut dense, &mut counters)
                    .unwrap();
                let mut filled = vec![0.0; n * 5];
                (band.fill)(0.0, &state, &mut filled).unwrap();
                let mut entries = 0;
                for i in 0..n {
                    for j in 0..n {
                        let d = dense[(i, j)];
                        let b = if j + 2 >= i && j <= i + 2 {
                            filled[i * 5 + (j + 2 - i)]
                        } else {
                            0.0
                        };
                        assert_eq!(d.to_bits(), b.to_bits(), "{} ({i}, {j})", problem.id);
                        if d != 0.0 {
                            entries += 1;
                        }
                    }
                }
                assert!(entries >= 4 * cells, "{}: {entries} nonzeros", problem.id);
            }
        }
        for arm in [BANDED_ARM, BANDED_SLICES_ARM] {
            assert!(known_arm(arm) && !ARMS.contains(&arm));
        }
        let hires = benchmark_problems()
            .unwrap()
            .into_iter()
            .find(|p| p.id == "hires")
            .unwrap();
        assert!(run_banded(&hires, 1.0e-6, BandedKernel::Indexed).is_err());
    }

    /// Identity export of speed research node SPD03: both banded arms
    /// against v2 on the CLI Brusselators at the seven tolerances.
    #[test]
    #[ignore = "identity export of research/spd03_banded_arm_instructions_20261005; release build; set SPD03_IDENTITY"]
    fn spd03_identity_export() {
        let Ok(path) = std::env::var("SPD03_IDENTITY") else {
            println!("SPD03_IDENTITY not set: nothing written");
            return;
        };
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path);
        assert!(
            !path.exists(),
            "immutable output exists: {}",
            path.display()
        );
        let mut rows = Vec::new();
        for problem in benchmark_problems().unwrap() {
            if !["brusselator-1d-50", "brusselator-1d-200"].contains(&problem.id) {
                continue;
            }
            let cells = brusselator_cells(problem.id).unwrap();
            for rtol in TOLERANCES {
                let adaptive = adaptive_config(&problem, rtol);
                let output = OutputSchedule::new(vec![problem.t_span.0, problem.t_span.1]).unwrap();
                let v2 = integrate_rodas5p_fast_observed(
                    &problem.problem,
                    problem.t_span,
                    &problem.y0,
                    &adaptive,
                    &output,
                )
                .unwrap();
                let v2_row = row(
                    &v2.observed,
                    v2.attempts,
                    v2.accepted_steps,
                    v2.rejected_steps,
                    v2.jacobian_reuses,
                    "",
                );
                let mut entry = json!({"problem": problem.id, "rtol": rtol, "v2": v2_row.clone()});
                let mut works = Vec::new();
                for (name, kernel) in [
                    ("banded", BandedKernel::Indexed),
                    ("slices", BandedKernel::Slices),
                ] {
                    let run = integrate_rodas5p_fast_banded_observed_with_kernel(
                        &problem.problem,
                        &brusselator_band(cells),
                        problem.t_span,
                        &problem.y0,
                        &adaptive,
                        &output,
                        kernel,
                    )
                    .unwrap();
                    let r = row(
                        &run.fast.observed,
                        run.fast.attempts,
                        run.fast.accepted_steps,
                        run.fast.rejected_steps,
                        run.fast.jacobian_reuses,
                        "",
                    );
                    entry[format!("{name}_identical")] = json!(r == v2_row);
                    entry[format!("{name}_driver")] = json!(run.fast.driver);
                    entry[format!("{name}_work")] = serde_json::to_value(run.work).unwrap();
                    works.push(run.work);
                }
                entry["work_equal"] = json!(works[0] == works[1]);
                println!(
                    "{} {:e}: banded {} slices {} work_equal {}",
                    problem.id,
                    rtol,
                    entry["banded_identical"],
                    entry["slices_identical"],
                    entry["work_equal"]
                );
                rows.push(entry);
            }
        }
        let out = json!({"schema": "vigilode-spd03-identity-v1", "cli_rows": rows});
        std::fs::write(&path, serde_json::to_string(&out).unwrap() + "\n").unwrap();
        println!("wrote {}", path.display());
    }
}

/// Speed research node SPD06 (`research/spd06_small_static_stages_20261007`):
/// the CLI arms of the fixed stage structure. The identity export is the
/// library test `crates/rodas5p-integrators/tests/spd06_small_static_stages.rs`.
#[cfg(test)]
mod spd06 {
    use super::*;

    #[test]
    fn the_static_arms_are_known_and_equal_the_small_arm() {
        for arm in SMALL_STATIC_ARMS {
            assert!(known_arm(arm) && !ARMS.contains(&arm) && !OPTION_ARMS.contains(&arm));
            assert!(option_arm(arm).is_none());
        }
        assert!(small_static_arm(SMALL_ARM).is_none());
        let problems = benchmark_problems().unwrap();
        for id in ["van-der-pol-mu1000", "robertson", "hires"] {
            let problem = problems.iter().find(|p| p.id == id).unwrap();
            let legacy = run_arm(SMALL_ARM, problem, 1.0e-4).unwrap();
            for arm in SMALL_STATIC_ARMS {
                let run = run_arm(arm, problem, 1.0e-4).unwrap();
                assert_eq!(run.observed.y, legacy.observed.y, "{arm} {id}");
                assert_eq!(run.observed.counters, legacy.observed.counters);
                assert_eq!(run.diagnostics.attempts, legacy.diagnostics.attempts);
            }
        }
        let brusselator = problems
            .iter()
            .find(|p| p.id == "brusselator-1d-50")
            .unwrap();
        assert!(run_arm(SMALL_STATIC_ARMS[0], brusselator, 1.0e-4).is_err());
        let ensemble = |arm: &str| ensemble_run(arm, 4, 1.0e-4, 1, None).unwrap();
        let (a, b) = (ensemble(SMALL_ARM), ensemble(SMALL_STATIC_ARMS[0]));
        assert_eq!(a["attempts"], b["attempts"]);
        assert_eq!(
            a["checksum"].as_f64().unwrap().to_bits(),
            b["checksum"].as_f64().unwrap().to_bits()
        );
        assert!(ensemble_run(SMALL_STATIC_ARMS[1], 4, 1.0e-4, 1, None).is_err());
    }
}

/// Speed research node SPD09 (`research/spd09_colext_threshold_20261007`):
/// the column-extent LU above `RODAS5P_FAST_SMALL_LU_MAX` only, against the
/// legacy dense driver.
#[cfg(test)]
mod spd09 {
    use rodas5p_integrators::{
        RODAS5P_FAST_COLEXT64_DRIVER_ID, RODAS5P_FAST_SMALL_LU_MAX, Rodas5pFastLu,
        Rodas5pFastResult,
    };

    use super::*;

    fn hx(v: f64) -> String {
        format!("{:016x}", v.to_bits())
    }

    /// Everything a run reports, the floats as IEEE bits (no driver id).
    fn row(r: &Rodas5pFastResult) -> Value {
        let o = &r.observed;
        json!({
            "success": o.success, "message": o.message,
            "t": o.t.iter().map(|v| hx(*v)).collect::<Vec<_>>(),
            "y": o.y.iter().map(|s| s.iter().map(|v| hx(*v)).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "attempts": r.attempts, "accepted_steps": r.accepted_steps,
            "rejected_steps": r.rejected_steps, "jacobian_reuses": r.jacobian_reuses,
            "internal_steps": o.internal_steps, "output_clipped_steps": o.output_clipped_steps,
            "counters": serde_json::to_value(o.counters).unwrap(),
        })
    }

    fn colext64() -> Rodas5pFastOptions {
        option_arm("rodas5p-fast-colext64").unwrap().1
    }

    /// `(legacy, colext64)` on one problem at one tolerance.
    fn pair(problem: &BenchmarkProblem, rtol: f64) -> (Rodas5pFastResult, Rodas5pFastResult) {
        let adaptive = adaptive_config(problem, rtol);
        let output = OutputSchedule::new(vec![problem.t_span.0, problem.t_span.1]).unwrap();
        let legacy = integrate_rodas5p_fast_observed(
            &problem.problem,
            problem.t_span,
            &problem.y0,
            &adaptive,
            &output,
        )
        .unwrap();
        let new = integrate_rodas5p_fast_observed_with_options(
            &problem.problem,
            problem.t_span,
            &problem.y0,
            &adaptive,
            &output,
            colext64(),
        )
        .unwrap();
        (legacy, new)
    }

    /// The registered points: the five benchmark problems at the seven
    /// tolerances, then `brusselator-1d-30` and `-40` at rtol 1e-6.
    fn points() -> Vec<(BenchmarkProblem, Vec<f64>)> {
        let mut out: Vec<_> = benchmark_problems()
            .unwrap()
            .into_iter()
            .map(|p| (p, TOLERANCES.to_vec()))
            .collect();
        for problem in profile_problems().unwrap() {
            if ["brusselator-1d-30", "brusselator-1d-40"].contains(&problem.id) {
                out.push((problem, vec![1.0e-6]));
            }
        }
        out
    }

    /// A small identity check (the export runs all 37 points): both sides of
    /// the threshold, with the LU each side must choose.
    #[test]
    fn colext64_equals_the_legacy_driver_on_both_sides_of_the_threshold() {
        assert!(known_arm("rodas5p-fast-colext64") && !ARMS.contains(&"rodas5p-fast-colext64"));
        assert!(option_arm("rodas5p-fast-small-colext64").is_none());
        let problems = profile_problems().unwrap();
        for (id, lu) in [
            ("van-der-pol-mu1000", Rodas5pFastLu::InPlaceZeroSkipping),
            ("hires", Rodas5pFastLu::InPlaceZeroSkipping),
            ("brusselator-1d-30", Rodas5pFastLu::InPlaceZeroSkipping),
            ("brusselator-1d-40", Rodas5pFastLu::InPlaceColumnExtents),
        ] {
            let problem = problems.iter().find(|p| p.id == id).unwrap();
            let n = problem.problem.dimension;
            assert_eq!(
                lu == Rodas5pFastLu::InPlaceColumnExtents,
                n > RODAS5P_FAST_SMALL_LU_MAX
            );
            let (legacy, new) = pair(problem, 1.0e-3);
            assert!(legacy.observed.success);
            assert_eq!(row(&new), row(&legacy), "{id}");
            assert_eq!(new.lu, lu, "{id}");
            assert_eq!(legacy.lu, Rodas5pFastLu::InPlaceZeroSkipping);
            assert_eq!(new.driver, RODAS5P_FAST_COLEXT64_DRIVER_ID);
        }
        // The benchmark keeps its five problems; the profile adds three.
        assert_eq!(benchmark_problems().unwrap().len(), 5);
        let dims: Vec<_> = problems
            .iter()
            .map(|p| (p.id, p.problem.dimension))
            .collect();
        assert!(
            dims.contains(&("brusselator-1d-30", 60)) && dims.contains(&("brusselator-1d-40", 80))
        );
    }

    /// Identity export of SPD09: 37 points, colext64 against the legacy
    /// dense driver, as IEEE bits.
    #[test]
    #[ignore = "identity export of research/spd09_colext_threshold_20261007; release build; set SPD09_IDENTITY"]
    fn spd09_identity_export() {
        let Ok(path) = std::env::var("SPD09_IDENTITY") else {
            println!("SPD09_IDENTITY not set: nothing written");
            return;
        };
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path);
        assert!(
            !path.exists(),
            "immutable output exists: {}",
            path.display()
        );
        let mut rows = Vec::new();
        for (problem, rtol) in points()
            .iter()
            .flat_map(|(p, tols)| tols.iter().map(move |&rtol| (p, rtol)))
        {
            let (legacy, new) = pair(problem, rtol);
            let (a, b) = (row(&legacy), row(&new));
            let identical = a == b;
            println!(
                "{} (n = {}) {:e}: identical {identical}, lu {:?} / {:?}",
                problem.id, problem.problem.dimension, rtol, legacy.lu, new.lu
            );
            rows.push(json!({
                "problem": problem.id, "dimension": problem.problem.dimension, "rtol": rtol,
                "identical": identical,
                "legacy_driver": legacy.driver, "colext64_driver": new.driver,
                "legacy_lu": format!("{:?}", legacy.lu), "colext64_lu": format!("{:?}", new.lu),
                "legacy": a, "colext64": if identical { Value::Null } else { b },
            }));
        }
        let out = json!({
            "schema": "vigilode-spd09-identity-v1",
            "threshold": RODAS5P_FAST_SMALL_LU_MAX,
            "rows": rows,
        });
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, serde_json::to_string(&out).unwrap() + "\n").unwrap();
        println!("wrote {}", path.display());
    }
}
