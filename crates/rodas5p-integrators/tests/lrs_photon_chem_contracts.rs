//! Contracts of the photon chemistry adapter (BASS/rei DAG node P04,
//! `examples/lrs_photon_fv_support/chem.rs`; the spec is
//! `research/bassrei_p04_source_exchange_stress_20261022/MODEL.md`):
//!
//! * (a) Case-A PCHIP: the table parses (exact header, 129 rows, the spec's
//!   sha256); at every node `P` is `ln v_k` bit for bit (the last node: the
//!   end of the last interval) and `exp(P)` returns `v_k` within the
//!   round-trip bound `(|ln v_k| + 2) eps` (the `exp(ln v)` round trip alone
//!   can lose `|ln v| eps / 2`, about 32 ulp at `ln v = -92`, so a flat
//!   `1e-15` is not attainable); `P` is continuous and `P'` is continuous
//!   at every interior node to rounding; every segment stays within its
//!   two nodes; the domain is inclusive and `next_down(T_min)`,
//!   `next_up(T_max)`, NaN and infinities are the typed `CASE_A_T_DOMAIN`
//!   refusal, each counted once.
//! * (b) Verner: zero below `chi`, positive at and above; the cell averages
//!   vanish exactly below each threshold, are positive above, and match a
//!   fine composite Simpson rule; the 16-point Gauss-Legendre rule is
//!   symmetric and exact for even monomials up to degree 30; the energy
//!   faces are the logarithms of the thresholds bit for bit.
//! * (c) Invariants: for random admissible states, each of the five ledger
//!   weight vectors `w` has `|w . f| <= 1e-14 A` with `A = sum_{r,c}
//!   |w_r J_rc y_c|` (as P02's C3: the absolute size of every term the RHS
//!   accumulates, a bilinear or quadratic term counted once per factor).
//!   The weighted sum uses Neumaier compensation.
//! * (d) Jacobian columns against Richardson-extrapolated central
//!   differences (relative step `1e-6` of `max(|y_k|, scale_k)`) at most
//!   `1e-6` relative per column, away from PCHIP nodes and upwind flips,
//!   also at `amp = 1e30` (the shear row's entries dominate their columns);
//!   the in-place fill over a used or NaN-poisoned pattern equals a fresh
//!   fill, the allocating closure equals it, nothing lies outside the
//!   declared pattern.
//! * (e) `f_t` against Richardson central differences in `t`, at most
//!   `1e-6` relative per component (components below `1e-10 H |f_k|` are
//!   negligible and skipped), including `amp = 1e30`, where the shear
//!   row's `K_D amp (D_gamma + D_m) d(a^-3)/dt` dominates (with a negative
//!   control that drops it).
//! * (f) VG-1: the callbacks allocate nothing; the dense fast driver
//!   allocates the same count at `rtol = 1e-6` and `1e-9` while the attempt
//!   counts differ, with the in-place zero-skipping LU.
//! * (g) VG-3: repeated runs are bit for bit identical with equal counters,
//!   and a uniform vector atol is the scalar run bit for bit.
//! * (h) Typed refusals: invalid grids, parameters, tables and initial data
//!   are `InvalidInput`; wrong shapes are `Dimension`; a state outside the
//!   table's temperatures is the counted `NonFinite` `CASE_A_T_DOMAIN`
//!   refusal of every callback; a run that leaves the domain ends in a
//!   `DriverPartial` at the boundary whose rejections are all that refusal
//!   (with a `min_step` exit the last failure is `NonFinite`; with an
//!   attempt-budget exit it may be `None`).
//! * Pins: the transport is P02's operator bit for bit (`L_lo`, `L_hi` and
//!   `W_gamma`), the single-term matter rows follow MODEL.md's operation
//!   order bit for bit, and the initial data of section 9.

use std::alloc::{GlobalAlloc, Layout as AllocLayout, System};
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rodas5p_core::{CoreError, CoreResult, DenseMatrix, WorkCounters};
use rodas5p_integrators::{
    AdaptiveFailureKind, AdaptiveStepConfig, DriverStopReason, OutputSchedule, Rodas5pFastLu,
    Rodas5pFastResult, integrate_rodas5p_fast_observed, integrate_rodas5p_fast_observed_with_atol,
};

#[path = "../examples/lrs_photon_fv_support/mod.rs"]
mod support;
use support::chem::{
    self, BIND, CASE_A_COLUMNS, CASE_A_HEADER, CASE_A_ROWS, CASE_A_T_DOMAIN, CaseATable, ChemGrid,
    ChemModel, ChemParams, EV_ERG, FIT, InitialData, slot,
};
use support::{Geometry, Layout, Model};

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

