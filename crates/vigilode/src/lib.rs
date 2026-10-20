//! # VigilODE
//!
//! One problem type, one options type, one solver and one result over the
//! protected RODAS5P drivers of `rodas5p-integrators` (plan §3, "The public
//! API").
//!
//! ```
//! use vigilode::{Atol, CallbackError, Jacobian, Options, Problem, Rodas5p, Status};
//!
//! // Robertson's stiff chemical kinetics, with an analytic Jacobian.
//! let problem = Problem::new(3, |_t, y, dy| {
//!     dy[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
//!     dy[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
//!     dy[2] = 3.0e7 * y[1] * y[1];
//!     Ok::<(), CallbackError>(())
//! })
//! .jacobian(Jacobian::dense(|_t, y, jac| {
//!     // Row major: jac[i * 3 + j] = d f_i / d y_j.
//!     jac.copy_from_slice(&[
//!         -0.04, 1.0e4 * y[2], 1.0e4 * y[1],
//!         0.04, -1.0e4 * y[2] - 6.0e7 * y[1], -1.0e4 * y[1],
//!         0.0, 6.0e7 * y[1], 0.0,
//!     ]);
//!     Ok(())
//! }))
//! .autonomous();
//!
//! let opts = Options { rtol: 1e-6, atol: Atol::Scalar(1e-10), ..Default::default() };
//! let sol = Rodas5p::solve(&problem, (0.0, 40.0), &[1.0, 0.0, 0.0], &opts)?;
//! assert_eq!(sol.status, Status::Success);
//! let y = sol.final_state().unwrap();
//! assert!((y[0] + y[1] + y[2] - 1.0).abs() < 1e-8); // mass is conserved
//! assert!(sol.stats.accepted_steps > 0);
//! # Ok::<(), vigilode::SolveError>(())
//! ```
//!
//! ## Routing
//!
//! [`Tier::Protected`] calls only existing protected drivers:
//!
//! | Jacobian provider | Structure | Entry point | Driver |
//! | --- | --- | --- | --- |
//! | [`Jacobian::Dense`], [`Jacobian::FiniteDifference`] | none | `integrate_rodas5p_routed_observed` (rule 3) | dense fast |
//! | [`Jacobian::Dense`] | [`Problem::banded`] | `integrate_rodas5p_routed_observed` (rule 1, band verified at `t0`) | banded fast |
//! | [`Jacobian::Banded`], [`Jacobian::FiniteDifference`] | band | `integrate_rodas5p_fast_banded_observed` | banded fast |
//! | [`Jacobian::Jvp`] | none | `integrate_rodas5p_routed_observed` (rule 4) | matrix-free U-form (GMRES) |
//! | Dense, Banded, FiniteDifference | [`Problem::mass`] or [`Options::dense_output`] | `integrate_adaptive_dense_observed_with_segments` | sequential dense (direct LU) |
//!
//! A mass matrix (constant, singular allowed: index-1 DAEs) or dense output
//! runs the protected sequential dense driver. Under dense output the
//! solution holds every accepted step and [`Solution::at`] evaluates the
//! RODAS5P continuous extension anywhere in the span, with the interior error
//! controlled on every step by [`Options::interior_error`]
//! ([`InteriorError::WFilteredEnforce`] by default).
//!
//! [`Tier::Performance`] is accepted and, in this release, executes exactly
//! the `Protected` route (no candidate driver is attached yet).
//!
//! ## What is refused
//!
//! Requests that no protected driver implements are refused with
//! [`SolveError::Unsupported`] before any step rather than silently replaced:
//! a vector [`Atol`], a band declaration on a JVP-only problem, a mass matrix
//! or dense output on a JVP-only problem, and a declared band on a dense
//! Jacobian together with a mass matrix or dense output.
//!
//! ## Failures
//!
//! A run that stops before `t1` is returned as `Ok` with [`Status::Failed`],
//! its partial outputs (up to the last accepted state) and its full
//! [`Stats`]. The [`FailureReason`] names what stopped it: the attempt
//! budget, a [`CallbackError::Fatal`], or the step size falling below the
//! minimum after repeated local-error, linear-solve, non-finite or
//! nonlinear-solve rejections ([`Stats::stop`] keeps the driver's record).
//! Errors (`Err`) are refusals before integration, or a driver error that
//! left no partial result.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod error;
mod fd;
mod options;
mod problem;
mod solution;
mod solve;

pub use error::{CallbackError, SolveError};
pub use options::{Atol, InteriorError, Options, Tier};
pub use problem::{
    BandJacobianFn, DenseJacobianFn, Jacobian, JvpFn, Problem, RhsFn, TimeDerivativeFn,
};
pub use solution::{
    Callback, CallbackWork, DriverWork, FD_INCREMENT_RULE, FailureReason, FiniteDifferenceWork,
    JacobianRecord, KrylovStartRecord, RouteRecord, STATS_SCHEMA, Solution, Stats, Status,
    StopRecord,
};
pub use solve::Rodas5p;
