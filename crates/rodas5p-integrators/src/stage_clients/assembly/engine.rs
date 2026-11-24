//! The finite-difference assembly engine of [`super::AssemblyStageClient`]
//! (loop node V08): the column loops of the facade's finite differences
//! (`crates/vigilode/src/fd.rs`), sequential or with the right-hand sides on
//! a thread pool owned by the engine.
//!
//! Every entry is `(f(t, y + shift)[i] - f(t, y)[i]) / delta_j`, the
//! expression of `fd.rs:499` (dense) and `fd.rs:611` (grouped), with the
//! increment of `fd.rs:443-455`. An entry depends on two right-hand-side
//! vectors and one increment only, so where and on which thread it is
//! computed does not change its bits as long as the right-hand side returns
//! the same bits for the same input.

use std::sync::atomic::{AtomicUsize, Ordering};

use rayon::prelude::*;
use rayon::{ThreadPool, ThreadPoolBuilder};
use rodas5p_core::{CoreError, CoreResult, WorkCounters};

use super::multi::CheckColourings;
use super::pattern::{SparsityPattern, colour_columns};
use super::{AssemblyError, CheckColumns};
use crate::OdeProblem;

/// Bytes of state and right-hand-side vectors the threaded engine may hold
/// (two vectors per task of a build).
const THREADED_SLOT_BYTES_MAX: usize = 1 << 30;

/// Why a build or a verification stopped.
#[derive(Debug)]
pub(super) enum BuildFailure {
    /// An error of the right-hand side or a non-finite entry, returned to
    /// the driver unchanged (the sequential finite differences of the
    /// facade return the same kinds).
    Core(CoreError),
    /// The assembly contract is violated; the run must end.
    Assembly(AssemblyError),
}

impl From<CoreError> for BuildFailure {
    fn from(error: CoreError) -> Self {
        Self::Core(error)
    }
}

/// Right-hand sides entered by the engine.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Tally {
    /// Entered by an assembly (the base evaluation and one per task).
    pub(super) assembly: u64,
    /// Entered by a verification pass.
    pub(super) verification: u64,
}

fn overflowed() -> BuildFailure {
    BuildFailure::Core(CoreError::NonFinite(
        "finite-difference Jacobian entry overflowed".into(),
    ))
}

/// The forward-difference increment of component `y_j`: the shifted value
/// and the represented difference `(y_j + sqrt(eps) max(|y_j|, 1)) - y_j`,
/// backward where the forward shift overflows. Transcribed from
/// `crates/vigilode/src/fd.rs:443-455`.
#[inline]
fn increment(yj: f64) -> (f64, f64) {
    let raw = f64::EPSILON.sqrt() * yj.abs().max(1.0);
    let forward = yj + raw;
    let shifted = if forward.is_finite() {
        forward
    } else {
        yj - raw
    };
    (shifted, shifted - yj)
}

/// The storage the back end reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TargetLayout {
    /// Row-major `n x n`.
    Dense,
    /// The band layout of [`crate::BandedJacobianFn`]: row `i`, column `j`
    /// at `i (lower + upper + 1) + (j + lower - i)`.
    Band { lower: usize, upper: usize },
}

impl TargetLayout {
    /// Stored slots per row.
    pub(super) fn stride(self, n: usize) -> usize {
        match self {
            Self::Dense => n,
            Self::Band { lower, upper } => lower + upper + 1,
        }
    }

    /// The slot of `(i, j)` inside row `i`, if the layout stores it.
    fn local(self, i: usize, j: usize) -> Option<usize> {
        match self {
            Self::Dense => Some(j),
            Self::Band { lower, upper } => {
                (j + lower >= i && j <= i + upper).then(|| j + lower - i)
            }
        }
    }
}

/// One declared entry of a column group.
#[derive(Clone, Copy, Debug)]
struct Entry {
    row: u32,
    col: u32,
    /// Index into the target.
    dest: u32,
}

/// Where the gather finds one stored slot of a row.
#[derive(Clone, Copy, Debug, Default)]
struct RowSource {
    group: u32,
    /// Position among the group's entries.
    pos: u32,
    /// Slot inside the row.
    local: u32,
}

/// One colour: its columns, their declared entries by column and then by
/// row (the order of `fd.rs:609-610`), and the rows those entries cover,
/// increasing. No two columns of a colour share a row, so the rows are
/// distinct.
struct Group {
    columns: Vec<u32>,
    entries: Vec<Entry>,
    covered: Vec<u32>,
}

/// The column groups of a declared pattern.
struct Coloured {
    groups: Vec<Group>,
    /// The entries of row `i` are `row_src[row_ptr[i]..row_ptr[i + 1]]`.
    row_ptr: Vec<usize>,
    row_src: Vec<RowSource>,
    /// Column `j`: its group and the range of its entries in that group.
    col_group: Vec<u32>,
    col_start: Vec<u32>,
    col_len: Vec<u32>,
}