/// 64-bit linear congruential generator (Knuth's MMIX constants).
struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// Uniform in `[0, 1)` from the top 53 bits.
    fn uniform(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

/// Neumaier-compensated sum.
fn compensated_sum(v: impl IntoIterator<Item = f64>) -> f64 {
    let (mut s, mut c) = (0.0_f64, 0.0_f64);
    for x in v {
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

fn table_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../research/bassrei_p04_source_exchange_stress_20261022/source/owner_grackle_rates_129.csv",
    )
}

fn table_text() -> String {
    std::fs::read_to_string(table_path()).expect("the Case-A table must be readable")
}

fn table() -> CaseATable {
    CaseATable::from_csv_path(table_path()).expect("the Case-A table must parse")
}

const PARAMS: ChemParams = ChemParams {
    h0: 3.0e-17,
    amp: 1.0,
    tau_m: 1.0e14,
};

const SMOKE: InitialData = InitialData {
    n_h0: 2.4e-5,
    x_hi0: 0.6,
    f_hei: 0.2,
    f_heii: 0.5,
    temperature0: 12000.0,
    d_m0: 0.0,
    s0_over_h0: 0.1,
    photons_per_h: 0.05,
    alpha: 1.5,
    a2: 0.5,
};

/// `n = 2 + 9 * 5 + 12 = 59 <= RODAS5P_FAST_SMALL_LU_MAX`: the dense fast
/// driver takes the in-place zero-skipping LU.
const MODERATE: ChemGrid = ChemGrid {
    n1: 3,
    n2: 3,
    n3: 3,
    nm: 4,
};

const GRIDS: [ChemGrid; 3] = [
    MODERATE,
    ChemGrid {
        n1: 4,
        n2: 4,
        n3: 4,
        nm: 4,
    },
    ChemGrid {
        n1: 2,
        n2: 3,
        n3: 5,
        nm: 6,
    },
];

fn model_on(grid: ChemGrid) -> ChemModel {
    ChemModel::new(table(), grid, PARAMS).unwrap()
}

/// A random admissible state at a random time `t <= t0 / 2`: positive
/// photon numbers and photon ledgers, positive species, the temperature
/// log-uniform inside the table (2% away from both ends), signed `D_m`,
/// `W_gamma`, `W_m`, `E_esc` and shear `0.05 <= |s|/H0 <= 0.3`. With
/// `H >= 2 H0 / 3` and `|c_j| <= 2` no upwind direction is near a flip.
fn random_state(model: &ChemModel, rng: &mut Lcg) -> (f64, Vec<f64>) {
    let n = model.n();
    let b = model.n_ph;
    let n_h = 2.4e-5;
    let mut y = vec![0.0; n];
    for v in &mut y[..b] {
        *v = 1.0e-7 * (0.05 + rng.uniform());
    }
    for k in [slot::HI, slot::HII] {
        y[b + k] = n_h * (0.02 + rng.uniform());
    }
    for k in [slot::HEI, slot::HEII, slot::HEIII] {
        y[b + k] = 0.1 * n_h * (0.02 + rng.uniform());
    }
    let (lo, hi) = (model.table.t_min().ln(), model.table.t_max().ln());
    let temperature = (lo + 0.02 + (hi - lo - 0.04) * rng.uniform()).exp();
    y[b + slot::U] = 1.0;
    let per_unit = model.temperature(&y);
    let u = temperature / per_unit;
    y[b + slot::U] = u;
    y[b + slot::DM] = u * (rng.uniform() - 0.5);
    y[b + slot::W_GAMMA] = 1.0e-6 * (rng.uniform() - 0.5);
    y[b + slot::WM] = u * (rng.uniform() - 0.5);
    y[b + slot::EESC] = u * rng.uniform();
    y[b + slot::RE] = n_h * rng.uniform();
    let sign = if rng.uniform() < 0.5 { -1.0 } else { 1.0 };
    y[b + slot::S] = sign * PARAMS.h0 * (0.05 + 0.25 * rng.uniform());
    let t = 0.5 * model.age0 * rng.uniform();
    (t, y)
}

/// Whether `ln T` is at least `gap` from every table node.
fn away_from_nodes(model: &ChemModel, y: &[f64], gap: f64) -> bool {
    let x = model.temperature(y).ln();
    model.table.x.iter().all(|xk| (xk - x).abs() >= gap)
}

/// The per-slot scale of states and tolerances: photon slots `photon`,
/// the gas energies `|U|`, species and the electron ledger `n_h`, shear
/// `H0`.
fn slot_scale(model: &ChemModel, k: usize, photon: f64, u: f64, n_h: f64) -> f64 {
    let b = model.n_ph;
    if k < b {
        return photon;
    }
    match k - b {
        slot::W_GAMMA | slot::U | slot::DM | slot::WM | slot::EESC => u.abs(),
        slot::S => model.params.h0,
        _ => n_h,
    }
}

fn fresh_dense(model: &ChemModel, t: f64, y: &[f64]) -> DenseMatrix {
    let n = model.n();
    let mut m = DenseMatrix::zeros(n, n);
    model.dense_fill(t, y, &mut m).unwrap();
    m
}

/// Whether `(r, c)` lies in the declared pattern: photon rows on the band
/// and the matter columns, matter rows everywhere.
fn in_pattern(model: &ChemModel, r: usize, c: usize) -> bool {
    let b = model.n_ph;
    r >= b || c >= b || r.abs_diff(c) <= model.layout().bandwidth()
}

#[test]
fn a_case_a_pchip_reproduces_nodes_is_c1_monotone_and_refuses_outside_the_domain() {
    let table = table();
    assert_eq!(table.sha256, chem::CASE_A_SHA256);
    assert_eq!(table.temperatures.len(), CASE_A_ROWS);
    assert_eq!(table.coef.len(), CASE_A_ROWS - 1);
    let m = CASE_A_ROWS;

    // Nodes.
    let mut worst_node = 0.0_f64;
    let mut above_1e15 = 0usize;
    for k in 0..m {
        let tk = table.temperatures[k];
        let x = tk.ln();
        assert_eq!(x.to_bits(), table.x[k].to_bits());
        let kk = table.interval(x);
        assert_eq!(kk, k.min(m - 2), "node {k}: interval");
        let (p, _) = table.segment(kk, x - table.x[kk]);
        let (v, _) = table.eval(tk).unwrap();
        for c in 0..CASE_A_COLUMNS {
            let yk = table.y[k][c];
            if k < m - 1 {
                assert_eq!(p[c].to_bits(), yk.to_bits(), "node {k} column {c}: P");
            } else {
                assert!(
                    (p[c] - yk).abs() <= 4.0 * EPS * yk.abs(),
                    "last node column {c}: P {} vs {yk}",
                    p[c]
                );
            }
            let vk = table.values[k][c];
            let rel = (v[c] - vk).abs() / vk;
            let bound = if k < m - 1 {
                (yk.abs() + 2.0) * EPS
            } else {
                (5.0 * yk.abs() + 2.0) * EPS
            };
            assert!(
                rel <= bound,
                "node {k} column {c}: exp(P) {} vs {vk} (rel {rel:e} > {bound:e})",
                v[c]
            );
            worst_node = worst_node.max(rel);
            if rel > 1.0e-15 {
                above_1e15 += 1;
            }
        }
    }

    // Continuity of P and P' at the interior nodes (the right interval
    // starts at y_k and d_k exactly).
    let mut worst_p = 0.0_f64;
    let mut worst_dp = 0.0_f64;
    for k in 1..m - 1 {
        let h = table.x[k] - table.x[k - 1];
        let (p_left, dp_left) = table.segment(k - 1, h);
        let (p_right, dp_right) = table.segment(k, 0.0);
        for c in 0..CASE_A_COLUMNS {
            assert_eq!(p_right[c].to_bits(), table.y[k][c].to_bits());
            assert_eq!(dp_right[c].to_bits(), table.d[k][c].to_bits());
            let slope = (table.y[k][c] - table.y[k - 1][c]) / h;
            let p_scale = table.y[k - 1][c].abs() + table.y[k][c].abs();
            let dp_scale = table.d[k - 1][c].abs() + table.d[k][c].abs() + 2.0 * slope.abs();
            let (ep, edp) = (
                (p_left[c] - table.y[k][c]).abs(),
                (dp_left[c] - table.d[k][c]).abs(),
            );
            assert!(
                ep <= 8.0 * EPS * p_scale,
                "node {k} column {c}: P jump {ep:e}"
            );
            assert!(
                edp <= 32.0 * EPS * dp_scale.max(f64::MIN_POSITIVE),
                "node {k} column {c}: P' jump {edp:e} (scale {dp_scale:e})"
            );
            worst_p = worst_p.max(ep / p_scale);
            if dp_scale > 0.0 {
                worst_dp = worst_dp.max(edp / dp_scale);
            }
        }
    }

    // Every segment stays within its nodes and is monotone (sampled).
    for k in 0..m - 1 {
        let h = table.x[k + 1] - table.x[k];
        for c in 0..CASE_A_COLUMNS {
            let (a, b) = (table.y[k][c], table.y[k + 1][c]);
            let (lo, hi) = (a.min(b), a.max(b));
            let tol = 4.0 * EPS * a.abs().max(b.abs());
            let mut previous = a;
            for q in 1..64 {
                let (p, _) = table.segment(k, h * q as f64 / 64.0);
                let p = p[c];
                assert!(
                    p >= lo - tol && p <= hi + tol,
                    "segment {k} column {c}: P {p} outside [{lo}, {hi}]"
                );
                if b >= a {
                    assert!(
                        p >= previous - tol,
                        "segment {k} column {c}: not increasing"
                    );
                } else {
                    assert!(
                        p <= previous + tol,
                        "segment {k} column {c}: not decreasing"
                    );
                }
                previous = p;
            }
        }
    }

    // The inclusive domain and the counted typed refusal.
    let (t_min, t_max) = (table.t_min(), table.t_max());
    assert!(table.eval(t_min).is_ok() && table.eval(t_max).is_ok());
    assert_eq!(table.domain_refusals(), 0);
    let outside = [
        t_min.next_down(),
        t_max.next_up(),
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        0.0,
        -t_min,
    ];
    for (k, temperature) in outside.into_iter().enumerate() {
        match table.eval(temperature) {
            Err(CoreError::NonFinite(msg)) => assert!(
                msg.starts_with(CASE_A_T_DOMAIN),
                "T = {temperature:e}: message {msg:?}"
            ),
            other => panic!("T = {temperature:e}: expected the domain refusal, got {other:?}"),
        }
        assert_eq!(table.domain_refusals(), k as u64 + 1);
    }
    eprintln!(
        "(a) PCHIP: node max rel {worst_node:e} ({above_1e15} of {} node values above 1e-15); \
         interior P jump max rel {worst_p:e}, P' jump max rel {worst_dp:e}",
        m * CASE_A_COLUMNS
    );
}

#[test]
fn b_verner_cross_sections_cell_averages_and_the_quadrature_rule() {
    for (s, fit) in FIT.iter().enumerate() {
        let chi = fit[0];
        assert_eq!(chem::verner_sigma(s, chi.next_down()), 0.0);
        assert_eq!(chem::verner_sigma(s, 0.5 * chi), 0.0);
        for e in [chi, chi.next_up(), 1.5 * chi, 99.0, 100.0, 1000.0] {
            let sigma = chem::verner_sigma(s, e);
            assert!(
                sigma.is_finite() && sigma > 0.0,
                "species {s} E={e}: {sigma}"
            );
        }
    }

    let rule = chem::gauss_legendre_16();
    let total: f64 = rule.weights.iter().sum();
    assert!((total - 2.0).abs() <= 4.0 * EPS, "weights sum {total}");
    for i in 0..16 {
        assert_eq!(rule.nodes[i].to_bits(), (-rule.nodes[15 - i]).to_bits());
        assert_eq!(rule.weights[i].to_bits(), rule.weights[15 - i].to_bits());
        assert!(rule.weights[i] > 0.0 && rule.nodes[i].abs() < 1.0);
        if i > 0 {
            assert!(rule.nodes[i] > rule.nodes[i - 1]);
        }
    }
    let mut worst_rule = 0.0_f64;
    for k in 0..=15 {
        let exact = 2.0 / (2 * k + 1) as f64;
        let q: f64 = rule
            .nodes
            .iter()
            .zip(&rule.weights)
            .map(|(x, w)| w * x.powi(2 * k))
            .sum();
        let rel = (q - exact).abs() / exact;
        assert!(rel <= 1.0e-14, "x^{}: {q} vs {exact}", 2 * k);
        worst_rule = worst_rule.max(rel);
    }

    let mut worst_cell = 0.0_f64;
    for grid in GRIDS {
        let model = model_on(grid);
        let l = model.layout();
        let q = &l.q_edges;
        assert_eq!(q.len(), grid.n1 + grid.n2 + grid.n3 + 1);
        assert_eq!(q[0].to_bits(), FIT[0][0].ln().to_bits());
        assert_eq!(q[grid.n1].to_bits(), FIT[1][0].ln().to_bits());
        assert_eq!(q[grid.n1 + grid.n2].to_bits(), FIT[2][0].ln().to_bits());
        assert_eq!(q[q.len() - 1].to_bits(), chem::E_MAX.ln().to_bits());
        assert_eq!(l.mu_edges[0], -1.0);
        assert_eq!(l.mu_edges[grid.nm], 1.0);
        let first_above = [0, grid.n1, grid.n1 + grid.n2];
        for (s, &first) in first_above.iter().enumerate() {
            for i in 0..l.nq {
                let sbar = model.sbar[s][i];
                if i < first {
                    assert_eq!(sbar.to_bits(), 0, "species {s} cell {i} below threshold");
                    continue;
                }
                assert!(sbar > 0.0, "species {s} cell {i}");
                // Composite Simpson in q, 4000 panels.
                let (lo, hi) = (q[i], q[i + 1]);
                let panels = 4000;
                let step = (hi - lo) / panels as f64;
                let f = |x: f64| chem::verner_sigma(s, x.exp());
                let mut acc = f(lo) + f(hi);
                for p in 1..panels {
                    acc += f(lo + p as f64 * step) * if p % 2 == 1 { 4.0 } else { 2.0 };
                }
                let reference = acc * step / 3.0 / (hi - lo);
                let rel = (sbar - reference).abs() / reference;
                assert!(
                    rel <= 1.0e-12,
                    "species {s} cell {i}: {sbar} vs {reference}"
                );
                worst_cell = worst_cell.max(rel);
            }
        }
    }
    eprintln!(
        "(b) GL16 even-monomial max rel {worst_rule:e}; sbar vs Simpson max rel {worst_cell:e}"
    );
}

#[test]
fn c_rhs_preserves_the_five_ledgers_to_rounding() {
    let mut rng = Lcg::new(0x00c0_ffee_0004);
    let mut worst = [0.0_f64; 5];
    let mut worst_l1 = [0.0_f64; 5];
    let mut states = 0usize;
    for grid in GRIDS {
        let model = model_on(grid);
        let n = model.n();
        let weights = model.ledger_weights();
        let mut f = vec![0.0; n];
        for _ in 0..40 {
            let (t, y) = random_state(&model, &mut rng);
            // The weights reproduce the ledgers.
            let ledgers = model.ledgers(&y).as_array();
            for (k, w) in weights.iter().enumerate() {
                let dot = compensated_sum(w.iter().zip(&y).map(|(a, b)| a * b));
                let abs: f64 = w.iter().zip(&y).map(|(a, b)| (a * b).abs()).sum();
                assert!(
                    (dot - ledgers[k]).abs() <= 1.0e-14 * abs,
                    "{:?} ledger {k}: weights {dot:e} vs {:e}",
                    grid,
                    ledgers[k]
                );
            }
            model.rhs(t, &y, &mut f).unwrap();
            let j = fresh_dense(&model, t, &y);
            for (k, w) in weights.iter().enumerate() {
                let total = compensated_sum(w.iter().zip(&f).map(|(a, b)| a * b)).abs();
                let mut a = 0.0;
                for r in 0..n {
                    if w[r] == 0.0 {
                        continue;
                    }
                    for c in 0..n {
                        a += (w[r] * j[(r, c)] * y[c]).abs();
                    }
                }
                let l1: f64 = w.iter().zip(&f).map(|(a, b)| (a * b).abs()).sum();
                assert!(a > 0.0, "{grid:?} ledger {k}: no terms");
                assert!(
                    total <= 1.0e-14 * a,
                    "{grid:?} {} ledger: |w.f| = {total:e}, A = {a:e}",
                    chem::Ledgers::NAMES[k]
                );
                worst[k] = worst[k].max(total / a);
                worst_l1[k] = worst_l1[k].max(total / l1);
            }
            states += 1;
        }
    }
    eprintln!(
        "(c) {states} states; max |w.f|/A per ledger {:?}: {worst:?}; /sum|w f|: {worst_l1:?}",
        chem::Ledgers::NAMES
    );
}

#[test]
fn d_jacobian_matches_richardson_differences_and_refills_exactly() {
    let mut rng = Lcg::new(0x00d0_d0d0);
    let mut worst = 0.0_f64;
    let mut total_checked = 0usize;
    // amp = 1e30 makes the shear row's photon and D_m entries dominate
    // their columns, so they are checked too.
    let large_amp = ChemParams {
        amp: 1.0e30,
        ..PARAMS
    };
    for (grid, params, count) in [(MODERATE, PARAMS, 4), (GRIDS[2], large_amp, 2)] {
        let model = ChemModel::new(table(), grid, params).unwrap();
        let n = model.n();
        let b = model.n_ph;
        let (mut fp, mut fm) = (vec![0.0; n], vec![0.0; n]);
        let (mut checked, mut zero_columns, mut states) = (0usize, 0usize, 0usize);
        let mut previous: Option<(f64, Vec<f64>)> = None;
        while states < count {
            let (t, y) = random_state(&model, &mut rng);
            if !away_from_nodes(&model, &y, 1.0e-4) {
                continue;
            }
            let jac = fresh_dense(&model, t, &y);
            let photon = 1.0e-7;
            let (u, n_h) = (y[b + slot::U], 2.4e-5);
            let mut yp = y.clone();
            let mut central = |c: usize, h: f64| -> Vec<f64> {
                yp[c] = y[c] + h;
                let hp = yp[c] - y[c];
                model.rhs(t, &yp, &mut fp).unwrap();
                yp[c] = y[c] - h;
                let hm = y[c] - yp[c];
                model.rhs(t, &yp, &mut fm).unwrap();
                yp[c] = y[c];
                (0..n).map(|r| (fp[r] - fm[r]) / (hp + hm)).collect()
            };
            for c in 0..n {
                let step = 1.0e-6 * y[c].abs().max(slot_scale(&model, c, photon, u, n_h));
                let (d1, d2) = (central(c, step), central(c, 0.5 * step));
                let fd: Vec<f64> = (0..n).map(|r| (4.0 * d2[r] - d1[r]) / 3.0).collect();
                let scale = (0..n).fold(0.0_f64, |s, r| s.max(jac[(r, c)].abs()).max(fd[r].abs()));
                let dev = (0..n).fold(0.0_f64, |s, r| s.max((jac[(r, c)] - fd[r]).abs()));
                if scale == 0.0 {
                    zero_columns += 1;
                    continue;
                }
                assert!(
                    dev <= 1.0e-6 * scale,
                    "{grid:?} amp={:e} state {states} column {c}: J vs Richardson {:e}",
                    params.amp,
                    dev / scale
                );
                worst = worst.max(dev / scale);
                checked += 1;
            }

            // Nothing outside the pattern; the allocating closure and the
            // in-place refills over the previous state's matrix and over a
            // NaN-poisoned pattern equal the fresh fill.
            for r in 0..n {
                for c in 0..n {
                    if !in_pattern(&model, r, c) {
                        assert_eq!(jac[(r, c)].to_bits(), 0, "({r},{c}) outside the pattern");
                    }
                }
            }
            let problem =
                chem::problem(Arc::new(ChemModel::new(table(), grid, params).unwrap())).unwrap();
            let mut counters = WorkCounters::default();
            let allocated = problem.dense_jacobian(t, &y, &mut counters).unwrap();
            assert_eq!(bits(allocated.as_slice()), bits(jac.as_slice()));
            if let Some((tp, yp)) = &previous {
                let mut m = fresh_dense(&model, *tp, yp);
                model.dense_fill(t, &y, &mut m).unwrap();
                assert_eq!(bits(m.as_slice()), bits(jac.as_slice()), "refill");
                for r in 0..n {
                    for c in 0..n {
                        if in_pattern(&model, r, c) {
                            m[(r, c)] = f64::NAN;
                        }
                    }
                }
                problem
                    .dense_jacobian_into(t, &y, &mut m, &mut counters)
                    .unwrap();
                assert_eq!(bits(m.as_slice()), bits(jac.as_slice()), "poisoned refill");
            }
            previous = Some((t, y));
            states += 1;
        }
        // The RHS reads neither L_lo, L_hi, A_i nor W_gamma, W_m, E_esc, R_e.
        let nq = model.layout().nq;
        assert_eq!(zero_columns, count * (nq + 6));
        assert_eq!(checked, count * (n - nq - 6));
        total_checked += checked;
    }
    eprintln!("(d) {total_checked} columns: max J vs Richardson rel {worst:e}");
}

#[test]
fn e_partial_t_matches_richardson_differences_in_t() {
    let mut rng = Lcg::new(0x00e0_e0e0);
    let mut worst = 0.0_f64;
    let mut worst_shear = 0.0_f64;
    let mut skipped = 0usize;
    // A small state (nq = 3, nm = 2, n = 23) and the moderate grid at
    // amp = 1e30, where the a^-3 term `K_D amp (D_gamma + D_m) d(a^-3)/dt`
    // of the shear row dominates `-3 (dH/dt) s`.
    let small = ChemGrid {
        n1: 1,
        n2: 1,
        n3: 1,
        nm: 2,
    };
    let cases = GRIDS
        .map(|g| (g, 1.0))
        .into_iter()
        .chain([(small, 1.0e30), (MODERATE, 1.0e30)]);
    for (grid, amp) in cases {
        let model = ChemModel::new(table(), grid, ChemParams { amp, ..PARAMS }).unwrap();
        let n = model.n();
        let is = model.slot(slot::S);
        let (mut ft, mut f, mut fp, mut fm) =
            (vec![0.0; n], vec![0.0; n], vec![0.0; n], vec![0.0; n]);
        for _ in 0..6 {
            let (t, y) = random_state(&model, &mut rng);
            let t = t + 0.1 * model.age0;
            model.partial_t(t, &y, &mut ft).unwrap();
            model.rhs(t, &y, &mut f).unwrap();
            let delta = 1.0e-3 * (model.age0 + t);
            let mut central = |e: f64| -> Vec<f64> {
                model.rhs(t + e, &y, &mut fp).unwrap();
                model.rhs(t - e, &y, &mut fm).unwrap();
                let span = (t + e) - (t - e);
                (0..n).map(|k| (fp[k] - fm[k]) / span).collect()
            };
            let (d1, d2) = (central(delta), central(0.5 * delta));
            let fd: Vec<f64> = (0..n).map(|k| (4.0 * d2[k] - d1[k]) / 3.0).collect();
            let bg = model.background(t);
            for k in 0..n {
                let scale = ft[k].abs().max(fd[k].abs());
                if scale <= 1.0e-10 * bg.h * f[k].abs() {
                    skipped += 1;
                    continue;
                }
                let rel = (ft[k] - fd[k]).abs() / scale;
                assert!(
                    rel <= 1.0e-6,
                    "{grid:?} amp={amp:e} t={t:e} slot {k}: f_t {} vs {}",
                    ft[k],
                    fd[k]
                );
                worst = worst.max(rel);
            }
            // The shear row: f_t[IS] = -3 (dH/dt) s + K_D amp (D_gamma +
            // D_m) d(a^-3)/dt, bit for bit in this order.
            let (s, d_m) = (y[is], y[model.slot(slot::DM)]);
            let source = model.k_d_amp * (model.moments(&y).d_gamma + d_m) * bg.da_inv3;
            let drift = -3.0 * bg.dh * s;
            assert_eq!(ft[is].to_bits(), (drift + source).to_bits());
            let rel = (ft[is] - fd[is]).abs() / ft[is].abs().max(fd[is].abs());
            assert!(rel <= 1.0e-6, "{grid:?} amp={amp:e}: shear f_t {rel:e}");
            if amp > 1.0 {
                // The a^-3 term dominates, and dropping it (negative
                // control) misses the finite difference.
                assert!(
                    source.abs() >= 1.0e6 * drift.abs(),
                    "{grid:?}: no dominance"
                );
                let dropped = (drift - fd[is]).abs() / fd[is].abs();
                assert!(dropped >= 0.5, "{grid:?}: the a^-3 term is not seen");
                worst_shear = worst_shear.max(rel);
            }
        }
    }
    eprintln!(
        "(e) max f_t vs Richardson rel {worst:e} ({skipped} negligible components); shear row \
         at amp = 1e30 {worst_shear:e}"
    );
}

/// The dense fast driver on the moderate grid from the section 9 data.
const SHORT_SPAN: (f64, f64) = (0.0, 2.0e11);

fn smoke_state(model: &ChemModel) -> Vec<f64> {
    model.initial_state(&SMOKE).unwrap()
}

/// `factor max(|y0_k|, block scale)` with the photon block scale the
/// largest initial photon number.
fn atol_vector(model: &ChemModel, y0: &[f64], factor: f64) -> Vec<f64> {
    let b = model.n_ph;
    let photon = y0[..b].iter().fold(0.0_f64, |m, v| m.max(v.abs()));
    let (u, n_h) = (y0[b + slot::U], y0[b + slot::HI] + y0[b + slot::HII]);
    (0..model.n())
        .map(|k| factor * y0[k].abs().max(slot_scale(model, k, photon, u, n_h)))
        .collect()
}

fn short_config(rtol: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: 1.0e-30,
        rtol,
        initial_step: 1.0e6,
        max_step: SHORT_SPAN.1 - SHORT_SPAN.0,
        max_attempts: 100_000,
        ..AdaptiveStepConfig::default()
    }
}

