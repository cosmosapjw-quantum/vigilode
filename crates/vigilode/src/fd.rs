//! The callback guard of one solve and the finite-difference Jacobians.

use std::sync::{
    Arc, Mutex, TryLockError,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use rodas5p_core::{CoreError, CoreResult, DenseMatrix};

use crate::{
    error::CallbackError,
    problem::RhsFn,
    solution::{Callback, CallbackWork, FiniteDifferenceWork},
};

/// The first fatal callback error of a run.
#[derive(Clone, Debug)]
pub(crate) struct FatalRecord {
    pub(crate) callback: Callback,
    pub(crate) t: f64,
    pub(crate) message: String,
}

/// Per-solve state shared by every wrapped callback: the fatal-error latch
/// and the facade's own work counters. Vector-output callbacks also enforce
/// finite inputs and complete finite output writes before a stage can use them.
pub(crate) struct Guard {
    poisoned: AtomicBool,
    fatal: Mutex<Option<FatalRecord>>,
    fatal_errors: AtomicU64,
    recoverable_errors: AtomicU64,
    short_circuited_calls: AtomicU64,
    short_circuited_rhs_calls: AtomicU64,
    fd_builds: AtomicU64,
    fd_rhs_calls: AtomicU64,
    charged_units: AtomicU64,
    /// Work ceiling in callback units; `NO_CEILING` until armed.
    ceiling: AtomicU64,
    ceiling_hit: AtomicBool,
}

const NO_CEILING: u64 = u64::MAX;

impl Default for Guard {
    fn default() -> Self {
        Self {
            poisoned: AtomicBool::new(false),
            fatal: Mutex::new(None),
            fatal_errors: AtomicU64::new(0),
            recoverable_errors: AtomicU64::new(0),
            short_circuited_calls: AtomicU64::new(0),
            short_circuited_rhs_calls: AtomicU64::new(0),
            fd_builds: AtomicU64::new(0),
            fd_rhs_calls: AtomicU64::new(0),
            charged_units: AtomicU64::new(0),
            ceiling: AtomicU64::new(NO_CEILING),
            ceiling_hit: AtomicBool::new(false),
        }
    }
}

impl Guard {
    /// Stop the run once the charged callback work exceeds `ceiling`: RHS,
    /// JVP and time-derivative calls cost one unit each, a dense or band
    /// Jacobian the units its caller charges (`n`). Calls charged before
    /// arming count toward the total.
    pub(crate) fn arm_ceiling(&self, ceiling: u64) {
        self.ceiling
            .store(ceiling.min(NO_CEILING - 1), Ordering::Release);
    }

    /// The ceiling and the charged work, once the ceiling was exceeded.
    pub(crate) fn ceiling_hit(&self) -> Option<(u64, u64)> {
        self.ceiling_hit.load(Ordering::Acquire).then(|| {
            (
                self.ceiling.load(Ordering::Relaxed),
                self.charged_units.load(Ordering::Relaxed),
            )
        })
    }

    /// Charge `units` of completed callback work; the call that takes the
    /// total past an armed ceiling latches the terminal signal.
    fn charge(&self, callback: Callback, t: f64, units: u64) -> CoreResult<()> {
        let charged = self.charged_units.fetch_add(units, Ordering::Relaxed) + units;
        let ceiling = self.ceiling.load(Ordering::Acquire);
        if charged > ceiling {
            self.ceiling_hit.store(true, Ordering::Release);
            self.poisoned.store(true, Ordering::Release);
            return Err(CoreError::FatalCallback(format!(
                "{callback:?} callback at t = {t:e} completed, but the charged \
                 callback work {charged} exceeds the work ceiling {ceiling}"
            )));
        }
        Ok(())
    }

    /// Evaluate a vector-output callback. An invalid numerical stage is
    /// refused before entering user code; an `Ok` callback that leaves a
    /// component unwritten or nonfinite violates its contract and is terminal.
    /// Dense/banded Jacobians deliberately do not use this helper: their
    /// zero-filled buffers support documented sparse writes.
    pub(crate) fn vector_call(
        &self,
        callback: Callback,
        t: f64,
        y: &[f64],
        out: &mut [f64],
        user: impl FnOnce(&mut [f64]) -> Result<(), CallbackError>,
    ) -> CoreResult<()> {
        if !t.is_finite() || !y.iter().all(|v| v.is_finite()) {
            // Preserve an already latched terminal error even if a caller
            // presents another invalid stage afterward.
            if self.poisoned.load(Ordering::Acquire) {
                return self.call(callback, t, || {
                    unreachable!("fatal latch must short-circuit")
                });
            }
            return Err(CoreError::NonFinite(format!(
                "{callback:?} callback input contains NaN/Inf"
            )));
        }
        self.call(callback, t, || {
            out.fill(f64::NAN);
            user(out)?;
            if out.iter().all(|v| v.is_finite()) {
                Ok(())
            } else {
                Err(CallbackError::Fatal(format!(
                    "{callback:?} callback returned Ok with unwritten or nonfinite output"
                )))
            }
        })
    }

    /// Recoverable errors retain the numerical retry signal. Fatal errors
    /// carry a distinct terminal signal so native drivers stop immediately.
    pub(crate) fn call(
        &self,
        callback: Callback,
        t: f64,
        user: impl FnOnce() -> Result<(), CallbackError>,
    ) -> CoreResult<()> {
        self.call_charged(callback, t, 1, user)
    }

    /// [`Guard::call`] charging `units` of callback work against the work
    /// ceiling, if one is set. A call is charged after it completed, so every
    /// counted call entered user code; the call that takes the charged total
    /// past the ceiling is answered with the terminal signal and latches it.
    pub(crate) fn call_charged(
        &self,
        callback: Callback,
        t: f64,
        units: u64,
        user: impl FnOnce() -> Result<(), CallbackError>,
    ) -> CoreResult<()> {
        if self.poisoned.load(Ordering::Acquire) {
            self.short_circuited_calls.fetch_add(1, Ordering::Relaxed);
            if callback == Callback::Rhs {
                self.short_circuited_rhs_calls
                    .fetch_add(1, Ordering::Relaxed);
            }
            return Err(CoreError::FatalCallback(format!(
                "{callback:?} callback not entered: an earlier callback failed fatally"
            )));
        }
        match user() {
            Ok(()) => self.charge(callback, t, units),
            Err(CallbackError::Recoverable(message)) => {
                self.recoverable_errors.fetch_add(1, Ordering::Relaxed);
                // The call entered user code and is counted by the driver,
                // so it is charged as well; past the ceiling it is terminal.
                self.charge(callback, t, units)?;
                Err(CoreError::NonFinite(format!(
                    "recoverable {callback:?} callback error at t = {t:e}: {message}"
                )))
            }
            Err(CallbackError::Fatal(message)) => {
                self.fatal_errors.fetch_add(1, Ordering::Relaxed);
                let text = format!("fatal {callback:?} callback error at t = {t:e}: {message}");
                {
                    let mut slot = self.fatal.lock().unwrap_or_else(|e| e.into_inner());
                    if slot.is_none() {
                        *slot = Some(FatalRecord {
                            callback,
                            t,
                            message,
                        });
                    }
                }
                self.poisoned.store(true, Ordering::Release);
                Err(CoreError::FatalCallback(text))
            }
        }
    }

    pub(crate) fn fatal(&self) -> Option<FatalRecord> {
        self.fatal.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub(crate) fn callback_work(&self) -> CallbackWork {
        CallbackWork {
            recoverable_errors: self.recoverable_errors.load(Ordering::Relaxed),
            fatal_errors: self.fatal_errors.load(Ordering::Relaxed),
            short_circuited_calls: self.short_circuited_calls.load(Ordering::Relaxed),
            short_circuited_rhs_calls: self.short_circuited_rhs_calls.load(Ordering::Relaxed),
        }
    }

    pub(crate) fn fd_work(&self) -> FiniteDifferenceWork {
        FiniteDifferenceWork {
            jacobian_builds: self.fd_builds.load(Ordering::Relaxed),
            rhs_calls: self.fd_rhs_calls.load(Ordering::Relaxed),
        }
    }

    /// One right-hand-side call made by a finite-difference build: counted
    /// before user code is entered, and checked finite.
    fn fd_rhs(&self, rhs: &RhsFn, t: f64, y: &[f64], out: &mut [f64]) -> CoreResult<()> {
        self.fd_rhs_calls.fetch_add(1, Ordering::Relaxed);
        self.vector_call(Callback::Rhs, t, y, out, |out| rhs(t, y, out))
    }
}

/// The forward-difference increment of component `y_j`: the represented
/// difference `(y_j + sqrt(eps) max(|y_j|, 1)) - y_j`.
fn increment(yj: f64) -> (f64, f64) {
    let raw = f64::EPSILON.sqrt() * yj.abs().max(1.0);
    let forward = yj + raw;
    // At the positive finite boundary the forward shift overflows even
    // though a backward difference is representable. Keep the usual rule
    // bitwise unchanged everywhere else and divide by the signed increment.
    let shifted = if forward.is_finite() {
        forward
    } else {
        yj - raw
    };
    (shifted, shifted - yj)
}

/// One solve owns this scratch. The lock covers a complete build, so another
/// caller cannot observe the shifted vector half restored. A reentrant or
/// concurrent call uses separate scratch rather than waiting for an RHS
/// callback that may itself have entered this Jacobian provider.
struct DenseFdScratch {
    f0: Vec<f64>,
    shifted: Vec<f64>,
    f1: Vec<f64>,
}

impl DenseFdScratch {
    fn new(n: usize) -> Self {
        Self {
            f0: vec![0.0; n],
            shifted: vec![0.0; n],
            f1: vec![0.0; n],
        }
    }

    fn fill(
        &mut self,
        guard: &Guard,
        rhs: &RhsFn,
        t: f64,
        y: &[f64],
        matrix: &mut DenseMatrix,
    ) -> CoreResult<()> {
        // The driver's matrix can contain the previous build. A failed
        // callback must not leave those entries looking current.
        matrix.as_mut_slice().fill(0.0);
        guard.fd_builds.fetch_add(1, Ordering::Relaxed);
        self.f0.fill(0.0);
        guard.fd_rhs(rhs, t, y, &mut self.f0)?;
        self.shifted.copy_from_slice(y);
        for j in 0..y.len() {
            let (yj_shifted, delta) = increment(y[j]);
            self.shifted[j] = yj_shifted;
            self.f1.fill(0.0);
            let outcome = guard.fd_rhs(rhs, t, &self.shifted, &mut self.f1);
            self.shifted[j] = y[j];
            outcome?;
            for i in 0..y.len() {
                let entry = (self.f1[i] - self.f0[i]) / delta;
                if !entry.is_finite() {
                    return Err(CoreError::NonFinite(
                        "finite-difference Jacobian entry overflowed".into(),
                    ));
                }
                matrix[(i, j)] = entry;
            }
        }
        Ok(())
    }
}

fn fill_dense_with_scratch(
    scratch: &Mutex<DenseFdScratch>,
    n: usize,
    guard: &Guard,
    rhs: &RhsFn,
    t: f64,
    y: &[f64],
    matrix: &mut DenseMatrix,
) -> CoreResult<()> {
    match scratch.try_lock() {
        Ok(mut scratch) => scratch.fill(guard, rhs, t, y, matrix),
        Err(TryLockError::Poisoned(e)) => e.into_inner().fill(guard, rhs, t, y, matrix),
        Err(TryLockError::WouldBlock) => {
            // A callback may reenter on the same thread. Waiting on the
            // mutex would deadlock; isolated fallback preserves the call.
            DenseFdScratch::new(n).fill(guard, rhs, t, y, matrix)
        }
    }
}

/// Dense forward differences: `n + 1` RHS calls per build.
pub(crate) fn dense_fd_jacobian(
    guard: Arc<Guard>,
    rhs: RhsFn,
    n: usize,
) -> impl Fn(f64, &[f64]) -> CoreResult<DenseMatrix> + Send + Sync + 'static {
    let scratch = Mutex::new(DenseFdScratch::new(n));
    move |t, y| {
        let mut matrix = DenseMatrix::zeros(n, n);
        fill_dense_with_scratch(&scratch, n, &guard, &rhs, t, y, &mut matrix)?;
        Ok(matrix)
    }
}

/// The fast dense driver's in-place callback; no matrix or scratch allocation
/// is needed for an uncontended build after the solve has been constructed.
pub(crate) fn dense_fd_fill(
    guard: Arc<Guard>,
    rhs: RhsFn,
    n: usize,
) -> impl Fn(f64, &[f64], &mut DenseMatrix) -> CoreResult<()> + Send + Sync + 'static {
    let scratch = Mutex::new(DenseFdScratch::new(n));
    move |t, y, matrix| fill_dense_with_scratch(&scratch, n, &guard, &rhs, t, y, matrix)
}