impl Coloured {
    fn new(pattern: &SparsityPattern, layout: TargetLayout) -> Result<Self, AssemblyError> {
        let n = pattern.n();
        let stride = layout.stride(n);
        let addressable = n
            .checked_mul(stride)
            .is_some_and(|len| u32::try_from(len).is_ok());
        if !addressable {
            return Err(AssemblyError::Configuration(format!(
                "the coloured route addresses its target with 32 bits; order {n} with {stride} \
                 slots per row is too large"
            )));
        }
        let (colour, count) = colour_columns(pattern);
        let columns = pattern.by_column();
        let mut groups: Vec<Group> = (0..count)
            .map(|_| Group {
                columns: Vec::new(),
                entries: Vec::new(),
                covered: Vec::new(),
            })
            .collect();
        let mut row_ptr = Vec::with_capacity(n + 1);
        row_ptr.push(0);
        for i in 0..n {
            row_ptr.push(row_ptr[i] + pattern.row(i).len());
        }
        let mut row_src = vec![RowSource::default(); pattern.nnz()];
        let mut col_start = vec![0_u32; n];
        let mut col_len = vec![0_u32; n];
        for j in 0..n {
            let group = &mut groups[colour[j]];
            group.columns.push(j as u32);
            col_start[j] = group.entries.len() as u32;
            col_len[j] = columns.rows_of(j).len() as u32;
            for &i in columns.rows_of(j) {
                let Some(local) = layout.local(i, j) else {
                    return Err(AssemblyError::Configuration(format!(
                        "pattern entry ({i}, {j}) lies outside the storage of the back end"
                    )));
                };
                let slot = pattern
                    .row(i)
                    .binary_search(&j)
                    .expect("the column view lists declared entries");
                row_src[row_ptr[i] + slot] = RowSource {
                    group: colour[j] as u32,
                    pos: group.entries.len() as u32,
                    local: local as u32,
                };
                group.entries.push(Entry {
                    row: i as u32,
                    col: j as u32,
                    dest: (i * stride + local) as u32,
                });
            }
        }
        for group in &mut groups {
            group.covered = group.entries.iter().map(|entry| entry.row).collect();
            group.covered.sort_unstable();
        }
        Ok(Self {
            groups,
            row_ptr,
            row_src,
            col_group: colour.iter().map(|&c| c as u32).collect(),
            col_start,
            col_len,
        })
    }

    /// The declared entries of column `j`, rows increasing.
    fn column_entries(&self, j: usize) -> &[Entry] {
        let start = self.col_start[j] as usize;
        &self.groups[self.col_group[j] as usize].entries[start..start + self.col_len[j] as usize]
    }
}

/// Which right-hand sides a build evaluates.
enum Route {
    /// One per column.
    Dense,
    /// One per colour of a declared pattern.
    Coloured(Coloured),
}

impl Route {
    /// Right-hand sides of a build besides the base evaluation.
    fn tasks(&self, n: usize) -> usize {
        match self {
            Self::Dense => n,
            Self::Coloured(table) => table.groups.len(),
        }
    }

    /// Shift the columns of task `k` in `state`, a copy of `y`.
    fn shift(&self, k: usize, shifted_values: &[f64], state: &mut [f64]) {
        match self {
            Self::Dense => state[k] = shifted_values[k],
            Self::Coloured(table) => {
                for &j in &table.groups[k].columns {
                    state[j as usize] = shifted_values[j as usize];
                }
            }
        }
    }
}

/// One right-hand side of a threaded build with everything it writes: slot
/// 0 is the base evaluation, slot `k + 1` task `k`. A slot is touched by
/// one thread at a time and shares nothing mutable with another slot.
struct Slot {
    /// `y` with the columns of the task shifted.
    state: Vec<f64>,
    out: Vec<f64>,
    /// Coloured route: the entries of the group, in the group's order.
    vals: Vec<f64>,
    counters: WorkCounters,
    failure: Option<BuildFailure>,
}

struct Threaded {
    pool: ThreadPool,
    slots: Vec<Slot>,
}

/// The finite-difference Jacobian of one problem dimension and one target
/// layout. Everything is allocated in [`Engine::new`]; the sequential
/// builds allocate nothing.
pub(super) struct Engine {
    n: usize,
    stride: usize,
    route: Route,
    /// Sequential scratch (and the verification pass of every mode).
    f0: Vec<f64>,
    f1: Vec<f64>,
    shifted: Vec<f64>,
    /// The increment and the shifted value of every column of this build.
    deltas: Vec<f64>,
    shifted_values: Vec<f64>,
    threaded: Option<Threaded>,
    workers: Option<usize>,
    /// The further colourings of the multi-colouring check, built at its
    /// first use.
    multi: Option<CheckColourings>,
    /// Row marks of the multi-colouring check on the calling thread.
    mark: Vec<usize>,
    /// One scratch per pool thread for the pooled checks, allocated at
    /// their first use.
    scratch: Vec<CheckScratch>,
}

