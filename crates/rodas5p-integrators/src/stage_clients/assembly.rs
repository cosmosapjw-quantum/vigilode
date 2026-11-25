//! Exact threaded and coloured finite-difference Jacobian assembly as an
//! opt-in stage client (loop node V08; evidence in
//! `research/production_loop_20261116/jacobian_assembly/`).
//!
//! [`AssemblyStageClient`] builds the Jacobian of a problem from its
//! right-hand side at every linearization and hands it to one of the kit's
//! linear back ends ([`LinearBackend`]), which factors `W` and solves the
//! stages unchanged. The problem's own Jacobian or JVP callback is never
//! called. Nothing here is selected by default.
//!
//! # What is computed
//!
//! The arithmetic is that of the facade's finite differences,
//! `crates/vigilode/src/fd.rs`: the increment of column `j` is the
//! represented difference `(y_j + sqrt(eps) max(|y_j|, 1)) - y_j`
//! (`fd.rs:443-455`) and an entry is `(f(t, y + shift)[i] - f(t, y)[i]) / delta_j`
//! (`fd.rs:499`, `fd.rs:611`).
//!
//! - [`ColumnRoute::Dense`]: one right-hand side per column and the base
//!   evaluation, `n + 1` per build, the loop of `fd.rs:476-509`.
//! - [`ColumnRoute::Coloured`]: the columns of a declared
//!   [`SparsityPattern`] are grouped by Curtis-Powell-Reid
//!   ([`colour_columns`]) and one right-hand side shifts every column of a
//!   group, `colours + 1` per build, the loop of `fd.rs:581-622`. For a
//!   full band the groups are the facade's (`j mod (lower + upper + 1)`).
//!
//! With [`AssemblyExecution::Sequential`] everything runs on the calling
//! thread in the order of `fd.rs`. With [`AssemblyExecution::Threaded`] the
//! right-hand sides of a build run on a rayon pool that the client owns
//! (exactly `workers` threads; the global pool is never used) while the
//! calling thread waits. Each entry is computed from the same two vectors
//! and the same increment as in the sequential order, so the Jacobian, and
//! with it the run of the driver, has the same bits, provided the
//! right-hand side keeps the contract below.
//!
//! # Purity contract of threaded execution
//!
//! In a threaded build the right-hand side is called from several threads
//! at once, for different shifted states, in no fixed order. `Send + Sync`,
//! which the callback type requires, only makes that memory-safe. The
//! caller of [`RhsPurity::pure_and_reentrant`] states more:
//!
//! 1. the result is a function of `(t, y)` alone: no state carried from one
//!    call to the next (caches keyed on the previous call, counters that
//!    feed the result, random numbers, adaptive tables);
//! 2. concurrent calls do not influence each other: shared scratch is
//!    either absent or its use cannot change a result;
//! 3. the result does not depend on the thread that runs the call:
//!    no thread-local state, and the same floating-point environment
//!    (rounding mode, flush-to-zero) on every thread.
//!
//! A right-hand side that breaks the contract gives a Jacobian that
//! depends on scheduling. The client cannot prove the contract; it checks
//! it where it can (next section) and fails closed when a check fails.
//!
//! # Checks: a wrong pattern or an impure right-hand side fails closed
//!
//! - Every coloured build: a row that no column of a colour declares must
//!   come back from that colour's evaluation with the bits of the base
//!   evaluation. No extra right-hand side.
//! - First linearization of a client, and again on request
//!   ([`AssemblyReportHandle::request_check`], [`AssemblyStageClient::verify`]):
//!   a check ([`CheckColumns`]) after the build. The default first-use check
//!   follows the route and, on the coloured route, the declared pattern
//!   ([`AssemblyOptions::default_first_use_check`]; loop nodes V29 and V31,
//!   owner decisions OD-L1-24 and OD-L1-27):
//!   - coloured route, whatever the back end (the band back ends included)
//!     and the execution: the multi-colouring check
//!     [`CheckColumns::MultiColouring`], on the calling thread. Every group
//!     of a few further colourings of the declared pattern, whose colour
//!     tuples tell every two columns apart ([`CheckColourings`]), is shifted
//!     at once; every declared entry must have the bits of the entry of the
//!     client's own colouring, and every row the group does not cover must
//!     keep its bits. [`CheckColourings::total_groups`] right-hand sides: 18,
//!     59 and 150 for the bands of the cells bruss1d-n128, bruss1d-n131072
//!     and nlkpp-band-n4096-r24 of node V08. Where the further colourings
//!     have `n` groups or more (wide declared bands), the full check
//!     [`CheckColumns::All`] instead: every column alone, on the calling
//!     thread, `n` right-hand sides, exact up to rounding;
//!   - dense route: 8 evenly spread check columns ([`CheckColumns::Sample`]),
//!     one at a time on the calling thread; each must have the bits of the
//!     assembled column. The multi-colouring check needs a declared pattern
//!     and does not apply. Only threaded execution makes this check: the
//!     sequential dense route has nothing to compare with and makes no check
//!     unless one is requested.
//!
//!   Threaded execution repeats the base evaluation on the calling thread in
//!   front of every check.
//! - Every check stays selectable ([`AssemblyOptions::with_first_use_check`],
//!   [`AssemblyReportHandle::request_check`], [`AssemblyStageClient::verify`]):
//!   [`CheckColumns::None`], [`CheckColumns::Sample`] (single columns
//!   evenly spread), [`CheckColumns::All`] (every column alone),
//!   [`CheckColumns::AllPooled`] and [`CheckColumns::MultiColouringPooled`]
//!   (the evaluations on the client's pool; they report the failure the
//!   order of the check on the calling thread meets first, so they decide as
//!   [`CheckColumns::All`] and [`CheckColumns::MultiColouring`]). The
//!   multi-colouring checks need the coloured route.
//!
//! A failed check is an [`AssemblyError`]: the driver receives
//! [`CoreError::InvalidInput`] (fatal, the run ends with `Err`) and the
//! typed error stays readable through the [`AssemblyReportHandle`].
//! What the checks detect and what they cannot see is stated at
//! [`CheckColumns`]: in short, no check sees an undeclared entry whose
//! effect on its row is lost to rounding at every checked state;
//! [`CheckColumns::All`] and [`CheckColumns::AllPooled`] are the exact choice
//! up to rounding; the multi-colouring check detected every wrong declaration
//! of the frozen corpus of node V26 that the full check detected, but it
//! misses rows whose undeclared couplings cancel in pairs and entry collapse
//! (ledger L-0338); sampled columns miss undeclared entries aliased into
//! declared entries of columns that were not sampled.
//!
//! # Counters and allocation
//!
//! Every right-hand side is called through [`OdeProblem::eval_rhs_into`]
//! and charged to the run's counters (`rhs_calls`, `rhs_evaluations`);
//! the client reports assembly and verification calls separately
//! ([`AssemblyStats`]). The sequential routes allocate nothing after
//! [`AssemblyStageClient::new`] (with an allocation-free back end; the
//! single-column checks work in the client's own scratch, the further
//! colourings of a multi-colouring first-use check are built there, those
//! of a requested multi-colouring check at its first use); the threaded
//! routes leave allocation to rayon.

