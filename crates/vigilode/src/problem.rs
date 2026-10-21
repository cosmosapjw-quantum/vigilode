//! The problem description: right-hand side, Jacobian provider, structure,
//! mass matrix and time dependence.

use std::{fmt, sync::Arc};

use crate::{
    error::CallbackError,
    events::{Direction, EventSpec},
};

/// The right-hand side `f(t, y, dy)`: writes `dy = f(t, y)`.
pub type RhsFn = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> Result<(), CallbackError> + Send + Sync>;

/// A dense Jacobian `J(t, y, jac)`: writes `jac[i * n + j] = df_i/dy_j`
/// (row major, `n * n` entries). The buffer is zeroed before every call.
pub type DenseJacobianFn =
    Arc<dyn Fn(f64, &[f64], &mut [f64]) -> Result<(), CallbackError> + Send + Sync>;

/// A band Jacobian `J(t, y, band)`: writes `df_i/dy_j` for
/// `i - lower <= j <= i + upper` at `band[i * (lower + upper + 1) + (j + lower - i)]`.
/// The buffer is zeroed before every call; entries outside the matrix are
/// never read.
pub type BandJacobianFn =
    Arc<dyn Fn(f64, &[f64], &mut [f64]) -> Result<(), CallbackError> + Send + Sync>;

/// A Jacobian-vector product `jvp(t, y, v, out)`: writes `out = J(t, y) v`.
pub type JvpFn =
    Arc<dyn Fn(f64, &[f64], &[f64], &mut [f64]) -> Result<(), CallbackError> + Send + Sync>;

/// The explicit time derivative `ft(t, y, out)`: writes `out = df/dt (t, y)`.
pub type TimeDerivativeFn =
    Arc<dyn Fn(f64, &[f64], &mut [f64]) -> Result<(), CallbackError> + Send + Sync>;

/// How the solver obtains the Jacobian `df/dy`.
#[derive(Clone, Default)]
#[non_exhaustive]
pub enum Jacobian {
    /// A dense Jacobian callback; see [`DenseJacobianFn`].
    Dense(DenseJacobianFn),
    /// A band Jacobian with `lower` sub- and `upper` super-diagonals; see
    /// [`BandJacobianFn`]. Runs the banded protected driver.
    Banded {
        /// Number of sub-diagonals.
        lower: usize,
        /// Number of super-diagonals.
        upper: usize,
        /// The band callback.
        fill: BandJacobianFn,
    },
    /// A Jacobian-vector product only; runs the matrix-free (GMRES) protected
    /// driver and never assembles a matrix.
    Jvp(JvpFn),
    /// Forward differences of the right-hand side (the default).
    ///
    /// Column `j` uses the increment `sqrt(eps) * max(|y_j|, 1)`, taken as
    /// the represented difference `(y_j + inc) - y_j`. Without a declared
    /// structure the dense Jacobian costs `n + 1` right-hand-side calls; with
    /// [`Problem::banded`] the columns are grouped (Curtis–Powell–Reid: column
    /// `j` in group `j mod (lower + upper + 1)`) and one band costs
    /// `min(lower + upper + 1, n) + 1` calls. Every call is counted in
    /// [`crate::Stats::counters`] and separately in
    /// [`crate::Stats::finite_difference`].
    #[default]
    FiniteDifference,
}

impl Jacobian {
    /// [`Jacobian::Dense`] from a closure (helps closure type inference).
    pub fn dense<F>(f: F) -> Self
    where
        F: Fn(f64, &[f64], &mut [f64]) -> Result<(), CallbackError> + Send + Sync + 'static,
    {
        Self::Dense(Arc::new(f))
    }

    /// [`Jacobian::Banded`] from a closure.
    pub fn banded<F>(lower: usize, upper: usize, f: F) -> Self
    where
        F: Fn(f64, &[f64], &mut [f64]) -> Result<(), CallbackError> + Send + Sync + 'static,
    {
        Self::Banded {
            lower,
            upper,
            fill: Arc::new(f),
        }
    }