/// What a check evaluates.
#[derive(Clone, Copy, Debug)]
enum CheckKind {
    /// `count` single columns (every column, or evenly spread ones).
    Columns { count: usize, pooled: bool },
    /// Every group of every further colouring.
    Multi { pooled: bool },
}

/// What one pool thread of a pooled check owns.
struct CheckScratch {
    /// `y`, with the columns of the current item shifted.
    state: Vec<f64>,
    out: Vec<f64>,
    /// Multi-colouring check: the item that last covered each row.
    mark: Vec<usize>,
    counters: WorkCounters,
    /// Right-hand sides entered.
    evaluations: u64,
    /// The first failing item this thread met, with its failure.
    failure: Option<(usize, BuildFailure)>,
}

impl CheckScratch {
    fn new(n: usize) -> Self {
        Self {
            state: vec![0.0; n],
            out: vec![0.0; n],
            mark: vec![0; n],
            counters: WorkCounters::default(),
            evaluations: 0,
            failure: None,
        }
    }
}

impl Engine {
    /// An engine of order `n` writing `layout`: the dense route without a
    /// pattern, the coloured route with one; sequential without `workers`,
    /// otherwise with a pool of that many threads.
    pub(super) fn new(
        n: usize,
        layout: TargetLayout,
        pattern: Option<&SparsityPattern>,
        workers: Option<usize>,
    ) -> Result<Self, AssemblyError> {
        let stride = layout.stride(n);
        let route = match pattern {
            None => Route::Dense,
            Some(pattern) => Route::Coloured(Coloured::new(pattern, layout)?),
        };
        let threaded = match workers {
            None => None,
            Some(0) => {
                return Err(AssemblyError::Configuration(
                    "threaded assembly needs at least one worker".into(),
                ));
            }
            Some(workers) => {
                let tasks = route.tasks(n);
                let bytes = (tasks + 1)
                    .checked_mul(n)
                    .and_then(|len| len.checked_mul(2 * std::mem::size_of::<f64>()));
                if bytes.is_none_or(|bytes| bytes > THREADED_SLOT_BYTES_MAX) {
                    return Err(AssemblyError::Configuration(format!(
                        "threaded assembly keeps a state and a right-hand-side vector per task: \
                         {} tasks of order {n} exceed {THREADED_SLOT_BYTES_MAX} bytes",
                        tasks + 1
                    )));
                }
                let pool = ThreadPoolBuilder::new()
                    .num_threads(workers)
                    .thread_name(|index| format!("rodas5p-fd-assembly-{index}"))
                    .build()
                    .map_err(|error| {
                        AssemblyError::Configuration(format!(
                            "the assembly thread pool could not be created: {error}"
                        ))
                    })?;
                let slot = |k: usize| Slot {
                    state: vec![0.0; if k == 0 { 0 } else { n }],
                    out: vec![0.0; n],
                    vals: match &route {
                        Route::Coloured(table) if k > 0 => {
                            vec![0.0; table.groups[k - 1].entries.len()]
                        }
                        _ => Vec::new(),
                    },
                    counters: WorkCounters::default(),
                    failure: None,
                };
                Some(Threaded {
                    pool,
                    slots: (0..=tasks).map(slot).collect(),
                })
            }
        };
        Ok(Self {
            n,
            stride,
            f0: vec![0.0; n],
            f1: vec![0.0; n],
            shifted: vec![0.0; n],
            deltas: vec![0.0; n],
            shifted_values: vec![0.0; n],
            route,
            threaded,
            workers,
            multi: None,
            mark: Vec::new(),
            scratch: Vec::new(),
        })
    }

    /// The colours of the coloured route.
    pub(super) fn colours(&self) -> Option<usize> {
        match &self.route {
            Route::Dense => None,
            Route::Coloured(table) => Some(table.groups.len()),
        }
    }

    /// Right-hand sides of one completed build: the base evaluation and one
    /// per column (dense route) or per colour (coloured route).
    pub(super) fn rhs_per_build(&self) -> usize {
        self.route.tasks(self.n) + 1
    }

    /// Pool threads (`None` for sequential execution).
    pub(super) fn workers(&self) -> Option<usize> {
        self.workers
    }

    /// The Jacobian of `problem` at `(t, y)` into `target`. `Err` leaves
    /// `target` unspecified.
    pub(super) fn build(
        &mut self,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        target: &mut [f64],
        counters: &mut WorkCounters,
        tally: &mut Tally,
    ) -> Result<(), BuildFailure> {
        debug_assert_eq!(target.len(), self.n * self.stride);
        if self.threaded.is_some() {
            self.build_threaded(problem, t, y, target, counters, tally)
        } else if matches!(self.route, Route::Dense) {
            self.build_dense_sequential(problem, t, y, target, counters, tally)
        } else {
            self.build_coloured_sequential(problem, t, y, target, counters, tally)
        }
    }

