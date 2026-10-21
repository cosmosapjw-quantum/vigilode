//! The solver arms. Every arm integrates the same [`Model`] with the same
//! scalar `rtol` and `atol` and returns its states on the common output grid
//! through its own output interface.

use std::sync::Arc;

use diffsol::{
    DenseMatrix as _, FaerLU, FaerMat, FaerSparseLU, FaerSparseMat, MatrixCommon, OdeBuilder,
    OdeSolverMethod, VectorHost,
};
use rodas5p_core::{CoreResult, DenseMatrix};
use rodas5p_integrators::{
    AdaptiveStepConfig, BandedJacobian, OdeProblem, OutputSamplingPlan, OutputSchedule,
    RadauConfig, RoutedRun, integrate_radau_adaptive_dense_observed,
    integrate_rodas5p_fast_banded_observed, integrate_rodas5p_routed_observed,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use vigilode::{
    Atol, CallbackError, DriverWork, InteriorError, Jacobian, Options, Problem, Rodas5p,
};

use crate::alloc::Mark;
use crate::ffi::{self, Hairer, NativeOutcome};
use crate::model::Model;

/// Facade defaults (`vigilode::Options::default()`), used by the VigilODE arms.
pub const VIGILODE_H0: f64 = 1.0e-3;
/// Facade default attempt budget.
pub const VIGILODE_MAX_STEPS: usize = 100_000;
/// Total-step cap of the CVODE one-step loop (the Hairer codes' NMAX default).
pub const CVODE_MAX_STEPS: i64 = 100_000;

/// A solver arm.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Arm {
    /// VigilODE Protected RODAS5P, dense analytic Jacobian (the facade's
    /// `Jacobian::Dense` route).
    Vigilode,
    /// VigilODE Protected RODAS5P, banded analytic Jacobian (the facade's
    /// `Jacobian::Banded` route).
    VigilodeBanded,
    /// VigilODE through `Rodas5p::solve` with `dense_output = true`, interior
    /// error control `WFilteredEnforce` (the product default), dense analytic
    /// Jacobian; grid values from `Solution::at`.
    VigilodeDense,
    /// As [`Arm::VigilodeDense`] with `InteriorError::Off` (control arm of the
    /// interior-error evidence).
    VigilodeDenseOff,
    /// As [`Arm::VigilodeDense`] with the banded analytic Jacobian.
    VigilodeBandedDense,
    /// As [`Arm::VigilodeBandedDense`] with `InteriorError::Off`.
    VigilodeBandedDenseOff,
    /// Diffsol 0.16.1 BDF, `FaerMat` + `FaerLU`.
    DiffsolBdfDense,
    /// Diffsol 0.16.1 BDF, `FaerSparseMat` + `FaerSparseLU` (coloured Jacobian).
    DiffsolBdfSparse,
    /// SUNDIALS CVODE 6.4.1 BDF, dense direct linear solver.
    CvodeBdfDense,
    /// SUNDIALS CVODE 6.4.1 BDF, band direct linear solver.
    CvodeBdfBand,
    /// Netlib DVODE, MF = 21 (BDF, dense analytic Jacobian).
    DvodeMf21,
    /// Netlib DVODE, MF = 24 (BDF, banded analytic Jacobian).
    DvodeMf24,
    /// Hairer RADAU5, full analytic Jacobian.
    Radau5,
    /// Hairer RADAU5, banded analytic Jacobian.
    Radau5Band,
    /// Hairer RODAS, full analytic Jacobian.
    Rodas,
    /// Hairer RODAS, banded analytic Jacobian.
    RodasBand,
    /// The internal Radau IIA (3-stage) of `rodas5p-integrators`.
    InternalRadauIia3,
}

/// Comparator fidelity tag (plan §7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Fidelity {
    /// The production solver under test.
    SolverUnderTest,
    /// An external production code.
    Production,
    /// An internal reference implementation; never in a speed table.
    ReferenceImplementationOnly,
}

impl Arm {
    /// Every arm, in report order.
    pub const ALL: [Arm; 17] = [
        Arm::Vigilode,
        Arm::VigilodeBanded,
        Arm::VigilodeDense,
        Arm::VigilodeDenseOff,
        Arm::VigilodeBandedDense,
        Arm::VigilodeBandedDenseOff,
        Arm::DiffsolBdfDense,
        Arm::DiffsolBdfSparse,
        Arm::CvodeBdfDense,
        Arm::CvodeBdfBand,
        Arm::DvodeMf21,
        Arm::DvodeMf24,
        Arm::Radau5,
        Arm::Radau5Band,
        Arm::Rodas,
        Arm::RodasBand,
        Arm::InternalRadauIia3,
    ];