    /// [`Jacobian::Jvp`] from a closure.
    pub fn jvp<F>(f: F) -> Self
    where
        F: Fn(f64, &[f64], &[f64], &mut [f64]) -> Result<(), CallbackError> + Send + Sync + 'static,
    {
        Self::Jvp(Arc::new(f))
    }
}

impl fmt::Debug for Jacobian {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dense(_) => f.write_str("Dense(..)"),
            Self::Banded { lower, upper, .. } => f
                .debug_struct("Banded")
                .field("lower", lower)
                .field("upper", upper)
                .finish_non_exhaustive(),
            Self::Jvp(_) => f.write_str("Jvp(..)"),
            Self::FiniteDifference => f.write_str("FiniteDifference"),
        }
    }
}

/// An initial value problem `M y' = f(t, y)` of dimension `n`.
///
/// Built with [`Problem::new`] and the builder methods; nothing is validated
/// until [`crate::Rodas5p::solve`], which refuses a malformed problem with
/// [`crate::SolveError::InvalidInput`] before any step.
#[derive(Clone)]
pub struct Problem {
    pub(crate) dimension: usize,
    pub(crate) rhs: RhsFn,
    pub(crate) jacobian: Jacobian,
    pub(crate) band: Option<(usize, usize)>,
    pub(crate) mass: Option<Vec<f64>>,
    pub(crate) time_derivative: Option<TimeDerivativeFn>,
    pub(crate) autonomous: bool,
    pub(crate) events: Vec<EventSpec>,
}

impl Problem {
    /// A problem of dimension `n` with right-hand side `rhs(t, y, dy)`, a
    /// finite-difference Jacobian, no declared structure, the identity mass
    /// matrix and an RHS that may depend on `t` (its time derivative is taken
    /// by the drivers' central difference unless [`Problem::time_derivative`]
    /// or [`Problem::autonomous`] is given).
    pub fn new<F>(n: usize, rhs: F) -> Self
    where
        F: Fn(f64, &[f64], &mut [f64]) -> Result<(), CallbackError> + Send + Sync + 'static,
    {
        Self {
            dimension: n,
            rhs: Arc::new(rhs),
            jacobian: Jacobian::FiniteDifference,
            band: None,
            mass: None,
            time_derivative: None,
            autonomous: false,
            events: Vec::new(),
        }
    }

    /// Set the Jacobian provider.
    pub fn jacobian(mut self, jacobian: Jacobian) -> Self {
        self.jacobian = jacobian;
        self
    }

    /// Declare that the Jacobian is zero outside `lower` sub- and `upper`
    /// super-diagonals.
    ///
    /// * With [`Jacobian::FiniteDifference`] the band is built by grouped
    ///   forward differences and the banded driver runs.
    /// * With [`Jacobian::Dense`] the band is read from the dense callback and
    ///   verified at `(t0, y0)` against the full dense Jacobian by the
    ///   protected router before any step; a mismatch refuses the run
    ///   ([`crate::SolveError::StructureRefused`]).
    /// * With [`Jacobian::Banded`] the declaration must name the same band.
    /// * With [`Jacobian::Jvp`] it is refused as unsupported in this release.
    ///
    /// Under [`Problem::mass`] or [`crate::Options::dense_output`] the run
    /// goes to the sequential dense driver instead, which has no banded
    /// linear algebra. There a finite-difference band or a
    /// [`Jacobian::Banded`] provider is assembled into a dense matrix and
    /// factorized by a direct dense LU, so the cost per factorization is
    /// O(n^3) rather than O(n (lower + upper)^2). A [`Jacobian::Dense`]
    /// provider with a band is refused on that route
    /// ([`crate::SolveError::StructureRefused`]).
    pub fn banded(mut self, lower: usize, upper: usize) -> Self {
        self.band = Some((lower, upper));
        self
    }