    /// The dense forward differences of `crates/vigilode/src/fd.rs:476-509`,
    /// statement for statement: zero the matrix (`:486`), the base
    /// evaluation into a zeroed buffer (`:488-489`), a copy of `y` (`:490`),
    /// and per column the shift (`:492-493`), the evaluation into a zeroed
    /// buffer (`:494-495`), the restore before the error is looked at
    /// (`:496-497`) and the entries of the column, rows ascending, each
    /// checked finite before it is stored (`:498-506`).
    fn build_dense_sequential(
        &mut self,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        target: &mut [f64],
        counters: &mut WorkCounters,
        tally: &mut Tally,
    ) -> Result<(), BuildFailure> {
        let n = self.n;
        target.fill(0.0);
        self.f0.fill(0.0);
        tally.assembly += 1;
        problem.eval_rhs_into(t, y, &mut self.f0, counters)?;
        self.shifted.copy_from_slice(y);
        for j in 0..n {
            let (yj_shifted, delta) = increment(y[j]);
            self.shifted[j] = yj_shifted;
            self.f1.fill(0.0);
            tally.assembly += 1;
            let outcome = problem.eval_rhs_into(t, &self.shifted, &mut self.f1, counters);
            self.shifted[j] = y[j];
            outcome?;
            for i in 0..n {
                let entry = (self.f1[i] - self.f0[i]) / delta;
                if !entry.is_finite() {
                    return Err(overflowed());
                }
                target[i * n + j] = entry;
            }
        }
        Ok(())
    }

    /// The grouped forward differences of `crates/vigilode/src/fd.rs:581-622`
    /// for the groups of a declared pattern: the base evaluation
    /// (`:594-595`), and per group the shifts and increments of its columns
    /// (`:598-602`), one evaluation (`:603-604`), the restore (`:605-608`)
    /// and the declared entries by column and row (`:609-619`). After the
    /// entries of a group, every row that no column of the group declares
    /// must be bitwise unchanged: a changed row is an undeclared dependence.
    fn build_coloured_sequential(
        &mut self,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        target: &mut [f64],
        counters: &mut WorkCounters,
        tally: &mut Tally,
    ) -> Result<(), BuildFailure> {
        let Route::Coloured(table) = &self.route else {
            unreachable!("the coloured build runs on the coloured route");
        };
        self.f0.fill(0.0);
        tally.assembly += 1;
        problem.eval_rhs_into(t, y, &mut self.f0, counters)?;
        self.shifted.copy_from_slice(y);
        for (colour, group) in table.groups.iter().enumerate() {
            for &j in &group.columns {
                let j = j as usize;
                let (yj_shifted, delta) = increment(y[j]);
                self.shifted[j] = yj_shifted;
                self.deltas[j] = delta;
            }
            self.f1.fill(0.0);
            tally.assembly += 1;
            let outcome = problem.eval_rhs_into(t, &self.shifted, &mut self.f1, counters);
            for &j in &group.columns {
                self.shifted[j as usize] = y[j as usize];
            }
            outcome?;
            for entry in &group.entries {
                let i = entry.row as usize;
                let value = (self.f1[i] - self.f0[i]) / self.deltas[entry.col as usize];
                if !value.is_finite() {
                    return Err(overflowed());
                }
                target[entry.dest as usize] = value;
            }
            uncovered_rows_unchanged(&group.covered, &self.f1, &self.f0, colour)?;
        }
        Ok(())
    }