fn short_schedule() -> OutputSchedule {
    OutputSchedule::new(vec![0.0, 1.0e11, 2.0e11]).unwrap()
}

#[test]
fn f_callbacks_and_the_dense_driver_allocate_independently_of_the_step_count() {
    let model = Arc::new(model_on(MODERATE));
    let n = model.n();
    let y0 = smoke_state(&model);
    let problem = chem::problem(model.clone()).unwrap();
    assert!(!problem.autonomous && problem.has_explicit_jacobian() && !problem.has_jvp());
    let mut out = vec![0.0; n];
    let mut dense = DenseMatrix::zeros(n, n);
    let mut counters = WorkCounters::default();
    let time = |k: usize| 1.0e11 * k as f64 / 1000.0;
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
            model.dense_fill(time(k), &y0, &mut dense).unwrap();
        }
    });
    counts.push(("dense_fill", a));
    let (_, a) = allocations_during(|| {
        for k in 0..1000 {
            problem
                .eval_rhs_into(time(k), &y0, &mut out, &mut counters)
                .unwrap();
        }
    });
    counts.push(("problem eval_rhs_into", a));
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
            problem
                .dense_jacobian_into(time(k), &y0, &mut dense, &mut counters)
                .unwrap();
        }
    });
    counts.push(("problem dense_jacobian_into", a));
    // Positive control: the counter sees an allocation on this thread.
    let (_, control) = allocations_during(|| std::hint::black_box(vec![0.0_f64; n]));
    assert!(control >= 1, "the counting allocator is not active");
    for (name, a) in &counts {
        assert_eq!(*a, 0, "{name} allocated {a} times in 1000 calls");
    }

    let output = short_schedule();
    let run = |rtol: f64| {
        let atol = atol_vector(&model, &y0, 1.0e-2 * rtol);
        let config = short_config(rtol);
        allocations_during(|| {
            integrate_rodas5p_fast_observed_with_atol(
                &problem, SHORT_SPAN, &y0, &config, &output, &atol,
            )
            .unwrap()
        })
    };
    run(1.0e-6); // warm the coefficient cache and lazy statics
    let mut runs = Vec::new();
    for rtol in [1.0e-6, 1.0e-9] {
        let ((fast, partial), allocations) = run(rtol);
        assert!(fast.observed.success && partial.is_none(), "rtol={rtol:e}");
        assert_eq!(fast.lu, Rodas5pFastLu::InPlaceZeroSkipping);
        assert_eq!(fast.observed.y.len(), 3);
        runs.push((
            fast.attempts,
            fast.rejected_steps,
            fast.observed.counters.ft_calls,
            allocations,
        ));
    }
    assert!(runs[1].0 > runs[0].0, "attempts must differ: {runs:?}");
    assert!(
        runs[1].2 > runs[0].2,
        "f_t must be evaluated per step: {runs:?}"
    );
    assert_eq!(
        runs[0].3, runs[1].3,
        "allocations depend on the step count: {runs:?}"
    );
    assert_eq!(model.domain_refusals(), 0);
    eprintln!(
        "(f) callbacks (1000 calls each): {counts:?}, positive control {control}; dense driver \
         (attempts, rejected, ft_calls, allocations) at rtol 1e-6, 1e-9: {runs:?}"
    );
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
    assert_eq!(a.attempts, b.attempts, "{what}");
    assert_eq!(a.accepted_steps, b.accepted_steps, "{what}");
    assert_eq!(a.rejected_steps, b.rejected_steps, "{what}");
    assert_eq!(a.jacobian_reuses, b.jacobian_reuses, "{what}");
    assert_eq!(a.lu, b.lu, "{what}");
    assert_eq!(a.driver, b.driver, "{what}");
}

