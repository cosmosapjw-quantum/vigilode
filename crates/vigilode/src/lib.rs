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
//! | [`Jacobian::Dense`], [`Jacobian::FiniteDifference`] | none | `integrate_rodas5p_routed_observed` (rule 3); with a vector [`Atol`], `integrate_rodas5p_fast_observed_with_atol` | dense fast |
//! | [`Jacobian::Dense`] | [`Problem::banded`] | `integrate_rodas5p_routed_observed` (rule 1, band verified at `t0`) | banded fast |
//! | [`Jacobian::Banded`], [`Jacobian::FiniteDifference`] | band | `integrate_rodas5p_fast_banded_observed` (`..._with_atol` with a vector [`Atol`]) | banded fast |
//! | [`Jacobian::Jvp`] | none | `integrate_rodas5p_routed_observed` (rule 4) | matrix-free U-form (GMRES) |
//! | [`Jacobian::Dense`], [`Jacobian::FiniteDifference`] | [`Options::dense_output`], no mass | `integrate_rodas5p_fast_dense_observed` (`..._with_atol` with a vector [`Atol`]) | fast dense with continuous extension |
//! | [`Jacobian::Banded`], [`Jacobian::FiniteDifference`] | band, [`Options::dense_output`], no mass | `integrate_rodas5p_fast_banded_dense_observed` (`..._with_atol`) | banded fast dense with continuous extension |
//! | Dense, Banded, FiniteDifference | [`Problem::mass`], or dense output under [`InteriorError::Enforce`] | `integrate_adaptive_dense_observed_with_segments` (`..._with_partial` without dense output) | sequential dense (direct LU) |
//! |  | band (on the sequential route) | the same; a finite-difference or [`Jacobian::Banded`] band is assembled into a dense matrix (O(n^3) per factorization) | sequential dense (direct LU) |
//!
//! A declared band on a [`Jacobian::Dense`] provider is refused with a mass
//! matrix or dense output.
//!
//! Dense output without a mass matrix runs the fast dense drivers: the
//! solution holds every accepted step and [`Solution::at`] evaluates the
//! RODAS5P continuous extension anywhere in the span, with the interior error
//! controlled on every step by [`Options::interior_error`]
//! ([`InteriorError::WFilteredEnforce`] by default: one right-hand side and
//! one back-solve with the step's own factors per step; `Off` is the fast
//! driver bit for bit). A mass matrix (constant, singular allowed: index-1
//! DAEs), or dense output under [`InteriorError::Enforce`], runs the
//! protected sequential dense driver.
//!
//! [`Tier::Performance`] is accepted and, in this release, executes exactly
//! the `Protected` route (no candidate driver is attached yet).
//!
//! ## What is refused
//!
//! Requests that no protected driver implements are refused with
//! [`SolveError::Unsupported`] before any step rather than silently replaced:
//! a vector [`Atol`] on the matrix-free route, on the sequential route (a
//! mass matrix, or dense output under [`InteriorError::Enforce`]), or on a
//! dense Jacobian with a declared band (a vector is accepted on the dense and
//! banded fast routes, with or without dense output), a band declaration on a JVP-only
//! problem, a mass matrix or dense output on a JVP-only problem, and a
//! declared band on a dense Jacobian together with a mass matrix or dense
//! output, and events without [`Options::dense_output`].
//!
//! ## Events
//!
//! [`Problem::event`] and [`Problem::terminal_event`] add roots of `g(t, y)`
//! located by Illinois root finding on the continuous extension of the
//! accepted steps, reported in [`Solution::events`]. They are post-processing
//! and need [`Options::dense_output`]. A terminal event cuts the outputs
//! after the run: the integration is not stopped, and the work after the
//! event stays charged in [`Stats::counters`] (stopping in the step loop is
//! a later item).
//!
//! `g` is sampled at both ends and the midpoint of each accepted step, and
//! only a sign change between samples counts. An even number of crossings in
//! a half interval and a tangent touch are therefore missed, and with three
//! or more roots in a half interval the reported direction is that half
//! interval's sign change.
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
mod events;
mod fd;
mod options;
mod problem;
mod solution;
mod solve;

pub use error::{CallbackError, SolveError};
pub use events::{Direction, EventRecord};
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