    /// The same Jacobian with the right-hand sides on the pool. Three
    /// parallel passes inside one `install`: every right-hand side into its
    /// own slot, from the slot's own copy of the state; the entries of every
    /// task from its slot and the base slot; the target by rows. No lock is
    /// held while a right-hand side runs. Failures are reported in task
    /// order, base first, which is the order in which the sequential build
    /// meets them; unlike the sequential build, every right-hand side of the
    /// build has been entered by then.
    fn build_threaded(
        &mut self,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        target: &mut [f64],
        counters: &mut WorkCounters,
        tally: &mut Tally,
    ) -> Result<(), BuildFailure> {
        let stride = self.stride;
        for (j, &yj) in y.iter().enumerate() {
            let (yj_shifted, delta) = increment(yj);
            self.shifted_values[j] = yj_shifted;
            self.deltas[j] = delta;
        }
        let route = &self.route;
        let deltas = &self.deltas[..];
        let shifted_values = &self.shifted_values[..];
        let Threaded { pool, slots } = self.threaded.as_mut().expect("threaded build");
        tally.assembly += slots.len() as u64;
        pool.install(|| {
            slots
                .par_iter_mut()
                .with_max_len(1)
                .enumerate()
                .for_each(|(slot_index, slot)| {
                    slot.out.fill(0.0);
                    let outcome = if slot_index == 0 {
                        problem.eval_rhs_into(t, y, &mut slot.out, &mut slot.counters)
                    } else {
                        slot.state.copy_from_slice(y);
                        route.shift(slot_index - 1, shifted_values, &mut slot.state);
                        problem.eval_rhs_into(t, &slot.state, &mut slot.out, &mut slot.counters)
                    };
                    slot.failure = outcome.err().map(BuildFailure::Core);
                });
            let (base, tasks) = slots.split_first_mut().expect("the base slot");
            if base.failure.is_some() {
                return;
            }
            let f0 = &base.out[..];
            tasks
                .par_iter_mut()
                .with_max_len(1)
                .enumerate()
                .for_each(|(task, slot)| {
                    if slot.failure.is_some() {
                        return;
                    }
                    let extracted = match route {
                        Route::Dense => extract_column(f0, deltas[task], &mut slot.out),
                        Route::Coloured(table) => extract_group(
                            &table.groups[task],
                            task,
                            f0,
                            deltas,
                            &slot.out,
                            &mut slot.vals,
                        ),
                    };
                    slot.failure = extracted.err();
                });
            if tasks.iter().any(|slot| slot.failure.is_some()) {
                return;
            }
            let tasks = &tasks[..];
            match route {
                Route::Dense => {
                    target
                        .par_chunks_mut(stride)
                        .enumerate()
                        .for_each(|(i, row)| {
                            for (entry, slot) in row.iter_mut().zip(tasks) {
                                *entry = slot.out[i];
                            }
                        });
                }
                Route::Coloured(table) => {
                    target
                        .par_chunks_mut(stride)
                        .enumerate()
                        .for_each(|(i, row)| {
                            for source in &table.row_src[table.row_ptr[i]..table.row_ptr[i + 1]] {
                                row[source.local as usize] =
                                    tasks[source.group as usize].vals[source.pos as usize];
                            }
                        });
                }
            }
        });
        let mut first_failure = None;
        for slot in slots.iter_mut() {
            counters.accumulate(slot.counters);
            slot.counters = WorkCounters::default();
            let failure = slot.failure.take();
            if first_failure.is_none() {
                first_failure = failure;
            }
        }
        match first_failure {
            Some(failure) => Err(failure),
            None => Ok(()),
        }
    }

    /// Compare `target`, the Jacobian this engine has just built at
    /// `(t, y)`, with further evaluations of the right-hand side. Returns
    /// the number of columns the check covers.
    ///
    /// Single columns ([`CheckColumns::Sample`], [`CheckColumns::All`],
    /// [`CheckColumns::AllPooled`]): each checked column is shifted alone.
    /// Dense route: every entry of a checked column must have the bits of
    /// the assembled entry. Coloured route: every declared entry of the
    /// column must have the bits of the assembled entry (an entry assembled
    /// from a group can differ only if another column of the group reaches
    /// the row, or if the right-hand side is not a function of its input),
    /// and every other row must be bitwise unchanged by the shift of the
    /// column. The first failure in (column, row) order is returned.
    ///
    /// Multi-colouring ([`CheckColumns::MultiColouring`],
    /// [`CheckColumns::MultiColouringPooled`], coloured route only): the
    /// columns of every group of every further colouring
    /// ([`CheckColourings`]) are shifted at once; every declared entry of
    /// those columns must have the bits of the assembled entry, and every row
    /// that no column of the group declares must be bitwise unchanged. The
    /// first failure in (colouring, group, column, row) order is returned.
    ///
    /// The pooled options hand the columns or the groups to the threads of
    /// the client's pool (a sequential engine has no pool and runs them on
    /// the calling thread) and return the failure the order above meets
    /// first: they decide as the checks on the calling thread. Threaded
    /// execution: the base evaluation is repeated on the calling thread
    /// first and must have the bits the pool produced.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn verify(
        &mut self,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        target: &[f64],
        columns: CheckColumns,
        counters: &mut WorkCounters,
        tally: &mut Tally,
    ) -> Result<usize, BuildFailure> {
        let n = self.n;
        let kind = match columns {
            CheckColumns::None => return Ok(0),
            CheckColumns::Sample(count) => match count.min(n) {
                0 => return Ok(0),
                count => CheckKind::Columns {
                    count,
                    pooled: false,
                },
            },
            CheckColumns::All => CheckKind::Columns {
                count: n,
                pooled: false,
            },
            CheckColumns::AllPooled => CheckKind::Columns {
                count: n,
                pooled: true,
            },
            CheckColumns::MultiColouring => CheckKind::Multi { pooled: false },
            CheckColumns::MultiColouringPooled => CheckKind::Multi { pooled: true },
        };
        if matches!(kind, CheckKind::Multi { .. }) {
            self.prepare_multi().map_err(BuildFailure::Assembly)?;
        }
        let Self {
            route,
            f0,
            f1,
            shifted,
            deltas,
            shifted_values,
            threaded,
            multi,
            mark,
            scratch,
            ..
        } = self;
        let route = &*route;
        let threaded = threaded.as_ref();
        let base: &[f64] = match threaded {
            Some(threaded) => {
                f1.fill(0.0);
                tally.verification += 1;
                problem.eval_rhs_into(t, y, f1, counters)?;
                let pooled = &threaded.slots[0].out[..];
                if let Some(row) = (0..n).find(|&i| f1[i].to_bits() != pooled[i].to_bits()) {
                    return Err(BuildFailure::Assembly(AssemblyError::NotReproducible {
                        row,
                        column: None,
                    }));
                }
                pooled
            }
            None => &f0[..],
        };
        match kind {
            CheckKind::Columns { count, pooled } => {
                let check = ColumnCheck {
                    route,
                    n,
                    count,
                    base,
                    target,
                    y,
                    problem,
                    t,
                };
                match threaded.filter(|_| pooled) {
                    Some(threaded) => check.pooled(&threaded.pool, scratch, counters, tally)?,
                    None => check.sequential(shifted, f1, counters, tally)?,
                }
                Ok(count)
            }
            CheckKind::Multi { pooled } => {
                let Route::Coloured(table) = route else {
                    unreachable!("prepare_multi refuses the dense route");
                };
                let plan = multi.as_ref().expect("prepared above");
                for (j, &yj) in y.iter().enumerate() {
                    let (yj_shifted, delta) = increment(yj);
                    shifted_values[j] = yj_shifted;
                    deltas[j] = delta;
                }
                let check = GroupCheck {
                    table,
                    plan,
                    base,
                    target,
                    y,
                    shifted_values,
                    deltas,
                    problem,
                    t,
                };
                match threaded.filter(|_| pooled) {
                    Some(threaded) => check.pooled(&threaded.pool, scratch, counters, tally)?,
                    None => check.sequential(shifted, f1, mark, counters, tally)?,
                }
                Ok(n)
            }
        }
    }

    /// The further colourings of the multi-colouring check, built at its
    /// first use from the client's colouring and the band of the declared
    /// entries. The dense route declares no pattern and is refused.
    fn prepare_multi(&mut self) -> Result<(), AssemblyError> {
        if self.multi.is_some() {
            return Ok(());
        }
        let Route::Coloured(table) = &self.route else {
            return Err(AssemblyError::Configuration(
                "the multi-colouring check needs the coloured route: the dense route declares \
                 no pattern"
                    .into(),
            ));
        };
        let (mut lower, mut upper) = (0, 0);
        for entry in table.groups.iter().flat_map(|group| &group.entries) {
            let (i, j) = (entry.row as usize, entry.col as usize);
            lower = lower.max(i.saturating_sub(j));
            upper = upper.max(j.saturating_sub(i));
        }
        self.multi = Some(CheckColourings::from_parts(
            self.n,
            &table.col_group,
            lower,
            upper,
        )?);
        self.mark = vec![0; self.n];
        Ok(())
    }
}