#[test]
fn g_runs_are_deterministic_and_uniform_vector_atol_is_scalar() {
    let model = Arc::new(model_on(MODERATE));
    let n = model.n();
    let y0 = smoke_state(&model);
    let problem = chem::problem(model.clone()).unwrap();
    let output = short_schedule();
    let config = short_config(1.0e-7);
    let atol = atol_vector(&model, &y0, 1.0e-9);
    let vector = || {
        integrate_rodas5p_fast_observed_with_atol(
            &problem, SHORT_SPAN, &y0, &config, &output, &atol,
        )
        .unwrap()
    };
    let (a, pa) = vector();
    let (b, pb) = vector();
    assert!(pa.is_none() && pb.is_none());
    assert_same_fast(&a, &b, "repeat");

    // A second model instance from the same declaration is the same run.
    let other = chem::problem(Arc::new(model_on(MODERATE))).unwrap();
    let (c, _) =
        integrate_rodas5p_fast_observed_with_atol(&other, SHORT_SPAN, &y0, &config, &output, &atol)
            .unwrap();
    assert_same_fast(&a, &c, "second instance");

    let config = AdaptiveStepConfig {
        atol: 1.0e-25,
        ..config
    };
    let uniform = vec![config.atol; n];
    let scalar =
        integrate_rodas5p_fast_observed(&problem, SHORT_SPAN, &y0, &config, &output).unwrap();
    let (vector, partial) = integrate_rodas5p_fast_observed_with_atol(
        &problem, SHORT_SPAN, &y0, &config, &output, &uniform,
    )
    .unwrap();
    assert!(partial.is_none());
    assert_same_fast(&scalar, &vector, "uniform vector vs scalar atol");
    let y_end = &a.observed.y[2];
    let drift: Vec<f64> = model
        .ledgers(y_end)
        .as_array()
        .iter()
        .zip(model.ledgers(&y0).as_array())
        .map(|(e, s)| ((e - s) / s).abs())
        .collect();
    eprintln!(
        "(g) attempts vector {} scalar {}; final T {:.3} K; ledger rel drift {:?}: {drift:?}",
        a.attempts,
        scalar.attempts,
        model.temperature(y_end),
        chem::Ledgers::NAMES
    );
}