    /// The free-stepping VigilODE arms that go through the facade with
    /// `dense_output = true`.
    pub fn is_facade_dense(self) -> bool {
        matches!(
            self,
            Arm::VigilodeDense
                | Arm::VigilodeDenseOff
                | Arm::VigilodeBandedDense
                | Arm::VigilodeBandedDenseOff
        )
    }

    /// The v1 VigilODE continuity arms, whose steps are clipped to the grid.
    pub fn is_grid_clipped(self) -> bool {
        matches!(self, Arm::Vigilode | Arm::VigilodeBanded)
    }

    /// The kebab-case id used in the JSON.
    pub fn id(self) -> String {
        serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_default()
    }

    /// Fidelity tag.
    pub fn fidelity(self) -> Fidelity {
        match self {
            a if a.is_grid_clipped() || a.is_facade_dense() => Fidelity::SolverUnderTest,
            Arm::InternalRadauIia3 => Fidelity::ReferenceImplementationOnly,
            _ => Fidelity::Production,
        }
    }

    /// Whether the arm uses the band structure (run only on banded problems).
    pub fn banded(self) -> bool {
        matches!(
            self,
            Arm::VigilodeBanded
                | Arm::VigilodeBandedDense
                | Arm::VigilodeBandedDenseOff
                | Arm::CvodeBdfBand
                | Arm::DvodeMf24
                | Arm::Radau5Band
                | Arm::RodasBand
                | Arm::DiffsolBdfSparse
        )
    }

    /// The solver's own output interface used for the common grid.
    pub fn output_interface(self) -> &'static str {
        match self {
            Arm::Vigilode | Arm::VigilodeBanded => {
                "grid-clipped (continuity arm): OutputSchedule of the protected fast driver, each grid time is a step landing (steps clipped to it); the endpoint-only facade route is not used"
            }
            Arm::VigilodeDense
            | Arm::VigilodeDenseOff
            | Arm::VigilodeBandedDense
            | Arm::VigilodeBandedDenseOff => {
                "Rodas5p::solve with dense_output = true (free stepping), grid values from Solution::at(t_k); the driver that ran is recorded per row as native.driver_id"
            }
            Arm::DiffsolBdfDense | Arm::DiffsolBdfSparse => {
                "OdeSolverMethod::solve_dense(t_eval) (BDF interpolation, stop time = last grid point)"
            }
            Arm::CvodeBdfDense | Arm::CvodeBdfBand => {
                "CV_ONE_STEP loop + CVodeGetDky(t, 0) for every grid time inside the last step"
            }
            Arm::DvodeMf21 | Arm::DvodeMf24 => "DVODE ITASK = 1 (interpolated output at TOUT)",
            Arm::Radau5 | Arm::Radau5Band => "SOLOUT (IOUT = 1) + CONTR5",
            Arm::Rodas | Arm::RodasBand => "SOLOUT (IOUT = 1) + CONTRO",
            Arm::InternalRadauIia3 => {
                "OutputSamplingPlan::dense (Radau collocation interpolant, no step clipping)"
            }
        }
    }

    /// Every option that is not the vendor default plus rtol/atol, and every
    /// adapter choice that can affect the result or its cost.
    pub fn deviations(self) -> Vec<&'static str> {
        match self {
            Arm::Vigilode | Arm::VigilodeBanded => vec![
                "driver called directly with the facade's endpoint-only route and defaults (h0 = 1e-3, max_steps = 100000) and an OutputSchedule over the grid; route equivalence is tested bit-for-bit at t1",
                "grid landings add clipped steps that a free-stepping run would not take (grid-clipped continuity arm)",
            ],
            Arm::VigilodeDense | Arm::VigilodeBandedDense => vec![
                "facade defaults except dense_output = true and Atol::Scalar(rtol * atol_scale); interior_error = WFilteredEnforce (the product default)",
                "a row with output_clipped_steps != 0, or with Solution::at(t_k) = None inside the span of a successful run, is a protocol-violation",
            ],
            Arm::VigilodeDenseOff | Arm::VigilodeBandedDenseOff => vec![
                "facade defaults except dense_output = true, Atol::Scalar(rtol * atol_scale) and interior_error = Off (control arm: only the endpoint of each step is error controlled)",
                "a row with output_clipped_steps != 0, or with Solution::at(t_k) = None inside the span of a successful run, is a protocol-violation",
            ],
            Arm::DiffsolBdfDense => vec![
                "Jacobian assembled by Diffsol from n JVPs of the shared analytic Jacobian (builder rhs_implicit; no coloring, the dense default)",
            ],
            Arm::DiffsolBdfSparse => vec![
                "Jacobian assembled by Diffsol from coloured JVPs of the shared analytic Jacobian (sparsity detected by NaN propagation, the sparse default)",
            ],
            Arm::CvodeBdfDense | Arm::CvodeBdfBand => vec![
                "one-step mode with a total cap of 100000 internal steps (CVODE's mxstep = 500 applies per CVode call and would not bind in one-step mode)",
            ],
            Arm::DvodeMf21 | Arm::DvodeMf24 => vec![],
            Arm::Radau5 | Arm::Radau5Band | Arm::Rodas | Arm::RodasBand => {
                vec!["initial H = 0 (the code then uses 1e-6)"]
            }
            Arm::InternalRadauIia3 => vec![
                "RadauConfig::default() with the facade's adaptive defaults; reference implementation only",
            ],
        }
    }
}