    /// A constant mass matrix `M` (row major, `n * n`), which may be
    /// singular (an index-1 DAE: a zero row makes that equation algebraic).
    ///
    /// The run goes to the protected sequential dense driver (direct LU of
    /// `M - h gamma J`), which needs a matrix Jacobian: [`Jacobian::Dense`],
    /// [`Jacobian::Banded`] or [`Jacobian::FiniteDifference`]. A JVP-only
    /// problem with a mass matrix is refused with
    /// [`crate::SolveError::Unsupported`].
    pub fn mass(mut self, matrix: Vec<f64>) -> Self {
        self.mass = Some(matrix);
        self
    }

    /// The explicit time derivative `df/dt`, used instead of the drivers'
    /// central difference in `t`. Incompatible with [`Problem::autonomous`].
    pub fn time_derivative<F>(mut self, ft: F) -> Self
    where
        F: Fn(f64, &[f64], &mut [f64]) -> Result<(), CallbackError> + Send + Sync + 'static,
    {
        self.time_derivative = Some(Arc::new(ft));
        self
    }

    /// Declare that `f` does not depend on `t`: the drivers skip the time
    /// derivative. A false declaration changes the result.
    pub fn autonomous(mut self) -> Self {
        self.autonomous = true;
        self
    }

    /// Add an event `g(t, y) = 0`, located after the run on the continuous
    /// extension of the accepted steps. Events are numbered in declaration
    /// order ([`crate::EventRecord::index`]).
    ///
    /// Requires [`crate::Options::dense_output`]: without it the solve is
    /// refused with [`crate::SolveError::Unsupported`]. On each accepted step
    /// `g` is sampled at both ends and at the midpoint; a half interval with
    /// `g(a) < 0 <= g(b)` is an `Up` crossing and one with `g(a) > 0 >= g(b)`
    /// a `Down` crossing, and `direction` selects which count. Illinois
    /// root finding then narrows the bracket to `4 eps max(|t|, 1)` (60
    /// Illinois iterations, then bisection until the tolerance is met) and the
    /// end on the after-crossing side is reported. So `g(t0) = 0` is no
    /// event, and a root at a step end is reported once.
    ///
    /// Only the sign change between the three samples counts. An even number
    /// of crossings inside one half interval is missed, and so is a tangent
    /// touch without a sign change. With three or more roots in a half
    /// interval one event is reported, with that half interval's direction.
    ///
    /// A non-finite `g` ends the run with
    /// [`crate::FailureReason::EventFunctionNonFinite`] and truncates the
    /// outputs at the start of that step; if the driver had already failed,
    /// the driver's own reason is kept and the outputs are still truncated.
    pub fn event<G>(self, g: G, direction: Direction) -> Self
    where
        G: Fn(f64, &[f64]) -> f64 + Send + Sync + 'static,
    {
        self.push_event(g, direction, false)
    }

    /// Like [`Problem::event`], and the first such event in time ends the
    /// outputs: `t`, `y` and the continuous extension are cut at the event,
    /// `(t_e, y_e)` is appended, and the status is `Success` even if the
    /// driver failed later ([`crate::Stats::stop`] keeps that record).
    ///
    /// The cut is made after the run. The integration is not stopped, and the
    /// work after the event stays charged in [`crate::Stats::counters`].
    /// Stopping inside the step loop is not available in this release.
    pub fn terminal_event<G>(self, g: G, direction: Direction) -> Self
    where
        G: Fn(f64, &[f64]) -> f64 + Send + Sync + 'static,
    {
        self.push_event(g, direction, true)
    }

    fn push_event<G>(mut self, g: G, direction: Direction, terminal: bool) -> Self
    where
        G: Fn(f64, &[f64]) -> f64 + Send + Sync + 'static,
    {
        self.events.push(EventSpec {
            g: Arc::new(g),
            direction,
            terminal,
        });
        self
    }

    /// The dimension `n`.
    pub fn dimension(&self) -> usize {
        self.dimension
    }
}

impl fmt::Debug for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Problem")
            .field("dimension", &self.dimension)
            .field("jacobian", &self.jacobian)
            .field("band", &self.band)
            .field("mass", &self.mass.is_some())
            .field("time_derivative", &self.time_derivative.is_some())
            .field("autonomous", &self.autonomous)
            .field("events", &self.events.len())
            .finish_non_exhaustive()
    }
}
