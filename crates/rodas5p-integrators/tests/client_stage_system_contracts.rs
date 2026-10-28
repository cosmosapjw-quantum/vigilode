//! Contracts of the client-owned stage linear solver of the fast driver
//! (library step L2, [`StageLinearSystem`]): the banded delegate
//! [`BandedStageLu`] reproduces the banded driver bit for bit; a client
//! error of a retryable kind is a rejection whose work stays charged and any
//! other kind is fatal; invalid inputs are refused before the client is
//! used; and with an allocation-free client a step allocates nothing.
//! Allocations are counted per thread, as in `rodas5p_fast_allocations.rs`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use rodas5p_core::{CoreError, CoreResult, DenseMatrix, WorkCounters};
use rodas5p_integrators::{
    AdaptiveStepConfig, BandedJacobian, BandedStageLu, OdeProblem, OutputSchedule,
    RODAS5P_FAST_CLIENT_DRIVER_ID, Rodas5pFastLu, StageLinearSystem,
    integrate_rodas5p_fast_banded_observed_with_atol,
    integrate_rodas5p_fast_client_observed_with_atol,
};

#[path = "b1_problems/mod.rs"]
mod b1_problems;
use b1_problems::{Case, brusselator_case, config, hires_case, robertson_case, schedule};

struct Counting;