/// What every column of a single-column check reads.
struct ColumnCheck<'a> {
    route: &'a Route,
    n: usize,
    count: usize,
    base: &'a [f64],
    target: &'a [f64],
    y: &'a [f64],
    problem: &'a OdeProblem,
    t: f64,
}

impl ColumnCheck<'_> {
    /// The column of sample `sample`: every column in order, or column
    /// `floor((2 sample + 1) n / (2 count))`.
    fn column(&self, sample: usize) -> usize {
        if self.count == self.n {
            sample
        } else {
            (2 * sample + 1) * self.n / (2 * self.count)
        }
    }

    /// Column `j` alone: shifted in `state` (a copy of `y`, restored before
    /// the outcome is looked at), one right-hand side into `out`, then the
    /// comparisons in row order.
    fn check(
        &self,
        j: usize,
        state: &mut [f64],
        out: &mut [f64],
        counters: &mut WorkCounters,
    ) -> Result<(), BuildFailure> {
        let (n, base, target, y) = (self.n, self.base, self.target, self.y);
        let (yj_shifted, delta) = increment(y[j]);
        state[j] = yj_shifted;
        out.fill(0.0);
        let outcome = self.problem.eval_rhs_into(self.t, state, out, counters);
        state[j] = y[j];
        outcome?;
        match self.route {
            Route::Dense => {
                for i in 0..n {
                    let single = (out[i] - base[i]) / delta;
                    if single.to_bits() != target[i * n + j].to_bits() {
                        return Err(BuildFailure::Assembly(AssemblyError::NotReproducible {
                            row: i,
                            column: Some(j),
                        }));
                    }
                }
            }
            Route::Coloured(table) => {
                let mut declared = table.column_entries(j).iter().peekable();
                for i in 0..n {
                    if let Some(entry) = declared.next_if(|entry| entry.row as usize == i) {
                        let single = (out[i] - base[i]) / delta;
                        let assembled = target[entry.dest as usize];
                        if single.to_bits() != assembled.to_bits() {
                            return Err(BuildFailure::Assembly(AssemblyError::AliasedEntry {
                                row: i,
                                column: j,
                                assembled,
                                single_column: single,
                            }));
                        }
                    } else if out[i].to_bits() != base[i].to_bits() {
                        return Err(BuildFailure::Assembly(AssemblyError::UndeclaredEntry {
                            row: i,
                            column: j,
                        }));
                    }
                }
            }
        }
        Ok(())
    }

    /// Every sample in order on the calling thread; the first failure ends
    /// the check.
    fn sequential(
        &self,
        state: &mut [f64],
        out: &mut [f64],
        counters: &mut WorkCounters,
        tally: &mut Tally,
    ) -> Result<(), BuildFailure> {
        state.copy_from_slice(self.y);
        for sample in 0..self.count {
            tally.verification += 1;
            self.check(self.column(sample), state, out, counters)?;
        }
        Ok(())
    }

    /// The samples on the pool ([`run_pooled`]).
    fn pooled(
        &self,
        pool: &ThreadPool,
        scratch: &mut Vec<CheckScratch>,
        counters: &mut WorkCounters,
        tally: &mut Tally,
    ) -> Result<(), BuildFailure> {
        let items = Items {
            n: self.n,
            y: self.y,
            count: self.count,
        };
        run_pooled(pool, scratch, items, counters, tally, |sample, worker| {
            self.check(
                self.column(sample),
                &mut worker.state,
                &mut worker.out,
                &mut worker.counters,
            )
        })
    }
}

