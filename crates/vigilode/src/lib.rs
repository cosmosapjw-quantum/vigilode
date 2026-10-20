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
//!
//! [`Tier::Performance`] is accepted and, in this release, executes exactly
//! the `Protected` route (no candidate driver is attached yet).
//!
//! ## What is refused
//!
//! Requests that no protected driver implements are refused with
//! [`SolveError::Unsupported`] before any step rather than silently replaced:
//! a vector [`Atol`], a mass matrix, [`Options::dense_output`] and a band
//! declaration on a JVP-only problem.
//!
//! ## Failures
//!
//! A run that stops before `t1` (attempt budget, minimum step, or a
//! [`CallbackError::Fatal`]) is returned as `Ok` with [`Status::Failed`], its
//! partial outputs and its full [`Stats`]. Errors (`Err`) are refusals before
//! integration, or a driver error that left no partial result.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod error;
mod fd;
mod options;
mod problem;
mod solution;
mod solve;

pub use error::{CallbackError, SolveError};
pub use options::{Atol, Options, Tier};
pub use problem::{
    BandJacobianFn, DenseJacobianFn, Jacobian, JvpFn, Problem, RhsFn, TimeDerivativeFn,
};
pub use solution::{
    Callback, CallbackWork, DriverWork, FD_INCREMENT_RULE, FailureReason, FiniteDifferenceWork,
    JacobianRecord, KrylovStartRecord, RouteRecord, STATS_SCHEMA, Solution, Stats, Status,
};
pub use solve::Rodas5p;