thread_local! {
    // A const-initialized `Cell` needs no allocation and no destructor, so
    // the allocator can use it on any thread at any time.
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

fn count() {
    let _ = ALLOCATIONS.try_with(|n| n.set(n.get() + 1));
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count();
        unsafe { System.realloc(ptr, layout, new_size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

/// Allocations made by this thread during `f`.
fn allocations_during<T>(f: impl FnOnce() -> T) -> (T, usize) {
    let before = ALLOCATIONS.with(Cell::get);
    let value = f();
    (value, ALLOCATIONS.with(Cell::get) - before)
}

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

/// The 1D Brusselator's band (u and v interleaved: lower = upper = 2), as
/// in `fast_vector_atol_contracts.rs`.
fn brusselator_band(cells: usize) -> BandedJacobian {
    let c = (cells as f64 + 1.0).powi(2) / 50.0;
    BandedJacobian {
        lower: 2,
        upper: 2,
        fill: Arc::new(move |_t, y: &[f64], band: &mut [f64]| {
            let at = |band: &mut [f64], i: usize, j: usize, v: f64| {
                band[i * 5 + (j + 2 - i)] = v;
            };
            for i in 0..cells {
                let (u, v) = (y[2 * i], y[2 * i + 1]);
                let (a, b) = (2 * i, 2 * i + 1);
                at(band, a, a, 2.0 * u * v - 4.0 - 2.0 * c);
                at(band, a, b, u * u);
                at(band, b, a, 3.0 - 2.0 * u * v);
                at(band, b, b, -u * u - 2.0 * c);
                if i > 0 {
                    at(band, a, a - 2, c);
                    at(band, b, b - 2, c);
                }
                if i + 1 < cells {
                    at(band, a, a + 2, c);
                    at(band, b, b + 2, c);
                }
            }
            Ok(())
        }),
    }
}

/// The band `lower = upper = 2` of the problem's dense Jacobian; an entry
/// outside it is an error (HIRES and Robertson have none).
fn dense_band(problem: &OdeProblem) -> BandedJacobian {
    let problem = problem.clone();
    BandedJacobian {
        lower: 2,
        upper: 2,
        fill: Arc::new(move |t, y: &[f64], band: &mut [f64]| {
            let n = problem.dimension;
            let j = problem.dense_jacobian(t, y, &mut WorkCounters::default())?;
            for i in 0..n {
                for c in 0..n {
                    if i.abs_diff(c) <= 2 {
                        band[i * 5 + (c + 2 - i)] = j[(i, c)];
                    } else if j[(i, c)] != 0.0 {
                        return Err(CoreError::InvalidInput(format!(
                            "Jacobian entry ({i}, {c}) outside the band"
                        )));
                    }
                }
            }
            Ok(())
        }),
    }
}

/// A nonuniform per-component tolerance: `atol`, `2 atol`, `3 atol`, ...
fn atol_vector(n: usize, adaptive: &AdaptiveStepConfig) -> Vec<f64> {
    (0..n).map(|i| adaptive.atol * (1 + i % 3) as f64).collect()
}

/// The client run with [`BandedStageLu`] equals the banded driver's run:
/// every output state bit for bit, the counters and the step statistics.
fn assert_delegate_reproduces_banded(case: &Case, band: &BandedJacobian) -> (usize, usize) {
    let n = case.problem.dimension;
    let output = schedule(case.span, 10);
    let (mut rejected, mut reuses) = (0, 0);
    for rtol in [1.0e-6, 1.0e-9] {
        let adaptive = config(case, rtol);
        let atol = atol_vector(n, &adaptive);
        let banded = integrate_rodas5p_fast_banded_observed_with_atol(
            &case.problem,
            band,
            case.span,
            &case.y0,
            &adaptive,
            &output,
            &atol,
        )
        .unwrap();
        let (client, partial, _delegate) = integrate_rodas5p_fast_client_observed_with_atol(
            &case.problem,
            Box::new(BandedStageLu::new(n, band).unwrap()),
            case.span,
            &case.y0,
            &adaptive,
            &output,
            &atol,
        )
        .unwrap();
        let reference = &banded.fast;
        let label = format!("{} at rtol {rtol:e}", case.id);
        assert!(reference.observed.success, "{label}");
        assert!(client.observed.success, "{label}");
        assert_eq!(partial, banded.partial, "{label}");
        assert_eq!(
            bits(&client.observed.t),
            bits(&reference.observed.t),
            "{label}"
        );
        assert_eq!(
            client.observed.y.len(),
            reference.observed.y.len(),
            "{label}"
        );
        for (a, b) in client.observed.y.iter().zip(&reference.observed.y) {
            assert_eq!(bits(a), bits(b), "{label}");
        }
        assert_eq!(
            client.observed.counters, reference.observed.counters,
            "{label}"
        );
        assert_eq!(client.attempts, reference.attempts, "{label}");
        assert_eq!(client.accepted_steps, reference.accepted_steps, "{label}");
        assert_eq!(client.rejected_steps, reference.rejected_steps, "{label}");
        assert_eq!(client.jacobian_reuses, reference.jacobian_reuses, "{label}");
        assert_eq!(
            client.observed.internal_steps, reference.observed.internal_steps,
            "{label}"
        );
        assert_eq!(
            client.observed.output_clipped_steps, reference.observed.output_clipped_steps,
            "{label}"
        );
        assert_eq!(reference.lu, Rodas5pFastLu::Banded);
        assert_eq!(client.lu, Rodas5pFastLu::Client);
        assert_eq!(client.driver, RODAS5P_FAST_CLIENT_DRIVER_ID);
        assert!(
            client.observed.counters.direct_factorizations > 0,
            "{label}"
        );
        assert_eq!(
            client.observed.counters.direct_solve_calls, client.observed.counters.linear_solves,
            "{label}"
        );
        eprintln!(
            "{label}: attempts {}, rejected {}, reuses {}",
            client.attempts, client.rejected_steps, client.jacobian_reuses
        );
        rejected += client.rejected_steps;
        reuses += client.jacobian_reuses;
    }
    (rejected, reuses)
}

#[test]
fn banded_delegate_reproduces_the_banded_driver_bitwise() {
    let brusselator = brusselator_case();
    let hires = hires_case();
    let robertson = robertson_case();
    let mut rejected = 0;
    let mut reuses = 0;
    for (case, band) in [
        (&brusselator, brusselator_band(50)),
        (&hires, dense_band(&hires.problem)),
        (&robertson, dense_band(&robertson.problem)),
    ] {
        let (r, u) = assert_delegate_reproduces_banded(case, &band);
        rejected += r;
        reuses += u;
    }
    // The rejection and reuse paths were exercised.
    assert!(rejected > 0 && reuses > 0, "{rejected} {reuses}");
}

/// The banded delegate whose `fail_at`-th factorization (1-based) or
/// solve fails after charging its work.
struct Faulty {
    inner: BandedStageLu,
    factors: Arc<AtomicUsize>,
    solves: Arc<AtomicUsize>,
    fault: Fault,
}

#[derive(Clone, Copy)]
enum Fault {
    /// The given factorization returns `CoreError::LinearSolve`.
    LinearSolveFactor(usize),
    /// The given factorization returns `CoreError::InvalidInput`.
    FatalFactor(usize),
    /// The given solve returns a NaN.
    NanSolve(usize),
}

impl Faulty {
    fn new(n: usize, band: &BandedJacobian, fault: Fault) -> Self {
        Self {
            inner: BandedStageLu::new(n, band).unwrap(),
            factors: Arc::new(AtomicUsize::new(0)),
            solves: Arc::new(AtomicUsize::new(0)),
            fault,
        }
    }
}

impl StageLinearSystem for Faulty {
    fn dimension(&self) -> usize {
        self.inner.dimension()
    }

    fn linearize(
        &mut self,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        self.inner.linearize(problem, t, y, counters)
    }

    fn factor(&mut self, inv_h_gamma: f64, counters: &mut WorkCounters) -> CoreResult<()> {
        let call = self.factors.fetch_add(1, Ordering::Relaxed) + 1;
        match self.fault {
            Fault::LinearSolveFactor(k) if k == call => {
                counters.direct_factorizations += 1;
                Err(CoreError::LinearSolve(
                    "injected factorization failure".into(),
                ))
            }
            Fault::FatalFactor(k) if k == call => {
                counters.direct_factorizations += 1;
                Err(CoreError::InvalidInput("injected fatal failure".into()))
            }
            _ => self.inner.factor(inv_h_gamma, counters),
        }
    }

    fn solve_in_place(&mut self, b: &mut [f64], counters: &mut WorkCounters) -> CoreResult<()> {
        let call = self.solves.fetch_add(1, Ordering::Relaxed) + 1;
        self.inner.solve_in_place(b, counters)?;
        if matches!(self.fault, Fault::NanSolve(k) if k == call) {
            b[0] = f64::NAN;
        }
        Ok(())
    }
}

#[test]
fn retryable_client_errors_are_rejections_with_their_work_kept() {
    let case = brusselator_case();
    let band = brusselator_band(50);
    let n = case.problem.dimension;
    let output = schedule(case.span, 10);
    let adaptive = config(&case, 1.0e-6);
    let atol = atol_vector(n, &adaptive);
    let run = |client: Box<dyn StageLinearSystem>| {
        integrate_rodas5p_fast_client_observed_with_atol(
            &case.problem,
            client,
            case.span,
            &case.y0,
            &adaptive,
            &output,
            &atol,
        )
    };
    let (clean, _, _) = run(Box::new(BandedStageLu::new(n, &band).unwrap())).unwrap();
    assert_eq!(clean.observed.counters.linear_solve_failures, 0);

    // A LinearSolve error on the third factorization, and a NaN from the
    // tenth solve (the driver's finiteness check): one rejection each.
    for fault in [Fault::LinearSolveFactor(3), Fault::NanSolve(10)] {
        let client = Faulty::new(n, &band, fault);
        let (factors, solves) = (client.factors.clone(), client.solves.clone());
        let (fast, partial, _client) = run(Box::new(client)).unwrap();
        let counters = &fast.observed.counters;
        assert!(fast.observed.success && partial.is_none());
        assert!(fast.rejected_steps >= 1);
        assert_eq!(counters.linear_solve_failures, 1);
        assert_eq!(counters.rejected_steps, fast.rejected_steps as u64);
        assert_eq!(counters.accepted_steps, fast.accepted_steps as u64);
        assert_eq!(fast.attempts, fast.accepted_steps + fast.rejected_steps);
        // The failed attempt's work stays charged: every factorization and
        // solve the client saw, the injected failure included.
        assert_eq!(
            counters.direct_factorizations,
            factors.load(Ordering::Relaxed) as u64
        );
        assert_eq!(
            counters.direct_solve_calls,
            solves.load(Ordering::Relaxed) as u64
        );
        assert_eq!(counters.linear_solves, counters.direct_solve_calls);
        // A failed attempt linearizes the state again: one build per
        // attempt that did not reuse.
        assert_eq!(
            counters.jacobian_builds,
            (fast.attempts - fast.jacobian_reuses) as u64
        );
        assert_eq!(fast.lu, Rodas5pFastLu::Client);
        let (y, reference) = (
            fast.observed.y.last().unwrap(),
            clean.observed.y.last().unwrap(),
        );
        for (a, b) in y.iter().zip(reference) {
            assert!((a - b).abs() <= 1.0e-4 * (1.0 + b.abs()), "{a} {b}");
        }
    }

    // Any other kind is fatal.
    let error = run(Box::new(Faulty::new(n, &band, Fault::FatalFactor(3))))
        .map(|_| ())
        .unwrap_err();
    assert!(matches!(error, CoreError::InvalidInput(_)), "{error:?}");
}

/// `y_i' = -k_i (y_i - 1) - y_i^3`, `k_i` from 1 to 1e4: a diagonal stiff
/// problem. Its dense Jacobian exists only to satisfy [`OdeProblem::new`].
fn diagonal_problem(mass: bool) -> (OdeProblem, Vec<f64>, Vec<f64>) {
    let n = 8;
    let rates: Vec<f64> = (0..n)
        .map(|i| 10f64.powf(4.0 * i as f64 / (n - 1) as f64))
        .collect();
    let k = rates.clone();
    let rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        for ((o, y), k) in out.iter_mut().zip(y).zip(&k) {
            *o = -k * (y - 1.0) - y * y * y;
        }
        Ok(())
    });
    let k = rates.clone();
    let jacobian = Arc::new(move |_t: f64, y: &[f64]| {
        let mut j = DenseMatrix::zeros(n, n);
        for i in 0..n {
            j[(i, i)] = -k[i] - 3.0 * y[i] * y[i];
        }
        Ok(j)
    });
    let mass_matrix = mass.then(|| {
        let mut m = DenseMatrix::zeros(n, n);
        for i in 0..n {
            m[(i, i)] = 1.0;
        }
        m
    });
    let problem = OdeProblem::new(
        "diagonal-cubic",
        n,
        rhs,
        None,
        Some(jacobian),
        None,
        None,
        true,
        mass_matrix,
        None,
    )
    .unwrap();
    let y0 = (0..n).map(|i| 3.0 - 0.25 * i as f64).collect();
    (problem, y0, rates)
}

/// The diagonal `W`: factor stores `1/(1/(h gamma) - J_ii)`, solve
/// multiplies. No allocation after construction.
struct Diagonal {
    rates: Vec<f64>,
    jacobian: Vec<f64>,
    inverse: Vec<f64>,
}

impl Diagonal {
    fn new(rates: &[f64]) -> Self {
        Self {
            rates: rates.to_vec(),
            jacobian: vec![0.0; rates.len()],
            inverse: vec![0.0; rates.len()],
        }
    }
}

impl StageLinearSystem for Diagonal {
    fn dimension(&self) -> usize {
        self.rates.len()
    }

    fn linearize(
        &mut self,
        _problem: &OdeProblem,
        _t: f64,
        y: &[f64],
        _counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        for ((j, k), y) in self.jacobian.iter_mut().zip(&self.rates).zip(y) {
            *j = -k - 3.0 * y * y;
        }
        Ok(())
    }

    fn factor(&mut self, inv_h_gamma: f64, counters: &mut WorkCounters) -> CoreResult<()> {
        counters.direct_factorizations += 1;
        for (w, j) in self.inverse.iter_mut().zip(&self.jacobian) {
            // inv_h_gamma > 0 and J_ii < 0: the diagonal is positive.
            *w = 1.0 / (inv_h_gamma - j);
        }
        Ok(())
    }

    fn solve_in_place(&mut self, b: &mut [f64], counters: &mut WorkCounters) -> CoreResult<()> {
        counters.direct_solve_calls += 1;
        for (x, w) in b.iter_mut().zip(&self.inverse) {
            *x *= w;
        }
        Ok(())
    }
}

fn diagonal_config(rtol: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: 1.0e-4 * rtol,
        rtol,
        initial_step: 1.0e-6,
        min_step: 1.0e-14,
        max_step: 10.0,
        max_attempts: 1_000_000,
        ..AdaptiveStepConfig::default()
    }
}

#[test]
fn a_client_step_allocates_nothing() {
    let (problem, y0, rates) = diagonal_problem(false);
    let output = OutputSchedule::new(vec![0.0, 10.0]).unwrap();
    let run = |rtol: f64| {
        let adaptive = diagonal_config(rtol);
        let atol = vec![adaptive.atol; rates.len()];
        let client = Box::new(Diagonal::new(&rates));
        allocations_during(|| {
            integrate_rodas5p_fast_client_observed_with_atol(
                &problem,
                client,
                (0.0, 10.0),
                &y0,
                &adaptive,
                &output,
                &atol,
            )
            .unwrap()
        })
    };
    run(1.0e-6); // warm the coefficient cache and any lazy statics
    let mut runs = Vec::new();
    for rtol in [1.0e-6, 1.0e-10] {
        let ((fast, partial, _client), allocations) = run(rtol);
        assert!(fast.observed.success && partial.is_none());
        assert_eq!(fast.lu, Rodas5pFastLu::Client);
        // The exact solution tends to the root of k (y - 1) + y^3 = 0.
        for ((y, k), y0) in fast.observed.y[1].iter().zip(&rates).zip(&y0) {
            let residual = k * (y - 1.0) + y * y * y;
            assert!(residual.abs() <= 1.0e-3 * k, "{y} {k} {y0}");
        }
        runs.push((fast.attempts, allocations));
    }
    assert!(runs[1].0 >= 2 * runs[0].0, "{runs:?}");
    assert_eq!(
        runs[0].1, runs[1].1,
        "allocations depend on the step count: {runs:?}"
    );
    eprintln!("client: {runs:?} (attempts, allocations)");
}

#[test]
fn invalid_client_inputs_are_refused() {
    let case = brusselator_case();
    let band = brusselator_band(50);
    let n = case.problem.dimension;
    let output = schedule(case.span, 10);
    let adaptive = config(&case, 1.0e-6);
    let atol = atol_vector(n, &adaptive);
    let refused = |client: Box<dyn StageLinearSystem>, atol: &[f64]| {
        integrate_rodas5p_fast_client_observed_with_atol(
            &case.problem,
            client,
            case.span,
            &case.y0,
            &adaptive,
            &output,
            atol,
        )
        .map(|_| ())
        .unwrap_err()
    };
    // A client of another dimension (49 cells).
    let error = refused(
        Box::new(BandedStageLu::new(n - 2, &brusselator_band(49)).unwrap()),
        &atol,
    );
    assert!(matches!(error, CoreError::InvalidInput(_)), "{error:?}");
    // A tolerance vector of the wrong length, or none.
    for bad in [&atol[..n - 1], &[]] {
        let error = refused(Box::new(BandedStageLu::new(n, &band).unwrap()), bad);
        assert!(matches!(error, CoreError::InvalidInput(_)), "{error:?}");
    }
    // A band outside the matrix.
    let wide = BandedJacobian {
        lower: n,
        ..band.clone()
    };
    assert!(matches!(
        BandedStageLu::new(n, &wide),
        Err(CoreError::InvalidInput(_))
    ));
    // A mass matrix.
    let (problem, y0, rates) = diagonal_problem(true);
    let adaptive = diagonal_config(1.0e-6);
    let error = integrate_rodas5p_fast_client_observed_with_atol(
        &problem,
        Box::new(Diagonal::new(&rates)),
        (0.0, 1.0),
        &y0,
        &adaptive,
        &OutputSchedule::new(vec![0.0, 1.0]).unwrap(),
        &vec![adaptive.atol; rates.len()],
    )
    .map(|_| ())
    .unwrap_err();
    assert!(matches!(error, CoreError::InvalidInput(_)), "{error:?}");
}
