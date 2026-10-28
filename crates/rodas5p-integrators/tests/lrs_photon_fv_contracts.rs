//! Contracts of the fixed-grid LRS photon transport adapter (BASS/rei DAG
//! node P02, `examples/lrs_photon_fv_support`) and its runs through the
//! RODAS5P fast drivers:
//!
//! * C2: every column of the in-place dense Jacobian is the RHS of the unit
//!   vector (the operator is linear: `f(2v) = 2 f(v)` bit for bit and
//!   `J v = f(v)` to rounding), the packed band (through the model and
//!   through the `support::band` declaration the banded driver uses, whose
//!   widths are `lower = upper = nm + 1`) is the dense matrix on the band
//!   bit for bit, nothing lies outside the band, and a refill over a used
//!   or poisoned matrix equals a fresh fill.
//! * C3: the semi-discrete number ledger. The sum of all components of the
//!   RHS vanishes to rounding (outflow, absorber and Thomson), and a closed
//!   grid without absorber leaves every ledger slot exactly zero. The bound
//!   is `1e-14 A` with `A = sum_{r,c} |J_rc y_c|`, the sum of the absolute
//!   values of every term the RHS accumulates (a flux counts once at each of
//!   the two slots it connects): accumulation rounding is a small multiple
//!   of `eps A`, while `sum_i |f_i|` can be smaller than `A` by cancellation
//!   of inflow and outflow. The total is summed with Neumaier compensation
//!   so the test's own summation does not enter. States are positive (number
//!   densities).
//! * C8 (VG-1): on the non-autonomous scenario the dense and banded fast
//!   drivers allocate the same count at two tolerances whose attempt counts
//!   differ at least twice, and the callbacks allocate nothing.
//! * C9 (VG-3): repeated runs are bit for bit identical, a uniform vector
//!   atol is the scalar run bit for bit, and the dense and banded arms'
//!   final states agree with WRMS <= 1 (weights `atol_i + rtol |y_i|`).
//! * C10: invalid declarations and shapes are typed refusals.
//! * Sanity: the exact `f_t` against central differences away from kinks
//!   and against the right-sided difference at an exact threshold tie (the
//!   tie rule follows the threshold's motion; right-continuous, a jump from
//!   the left), the closed-form absorber cell average against quadrature,
//!   the negative control `drop_partial_t`, the JVP callback and the
//!   autonomy flag.

use std::alloc::{GlobalAlloc, Layout as AllocLayout, System};
use std::cell::Cell;
use std::sync::Arc;

use rodas5p_core::{CoreError, CoreResult, DenseMatrix, WorkCounters};
use rodas5p_integrators::{
    AdaptiveStepConfig, OutputSchedule, Rodas5pFastBandedResult, Rodas5pFastLu, Rodas5pFastResult,
    integrate_rodas5p_fast_banded_observed, integrate_rodas5p_fast_banded_observed_with_atol,
    integrate_rodas5p_fast_observed, integrate_rodas5p_fast_observed_with_atol,
};

#[path = "../examples/lrs_photon_fv_support/mod.rs"]
mod support;
use support::{Absorber, Boundary, Coordinate, Geometry, Layout, Model};

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

/// Allocations made by this thread during `f` (the drivers and callbacks
/// measured here run on the calling thread).
fn allocations_during<T>(f: impl FnOnce() -> T) -> (T, usize) {
    let before = ALLOCATIONS.with(Cell::get);
    let value = f();
    (value, ALLOCATIONS.with(Cell::get) - before)
}

const EPS: f64 = f64::EPSILON;

/// Deterministic xorshift64 (no external crate).
struct XorShift(u64);