mod engine;
mod multi;
mod pattern;

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use rodas5p_core::{CoreError, CoreResult, DenseMatrix, WorkCounters};

use self::engine::{BuildFailure, Engine, Tally, TargetLayout};
pub use self::multi::{CheckColourings, CheckConstruction};
pub use self::pattern::{SparsityPattern, colour_columns, colouring_is_valid};
use super::{BandedTightStageLu, DenseLuMode, DenseLuStageClient};
use crate::{BandedJacobian, BandedJacobianFn, BandedStageLu, OdeProblem, StageLinearSystem};

/// A failure of the assembly contract, or a refused configuration.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum AssemblyError {
    /// The client cannot be built as requested.
    #[error("invalid assembly configuration: {0}")]
    Configuration(String),
    /// Coloured route, every build: `row` changed in the evaluation of
    /// `colour` although no column of the colour declares it.
    #[error(
        "the declared sparsity pattern is wrong: row {row} changed in the evaluation of colour \
         {colour}, none of whose columns declares an entry in that row (or the right-hand side \
         is not a function of its input)"
    )]
    UncoveredRowChanged { row: usize, colour: usize },
    /// Coloured route, check column: shifting `column` alone changed `row`,
    /// which the pattern does not declare for it.
    #[error(
        "the declared sparsity pattern is wrong: entry ({row}, {column}) is not declared but \
         row {row} changes with column {column}"
    )]
    UndeclaredEntry { row: usize, column: usize },
    /// Coloured route, check column: the entry assembled from the colour
    /// differs from the single-column difference, so another column of the
    /// colour reaches the row (an undeclared entry aliased into a declared
    /// one) or the right-hand side is not reproducible.
    #[error(
        "entry ({row}, {column}) assembled from its colour is {assembled:e} but the column \
         alone gives {single_column:e}: another column of the colour reaches row {row} (wrong \
         pattern) or the right-hand side is not a function of its input"
    )]
    AliasedEntry {
        row: usize,
        column: usize,
        assembled: f64,
        single_column: f64,
    },
    /// A right-hand side evaluated again on the calling thread did not
    /// reproduce the bits of the assembly (`column` is `None` for the base
    /// evaluation): the purity contract is broken.
    #[error(
        "the right-hand side is not reproducible: row {row} of column {column:?} (None: the \
         base evaluation) differs between the assembly and a repeated evaluation"
    )]
    NotReproducible { row: usize, column: Option<usize> },
    /// Multi-colouring check ([`CheckColumns::MultiColouring`]): `row`
    /// changed in the evaluation of group `group` of further colouring
    /// `colouring` (1, 2, ..; the client's own colouring is 0), although no
    /// column of the group declares an entry in that row.
    #[error(
        "the declared sparsity pattern is wrong: row {row} changed in the evaluation of group \
         {group} of check colouring {colouring}, none of whose columns declares an entry in that \
         row (or the right-hand side is not a function of its input)"
    )]
    MultiColouringUncoveredRow {
        row: usize,
        colouring: usize,
        group: usize,
    },
    /// Multi-colouring check: entry `(row, column)` computed from group
    /// `group` of further colouring `colouring` differs from the entry the
    /// client's own colouring (colouring 0) assembled. An undeclared
    /// dependency of the row shares a group with `column` in one of the two
    /// colourings and not in the other (wrong pattern), or the right-hand
    /// side is not a function of its input.
    #[error(
        "entry ({row}, {column}) assembled from the client's colouring is {assembled:e} but group \
         {group} of check colouring {colouring} gives {recomputed:e}: an undeclared dependency of \
         row {row} is aliased into the entry in one of the two colourings (wrong pattern) or the \
         right-hand side is not a function of its input"
    )]
    MultiColouringEntryDiffers {
        row: usize,
        column: usize,
        colouring: usize,
        group: usize,
        assembled: f64,
        recomputed: f64,
    },
    /// A back end asked for the Jacobian at a state it was not assembled
    /// for.
    #[error("the assembled Jacobian was requested for a state it was not assembled at")]
    StaleRequest,
    /// The right-hand side failed during [`AssemblyStageClient::verify`].
    #[error("the right-hand side failed during a requested check: {0}")]
    RightHandSide(String),
}