/// What every group of the multi-colouring check reads.
struct GroupCheck<'a> {
    table: &'a Coloured,
    plan: &'a CheckColourings,
    base: &'a [f64],
    target: &'a [f64],
    y: &'a [f64],
    shifted_values: &'a [f64],
    deltas: &'a [f64],
    problem: &'a OdeProblem,
    t: f64,
}

impl GroupCheck<'_> {
    /// Item `item` of the check order (group `g` of further colouring `r`):
    /// the group's columns shifted at once in `state` (a copy of `y`,
    /// restored before the outcome is looked at), one right-hand side into
    /// `out`, every declared entry of the group's columns (columns, then
    /// rows, increasing) against the assembled entry, then every row the
    /// group does not cover against the base evaluation. `mark` keeps, per
    /// row, the last item that covered it.
    fn check(
        &self,
        item: usize,
        state: &mut [f64],
        out: &mut [f64],
        mark: &mut [usize],
        counters: &mut WorkCounters,
    ) -> Result<(), BuildFailure> {
        let (r, g) = self.plan.locate(item);
        let stamp = item + 1;
        for j in self.plan.group_columns(r, g) {
            state[j] = self.shifted_values[j];
        }
        out.fill(0.0);
        let outcome = self.problem.eval_rhs_into(self.t, state, out, counters);
        for j in self.plan.group_columns(r, g) {
            state[j] = self.y[j];
        }
        outcome?;
        for j in self.plan.group_columns(r, g) {
            for entry in self.table.column_entries(j) {
                let i = entry.row as usize;
                mark[i] = stamp;
                let recomputed = (out[i] - self.base[i]) / self.deltas[j];
                let assembled = self.target[entry.dest as usize];
                if recomputed.to_bits() != assembled.to_bits() {
                    return Err(BuildFailure::Assembly(
                        AssemblyError::MultiColouringEntryDiffers {
                            row: i,
                            column: j,
                            colouring: r + 1,
                            group: g,
                            assembled,
                            recomputed,
                        },
                    ));
                }
            }
        }
        for (i, (value, base)) in out.iter().zip(self.base).enumerate() {
            if mark[i] != stamp && value.to_bits() != base.to_bits() {
                return Err(BuildFailure::Assembly(
                    AssemblyError::MultiColouringUncoveredRow {
                        row: i,
                        colouring: r + 1,
                        group: g,
                    },
                ));
            }
        }
        Ok(())
    }

    /// Every group in order on the calling thread; the first failure ends
    /// the check.
    fn sequential(
        &self,
        state: &mut [f64],
        out: &mut [f64],
        mark: &mut [usize],
        counters: &mut WorkCounters,
        tally: &mut Tally,
    ) -> Result<(), BuildFailure> {
        state.copy_from_slice(self.y);
        mark.fill(0);
        for item in 0..self.plan.total_groups() {
            tally.verification += 1;
            self.check(item, state, out, mark, counters)?;
        }
        Ok(())
    }

    /// The groups on the pool ([`run_pooled`]).
    fn pooled(
        &self,
        pool: &ThreadPool,
        scratch: &mut Vec<CheckScratch>,
        counters: &mut WorkCounters,
        tally: &mut Tally,
    ) -> Result<(), BuildFailure> {
        let items = Items {
            n: self.y.len(),
            y: self.y,
            count: self.plan.total_groups(),
        };
        run_pooled(pool, scratch, items, counters, tally, |item, worker| {
            self.check(
                item,
                &mut worker.state,
                &mut worker.out,
                &mut worker.mark,
                &mut worker.counters,
            )
        })
    }
}

/// The items of a pooled check: `count` of them, on states of order `n`
/// that start from `y`.
struct Items<'a> {
    n: usize,
    y: &'a [f64],
    count: usize,
}

