//! The callback guard of one solve and the finite-difference Jacobians.

use std::sync::{
    Arc, Mutex,
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
/// and the facade's own work counters. It only observes and forwards; it
/// performs no floating-point operation on the callbacks' values.
#[derive(Default)]
pub(crate) struct Guard {
    poisoned: AtomicBool,
    fatal: Mutex<Option<FatalRecord>>,
    fatal_errors: AtomicU64,
    recoverable_errors: AtomicU64,
    short_circuited_calls: AtomicU64,
    short_circuited_rhs_calls: AtomicU64,
    fd_builds: AtomicU64,
    fd_rhs_calls: AtomicU64,
}

impl Guard {
    /// Run one user callback. A recoverable error, a fatal error and a call
    /// after a fatal error all reach the driver as `CoreError::NonFinite`,
    /// which every protected driver treats as a rejected step attempt.
    pub(crate) fn call(
        &self,
        callback: Callback,
        t: f64,
        user: impl FnOnce() -> Result<(), CallbackError>,
    ) -> CoreResult<()> {
        if self.poisoned.load(Ordering::Acquire) {
            self.short_circuited_calls.fetch_add(1, Ordering::Relaxed);
            if callback == Callback::Rhs {
                self.short_circuited_rhs_calls
                    .fetch_add(1, Ordering::Relaxed);
            }
            return Err(CoreError::NonFinite(format!(
                "{callback:?} callback not entered: an earlier callback failed fatally"
            )));
        }
        match user() {
            Ok(()) => Ok(()),
            Err(CallbackError::Recoverable(message)) => {
                self.recoverable_errors.fetch_add(1, Ordering::Relaxed);
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
                Err(CoreError::NonFinite(text))
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
        self.call(Callback::Rhs, t, || rhs(t, y, out))?;
        if out.iter().all(|v| v.is_finite()) {
            Ok(())
        } else {
            Err(CoreError::NonFinite(
                "finite-difference RHS produced NaN/Inf".into(),
            ))
        }
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

/// Dense forward differences: `n + 1` RHS calls per build.
pub(crate) fn dense_fd_jacobian(
    guard: Arc<Guard>,
    rhs: RhsFn,
    n: usize,
) -> impl Fn(f64, &[f64]) -> CoreResult<DenseMatrix> + Send + Sync + 'static {
    move |t, y| {
        guard.fd_builds.fetch_add(1, Ordering::Relaxed);
        let mut f0 = vec![0.0; n];
        guard.fd_rhs(&rhs, t, y, &mut f0)?;
        let mut shifted = y.to_vec();
        let mut f1 = vec![0.0; n];
        let mut matrix = DenseMatrix::zeros(n, n);
        for j in 0..n {
            let (yj_shifted, delta) = increment(y[j]);
            shifted[j] = yj_shifted;
            f1.fill(0.0);
            let outcome = guard.fd_rhs(&rhs, t, &shifted, &mut f1);
            shifted[j] = y[j];
            outcome?;
            for i in 0..n {
                let entry = (f1[i] - f0[i]) / delta;
                if !entry.is_finite() {
                    return Err(CoreError::NonFinite(
                        "finite-difference Jacobian entry overflowed".into(),
                    ));
                }
                matrix[(i, j)] = entry;
            }
        }
        Ok(matrix)
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
    let width = lower + upper + 1;
    let groups = band_groups(n, lower, upper);
    move |t, y, band| {
        guard.fd_builds.fetch_add(1, Ordering::Relaxed);
        let mut f0 = vec![0.0; n];
        guard.fd_rhs(&rhs, t, y, &mut f0)?;
        let mut shifted = y.to_vec();
        let mut f1 = vec![0.0; n];
        let mut deltas = vec![0.0; n];
        for group in 0..groups {
            for j in (group..n).step_by(width) {
                let (yj_shifted, delta) = increment(y[j]);
                shifted[j] = yj_shifted;
                deltas[j] = delta;
            }
            f1.fill(0.0);
            let outcome = guard.fd_rhs(&rhs, t, &shifted, &mut f1);
            for j in (group..n).step_by(width) {
                shifted[j] = y[j];
            }
            outcome?;
            // Columns of one group are `width` apart, so their row ranges
            // `[j - upper, j + lower]` are disjoint.
            for j in (group..n).step_by(width) {
                for i in j.saturating_sub(upper)..=(j + lower).min(n - 1) {
                    let entry = (f1[i] - f0[i]) / deltas[j];
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