impl AssemblyError {
    /// Whether the error says that the declared pattern (or the purity of
    /// the right-hand side) is violated, as opposed to a refused
    /// configuration or a failed callback.
    pub fn is_contract_violation(&self) -> bool {
        matches!(
            self,
            Self::UncoveredRowChanged { .. }
                | Self::UndeclaredEntry { .. }
                | Self::AliasedEntry { .. }
                | Self::NotReproducible { .. }
                | Self::MultiColouringUncoveredRow { .. }
                | Self::MultiColouringEntryDiffers { .. }
        )
    }
}

/// The caller's acknowledgement of the purity contract of threaded assembly
/// (module documentation). There is no default: threaded execution cannot be
/// requested without naming the contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RhsPurity(());

impl RhsPurity {
    /// The caller states that the right-hand side of the problem is a pure
    /// function of `(t, y)`, may be called from several threads at once,
    /// and returns the same bits whichever thread runs it and in whatever
    /// order the calls are made.
    pub fn pure_and_reentrant() -> Self {
        Self(())
    }
}

/// Where the right-hand sides of a build run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssemblyExecution {
    /// On the calling thread, in the order of the facade's finite
    /// differences. No thread pool is created.
    Sequential,
    /// On a pool of `workers` threads owned by the client. Opt-in: needs
    /// the [`RhsPurity`] acknowledgement.
    Threaded { workers: usize, purity: RhsPurity },
}

/// Which right-hand sides a build evaluates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ColumnRoute {
    /// One per column: `n + 1` per build.
    Dense,
    /// One per Curtis-Powell-Reid colour of the declared pattern:
    /// `colours + 1` per build.
    Coloured(SparsityPattern),
}