fn invalid_input<T>(r: CoreResult<T>) -> bool {
    matches!(r, Err(CoreError::InvalidInput(_)))
}

fn dimension<T>(r: CoreResult<T>) -> bool {
    matches!(r, Err(CoreError::Dimension(_)))
}

fn domain_refusal<T>(r: CoreResult<T>) -> bool {
    matches!(r, Err(CoreError::NonFinite(msg)) if msg.starts_with(CASE_A_T_DOMAIN))
}

#[test]
fn h_invalid_declarations_shapes_and_temperatures_are_typed_refusals() {
    // Grids.
    for (n1, n2, n3, nm) in [
        (0, 3, 3, 4),
        (3, 0, 3, 4),
        (3, 3, 0, 4),
        (3, 3, 3, 0),
        (usize::MAX, 1, 1, 1),
        (1, 1, 1, usize::MAX),
        (1 << 19, 1 << 19, 1, 1),
    ] {
        let grid = ChemGrid { n1, n2, n3, nm };
        assert!(
            invalid_input(ChemModel::new(table(), grid, PARAMS)),
            "{grid:?}"
        );
    }
    // Parameters.
    for (h0, amp, tau_m) in [
        (0.0, 1.0, 1.0e14),
        (-3.0e-17, 1.0, 1.0e14),
        (f64::NAN, 1.0, 1.0e14),
        (f64::INFINITY, 1.0, 1.0e14),
        (3.0e-17, f64::NAN, 1.0e14),
        (3.0e-17, f64::INFINITY, 1.0e14),
        (3.0e-17, 1.0, 0.0),
        (3.0e-17, 1.0, -1.0),
        (3.0e-17, 1.0, f64::NAN),
        (3.0e-17, 1.0, f64::INFINITY),
    ] {
        let params = ChemParams { h0, amp, tau_m };
        assert!(
            invalid_input(ChemModel::new(table(), MODERATE, params)),
            "{params:?}"
        );
    }
    // Tables.
    let text = table_text();
    let lines: Vec<&str> = text.lines().collect();
    let join = |rows: &[&str]| rows.join("\n") + "\n";
    let mut swapped = lines.clone();
    swapped.swap(5, 6);
    let row = lines[3];
    let with_rate = |value: &str| {
        let mut fields: Vec<&str> = row.split(',').collect();
        fields[1] = value;
        fields.join(",")
    };
    let negative = with_rate(&format!("-{}", row.split(',').nth(1).unwrap()));
    let nan = with_rate("nan");
    let short = row.rsplit_once(',').unwrap().0.to_string();
    let long = format!("{row},1.0");
    let bad_tables: Vec<(&str, String)> = vec![
        ("empty", String::new()),
        ("header only", join(&lines[..1])),
        ("wrong header", text.replacen("T_K", "T", 1)),
        ("128 rows", join(&lines[..CASE_A_ROWS])),
        ("130 rows", {
            let mut more = lines.clone();
            more.push(lines[CASE_A_ROWS]);
            join(&more)
        }),
        ("non-increasing T", join(&swapped)),
        ("negative value", {
            let mut l = lines.clone();
            l[3] = &negative;
            join(&l)
        }),
        ("NaN value", {
            let mut l = lines.clone();
            l[3] = &nan;
            join(&l)
        }),
        ("8 fields", {
            let mut l = lines.clone();
            l[3] = &short;
            join(&l)
        }),
        ("10 fields", {
            let mut l = lines.clone();
            l[3] = &long;
            join(&l)
        }),
        ("blank line", {
            let mut l = lines.clone();
            l.insert(4, "");
            join(&l)
        }),
    ];
    assert_eq!(lines[0], CASE_A_HEADER);
    assert!(CaseATable::from_csv_str(&join(&lines)).is_ok());
    for (name, bad) in &bad_tables {
        match CaseATable::from_csv_str(bad) {
            Err(CoreError::InvalidInput(msg)) => {
                assert!(msg.starts_with("CASE_A_TABLE:"), "{name}: {msg}")
            }
            other => panic!("{name}: expected InvalidInput, got {:?}", other.err()),
        }
    }
    assert!(invalid_input(CaseATable::from_csv_path(
        "/nonexistent/owner_grackle_rates_129.csv"
    )));

    // Initial data.
    let model = Arc::new(model_on(MODERATE));
    let n = model.n();
    assert!(model.initial_state(&SMOKE).is_ok());
    let bad_initial: [(&str, InitialData); 11] = [
        (
            "NaN nH0",
            InitialData {
                n_h0: f64::NAN,
                ..SMOKE
            },
        ),
        ("zero nH0", InitialData { n_h0: 0.0, ..SMOKE }),
        (
            "xHI0 > 1",
            InitialData {
                x_hi0: 1.5,
                ..SMOKE
            },
        ),
        (
            "xHI0 < 0",
            InitialData {
                x_hi0: -0.1,
                ..SMOKE
            },
        ),
        (
            "fHeI < 0",
            InitialData {
                f_hei: -0.1,
                ..SMOKE
            },
        ),
        (
            "fHeI + fHeII > 1",
            InitialData {
                f_hei: 0.6,
                ..SMOKE
            },
        ),
        (
            "T0 = 0",
            InitialData {
                temperature0: 0.0,
                ..SMOKE
            },
        ),
        (
            "photons < 0",
            InitialData {
                photons_per_h: -1.0,
                ..SMOKE
            },
        ),
        ("negative weight", InitialData { a2: -3.0, ..SMOKE }),
        (
            "infinite alpha",
            InitialData {
                alpha: f64::INFINITY,
                ..SMOKE
            },
        ),
        (
            "overflowing weight",
            InitialData {
                alpha: -1.0e3,
                ..SMOKE
            },
        ),
    ];
    for (name, init) in bad_initial {
        assert!(invalid_input(model.initial_state(&init)), "{name}");
    }

    // Shapes.
    let y0 = smoke_state(&model);
    for (ny, nout) in [(n - 1, n), (n + 1, n), (n, n - 1), (n, n + 1), (0, 0)] {
        let mut out = vec![0.0; nout];
        let state = if ny <= n {
            y0[..ny].to_vec()
        } else {
            let mut s = y0.clone();
            s.push(0.0);
            s
        };
        assert!(
            dimension(model.rhs(0.0, &state, &mut out)),
            "rhs {ny} {nout}"
        );
        assert!(
            dimension(model.partial_t(0.0, &state, &mut out)),
            "partial_t {ny} {nout}"
        );
        if nout == n {
            let mut m = DenseMatrix::zeros(n, n);
            assert!(
                dimension(model.dense_fill(0.0, &state, &mut m)),
                "dense y {ny}"
            );
            assert!(
                dimension(model.diagnostics(0.0, &state)),
                "diagnostics {ny}"
            );
        }
    }
    for (r, c) in [(n - 1, n), (n, n - 1), (n + 1, n + 1), (0, 0)] {
        let mut m = DenseMatrix::zeros(r, c);
        assert!(
            dimension(model.dense_fill(0.0, &y0, &mut m)),
            "dense {r}x{c}"
        );
    }

    // Temperatures outside the table: every callback refuses and counts.
    let b = model.n_ph;
    let u0 = y0[b + slot::U];
    let t_now = model.temperature(&y0);
    let mut states = Vec::new();
    for factor in [
        0.5 * model.table.t_min() / t_now,
        2.0 * model.table.t_max() / t_now,
    ] {
        let mut y = y0.clone();
        y[b + slot::U] = factor * u0;
        states.push(y);
    }
    let mut nan = y0.clone();
    nan[b + slot::U] = f64::NAN;
    states.push(nan);
    let mut empty = y0.clone();
    for k in [slot::HI, slot::HII, slot::HEI, slot::HEII, slot::HEIII] {
        empty[b + k] = 0.0;
    }
    states.push(empty);
    let mut negative = y0.clone();
    negative[b + slot::U] = -u0;
    states.push(negative);
    let problem = chem::problem(model.clone()).unwrap();
    let before = model.domain_refusals();
    let mut out = vec![0.0; n];
    let mut m = DenseMatrix::zeros(n, n);
    let mut counters = WorkCounters::default();
    for (k, y) in states.iter().enumerate() {
        assert!(
            domain_refusal(model.rhs(0.0, y, &mut out)),
            "state {k}: rhs"
        );
        assert!(
            domain_refusal(model.partial_t(0.0, y, &mut out)),
            "state {k}: f_t"
        );
        assert!(
            domain_refusal(model.dense_fill(0.0, y, &mut m)),
            "state {k}: J"
        );
        assert!(
            domain_refusal(problem.eval_rhs_into(0.0, y, &mut out, &mut counters)),
            "state {k}: problem rhs"
        );
        assert_eq!(model.domain_refusals(), before + 4 * (k as u64 + 1));
    }
}