/// Outcome status of one run. Failed runs are rows, never dropped.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum RunStatus {
    /// Every grid point was produced and the solver reported success.
    Success,
    /// The solver reported failure (code and message kept).
    SolverFailure {
        /// Solver return code, or 0 where the solver has none.
        code: i64,
        /// Solver or adapter message.
        message: String,
    },
    /// The run violated the endpoint-only stepping protocol of the dense
    /// arms (clipped steps, or a grid time the interpolant does not answer).
    ProtocolViolation {
        /// What was violated.
        message: String,
    },
    /// The adapter could not set up the solver.
    SetupFailure {
        /// Error text.
        message: String,
    },
}

/// Normalized work counters as reported by each solver (`None` where the
/// solver does not report the quantity).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Work {
    /// Accepted steps.
    pub accepted_steps: Option<u64>,
    /// Steps rejected by the error test.
    pub rejected_steps: Option<u64>,
    /// All step attempts, where reported.
    pub step_attempts: Option<u64>,
    /// RHS evaluations reported by the solver.
    pub rhs_evaluations: Option<u64>,
    /// Jacobian evaluations reported by the solver.
    pub jacobian_evaluations: Option<u64>,
    /// LU factorizations (or linear-solver setups) reported by the solver.
    pub factorizations: Option<u64>,
    /// Linear solves (forward/back substitutions) reported by the solver.
    pub linear_solves: Option<u64>,
    /// Nonlinear (Newton) iterations.
    pub nonlinear_iterations: Option<u64>,
    /// Nonlinear convergence failures.
    pub nonlinear_failures: Option<u64>,
}

/// One run of one arm.
#[derive(Clone, Debug)]
pub struct RunOutput {
    /// Status.
    pub status: RunStatus,
    /// Grid states actually produced (a prefix of the grid).
    pub states: Vec<Vec<f64>>,
    /// Normalized work.
    pub work: Work,
    /// The solver's raw statistics.
    pub native: Value,
    /// End of the measured window (`None`: the end of [`run`]).
    pub end: Option<Mark>,
}

fn u(x: impl TryInto<u64>) -> Option<u64> {
    x.try_into().ok()
}

fn columns(o: &NativeOutcome, n: usize) -> Vec<Vec<f64>> {
    o.values
        .chunks(n)
        .take(o.completed)
        .map(<[f64]>::to_vec)
        .collect()
}

/// Run `arm` on `model` over `grid` (first and last entries are the span).
pub fn run(arm: Arm, model: &Model, grid: &[f64], rtol: f64, atol: f64) -> RunOutput {
    run_with(arm, model, grid, rtol, atol, None)
}