/// What a check evaluates after a build (module documentation).
///
/// The default first-use check follows the route
/// ([`AssemblyOptions::default_first_use_check`], loop nodes V29 and V31):
/// on the coloured route (the band back ends included) `MultiColouring`
/// where [`CheckColourings::total_groups_of`] the declared pattern is below
/// `n` and `All` where it is `n` or more; `Sample(8)`
/// ([`AssemblyOptions::DEFAULT_CHECK_COLUMNS`]) on the dense route, where the
/// multi-colouring check does not apply (no declared pattern). Every variant
/// can be chosen instead
/// ([`AssemblyOptions::with_first_use_check`],
/// [`AssemblyReportHandle::request_check`], [`AssemblyStageClient::verify`]);
/// the multi-colouring checks need the coloured route.
///
/// # What the checks detect and what they cannot see
///
/// No check has a tolerance: an undeclared dependency shows only through
/// changed bits, and no check sees one whose effect on its row is lost to
/// rounding at every state the check evaluates (for example a finite
/// difference that is exactly zero there).
///
/// - `All` and `AllPooled` shift every column alone: an undeclared entry
///   `(i, k)` is detected whenever the shift of `y_k` alone changes a bit of
///   row `i` at the checked state. They are the exact choice up to rounding,
///   at the cost of `n` right-hand sides.
/// - `MultiColouring` and `MultiColouringPooled`: on the frozen corpus of
///   loop node V26 (4846 cases with at most one undeclared dependency per
///   row: the negative controls of node V08, the periodic ring declared
///   tridiagonal for 13 orders, every single-entry omission of the
///   Brusselator pattern of orders 128 and 1024 and 200 of order 131072,
///   bands one too narrow, and 23 correct patterns;
///   `research/production_loop_20261116/assembly_check/`) they detect every
///   wrong declaration the full check detects, miss the same single
///   rounding-limited case and reject no correct pattern, with 150 instead
///   of 4096 right-hand sides for the band (24, 24) of order 4096 and 59
///   instead of 131072 for the band (2, 2) of order 131072. They are not
///   exact in general (ledger L-0338, the review of node V26; reproducible
///   with `research/production_loop_20261116/assembly_check/review_probe`,
///   subcommands `cancel` and `collapse`):
///   - pairwise cancelling undeclared couplings: a row with several
///     undeclared couplings whose contributions cancel in pairs at the
///     checked state, in every group of every colouring (the client's own
///     included), passes; constructed for both constructions of
///     [`CheckColourings`]. The full check detects such a row; `Sample(8)`
///     passed the constructed rows too;
///   - entry collapse: an undeclared coupling of about one ulp of its row
///     (constructed, about 1e-16) changes the bits of the row, but each pair
///     of entries the check compares rounds to the same number after the
///     subtraction of the base value and the division by the increment, so
///     it passes. The full check detects it, and so does `Sample(8)` when
///     the column is one of its samples.
/// - `Sample(k)` sees only its `k` columns: an undeclared entry aliased into
///   a declared entry of the same colour in a column that is not sampled
///   passes (576 wrong declarations of the V26 corpus pass `Sample(8)`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckColumns {
    /// No check.
    None,
    /// This many columns (at most `n`), evenly spread: column
    /// `floor((2 s + 1) n / (2 k))` for `s = 0 .. k`, one at a time on the
    /// calling thread. `Sample(8)` is the default first-use check of the
    /// dense route.
    Sample(usize),
    /// Every column, one at a time on the calling thread: `n` right-hand
    /// sides. The default first-use check of the coloured route where the
    /// multi-colouring check would need `n` or more (loop node V31).
    All,
    /// Every column with the comparisons of [`CheckColumns::All`], the `n`
    /// single-column evaluations handed to the threads of the client's pool
    /// (threaded execution; a sequential client runs them on the calling
    /// thread). Decides as [`CheckColumns::All`]: the failure of the
    /// smallest failing column is reported.
    AllPooled,
    /// The multi-colouring check of loop node V26 (coloured route only;
    /// [`CheckColourings`]): every group of every further colouring of the
    /// declared pattern is evaluated once on the calling thread, every
    /// declared entry is compared bit for bit with the entry of the
    /// client's own colouring, and every row a group does not cover must
    /// keep its bits. [`CheckColourings::total_groups`] right-hand sides,
    /// for example 59 for a band `(2, 2)` of order 131072. For wide declared
    /// bands the groups can outnumber the columns: the review of loop node
    /// V29 counted more than `n` groups for 59 of 75 random patterns whose
    /// band half-width lies between `0.2 n` and `0.45 n`, at most `1.85 n`;
    /// [`CheckColumns::All`] is then cheaper and exact up to rounding, and
    /// the default first-use check takes it (loop node V31). The default
    /// first-use check of the coloured route where it needs fewer than `n`
    /// right-hand sides (loop nodes V29 and V31); what it detects and what
    /// it misses (ledger L-0338) is stated at the type.
    MultiColouring,
    /// [`CheckColumns::MultiColouring`] with the groups handed to the
    /// threads of the client's pool; decides as the check on the calling
    /// thread.
    MultiColouringPooled,
}

/// The linear algebra behind the assembled Jacobian: a client of the kit,
/// unchanged, that reads the Jacobian the assembly serves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinearBackend {
    /// Dense storage and the LU of the protected dense driver
    /// ([`DenseLuStageClient`]).
    DenseLu(DenseLuMode),
    /// Band storage and the protected banded kernel ([`BandedStageLu`],
    /// kernel `Indexed`). Coloured route only; the pattern must lie inside
    /// the band.
    BandedProtected { lower: usize, upper: usize },
    /// Band storage and the tight kernel of loop node V21
    /// ([`BandedTightStageLu`], bit-identical to the protected kernel).
    /// Coloured route only; the pattern must lie inside the band.
    BandedTight { lower: usize, upper: usize },
}

/// How an [`AssemblyStageClient`] assembles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssemblyOptions {
    pub route: ColumnRoute,
    pub execution: AssemblyExecution,
    /// The check of the first linearization (module documentation):
    /// [`AssemblyOptions::default_first_use_check`] of the route unless
    /// stated otherwise. Not applied to the sequential dense route.
    pub first_use_check: CheckColumns,
    /// Keep running fingerprints of every linearization point and every
    /// assembled Jacobian in [`AssemblyStats`] (one pass over the stored
    /// entries per build; for identity tests).
    pub fingerprints: bool,
}

impl AssemblyOptions {
    /// Check columns of the default first-use check of the dense route
    /// ([`AssemblyOptions::default_first_use_check`]); before loop node V29
    /// the default first-use check of every route.
    pub const DEFAULT_CHECK_COLUMNS: usize = 8;