/// Adiabatic cooling without photons from just above `T_min`: the gas
/// leaves the table, every attempt across the boundary is the typed refusal
/// (a rejection, not an error) and the run ends in a partial result at the
/// boundary. The driver approaches the boundary Zeno-like, alternating
/// accepted and refused attempts: with a `min_step` exit (which only a
/// rejection can trigger, accepted steps grow) the last failure is
/// `NonFinite`; with the attempt budget as the exit the last attempt may be
/// an accepted one, and then the last failure is `None`.
#[test]
fn h_a_run_leaving_the_table_ends_in_a_partial_at_the_boundary() {
    let model = Arc::new(model_on(MODERATE));
    let n = model.n();
    let b = model.n_ph;
    let y0 = model
        .initial_state(&InitialData {
            temperature0: 1.02e4,
            photons_per_h: 0.0,
            ..SMOKE
        })
        .unwrap();
    let problem = chem::problem(model.clone()).unwrap();
    let span = (0.0, model.age0);
    let output = OutputSchedule::new(vec![0.0, span.1]).unwrap();
    let mut atol = atol_vector(&model, &y0, 1.0e-8);
    for a in &mut atol[..b] {
        *a = 1.0e-20;
    }
    for (min_step, max_attempts) in [(1.0e3, 100_000), (1.0e-14, 2000)] {
        let config = AdaptiveStepConfig {
            atol: 1.0e-30,
            rtol: 1.0e-6,
            initial_step: 1.0e10,
            min_step,
            max_step: span.1,
            max_attempts,
            ..AdaptiveStepConfig::default()
        };
        let before = model.domain_refusals();
        let (fast, partial) =
            integrate_rodas5p_fast_observed_with_atol(&problem, span, &y0, &config, &output, &atol)
                .unwrap();
        let refusals = model.domain_refusals() - before;
        assert!(!fast.observed.success);
        let partial = partial.expect("a partial result");
        assert_eq!(partial.y.len(), n);
        assert!(partial.t > 0.0 && partial.t < span.1);
        let c = &fast.observed.counters;
        assert!(fast.rejected_steps > 0);
        assert_eq!(c.nonfinite_step_failures as usize, fast.rejected_steps);
        assert_eq!(refusals as usize, fast.rejected_steps);
        if min_step > 1.0 {
            assert_eq!(partial.reason, DriverStopReason::MinimumStep);
            assert_eq!(partial.last_failure, Some(AdaptiveFailureKind::NonFinite));
        } else {
            assert_eq!(partial.reason, DriverStopReason::AttemptBudget);
            assert!(matches!(
                partial.last_failure,
                None | Some(AdaptiveFailureKind::NonFinite)
            ));
        }
        let temperature = model.temperature(&partial.y);
        let t_min = model.table.t_min();
        assert!(temperature >= t_min && temperature < (1.0 + 1.0e-9) * t_min);
        eprintln!(
            "(h) min_step {min_step:e}: partial at t = {:e} s ({:?}, last {:?}), T - T_min = \
             {:e} K, {} attempts ({} accepted, {} refused)",
            partial.t,
            partial.reason,
            partial.last_failure,
            temperature - t_min,
            fast.attempts,
            fast.accepted_steps,
            fast.rejected_steps
        );
    }
}

