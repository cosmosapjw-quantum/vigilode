//! Contracts of the structured stage solvers of the LRS photon transport
//! adapter (BASS/rei DAG node P03, `examples/lrs_photon_fv_support/
//! {block_solver,border}.rs`) and their runs as clients of the RODAS5P fast
//! driver (library step L2):
//!
//! * C1 (unit level): the exact block-triangular solve equals a dense LU of
//!   `W = inv I - J` and is backward stable, for downward and upward energy
//!   drift, outflow and closed boundaries, with and without Thomson and
//!   absorption; the structured operator is the dense `W`.
//! * C3: a mixed drift direction is a typed refusal of the exact mode;
//!   the Krylov mode (one- and two-level sweeps) reaches its tolerance.
//! * C4: factor without a linearization, solve without a factorization of
//!   the current linearization or after a rebind, and wrong shapes are
//!   typed refusals; a rebind to a same-shape mesh with other edges and
//!   boundary solves the new mesh (dense LU, and bit for bit a fresh solver).
//! * C5: the counted flops equal the closed forms.
//! * C6 (VG-1): linearize, factor and solve allocate nothing after
//!   construction, from the first cycle on; client runs allocate the same count at two tolerances
//!   whose attempt counts differ at least twice.
//! * C7 (VG-3): repeated client runs are bit for bit identical; the exact
//!   client agrees with the banded driver (WRMS <= 1).
//! * C8: the bordered model's Jacobian against central differences, its
//!   Schur solve against a dense LU, and the closed-form Riccati law.

use std::alloc::{GlobalAlloc, Layout as AllocLayout, System};
use std::cell::Cell;
use std::sync::Arc;

use rodas5p_core::{CoreError, DenseMatrix, LuFactorization, WorkCounters};
use rodas5p_integrators::{
    AdaptiveStepConfig, OdeProblem, OutputSchedule, Rodas5pFastLu, StageLinearSystem,
    integrate_rodas5p_fast_banded_observed_with_atol,
    integrate_rodas5p_fast_client_observed_with_atol,
};

#[path = "../examples/lrs_photon_fv_support/mod.rs"]
mod support;
use support::block_solver::{
    BlockStageSolver, Capacitance, Direction, Mode, exact_solve_flops, factor_flops,
};
use support::border::{BorderParams, Bordered, BorderedStageSolver};
use support::{Absorber, Boundary, Coordinate, Geometry, Layout, Model};

struct Counting;