/// Items `0 .. count` of a check on the pool. Every thread takes the next
/// item in increasing order and checks it on its own scratch (`state` a copy
/// of `y`), so no item is taken before every smaller one; the failure of the
/// smallest failing item is returned, which is the failure the sequential
/// order meets first. Items above a failure found are skipped: a failing
/// check may enter a different number of right-hand sides than the
/// sequential one, a passing check enters the same.
fn run_pooled<F>(
    pool: &ThreadPool,
    scratch: &mut Vec<CheckScratch>,
    items: Items<'_>,
    counters: &mut WorkCounters,
    tally: &mut Tally,
    check: F,
) -> Result<(), BuildFailure>
where
    F: Fn(usize, &mut CheckScratch) -> Result<(), BuildFailure> + Sync,
{
    let threads = pool.current_num_threads();
    if scratch.len() != threads || scratch.iter().any(|worker| worker.state.len() != items.n) {
        *scratch = (0..threads).map(|_| CheckScratch::new(items.n)).collect();
    }
    let next = AtomicUsize::new(0);
    let first_failure = AtomicUsize::new(usize::MAX);
    pool.install(|| {
        scratch
            .par_iter_mut()
            .with_max_len(1)
            .for_each(|worker| {
                worker.state.copy_from_slice(items.y);
                worker.mark.fill(0);
                loop {
                    let item = next.fetch_add(1, Ordering::Relaxed);
                    if item >= items.count || item > first_failure.load(Ordering::Relaxed) {
                        break;
                    }
                    worker.evaluations += 1;
                    if let Err(failure) = check(item, worker) {
                        first_failure.fetch_min(item, Ordering::Relaxed);
                        worker.failure = Some((item, failure));
                        break;
                    }
                }
            });
    });
    let mut first: Option<(usize, BuildFailure)> = None;
    for worker in scratch.iter_mut() {
        counters.accumulate(worker.counters);
        worker.counters = WorkCounters::default();
        tally.verification += worker.evaluations;
        worker.evaluations = 0;
        if let Some((item, failure)) = worker.failure.take()
            && first.as_ref().is_none_or(|(best, _)| item < *best)
        {
            first = Some((item, failure));
        }
    }
    match first {
        Some((_, failure)) => Err(failure),
        None => Ok(()),
    }
}

/// Rows a colour does not cover must not have changed in its evaluation.
/// `covered` holds the covered rows, increasing.
fn uncovered_rows_unchanged(
    covered: &[u32],
    shifted_rhs: &[f64],
    base: &[f64],
    colour: usize,
) -> Result<(), BuildFailure> {
    if covered.len() == base.len() {
        return Ok(());
    }
    let mut next = 0;
    for (i, (a, b)) in shifted_rhs.iter().zip(base).enumerate() {
        if covered.get(next).is_some_and(|&row| row as usize == i) {
            next += 1;
        } else if a.to_bits() != b.to_bits() {
            return Err(BuildFailure::Assembly(AssemblyError::UncoveredRowChanged {
                row: i,
                colour,
            }));
        }
    }
    Ok(())
}

/// The entries of one column in place (`fd.rs:498-504`).
fn extract_column(f0: &[f64], delta: f64, out: &mut [f64]) -> Result<(), BuildFailure> {
    for (value, base) in out.iter_mut().zip(f0) {
        let entry = (*value - *base) / delta;
        if !entry.is_finite() {
            return Err(overflowed());
        }
        *value = entry;
    }
    Ok(())
}

/// The entries of one colour into `vals` (`fd.rs:609-616`), then the check
/// of the rows the colour does not cover.
fn extract_group(
    group: &Group,
    colour: usize,
    f0: &[f64],
    deltas: &[f64],
    out: &[f64],
    vals: &mut [f64],
) -> Result<(), BuildFailure> {
    for (value, entry) in vals.iter_mut().zip(&group.entries) {
        let i = entry.row as usize;
        let computed = (out[i] - f0[i]) / deltas[entry.col as usize];
        if !computed.is_finite() {
            return Err(overflowed());
        }
        *value = computed;
    }
    uncovered_rows_unchanged(&group.covered, out, f0, colour)
}

/// The dense forward differences of `problem` at `(t, y)`, row-major, with
/// a fresh sequential engine (set-up helper; the calls are charged to
/// `counters`).
pub(super) fn dense_sequential_once(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    counters: &mut WorkCounters,
) -> CoreResult<Vec<f64>> {
    let n = problem.dimension;
    if y.len() != n {
        return Err(CoreError::Dimension("Jacobian state shape mismatch".into()));
    }
    let mut engine = Engine::new(n, TargetLayout::Dense, None, None)
        .map_err(|error| CoreError::InvalidInput(error.to_string()))?;
    let mut target = vec![0.0; n * n];
    match engine.build(problem, t, y, &mut target, counters, &mut Tally::default()) {
        Ok(()) => Ok(target),
        Err(BuildFailure::Core(error)) => Err(error),
        Err(BuildFailure::Assembly(error)) => Err(CoreError::InvalidInput(error.to_string())),
    }
}