    /// The first-use check of `route` unless stated otherwise (loop nodes
    /// V29 and V31, owner decisions OD-L1-24 and OD-L1-27):
    ///
    /// - [`ColumnRoute::Coloured`], whatever the back end (the band back
    ///   ends included) and the execution, decided from the declared pattern
    ///   alone by the number of right-hand sides the multi-colouring check
    ///   would spend, [`CheckColourings::total_groups_of`] the pattern
    ///   (no state and no right-hand side enter):
    ///   - fewer than `n`: [`CheckColumns::MultiColouring`], on the calling
    ///     thread. On the frozen corpus of loop node V26 it detects every
    ///     wrong declaration that the full check detects (one
    ///     rounding-limited case escapes both), where `Sample(8)` passes 576
    ///     wrong declarations. It is not exact in general: rows whose
    ///     undeclared couplings cancel in pairs and entry collapse pass it
    ///     (ledger L-0338; reproducible with
    ///     `research/production_loop_20261116/assembly_check/review_probe`,
    ///     subcommands `cancel` and `collapse`);
    ///   - `n` or more: [`CheckColumns::All`], on the calling thread. The
    ///     full check shifts every column alone, is the exact choice up to
    ///     rounding, and costs `n` right-hand sides, so it is not more
    ///     expensive there; at equality it costs as much and is exact. The
    ///     further colourings outnumber the columns for wide declared
    ///     bands: the review of loop node V29 counted more than `n` groups
    ///     for 59 of 75 random banded patterns with half-width between
    ///     `0.2 n` and `0.45 n` (at most `1.85 n`), and the survey of loop
    ///     node V31 (`research/production_loop_20261116/assembly_wide_band/`)
    ///     `n` or more for 184 of 378 random banded patterns of order 60 to
    ///     500 with half-width between `0.05 n` and `0.45 n`.
    ///
    ///   A pattern whose own colouring already shifts every column alone (a
    ///   declared dense pattern, for example) has no further colourings: its
    ///   multi-colouring check spends no right-hand side and it stays the
    ///   default. [`CheckColumns::AllPooled`] and the multi-colouring checks
    ///   stay selectable; [`CheckColumns`] states what each check detects.
    /// - The dense route ([`ColumnRoute::Dense`]): `Sample(8)`
    ///   ([`Self::DEFAULT_CHECK_COLUMNS`]). The multi-colouring check needs a
    ///   declared pattern and does not apply; the sequential dense route
    ///   makes no first-use check at all.
    pub fn default_first_use_check(route: &ColumnRoute) -> CheckColumns {
        match route {
            ColumnRoute::Dense => CheckColumns::Sample(Self::DEFAULT_CHECK_COLUMNS),
            ColumnRoute::Coloured(pattern) => {
                if CheckColourings::total_groups_of(pattern) >= pattern.n() {
                    CheckColumns::All
                } else {
                    CheckColumns::MultiColouring
                }
            }
        }
    }

    /// The sequential dense route, with the default first-use check of the
    /// dense route.
    pub fn dense() -> Self {
        Self {
            route: ColumnRoute::Dense,
            execution: AssemblyExecution::Sequential,
            first_use_check: Self::default_first_use_check(&ColumnRoute::Dense),
            fingerprints: false,
        }
    }

    /// The sequential coloured route of `pattern`, with the default
    /// first-use check of the coloured route, decided here from `pattern`
    /// ([`Self::default_first_use_check`]: the multi-colouring check, or the
    /// full check where the multi-colouring check would need `n` or more
    /// right-hand sides).
    pub fn coloured(pattern: SparsityPattern) -> Self {
        let route = ColumnRoute::Coloured(pattern);
        Self {
            first_use_check: Self::default_first_use_check(&route),
            route,
            ..Self::dense()
        }
    }

    /// The same route with the right-hand sides on `workers` threads.
    pub fn threaded(mut self, workers: usize, purity: RhsPurity) -> Self {
        self.execution = AssemblyExecution::Threaded { workers, purity };
        self
    }

    /// The same options with another first-use check.
    pub fn with_first_use_check(mut self, columns: CheckColumns) -> Self {
        self.first_use_check = columns;
        self
    }

    /// The same options with fingerprints.
    pub fn with_fingerprints(mut self) -> Self {
        self.fingerprints = true;
        self
    }
}

/// The start value of the fingerprints of [`AssemblyStats`].
pub const FINGERPRINT_SEED: u64 = 0xcbf2_9ce4_8422_2325;