/// [`run`] with an optional attempt budget override (used for references only).
pub fn run_with(
    arm: Arm,
    model: &Model,
    grid: &[f64],
    rtol: f64,
    atol: f64,
    max_steps: Option<usize>,
) -> RunOutput {
    let y0 = model.y0();
    let band = model.band();
    match arm {
        Arm::Vigilode => vigilode(model, grid, &y0, rtol, atol, None, max_steps),
        Arm::VigilodeBanded => vigilode(model, grid, &y0, rtol, atol, Some(band), max_steps),
        Arm::VigilodeDense | Arm::VigilodeDenseOff => {
            let interior = interior_of(arm);
            vigilode_dense(model, grid, &y0, rtol, atol, None, interior, max_steps)
        }
        Arm::VigilodeBandedDense | Arm::VigilodeBandedDenseOff => {
            let interior = interior_of(arm);
            vigilode_dense(
                model,
                grid,
                &y0,
                rtol,
                atol,
                Some(band),
                interior,
                max_steps,
            )
        }
        Arm::DiffsolBdfDense => diffsol_dense(model, grid, &y0, rtol, atol),
        Arm::DiffsolBdfSparse => diffsol_sparse(model, grid, &y0, rtol, atol),
        Arm::CvodeBdfDense | Arm::CvodeBdfBand => {
            let b = (arm == Arm::CvodeBdfBand).then_some(band);
            let cap = max_steps.map_or(CVODE_MAX_STEPS, |m| m as i64);
            let o = ffi::cvode(model, grid, &y0, rtol, atol, b, cap);
            let states = columns(&o, model.n);
            let end = Some(Mark::now());
            let s = &o.stats;
            let status = if o.code == 0 && o.completed == grid.len() {
                RunStatus::Success
            } else {
                RunStatus::SolverFailure {
                    code: o.code,
                    message: format!("CVODE flag {}", o.code),
                }
            };
            RunOutput {
                status,
                states,
                work: Work {
                    accepted_steps: u(s[0]),
                    rejected_steps: u(s[3]),
                    step_attempts: None,
                    rhs_evaluations: u(s[1] + s[7]),
                    jacobian_evaluations: u(s[6]),
                    factorizations: u(s[2]),
                    linear_solves: None,
                    nonlinear_iterations: u(s[4]),
                    nonlinear_failures: u(s[5]),
                },
                native: json!({"nst": s[0], "nfe": s[1], "nsetups": s[2], "netf": s[3], "nni": s[4], "ncfn": s[5], "nje": s[6], "nfeLS": s[7], "flag": o.code}),
                end,
            }
        }
        Arm::DvodeMf21 | Arm::DvodeMf24 => {
            let b = (arm == Arm::DvodeMf24).then_some(band);
            let o = ffi::dvode(model, grid, &y0, rtol, atol, b);
            let states = columns(&o, model.n);
            let end = Some(Mark::now());
            let s = &o.stats;
            let status = if o.code == 2 && o.completed == grid.len() {
                RunStatus::Success
            } else {
                RunStatus::SolverFailure {
                    code: o.code,
                    message: format!("DVODE ISTATE {}", o.code),
                }
            };
            RunOutput {
                status,
                states,
                work: Work {
                    accepted_steps: u(s[0]),
                    rejected_steps: u(s[6]),
                    step_attempts: None,
                    rhs_evaluations: u(s[1]),
                    jacobian_evaluations: u(s[2]),
                    factorizations: u(s[3]),
                    linear_solves: None,
                    nonlinear_iterations: u(s[4]),
                    nonlinear_failures: u(s[5]),
                },
                native: json!({"nst": s[0], "nfe": s[1], "nje": s[2], "nlu": s[3], "nni": s[4], "ncfn": s[5], "netf": s[6], "nqu": s[7], "istate": o.code}),
                end,
            }
        }
        Arm::Radau5 | Arm::Radau5Band | Arm::Rodas | Arm::RodasBand => {
            let which = if matches!(arm, Arm::Radau5 | Arm::Radau5Band) {
                Hairer::Radau5
            } else {
                Hairer::Rodas
            };
            let b = matches!(arm, Arm::Radau5Band | Arm::RodasBand).then_some(band);
            let o = ffi::hairer(which, model, grid, &y0, rtol, atol, b);
            let states = columns(&o, model.n);
            let end = Some(Mark::now());
            let s = &o.stats;
            let status = if o.code > 0 && o.completed == grid.len() {
                RunStatus::Success
            } else {
                RunStatus::SolverFailure {
                    code: o.code,
                    message: format!("IDID {}", o.code),
                }
            };
            RunOutput {
                status,
                states,
                work: Work {
                    accepted_steps: u(s[3]),
                    rejected_steps: u(s[4]),
                    step_attempts: u(s[2]),
                    rhs_evaluations: u(s[0]),
                    jacobian_evaluations: u(s[1]),
                    factorizations: u(s[5]),
                    linear_solves: u(s[6]),
                    nonlinear_iterations: None,
                    nonlinear_failures: None,
                },
                native: json!({"nfcn": s[0], "njac": s[1], "nstep": s[2], "naccpt": s[3], "nrejct": s[4], "ndec": s[5], "nsol": s[6], "idid": o.code}),
                end,
            }
        }
        Arm::InternalRadauIia3 => internal_radau(model, grid, &y0, rtol, atol),
    }
}

