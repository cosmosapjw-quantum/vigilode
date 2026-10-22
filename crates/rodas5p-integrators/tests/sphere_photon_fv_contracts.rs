//! Contracts of the fixed-grid full-sphere photon transport adapter
//! (BASS/rei DAG node P06, `examples/sphere_photon_fv_support`) against the
//! formula oracle of `research/bassrei_p06_full_sphere_scoped_20261022`
//! (normative: `FULL_SPHERE_THEORY.md`) and its runs through the RODAS5P
//! fast drivers:
//!
//! * (a) parity: the inputs, every grid constant, the transport (outflow,
//!   closed, select), one Thomson group and the full state RHS of the
//!   oracle's golden fixture (`CHECKS.json -> diagnostics.parity_fixture`)
//!   bit for bit (Python `float.hex`), then the fixture's output digest.
//! * (b) every column of the in-place dense Jacobian is the RHS of the unit
//!   vector bit for bit, the packed band (through the model and through the
//!   `support::band` declaration) is the dense matrix on the band bit for
//!   bit, nothing lies outside the band `M + 1`, the band is attained, and
//!   a refill over a used or poisoned matrix equals a fresh fill.
//! * (c) number ledger: `|sum f| <= 1e-14 A` with `A = sum_{r,c} |J_rc y_c|`
//!   (P02 C3), for outflow (Thomson, absorber) and closed boundaries; a
//!   closed grid without absorber leaves every ledger slot exactly zero.
//! * (d) Thomson: the isotropic state is stationary and the group number is
//!   conserved (the oracle's `d2`/`d3` bounds), also inside the state RHS.
//! * (e) LRS reduction: with `H1 == H2` the phi-summed RHS on axisymmetric
//!   data equals the P02 adapter's RHS to `1e-14` relative (not bitwise: the
//!   flux forms differ).
//! * (f) VG-1: the callbacks allocate nothing, and the dense and banded
//!   drivers allocate the same count at two tolerances whose attempt counts
//!   differ at least twice.
//! * (g) VG-3: repeated runs are bit for bit identical and a uniform vector
//!   atol is the scalar run bit for bit.
//! * (h) every invalid grid, geometry, rate and shape is a typed refusal
//!   with its stable code.
//! * Sanity: the exact `f_t` against central differences, the JVP callback
//!   and the autonomy flag.

use std::alloc::{GlobalAlloc, Layout as AllocLayout, System};
use std::cell::Cell;
use std::f64::consts::TAU;
use std::sync::Arc;

use rodas5p_core::{CoreError, CoreResult, DenseMatrix, WorkCounters, sha256_hex};
use rodas5p_integrators::{
    AdaptiveStepConfig, OutputSchedule, Rodas5pFastBandedResult, Rodas5pFastLu, Rodas5pFastResult,
    integrate_rodas5p_fast_banded_observed, integrate_rodas5p_fast_banded_observed_with_atol,
    integrate_rodas5p_fast_observed, integrate_rodas5p_fast_observed_with_atol,
};
use serde_json::Value;

#[path = "../examples/sphere_photon_fv_support/mod.rs"]
mod support;
use support::{
    Absorber, Boundary, Coordinate, Geometry3, Layout3, Model3, float_hex, refusal_code,
    uniform_edges, uniform_mu_edges, uniform_phi_edges,
};

#[path = "../examples/lrs_photon_fv_support/mod.rs"]
mod lrs_support;

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

    fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.uniform()
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

/// Strictly increasing edges `lo .. hi` with relative widths in
/// `[min_frac, 1)` (the oracle's `_random_edges`).
fn random_edges(rng: &mut XorShift, lo: f64, hi: f64, n: usize) -> Vec<f64> {
    let w: Vec<f64> = (0..n).map(|_| 0.25 + 0.75 * rng.uniform()).collect();
    let total: f64 = w.iter().sum();
    let mut edges = vec![lo];
    let mut acc = 0.0;
    for x in &w[..n - 1] {
        acc += x;
        edges.push(lo + (hi - lo) * acc / total);
    }
    edges.push(hi);
    edges
}

/// A periodic cover `offset .. offset + TAU` (the oracle's
/// `_random_phi_edges`).
fn random_phi_edges(rng: &mut XorShift, n: usize, offset: f64) -> Vec<f64> {
    let w: Vec<f64> = (0..n).map(|_| 0.25 + 0.75 * rng.uniform()).collect();
    let total: f64 = w.iter().sum();
    let mut edges = vec![offset];
    let mut acc = 0.0;
    for x in &w[..n - 1] {
        acc += x;
        edges.push(offset + TAU * acc / total);
    }
    edges.push(offset + TAU);
    edges
}

struct Grid3 {
    name: &'static str,
    q: Vec<f64>,
    mu: Vec<f64>,
    phi: Vec<f64>,
}

impl Grid3 {
    /// Midpoint of the middle energy cell (a threshold inside a cell).
    fn mid(&self) -> f64 {
        let i = (self.q.len() - 1) / 2;
        0.5 * (self.q[i] + self.q[i + 1])
    }

    fn layout(&self, coordinate: Coordinate, boundary: Boundary) -> Layout3 {
        Layout3::new(
            self.q.clone(),
            self.mu.clone(),
            self.phi.clone(),
            coordinate,
            boundary,
        )
        .unwrap()
    }
}

fn fixture_grid() -> Grid3 {
    Grid3 {
        name: "fixture3x3x4",
        q: vec![-1.0, -0.25, 0.5, 1.75],
        mu: vec![-1.0, -0.375, 0.25, 1.0],
        phi: uniform_phi_edges(4, 0.25).unwrap(),
    }
}

/// Grids with `nphi = 1, 2, 4, 5, 6, 7`, offsets `0`, `0.25`, `0.3`, `2.9`,
/// `4.0`, `5.5` (a wrap face carrying flux) and random widths.
fn grids() -> Vec<Grid3> {
    let mut rng = XorShift::new(0x0006_9e1d);
    vec![
        fixture_grid(),
        Grid3 {
            name: "nphi1",
            q: uniform_edges(-1.0, 1.0, 3).unwrap(),
            mu: uniform_mu_edges(3).unwrap(),
            phi: vec![0.0, TAU],
        },
        Grid3 {
            name: "random2x3x2",
            q: random_edges(&mut rng, -1.0, 1.0, 2),
            mu: random_edges(&mut rng, -1.0, 1.0, 3),
            phi: random_phi_edges(&mut rng, 2, 4.0),
        },
        Grid3 {
            name: "uniform5x4x6",
            q: uniform_edges(-2.0, 1.5, 5).unwrap(),
            mu: uniform_mu_edges(4).unwrap(),
            phi: uniform_phi_edges(6, 0.3).unwrap(),
        },
        Grid3 {
            name: "random4x3x5",
            q: random_edges(&mut rng, -2.0, 1.5, 4),
            mu: random_edges(&mut rng, -1.0, 1.0, 3),
            phi: random_phi_edges(&mut rng, 5, 2.9),
        },
        Grid3 {
            name: "random3x2x7",
            q: random_edges(&mut rng, -1.5, 1.0, 3),
            mu: random_edges(&mut rng, -1.0, 1.0, 2),
            phi: random_phi_edges(&mut rng, 7, 5.5),
        },
        Grid3 {
            name: "random3x4x5_phi0",
            q: random_edges(&mut rng, -1.0, 1.0, 3),
            mu: random_edges(&mut rng, -1.0, 1.0, 4),
            phi: random_phi_edges(&mut rng, 5, 0.0),
        },
    ]
}