/// `seed` continued over the bits of `values`: FNV-1a with 64-bit words.
pub fn fingerprint(seed: u64, values: &[f64]) -> u64 {
    values.iter().fold(seed, |hash, value| {
        (hash ^ value.to_bits()).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// What a client has done so far.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AssemblyStats {
    /// Completed builds.
    pub builds: u64,
    /// Builds that ended with an error of the right-hand side, a non-finite
    /// entry or a failed check.
    pub failed_builds: u64,
    /// Right-hand sides entered by completed builds, without their checks.
    pub assembly_rhs_calls: u64,
    /// Right-hand sides entered by failed builds, checks included.
    pub failed_build_rhs_calls: u64,
    /// Completed check passes.
    pub verifications: u64,
    /// Columns evaluated by completed check passes.
    pub verified_columns: u64,
    /// Right-hand sides entered by completed check passes (the columns and,
    /// with threaded execution, one base evaluation per pass).
    pub verification_rhs_calls: u64,
    /// With [`AssemblyOptions::fingerprints`]: [`fingerprint`] chained over
    /// `t` and `y` of every completed build.
    pub state_fingerprint: u64,
    /// With [`AssemblyOptions::fingerprints`]: [`fingerprint`] chained over
    /// every stored entry of every completed build, in the layout of the
    /// back end.
    pub jacobian_fingerprint: u64,
}

impl Default for AssemblyStats {
    fn default() -> Self {
        Self {
            builds: 0,
            failed_builds: 0,
            assembly_rhs_calls: 0,
            failed_build_rhs_calls: 0,
            verifications: 0,
            verified_columns: 0,
            verification_rhs_calls: 0,
            state_fingerprint: FINGERPRINT_SEED,
            jacobian_fingerprint: FINGERPRINT_SEED,
        }
    }
}

#[derive(Debug, Default)]
struct Report {
    stats: AssemblyStats,
    last_error: Option<AssemblyError>,
    requested: Option<CheckColumns>,
}

/// A reader of a client's statistics and last contract error that outlives
/// the client's move into the driver (which drops the client when a run
/// ends with `Err`), and the way to request a check.
#[derive(Clone, Debug)]
pub struct AssemblyReportHandle {
    shared: Arc<Mutex<Report>>,
}

impl AssemblyReportHandle {
    /// The statistics published so far.
    pub fn stats(&self) -> AssemblyStats {
        lock(&self.shared).stats
    }

    /// The last [`AssemblyError`] a linearization or a check ended with.
    pub fn last_error(&self) -> Option<AssemblyError> {
        lock(&self.shared).last_error.clone()
    }

    /// Check `columns` at the next linearization.
    pub fn request_check(&self, columns: CheckColumns) {
        lock(&self.shared).requested = Some(columns);
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The assembled Jacobian in the layout of the back end, with the state it
/// belongs to.
struct Served {
    values: Vec<f64>,
    t_bits: u64,
    y: Vec<f64>,
    valid: bool,
}

impl Served {
    /// Refuse a request for another state than the assembled one.
    fn check(&self, t: f64, y: &[f64]) -> CoreResult<()> {
        let same = self.valid
            && self.t_bits == t.to_bits()
            && self.y.len() == y.len()
            && self
                .y
                .iter()
                .zip(y)
                .all(|(a, b)| a.to_bits() == b.to_bits());
        if same {
            Ok(())
        } else {
            Err(CoreError::InvalidInput(
                AssemblyError::StaleRequest.to_string(),
            ))
        }
    }
}

/// A stage client that assembles the Jacobian by finite differences of the
/// right-hand side and delegates factorization and stage solves to a back
/// end of the kit. See the module documentation.
pub struct AssemblyStageClient {
    n: usize,
    engine: Engine,
    served: Arc<Mutex<Served>>,
    inner: Box<dyn StageLinearSystem>,
    /// Dense back ends read the Jacobian through this problem's explicit
    /// Jacobian callbacks.
    view: Option<OdeProblem>,
    report: Arc<Mutex<Report>>,
    first_use_check: CheckColumns,
    first_use_done: bool,
    fingerprints: bool,
    state_fingerprint: u64,
    jacobian_fingerprint: u64,
}

impl AssemblyStageClient {
    /// A client of order `n`. Refuses `n == 0`, the dense route on a banded
    /// back end, a pattern of another order or outside the band, a band
    /// outside the matrix, zero workers and the multi-colouring check as the
    /// first-use check of the dense route. With a multi-colouring first-use
    /// check (the default of the coloured route unless the declared pattern
    /// needs `n` or more of its right-hand sides,
    /// [`AssemblyOptions::default_first_use_check`]) the further colourings
    /// of the check are built here, so that the first linearization
    /// allocates nothing for them; the full check needs nothing built.
    pub fn new(
        n: usize,
        backend: LinearBackend,
        options: AssemblyOptions,
    ) -> Result<Self, AssemblyError> {
        if n == 0 {
            return Err(AssemblyError::Configuration(
                "the assembly client needs a positive order".into(),
            ));
        }
        let multi_colouring = matches!(
            options.first_use_check,
            CheckColumns::MultiColouring | CheckColumns::MultiColouringPooled
        );
        if multi_colouring && options.route == ColumnRoute::Dense {
            return Err(AssemblyError::Configuration(
                "the multi-colouring check needs the coloured route: the dense route declares no \
                 pattern"
                    .into(),
            ));
        }
        let pattern = match &options.route {
            ColumnRoute::Dense => None,
            ColumnRoute::Coloured(pattern) => {
                if pattern.n() != n {
                    return Err(AssemblyError::Configuration(format!(
                        "the pattern has order {}, the client {n}",
                        pattern.n()
                    )));
                }
                Some(pattern)
            }
        };
        let layout = match backend {
            LinearBackend::DenseLu(_) => TargetLayout::Dense,
            LinearBackend::BandedProtected { lower, upper }
            | LinearBackend::BandedTight { lower, upper } => {
                if pattern.is_none() {
                    return Err(AssemblyError::Configuration(
                        "the dense route needs the dense back end: a band cannot hold the \
                         columns of an undeclared Jacobian"
                            .into(),
                    ));
                }
                if lower >= n || upper >= n {
                    return Err(AssemblyError::Configuration(format!(
                        "the band ({lower}, {upper}) must lie inside the matrix of order {n}"
                    )));
                }
                TargetLayout::Band { lower, upper }
            }
        };
        let stride = layout.stride(n);
        let len = n
            .checked_mul(stride)
            .filter(|len| *len <= isize::MAX as usize / std::mem::size_of::<f64>());
        let Some(len) = len else {
            return Err(AssemblyError::Configuration(format!(
                "a Jacobian of order {n} with {stride} slots per row cannot be addressed"
            )));
        };
        let workers = match options.execution {
            AssemblyExecution::Sequential => None,
            AssemblyExecution::Threaded { workers, .. } => Some(workers),
        };
        let mut engine = Engine::new(n, layout, pattern, workers)?;
        if multi_colouring {
            engine.prepare_multi()?;
        }
        let served = Arc::new(Mutex::new(Served {
            values: vec![0.0; len],
            t_bits: 0,
            y: vec![0.0; n],
            valid: false,
        }));
        let configuration = |error: CoreError| AssemblyError::Configuration(error.to_string());
        let (inner, view): (Box<dyn StageLinearSystem>, Option<OdeProblem>) = match backend {
            LinearBackend::DenseLu(mode) => (
                Box::new(DenseLuStageClient::new(n, mode).map_err(configuration)?),
                Some(jacobian_view(n, &served).map_err(configuration)?),
            ),
            LinearBackend::BandedProtected { lower, upper } => {
                let band = served_band(lower, upper, &served);
                (
                    Box::new(BandedStageLu::new(n, &band).map_err(configuration)?),
                    None,
                )
            }
            LinearBackend::BandedTight { lower, upper } => {
                let band = served_band(lower, upper, &served);
                (
                    Box::new(BandedTightStageLu::new(n, &band).map_err(configuration)?),
                    None,
                )
            }
        };
        Ok(Self {
            n,
            engine,
            served,
            inner,
            view,
            report: Arc::new(Mutex::new(Report::default())),
            first_use_check: options.first_use_check,
            first_use_done: false,
            fingerprints: options.fingerprints,
            state_fingerprint: FINGERPRINT_SEED,
            jacobian_fingerprint: FINGERPRINT_SEED,
        })
    }

    /// The LU the protected dense driver picks for a run of `problem` from
    /// `(t0, y0)` when its Jacobian is the sequential dense finite
    /// difference ([`DenseLuMode::protected_choice`] of that matrix). The
    /// `n + 1` right-hand sides are charged to scratch counters.
    pub fn protected_dense_mode(
        problem: &OdeProblem,
        t0: f64,
        y0: &[f64],
    ) -> CoreResult<DenseLuMode> {
        let n = problem.dimension;
        let values = engine::dense_sequential_once(problem, t0, y0, &mut WorkCounters::default())?;
        Ok(DenseLuMode::protected_choice(&DenseMatrix::new(
            n, n, values,
        )?))
    }

    /// A reader of the statistics and the last contract error, and the way
    /// to request a check.
    pub fn handle(&self) -> AssemblyReportHandle {
        AssemblyReportHandle {
            shared: Arc::clone(&self.report),
        }
    }

    /// The colours of the coloured route (`None` on the dense route).
    pub fn colours(&self) -> Option<usize> {
        self.engine.colours()
    }

    /// Right-hand sides of one completed build without checks: `n + 1` on
    /// the dense route, `colours + 1` on the coloured route.
    pub fn rhs_per_build(&self) -> usize {
        self.engine.rhs_per_build()
    }

    /// Pool threads (`None` for sequential execution).
    pub fn workers(&self) -> Option<usize> {
        self.engine.workers()
    }

    /// A copy of the Jacobian of the last completed build in the layout of
    /// the back end (row-major `n x n`, or the band layout of
    /// [`BandedJacobianFn`]); `None` before the first completed build and
    /// after a failed one.
    pub fn jacobian_values(&self) -> Option<Vec<f64>> {
        let served = lock(&self.served);
        served.valid.then(|| served.values.clone())
    }

    /// Assemble at `(t, y)` and check `columns` now (see the module
    /// documentation), without a driver. Returns the number of columns
    /// checked. The right-hand sides are charged to `counters`.
    pub fn verify(
        &mut self,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        columns: CheckColumns,
        counters: &mut WorkCounters,
    ) -> Result<usize, AssemblyError> {
        if problem.dimension != self.n || y.len() != self.n {
            return Err(AssemblyError::Configuration(format!(
                "the client has order {}, the problem {} and the state {}",
                self.n,
                problem.dimension,
                y.len()
            )));
        }
        match self.assemble(problem, t, y, Some(columns), counters) {
            Ok(checked) => Ok(checked),
            Err(BuildFailure::Assembly(error)) => Err(error),
            Err(BuildFailure::Core(error)) => Err(AssemblyError::RightHandSide(error.to_string())),
        }
    }

    /// One build at `(t, y)` into the served storage, followed by the check
    /// of `check`, and the bookkeeping of both.
    fn assemble(
        &mut self,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        check: Option<CheckColumns>,
        counters: &mut WorkCounters,
    ) -> Result<usize, BuildFailure> {
        let mut tally = Tally::default();
        let outcome = {
            let mut served = lock(&self.served);
            let served = &mut *served;
            served.valid = false;
            let built = self
                .engine
                .build(problem, t, y, &mut served.values, counters, &mut tally);
            let checked = built.and_then(|()| match check {
                Some(columns) => {
                    self.engine
                        .verify(problem, t, y, &served.values, columns, counters, &mut tally)
                }
                None => Ok(0),
            });
            if checked.is_ok() {
                served.t_bits = t.to_bits();
                served.y.copy_from_slice(y);
                served.valid = true;
                if self.fingerprints {
                    self.state_fingerprint =
                        fingerprint(fingerprint(self.state_fingerprint, &[t]), y);
                    self.jacobian_fingerprint =
                        fingerprint(self.jacobian_fingerprint, &served.values);
                }
            }
            checked
        };
        let mut report = lock(&self.report);
        let report = &mut *report;
        match &outcome {
            Ok(checked) => {
                let stats = &mut report.stats;
                stats.builds += 1;
                stats.assembly_rhs_calls += tally.assembly;
                if check.is_some_and(|columns| columns != CheckColumns::None) {
                    stats.verifications += 1;
                    stats.verified_columns += *checked as u64;
                    stats.verification_rhs_calls += tally.verification;
                }
                stats.state_fingerprint = self.state_fingerprint;
                stats.jacobian_fingerprint = self.jacobian_fingerprint;
            }
            Err(failure) => {
                report.stats.failed_builds += 1;
                report.stats.failed_build_rhs_calls += tally.assembly + tally.verification;
                if let BuildFailure::Assembly(error) = failure {
                    report.last_error = Some(error.clone());
                }
            }
        }
        outcome
    }
}

/// A problem whose explicit Jacobian callbacks serve the assembled matrix
/// (row-major `n x n`) and nothing else: its right-hand side refuses.
fn jacobian_view(n: usize, served: &Arc<Mutex<Served>>) -> CoreResult<OdeProblem> {
    let for_matrix = Arc::clone(served);
    let for_into = Arc::clone(served);
    let view = OdeProblem::new(
        "assembled-jacobian-view",
        n,
        Arc::new(|_t: f64, _y: &[f64], _out: &mut [f64]| -> CoreResult<()> {
            Err(CoreError::InvalidInput(
                "the assembled-Jacobian view has no right-hand side".into(),
            ))
        }),
        None,
        Some(Arc::new(
            move |t: f64, y: &[f64]| -> CoreResult<DenseMatrix> {
                let served = lock(&for_matrix);
                served.check(t, y)?;
                DenseMatrix::new(n, n, served.values.clone())
            },
        )),
        None,
        None,
        true,
        None,
        None,
    )?;
    Ok(view.with_jacobian_into(Arc::new(
        move |t: f64, y: &[f64], out: &mut DenseMatrix| -> CoreResult<()> {
            let served = lock(&for_into);
            served.check(t, y)?;
            out.as_mut_slice().copy_from_slice(&served.values);
            Ok(())
        },
    )))
}

/// A band provider that serves the assembled band.
fn served_band(lower: usize, upper: usize, served: &Arc<Mutex<Served>>) -> BandedJacobian {
    let served = Arc::clone(served);
    let fill: BandedJacobianFn = Arc::new(
        move |t: f64, y: &[f64], band: &mut [f64]| -> CoreResult<()> {
            let served = lock(&served);
            served.check(t, y)?;
            if band.len() != served.values.len() {
                return Err(CoreError::Dimension("assembled band shape mismatch".into()));
            }
            band.copy_from_slice(&served.values);
            Ok(())
        },
    );
    BandedJacobian { lower, upper, fill }
}

impl StageLinearSystem for AssemblyStageClient {
    fn dimension(&self) -> usize {
        self.n
    }

    /// The finite-difference Jacobian at `(t, y)` from the right-hand side
    /// of `problem`, the pending check if there is one, and the
    /// linearization of the back end from the assembled values. A failed
    /// check ends the run: the driver receives [`CoreError::InvalidInput`]
    /// and the handle keeps the [`AssemblyError`]. An error of the
    /// right-hand side or a non-finite entry is returned unchanged, as the
    /// sequential finite differences return it.
    fn linearize(
        &mut self,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        if problem.dimension != self.n || y.len() != self.n {
            return Err(CoreError::Dimension(
                "assembly client state shape mismatch".into(),
            ));
        }
        let requested = lock(&self.report).requested.take();
        let first_use = (!self.first_use_done
            && (self.engine.colours().is_some() || self.engine.workers().is_some()))
        .then_some(self.first_use_check);
        let check = requested.or(first_use);
        match self.assemble(problem, t, y, check, counters) {
            Ok(_) => {}
            Err(BuildFailure::Core(error)) => return Err(error),
            Err(BuildFailure::Assembly(error)) => {
                return Err(CoreError::InvalidInput(error.to_string()));
            }
        }
        if check.is_some() {
            self.first_use_done = true;
        }
        match &self.view {
            Some(view) => self.inner.linearize(view, t, y, counters),
            None => self.inner.linearize(problem, t, y, counters),
        }
    }

    fn factor(&mut self, inv_h_gamma: f64, counters: &mut WorkCounters) -> CoreResult<()> {
        self.inner.factor(inv_h_gamma, counters)
    }

    fn solve_in_place(&mut self, b: &mut [f64], counters: &mut WorkCounters) -> CoreResult<()> {
        self.inner.solve_in_place(b, counters)
    }
}