#[test]
fn pins_transport_matter_rows_and_initial_data() {
    let model = model_on(GRIDS[2]);
    let n = model.n();
    let b = model.n_ph;
    let l = model.layout();
    let mut rng = Lcg::new(0x0919_0004);
    let mut f = vec![0.0; n];
    for _ in 0..10 {
        let (t, y) = random_state(&model, &mut rng);
        model.rhs(t, &y, &mut f).unwrap();
        let bg = model.background(t);
        let s = y[b + slot::S];

        // P02's operator at (H, s): L_lo and L_hi receive transport only.
        let layout = Layout::new(
            l.q_edges.clone(),
            l.mu_edges.clone(),
            support::Coordinate::Physical,
            support::Boundary::Outflow,
        )
        .unwrap();
        let p02 = Model::new(layout, Geometry::Constant { h: bg.h, s }, 0.0, None, false).unwrap();
        let mut tr = vec![0.0; b];
        p02.rhs(t, &y[..b], &mut tr).unwrap();
        assert_eq!(f[Layout::LO].to_bits(), tr[Layout::LO].to_bits());
        assert_eq!(f[l.hi()].to_bits(), tr[l.hi()].to_bits());
        let mut work = 0.0;
        for i in 0..l.nq {
            for j in 0..l.nm {
                work += model.ebar[i] * tr[l.cell(i, j)];
            }
        }
        work += model.ebar[0] * tr[Layout::LO];
        work += model.ebar[l.nq - 1] * tr[l.hi()];
        assert_eq!(f[b + slot::W_GAMMA].to_bits(), (-work).to_bits());

        // Single-term matter rows in MODEL.md's operation order.
        let (d_m, u) = (y[b + slot::DM], y[b + slot::U]);
        let stress = -2.0 * bg.h * d_m - d_m / PARAMS.tau_m - 4.0 * s * u;
        assert_eq!(f[b + slot::DM].to_bits(), stress.to_bits());
        let (a1, a2) = (2.0 * bg.h * u, 2.0 * s * d_m);
        assert_eq!(f[b + slot::WM].to_bits(), (a1 + a2).to_bits());
        let d_gamma = model.moments(&y).d_gamma;
        let shear = -3.0 * bg.h * s + chem::K_D * PARAMS.amp * (d_gamma + d_m) * bg.a_inv3;
        assert_eq!(f[b + slot::S].to_bits(), shear.to_bits());
        let (v, _) = model.table.eval(model.temperature(&y)).unwrap();
        let (n_hii, n_heii, n_heiii) = (y[b + slot::HII], y[b + slot::HEII], y[b + slot::HEIII]);
        let ne_p = (n_hii + n_heii + 2.0 * n_heiii) * bg.a_inv3;
        let r_h = v[0] * ne_p * n_hii;
        let r2 = (v[1] + v[3]) * ne_p * n_heii;
        let r3 = v[2] * ne_p * n_heiii;
        let lam = ne_p * (n_hii * v[4] + n_heii * v[5] + n_heiii * v[6] + n_heii * v[7]) / EV_ERG;
        assert_eq!(f[b + slot::RE].to_bits(), (r_h + r2 + r3).to_bits());
        let escape = BIND[0] * r_h + BIND[1] * r2 + BIND[2] * r3 + lam;
        assert_eq!(f[b + slot::EESC].to_bits(), escape.to_bits());
    }

    // Section 9.
    let y0 = model.initial_state(&SMOKE).unwrap();
    let n_he = SMOKE.n_h0 * chem::Y_HE / (4.0 * (1.0 - chem::Y_HE));
    assert_eq!(y0[b + slot::HI].to_bits(), (0.6 * 2.4e-5_f64).to_bits());
    assert_eq!(y0[b + slot::HEI].to_bits(), (0.2 * n_he).to_bits());
    assert_eq!(
        y0[b + slot::S].to_bits(),
        (SMOKE.s0_over_h0 * PARAMS.h0).to_bits()
    );
    for k in [slot::W_GAMMA, slot::DM, slot::WM, slot::EESC, slot::RE] {
        assert_eq!(y0[b + k], 0.0);
    }
    for i in 0..l.nq {
        assert_eq!(y0[l.absorbed(i)], 0.0);
    }
    assert_eq!((y0[Layout::LO], y0[l.hi()]), (0.0, 0.0));
    let m = model.moments(&y0);
    let total = SMOKE.photons_per_h * SMOKE.n_h0;
    assert!((m.n_gamma - total).abs() <= 64.0 * EPS * total);
    let temperature = model.temperature(&y0);
    assert!((temperature - SMOKE.temperature0).abs() <= 8.0 * EPS * SMOKE.temperature0);
    let d = model.diagnostics(0.0, &y0).unwrap();
    assert!((d.s_over_h - SMOKE.s0_over_h0).abs() <= 4.0 * EPS * SMOKE.s0_over_h0);
    assert!((d.x_hii - 0.4).abs() <= 4.0 * EPS);
    assert!((d.x_heii - 0.5).abs() <= 4.0 * EPS && (d.x_heiii - 0.3).abs() <= 8.0 * EPS);
    assert!(d.moments.d_gamma > 0.0, "a2 > 0 gives positive D_gamma");
    assert_eq!(d.ledgers.photon.to_bits(), m.n_gamma.to_bits());
}