const GEOMETRIES: [Geometry3; 4] = [
    Geometry3::Constant {
        h1: 0.3,
        h2: -0.1,
        h3: 0.05,
    },
    Geometry3::Constant {
        h1: -0.2,
        h2: 0.4,
        h3: 0.1,
    },
    // LRS (H1 == H2): every phi flux is a signed zero.
    Geometry3::Constant {
        h1: 0.2,
        h2: 0.2,
        h3: -0.1,
    },
    Geometry3::PowerLaw {
        hh1: 0.5,
        hh2: 0.1,
        hh3: -0.3,
    },
];

const COORDINATES: [(Coordinate, Boundary); 4] = [
    (Coordinate::Physical, Boundary::Outflow),
    (Coordinate::Physical, Boundary::Closed),
    (Coordinate::Comoving, Boundary::Outflow),
    (Coordinate::Comoving, Boundary::Closed),
];

fn model_on(
    grid: &Grid3,
    coordinate: Coordinate,
    boundary: Boundary,
    geometry: Geometry3,
    gamma: f64,
    absorber: Option<Absorber>,
) -> Model3 {
    Model3::new(
        grid.layout(coordinate, boundary),
        geometry,
        gamma,
        absorber,
        false,
    )
    .unwrap()
}

/// Dense `J(t)` from a fresh zero matrix.
fn fresh_dense(model: &Model3, t: f64) -> DenseMatrix {
    let n = model.layout.n();
    let mut m = DenseMatrix::zeros(n, n);
    model.dense_fill(t, &mut m).unwrap();
    m
}

fn positive_state(rng: &mut XorShift, n: usize) -> Vec<f64> {
    (0..n).map(|_| 0.01 + rng.uniform()).collect()
}

// ---------------------------------------------------------------------------
// (a) Golden fixture parity
// ---------------------------------------------------------------------------

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/bassrei_p06_full_sphere_scoped_20261022/CHECKS.json"
    );
    let text = std::fs::read_to_string(path).expect("the oracle's CHECKS.json must be readable");
    let checks: Value = serde_json::from_str(&text).expect("CHECKS.json must be JSON");
    assert_eq!(checks["status"], "pass_scoped", "the oracle run must pass");
    checks["diagnostics"]["parity_fixture"].clone()
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .expect("hex list")
        .iter()
        .map(|s| s.as_str().expect("hex string").to_string())
        .collect()
}

/// Parses a Python `float.hex` string bit-exactly.
fn parse_hex(s: &str) -> f64 {
    let (negative, body) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s),
    };
    let body = body.strip_prefix("0x").expect("0x prefix");
    let (mantissa, exponent) = body.split_once('p').expect("p exponent");
    let (lead, frac) = mantissa.split_once('.').unwrap_or((mantissa, "0"));
    let exponent: i64 = exponent.parse().expect("exponent");
    assert!(frac.len() <= 13, "{s}: too many hex digits");
    let frac_bits = u64::from_str_radix(frac, 16).expect("hex digits") << (4 * (13 - frac.len()));
    let sign = if negative { 1_u64 << 63 } else { 0 };
    let magnitude = match lead {
        "1" => (((exponent + 1023) as u64) << 52) | frac_bits,
        "0" if frac_bits == 0 => 0,
        "0" => {
            assert_eq!(exponent, -1022, "{s}: subnormal exponent");
            frac_bits
        }
        _ => panic!("{s}: unexpected leading digit"),
    };
    f64::from_bits(sign | magnitude)
}

/// Distance in units in the last place (same sign).
fn ulps(a: f64, b: f64) -> u64 {
    let key = |x: f64| {
        let b = x.to_bits() as i64;
        if b < 0 { i64::MIN - b } else { b }
    };
    key(a).abs_diff(key(b))
}

/// Element-wise hex comparison; mismatches are recorded with their ulp
/// distance.
fn compare(name: &str, got: &[f64], want: &[String], mismatches: &mut Vec<String>) -> usize {
    assert_eq!(got.len(), want.len(), "{name}: length");
    for (k, (g, w)) in got.iter().zip(want).enumerate() {
        let h = float_hex(*g);
        if &h != w {
            mismatches.push(format!(
                "{name}[{k}]: rust {h} oracle {w} ({} ulp)",
                ulps(*g, parse_hex(w))
            ));
        }
    }
    got.len()
}