#[derive(Clone, Copy)]
struct BandShape {
    n: usize,
    lower: usize,
    upper: usize,
}

struct BandedFdScratch {
    f0: Vec<f64>,
    shifted: Vec<f64>,
    f1: Vec<f64>,
    deltas: Vec<f64>,
}

impl BandedFdScratch {
    fn new(n: usize) -> Self {
        Self {
            f0: vec![0.0; n],
            shifted: vec![0.0; n],
            f1: vec![0.0; n],
            deltas: vec![0.0; n],
        }
    }

    fn fill(
        &mut self,
        guard: &Guard,
        rhs: &RhsFn,
        t: f64,
        y: &[f64],
        band: &mut [f64],
        shape: BandShape,
    ) -> CoreResult<()> {
        let BandShape { n, lower, upper } = shape;
        let width = lower + upper + 1;
        let groups = band_groups(n, lower, upper);
        guard.fd_builds.fetch_add(1, Ordering::Relaxed);
        self.f0.fill(0.0);
        guard.fd_rhs(rhs, t, y, &mut self.f0)?;
        self.shifted.copy_from_slice(y);
        for group in 0..groups {
            for j in (group..n).step_by(width) {
                let (yj_shifted, delta) = increment(y[j]);
                self.shifted[j] = yj_shifted;
                self.deltas[j] = delta;
            }
            self.f1.fill(0.0);
            let outcome = guard.fd_rhs(rhs, t, &self.shifted, &mut self.f1);
            for j in (group..n).step_by(width) {
                self.shifted[j] = y[j];
            }
            outcome?;
            for j in (group..n).step_by(width) {
                for i in j.saturating_sub(upper)..=(j + lower).min(n - 1) {
                    let entry = (self.f1[i] - self.f0[i]) / self.deltas[j];
                    if !entry.is_finite() {
                        return Err(CoreError::NonFinite(
                            "finite-difference band entry overflowed".into(),
                        ));
                    }
                    band[i * width + (j + lower - i)] = entry;
                }
            }
        }
        Ok(())
    }
}