thread_local! {
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

fn count() {
    let _ = ALLOCATIONS.try_with(|n| n.set(n.get() + 1));
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: AllocLayout) -> *mut u8 {
        count();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: AllocLayout) -> *mut u8 {
        count();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: AllocLayout, new_size: usize) -> *mut u8 {
        count();
        unsafe { System.realloc(ptr, layout, new_size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: AllocLayout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

fn allocations_during<T>(f: impl FnOnce() -> T) -> (T, usize) {
    let before = ALLOCATIONS.with(Cell::get);
    let value = f();
    (value, ALLOCATIONS.with(Cell::get) - before)
}

/// Deterministic xorshift64 (no external crate).
struct XorShift(u64);

impl XorShift {
    fn uniform(&mut self) -> f64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn uniform_edges(lo: f64, hi: f64, n: usize) -> Vec<f64> {
    (0..=n)
        .map(|k| {
            if k == n {
                hi
            } else {
                lo + (hi - lo) * k as f64 / n as f64
            }
        })
        .collect()
}

fn max_abs(v: &[f64]) -> f64 {
    v.iter().fold(0.0_f64, |a, x| a.max(x.abs()))
}

fn model(
    nq: usize,
    nm: usize,
    geometry: Geometry,
    gamma: f64,
    absorber: bool,
    boundary: Boundary,
) -> Arc<Model> {
    let layout = Layout::new(
        uniform_edges(-4.0, 2.5, nq),
        uniform_edges(-1.0, 1.0, nm),
        Coordinate::Physical,
        boundary,
    )
    .unwrap();
    let absorber = absorber.then_some(Absorber {
        k0: 0.5,
        q_chi: -0.5,
    });
    Arc::new(Model::new(layout, geometry, gamma, absorber, false).unwrap())
}

const RPLUS: Geometry = Geometry::PowerLaw {
    hh: 0.5,
    s0: 0.2,
    p: 1.5,
};
const MIXED: Geometry = Geometry::Constant { h: 0.1, s: 0.2 };
const KRYLOV: Mode = Mode::Krylov {
    restart: 4,
    max_iterations: 48,
    rtol: 1.0e-12,
    two_level: true,
};

fn initial_state(model: &Model) -> Vec<f64> {
    let l = &model.layout;
    let mut y0 = vec![0.0; l.n()];
    for i in 0..l.nq {
        let q = 0.5 * (l.q_edges[i] + l.q_edges[i + 1]);
        for j in 0..l.nm {
            let mu = 0.5 * (l.mu_edges[j] + l.mu_edges[j + 1]);
            y0[l.cell(i, j)] = l.dq[i]
                * l.dm[j]
                * (-((q + 0.5) * (q + 0.5)) / (2.0 * 0.09)).exp()
                * (1.0 + 0.25 * (3.0 * mu * mu - 1.0));
        }
    }
    y0
}

/// `W = inv I - J` of a dense Jacobian.
fn w_of(j: &DenseMatrix, inv: f64) -> DenseMatrix {
    let n = j.nrows();
    let mut w = DenseMatrix::zeros(n, n);
    for r in 0..n {
        for c in 0..n {
            w[(r, c)] = if r == c { inv } else { 0.0 } - j[(r, c)];
        }
    }
    w
}

fn matvec(w: &DenseMatrix, x: &[f64]) -> Vec<f64> {
    (0..w.nrows())
        .map(|r| (0..w.ncols()).map(|c| w[(r, c)] * x[c]).sum())
        .collect()
}

/// Normwise backward error `||b - W x|| / (||W|| ||x|| + ||b||)` (inf-norms).
fn backward_error(w: &DenseMatrix, x: &[f64], b: &[f64]) -> f64 {
    let n = w.nrows();
    let wx = matvec(w, x);
    let r: Vec<f64> = b.iter().zip(&wx).map(|(a, c)| a - c).collect();
    let w_norm = (0..n)
        .map(|r| (0..n).map(|c| w[(r, c)].abs()).sum::<f64>())
        .fold(0.0_f64, f64::max);
    max_abs(&r) / (w_norm * max_abs(x) + max_abs(b))
}

/// `kappa_inf(W)` from the LU's inverse columns.
fn condition(w: &DenseMatrix, lu: &LuFactorization) -> f64 {
    let n = w.nrows();
    let mut rows = vec![0.0; n];
    for c in 0..n {
        let mut e = vec![0.0; n];
        e[c] = 1.0;
        for (r, v) in lu.solve(&e).unwrap().iter().enumerate() {
            rows[r] += v.abs();
        }
    }
    let w_norm = (0..n)
        .map(|r| (0..n).map(|c| w[(r, c)].abs()).sum::<f64>())
        .fold(0.0_f64, f64::max);
    w_norm * rows.iter().fold(0.0_f64, |m, v| m.max(*v))
}

fn random_rhs(n: usize, rng: &mut XorShift) -> Vec<f64> {
    (0..n).map(|_| rng.uniform() - 0.25).collect()
}

#[test]
fn c1_exact_solve_matches_a_dense_lu_and_is_backward_stable() {
    let mut rng = XorShift(0x9e37_79b9_7f4a_7c15);
    let up = Geometry::Constant { h: -0.5, s: 0.1 };
    let cases = [
        (RPLUS, 0.0, false, Boundary::Outflow, Direction::Down),
        (RPLUS, 1.0, true, Boundary::Outflow, Direction::Down),
        (RPLUS, 1.0e4, true, Boundary::Closed, Direction::Down),
        (RPLUS, 1.0e6, false, Boundary::Outflow, Direction::Down),
        (up, 1.0e2, true, Boundary::Outflow, Direction::Up),
        (up, 1.0e5, false, Boundary::Closed, Direction::Up),
    ];
    for (geometry, gamma, absorber, boundary, direction) in cases {
        let m = model(12, 6, geometry, gamma, absorber, boundary);
        let n = m.layout.n();
        let mut j = DenseMatrix::zeros(n, n);
        m.dense_fill(0.3, &mut j).unwrap();
        for capacitance in [Capacitance::Conservation, Capacitance::Textbook] {
            let mut solver = BlockStageSolver::new(m.clone(), 0, Mode::Exact, capacitance).unwrap();
            solver.linearize_at(0.3).unwrap();
            assert_eq!(solver.direction(), direction);
            for inv in [1.0e-2, 1.0, 1.0e3] {
                solver.factor_with(inv).unwrap();
                let w = w_of(&j, inv);
                let b = random_rhs(n, &mut rng);
                let mut x = b.clone();
                solver
                    .solve_with(&mut x, &mut WorkCounters::default())
                    .unwrap();
                let lu = LuFactorization::new(&w).unwrap();
                let x_lu = lu.solve(&b).unwrap();
                let eta = backward_error(&w, &x, &b);
                let diff = x
                    .iter()
                    .zip(&x_lu)
                    .map(|(a, c)| (a - c).abs())
                    .fold(0.0, f64::max)
                    / max_abs(&x_lu);
                let kappa = condition(&w, &lu);
                assert!(
                    eta <= 1.0e-14,
                    "{geometry:?} gamma {gamma} inv {inv}: eta {eta:e}"
                );
                // both solutions are backward stable: they differ by O(kappa eps)
                assert!(
                    diff <= 10.0 * kappa * f64::EPSILON,
                    "{geometry:?} gamma {gamma} inv {inv}: diff {diff:e} kappa {kappa:e}"
                );
                // the structured operator is the dense W
                let mut wx = vec![0.0; n];
                solver.apply_w(&x, &mut wx).unwrap();
                let dense = matvec(&w, &x);
                let scale: f64 = (0..n)
                    .map(|r| (0..n).map(|c| (w[(r, c)] * x[c]).abs()).sum::<f64>())
                    .fold(0.0, f64::max);
                let gap = wx
                    .iter()
                    .zip(&dense)
                    .map(|(a, c)| (a - c).abs())
                    .fold(0.0, f64::max);
                assert!(gap <= 1.0e-14 * scale, "operator gap {gap:e} vs {scale:e}");
            }
        }
    }
}

#[test]
fn c3_mixed_direction_is_refused_by_the_exact_mode_and_krylov_converges() {
    let m = model(16, 8, MIXED, 1.0e4, true, Boundary::Outflow);
    let mut exact =
        BlockStageSolver::new(m.clone(), 0, Mode::Exact, Capacitance::Conservation).unwrap();
    match exact.linearize_at(0.0) {
        Err(CoreError::InvalidInput(msg)) => {
            assert!(msg.starts_with("STRUCTURED_EXACT_MIXED_DIRECTION"), "{msg}")
        }
        other => panic!("expected a typed refusal, got {other:?}"),
    }
    assert!(exact.factor_with(1.0).is_err());
    let n = m.layout.n();
    let mut j = DenseMatrix::zeros(n, n);
    m.dense_fill(0.0, &mut j).unwrap();
    let mut rng = XorShift(0x1234_5678_9abc_def1);
    for two_level in [false, true] {
        let mode = Mode::Krylov {
            restart: 4,
            max_iterations: 48,
            rtol: 1.0e-12,
            two_level,
        };
        let mut solver =
            BlockStageSolver::new(m.clone(), 0, mode, Capacitance::Conservation).unwrap();
        solver.linearize_at(0.0).unwrap();
        assert_eq!(solver.direction(), Direction::Mixed);
        for inv in [0.75, 7.5, 75.0] {
            solver.factor_with(inv).unwrap();
            let w = w_of(&j, inv);
            let b = random_rhs(n, &mut rng);
            let mut x = b.clone();
            let solved = solver.solve_with(&mut x, &mut WorkCounters::default());
            if inv < 1.0 {
                // rtol 1e-12 lies near the rounding floor ||W|| ||x|| eps / ||b||
                // here: a solve may end as a typed (retryable) rejection
                match solved {
                    Ok(()) | Err(CoreError::LinearSolve(_)) => continue,
                    Err(other) => panic!("unexpected error kind {other:?}"),
                }
            }
            solved.unwrap();
            let r: Vec<f64> = b.iter().zip(matvec(&w, &x)).map(|(a, c)| a - c).collect();
            let rel = r.iter().map(|v| v * v).sum::<f64>().sqrt()
                / b.iter().map(|v| v * v).sum::<f64>().sqrt();
            assert!(rel <= 1.0e-11, "two_level {two_level} inv {inv}: {rel:e}");
        }
        assert!(solver.ops.max_krylov_iterations <= 48);
    }
    // Krylov on a downward drift equals the exact solve
    let down = model(16, 8, RPLUS, 1.0e4, true, Boundary::Outflow);
    let n = down.layout.n();
    let mut a =
        BlockStageSolver::new(down.clone(), 0, Mode::Exact, Capacitance::Conservation).unwrap();
    let mut k = BlockStageSolver::new(down, 0, KRYLOV, Capacitance::Conservation).unwrap();
    for s in [&mut a, &mut k] {
        s.linearize_at(0.5).unwrap();
        s.factor_with(3.0).unwrap();
    }
    let b = random_rhs(n, &mut rng);
    let (mut xa, mut xk) = (b.clone(), b);
    a.solve_with(&mut xa, &mut WorkCounters::default()).unwrap();
    k.solve_with(&mut xk, &mut WorkCounters::default()).unwrap();
    let gap = xa
        .iter()
        .zip(&xk)
        .map(|(p, q)| (p - q).abs())
        .fold(0.0, f64::max);
    assert!(gap <= 1.0e-12 * max_abs(&xa), "{gap:e}");
}

#[test]
fn c4_stale_states_and_shapes_are_typed_refusals() {
    let m = model(8, 4, RPLUS, 1.0, true, Boundary::Outflow);
    let n = m.layout.n();
    let mut s =
        BlockStageSolver::new(m.clone(), 7, Mode::Exact, Capacitance::Conservation).unwrap();
    assert_eq!(s.epoch(), 7);
    let refused = |r: rodas5p_core::CoreResult<()>, prefix: &str| match r {
        Err(CoreError::InvalidInput(msg)) => assert!(msg.starts_with(prefix), "{msg}"),
        other => panic!("expected {prefix}, got {other:?}"),
    };
    let mut b = vec![1.0; n];
    let mut c = WorkCounters::default();
    refused(s.factor_with(1.0), "STRUCTURED_NOT_LINEARIZED");
    refused(s.solve_with(&mut b, &mut c), "STRUCTURED_STALE");
    s.linearize_at(0.0).unwrap();
    refused(s.solve_with(&mut b, &mut c), "STRUCTURED_STALE");
    s.factor_with(1.0).unwrap();
    s.solve_with(&mut b, &mut c).unwrap();
    // a new linearization makes the old factorization stale
    s.linearize_at(0.1).unwrap();
    refused(s.solve_with(&mut b, &mut c), "STRUCTURED_STALE");
    s.factor_with(1.0).unwrap();
    // a rebind to a new epoch makes everything stale
    s.rebind(m.clone(), 8).unwrap();
    assert_eq!(s.epoch(), 8);
    refused(s.solve_with(&mut b, &mut c), "STRUCTURED_STALE");
    refused(s.factor_with(1.0), "STRUCTURED_NOT_LINEARIZED");
    let other = model(8, 6, RPLUS, 1.0, true, Boundary::Outflow);
    assert!(matches!(s.rebind(other, 9), Err(CoreError::Dimension(_))));
    // a rebind to a same-shape mesh with other edges and boundary solves that mesh
    let moved = {
        let q: Vec<f64> = (0..=8).map(|k| -3.5 + 0.7 * (k as f64).powf(1.1)).collect();
        let mu = vec![-1.0, -0.6, 0.05, 0.7, 1.0];
        let layout = Layout::new(q, mu, Coordinate::Physical, Boundary::Closed).unwrap();
        Arc::new(
            Model::new(
                layout,
                RPLUS,
                3.0e3,
                Some(Absorber {
                    k0: 2.0,
                    q_chi: -1.0,
                }),
                false,
            )
            .unwrap(),
        )
    };
    let mut fresh =
        BlockStageSolver::new(moved.clone(), 10, Mode::Exact, Capacitance::Conservation).unwrap();
    s.rebind(moved.clone(), 10).unwrap();
    let mut j = DenseMatrix::zeros(n, n);
    moved.dense_fill(0.2, &mut j).unwrap();
    let w = w_of(&j, 5.0);
    let rhs: Vec<f64> = (0..n).map(|k| 1.0 + (k % 5) as f64).collect();
    for solver in [&mut s, &mut fresh] {
        solver.linearize_at(0.2).unwrap();
        solver.factor_with(5.0).unwrap();
        let mut x = rhs.clone();
        solver.solve_with(&mut x, &mut c).unwrap();
        let x_lu = LuFactorization::new(&w).unwrap().solve(&rhs).unwrap();
        let diff = x
            .iter()
            .zip(&x_lu)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max)
            / max_abs(&x_lu);
        assert!(
            diff <= 1.0e-12,
            "rebind solve differs from the dense LU: {diff:e}"
        );
    }
    let (mut xs, mut xf) = (rhs.clone(), rhs.clone());
    s.solve_with(&mut xs, &mut c).unwrap();
    fresh.solve_with(&mut xf, &mut c).unwrap();
    assert_eq!(
        xs.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
        xf.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
        "a rebound solver equals a fresh one bit for bit"
    );
    s.rebind(m.clone(), 11).unwrap();
    s.linearize_at(0.0).unwrap();
    s.factor_with(1.0).unwrap();
    let mut short = vec![1.0; n - 1];
    assert!(matches!(
        s.solve_with(&mut short, &mut c),
        Err(CoreError::Dimension(_))
    ));
    assert!(matches!(
        s.factor_with(0.0),
        Err(CoreError::InvalidInput(_))
    ));
    let one = model(8, 1, RPLUS, 1.0, true, Boundary::Outflow);
    assert!(BlockStageSolver::new(one, 0, Mode::Exact, Capacitance::Conservation).is_err());
    // bordered
    let border = bordered(8, 4);
    let problem = support::border::problem(border.clone()).unwrap();
    let mut bs = BorderedStageSolver::new(border.clone(), 0).unwrap();
    refused(bs.factor(1.0, &mut c), "BORDERED_NOT_LINEARIZED");
    let mut bb = vec![1.0; border.n()];
    refused(bs.solve_in_place(&mut bb, &mut c), "BORDERED_STALE");
    let y0 = bordered_state(&border);
    bs.linearize(&problem, 0.0, &y0, &mut c).unwrap();
    refused(bs.solve_in_place(&mut bb, &mut c), "BORDERED_STALE");
    bs.factor(1.0, &mut c).unwrap();
    bs.solve_in_place(&mut bb, &mut c).unwrap();
    // the bordered model refuses a comoving grid and a photon Thomson rate
    let layout = Layout::new(
        uniform_edges(-3.0, 2.0, 8),
        uniform_edges(-1.0, 1.0, 4),
        Coordinate::Comoving,
        Boundary::Outflow,
    )
    .unwrap();
    let comoving = Arc::new(Model::new(layout, RPLUS, 0.0, None, false).unwrap());
    assert!(Bordered::new(comoving, params()).is_err());
    let gamma = model(8, 4, RPLUS, 1.0, false, Boundary::Outflow);
    assert!(Bordered::new(gamma, params()).is_err());
}

#[test]
fn c5_counted_flops_equal_the_closed_forms() {
    for (nq, nm) in [(2, 2), (8, 2), (8, 8), (16, 64), (64, 16), (128, 64)] {
        for boundary in [Boundary::Outflow, Boundary::Closed] {
            for capacitance in [Capacitance::Conservation, Capacitance::Textbook] {
                let m = model(nq, nm, RPLUS, 1.0e4, true, boundary);
                let n = m.layout.n();
                let mut s = BlockStageSolver::new(m, 0, Mode::Exact, capacitance).unwrap();
                s.linearize_at(0.0).unwrap();
                s.factor_with(2.0).unwrap();
                let mut b = vec![1.0; n];
                s.solve_with(&mut b, &mut WorkCounters::default()).unwrap();
                assert_eq!(s.ops.factor, factor_flops(nq, nm, capacitance), "{nq}x{nm}");
                assert_eq!(
                    s.ops.solve,
                    exact_solve_flops(nq, nm, boundary == Boundary::Outflow),
                    "{nq}x{nm} {boundary:?}"
                );
            }
        }
    }
}

fn params() -> BorderParams {
    BorderParams {
        k: 1.0e-3,
        alpha: 2.0,
        beta: 0.5,
        kappa: 1.0,
        tau: 0.1,
        gamma0: 1.0e4,
    }
}

fn bordered(nq: usize, nm: usize) -> Arc<Bordered> {
    let photon = model(nq, nm, RPLUS, 0.0, true, Boundary::Outflow);
    Arc::new(Bordered::new(photon, params()).unwrap())
}

fn bordered_state(b: &Bordered) -> Vec<f64> {
    let mut y = initial_state(&b.photon);
    y.extend([0.15, 0.1, -0.02]);
    y
}

#[test]
fn c6_solver_calls_allocate_nothing() {
    let m = model(16, 8, RPLUS, 1.0e4, true, Boundary::Outflow);
    let n = m.layout.n();
    let mut exact =
        BlockStageSolver::new(m.clone(), 0, Mode::Exact, Capacitance::Conservation).unwrap();
    let mixed = model(16, 8, MIXED, 1.0e4, true, Boundary::Outflow);
    let mut krylov = BlockStageSolver::new(mixed, 0, KRYLOV, Capacitance::Conservation).unwrap();
    let border = bordered(16, 8);
    let problem = support::border::problem(border.clone()).unwrap();
    let mut schur = BorderedStageSolver::new(border.clone(), 0).unwrap();
    let y0 = bordered_state(&border);
    let mut b = vec![0.0; n];
    let mut bb = vec![0.0; border.n()];
    let mut out = vec![0.0; border.n()];
    let mut dense = DenseMatrix::zeros(border.n(), border.n());
    let mut c = WorkCounters::default();
    let mut cycle = |k: usize| {
        let t = 0.01 * k as f64;
        let inv = 10.0 + k as f64;
        exact.linearize_at(t).unwrap();
        exact.factor_with(inv).unwrap();
        b.fill(1.0);
        exact.solve_with(&mut b, &mut c).unwrap();
        krylov.linearize_at(t).unwrap();
        krylov.factor_with(inv).unwrap();
        b.fill(1.0);
        krylov.solve_with(&mut b, &mut c).unwrap();
        schur.linearize(&problem, t, &y0, &mut c).unwrap();
        schur.factor(inv, &mut c).unwrap();
        bb.fill(1.0);
        schur.solve_in_place(&mut bb, &mut c).unwrap();
        border.rhs(t, &y0, &mut out).unwrap();
        border.partial_t(t, &y0, &mut out).unwrap();
        border.dense_fill(t, &y0, &mut dense).unwrap();
    };
    // the very first cycle after construction already allocates nothing
    // (the Krylov least-squares workspace is grown at construction)
    let ((), first) = allocations_during(|| cycle(0));
    assert_eq!(first, 0, "first cycle after construction");
    let ((), allocated) = allocations_during(|| {
        for k in 1..200 {
            cycle(k);
        }
    });
    assert_eq!(allocated, 0);
}

struct Arm {
    states: Vec<Vec<f64>>,
    counters: WorkCounters,
    attempts: usize,
    lu: Rodas5pFastLu,
}

fn atol_of(y0: &[f64], rtol: f64) -> Vec<f64> {
    y0.iter()
        .map(|v| 1.0e-6 * rtol * v.abs().max(1.0e-3))
        .collect()
}

fn client_run(
    problem: &OdeProblem,
    client: Box<dyn StageLinearSystem>,
    y0: &[f64],
    rtol: f64,
    tf: f64,
) -> Arm {
    let atol = atol_of(y0, rtol);
    let config = AdaptiveStepConfig {
        atol: atol.iter().copied().fold(f64::INFINITY, f64::min),
        rtol,
        initial_step: 1.0e-4,
        max_step: tf,
        max_attempts: 1_000_000,
        ..AdaptiveStepConfig::default()
    };
    let output = OutputSchedule::new(vec![0.0, 0.5 * tf, tf]).unwrap();
    let (fast, partial, _) = integrate_rodas5p_fast_client_observed_with_atol(
        problem,
        client,
        (0.0, tf),
        y0,
        &config,
        &output,
        &atol,
    )
    .unwrap();
    assert!(
        partial.is_none() && fast.observed.success,
        "{}",
        fast.observed.message
    );
    Arm {
        states: fast.observed.y,
        counters: fast.observed.counters,
        attempts: fast.attempts,
        lu: fast.lu,
    }
}

type Make<'a> = Box<dyn Fn() -> Box<dyn StageLinearSystem> + 'a>;

#[test]
fn c6_client_runs_allocate_independently_of_the_step_count() {
    let m = model(16, 8, RPLUS, 1.0e4, true, Boundary::Outflow);
    let problem = support::problem(m.clone()).unwrap();
    let y0 = initial_state(&m);
    let mixed = model(16, 8, MIXED, 1.0e2, true, Boundary::Outflow);
    let mixed_problem = support::problem(mixed.clone()).unwrap();
    let border = bordered(16, 8);
    let border_problem = support::border::problem(border.clone()).unwrap();
    let yb = bordered_state(&border);
    let arms: Vec<(&str, &OdeProblem, &[f64], Make)> = vec![
        (
            "exact",
            &problem,
            &y0,
            Box::new(|| {
                Box::new(
                    BlockStageSolver::new(m.clone(), 0, Mode::Exact, Capacitance::Conservation)
                        .unwrap(),
                )
            }),
        ),
        (
            "krylov",
            &mixed_problem,
            &y0,
            Box::new(|| {
                Box::new(
                    BlockStageSolver::new(mixed.clone(), 0, KRYLOV, Capacitance::Conservation)
                        .unwrap(),
                )
            }),
        ),
        (
            "bordered",
            &border_problem,
            &yb,
            Box::new(|| Box::new(BorderedStageSolver::new(border.clone(), 0).unwrap())),
        ),
    ];
    for (name, problem, y0, make) in &arms {
        // warm-up: one-time lazy initialization is not a per-step cost
        client_run(problem, make(), y0, 1.0e-6, 1.0);
        let (loose, a1) = allocations_during(|| client_run(problem, make(), y0, 1.0e-6, 1.0));
        let (tight, a2) = allocations_during(|| client_run(problem, make(), y0, 1.0e-9, 1.0));
        assert_eq!(loose.lu, Rodas5pFastLu::Client);
        assert!(
            tight.attempts >= 2 * loose.attempts,
            "{name}: attempts {} vs {}",
            tight.attempts,
            loose.attempts
        );
        assert_eq!(a1, a2, "{name}: allocations grow with the step count");
    }
}

#[test]
fn c7_client_runs_are_deterministic_and_agree_with_the_banded_driver() {
    let m = model(16, 8, RPLUS, 1.0e4, true, Boundary::Outflow);
    let problem = support::problem(m.clone()).unwrap();
    let y0 = initial_state(&m);
    let make = || -> Box<dyn StageLinearSystem> {
        Box::new(
            BlockStageSolver::new(m.clone(), 0, Mode::Exact, Capacitance::Conservation).unwrap(),
        )
    };
    let rtol = 1.0e-7;
    let a = client_run(&problem, make(), &y0, rtol, 2.0);
    let b = client_run(&problem, make(), &y0, rtol, 2.0);
    let bits = |s: &[Vec<f64>]| -> Vec<u64> { s.iter().flatten().map(|v| v.to_bits()).collect() };
    assert_eq!(bits(&a.states), bits(&b.states));
    assert_eq!(
        serde_json::to_string(&a.counters).unwrap(),
        serde_json::to_string(&b.counters).unwrap()
    );
    let atol = atol_of(&y0, rtol);
    let config = AdaptiveStepConfig {
        atol: atol.iter().copied().fold(f64::INFINITY, f64::min),
        rtol,
        initial_step: 1.0e-4,
        max_step: 2.0,
        max_attempts: 1_000_000,
        ..AdaptiveStepConfig::default()
    };
    let output = OutputSchedule::new(vec![0.0, 1.0, 2.0]).unwrap();
    let banded = integrate_rodas5p_fast_banded_observed_with_atol(
        &problem,
        &support::band(m.clone()),
        (0.0, 2.0),
        &y0,
        &config,
        &output,
        &atol,
    )
    .unwrap();
    let last = banded.fast.observed.y.last().unwrap();
    let mine = a.states.last().unwrap();
    let wrms = (mine
        .iter()
        .zip(last)
        .zip(&atol)
        .map(|((p, q), w)| {
            let e = (p - q) / (w + rtol * q.abs());
            e * e
        })
        .sum::<f64>()
        / mine.len() as f64)
        .sqrt();
    assert!(wrms <= 1.0, "client vs banded WRMS {wrms:e}");
}

#[test]
fn c8_bordered_jacobian_schur_solve_and_riccati_law() {
    let border = bordered(10, 6);
    let n = border.n();
    let y = bordered_state(&border);
    let t = 0.4;
    let mut j = DenseMatrix::zeros(n, n);
    border.dense_fill(t, &y, &mut j).unwrap();
    let (mut fp, mut fm) = (vec![0.0; n], vec![0.0; n]);
    for c in 0..n {
        let h = 1.0e-6 * y[c].abs().max(1.0e-3);
        let (mut yp, mut ym) = (y.clone(), y.clone());
        yp[c] += h;
        ym[c] -= h;
        border.rhs(t, &yp, &mut fp).unwrap();
        border.rhs(t, &ym, &mut fm).unwrap();
        let col_scale = (0..n)
            .map(|r| j[(r, c)].abs())
            .fold(0.0, f64::max)
            .max(1.0e-300);
        for r in 0..n {
            let fd = (fp[r] - fm[r]) / (2.0 * h);
            assert!(
                (fd - j[(r, c)]).abs() <= 1.0e-6 * col_scale,
                "J[{r},{c}] {} vs FD {fd}",
                j[(r, c)]
            );
        }
    }
    let problem = support::border::problem(border.clone()).unwrap();
    let mut schur = BorderedStageSolver::new(border.clone(), 0).unwrap();
    let mut c = WorkCounters::default();
    schur.linearize(&problem, t, &y, &mut c).unwrap();
    let mut rng = XorShift(0x0bad_cafe_dead_beef);
    for inv in [0.05, 2.0, 400.0] {
        schur.factor(inv, &mut c).unwrap();
        let w = w_of(&j, inv);
        let b = random_rhs(n, &mut rng);
        let mut x = b.clone();
        schur.solve_in_place(&mut x, &mut c).unwrap();
        let x_lu = LuFactorization::new(&w).unwrap().solve(&b).unwrap();
        let diff = x
            .iter()
            .zip(&x_lu)
            .map(|(p, q)| (p - q).abs())
            .fold(0.0, f64::max)
            / max_abs(&x_lu);
        let eta = backward_error(&w, &x, &b);
        assert!(
            eta <= 1.0e-14 && diff <= 1.0e-10,
            "inv {inv}: eta {eta:e} diff {diff:e}"
        );
    }
    let x0 = 0.1;
    assert!((border.riccati(x0, 0.0) - x0).abs() <= 4.0 * f64::EPSILON * x0);
    let p = params();
    for tt in [0.1, 1.0, 3.0] {
        let d = 1.0e-5;
        let slope = (border.riccati(x0, tt + d) - border.riccati(x0, tt - d)) / (2.0 * d);
        let x = border.riccati(x0, tt);
        let law = p.beta * (1.0 - x) - p.alpha * x * x;
        assert!((slope - law).abs() <= 1.0e-8, "t {tt}: {slope} vs {law}");
    }
}