fn setup_failure(message: String) -> RunOutput {
    RunOutput {
        status: RunStatus::SetupFailure { message },
        states: Vec::new(),
        work: Work::default(),
        native: Value::Null,
        end: None,
    }
}

type CoreRhs = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;
type CoreJacobian = Arc<dyn Fn(f64, &[f64]) -> CoreResult<DenseMatrix> + Send + Sync>;

/// The facade's driver-level problem for `Jacobian::Dense` / `Jacobian::Banded`
/// (`crates/vigilode/src/solve.rs::build`, without the callback guard).
fn facade_ode(
    model: &Model,
    band: Option<(usize, usize)>,
) -> CoreResult<(OdeProblem, Option<BandedJacobian>)> {
    let m = *model;
    let n = m.n;
    let rhs: CoreRhs = Arc::new(move |t, y, out| {
        m.rhs(t, y, out);
        Ok(())
    });
    let (jacobian, banded): (CoreJacobian, Option<BandedJacobian>) = match band {
        None => (
            Arc::new(move |t, y| {
                let mut j = DenseMatrix::zeros(n, n);
                m.jac_dense_row_major(t, y, j.as_mut_slice());
                Ok(j)
            }),
            None,
        ),
        Some((lower, upper)) => {
            let fill: CoreRhs = Arc::new(move |t, y, out| {
                m.jac_band_row_major(t, y, lower, upper, out);
                Ok(())
            });
            let f2 = fill.clone();
            let w = lower + upper + 1;
            (
                Arc::new(move |t, y| {
                    let mut values = vec![0.0; n * w];
                    f2(t, y, &mut values)?;
                    let mut d = DenseMatrix::zeros(n, n);
                    for i in 0..n {
                        for j in i.saturating_sub(lower)..=(i + upper).min(n - 1) {
                            d[(i, j)] = values[i * w + (j + lower - i)];
                        }
                    }
                    Ok(d)
                }),
                Some(BandedJacobian { lower, upper, fill }),
            )
        }
    };
    let ode = OdeProblem::new(
        "vigilode-facade",
        n,
        rhs,
        None,
        Some(jacobian),
        None,
        None,
        true,
        None,
        None,
    )?;
    Ok((ode, banded))
}

/// The facade's `AdaptiveStepConfig` for `Options { rtol, atol, ..Default }`.
pub fn facade_adaptive(rtol: f64, atol: f64, max_steps: Option<usize>) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        rtol,
        atol,
        initial_step: VIGILODE_H0,
        max_attempts: max_steps.unwrap_or(VIGILODE_MAX_STEPS),
        ..AdaptiveStepConfig::default()
    }
}

enum Extra {
    Routing(Box<rodas5p_integrators::RoutingRecord>),
    Banded(rodas5p_integrators::BandedWork),
}

