//! The residual-accounting contract of the production restarted GMRES
//! kernels, [`crate::solve_gmres_with_workspace_and_accounting`] and
//! [`crate::solve_gmres_into_with_accounting`] (research node
//! `research/sp01_dupfix_adoption_20261010`, SP01).
//!
//! Both kernels test a true residual `b - A x` of the current iterate at the
//! top of every restart pass and leave the loop only when it meets the
//! threshold (or fail on budget exhaustion or a breakdown). After the loop
//! the legacy kernels compute the true residual of the same iterate once
//! more (a `Diagnostic` operator application) and report its norm.
//!
//! * [`ResidualAccounting::RecomputeFinal`] (v1) is that legacy behaviour.
//! * [`ResidualAccounting::ReuseConfirmed`] (v2) skips the final residual
//!   only when the loop left on a true residual of exactly the current
//!   iterate that the operator computed: at least one restart cycle ran and
//!   the iterate is not identically zero (for a zero iterate the loop uses
//!   `b` itself and applies no operator). The report then carries that
//!   residual's norm, which is the recomputation's bit for bit (same
//!   operator, same iterate, same arithmetic). Every other exit keeps the
//!   legacy recomputation: a zero right-hand side or any other exit at a zero
//!   iterate, and a nonzero initial guess that already met the threshold
//!   before any cycle. Failures (budget exhaustion, breakdown) return before
//!   the final residual in both accountings, as before.
//!
//! The solution, the decisions and every report field are the same under
//! both accountings; only the `Diagnostic` operator applications (and the
//! JVP/matvec work they charge) differ, by one per successful solve that
//! left on a confirmed residual after at least one cycle.
//!
//! ALG04's `GmresIntoOptions::skip_final_residual` (research arm `DupFix`)
//! is kept unchanged: it reuses the loop's residual norm at every exit,
//! including the zero-iterate and no-cycle exits, and overrides the
//! accounting when set.
//!
//! The default of both kernels is [`ResidualAccounting::DEFAULT`], fixed by
//! SP01's registered decision rule: with `ReuseConfirmed` as the default of
//! both kernels nine pre-existing tests failed (they pin diagnostic counts,
//! counter digests or recorded base exports; the receipt is
//! `research/sp01_dupfix_adoption_20261010/DEFAULT_TRIAL.json`), so the
//! default stays `RecomputeFinal` and `ReuseConfirmed` is the explicit,
//! versioned v2 accounting.

use serde::{Deserialize, Serialize};

/// How a GMRES kernel accounts for the true residual of its final iterate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResidualAccounting {
    /// v1: recompute the final true residual after the loop (legacy).
    RecomputeFinal,
    /// v2: reuse the loop's confirmed true residual of the same iterate.
    ReuseConfirmed,
}

impl ResidualAccounting {
    /// The kernels' default accounting (SP01 decision rule outcome:
    /// "versioned", so v1).
    pub const DEFAULT: Self = Self::RecomputeFinal;

    /// Stable identifier for research records.
    pub const fn id(self) -> &'static str {
        match self {
            Self::RecomputeFinal => "gmres-residual-accounting-recompute-final-v1",
            Self::ReuseConfirmed => "gmres-residual-accounting-reuse-confirmed-v2",
        }
    }
}

impl Default for ResidualAccounting {
    /// [`ResidualAccounting::DEFAULT`].
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// What the loop of a GMRES kernel left with.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LoopExit {
    /// The norm of the true residual that met the threshold.
    pub residual_norm: f64,
    /// That residual came from an operator application on the current
    /// (nonzero) iterate after at least one restart cycle.
    pub confirmed: bool,
}

/// Whether the final residual is reused (`true`) or recomputed.
pub(crate) fn reuse_final_residual(
    accounting: ResidualAccounting,
    exit: LoopExit,
    skip_always: bool,
) -> bool {
    skip_always || (accounting == ResidualAccounting::ReuseConfirmed && exit.confirmed)
}