impl XorShift {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Uniform in `[0, 1)`.
    fn uniform(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

fn max_abs(v: &[f64]) -> f64 {
    v.iter().fold(0.0_f64, |a, x| a.max(x.abs()))
}

/// Neumaier-compensated sum.
fn compensated_sum(v: &[f64]) -> f64 {
    let (mut s, mut c) = (0.0_f64, 0.0_f64);
    for &x in v {
        let t = s + x;
        if s.abs() >= x.abs() {
            c += (s - t) + x;
        } else {
            c += (x - t) + s;
        }
        s = t;
    }
    s + c
}

fn uniform_edges(a: f64, b: f64, cells: usize) -> Vec<f64> {
    (0..=cells)
        .map(|k| a + (b - a) * k as f64 / cells as f64)
        .collect()
}

struct Grid {
    name: &'static str,
    q: Vec<f64>,
    mu: Vec<f64>,
}

impl Grid {
    /// Midpoint of the middle energy cell (a threshold inside a cell).
    fn mid(&self) -> f64 {
        let i = (self.q.len() - 1) / 2;
        0.5 * (self.q[i] + self.q[i + 1])
    }

    fn layout(&self, coordinate: Coordinate, boundary: Boundary) -> Layout {
        Layout::new(self.q.clone(), self.mu.clone(), coordinate, boundary).unwrap()
    }
}

fn random_grid(nq: usize, nm: usize, seed: u64) -> Grid {
    let mut rng = XorShift::new(seed);
    let mut q = vec![-1.5];
    for _ in 0..nq {
        let last = q[q.len() - 1];
        q.push(last + 0.1 + 0.7 * rng.uniform());
    }
    let widths: Vec<f64> = (0..nm).map(|_| 0.2 + rng.uniform()).collect();
    let total: f64 = widths.iter().sum();
    let mut mu = vec![-1.0];
    let mut acc = 0.0;
    for w in &widths[..nm - 1] {
        acc += w;
        mu.push(-1.0 + 2.0 * acc / total);
    }
    mu.push(1.0);
    Grid {
        name: "random6x5",
        q,
        mu,
    }
}

fn grids() -> [Grid; 3] {
    [
        Grid {
            name: "oracle",
            q: vec![0.0, 0.2, 0.9, 2.0],
            mu: vec![-1.0, -0.7, 0.1, 0.8, 1.0],
        },
        Grid {
            name: "uniform16x8",
            q: uniform_edges(-3.0, 2.0, 16),
            mu: uniform_edges(-1.0, 1.0, 8),
        },
        random_grid(6, 5, 0x0bad_5eed_0002),
    ]
}

const GEOMETRIES: [Geometry; 3] = [
    Geometry::Constant { h: 0.3, s: 0.2 },
    Geometry::Constant { h: 0.1, s: -0.25 },
    Geometry::PowerLaw {
        hh: 0.5,
        s0: 0.2,
        p: 1.5,
    },
];

const COORDINATES: [(Coordinate, Boundary); 3] = [
    (Coordinate::Physical, Boundary::Outflow),
    (Coordinate::Physical, Boundary::Closed),
    (Coordinate::Comoving, Boundary::Outflow),
];

fn model_on(
    grid: &Grid,
    coordinate: Coordinate,
    boundary: Boundary,
    geometry: Geometry,
    gamma: f64,
    absorber: Option<Absorber>,
) -> Model {
    Model::new(
        grid.layout(coordinate, boundary),
        geometry,
        gamma,
        absorber,
        false,
    )
    .unwrap()
}

/// Dense `J (t)` from a fresh zero matrix.
fn fresh_dense(model: &Model, t: f64) -> DenseMatrix {
    let n = model.layout.n();
    let mut m = DenseMatrix::zeros(n, n);
    model.dense_fill(t, &mut m).unwrap();
    m
}

fn positive_state(rng: &mut XorShift, n: usize) -> Vec<f64> {
    (0..n).map(|_| 0.01 + rng.uniform()).collect()
}

#[test]
fn c2_dense_columns_are_unit_rhs_and_the_band_is_exact() {
    let mut rng = XorShift::new(0x00c2_c2c2);
    let mut cases = 0usize;
    let mut columns = 0usize;
    let mut inexact_columns = 0usize;
    let mut max_column_rel = 0.0_f64;
    let mut max_linear_rel = 0.0_f64;
    for grid in grids() {
        let mut reused: Option<DenseMatrix> = None;
        let mid = grid.mid();
        let absorbers = [
            None,
            Some(Absorber {
                k0: 2.0,
                q_chi: -0.5,
            }),
            Some(Absorber {
                k0: 2.0,
                q_chi: mid,
            }),
        ];
        for (coordinate, boundary) in COORDINATES {
            for gamma in [0.0, 3.0] {
                for absorber in absorbers {
                    for geometry in GEOMETRIES {
                        for t in [0.0, 0.7] {
                            let label = format!(
                                "{} {coordinate:?} {boundary:?} gamma={gamma} {absorber:?} \
                                 {geometry:?} t={t}",
                                grid.name
                            );
                            let model = Arc::new(model_on(
                                &grid, coordinate, boundary, geometry, gamma, absorber,
                            ));
                            let n = model.layout.n();
                            let bw = model.layout.bandwidth();
                            assert_eq!(bw, model.layout.nm + 1);
                            let declared = support::band(model.clone());
                            assert_eq!(
                                (declared.lower, declared.upper),
                                (model.layout.nm + 1, model.layout.nm + 1),
                                "{label}: declared band widths"
                            );
                            let dense = fresh_dense(&model, t);

                            // Column c is f(t, e_c).
                            let mut e = vec![0.0; n];
                            let mut f = vec![0.0; n];
                            for c in 0..n {
                                e[c] = 1.0;
                                model.rhs(t, &e, &mut f).unwrap();
                                e[c] = 0.0;
                                let scale = max_abs(&f).max(f64::MIN_POSITIVE);
                                let mut bitwise = true;
                                for (r, fr) in f.iter().enumerate() {
                                    let dev = (fr - dense[(r, c)]).abs();
                                    assert!(
                                        dev <= 2.0 * EPS * scale,
                                        "{label}: J[{r},{c}]={} vs f(e_c)={fr}",
                                        dense[(r, c)]
                                    );
                                    max_column_rel = max_column_rel.max(dev / scale);
                                    bitwise &= *fr == dense[(r, c)];
                                }
                                columns += 1;
                                if !bitwise {
                                    inexact_columns += 1;
                                }
                            }

                            // Nothing outside the band.
                            for r in 0..n {
                                for c in 0..n {
                                    if r.abs_diff(c) > bw {
                                        assert!(
                                            dense[(r, c)] == 0.0,
                                            "{label}: J[{r},{c}] outside the band"
                                        );
                                    }
                                }
                            }

                            // The packed band is the dense matrix on the band,
                            // through the model and through the declaration
                            // the banded driver uses (zeroed buffer, as the
                            // driver does; the state argument is unused).
                            let width = 2 * bw + 1;
                            let mut packed = vec![0.0; n * width];
                            model.band_fill(t, &mut packed).unwrap();
                            let mut via_declaration = vec![0.0; n * width];
                            let y_any: Vec<f64> = (0..n).map(|k| 1.0 + k as f64).collect();
                            (declared.fill)(t, &y_any, &mut via_declaration).unwrap();
                            for (what, buf) in
                                [("model", &packed), ("declaration", &via_declaration)]
                            {
                                for r in 0..n {
                                    for d in 0..width {
                                        let c = r as isize + d as isize - bw as isize;
                                        let v = buf[r * width + d];
                                        if c < 0 || c >= n as isize {
                                            assert!(
                                                v.to_bits() == 0,
                                                "{label}: {what} band slot ({r},{d}) off matrix"
                                            );
                                        } else {
                                            assert_eq!(
                                                v.to_bits(),
                                                dense[(r, c as usize)].to_bits(),
                                                "{label}: {what} band ({r},{c})"
                                            );
                                        }
                                    }
                                }
                            }

                            // Linearity: f(2v) = 2 f(v) exactly, J v = f(v) to
                            // rounding (a row's bound: 4 n eps sum_c |J_rc v_c|).
                            let v: Vec<f64> = (0..n).map(|_| rng.uniform() - 0.3).collect();
                            let v2: Vec<f64> = v.iter().map(|x| 2.0 * x).collect();
                            let mut fv = vec![0.0; n];
                            let mut f2v = vec![0.0; n];
                            model.rhs(t, &v, &mut fv).unwrap();
                            model.rhs(t, &v2, &mut f2v).unwrap();
                            let doubled: Vec<f64> = fv.iter().map(|x| 2.0 * x).collect();
                            assert_eq!(bits(&f2v), bits(&doubled), "{label}: homogeneity");
                            let jv = dense.matvec(&v).unwrap();
                            for r in 0..n {
                                let abs: f64 = (0..n).map(|c| (dense[(r, c)] * v[c]).abs()).sum();
                                let dev = (jv[r] - fv[r]).abs();
                                assert!(
                                    dev <= 4.0 * n as f64 * EPS * abs,
                                    "{label}: (Jv)[{r}]={} f(v)={}",
                                    jv[r],
                                    fv[r]
                                );
                                if abs > 0.0 {
                                    max_linear_rel = max_linear_rel.max(dev / abs);
                                }
                            }

                            // A refill over the previous case's matrix (another
                            // time and geometry sign), then over a band poisoned
                            // with NaN, equals the fresh fill.
                            let target = reused.get_or_insert_with(|| DenseMatrix::zeros(n, n));
                            model.dense_fill(t, target).unwrap();
                            assert_eq!(
                                bits(target.as_slice()),
                                bits(dense.as_slice()),
                                "{label}: refill"
                            );
                            for r in 0..n {
                                for c in r.saturating_sub(bw)..(r + bw + 1).min(n) {
                                    target[(r, c)] = f64::NAN;
                                }
                            }
                            model.dense_fill(t, target).unwrap();
                            assert_eq!(
                                bits(target.as_slice()),
                                bits(dense.as_slice()),
                                "{label}: poisoned refill"
                            );
                            cases += 1;
                        }
                    }
                }
            }
        }
    }

    // The explicit sign-flip case: a matrix filled at t = 0 with s > 0 and
    // refilled at t = 0.7 with s < 0 equals the fresh s < 0 fill.
    let grid = &grids()[1];
    let plus = model_on(
        grid,
        Coordinate::Comoving,
        Boundary::Outflow,
        Geometry::Constant { h: 0.3, s: 0.2 },
        3.0,
        Some(Absorber {
            k0: 2.0,
            q_chi: grid.mid(),
        }),
    );
    let minus = model_on(
        grid,
        Coordinate::Comoving,
        Boundary::Outflow,
        Geometry::Constant { h: 0.1, s: -0.25 },
        3.0,
        Some(Absorber {
            k0: 2.0,
            q_chi: grid.mid(),
        }),
    );
    let mut m = fresh_dense(&plus, 0.0);
    minus.dense_fill(0.7, &mut m).unwrap();
    assert_eq!(
        bits(m.as_slice()),
        bits(fresh_dense(&minus, 0.7).as_slice())
    );
    assert_ne!(bits(fresh_dense(&plus, 0.0).as_slice()), bits(m.as_slice()));

    eprintln!(
        "C2: {cases} cases, {columns} columns, {inexact_columns} not bitwise; \
         max column rel dev {max_column_rel:e}, max J v vs f(v) rel dev {max_linear_rel:e}"
    );
}

/// `A = sum_{r,c} |J_rc y_c|` at `t`.
fn absolute_terms(model: &Model, t: f64, y: &[f64]) -> f64 {
    let j = fresh_dense(model, t);
    let n = y.len();
    let mut a = 0.0;
    for r in 0..n {
        for (c, yc) in y.iter().enumerate() {
            a += (j[(r, c)] * yc).abs();
        }
    }
    a
}

#[test]
fn c3_rhs_conserves_the_number_ledger() {
    let mut rng = XorShift::new(0x00c3_c3c3);
    let mut states = 0usize;
    let (mut worst_open_a, mut worst_open_f) = (0.0_f64, 0.0_f64);
    let (mut worst_closed_a, mut worst_closed_f) = (0.0_f64, 0.0_f64);
    for grid in grids() {
        for geometry in GEOMETRIES {
            for coordinate in [Coordinate::Physical, Coordinate::Comoving] {
                let open = model_on(
                    &grid,
                    coordinate,
                    Boundary::Outflow,
                    geometry,
                    3.0,
                    Some(Absorber {
                        k0: 2.0,
                        q_chi: grid.mid(),
                    }),
                );
                let n = open.layout.n();
                let mut f = vec![0.0; n];
                for _ in 0..100 {
                    let t = 2.0 * rng.uniform();
                    let y = positive_state(&mut rng, n);
                    open.rhs(t, &y, &mut f).unwrap();
                    let total = compensated_sum(&f).abs();
                    let a = absolute_terms(&open, t, &y);
                    let l1: f64 = f.iter().map(|x| x.abs()).sum();
                    assert!(
                        total <= 1.0e-14 * a,
                        "{} {coordinate:?} {geometry:?} t={t}: |sum f|={total:e}, A={a:e}",
                        grid.name
                    );
                    worst_open_a = worst_open_a.max(total / a);
                    worst_open_f = worst_open_f.max(total / l1);
                    states += 1;
                }
                for gamma in [0.0, 3.0] {
                    let closed =
                        model_on(&grid, coordinate, Boundary::Closed, geometry, gamma, None);
                    let l = &closed.layout;
                    for _ in 0..100 {
                        let t = 2.0 * rng.uniform();
                        let y = positive_state(&mut rng, n);
                        closed.rhs(t, &y, &mut f).unwrap();
                        assert!(f[Layout::LO] == 0.0 && f[l.hi()] == 0.0);
                        for i in 0..l.nq {
                            assert!(f[l.absorbed(i)] == 0.0);
                        }
                        let total = compensated_sum(&f).abs();
                        let a = absolute_terms(&closed, t, &y);
                        let l1: f64 = f.iter().map(|x| x.abs()).sum();
                        assert!(
                            total <= 1.0e-14 * a,
                            "{} closed {coordinate:?} {geometry:?} gamma={gamma} t={t}: \
                             |sum f|={total:e}, A={a:e}",
                            grid.name
                        );
                        worst_closed_a = worst_closed_a.max(total / a);
                        if l1 > 0.0 {
                            worst_closed_f = worst_closed_f.max(total / l1);
                        }
                        states += 1;
                    }
                }
            }
        }
    }
    eprintln!(
        "C3: {states} states; outflow max |sum f|/A {worst_open_a:e} (/sum|f| {worst_open_f:e}); \
         closed max |sum f|/A {worst_closed_a:e} (/sum|f| {worst_closed_f:e})"
    );
}

// The non-autonomous scenario of C8/C9: comoving 16x8 grid, power-law
// geometry, Thomson, a moving absorption threshold.
const SPAN: (f64, f64) = (0.0, 3.0);

fn scenario() -> (Arc<Model>, Vec<f64>) {
    let layout = Layout::new(
        uniform_edges(-3.0, 2.0, 16),
        uniform_edges(-1.0, 1.0, 8),
        Coordinate::Comoving,
        Boundary::Outflow,
    )
    .unwrap();
    let model = Model::new(
        layout,
        Geometry::PowerLaw {
            hh: 0.5,
            s0: 0.2,
            p: 1.5,
        },
        0.5,
        Some(Absorber {
            k0: 0.5,
            q_chi: -0.5,
        }),
        false,
    )
    .unwrap();
    let l = &model.layout;
    let mut y0 = vec![0.0; l.n()];
    for i in 0..l.nq {
        let q = 0.5 * (l.q_edges[i] + l.q_edges[i + 1]);
        for j in 0..l.nm {
            let mu = 0.5 * (l.mu_edges[j] + l.mu_edges[j + 1]);
            y0[l.cell(i, j)] =
                l.dq[i] * l.dm[j] * (-(q * q) / (2.0 * 0.36)).exp() * (1.0 + 0.3 * mu);
        }
    }
    (Arc::new(model), y0)
}

/// Nonuniform vector atol: `scale` times the cell measure on cells, `scale`
/// on the ledger slots (`L_lo`, `L_hi`, `A_i`; larger than any cell's).
fn scenario_atol(model: &Model, scale: f64) -> Vec<f64> {
    let l = &model.layout;
    let mut atol = vec![scale; l.n()];
    for i in 0..l.nq {
        for j in 0..l.nm {
            atol[l.cell(i, j)] = scale * l.dq[i] * l.dm[j];
        }
    }
    atol
}

fn scenario_config(rtol: f64, atol: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol,
        rtol,
        initial_step: 1.0e-4,
        min_step: 1.0e-14,
        max_attempts: 1_000_000,
        ..AdaptiveStepConfig::default()
    }
}

fn schedule() -> OutputSchedule {
    OutputSchedule::new(vec![0.0, 1.5, 3.0]).unwrap()
}

#[test]
fn c8_drivers_allocate_independently_of_the_step_count() {
    let (model, y0) = scenario();
    let problem = support::problem(model.clone()).unwrap();
    assert!(!problem.autonomous);
    let band = support::band(model.clone());
    let output = schedule();
    let n = model.layout.n();
    let distinct = scenario_atol(&model, 1.0);
    assert!(distinct.iter().any(|a| *a != distinct[0]));

    let dense_run = |rtol: f64| {
        let atol = scenario_atol(&model, 1.0e-2 * rtol);
        let config = scenario_config(rtol, 1.0e-2 * rtol);
        allocations_during(|| {
            integrate_rodas5p_fast_observed_with_atol(&problem, SPAN, &y0, &config, &output, &atol)
                .unwrap()
        })
    };
    let banded_run = |rtol: f64| {
        let atol = scenario_atol(&model, 1.0e-2 * rtol);
        let config = scenario_config(rtol, 1.0e-2 * rtol);
        allocations_during(|| {
            integrate_rodas5p_fast_banded_observed_with_atol(
                &problem, &band, SPAN, &y0, &config, &output, &atol,
            )
            .unwrap()
        })
    };
    dense_run(1.0e-6); // warm the coefficient cache and lazy statics
    banded_run(1.0e-6);
    let mut dense = Vec::new();
    let mut banded = Vec::new();
    for rtol in [1.0e-6, 1.0e-9] {
        let ((fast, partial), allocations) = dense_run(rtol);
        assert!(
            fast.observed.success && partial.is_none(),
            "dense rtol={rtol:e}"
        );
        assert_eq!(fast.lu, Rodas5pFastLu::InPlaceZeroSkipping);
        assert_eq!(fast.observed.y.len(), 3);
        assert!(fast.observed.y.iter().all(|y| y.len() == n));
        dense.push((
            fast.attempts,
            fast.rejected_steps,
            fast.observed.counters.ft_calls,
            allocations,
        ));
        let (result, allocations) = banded_run(rtol);
        assert!(
            result.fast.observed.success && result.partial.is_none(),
            "banded rtol={rtol:e}"
        );
        assert_eq!(result.fast.lu, Rodas5pFastLu::Banded);
        banded.push((
            result.fast.attempts,
            result.fast.rejected_steps,
            result.fast.observed.counters.ft_calls,
            allocations,
        ));
    }
    for (name, runs) in [("dense", &dense), ("banded", &banded)] {
        assert!(runs[1].0 >= 2 * runs[0].0, "{name}: {runs:?}");
        assert!(
            runs[1].2 > runs[0].2,
            "{name}: f_t must be evaluated per step: {runs:?}"
        );
        assert_eq!(
            runs[0].3, runs[1].3,
            "{name}: allocations depend on the step count: {runs:?}"
        );
    }
    eprintln!(
        "C8 drivers (attempts, rejected, ft_calls, allocations) at rtol 1e-6, 1e-9: \
         dense {dense:?}, banded {banded:?}"
    );
}

#[test]
fn c8_callbacks_allocate_nothing() {
    let (model, y0) = scenario();
    let problem = support::problem(model.clone()).unwrap();
    let jvp = problem.jvp_callback().unwrap().clone();
    let declared = support::band(model.clone());
    let n = model.layout.n();
    let bw = model.layout.bandwidth();
    let mut out = vec![0.0; n];
    let mut dense = DenseMatrix::zeros(n, n);
    let mut packed = vec![0.0; n * (2 * bw + 1)];
    let mut counters = WorkCounters::default();
    let v: Vec<f64> = (0..n).map(|k| (k as f64).sin()).collect();
    let time = |k: usize| 3.0 * k as f64 / 1000.0;
    let mut counts = Vec::new();
    let (_, a) = allocations_during(|| {
        for k in 0..1000 {
            model.rhs(time(k), &y0, &mut out).unwrap();
        }
    });
    counts.push(("rhs", a));
    let (_, a) = allocations_during(|| {
        for k in 0..1000 {
            model.partial_t(time(k), &y0, &mut out).unwrap();
        }
    });
    counts.push(("partial_t", a));
    let (_, a) = allocations_during(|| {
        for k in 0..1000 {
            jvp(time(k), &y0, &v, &mut out).unwrap();
        }
    });
    counts.push(("problem jvp", a));
    let (_, a) = allocations_during(|| {
        for k in 0..1000 {
            problem
                .eval_partial_t_into(time(k), &y0, &mut out, &mut counters)
                .unwrap();
        }
    });
    counts.push(("problem eval_partial_t_into", a));
    let (_, a) = allocations_during(|| {
        for k in 0..1000 {
            model.dense_fill(time(k), &mut dense).unwrap();
        }
    });
    counts.push(("dense_fill", a));
    let (_, a) = allocations_during(|| {
        for k in 0..1000 {
            packed.fill(0.0);
            model.band_fill(time(k), &mut packed).unwrap();
        }
    });
    counts.push(("band_fill", a));
    let (_, a) = allocations_during(|| {
        for k in 0..1000 {
            packed.fill(0.0);
            (declared.fill)(time(k), &y0, &mut packed).unwrap();
        }
    });
    counts.push(("declared band fill", a));
    // Positive control: the counter sees an allocation on this thread.
    let (_, a) = allocations_during(|| std::hint::black_box(vec![0.0_f64; n]));
    assert!(a >= 1, "the counting allocator is not active");
    for (name, a) in &counts {
        assert_eq!(*a, 0, "{name} allocated {a} times in 1000 calls");
    }
    eprintln!("C8 callbacks (1000 calls each): {counts:?}; positive control {a}");
}

fn assert_same_fast(a: &Rodas5pFastResult, b: &Rodas5pFastResult, what: &str) {
    assert!(a.observed.success && b.observed.success, "{what}");
    assert_eq!(bits(&a.observed.t), bits(&b.observed.t), "{what}: times");
    assert_eq!(a.observed.y.len(), b.observed.y.len(), "{what}");
    for (k, (x, y)) in a.observed.y.iter().zip(&b.observed.y).enumerate() {
        assert_eq!(bits(x), bits(y), "{what}: output {k}");
    }
    assert_eq!(a.observed.counters, b.observed.counters, "{what}: counters");
    assert_eq!(
        a.observed.internal_steps, b.observed.internal_steps,
        "{what}"
    );
    assert_eq!(
        a.observed.output_clipped_steps, b.observed.output_clipped_steps,
        "{what}"
    );
    assert_eq!(a.attempts, b.attempts, "{what}");
    assert_eq!(a.accepted_steps, b.accepted_steps, "{what}");
    assert_eq!(a.rejected_steps, b.rejected_steps, "{what}");
    assert_eq!(a.jacobian_reuses, b.jacobian_reuses, "{what}");
    assert_eq!(a.lu, b.lu, "{what}");
    assert_eq!(a.driver, b.driver, "{what}");
}

fn assert_same_banded(a: &Rodas5pFastBandedResult, b: &Rodas5pFastBandedResult, what: &str) {
    assert_same_fast(&a.fast, &b.fast, what);
    assert_eq!(a.work, b.work, "{what}: banded work");
    assert_eq!(a.partial, b.partial, "{what}: partial");
}

#[test]
fn c9_runs_are_deterministic_and_uniform_vector_atol_is_scalar() {
    let (model, y0) = scenario();
    let problem = support::problem(model.clone()).unwrap();
    let band = support::band(model.clone());
    let output = schedule();
    let n = model.layout.n();
    let rtol = 1.0e-8;
    let config = scenario_config(rtol, 1.0e-10);
    let atol = scenario_atol(&model, 1.0e-10);

    let dense_vector = || {
        integrate_rodas5p_fast_observed_with_atol(&problem, SPAN, &y0, &config, &output, &atol)
            .unwrap()
    };
    let (a, pa) = dense_vector();
    let (b, pb) = dense_vector();
    assert!(pa.is_none() && pb.is_none());
    assert_same_fast(&a, &b, "dense repeat");
    let banded_vector = || {
        integrate_rodas5p_fast_banded_observed_with_atol(
            &problem, &band, SPAN, &y0, &config, &output, &atol,
        )
        .unwrap()
    };
    let (ba, bb) = (banded_vector(), banded_vector());
    assert_same_banded(&ba, &bb, "banded repeat");

    // Uniform vector atol equal to the scalar.
    let uniform = vec![config.atol; n];
    let scalar = integrate_rodas5p_fast_observed(&problem, SPAN, &y0, &config, &output).unwrap();
    let (vector, partial) =
        integrate_rodas5p_fast_observed_with_atol(&problem, SPAN, &y0, &config, &output, &uniform)
            .unwrap();
    assert!(partial.is_none());
    assert_same_fast(&scalar, &vector, "dense uniform vector vs scalar");
    let dense_scalar_attempts = scalar.attempts;
    let scalar =
        integrate_rodas5p_fast_banded_observed(&problem, &band, SPAN, &y0, &config, &output)
            .unwrap();
    let vector = integrate_rodas5p_fast_banded_observed_with_atol(
        &problem, &band, SPAN, &y0, &config, &output, &uniform,
    )
    .unwrap();
    assert_same_banded(&scalar, &vector, "banded uniform vector vs scalar");

    // The two linear algebra arms agree to the tolerance at the final
    // output: WRMS of the difference with the run's weights
    // `atol_i + rtol |y_dense,i|` is at most one.
    assert_eq!(a.observed.y.len(), 3);
    assert_eq!(ba.fast.observed.y.len(), 3);
    assert_eq!(bits(&a.observed.t), bits(&ba.fast.observed.t));
    let (yd, yb) = (&a.observed.y[2], &ba.fast.observed.y[2]);
    assert!(yd.len() == n && yb.len() == n);
    let wrms = (yd
        .iter()
        .zip(yb)
        .zip(&atol)
        .map(|((x, y), at)| ((x - y) / (at + rtol * x.abs())).powi(2))
        .sum::<f64>()
        / n as f64)
        .sqrt();
    assert!(
        wrms.is_finite() && wrms <= 1.0,
        "C9: dense vs banded final WRMS {wrms:e} > 1"
    );
    let dev = yd
        .iter()
        .zip(yb)
        .fold(0.0_f64, |m, (x, y)| m.max((x - y).abs()));
    eprintln!(
        "C9: vector-atol attempts dense {} banded {}; scalar-atol attempts dense {} banded {}; \
         dense vs banded final WRMS {wrms:e}, max abs dev {dev:e}, invariant drift dense {:e}",
        a.attempts,
        ba.fast.attempts,
        dense_scalar_attempts,
        scalar.fast.attempts,
        (Model::invariant(yd) - Model::invariant(&y0)).abs()
    );
}

fn invalid_input<T>(r: CoreResult<T>) -> bool {
    matches!(r, Err(CoreError::InvalidInput(_)))
}

fn dimension<T>(r: CoreResult<T>) -> bool {
    matches!(r, Err(CoreError::Dimension(_)))
}

#[test]
fn c10_invalid_declarations_and_shapes_are_typed_refusals() {
    let q = || vec![0.0, 0.5, 1.0];
    let mu = || vec![-1.0, 0.0, 1.0];
    let layout =
        |q: Vec<f64>, mu: Vec<f64>| Layout::new(q, mu, Coordinate::Physical, Boundary::Outflow);
    let bad_layouts: [(&str, Vec<f64>, Vec<f64>); 13] = [
        ("empty q", vec![], mu()),
        ("one q edge", vec![0.0], mu()),
        ("one mu edge", q(), vec![-1.0]),
        ("equal q edges", vec![0.0, 0.5, 0.5, 1.0], mu()),
        ("decreasing q edges", vec![0.0, 1.0, 0.5], mu()),
        ("decreasing mu edges", q(), vec![-1.0, 0.5, 0.0, 1.0]),
        ("NaN q edge", vec![0.0, f64::NAN, 1.0], mu()),
        ("NaN mu edge", q(), vec![-1.0, f64::NAN, 1.0]),
        ("infinite q edge", vec![0.0, f64::INFINITY], mu()),
        ("overflowing q width", vec![-1.0e308, 1.0e308], mu()),
        ("mu not from -1", q(), vec![-0.9, 0.0, 1.0]),
        ("mu not to 1", q(), vec![-1.0, 0.0, 0.9]),
        ("mu beyond 1", q(), vec![-1.0, 0.0, 1.0, 1.5]),
    ];
    for (name, q, mu) in bad_layouts {
        assert!(invalid_input(layout(q, mu)), "{name}");
    }
    assert!(layout(q(), mu()).is_ok());

    let good = || layout(q(), mu()).unwrap();
    let constant = Geometry::Constant { h: 0.3, s: 0.2 };
    let absorber = Some(Absorber {
        k0: 1.0,
        q_chi: 0.2,
    });
    let bad_models: [(&str, Geometry, f64, Option<Absorber>); 12] = [
        ("negative gamma", constant, -1.0, None),
        ("NaN gamma", constant, f64::NAN, None),
        ("infinite gamma", constant, f64::INFINITY, None),
        (
            "negative k0",
            constant,
            1.0,
            Some(Absorber {
                k0: -1.0,
                q_chi: 0.2,
            }),
        ),
        (
            "NaN k0",
            constant,
            1.0,
            Some(Absorber {
                k0: f64::NAN,
                q_chi: 0.2,
            }),
        ),
        (
            "NaN q_chi",
            constant,
            1.0,
            Some(Absorber {
                k0: 1.0,
                q_chi: f64::NAN,
            }),
        ),
        (
            "power law p = 1",
            Geometry::PowerLaw {
                hh: 0.5,
                s0: 0.2,
                p: 1.0,
            },
            1.0,
            absorber,
        ),
        (
            "NaN hh",
            Geometry::PowerLaw {
                hh: f64::NAN,
                s0: 0.2,
                p: 1.5,
            },
            1.0,
            None,
        ),
        (
            "NaN s0",
            Geometry::PowerLaw {
                hh: 0.5,
                s0: f64::NAN,
                p: 1.5,
            },
            1.0,
            None,
        ),
        (
            "NaN p",
            Geometry::PowerLaw {
                hh: 0.5,
                s0: 0.2,
                p: f64::NAN,
            },
            1.0,
            None,
        ),
        (
            "NaN h",
            Geometry::Constant {
                h: f64::NAN,
                s: 0.2,
            },
            1.0,
            None,
        ),
        (
            "infinite s",
            Geometry::Constant {
                h: 0.3,
                s: f64::INFINITY,
            },
            1.0,
            None,
        ),
    ];
    for (name, geometry, gamma, absorber) in bad_models {
        assert!(
            invalid_input(Model::new(good(), geometry, gamma, absorber, false)),
            "{name}"
        );
    }

    let model = Model::new(
        good(),
        Geometry::PowerLaw {
            hh: 0.5,
            s0: 0.2,
            p: 1.5,
        },
        1.0,
        absorber,
        false,
    )
    .unwrap();
    let n = model.layout.n();
    let width = 2 * model.layout.bandwidth() + 1;
    for (ny, nout) in [(n - 1, n), (n + 1, n), (n, n - 1), (n, n + 1), (0, 0)] {
        let mut out = vec![0.0; nout];
        let state = vec![0.1; ny];
        assert!(
            dimension(model.rhs(0.5, &state, &mut out)),
            "rhs {ny} {nout}"
        );
        assert!(
            dimension(model.partial_t(0.5, &state, &mut out)),
            "partial_t {ny} {nout}"
        );
    }
    for (r, c) in [(n - 1, n), (n, n - 1), (n + 1, n + 1), (0, 0)] {
        let mut m = DenseMatrix::zeros(r, c);
        assert!(dimension(model.dense_fill(0.5, &mut m)), "dense {r}x{c}");
    }
    for len in [0, n * width - 1, n * width + 1, n * n] {
        let mut b = vec![0.0; len];
        assert!(dimension(model.band_fill(0.5, &mut b)), "band {len}");
    }
}

#[test]
fn sanity_partial_t_matches_central_differences_away_from_kinks() {
    let grid = &grids()[1];
    let mut rng = XorShift::new(0x5a17_0006);
    let absorber = Absorber {
        k0: 0.5,
        q_chi: -0.5,
    };
    let power = Geometry::PowerLaw {
        hh: 0.5,
        s0: 0.2,
        p: 1.5,
    };
    let cases = [
        (
            "comoving power law absorber",
            Coordinate::Comoving,
            power,
            Some(absorber),
        ),
        (
            "comoving constant absorber",
            Coordinate::Comoving,
            Geometry::Constant { h: 0.3, s: 0.2 },
            Some(absorber),
        ),
        (
            "physical power law absorber",
            Coordinate::Physical,
            power,
            Some(absorber),
        ),
        ("physical power law", Coordinate::Physical, power, None),
    ];
    let e = 1.0e-5;
    let mut worst = 0.0_f64;
    for (name, coordinate, geometry, absorber) in cases {
        let model = model_on(grid, coordinate, Boundary::Outflow, geometry, 0.5, absorber);
        assert!(!model.autonomous(), "{name}");
        let n = model.layout.n();
        for t in [0.3, 0.7, 2.0] {
            // Preconditions (not tolerances): the threshold stays well away
            // from every edge, and no upwind direction flips, on [t-e, t+e].
            if coordinate == Coordinate::Comoving {
                let x = absorber.unwrap().q_chi + geometry.ln_a(t);
                let gap = grid
                    .q
                    .iter()
                    .fold(f64::INFINITY, |m, edge| m.min((edge - x).abs()));
                assert!(gap > 1.0e-3, "{name} t={t}: threshold {x} near an edge");
            }
            for tt in [t - e, t + e] {
                let (h0, s0, _, _) = geometry.at(t);
                let (h1, s1, _, _) = geometry.at(tt);
                let h_eff = |h: f64| {
                    if coordinate == Coordinate::Physical {
                        h
                    } else {
                        0.0
                    }
                };
                for c in &model.layout.c {
                    let a = -h_eff(h0) - s0 * c;
                    let b = -h_eff(h1) - s1 * c;
                    assert!(a != 0.0 && (a > 0.0) == (b > 0.0), "{name}: upwind flip");
                }
                assert!((s0 > 0.0) == (s1 > 0.0) && s0 != 0.0);
            }
            let y = positive_state(&mut rng, n);
            let mut ft = vec![0.0; n];
            let (mut fp, mut fm) = (vec![0.0; n], vec![0.0; n]);
            model.partial_t(t, &y, &mut ft).unwrap();
            model.rhs(t + e, &y, &mut fp).unwrap();
            model.rhs(t - e, &y, &mut fm).unwrap();
            let dev = (0..n)
                .map(|k| (ft[k] - (fp[k] - fm[k]) / (2.0 * e)).abs())
                .fold(0.0_f64, f64::max);
            let rel = dev / max_abs(&ft);
            assert!(max_abs(&ft) > 0.0, "{name} t={t}: f_t vanishes");
            assert!(
                rel <= 1.0e-6,
                "{name} t={t}: f_t vs central difference {rel:e}"
            );
            worst = worst.max(rel);
        }
    }

    // The negative control reports exact zeros over garbage in `out`.
    let dropped = Model::new(
        grid.layout(Coordinate::Comoving, Boundary::Outflow),
        power,
        0.5,
        Some(absorber),
        true,
    )
    .unwrap();
    let n = dropped.layout.n();
    let y = positive_state(&mut rng, n);
    let mut out = vec![1.0e300; n];
    dropped.partial_t(0.7, &y, &mut out).unwrap();
    assert!(out.iter().all(|v| v.to_bits() == 0));
    let dropped_problem = support::problem(Arc::new(dropped)).unwrap();
    assert!(!dropped_problem.autonomous);
    eprintln!("sanity: max f_t vs central difference rel dev {worst:e}");
}

/// F1 tie rule: at a threshold exactly on a cell edge, `f_t` is the
/// right-sided derivative (the branch the threshold moves into) and is
/// continuous from the right; from the left it jumps. Comoving 16x8 grid
/// with constant geometry (the transport is then time independent, so only
/// the absorber rows change with `t`): `x_chi(t) = q_chi + h t` reaches the
/// edge `q_8 = -0.5` exactly at `t = 0.5`, moving up (`h > 0`, the tie cell
/// `8` with `lo == x_chi` straddles) and moving down (`h < 0`, the tie cell
/// `7` with `hi == x_chi` straddles with value 0).
#[test]
fn sanity_partial_t_is_right_sided_at_an_exact_threshold_tie() {
    let grid = &grids()[1];
    let edge = 8;
    assert_eq!(grid.q[edge], -0.5);
    let t = 0.5;
    let e = 1.0e-7;
    let mut rng = XorShift::new(0x0f1_71e5);
    for (h, q_chi) in [(0.5, -0.75), (-0.5, -0.25)] {
        let geometry = Geometry::Constant { h, s: 0.2 };
        let absorber = Absorber { k0: 2.0, q_chi };
        // Precondition (exact tie, not a tolerance).
        assert_eq!(
            q_chi + geometry.ln_a(t),
            grid.q[edge],
            "h={h}: no exact tie"
        );
        let model = model_on(
            grid,
            Coordinate::Comoving,
            Boundary::Outflow,
            geometry,
            0.5,
            Some(absorber),
        );
        let l = &model.layout;
        let n = l.n();
        let (below_cell, above_cell) = (edge - 1, edge);
        let tie_cell = if h > 0.0 { above_cell } else { below_cell };
        let dq = l.dq[tie_cell];
        let scale = absorber.k0 / (3.0 * dq);
        // The branch: straddle at the tie cell, the other neighbour on its
        // right-sided branch; values continuous (bit for bit with the
        // branch on the other side).
        let (k_tie, dk_tie) = model.kbar(tie_cell, t);
        if h > 0.0 {
            assert_eq!(k_tie.to_bits(), (-scale * (-3.0 * dq).exp_m1()).to_bits());
            let expected =
                -3.0 * scale * h * (-3.0 * (l.q_edges[tie_cell + 1] - grid.q[edge])).exp();
            assert_eq!(
                dk_tie.to_bits(),
                expected.to_bits(),
                "h={h}: straddle derivative"
            );
            assert_eq!(model.kbar(below_cell, t), (0.0, 0.0));
        } else {
            assert_eq!(k_tie, 0.0);
            assert_eq!(dk_tie.to_bits(), (-3.0 * scale * h).to_bits(), "h={h}");
            let (k_up, dk_up) = model.kbar(above_cell, t);
            assert!(k_up > 0.0 && dk_up.to_bits() == (3.0 * h * k_up).to_bits());
        }
        let mut rows: Vec<usize> = Vec::new();
        for i in [below_cell, above_cell] {
            rows.extend((0..l.nm).map(|j| l.cell(i, j)));
            rows.push(l.absorbed(i));
        }
        let y = positive_state(&mut rng, n);
        let rhs_at = |tt: f64| {
            let mut f = vec![0.0; n];
            model.rhs(tt, &y, &mut f).unwrap();
            f
        };
        let ft_at = |tt: f64| {
            let mut f = vec![0.0; n];
            model.partial_t(tt, &y, &mut f).unwrap();
            f
        };
        let on_rows = |a: &[f64], b: &[f64]| {
            rows.iter()
                .fold(0.0_f64, |m, &r| m.max((a[r] - b[r]).abs()))
        };
        let ft = ft_at(t);
        let ft_scale = rows.iter().fold(0.0_f64, |m, &r| m.max(ft[r].abs()));
        assert!(ft_scale > 0.0, "h={h}: f_t vanishes on the tie rows");
        let (f0, fp, fm) = (rhs_at(t), rhs_at(t + e), rhs_at(t - e));
        let right: Vec<f64> = (0..n).map(|k| (fp[k] - f0[k]) / e).collect();
        let left: Vec<f64> = (0..n).map(|k| (f0[k] - fm[k]) / e).collect();
        let right_rel = on_rows(&ft, &right) / ft_scale;
        let left_rel = on_rows(&ft, &left) / ft_scale;
        assert!(
            right_rel <= 1.0e-5,
            "h={h}: f_t vs right-sided difference {right_rel:e}"
        );
        // Negative control: the left-sided difference sees the other branch.
        assert!(
            left_rel >= 1.0e-1,
            "h={h}: no kink seen from the left ({left_rel:e})"
        );
        // Continuous from the right, a jump from the left.
        for d in [1.0e-12, 1.0e-9] {
            let from_right = on_rows(&ft_at(t + d), &ft) / ft_scale;
            let from_left = on_rows(&ft_at(t - d), &ft) / ft_scale;
            assert!(
                from_right <= 1.0e-6,
                "h={h} d={d:e}: f_t not right-continuous ({from_right:e})"
            );
            assert!(
                from_left >= 1.0e-1,
                "h={h} d={d:e}: no jump from the left ({from_left:e})"
            );
        }
        eprintln!("sanity tie h={h}: f_t vs right difference {right_rel:e}, vs left {left_rel:e}");
    }
}

#[test]
fn sanity_absorber_cell_average_matches_quadrature() {
    let grid = &grids()[1];
    let model = model_on(
        grid,
        Coordinate::Comoving,
        Boundary::Outflow,
        Geometry::PowerLaw {
            hh: 0.5,
            s0: 0.2,
            p: 1.5,
        },
        0.0,
        Some(Absorber {
            k0: 0.5,
            q_chi: -0.5,
        }),
    );
    let l = &model.layout;
    let (k0, q_chi) = (0.5, -0.5);
    let mut worst = 0.0_f64;
    let mut partial_cells = 0usize;
    for t in [0.0, 0.3, 0.7, 2.0, 3.0] {
        let x = q_chi + model.geometry.ln_a(t);
        for i in 0..l.nq {
            let (lo, hi) = (l.q_edges[i], l.q_edges[i + 1]);
            let a = lo.max(x);
            // Composite Simpson of k0 exp(-3 (q - x)) on [a, hi], 2000 panels.
            let reference = if hi <= x {
                0.0
            } else {
                if lo < x {
                    partial_cells += 1;
                }
                let m = 2000;
                let hstep = (hi - a) / m as f64;
                let f = |q: f64| k0 * (-3.0 * (q - x)).exp();
                let mut s = f(a) + f(hi);
                for k in 1..m {
                    s += f(a + k as f64 * hstep) * if k % 2 == 1 { 4.0 } else { 2.0 };
                }
                s * hstep / 3.0 / l.dq[i]
            };
            let (k, _) = model.kbar(i, t);
            let dev = (k - reference).abs();
            assert!(dev <= 1.0e-12 * k0, "cell {i} t={t}: {k} vs {reference}");
            worst = worst.max(dev / k0);
        }
    }
    assert!(partial_cells >= 3, "the threshold must cross cells");
    eprintln!("sanity: kbar vs Simpson max dev/k0 {worst:e} ({partial_cells} partial cells)");
}

#[test]
fn sanity_jvp_callback_and_autonomy_flags() {
    let grid = &grids()[0];
    let absorber = Some(Absorber {
        k0: 2.0,
        q_chi: grid.mid(),
    });
    let power = Geometry::PowerLaw {
        hh: 0.5,
        s0: 0.2,
        p: 1.5,
    };
    let constant = Geometry::Constant { h: 0.3, s: 0.2 };
    let expect = [
        (Coordinate::Physical, constant, None, true),
        (Coordinate::Physical, constant, absorber, true),
        (Coordinate::Comoving, constant, None, true),
        (Coordinate::Comoving, constant, absorber, false),
        (Coordinate::Physical, power, None, false),
        (Coordinate::Physical, power, absorber, false),
        (Coordinate::Comoving, power, None, false),
        (Coordinate::Comoving, power, absorber, false),
    ];
    let mut rng = XorShift::new(0x0a17_0a17);
    for (coordinate, geometry, absorber, autonomous) in expect {
        let model = Arc::new(model_on(
            grid,
            coordinate,
            Boundary::Outflow,
            geometry,
            3.0,
            absorber,
        ));
        assert_eq!(
            model.autonomous(),
            autonomous,
            "{coordinate:?} {geometry:?} {absorber:?}"
        );
        let problem = support::problem(model.clone()).unwrap();
        assert_eq!(problem.autonomous, autonomous);
        // An autonomous model's f_t is exactly zero.
        let n = model.layout.n();
        let y = positive_state(&mut rng, n);
        let v: Vec<f64> = (0..n).map(|_| rng.uniform() - 0.5).collect();
        let mut ft = vec![0.0; n];
        model.partial_t(0.7, &y, &mut ft).unwrap();
        if autonomous {
            assert!(ft.iter().all(|x| *x == 0.0), "{coordinate:?} {geometry:?}");
        }
        // The JVP callback is the RHS of the direction, bit for bit.
        let jvp = problem.jvp_callback().unwrap();
        let (mut a, mut b) = (vec![0.0; n], vec![0.0; n]);
        jvp(0.7, &y, &v, &mut a).unwrap();
        model.rhs(0.7, &v, &mut b).unwrap();
        assert_eq!(bits(&a), bits(&b));
    }
}