fn vigilode(
    model: &Model,
    grid: &[f64],
    y0: &[f64],
    rtol: f64,
    atol: f64,
    band: Option<(usize, usize)>,
    max_steps: Option<usize>,
) -> RunOutput {
    let adaptive = facade_adaptive(rtol, atol, max_steps);
    let span = (grid[0], grid[grid.len() - 1]);
    let schedule = match OutputSchedule::new(grid.to_vec()) {
        Ok(s) => s,
        Err(e) => return setup_failure(e.to_string()),
    };
    let (ode, banded) = match facade_ode(model, band) {
        Ok(x) => x,
        Err(e) => return setup_failure(e.to_string()),
    };
    let (mut fast, extra) = match banded {
        None => match integrate_rodas5p_routed_observed(&ode, span, y0, &adaptive, &schedule) {
            Ok(r) => match r.run {
                RoutedRun::Dense(f) => (f, Extra::Routing(Box::new(r.routing))),
                other => {
                    return setup_failure(format!(
                        "unexpected route {:?}",
                        std::mem::discriminant(&other)
                    ));
                }
            },
            Err(e) => {
                return RunOutput {
                    status: RunStatus::SolverFailure {
                        code: 0,
                        message: e.to_string(),
                    },
                    states: Vec::new(),
                    work: Work::default(),
                    native: Value::Null,
                    end: None,
                };
            }
        },
        Some(b) => {
            match integrate_rodas5p_fast_banded_observed(&ode, &b, span, y0, &adaptive, &schedule) {
                Ok(r) => (r.fast, Extra::Banded(r.work)),
                Err(e) => {
                    return RunOutput {
                        status: RunStatus::SolverFailure {
                            code: 0,
                            message: e.to_string(),
                        },
                        states: Vec::new(),
                        work: Work::default(),
                        native: Value::Null,
                        end: None,
                    };
                }
            }
        }
    };
    let c = fast.observed.counters;
    let ok = fast.observed.success && fast.observed.t.as_slice() == grid;
    let status = if ok {
        RunStatus::Success
    } else {
        RunStatus::SolverFailure {
            code: 0,
            message: if fast.attempts >= adaptive.max_attempts {
                format!("max_steps {} reached", adaptive.max_attempts)
            } else {
                fast.observed.message.clone()
            },
        }
    };
    let observed_t = std::mem::take(&mut fast.observed.t);
    let observed_y = std::mem::take(&mut fast.observed.y);
    let states = if ok {
        observed_y
    } else {
        // A failed run returns its prefix of the grid (t0 and every landed time).
        observed_t
            .iter()
            .zip(observed_y)
            .zip(grid)
            .take_while(|((t, _), g)| *t == *g)
            .map(|((_, y), _)| y)
            .collect()
    };
    drop(ode);
    let end = Some(Mark::now());
    RunOutput {
        status,
        states,
        work: Work {
            accepted_steps: u(fast.accepted_steps),
            rejected_steps: u(fast.rejected_steps),
            step_attempts: u(fast.attempts),
            rhs_evaluations: Some(c.rhs_evaluations),
            jacobian_evaluations: Some(c.jacobian_builds),
            factorizations: Some(c.direct_factorizations),
            linear_solves: Some(c.direct_solve_calls),
            nonlinear_iterations: None,
            nonlinear_failures: None,
        },
        native: json!({
            "driver": fast.driver,
            "counters": c,
            "attempts": fast.attempts,
            "jacobian_reuses": fast.jacobian_reuses,
            "internal_steps": fast.observed.internal_steps,
            "output_clipped_steps": fast.observed.output_clipped_steps,
            "lu": format!("{:?}", fast.lu),
            "route": match extra {
                Extra::Routing(r) => json!({"routing": r}),
                Extra::Banded(w) => json!({"banded_work": format!("{w:?}")}),
            },
        }),
        end,
    }
}

fn interior_of(arm: Arm) -> InteriorError {
    if matches!(arm, Arm::VigilodeDenseOff | Arm::VigilodeBandedDenseOff) {
        InteriorError::Off
    } else {
        InteriorError::WFilteredEnforce
    }
}

/// The facade `Problem` of a model, dense or banded analytic Jacobian.
pub fn facade_problem(model: &Model, band: Option<(usize, usize)>) -> Problem {
    let m = *model;
    let p = Problem::new(m.n, move |t, y, dy| {
        m.rhs(t, y, dy);
        Ok::<(), CallbackError>(())
    });
    let p = match band {
        Some((l, u)) => p.jacobian(Jacobian::banded(l, u, move |t, y, out| {
            m.jac_band_row_major(t, y, l, u, out);
            Ok(())
        })),
        None => p.jacobian(Jacobian::dense(move |t, y, jac| {
            m.jac_dense_row_major(t, y, jac);
            Ok(())
        })),
    };
    p.autonomous()
}

/// The facade `Options` of a dense-output arm.
pub fn dense_options(
    rtol: f64,
    atol: f64,
    interior: InteriorError,
    max_steps: Option<usize>,
) -> Options {
    Options {
        rtol,
        atol: Atol::Scalar(atol),
        max_steps: max_steps.unwrap_or(VIGILODE_MAX_STEPS),
        dense_output: true,
        interior_error: interior,
        ..Default::default()
    }
}