/// The Curtis–Powell–Reid column groups of a band: `min(lower + upper + 1, n)`.
pub(crate) fn band_groups(n: usize, lower: usize, upper: usize) -> usize {
    (lower + upper + 1).min(n)
}

/// Grouped forward differences of a declared band, in the band layout of
/// [`crate::BandJacobianFn`]: `band_groups + 1` RHS calls per build.
pub(crate) fn banded_fd_fill(
    guard: Arc<Guard>,
    rhs: RhsFn,
    n: usize,
    lower: usize,
    upper: usize,
) -> impl Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync + 'static {
    let shape = BandShape { n, lower, upper };
    let scratch = Mutex::new(BandedFdScratch::new(n));
    move |t, y, band| match scratch.try_lock() {
        Ok(mut scratch) => scratch.fill(&guard, &rhs, t, y, band, shape),
        Err(TryLockError::Poisoned(e)) => e.into_inner().fill(&guard, &rhs, t, y, band, shape),
        Err(TryLockError::WouldBlock) => {
            BandedFdScratch::new(n).fill(&guard, &rhs, t, y, band, shape)
        }
    }
}

/// The dense matrix of a band (for the `OdeProblem` constructor's required
/// Jacobian provider on the banded route; the banded driver reads the band).
pub(crate) fn dense_from_band(values: &[f64], n: usize, lower: usize, upper: usize) -> DenseMatrix {
    let width = lower + upper + 1;
    let mut matrix = DenseMatrix::zeros(n, n);
    for i in 0..n {
        for j in i.saturating_sub(lower)..=(i + upper).min(n - 1) {
            matrix[(i, j)] = values[i * width + (j + lower - i)];
        }
    }
    matrix
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dense_into_matches_returned_matrix_bits_and_counters() {
        let n = 7;
        let y = [0.1, -0.3, 0.6, 0.2, -0.8, 0.5, 1.1];
        let first = Arc::new(Guard::default());
        let second = Arc::new(Guard::default());
        let rhs = tridiagonal(n);
        let allocating = dense_fd_jacobian(first.clone(), rhs.clone(), n);
        let into = dense_fd_fill(second.clone(), rhs, n);
        let expected = allocating(0.4, &y).unwrap();
        let mut actual = DenseMatrix::zeros(n, n);
        actual.as_mut_slice().fill(-99.0);
        into(0.4, &y, &mut actual).unwrap();
        assert_eq!(
            expected
                .as_slice()
                .iter()
                .map(|x| x.to_bits())
                .collect::<Vec<_>>(),
            actual
                .as_slice()
                .iter()
                .map(|x| x.to_bits())
                .collect::<Vec<_>>()
        );
        assert_eq!(first.fd_work(), second.fd_work());
        into(0.4, &y, &mut actual).unwrap();
    }

    fn original_dense_reference(rhs: &RhsFn, n: usize, t: f64, y: &[f64]) -> DenseMatrix {
        // Exact pre-change arithmetic and column traversal, kept only as a
        // local regression oracle for the changed storage strategy.
        let mut f0 = vec![0.0; n];
        rhs(t, y, &mut f0).unwrap();
        let mut shifted = y.to_vec();
        let mut f1 = vec![0.0; n];
        let mut matrix = DenseMatrix::zeros(n, n);
        for j in 0..n {
            let (yj_shifted, delta) = increment(y[j]);
            shifted[j] = yj_shifted;
            f1.fill(0.0);
            rhs(t, &shifted, &mut f1).unwrap();
            shifted[j] = y[j];
            for i in 0..n {
                matrix[(i, j)] = (f1[i] - f0[i]) / delta;
            }
        }
        matrix
    }

    #[test]
    fn fd_workspace_preserves_original_dense_bits() {
        let n = 7;
        let y = [0.1, -0.3, 0.6, 0.2, -0.8, 0.5, 1.1];
        let rhs = tridiagonal(n);
        let original = original_dense_reference(&rhs, n, 0.25, &y);
        let optimized = dense_fd_jacobian(Arc::new(Guard::default()), rhs, n)(0.25, &y).unwrap();
        assert_eq!(
            original
                .as_slice()
                .iter()
                .map(|x| x.to_bits())
                .collect::<Vec<_>>(),
            optimized
                .as_slice()
                .iter()
                .map(|x| x.to_bits())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn recoverable_fd_failure_restores_shift_and_preserves_charges() {
        let guard = Arc::new(Guard::default());
        let calls = Arc::new(AtomicU64::new(0));
        let seen = Arc::new(Mutex::new(Vec::<Vec<u64>>::new()));
        let rhs: RhsFn = {
            let (calls, seen) = (calls.clone(), seen.clone());
            Arc::new(move |_, y, out| {
                seen.lock()
                    .unwrap()
                    .push(y.iter().map(|v| v.to_bits()).collect());
                let call = calls.fetch_add(1, Ordering::Relaxed);
                if call == 1 {
                    return Err(CallbackError::Recoverable("once".into()));
                }
                out.copy_from_slice(y);
                Ok(())
            })
        };
        let jac = dense_fd_jacobian(guard.clone(), rhs, 2);
        let y = [0.5, -1.5];
        assert!(matches!(jac(0.0, &y), Err(CoreError::NonFinite(_))));
        let matrix = jac(0.0, &y).unwrap();
        assert_eq!(matrix[(0, 0)].to_bits(), 1.0f64.to_bits());
        assert_eq!(matrix[(1, 1)].to_bits(), 1.0f64.to_bits());
        assert_eq!(guard.fd_work().jacobian_builds, 2);
        assert_eq!(guard.fd_work().rhs_calls, 5);
        assert_eq!(guard.callback_work().recoverable_errors, 1);
        let seen = seen.lock().unwrap();
        assert_eq!(seen[0], seen[2]);
        assert_eq!(seen[2], y.map(f64::to_bits));
    }

    #[test]
    fn fatal_fd_error_enters_callback_once_then_short_circuits() {
        let guard = Arc::new(Guard::default());
        let calls = Arc::new(AtomicU64::new(0));
        let rhs: RhsFn = {
            let calls = calls.clone();
            Arc::new(move |_, _, _| {
                calls.fetch_add(1, Ordering::Relaxed);
                Err(CallbackError::Fatal("stop".into()))
            })
        };
        let jac = banded_fd_fill(guard.clone(), rhs, 2, 0, 0);
        let mut band = [0.0; 2];
        assert!(matches!(
            jac(0.0, &[1.0, 2.0], &mut band),
            Err(CoreError::FatalCallback(_))
        ));
        assert!(matches!(
            jac(0.0, &[1.0, 2.0], &mut band),
            Err(CoreError::FatalCallback(_))
        ));
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        assert_eq!(guard.fd_work().jacobian_builds, 2);
        assert_eq!(guard.fd_work().rhs_calls, 2);
        assert_eq!(guard.callback_work().fatal_errors, 1);
        assert_eq!(guard.callback_work().short_circuited_rhs_calls, 1);
    }

    #[test]
    fn recoverable_band_failure_restores_group_shift_before_retry() {
        let guard = Arc::new(Guard::default());
        let calls = Arc::new(AtomicU64::new(0));
        let seen = Arc::new(Mutex::new(Vec::<Vec<u64>>::new()));
        let rhs: RhsFn = {
            let (calls, seen) = (calls.clone(), seen.clone());
            Arc::new(move |_, y, out| {
                seen.lock()
                    .unwrap()
                    .push(y.iter().map(|v| v.to_bits()).collect());
                if calls.fetch_add(1, Ordering::Relaxed) == 1 {
                    return Err(CallbackError::Recoverable("once".into()));
                }
                out.copy_from_slice(y);
                Ok(())
            })
        };
        let jac = banded_fd_fill(guard.clone(), rhs, 4, 1, 1);
        let y = [0.5, -1.5, 2.25, 0.1];
        let mut band = [0.0; 12];
        assert!(matches!(
            jac(0.0, &y, &mut band),
            Err(CoreError::NonFinite(_))
        ));
        jac(0.0, &y, &mut band).unwrap();
        let seen = seen.lock().unwrap();
        assert_eq!(seen[0], seen[2]);
        assert_eq!(seen[2], y.map(f64::to_bits));
        assert_eq!(guard.fd_work().jacobian_builds, 2);
        assert_eq!(guard.fd_work().rhs_calls, 6);
        assert_eq!(guard.callback_work().recoverable_errors, 1);
    }

    #[test]
    fn concurrent_build_uses_isolated_scratch_without_waiting_on_callback() {
        use std::time::{Duration, Instant};

        let base_calls = Arc::new(AtomicU64::new(0));
        let rhs: RhsFn = {
            let base_calls = base_calls.clone();
            Arc::new(move |_, y, out| {
                // Both f0 evaluations must enter before either can return.
                // A blocking mutex would time out here instead of hanging CI.
                if base_calls.fetch_add(1, Ordering::Relaxed) < 2 {
                    let deadline = Instant::now() + Duration::from_secs(2);
                    while base_calls.load(Ordering::Relaxed) < 2 {
                        if Instant::now() >= deadline {
                            return Err(CallbackError::Fatal("blocked build".into()));
                        }
                        std::thread::yield_now();
                    }
                }
                out.copy_from_slice(y);
                Ok(())
            })
        };
        let guard = Arc::new(Guard::default());
        let jac = dense_fd_jacobian(guard.clone(), rhs, 2);
        std::thread::scope(|scope| {
            let a = scope.spawn(|| jac(0.0, &[0.25, 0.5]).unwrap());
            let b = scope.spawn(|| jac(0.0, &[0.75, 1.5]).unwrap());
            for result in [a.join().unwrap(), b.join().unwrap()] {
                assert_eq!(result[(0, 0)], 1.0);
                assert_eq!(result[(1, 1)], 1.0);
            }
        });
        assert_eq!(guard.fd_work().jacobian_builds, 2);
        assert_eq!(guard.fd_work().rhs_calls, 6);
    }

    fn tridiagonal(n: usize) -> RhsFn {
        Arc::new(move |_t, y, dy| {
            for i in 0..n {
                let left = if i > 0 { y[i - 1] } else { 0.0 };
                let right = if i + 1 < n { y[i + 1] } else { 0.0 };
                dy[i] = 2.0 * left - 3.0 * y[i] * y[i] + 0.5 * right;
            }
            Ok(())
        })
    }

    #[test]
    fn grouped_band_equals_the_band_of_the_dense_difference() {
        let n = 9;
        let guard = Arc::new(Guard::default());
        let rhs = tridiagonal(n);
        let y: Vec<f64> = (0..n).map(|i| 0.3 + 0.7 * i as f64).collect();
        let dense = dense_fd_jacobian(guard.clone(), rhs.clone(), n)(0.0, &y).unwrap();
        let mut band = vec![0.0; 3 * n];
        banded_fd_fill(guard.clone(), rhs, n, 1, 1)(0.0, &y, &mut band).unwrap();
        let from_band = dense_from_band(&band, n, 1, 1);
        for i in 0..n {
            for j in 0..n {
                // Identical arithmetic per column: bit-equal entries.
                assert_eq!(
                    dense[(i, j)].to_bits(),
                    from_band[(i, j)].to_bits(),
                    "{i},{j}"
                );
            }
        }
        let work = guard.fd_work();
        assert_eq!(work.jacobian_builds, 2);
        assert_eq!(work.rhs_calls, (n as u64 + 1) + (3 + 1));
    }

    #[test]
    fn dense_difference_is_accurate_to_the_increment() {
        let n = 4;
        let guard = Arc::new(Guard::default());
        let y = [0.5, -2.0, 3.0, 1.0e3];
        let j = dense_fd_jacobian(guard, tridiagonal(n), n)(0.0, &y).unwrap();
        for i in 0..n {
            let exact = -6.0 * y[i];
            let tol = 1.0e-6 * y[i].abs().max(1.0) * 6.0;
            assert!((j[(i, i)] - exact).abs() <= tol, "{} vs {exact}", j[(i, i)]);
        }
    }

    #[test]
    fn finite_boundary_uses_a_signed_representable_increment() {
        // Multiplication by an exact power of two has an independent exact
        // derivative, including when that derivative is subnormal.
        let alpha = f64::from_bits(1_u64 << 51); // 2^-1023
        let rhs: RhsFn = Arc::new(move |_t, y, dy| {
            assert!(y.iter().all(|x| x.is_finite()));
            for (out, value) in dy.iter_mut().zip(y) {
                *out = alpha * value;
            }
            Ok(())
        });
        let guard = Arc::new(Guard::default());
        let y = [f64::MAX, -f64::MAX];
        let dense = dense_fd_jacobian(guard.clone(), rhs.clone(), 2)(0.0, &y).unwrap();
        let mut band = [0.0; 2];
        banded_fd_fill(guard.clone(), rhs, 2, 0, 0)(0.0, &y, &mut band).unwrap();
        for i in 0..2 {
            assert_eq!(dense[(i, i)], alpha);
            assert_eq!(band[i], alpha);
            assert_eq!(dense[(i, 1 - i)], 0.0);
        }
        assert_eq!(guard.fd_work().rhs_calls, 5);
        for x in [0.0, 1.0, -2.0, 1e200, -f64::MAX] {
            let raw = f64::EPSILON.sqrt() * x.abs().max(1.0);
            assert_eq!(increment(x), (x + raw, (x + raw) - x));
        }
    }

    #[test]
    fn nonfinite_difference_entries_fail_before_factorization() {
        let rhs: RhsFn = Arc::new(|_t, y, dy| {
            dy[0] = if y[0] == 0.0 { f64::MAX } else { -f64::MAX };
            Ok(())
        });
        let guard = Arc::new(Guard::default());
        assert!(matches!(
            dense_fd_jacobian(guard.clone(), rhs.clone(), 1)(0.0, &[0.0]),
            Err(CoreError::NonFinite(_))
        ));
        assert!(matches!(
            banded_fd_fill(guard.clone(), rhs, 1, 0, 0)(0.0, &[0.0], &mut [0.0]),
            Err(CoreError::NonFinite(_))
        ));
        assert_eq!(guard.fd_work().rhs_calls, 4);
    }

    #[test]
    fn vector_callback_refuses_invalid_input_without_entering_user_code() {
        for (t, y) in [(f64::INFINITY, 1.0), (0.0, f64::NAN)] {
            let guard = Guard::default();
            let mut out = [7.0];
            let mut entered = false;
            let result = guard.vector_call(Callback::Rhs, t, &[y], &mut out, |_| {
                entered = true;
                Ok(())
            });
            assert!(matches!(result, Err(CoreError::NonFinite(_))));
            assert!(!entered);
            assert_eq!(out, [7.0]);
            assert!(guard.fatal().is_none());
        }
    }

    #[test]
    fn vector_callback_detects_stale_output_on_reused_buffer() {
        let guard = Guard::default();
        let mut out = [3.0, 4.0];
        let result = guard.vector_call(Callback::Rhs, 0.0, &[1.0, 2.0], &mut out, |out| {
            out[0] = -1.0;
            Ok(())
        });
        assert!(matches!(result, Err(CoreError::FatalCallback(_))));
        assert!(out[1].is_nan());
        assert_eq!(guard.callback_work().fatal_errors, 1);
    }

    #[test]
    fn a_fatal_error_latches_and_short_circuits() {
        let guard = Guard::default();
        assert!(
            guard
                .call(Callback::Rhs, 1.0, || Err(CallbackError::Fatal("x".into())))
                .is_err()
        );
        let mut entered = false;
        assert!(
            guard
                .call(Callback::Jvp, 2.0, || {
                    entered = true;
                    Ok(())
                })
                .is_err()
        );
        assert!(!entered);
        let work = guard.callback_work();
        assert_eq!((work.fatal_errors, work.short_circuited_calls), (1, 1));
        let fatal = guard.fatal().unwrap();
        assert_eq!((fatal.callback, fatal.t), (Callback::Rhs, 1.0));
    }
}