#[test]
fn a_golden_fixture_is_bitwise() {
    let pf = fixture();
    let grid = fixture_grid();
    let h = [0.3, -0.1, 0.05];
    let select = [-0.2, 0.4, 0.1];
    let rate = 2.5;
    let kbar = [0.2, 0.0, 0.7];
    let mut mismatches = Vec::new();
    let mut compared = 0usize;

    // Inputs: the literals and the uniform phi helper.
    let inputs = &pf["inputs"];
    compared += compare("inputs.q", &grid.q, &strings(&inputs["q"]), &mut mismatches);
    compared += compare(
        "inputs.mu",
        &grid.mu,
        &strings(&inputs["mu"]),
        &mut mismatches,
    );
    compared += compare(
        "inputs.phi",
        &grid.phi,
        &strings(&inputs["phi"]),
        &mut mismatches,
    );
    compared += compare("inputs.H", &h, &strings(&inputs["H"]), &mut mismatches);
    compared += compare(
        "inputs.select",
        &select,
        &strings(&inputs["select"]),
        &mut mismatches,
    );
    compared += compare(
        "inputs.rate",
        &[rate],
        &[inputs["rate"].as_str().unwrap().to_string()],
        &mut mismatches,
    );
    compared += compare(
        "inputs.kbar",
        &kbar,
        &strings(&inputs["kbar"]),
        &mut mismatches,
    );
    assert_eq!(
        inputs["contents_formula"],
        "1 + (c % 7)*0.125 - (c % 3)*0.0625"
    );

    // Grid constants.
    let layout = grid.layout(Coordinate::Physical, Boundary::Outflow);
    let constants = &pf["constants"];
    for (name, v) in [
        ("dq", &layout.dq),
        ("dm", &layout.dm),
        ("dphi", &layout.dphi),
        ("m2", &layout.m2),
        ("t2", &layout.t2),
        ("p2", &layout.p2),
        ("r", &layout.r),
        ("e", &layout.e),
        ("g", &layout.g),
        ("cphi", &layout.cphi),
        ("sphi", &layout.sphi),
        ("c2", &layout.c2),
        ("s2h", &layout.s2h),
        ("wface", &layout.wface),
        ("sn2", &layout.sn2),
        ("om", &layout.om),
    ] {
        compared += compare(
            &format!("constants.{name}"),
            v,
            &strings(&constants[name]),
            &mut mismatches,
        );
    }
    let b_rows = constants["B"].as_array().unwrap();
    assert_eq!(b_rows.len(), 5);
    for (m, row) in b_rows.iter().enumerate() {
        let got: Vec<f64> = layout.b.iter().map(|b| b[m]).collect();
        compared += compare(
            &format!("constants.B[{m}]"),
            &got,
            &strings(row),
            &mut mismatches,
        );
    }
    // Aw = w_m B_m (not in the fixture; derived exactly as the oracle).
    for (a, b) in layout.aw.iter().zip(&layout.b) {
        for m in 0..5 {
            assert_eq!(a[m].to_bits(), (support::QUAD_WEIGHTS[m] * b[m]).to_bits());
        }
    }

    // Contents and state.
    let m = layout.m;
    let nq = layout.nq;
    let x: Vec<f64> = (0..nq * m)
        .map(|c| 1.0 + (c % 7) as f64 * 0.125 - (c % 3) as f64 * 0.0625)
        .collect();
    let n = layout.n();
    let mut y = vec![0.0; n];
    for i in 0..nq {
        for c in 0..m {
            y[layout.slot(i, c)] = x[i * m + c];
        }
    }
    let cells_and_ledgers = |model: &Model3, out: &[f64]| {
        let l = &model.layout;
        let mut v: Vec<f64> = (0..l.nq)
            .flat_map(|i| (0..l.m).map(move |c| (i, c)))
            .map(|(i, c)| out[l.slot(i, c)])
            .collect();
        v.push(out[Layout3::LO]);
        v.push(out[l.hi()]);
        v
    };
    let geometry = Geometry3::Constant {
        h1: h[0],
        h2: h[1],
        h3: h[2],
    };
    let mut outputs: Vec<(&str, Vec<String>)> = Vec::new();
    let mut out = vec![0.0; n];
    for (key, boundary) in [
        ("transport_outflow", Boundary::Outflow),
        ("transport_closed", Boundary::Closed),
    ] {
        let model = model_on(&grid, Coordinate::Physical, boundary, geometry, 0.0, None);
        model.rhs(0.0, &y, &mut out).unwrap();
        let got = cells_and_ledgers(&model, &out);
        compared += compare(key, &got, &strings(&pf[key]), &mut mismatches);
        outputs.push((key, got.iter().map(|v| float_hex(*v)).collect()));
        // The select path with select == velocity is the RHS bit for bit.
        let mut via = vec![0.0; n];
        model.transport_with_select(h, h, &y, &mut via).unwrap();
        assert_eq!(bits(&via), bits(&out), "{key}: select == velocity");
    }
    let model = model_on(
        &grid,
        Coordinate::Physical,
        Boundary::Outflow,
        geometry,
        rate,
        None,
    );
    model
        .transport_with_select(h, select, &y, &mut out)
        .unwrap();
    let got = cells_and_ledgers(&model, &out);
    compared += compare(
        "transport_select",
        &got,
        &strings(&pf["transport_select"]),
        &mut mismatches,
    );
    outputs.push((
        "transport_select",
        got.iter().map(|v| float_hex(*v)).collect(),
    ));
    let mut group = vec![0.0; m];
    model.thomson_group_rhs(&x[..m], rate, &mut group).unwrap();
    compared += compare(
        "thomson6_group0",
        &group,
        &strings(&pf["thomson6_group0"]),
        &mut mismatches,
    );
    outputs.push((
        "thomson6_group0",
        group.iter().map(|v| float_hex(*v)).collect(),
    ));
    model
        .rhs_with_group_rates(0.0, &y, &kbar, &mut out)
        .unwrap();
    compared += compare(
        "state_rhs",
        &out,
        &strings(&pf["state_rhs"]),
        &mut mismatches,
    );
    outputs.push(("state_rhs", out.iter().map(|v| float_hex(*v)).collect()));

    assert!(
        mismatches.is_empty(),
        "golden fixture: {} of {compared} values differ:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );

    // The digest of the five outputs, as the oracle forms it.
    let mut text = String::new();
    for (_, hexes) in &outputs {
        text.push_str(&hexes.join("\n"));
        text.push('\n');
    }
    let digest = sha256_hex(text.as_bytes());
    assert_eq!(
        digest,
        pf["sha256_outputs"].as_str().unwrap(),
        "output digest"
    );
    assert_eq!(
        digest,
        "1514967febca0332cee1775d44d711d57996ca556fce57aa680cdbd437db941a"
    );
    eprintln!("(a) golden fixture: {compared} values bitwise equal; digest {digest}");
}

// ---------------------------------------------------------------------------
// (b) Jacobian columns and band
// ---------------------------------------------------------------------------

#[test]
fn b_dense_columns_are_unit_rhs_bitwise_and_the_band_is_exact() {
    let mut rng = XorShift::new(0x00b6_b6b6);
    let (mut cases, mut columns, mut entries) = (0usize, 0usize, 0usize);
    let mut max_linear_rel = 0.0_f64;
    for grid in grids() {
        let mut reused: Option<DenseMatrix> = None;
        let absorbers = [
            None,
            Some(Absorber {
                k0: 2.0,
                q_chi: grid.mid(),
            }),
        ];
        for (coordinate, boundary) in COORDINATES {
            for gamma in [0.0, 2.5] {
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
                            let l = &model.layout;
                            let n = l.n();
                            let bw = l.bandwidth();
                            assert_eq!(bw, l.m + 1);
                            let declared = support::band(model.clone());
                            assert_eq!((declared.lower, declared.upper), (bw, bw), "{label}");
                            let dense = fresh_dense(&model, t);

                            // Column c is f(t, e_c), bit for bit.
                            let mut e = vec![0.0; n];
                            let mut f = vec![0.0; n];
                            for c in 0..n {
                                e[c] = 1.0;
                                model.rhs(t, &e, &mut f).unwrap();
                                e[c] = 0.0;
                                for (r, fr) in f.iter().enumerate() {
                                    assert_eq!(
                                        fr.to_bits(),
                                        dense[(r, c)].to_bits(),
                                        "{label}: J[{r},{c}]={} vs f(e_c)={fr}",
                                        dense[(r, c)]
                                    );
                                }
                                columns += 1;
                                entries += n;
                            }

                            // Nothing outside the band; the band is attained
                            // whenever an energy face couples two cells.
                            let mut observed = 0usize;
                            for r in 0..n {
                                for c in 0..n {
                                    if dense[(r, c)] != 0.0 {
                                        observed = observed.max(r.abs_diff(c));
                                    }
                                }
                            }
                            assert!(observed <= bw, "{label}: observed band {observed} > {bw}");
                            if boundary == Boundary::Outflow || l.nq >= 2 {
                                assert_eq!(observed, bw, "{label}: band not attained");
                            }

                            // The packed band is the dense matrix on the band,
                            // through the model and through the declaration.
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

                            // A refill over the previous case's matrix, then
                            // over a band poisoned with NaN, equals the fresh
                            // fill.
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
    eprintln!(
        "(b) {cases} cases, {columns} columns, {entries} entries bitwise; \
         max J v vs f(v) rel dev {max_linear_rel:e}"
    );
}

// ---------------------------------------------------------------------------
// (c) Number ledger
// ---------------------------------------------------------------------------

/// `A = sum_{r,c} |J_rc y_c|` at `t`.
fn absolute_terms(model: &Model3, t: f64, y: &[f64]) -> f64 {
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
fn c_rhs_conserves_the_number_ledger() {
    let mut rng = XorShift::new(0x00c6_c6c6);
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
                for _ in 0..30 {
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
                    for _ in 0..30 {
                        let t = 2.0 * rng.uniform();
                        let y = positive_state(&mut rng, n);
                        closed.rhs(t, &y, &mut f).unwrap();
                        assert!(f[Layout3::LO].to_bits() == 0 && f[l.hi()].to_bits() == 0);
                        for i in 0..l.nq {
                            assert!(f[l.absorbed(i)].to_bits() == 0);
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
        "(c) {states} states; outflow max |sum f|/A {worst_open_a:e} (/sum|f| {worst_open_f:e}); \
         closed max |sum f|/A {worst_closed_a:e} (/sum|f| {worst_closed_f:e})"
    );
}

// ---------------------------------------------------------------------------
// (d) Thomson
// ---------------------------------------------------------------------------

#[test]
fn d_thomson_isotropic_state_is_stationary_and_conserves_the_group_number() {
    let mut rng = XorShift::new(0x00d6_d6d6);
    let (mut worst_iso, mut worst_number, mut worst_state) = (0.0_f64, 0.0_f64, 0.0_f64);
    for case in 0..60 {
        let nm = 1 + (rng.next_u64() % 8) as usize;
        let nphi = 1 + (rng.next_u64() % 10) as usize;
        let mu = random_edges(&mut rng, -1.0, 1.0, nm);
        let offset = rng.range(0.0, 6.0);
        let phi = random_phi_edges(&mut rng, nphi, offset);
        let grid = Grid3 {
            name: "thomson",
            q: vec![0.0, 0.5, 1.0],
            mu,
            phi,
        };
        let geometry = Geometry3::Constant {
            h1: 0.0,
            h2: 0.0,
            h3: 0.0,
        };
        let gamma = if case % 2 == 0 {
            rng.range(0.5, 50.0)
        } else {
            rng.range(0.1, 1.0e6)
        };
        let model = model_on(
            &grid,
            Coordinate::Physical,
            Boundary::Closed,
            geometry,
            gamma,
            None,
        );
        let l = &model.layout;
        let m = l.m;
        let mut out = vec![0.0; m];

        // Isotropic N_c = dm_j dphi_k is stationary (the oracle's d3 bound).
        let iso: Vec<f64> = (0..nm)
            .flat_map(|j| (0..nphi).map(move |k| (j, k)))
            .map(|(j, k)| l.dm[j] * l.dphi[k])
            .collect();
        model.thomson_group_rhs(&iso, gamma, &mut out).unwrap();
        let err = max_abs(&out) / (gamma * max_abs(&iso));
        assert!(
            err <= 1.0e-14,
            "isotropic residual {err:e} (nm={nm}, nphi={nphi})"
        );
        worst_iso = worst_iso.max(err);

        // Group number (the oracle's d2 bound), on signed contents.
        let x: Vec<f64> = (0..m).map(|_| rng.range(-1.0, 4.0)).collect();
        model.thomson_group_rhs(&x, gamma, &mut out).unwrap();
        let scale: f64 = x.iter().map(|v| v.abs()).sum();
        let err = compensated_sum(&out).abs() / (gamma * scale);
        assert!(err <= 1.0e-14, "group number residual {err:e}");
        worst_number = worst_number.max(err);

        // Inside the state RHS (frozen geometry, closed): each group's
        // number is conserved and the ledgers stay exactly zero.
        let n = l.n();
        let y: Vec<f64> = (0..n).map(|_| rng.range(0.0, 3.0)).collect();
        let mut f = vec![0.0; n];
        model.rhs(0.3, &y, &mut f).unwrap();
        for i in 0..l.nq {
            let base = l.slot(i, 0);
            let s = compensated_sum(&f[base..base + m]).abs();
            let scale: f64 = y[base..base + m].iter().map(|v| v.abs()).sum();
            let err = s / (gamma * scale);
            assert!(err <= 1.0e-14, "state group {i} residual {err:e}");
            worst_state = worst_state.max(err);
            assert_eq!(f[l.absorbed(i)].to_bits(), 0);
        }
        assert_eq!(f[Layout3::LO].to_bits(), 0);
        assert_eq!(f[l.hi()].to_bits(), 0);
    }
    eprintln!(
        "(d) isotropic max {worst_iso:e}; group number max {worst_number:e}; \
         state groups max {worst_state:e}"
    );
}

// ---------------------------------------------------------------------------
// (e) LRS reduction against the P02 adapter
// ---------------------------------------------------------------------------

#[test]
fn e_phi_summed_rhs_equals_the_lrs_adapter_on_axisymmetric_data() {
    let mut rng = XorShift::new(0x00e6_e6e6);
    let phi_grids = [
        vec![0.0, TAU],
        uniform_phi_edges(2, 0.0).unwrap(),
        uniform_phi_edges(3, 0.4).unwrap(),
        random_phi_edges(&mut rng, 7, 1.1),
        uniform_phi_edges(16, 0.0).unwrap(),
        random_phi_edges(&mut rng, 5, 6.0),
    ];
    // (h, s) of the P02 geometry: H1 = H2 = h - s, H3 = h + 2 s.
    let geometries = [
        (0.1, 0.07),
        (-0.2, 0.05),
        (0.03, -0.3),
        (0.4, 0.4),
        (0.0, 0.0),
        (-0.1, -0.08),
    ];
    let (mut cases, mut worst) = (0usize, 0.0_f64);
    let (mut ties, mut zero_operators, mut zero_drift) = (0usize, 0usize, 0usize);
    let (mut worst_plain, mut worst_transport) = (0.0_f64, 0.0_f64);
    for phi in &phi_grids {
        for (h, s) in geometries {
            let nq = 1 + (rng.next_u64() % 5) as usize;
            let nm = 1 + (rng.next_u64() % 6) as usize;
            let q = random_edges(&mut rng, -2.0, 2.0, nq);
            let mu = random_edges(&mut rng, -1.0, 1.0, nm);
            let n2: Vec<f64> = (0..nq * nm).map(|_| rng.range(0.0, 5.0)).collect();
            for (coordinate, boundary) in COORDINATES {
                if coordinate == Coordinate::Comoving && s == 0.0 {
                    // sigma = H_k - (H1 + H2 + H3)/3 is a rounding residue
                    // there while the P02 shear is exactly zero.
                    continue;
                }
                for gamma in [0.0, 3.0] {
                    let mid = 0.5 * (q[nq / 2] + q[nq / 2 + 1]);
                    for with_absorber in [false, true] {
                        let label = format!(
                            "nphi={} (h,s)=({h},{s}) {coordinate:?} {boundary:?} gamma={gamma} \
                             absorber={with_absorber} nq={nq} nm={nm}",
                            phi.len() - 1
                        );
                        let t = 0.7;
                        let lrs_coordinate = match coordinate {
                            Coordinate::Physical => lrs_support::Coordinate::Physical,
                            Coordinate::Comoving => lrs_support::Coordinate::Comoving,
                        };
                        let lrs_boundary = match boundary {
                            Boundary::Outflow => lrs_support::Boundary::Outflow,
                            Boundary::Closed => lrs_support::Boundary::Closed,
                        };
                        let lrs = lrs_support::Model::new(
                            lrs_support::Layout::new(
                                q.clone(),
                                mu.clone(),
                                lrs_coordinate,
                                lrs_boundary,
                            )
                            .unwrap(),
                            lrs_support::Geometry::Constant { h, s },
                            gamma,
                            with_absorber.then_some(lrs_support::Absorber {
                                k0: 2.0,
                                q_chi: mid,
                            }),
                            false,
                        )
                        .unwrap();
                        let sphere = Model3::new(
                            Layout3::new(q.clone(), mu.clone(), phi.clone(), coordinate, boundary)
                                .unwrap(),
                            Geometry3::Constant {
                                h1: h - s,
                                h2: h - s,
                                h3: h + 2.0 * s,
                            },
                            gamma,
                            with_absorber.then_some(Absorber {
                                k0: 2.0,
                                q_chi: mid,
                            }),
                            false,
                        )
                        .unwrap();
                        let (l2, l3) = (&lrs.layout, &sphere.layout);
                        // Both adapters take the same upwind cell on every
                        // energy face, except on a column whose velocity is
                        // a rounding residue of zero in both (theory 6.7:
                        // the flux difference is then |v| N / dq, at the
                        // rounding level); such a tie must be rounding level.
                        let (v3, _) = sphere.drift_at(t).unwrap();
                        let (vq, _, vphi) = l3.face_velocities(v3);
                        let h_eff = if coordinate == Coordinate::Physical {
                            h
                        } else {
                            0.0
                        };
                        let speed = h_eff.abs() + 3.0 * s.abs();
                        for j in 0..nm {
                            let v2 = -h_eff - s * l2.c[j];
                            for k in 0..l3.nphi {
                                let v = vq[j * l3.nphi + k];
                                if (v2 >= 0.0) != (v >= 0.0) {
                                    assert!(
                                        v2.abs().max(v.abs()) <= 64.0 * EPS * speed,
                                        "{label}: upwind differs at column {j} \
                                         (v2={v2:e}, v3={v:e})"
                                    );
                                    ties += 1;
                                }
                            }
                        }
                        assert!(vphi.iter().all(|v| *v == 0.0), "{label}: phi drift");

                        let mut y2 = vec![0.0; l2.n()];
                        let mut y3 = vec![0.0; l3.n()];
                        for i in 0..nq {
                            for j in 0..nm {
                                y2[l2.cell(i, j)] = n2[i * nm + j];
                                for k in 0..l3.nphi {
                                    y3[l3.cell(i, j, k)] = n2[i * nm + j] * l3.g[k];
                                }
                            }
                        }
                        let mut f2 = vec![0.0; l2.n()];
                        let mut f3 = vec![0.0; l3.n()];
                        lrs.rhs(t, &y2, &mut f2).unwrap();
                        sphere.rhs(t, &y3, &mut f3).unwrap();
                        let scale = max_abs(&f2);
                        let mut dev = 0.0_f64;
                        for i in 0..nq {
                            for j in 0..nm {
                                let row: Vec<f64> =
                                    (0..l3.nphi).map(|k| f3[l3.cell(i, j, k)]).collect();
                                dev = dev.max((compensated_sum(&row) - f2[l2.cell(i, j)]).abs());
                            }
                            dev = dev.max((f3[l3.absorbed(i)] - f2[l2.absorbed(i)]).abs());
                        }
                        dev = dev.max((f3[lrs_support::Layout::LO] - f2[Layout3::LO]).abs());
                        dev = dev.max((f3[l3.hi()] - f2[l2.hi()]).abs());
                        // The reference is max(max |f2|, T) with the natural
                        // term scale T = max_ij N_ij (speed (1/dq_i + 1/dm_j)
                        // + gamma + kbar_max) of the operator (the oracle's
                        // f1 normalizes Thomson by gamma max N). A row's RHS
                        // can be a small difference of terms of size T: the
                        // Thomson gain and loss of the single mu cell
                        // [-1, 1] cancel exactly in reals (P02 folds them
                        // into one Jacobian entry, the sphere's gain carries
                        // sum_k g_k = 1 to rounding), and so does the
                        // comoving drift averaged over that cell.
                        let kmax = (0..nq).fold(0.0_f64, |m, i| m.max(lrs.kbar(i, t).0));
                        let mut term = 0.0_f64;
                        for i in 0..nq {
                            for j in 0..nm {
                                let rate = speed * (1.0 / l2.dq[i] + 1.0 / l2.dm[j]) + gamma + kmax;
                                term = term.max(n2[i * nm + j] * rate);
                            }
                        }
                        if term == 0.0 {
                            // No term at all (h = s = 0, no Thomson, no
                            // absorber): both RHS vanish exactly.
                            assert_eq!(max_abs(&f2), 0.0, "{label}");
                            assert_eq!(max_abs(&f3), 0.0, "{label}");
                            zero_operators += 1;
                            continue;
                        }
                        // Transport alone (the oracle's c1): relative to
                        // max |f2|, unless the operator vanishes in exact
                        // arithmetic (max |f2| a rounding residue of T).
                        // With Thomson or the absorber (the oracle's f1):
                        // relative to max(max |f2|, T).
                        let transport_only = gamma == 0.0 && !with_absorber;
                        let reference = if transport_only && scale > 64.0 * EPS * term {
                            scale
                        } else {
                            if transport_only {
                                zero_drift += 1;
                            }
                            scale.max(term)
                        };
                        let rel = dev / reference;
                        assert!(
                            rel <= 1.0e-14,
                            "{label}: dev {dev:e}, max|f2| {scale:e}, T {term:e}: rel {rel:e}"
                        );
                        if transport_only {
                            worst_transport = worst_transport.max(rel);
                        } else {
                            worst = worst.max(rel);
                            if scale > 0.0 {
                                worst_plain = worst_plain.max(dev / scale);
                            }
                        }
                        cases += 1;
                    }
                }
            }
        }
    }
    assert!(cases >= 400, "too few compared cases: {cases}");
    eprintln!(
        "(e) {cases} cases; transport alone: max deviation from the P02 adapter / max|f2| \
         {worst_transport:e} ({zero_drift} drifts zero in exact arithmetic, / T); with Thomson or \
         absorber: / max(max|f2|, T) {worst:e} (/max|f2| alone {worst_plain:e}); \
         {ties} rounding-level upwind ties; {zero_operators} exactly zero operators"
    );
}

// ---------------------------------------------------------------------------
// (f) VG-1 and (g) VG-3 on a non-autonomous triaxial scenario
// ---------------------------------------------------------------------------

const SPAN: (f64, f64) = (0.0, 3.0);

/// Comoving 12 x 3 x 5 grid (offset phi), triaxial power-law geometry,
/// Thomson, a moving absorption threshold.
fn scenario() -> (Arc<Model3>, Vec<f64>) {
    let layout = Layout3::new(
        uniform_edges(-3.0, 2.0, 12).unwrap(),
        uniform_mu_edges(3).unwrap(),
        uniform_phi_edges(5, 0.3).unwrap(),
        Coordinate::Comoving,
        Boundary::Outflow,
    )
    .unwrap();
    let model = Model3::new(
        layout,
        Geometry3::PowerLaw {
            hh1: 0.5,
            hh2: 0.2,
            hh3: -0.1,
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
            let st = ((1.0 - mu) * (1.0 + mu)).sqrt();
            for k in 0..l.nphi {
                let phi = 0.5 * (l.phi_edges[k] + l.phi_edges[k + 1]);
                let (n1, n2) = (st * phi.cos(), st * phi.sin());
                y0[l.cell(i, j, k)] = l.dq[i]
                    * l.dm[j]
                    * l.dphi[k]
                    * (-(q * q) / (2.0 * 0.36)).exp()
                    * (1.0 + 0.3 * n1 + 0.2 * mu + 0.1 * n1 * n2);
            }
        }
    }
    (Arc::new(model), y0)
}

/// Nonuniform vector atol: `scale` times the cell measure on cells, `scale`
/// on the ledger slots.
fn scenario_atol(model: &Model3, scale: f64) -> Vec<f64> {
    let l = &model.layout;
    let mut atol = vec![scale; l.n()];
    for i in 0..l.nq {
        for j in 0..l.nm {
            for k in 0..l.nphi {
                atol[l.cell(i, j, k)] = scale * l.dq[i] * l.dm[j] * l.dphi[k];
            }
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
fn f_drivers_allocate_independently_of_the_step_count() {
    let (model, y0) = scenario();
    let problem = support::problem(model.clone()).unwrap();
    assert!(!problem.autonomous);
    let band = support::band(model.clone());
    let output = schedule();
    let n = model.layout.n();

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
        "(f) drivers (attempts, rejected, ft_calls, allocations) at rtol 1e-6, 1e-9: \
         dense {dense:?}, banded {banded:?}"
    );
}

#[test]
fn f_callbacks_allocate_nothing() {
    let (model, y0) = scenario();
    let problem = support::problem(model.clone()).unwrap();
    let jvp = problem.jvp_callback().unwrap().clone();
    let declared = support::band(model.clone());
    let l = &model.layout;
    let n = l.n();
    let bw = l.bandwidth();
    let mut out = vec![0.0; n];
    let mut group = vec![0.0; l.m];
    let mut dense = DenseMatrix::zeros(n, n);
    let mut packed = vec![0.0; n * (2 * bw + 1)];
    let mut counters = WorkCounters::default();
    let rates: Vec<f64> = (0..l.nq).map(|i| 0.1 * i as f64).collect();
    let v: Vec<f64> = (0..n).map(|k| (k as f64).sin()).collect();
    let time = |k: usize| 3.0 * k as f64 / 500.0;
    let mut counts = Vec::new();
    let (_, a) = allocations_during(|| {
        for k in 0..500 {
            model.rhs(time(k), &y0, &mut out).unwrap();
        }
    });
    counts.push(("rhs", a));
    let (_, a) = allocations_during(|| {
        for k in 0..500 {
            model.partial_t(time(k), &y0, &mut out).unwrap();
        }
    });
    counts.push(("partial_t", a));
    let (_, a) = allocations_during(|| {
        for k in 0..500 {
            jvp(time(k), &y0, &v, &mut out).unwrap();
        }
    });
    counts.push(("problem jvp", a));
    let (_, a) = allocations_during(|| {
        for k in 0..500 {
            problem
                .eval_partial_t_into(time(k), &y0, &mut out, &mut counters)
                .unwrap();
        }
    });
    counts.push(("problem eval_partial_t_into", a));
    let (_, a) = allocations_during(|| {
        for k in 0..500 {
            model
                .rhs_with_group_rates(time(k), &y0, &rates, &mut out)
                .unwrap();
        }
    });
    counts.push(("rhs_with_group_rates", a));
    let (_, a) = allocations_during(|| {
        for k in 0..500 {
            let s = [0.1 * time(k), -0.2, 0.3];
            model
                .transport_with_select([0.3, -0.1, 0.05], s, &y0, &mut out)
                .unwrap();
        }
    });
    counts.push(("transport_with_select", a));
    let (_, a) = allocations_during(|| {
        for k in 0..500 {
            model
                .thomson_group_rhs(&y0[1..1 + l.m], 0.5 + time(k), &mut group)
                .unwrap();
        }
    });
    counts.push(("thomson_group_rhs", a));
    let (_, a) = allocations_during(|| {
        for k in 0..100 {
            model.dense_fill(time(k), &mut dense).unwrap();
        }
    });
    counts.push(("dense_fill", a));
    let (_, a) = allocations_during(|| {
        for k in 0..100 {
            packed.fill(0.0);
            model.band_fill(time(k), &mut packed).unwrap();
        }
    });
    counts.push(("band_fill", a));
    let (_, a) = allocations_during(|| {
        for k in 0..100 {
            packed.fill(0.0);
            (declared.fill)(time(k), &y0, &mut packed).unwrap();
        }
    });
    counts.push(("declared band fill", a));
    // Positive control: the counter sees an allocation on this thread.
    let (_, a) = allocations_during(|| std::hint::black_box(vec![0.0_f64; n]));
    assert!(a >= 1, "the counting allocator is not active");
    for (name, a) in &counts {
        assert_eq!(*a, 0, "{name} allocated {a} times");
    }
    eprintln!("(f) callbacks: {counts:?}; positive control {a}");
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
fn g_runs_are_deterministic_and_uniform_vector_atol_is_scalar() {
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

    // The number invariant (every slot, the outflow and absorbed ledgers
    // included) holds along both trajectories: relative drift at every
    // output at most 1e-11 (the P02 C3 / P04 C3 trajectory bound).
    let total0 = compensated_sum(&y0);
    let mut worst_drift = 0.0_f64;
    for (what, ys) in [("dense", &a.observed.y), ("banded", &ba.fast.observed.y)] {
        assert_eq!(ys.len(), 3, "{what}: outputs");
        for (k, y) in ys.iter().enumerate() {
            let drift = (compensated_sum(y) - total0).abs() / total0.abs();
            assert!(
                drift <= 1.0e-11,
                "{what} output {k}: invariant drift {drift:e} > 1e-11"
            );
            worst_drift = worst_drift.max(drift);
        }
    }

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
        "dense vs banded final WRMS {wrms:e} > 1"
    );
    eprintln!(
        "(g) vector-atol attempts dense {} banded {}; scalar-atol attempts dense {} banded {}; \
         dense vs banded final WRMS {wrms:e}, invariant drift dense {:e}, max relative \
         invariant drift over both trajectories {worst_drift:e}",
        a.attempts,
        ba.fast.attempts,
        dense_scalar_attempts,
        scalar.fast.attempts,
        (Model3::invariant(yd) - Model3::invariant(&y0)).abs()
    );
}

// ---------------------------------------------------------------------------
// (h) Typed refusals
// ---------------------------------------------------------------------------

fn code_of<T>(r: CoreResult<T>) -> (String, &'static str) {
    match r {
        Ok(_) => ("<accepted>".into(), "ok"),
        Err(e) => {
            let kind = match e {
                CoreError::InvalidInput(_) => "InvalidInput",
                CoreError::Dimension(_) => "Dimension",
                CoreError::NonFinite(_) => "NonFinite",
                _ => "other",
            };
            (refusal_code(&e).unwrap_or("<no code>").to_string(), kind)
        }
    }
}

/// `(case, q, mu, phi, expected code)`.
type BadGrid = (&'static str, Vec<f64>, Vec<f64>, Vec<f64>, &'static str);

#[test]
fn h_invalid_grids_geometries_rates_and_shapes_are_typed_refusals() {
    let q = || vec![0.0, 0.5, 1.0];
    let mu = || vec![-1.0, 0.0, 1.0];
    let phi = || uniform_phi_edges(3, 0.0).unwrap();
    let layout = |q: Vec<f64>, mu: Vec<f64>, phi: Vec<f64>| {
        Layout3::new(q, mu, phi, Coordinate::Physical, Boundary::Outflow)
    };
    let mut seen = std::collections::BTreeSet::new();
    let bad_grids: Vec<BadGrid> = vec![
        ("empty q", vec![], mu(), phi(), "q.too_few_edges"),
        ("one q edge", vec![0.0], mu(), phi(), "q.too_few_edges"),
        (
            "NaN q",
            vec![0.0, f64::NAN],
            mu(),
            phi(),
            "q.nonfinite_edge",
        ),
        (
            "infinite q",
            vec![0.0, f64::INFINITY],
            mu(),
            phi(),
            "q.nonfinite_edge",
        ),
        ("equal q", vec![0.0, 0.0], mu(), phi(), "q.width"),
        ("decreasing q", vec![1.0, 0.0], mu(), phi(), "q.width"),
        (
            "overflowing q width",
            vec![-1.0e308, 1.0e308],
            mu(),
            phi(),
            "q.width",
        ),
        (
            "subnormal q width",
            vec![0.0, 5.0e-324],
            mu(),
            phi(),
            "q.subnormal_width",
        ),
        ("one mu edge", q(), vec![-1.0], phi(), "mu.too_few_edges"),
        (
            "NaN mu",
            q(),
            vec![-1.0, f64::NAN, 1.0],
            phi(),
            "mu.nonfinite_edge",
        ),
        (
            "mu to 0.5",
            q(),
            vec![-1.0, 0.5],
            phi(),
            "mu.not_full_sphere",
        ),
        (
            "mu below -1",
            q(),
            vec![-1.0000001, 1.0],
            phi(),
            "mu.not_full_sphere",
        ),
        (
            "mu not from -1",
            q(),
            vec![-0.9, 1.0],
            phi(),
            "mu.not_full_sphere",
        ),
        (
            "mu decreasing",
            q(),
            vec![-1.0, 1.0, 0.5],
            phi(),
            "mu.width",
        ),
        ("one phi edge", q(), mu(), vec![0.0], "phi.too_few_edges"),
        (
            "phi half period",
            q(),
            mu(),
            vec![0.0, std::f64::consts::PI],
            "phi.not_periodic_cover",
        ),
        (
            "phi beyond TAU",
            q(),
            mu(),
            vec![0.0, TAU + 1.0e-12],
            "phi.not_periodic_cover",
        ),
        (
            "phi0 negative",
            q(),
            mu(),
            vec![-0.1, -0.1 + TAU],
            "phi.not_periodic_cover",
        ),
        (
            "phi0 == TAU",
            q(),
            mu(),
            vec![TAU, 2.0 * TAU],
            "phi.not_periodic_cover",
        ),
        (
            "phi decreasing",
            q(),
            mu(),
            vec![0.0, 3.0, 2.0, TAU],
            "phi.width",
        ),
        (
            "phi last edge",
            q(),
            mu(),
            vec![0.0, 3.0, TAU + 1.0],
            "phi.not_periodic_cover",
        ),
        (
            "NaN phi",
            q(),
            mu(),
            vec![0.0, f64::NAN, TAU],
            "phi.nonfinite_edge",
        ),
        (
            "subnormal phi width",
            q(),
            mu(),
            vec![0.0, 5.0e-324, TAU],
            "phi.subnormal_width",
        ),
    ];
    for (name, q, mu, phi, code) in bad_grids {
        assert_eq!(
            code_of(layout(q, mu, phi)),
            (code.to_string(), "InvalidInput"),
            "{name}"
        );
        seen.insert(code);
    }
    // Positive controls: offsets with phi[-1] == phi0 + TAU are accepted.
    assert!(layout(q(), mu(), vec![0.1, 3.0, 0.1 + TAU]).is_ok());
    assert!(layout(q(), mu(), vec![0.0, TAU]).is_ok());
    assert!(layout(q(), mu(), uniform_phi_edges(7, 6.0).unwrap()).is_ok());
    for r in [
        uniform_phi_edges(0, 0.0),
        uniform_mu_edges(0),
        uniform_edges(0.0, 1.0, 0),
    ] {
        assert_eq!(code_of(r), ("grid.cell_count".to_string(), "InvalidInput"));
    }
    seen.insert("grid.cell_count");

    let good = || layout(q(), mu(), phi()).unwrap();
    let constant = Geometry3::Constant {
        h1: 0.3,
        h2: -0.1,
        h3: 0.05,
    };
    let bad_models: Vec<(&str, Geometry3, f64, Option<Absorber>, &str)> = vec![
        (
            "NaN H1",
            Geometry3::Constant {
                h1: f64::NAN,
                h2: 0.0,
                h3: 0.0,
            },
            1.0,
            None,
            "geometry.nonfinite",
        ),
        (
            "infinite H3",
            Geometry3::Constant {
                h1: 0.1,
                h2: 0.0,
                h3: f64::NEG_INFINITY,
            },
            1.0,
            None,
            "geometry.nonfinite",
        ),
        (
            "(H1 + H2)/2 overflows",
            Geometry3::Constant {
                h1: 1.7e308,
                h2: 1.7e308,
                h3: 0.0,
            },
            1.0,
            None,
            "geometry.overflow",
        ),
        (
            "(H1 - H2)/2 overflows",
            Geometry3::Constant {
                h1: 1.7e308,
                h2: -1.7e308,
                h3: 0.0,
            },
            1.0,
            None,
            "geometry.overflow",
        ),
        (
            "NaN hh2",
            Geometry3::PowerLaw {
                hh1: 0.1,
                hh2: f64::NAN,
                hh3: 0.0,
            },
            1.0,
            None,
            "geometry.nonfinite",
        ),
        (
            "power law overflow",
            Geometry3::PowerLaw {
                hh1: 1.7e308,
                hh2: 1.7e308,
                hh3: 0.0,
            },
            1.0,
            None,
            "geometry.overflow",
        ),
        ("negative gamma", constant, -1.0, None, "rate.invalid"),
        ("NaN gamma", constant, f64::NAN, None, "rate.invalid"),
        (
            "infinite gamma",
            constant,
            f64::INFINITY,
            None,
            "rate.invalid",
        ),
        (
            "negative k0",
            constant,
            1.0,
            Some(Absorber {
                k0: -1.0,
                q_chi: 0.2,
            }),
            "absorber.invalid",
        ),
        (
            "NaN k0",
            constant,
            1.0,
            Some(Absorber {
                k0: f64::NAN,
                q_chi: 0.2,
            }),
            "absorber.invalid",
        ),
        (
            "infinite k0",
            constant,
            1.0,
            Some(Absorber {
                k0: f64::INFINITY,
                q_chi: 0.2,
            }),
            "absorber.invalid",
        ),
        (
            "NaN q_chi",
            constant,
            1.0,
            Some(Absorber {
                k0: 1.0,
                q_chi: f64::NAN,
            }),
            "absorber.invalid",
        ),
    ];
    for (name, geometry, gamma, absorber, code) in bad_models {
        assert_eq!(
            code_of(Model3::new(good(), geometry, gamma, absorber, false)),
            (code.to_string(), "InvalidInput"),
            "{name}"
        );
        seen.insert(code);
    }

    // Evaluation-time refusals of the geometry.
    let power = Model3::new(
        good(),
        Geometry3::PowerLaw {
            hh1: 0.5,
            hh2: 0.1,
            hh3: -0.3,
        },
        1.0,
        None,
        false,
    )
    .unwrap();
    let n = power.layout.n();
    let width = 2 * power.layout.bandwidth() + 1;
    let y = vec![0.1; n];
    let mut out = vec![0.0; n];
    for t in [-1.0, -2.0, f64::NAN] {
        let expect = ("geometry.domain".to_string(), "InvalidInput");
        assert_eq!(code_of(power.rhs(t, &y, &mut out)), expect, "rhs t={t}");
        assert_eq!(
            code_of(power.partial_t(t, &y, &mut out)),
            expect,
            "f_t t={t}"
        );
        let mut m = DenseMatrix::zeros(n, n);
        assert_eq!(code_of(power.dense_fill(t, &mut m)), expect, "dense t={t}");
        let mut b = vec![0.0; n * width];
        assert_eq!(code_of(power.band_fill(t, &mut b)), expect, "band t={t}");
    }
    seen.insert("geometry.domain");
    // The drift overflows at evaluation time (accepted parameters): the
    // power law near t = -1, and the comoving mean of a huge geometry.
    let huge = Model3::new(
        good(),
        Geometry3::PowerLaw {
            hh1: 1.0e300,
            hh2: 0.0,
            hh3: 0.0,
        },
        1.0,
        None,
        false,
    )
    .unwrap();
    assert_eq!(
        code_of(huge.rhs(-1.0 + 1.0e-15, &y, &mut out)),
        ("geometry.nonfinite".to_string(), "NonFinite")
    );
    assert!(huge.rhs(0.0, &y, &mut out).is_ok());
    let comoving = Model3::new(
        layout(q(), mu(), phi())
            .map(|mut l| {
                l.coordinate = Coordinate::Comoving;
                l
            })
            .unwrap(),
        Geometry3::Constant {
            h1: 1.5e308,
            h2: 0.0,
            h3: 1.5e308,
        },
        1.0,
        None,
        false,
    )
    .unwrap();
    assert_eq!(
        code_of(comoving.rhs(0.0, &y, &mut out)),
        ("geometry.nonfinite".to_string(), "NonFinite")
    );

    // Explicit group rates, select geometry, Thomson rate.
    let model = Model3::new(good(), constant, 1.0, None, false).unwrap();
    for rates in [vec![0.1], vec![0.1, -0.2], vec![0.1, f64::NAN], vec![]] {
        assert_eq!(
            code_of(model.rhs_with_group_rates(0.0, &y, &rates, &mut out)),
            ("absorber.invalid".to_string(), "InvalidInput"),
            "{rates:?}"
        );
    }
    assert!(
        model
            .rhs_with_group_rates(0.0, &y, &[0.1, 0.0], &mut out)
            .is_ok()
    );
    let h = [0.3, -0.1, 0.05];
    for select in [[0.1, f64::NAN, 0.0], [1.7e308, 1.7e308, 0.0]] {
        assert_eq!(
            code_of(model.transport_with_select(h, select, &y, &mut out)),
            ("geometry.select".to_string(), "InvalidInput"),
            "{select:?}"
        );
    }
    seen.insert("geometry.select");
    assert_eq!(
        code_of(model.transport_with_select([f64::NAN, 0.0, 0.0], h, &y, &mut out)),
        ("geometry.nonfinite".to_string(), "InvalidInput")
    );
    let m = model.layout.m;
    let mut g = vec![0.0; m];
    for rate in [-1.0, f64::NAN, f64::INFINITY] {
        assert_eq!(
            code_of(model.thomson_group_rhs(&vec![1.0; m], rate, &mut g)),
            ("rate.invalid".to_string(), "InvalidInput")
        );
    }
    assert_eq!(
        code_of(model.thomson_group_rhs(&vec![1.0; m - 1], 1.0, &mut g)),
        ("state.length".to_string(), "Dimension")
    );

    // Shapes.
    for (ny, nout) in [(n - 1, n), (n + 1, n), (n, n - 1), (n, n + 1), (0, 0)] {
        let mut out = vec![0.0; nout];
        let state = vec![0.1; ny];
        let expect = ("state.length".to_string(), "Dimension");
        assert_eq!(code_of(model.rhs(0.5, &state, &mut out)), expect);
        assert_eq!(code_of(model.partial_t(0.5, &state, &mut out)), expect);
        assert_eq!(
            code_of(model.rhs_with_group_rates(0.5, &state, &[0.1, 0.1], &mut out)),
            expect
        );
        assert_eq!(
            code_of(model.transport_with_select(h, h, &state, &mut out)),
            expect
        );
    }
    seen.insert("state.length");
    for (r, c) in [(n - 1, n), (n, n - 1), (n + 1, n + 1), (0, 0)] {
        let mut m = DenseMatrix::zeros(r, c);
        assert_eq!(
            code_of(model.dense_fill(0.5, &mut m)),
            ("jacobian.shape".to_string(), "Dimension")
        );
    }
    for len in [0, n * width - 1, n * width + 1, n * n] {
        let mut b = vec![0.0; len];
        assert_eq!(
            code_of(model.band_fill(0.5, &mut b)),
            ("band.shape".to_string(), "Dimension")
        );
    }
    eprintln!("(h) refusal codes exercised: {seen:?}");
}

// ---------------------------------------------------------------------------
// Sanity: f_t, JVP, autonomy
// ---------------------------------------------------------------------------

#[test]
fn sanity_partial_t_matches_central_differences_away_from_kinks() {
    let grid = &grids()[3];
    let mut rng = XorShift::new(0x5a17_0606);
    let absorber = Absorber {
        k0: 0.5,
        q_chi: -0.45,
    };
    let power = Geometry3::PowerLaw {
        hh1: 0.5,
        hh2: 0.1,
        hh3: -0.3,
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
            GEOMETRIES[0],
            Some(absorber),
        ),
        (
            "physical power law absorber",
            Coordinate::Physical,
            power,
            Some(absorber),
        ),
        ("physical power law", Coordinate::Physical, power, None),
        ("comoving power law", Coordinate::Comoving, power, None),
    ];
    let e = 1.0e-5;
    let mut worst = 0.0_f64;
    for (name, coordinate, geometry, absorber) in cases {
        let model = model_on(grid, coordinate, Boundary::Outflow, geometry, 0.5, absorber);
        assert!(!model.autonomous(), "{name}");
        let l = &model.layout;
        let n = l.n();
        for t in [0.3, 0.7, 2.0] {
            // Preconditions (not tolerances): the threshold stays well away
            // from every edge, and no upwind direction flips, on [t-e, t+e].
            if let (Coordinate::Comoving, Some(a)) = (coordinate, absorber) {
                let x = a.q_chi + geometry.mean_ln_a(t);
                let gap = grid
                    .q
                    .iter()
                    .fold(f64::INFINITY, |m, edge| m.min((edge - x).abs()));
                assert!(gap > 1.0e-3, "{name} t={t}: threshold {x} near an edge");
            }
            let signs = |tt: f64| {
                let (v, _) = model.drift_at(tt).unwrap();
                let (vq, vmu, vphi) = l.face_velocities(v);
                vq.iter()
                    .chain(&vmu)
                    .chain(&vphi)
                    .map(|x| (*x >= 0.0, *x == 0.0))
                    .collect::<Vec<_>>()
            };
            assert_eq!(signs(t - e), signs(t + e), "{name} t={t}: upwind flip");
            let y = positive_state(&mut rng, n);
            let mut ft = vec![0.0; n];
            let (mut fp, mut fm) = (vec![0.0; n], vec![0.0; n]);
            model.partial_t(t, &y, &mut ft).unwrap();
            model.rhs(t + e, &y, &mut fp).unwrap();
            model.rhs(t - e, &y, &mut fm).unwrap();
            let dev = (0..n)
                .map(|k| (ft[k] - (fp[k] - fm[k]) / (2.0 * e)).abs())
                .fold(0.0_f64, f64::max);
            assert!(max_abs(&ft) > 0.0, "{name} t={t}: f_t vanishes");
            let rel = dev / max_abs(&ft);
            assert!(
                rel <= 1.0e-6,
                "{name} t={t}: f_t vs central difference {rel:e}"
            );
            worst = worst.max(rel);
        }
    }

    // f_t's transport is the select path with velocities dv/dt.
    let model = model_on(
        grid,
        Coordinate::Comoving,
        Boundary::Outflow,
        power,
        0.0,
        None,
    );
    let n = model.layout.n();
    let y = positive_state(&mut rng, n);
    let (v, dv) = model.drift_at(0.7).unwrap();
    let (mut a, mut b) = (vec![0.0; n], vec![0.0; n]);
    model.partial_t(0.7, &y, &mut a).unwrap();
    model.transport_with_select(dv, v, &y, &mut b).unwrap();
    assert_eq!(bits(&a), bits(&b));

    // The negative control reports exact zeros over garbage in `out`.
    let dropped = Model3::new(
        grid.layout(Coordinate::Comoving, Boundary::Outflow),
        power,
        0.5,
        Some(absorber),
        true,
    )
    .unwrap();
    let mut out = vec![1.0e300; n];
    dropped.partial_t(0.7, &y, &mut out).unwrap();
    assert!(out.iter().all(|v| v.to_bits() == 0));
    let dropped_problem = support::problem(Arc::new(dropped)).unwrap();
    assert!(!dropped_problem.autonomous);
    eprintln!("sanity: max f_t vs central difference rel dev {worst:e}");
}

#[test]
fn sanity_jvp_callback_and_autonomy_flags() {
    let grid = &grids()[0];
    let absorber = Some(Absorber {
        k0: 2.0,
        q_chi: grid.mid(),
    });
    let power = GEOMETRIES[3];
    let constant = GEOMETRIES[0];
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
    let mut rng = XorShift::new(0x0a17_0a06);
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