#[allow(clippy::too_many_arguments)]
fn vigilode_dense(
    model: &Model,
    grid: &[f64],
    y0: &[f64],
    rtol: f64,
    atol: f64,
    band: Option<(usize, usize)>,
    interior: InteriorError,
    max_steps: Option<usize>,
) -> RunOutput {
    let problem = facade_problem(model, band);
    let opts = dense_options(rtol, atol, interior, max_steps);
    let span = (grid[0], grid[grid.len() - 1]);
    let sol = match Rodas5p::solve(&problem, span, y0, &opts) {
        Ok(s) => s,
        Err(e) => {
            return RunOutput {
                status: RunStatus::SolverFailure {
                    code: 0,
                    message: e.to_string(),
                },
                states: Vec::new(),
                work: Work::default(),
                native: Value::Null,
                end: None,
            };
        }
    };
    // Grid output in final form: the interpolant at every grid time.
    let mut states: Vec<Vec<f64>> = Vec::with_capacity(grid.len());
    for &t in grid {
        match sol.at(t) {
            Some(y) => states.push(y),
            None => break,
        }
    }
    let st = &sol.stats;
    let c = st.counters;
    let success = sol.is_success();
    let clipped = st.output_clipped_steps;
    let status = if clipped != 0 {
        RunStatus::ProtocolViolation {
            message: format!("output_clipped_steps = {clipped}, expected 0"),
        }
    } else if success && states.len() != grid.len() {
        RunStatus::ProtocolViolation {
            message: format!(
                "Solution::at answered {} of {} grid times of a successful run",
                states.len(),
                grid.len()
            ),
        }
    } else if success {
        RunStatus::Success
    } else {
        RunStatus::SolverFailure {
            code: 0,
            message: if st.attempts >= opts.max_steps {
                format!("max_steps {} reached", opts.max_steps)
            } else {
                st.driver_message.clone()
            },
        }
    };
    let work = Work {
        accepted_steps: u(st.accepted_steps),
        rejected_steps: u(st.rejected_steps),
        step_attempts: u(st.attempts),
        rhs_evaluations: Some(c.rhs_evaluations),
        jacobian_evaluations: Some(c.jacobian_builds),
        factorizations: Some(c.direct_factorizations),
        linear_solves: Some(c.direct_solve_calls),
        nonlinear_iterations: None,
        nonlinear_failures: None,
    };
    // What ran, read from the route's own record (never echoed from the
    // request). Other routes (a fast dense driver) slot in as further
    // `if let` branches; no exhaustive match on the facade enum.
    let mut ran = json!({"control": null, "samples": null, "interior_rejections": null, "max_accepted_estimate": null});
    if let DriverWork::SequentialDense {
        control,
        samples,
        interior_rejections,
        max_accepted_estimate,
        ..
    } = &st.driver_work
    {
        ran = json!({
            "control": format!("{control:?}"),
            "samples": samples,
            "interior_rejections": interior_rejections,
            "max_accepted_estimate": max_accepted_estimate,
        });
    }
    let native = json!({
        "ran": ran,
        "driver_id": st.route.driver_id,
        "entry_point": st.route.entry_point,
        "interior_error": if interior == InteriorError::Off { "off" } else { "w-filtered-enforce" },
        "attempts": st.attempts,
        "internal_steps": st.internal_steps,
        "output_clipped_steps": clipped,
        "output_times": sol.t.len(),
        "counters": c,
    });
    drop(sol);
    drop(problem);
    RunOutput {
        status,
        states,
        work,
        native,
        end: Some(Mark::now()),
    }
}

fn diffsol_stats(s: &diffsol::ode_solver::OdeSolverStatistics) -> (Work, Value) {
    (
        Work {
            accepted_steps: u(s.number_of_steps),
            rejected_steps: u(s.number_of_error_test_failures),
            step_attempts: None,
            rhs_evaluations: None,
            jacobian_evaluations: None,
            factorizations: u(s.number_of_linear_solver_setups),
            linear_solves: None,
            nonlinear_iterations: u(s.number_of_nonlinear_solver_iterations),
            nonlinear_failures: u(s.number_of_nonlinear_solver_fails),
        },
        serde_json::to_value(s).unwrap_or(Value::Null),
    )
}

macro_rules! diffsol_arm {
    ($name:ident, $mat:ty, $lu:ty, $coloring:expr) => {
        fn $name(model: &Model, grid: &[f64], y0: &[f64], rtol: f64, atol: f64) -> RunOutput {
            let (m1, m2) = (*model, *model);
            let n = model.n;
            let y0 = y0.to_vec();
            let t0 = grid[0];
            let built = OdeBuilder::<$mat>::new()
                .t0(t0)
                .rtol(rtol)
                .atol(vec![atol; n])
                .use_coloring($coloring)
                .rhs_implicit(
                    move |x, _p, t, out| m1.rhs(t, x.as_slice(), out.as_mut_slice()),
                    move |x, _p, t, v, out| m2.jvp(t, x.as_slice(), v.as_slice(), out.as_mut_slice()),
                )
                .init(
                    move |_p, _t, out| out.as_mut_slice().copy_from_slice(&y0),
                    n,
                )
                .build();
            let problem = match built {
                Ok(p) => p,
                Err(e) => return setup_failure(e.to_string()),
            };
            let mut solver = match problem.bdf::<$lu>() {
                Ok(s) => s,
                Err(e) => return setup_failure(e.to_string()),
            };
            let result = solver.solve_dense(grid);
            let stats = solver.get_statistics().clone();
            let result = result.map(|(mat, stop)| {
                let states: Vec<Vec<f64>> = (0..mat.ncols())
                    .map(|k| (0..n).map(|i| mat.get_index(i, k)).collect())
                    .collect();
                (states, stop)
            });
            drop(solver);
            drop(problem);
            let end = Some(Mark::now());
            let (work, native) = diffsol_stats(&stats);
            match result {
                Ok((states, stop)) => {
                    let ok = states.len() == grid.len();
                    RunOutput {
                        status: if ok {
                            RunStatus::Success
                        } else {
                            RunStatus::SolverFailure {
                                code: 0,
                                message: format!("{} of {} grid points, stop {stop:?}", states.len(), grid.len()),
                            }
                        },
                        states,
                        work,
                        native: json!({"statistics": native, "stop": format!("{stop:?}")}),
                        end,
                    }
                }
                Err(e) => RunOutput {
                    status: RunStatus::SolverFailure {
                        code: 0,
                        message: e.to_string(),
                    },
                    // solve_dense returns no partial matrix on error.
                    states: Vec::new(),
                    work,
                    native: json!({"statistics": native, "error": e.to_string()}),
                    end,
                },
            }
        }
    };
}

diffsol_arm!(diffsol_dense, FaerMat<f64>, FaerLU<f64>, false);
diffsol_arm!(diffsol_sparse, FaerSparseMat<f64>, FaerSparseLU<f64>, true);

fn internal_radau(model: &Model, grid: &[f64], y0: &[f64], rtol: f64, atol: f64) -> RunOutput {
    let adaptive = facade_adaptive(rtol, atol, None);
    let span = (grid[0], grid[grid.len() - 1]);
    let sampling = match OutputSchedule::new(grid.to_vec()) {
        Ok(s) => OutputSamplingPlan::dense(s),
        Err(e) => return setup_failure(e.to_string()),
    };
    let (ode, _) = match facade_ode(model, None) {
        Ok(x) => x,
        Err(e) => return setup_failure(e.to_string()),
    };
    match integrate_radau_adaptive_dense_observed(
        &ode,
        span,
        y0,
        &RadauConfig::default(),
        &adaptive,
        &sampling,
    ) {
        Ok(r) => {
            let c = r.observed.counters;
            let ok = r.observed.success && r.observed.t.as_slice() == grid;
            RunOutput {
                status: if ok {
                    RunStatus::Success
                } else {
                    RunStatus::SolverFailure {
                        code: 0,
                        message: r.observed.message.clone(),
                    }
                },
                states: r.observed.y.clone(),
                work: Work {
                    accepted_steps: u(r.diagnostics.accepted_macro_steps),
                    rejected_steps: u(r.diagnostics.rejected_macro_steps),
                    step_attempts: u(r.diagnostics.attempts),
                    rhs_evaluations: Some(c.rhs_evaluations),
                    jacobian_evaluations: Some(c.jacobian_builds),
                    factorizations: Some(c.direct_factorizations),
                    linear_solves: Some(c.direct_solve_calls),
                    nonlinear_iterations: Some(c.nonlinear_iterations),
                    nonlinear_failures: Some(c.nonlinear_failures),
                },
                native: json!({"counters": c}),
                end: None,
            }
        }
        Err(e) => RunOutput {
            status: RunStatus::SolverFailure {
                code: 0,
                message: e.to_string(),
            },
            states: Vec::new(),
            work: Work::default(),
            native: Value::Null,
            end: None,
        },
    }
}
