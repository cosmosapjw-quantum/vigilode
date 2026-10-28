//! Contracts of the accepted-step conservative remap of the LRS photon
//! transport adapter (BASS/rei DAG node P05,
//! `examples/lrs_photon_fv_support/remesh.rs`) on the comoving absorber
//! scenario (P02's power-law geometry `hh 0.5, s0 0.2, p 1.5`, base grid
//! 32 x 16 on `[-3, 2]`, absorber `k0 0.5, q_chi -0.5`, Thomson 0.5):
//!
//! * R1: an injected failure, and every typed refusal, leaves the state,
//!   the committed mesh and the epoch bit for bit unchanged and keeps its
//!   work counted.
//! * R2: one remap changes the total by at most 2e-15 relative and keeps
//!   every column sum (the angular columns and the absorbed ledgers) and
//!   `L_lo`, `L_hi`; signed columns are remapped per sign.
//! * R3: one refine/coarsen cycle is within 2 ulp per component (bitwise
//!   expected), 100 cycles within 1e-13 relative, with no allocation after
//!   warm-up; for the buffer helpers and for the remesher.
//! * R4: after a remap and a rebind to the new epoch, a solve with the old
//!   factorization is refused (`STRUCTURED_STALE`); a restart with the
//!   rebound client reaches `tf` and equals a fresh client's bit for bit.
//! * R5 (VG-1): a remap allocates nothing after warm-up; segmented runs
//!   allocate the same count when their remap counts are equal, at
//!   tolerances whose attempt counts differ, and a constant count per
//!   remap (trigger 1, 0.5 and 0.25).
//! * Determinism of segmented runs, and the aligned mesh's properties
//!   (strictly increasing, the face exactly at `x_chi`, widths in
//!   `[D / 2, 3 D / 2]` away from the ends).

use std::alloc::{GlobalAlloc, Layout as AllocLayout, System};
use std::cell::Cell;
use std::sync::{Arc, Mutex};

use rodas5p_core::{CoreError, WorkCounters};
use rodas5p_integrators::conservative_remap::RemapError;
use rodas5p_integrators::{
    AcceptControl, AcceptedStepView, AdaptiveStepConfig, DriverStopReason, OutputSchedule,
    StageLinearSystem, integrate_rodas5p_fast_banded_observed_with_atol,
    integrate_rodas5p_fast_client_observed_with_atol_and_control,
};

#[path = "../examples/lrs_photon_fv_support/mod.rs"]
mod support;
use support::block_solver::{BlockStageSolver, Capacitance, Mode};
use support::remesh::{
    Arm, BaseGrid, Linear, MAX_RELATIVE_TOTAL_CHANGE, Physics, RemapRefusal, Remesher, Restart,
    RunConfig, RunOutcome, SharedClient, aligned_edges, cell_atol, compensated_total,
    integrate_segments, merge_pairs, split_all, threshold_position,
};
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

const BASE: BaseGrid = BaseGrid {
    lo: -3.0,
    hi: 2.0,
    n: 32,
};
const NM: usize = 16;
const SPAN: (f64, f64) = (0.0, 3.0);
const KRYLOV: Mode = Mode::Krylov {
    restart: 4,
    max_iterations: 48,
    rtol: 1.0e-12,
    two_level: true,
};

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

fn physics() -> Physics {
    Physics {
        mu_edges: uniform_edges(-1.0, 1.0, NM),
        coordinate: Coordinate::Comoving,
        boundary: Boundary::Outflow,
        geometry: Geometry::PowerLaw {
            hh: 0.5,
            s0: 0.2,
            p: 1.5,
        },
        gamma: 0.5,
        absorber: Some(Absorber {
            k0: 0.5,
            q_chi: -0.5,
        }),
    }
}

fn base_model() -> Arc<Model> {
    Arc::new(
        physics()
            .model(uniform_edges(BASE.lo, BASE.hi, BASE.n))
            .unwrap(),
    )
}

/// The P02 scenario's initial state on a layout.
fn initial_state(l: &Layout) -> Vec<f64> {
    let mut y0 = vec![0.0; l.n()];
    for i in 0..l.nq {
        let q = 0.5 * (l.q_edges[i] + l.q_edges[i + 1]);
        for j in 0..l.nm {
            let mu = 0.5 * (l.mu_edges[j] + l.mu_edges[j + 1]);
            y0[l.cell(i, j)] =
                l.dq[i] * l.dm[j] * (-(q * q) / (2.0 * 0.36)).exp() * (1.0 + 0.3 * mu);
        }
    }
    y0
}

fn config(arm: Arm, linear: Linear, rtol: f64, output_times: Vec<f64>) -> RunConfig {
    let l = &base_model().layout;
    RunConfig {
        base: BASE,
        physics: physics(),
        y0: initial_state(l),
        t_span: SPAN,
        output_times,
        rtol,
        atol_scale: 1.0e-2 * rtol,
        initial_step: 1.0e-4,
        max_attempts: 1_000_000,
        linear,
        arm,
        refine: false,
        max_remaps: 256,
        restart: Restart::HNext,
        inject_failure_at_remap: None,
        record_remap_states: false,
    }
}

fn outputs() -> Vec<f64> {
    vec![0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0]
}

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

/// A realistic accepted state: the base mesh integrated to `t` (absorbed
/// ledgers and boundary ledgers nonzero).
fn state_at(t: f64) -> Vec<f64> {
    let model = base_model();
    let problem = support::problem(model.clone()).unwrap();
    let y0 = initial_state(&model.layout);
    let atol = cell_atol(&model.layout, 1.0e-10);
    let config = AdaptiveStepConfig {
        atol: 1.0e-10 * 1.0e-3,
        rtol: 1.0e-8,
        initial_step: 1.0e-4,
        max_attempts: 1_000_000,
        ..AdaptiveStepConfig::default()
    };
    let r = integrate_rodas5p_fast_banded_observed_with_atol(
        &problem,
        &support::band(model.clone()),
        (0.0, t),
        &y0,
        &config,
        &OutputSchedule::new(vec![0.0, t]).unwrap(),
        &atol,
    )
    .unwrap();
    assert!(r.fast.observed.success);
    r.fast.observed.y.last().unwrap().clone()
}

fn aligned_at(model: &Model, t: f64, refine: bool) -> (Vec<f64>, Option<usize>, f64) {
    let x = threshold_position(model, t).unwrap();
    let mut edges = Vec::new();
    let face = aligned_edges(BASE.lo, BASE.hi, BASE.n, x, refine, &mut edges);
    (edges, face, x)
}

#[test]
fn r1_refusals_roll_back_bit_for_bit_and_keep_their_work() {
    let model = base_model();
    let l = &model.layout;
    let y = state_at(1.3);
    for refine in [false, true] {
        let (edges, face, x) = aligned_at(&model, 1.3, refine);
        let k = face.unwrap();
        assert_eq!(edges[k].to_bits(), x.to_bits());
        let mut remesher = Remesher::new(&l.q_edges, NM, BASE.n + 2, 5).unwrap();
        let mut y_new = Vec::with_capacity(remesher.state_len(BASE.n + 2));
        y_new.extend_from_slice(&[7.0, 11.0]);
        let y_bits = bits(&y);
        let mesh_bits = bits(remesher.edges());
        let unchanged = |r: &Remesher, y: &[f64], y_new: &[f64]| {
            assert_eq!(bits(y), y_bits, "state changed");
            assert_eq!(bits(r.edges()), mesh_bits, "committed mesh changed");
            assert_eq!(r.epoch(), 5, "epoch changed");
            assert_eq!(bits(y_new), bits(&[7.0, 11.0]), "output buffer changed");
        };
        // the injected failure does all the work, then refuses
        let w0 = remesher.work;
        assert_eq!(
            remesher.propose(l, &y, &edges, true),
            Err(RemapRefusal::Injected)
        );
        let w1 = remesher.work;
        assert_eq!(w1.attempts, w0.attempts + 1);
        assert_eq!(w1.refusals, w0.refusals + 1);
        assert_eq!(w1.commits, w0.commits);
        // every column, both signs: (nm + 1) * 2 kernel calls of n..2n segments
        assert!(w1.overlap_segments >= (2 * (NM + 1) * BASE.n) as u64);
        assert!(w1.l1_change_sum > w0.l1_change_sum);
        unchanged(&remesher, &y, &y_new);
        assert_eq!(
            remesher.commit(&mut y_new),
            Err(RemapRefusal::NothingStaged)
        );
        unchanged(&remesher, &y, &y_new);
        // the typed refusals are atomic too
        let other = physics().model(edges.clone()).unwrap().layout;
        let other_state = vec![1.0; other.n()];
        let mut wrong_domain = edges.clone();
        *wrong_domain.last_mut().unwrap() = 2.5;
        let mut nan = y.clone();
        nan[l.cell(3, 2)] = f64::NAN;
        // a positive subnormal in both cells beside the moved edge: the
        // split fraction underflows in the kernel
        let mut tiny = y.clone();
        let base_face = if refine { k - 1 } else { k };
        for i in [base_face - 1, base_face] {
            tiny[l.cell(i, 4)] = f64::from_bits(1);
        }
        let cases: Vec<(&Layout, &[f64], &[f64], RemapRefusal)> = vec![
            (&other, &other_state, &edges, RemapRefusal::StaleMesh),
            (l, &y, &wrong_domain, RemapRefusal::Domain),
            (l, &nan, &edges, RemapRefusal::NonFinite),
            (
                l,
                &tiny,
                &edges,
                RemapRefusal::Kernel {
                    column: 4,
                    negative: false,
                    error: RemapError::ContributionUnderflow,
                },
            ),
            (
                l,
                &y[..l.n() - 1],
                &edges,
                RemapRefusal::Shape("state length differs from the old layout"),
            ),
        ];
        for (layout, state, new_edges, expected) in cases {
            let before = remesher.work;
            assert_eq!(
                remesher.propose(layout, state, new_edges, false),
                Err(expected)
            );
            assert_eq!(remesher.work.refusals, before.refusals + 1);
            assert_eq!(remesher.work.attempts, before.attempts + 1);
            assert!(remesher.work.overlap_segments >= before.overlap_segments);
            unchanged(&remesher, &y, &y_new);
            assert_eq!(
                remesher.commit(&mut y_new),
                Err(RemapRefusal::NothingStaged)
            );
        }
        // a normal proposal still commits
        let report = remesher.propose(l, &y, &edges, false).unwrap();
        assert_eq!(remesher.commit(&mut y_new), Ok(6));
        assert_eq!(bits(remesher.edges()), bits(&edges));
        assert_eq!(y_new.len(), 2 + report.destination_cells * (NM + 1));
        assert_eq!(remesher.work.commits, 1);
    }
}

/// Per column (the angular columns, then the absorbed ledgers): the
/// compensated column sum.
fn column_sums(y: &[f64], nq: usize) -> Vec<f64> {
    (0..=NM)
        .map(|col| {
            let v: Vec<f64> = (0..nq).map(|i| y[1 + i * (NM + 1) + col]).collect();
            compensated_total(&v)
        })
        .collect()
}

fn column_mass(y: &[f64], nq: usize) -> Vec<f64> {
    (0..=NM)
        .map(|col| (0..nq).map(|i| y[1 + i * (NM + 1) + col].abs()).sum())
        .collect()
}

#[test]
fn r2_one_remap_preserves_the_total_and_every_column() {
    let model = base_model();
    let l = &model.layout;
    let y = state_at(1.3);
    // a signed variant: one column negated, one with alternating signs
    let mut signed = y.clone();
    for i in 0..l.nq {
        signed[l.cell(i, 3)] = -signed[l.cell(i, 3)];
        if i % 2 == 1 {
            signed[l.cell(i, 5)] = -signed[l.cell(i, 5)];
        }
    }
    let mut worst_total = 0.0_f64;
    let mut worst_column = 0.0_f64;
    for refine in [false, true] {
        let (edges, face, x) = aligned_at(&model, 1.3, refine);
        let k = face.unwrap();
        for state in [&y, &signed] {
            let mut remesher = Remesher::new(&l.q_edges, NM, BASE.n + 2, 0).unwrap();
            let mut out = Vec::with_capacity(remesher.state_len(BASE.n + 2));
            let report = remesher.propose(l, state, &edges, false).unwrap();
            assert_eq!(remesher.commit(&mut out), Ok(1));
            let nq_new = edges.len() - 1;
            assert_eq!(nq_new, if refine { BASE.n + 2 } else { BASE.n });
            assert_eq!(out.len(), 2 + nq_new * (NM + 1));
            // the total, independently recomputed
            let (before, after) = (compensated_total(state), compensated_total(&out));
            let rel = (after - before).abs() / before.abs();
            assert!(rel <= MAX_RELATIVE_TOTAL_CHANGE, "total change {rel:e}");
            assert_eq!(report.total_before, before);
            assert_eq!(report.total_after, after);
            worst_total = worst_total.max(rel);
            // every column sum, against the column's absolute mass
            let (a, b, mass) = (
                column_sums(state, l.nq),
                column_sums(&out, nq_new),
                column_mass(state, l.nq),
            );
            for col in 0..=NM {
                let rel = (b[col] - a[col]).abs() / mass[col];
                assert!(rel <= MAX_RELATIVE_TOTAL_CHANGE, "column {col}: {rel:e}");
                worst_column = worst_column.max(rel);
            }
            // the boundary ledgers are copied
            assert_eq!(out[0].to_bits(), state[0].to_bits());
            assert_eq!(
                out[out.len() - 1].to_bits(),
                state[state.len() - 1].to_bits()
            );
            // the new layout keeps the angular grid, coordinate and boundary
            let new_layout = remesher.layout(l).unwrap();
            assert_eq!(bits(&new_layout.mu_edges), bits(&l.mu_edges));
            assert_eq!(new_layout.coordinate, l.coordinate);
            assert_eq!(new_layout.boundary, l.boundary);
            assert_eq!(new_layout.q_edges[k].to_bits(), x.to_bits());
            if !refine {
                // locality: only the two cells beside the moved edge change
                for i in 0..l.nq {
                    if i + 1 == k || i == k {
                        continue;
                    }
                    for col in 0..=NM {
                        let c = 1 + i * (NM + 1) + col;
                        assert_eq!(out[c].to_bits(), state[c].to_bits(), "cell {i} col {col}");
                    }
                }
                // nonnegative columns stay nonnegative
                for i in 0..nq_new {
                    assert!(out[1 + i * (NM + 1)] >= 0.0);
                }
            }
        }
        // a negated column remaps to the negated remap of the column
        let mut remesher = Remesher::new(&l.q_edges, NM, BASE.n + 2, 0).unwrap();
        let mut pos = Vec::new();
        remesher.propose(l, &y, &edges, false).unwrap();
        remesher.commit(&mut pos).unwrap();
        let mut remesher = Remesher::new(&l.q_edges, NM, BASE.n + 2, 0).unwrap();
        let mut neg = Vec::new();
        remesher.propose(l, &signed, &edges, false).unwrap();
        remesher.commit(&mut neg).unwrap();
        for i in 0..edges.len() - 1 {
            let c = 1 + i * (NM + 1) + 3;
            assert_eq!(neg[c], -pos[c], "cell {i}");
        }
    }
    eprintln!("R2: max total change {worst_total:e}, max column change {worst_column:e}");
}

fn max_ulps(a: &[f64], b: &[f64]) -> u64 {
    let key = |x: f64| {
        let i = x.to_bits() as i64;
        if i < 0 { i64::MIN - i } else { i }
    };
    a.iter()
        .zip(b)
        .map(|(x, y)| key(*x).abs_diff(key(*y)))
        .max()
        .unwrap()
}

fn max_rel(a: &[f64], reference: &[f64]) -> f64 {
    a.iter()
        .zip(reference)
        .map(|(x, r)| {
            if *r != 0.0 {
                (x - r).abs() / r.abs()
            } else if *x == 0.0 {
                0.0
            } else {
                f64::INFINITY
            }
        })
        .fold(0.0, f64::max)
}

#[test]
fn r3_refine_coarsen_cycles_are_exact_and_allocate_nothing() {
    let model = base_model();
    let l = &model.layout;
    let y0 = state_at(1.3);
    let cap = 2 * BASE.n;
    let (mut e1, mut y1) = (
        Vec::with_capacity(cap + 1),
        Vec::with_capacity(2 + cap * (NM + 1)),
    );
    let (mut e2, mut y2) = (
        Vec::with_capacity(cap + 1),
        Vec::with_capacity(2 + cap * (NM + 1)),
    );
    // the buffer helpers
    split_all(&l.q_edges, NM, &y0, &mut e1, &mut y1).unwrap();
    assert_eq!(e1.len(), 2 * BASE.n + 1);
    for (i, half) in y1[1..y1.len() - 1].chunks(NM + 1).enumerate() {
        let parent = &y0[1 + (i / 2) * (NM + 1)..1 + (i / 2 + 1) * (NM + 1)];
        for (h, p) in half.iter().zip(parent) {
            assert_eq!(h.to_bits(), (p / 2.0).to_bits());
        }
    }
    merge_pairs(&e1, NM, &y1, &mut e2, &mut y2).unwrap();
    assert!(max_ulps(&y2, &y0) <= 2);
    assert_eq!(bits(&y2), bits(&y0), "one helper cycle is bitwise");
    assert_eq!(bits(&e2), bits(&l.q_edges));
    let ((), helper_allocations) = allocations_during(|| {
        for _ in 0..100 {
            split_all(&e2, NM, &y2, &mut e1, &mut y1).unwrap();
            merge_pairs(&e1, NM, &y1, &mut e2, &mut y2).unwrap();
        }
    });
    assert_eq!(helper_allocations, 0);
    assert!(max_rel(&y2, &y0) <= 1.0e-13);
    // the remesher between the aligned mesh of x_chi(1.3) and its split
    let (coarse_edges, _, _) = aligned_at(&model, 1.3, false);
    let coarse = physics().model(coarse_edges.clone()).unwrap().layout;
    let mut fine_edges = Vec::new();
    split_all(
        &coarse_edges,
        NM,
        &vec![0.0; coarse.n()],
        &mut fine_edges,
        &mut Vec::new(),
    )
    .unwrap();
    let fine = physics().model(fine_edges.clone()).unwrap().layout;
    let mut remesher = Remesher::new(&l.q_edges, NM, cap, 0).unwrap();
    let mut y = Vec::with_capacity(remesher.state_len(cap));
    remesher.propose(l, &y0, &coarse_edges, false).unwrap();
    remesher.commit(&mut y).unwrap();
    let start = y.clone();
    let mut cycle = |y: &mut Vec<f64>| {
        let a = remesher.propose(&coarse, y, &fine_edges, false).unwrap();
        remesher.commit(y).unwrap();
        let b = remesher.propose(&fine, y, &coarse_edges, false).unwrap();
        remesher.commit(y).unwrap();
        a.relative_total_change.max(b.relative_total_change)
    };
    let worst = cycle(&mut y);
    let one = max_ulps(&y, &start);
    assert!(one <= 2, "remesher cycle: {one} ulp");
    let (worst_k, kernel_allocations) = allocations_during(|| {
        let mut worst = worst;
        for _ in 1..100 {
            worst = worst.max(cycle(&mut y));
        }
        worst
    });
    assert_eq!(kernel_allocations, 0, "remesher cycles after warm-up");
    assert!(worst_k <= MAX_RELATIVE_TOTAL_CHANGE);
    let rel = max_rel(&y, &start);
    assert!(rel <= 1.0e-13, "100 remesher cycles: {rel:e}");
    eprintln!(
        "R3: remesher cycle {one} ulp after one cycle (bitwise {}), {rel:e} after 100, \
         worst total change {worst_k:e}",
        bits(&y) == bits(&start)
    );
}

type Control<'c> = &'c mut dyn FnMut(&AcceptedStepView<'_>) -> AcceptControl;

/// One client segment from `(t, y)` with `initial_step = h0`.
fn client_segment(
    model: &Arc<Model>,
    client: Box<dyn StageLinearSystem>,
    t: f64,
    y: &[f64],
    h0: f64,
    control: Control<'_>,
) -> (
    rodas5p_integrators::Rodas5pFastResult,
    Option<rodas5p_integrators::DriverPartial>,
    Box<dyn StageLinearSystem>,
) {
    let problem = support::problem(model.clone()).unwrap();
    let rtol = 1.0e-6;
    let atol = cell_atol(&model.layout, 1.0e-2 * rtol);
    let config = AdaptiveStepConfig {
        atol: atol.iter().copied().fold(f64::INFINITY, f64::min),
        rtol,
        initial_step: h0,
        max_step: SPAN.1 - SPAN.0,
        max_attempts: 1_000_000,
        ..AdaptiveStepConfig::default()
    };
    integrate_rodas5p_fast_client_observed_with_atol_and_control(
        &problem,
        client,
        (t, SPAN.1),
        y,
        &config,
        &OutputSchedule::new(vec![t, SPAN.1]).unwrap(),
        &atol,
        control,
    )
    .unwrap()
}

#[test]
fn r4_rebind_makes_the_old_factorization_stale_and_the_rebound_client_restarts() {
    let model = base_model();
    let y0 = initial_state(&model.layout);
    let solver = Arc::new(Mutex::new(
        BlockStageSolver::new(model.clone(), 0, KRYLOV, Capacitance::Conservation).unwrap(),
    ));
    // segment 1: stop once the threshold left the base face by D / 2
    let face0 = threshold_position(&model, 0.0).unwrap();
    let width = 0.5 * BASE.width();
    let mut stop = |view: &AcceptedStepView<'_>| {
        if (threshold_position(&model, view.t).unwrap() - face0).abs() > width {
            AcceptControl::Stop
        } else {
            AcceptControl::Continue
        }
    };
    let (_, partial, _) = client_segment(
        &model,
        Box::new(SharedClient(solver.clone())),
        0.0,
        &y0,
        1.0e-4,
        &mut stop,
    );
    let partial = partial.expect("the control stops before tf");
    assert_eq!(partial.reason, DriverStopReason::ClientStop);
    let (t, h_next) = (partial.t, partial.h_next.unwrap());
    // the client holds the factorization of its last linearization
    let n = model.layout.n();
    let mut b = vec![1.0; n];
    let mut c = WorkCounters::default();
    solver.lock().unwrap().solve_with(&mut b, &mut c).unwrap();
    // remap at the stop and rebind to the new epoch
    let (edges, _, _) = aligned_at(&model, t, false);
    let mut remesher = Remesher::new(&model.layout.q_edges, NM, BASE.n, 0).unwrap();
    let mut y = Vec::with_capacity(n);
    remesher
        .propose(&model.layout, &partial.y, &edges, false)
        .unwrap();
    let epoch = remesher.commit(&mut y).unwrap();
    let moved = Arc::new(
        Model::new(
            remesher.layout(&model.layout).unwrap(),
            model.geometry,
            model.gamma,
            model.absorber,
            false,
        )
        .unwrap(),
    );
    solver.lock().unwrap().rebind(moved.clone(), epoch).unwrap();
    assert_eq!(solver.lock().unwrap().epoch(), epoch);
    let refused = |r: rodas5p_core::CoreResult<()>, prefix: &str| match r {
        Err(CoreError::InvalidInput(msg)) => assert!(msg.starts_with(prefix), "{msg}"),
        other => panic!("expected {prefix}, got {other:?}"),
    };
    b.fill(1.0);
    refused(
        solver.lock().unwrap().solve_with(&mut b, &mut c),
        "STRUCTURED_STALE",
    );
    refused(
        solver.lock().unwrap().factor_with(10.0),
        "STRUCTURED_NOT_LINEARIZED",
    );
    // the restart with the rebound client reaches tf and equals a fresh
    // client's run bit for bit
    let mut go = |_: &AcceptedStepView<'_>| AcceptControl::Continue;
    let (rebound, p1, _) = client_segment(
        &moved,
        Box::new(SharedClient(solver.clone())),
        t,
        &y,
        h_next,
        &mut go,
    );
    let fresh_solver =
        BlockStageSolver::new(moved.clone(), epoch, KRYLOV, Capacitance::Conservation).unwrap();
    let (fresh, p2, _) = client_segment(&moved, Box::new(fresh_solver), t, &y, h_next, &mut go);
    assert!(p1.is_none() && rebound.observed.success);
    assert!(p2.is_none() && fresh.observed.success);
    let flat = |s: &[Vec<f64>]| -> Vec<u64> { s.iter().flatten().map(|v| v.to_bits()).collect() };
    assert_eq!(flat(&rebound.observed.y), flat(&fresh.observed.y));
    assert_eq!(
        serde_json::to_string(&rebound.observed.counters).unwrap(),
        serde_json::to_string(&fresh.observed.counters).unwrap()
    );
    // the whole hook arm on the rebound client
    let run = integrate_segments(&config(
        Arm::Hook { trigger: 0.5 },
        Linear::Client(KRYLOV),
        1.0e-6,
        outputs(),
    ))
    .unwrap();
    assert!(run.success, "{}", run.message);
    assert!(!run.remaps.is_empty());
    assert_eq!(run.client_epoch, Some(run.remaps.len() as u64));
    assert_eq!(run.segments, run.remaps.len() + 1);
}

fn run(arm: Arm, linear: Linear, rtol: f64, output_times: Vec<f64>) -> RunOutcome {
    let r = integrate_segments(&config(arm, linear, rtol, output_times)).unwrap();
    assert!(r.success, "{}", r.message);
    r
}

#[test]
fn r5_remaps_allocate_nothing_and_restarts_a_constant_count() {
    let model = base_model();
    let l = &model.layout;
    let y0 = state_at(1.3);
    // a remap allocates nothing after warm-up
    let (a, _, _) = aligned_at(&model, 1.3, false);
    let (b, _, _) = aligned_at(&model, 1.6, false);
    let la = physics().model(a.clone()).unwrap().layout;
    let lb = physics().model(b.clone()).unwrap().layout;
    let mut remesher = Remesher::new(&l.q_edges, NM, BASE.n, 0).unwrap();
    let mut y = Vec::with_capacity(l.n());
    remesher.propose(l, &y0, &a, false).unwrap();
    remesher.commit(&mut y).unwrap();
    let ((), remap_allocations) = allocations_during(|| {
        for _ in 0..50 {
            remesher.propose(&la, &y, &b, false).unwrap();
            remesher.commit(&mut y).unwrap();
            remesher.propose(&lb, &y, &a, false).unwrap();
            remesher.commit(&mut y).unwrap();
        }
    });
    assert_eq!(remap_allocations, 0, "propose and commit after warm-up");
    // segmented runs: outputs only at t0 and tf, so that every segment
    // records the same outputs
    let ends = vec![SPAN.0, SPAN.1];
    for linear in [Linear::Banded, Linear::Client(KRYLOV)] {
        let measure = |trigger: f64, rtol: f64| {
            let (r, allocations) =
                allocations_during(|| run(Arm::Hook { trigger }, linear, rtol, ends.clone()));
            (r.remaps.len(), allocations, r.attempts)
        };
        measure(0.5, 1.0e-6); // warm-up of lazy statics
        let points = [
            measure(1.0, 1.0e-6),
            measure(0.5, 1.0e-6),
            measure(0.5, 1.0e-9),
            measure(0.25, 1.0e-6),
            measure(0.25, 1.0e-9),
        ];
        eprintln!("R5 {linear:?}: (remaps, allocations, attempts) {points:?}");
        // the tighter tolerance takes at least twice the attempts
        assert!(points[2].2 >= 2 * points[1].2, "{points:?}");
        assert!(points[4].2 >= 2 * points[3].2, "{points:?}");
        // the finer trigger remaps more
        assert!(points[3].0 > points[1].0, "{points:?}");
        // equal remap counts allocate equally; the count is affine in the
        // remap count with one per-remap constant
        let (r0, a0) = (points[0].0 as i64, points[0].1 as i64);
        let (r1, a1) = (points[3].0 as i64, points[3].1 as i64);
        assert!(r1 > r0);
        let per = (a1 - a0) / (r1 - r0);
        assert_eq!((a1 - a0) % (r1 - r0), 0, "{points:?}");
        for (r, alloc, _) in points {
            assert_eq!(
                alloc as i64,
                a0 + per * (r as i64 - r0),
                "allocations not affine in the remap count: {points:?}"
            );
        }
        eprintln!("R5 {linear:?}: {per} allocations per remap");
    }
    // equal remap counts at two tolerances: the static arm (none) and the
    // schedule arm (eleven) allocate independently of the step count
    let times: Vec<f64> = (1..12).map(|k| 0.25 * k as f64).collect();
    for (arm, linear) in [
        (Arm::Static, Linear::Banded),
        (
            Arm::Schedule {
                times: times.clone(),
            },
            Linear::Banded,
        ),
        (Arm::Schedule { times }, Linear::Client(KRYLOV)),
    ] {
        let measure =
            |rtol: f64| allocations_during(|| run(arm.clone(), linear, rtol, ends.clone()));
        measure(1.0e-6);
        let ((loose, a1), (tight, a2)) = (measure(1.0e-6), measure(1.0e-9));
        assert_eq!(loose.remaps.len(), tight.remaps.len());
        assert!(
            tight.attempts >= 2 * loose.attempts,
            "{arm:?}: attempts {} vs {}",
            tight.attempts,
            loose.attempts
        );
        assert_eq!(
            a1, a2,
            "{arm:?} {linear:?}: allocations depend on the step count"
        );
        eprintln!(
            "R5 {arm:?} {linear:?}: {} remaps, attempts {} / {}, allocations {a1}",
            loose.remaps.len(),
            loose.attempts,
            tight.attempts
        );
    }
}

#[test]
fn r1_a_refusal_inside_a_run_keeps_the_accepted_state_and_all_work() {
    for linear in [Linear::Banded, Linear::Client(KRYLOV)] {
        let mut cfg = config(Arm::Hook { trigger: 0.5 }, linear, 1.0e-6, outputs());
        cfg.record_remap_states = true;
        let full = run_cfg(&cfg);
        assert!(full.remaps.len() >= 4, "{}", full.remaps.len());
        assert_eq!(full.remap_states.len(), full.remaps.len());
        for (e, s) in full.remaps.iter().zip(&full.remap_states) {
            assert_eq!(e.t.to_bits(), s.t.to_bits());
            let (before, after) = (
                compensated_total(&s.y_before),
                compensated_total(&s.y_after),
            );
            assert!((after - before).abs() <= MAX_RELATIVE_TOTAL_CHANGE * before.abs());
            assert_eq!(s.old_edges.len(), s.new_edges.len());
        }
        for k in [1, 3] {
            let mut injected = cfg.clone();
            injected.inject_failure_at_remap = Some(k);
            let r = integrate_segments(&injected).unwrap();
            assert!(!r.success, "remap {k} was injected");
            assert!(r.message.contains("Injected"), "{}", r.message);
            let (e, s) = (&full.remaps[k - 1], &full.remap_states[k - 1]);
            // the accepted state, mesh and epoch just before the refused remap
            assert_eq!(r.final_t.to_bits(), e.t.to_bits());
            assert_eq!(bits(&r.final_state), bits(&s.y_before));
            assert_eq!(bits(&r.final_edges), bits(&s.old_edges));
            assert_eq!(r.final_epoch, (k - 1) as u64);
            // every segment's work, and the refused attempt
            assert_eq!(r.attempts, e.attempts_before);
            assert_eq!(
                serde_json::to_string(&r.counters).unwrap(),
                serde_json::to_string(&e.counters_before).unwrap()
            );
            assert_eq!(r.remaps.len(), k - 1);
            assert_eq!(r.remesh.attempts, k as u64);
            assert_eq!(r.remesh.refusals, 1);
            assert_eq!(r.remesh.commits, (k - 1) as u64);
            assert!(r.outputs.len() < full.outputs.len());
            for (a, b) in r.outputs.iter().zip(&full.outputs) {
                assert_eq!(a.t.to_bits(), b.t.to_bits());
                assert_eq!(bits(&a.state), bits(&b.state));
            }
            if let Linear::Client(_) = linear {
                assert_eq!(r.client_epoch, Some((k - 1) as u64));
                let ops = r.client_ops.unwrap();
                assert_eq!(ops.factorizations, r.counters.direct_factorizations);
            }
        }
    }
}

#[test]
fn r4_restarts_take_the_controllers_proposal_or_a_fresh_step() {
    for linear in [Linear::Banded, Linear::Client(KRYLOV)] {
        let cfg = config(Arm::Hook { trigger: 0.5 }, linear, 1.0e-6, outputs());
        let a = run_cfg(&cfg);
        assert!(!a.remaps.is_empty());
        for e in &a.remaps {
            // the driver's stop record and the accept view agree on h_next
            assert_eq!(e.h_restart.to_bits(), e.control_h_next.to_bits());
            assert_ne!(e.h_restart.to_bits(), cfg.initial_step.to_bits());
        }
        let mut fresh = cfg.clone();
        fresh.restart = Restart::Fresh;
        let b = run_cfg(&fresh);
        for e in &b.remaps {
            assert_eq!(e.h_restart.to_bits(), cfg.initial_step.to_bits());
        }
        eprintln!(
            "restart {linear:?}: h_next {} attempts / {} remaps, fresh {} attempts / {} remaps",
            a.attempts,
            a.remaps.len(),
            b.attempts,
            b.remaps.len()
        );
    }
}

#[test]
fn r4_refined_meshes_rebuild_the_client_and_keep_its_work() {
    let mut banded = config(
        Arm::Hook { trigger: 0.5 },
        Linear::Banded,
        1.0e-6,
        outputs(),
    );
    banded.refine = true;
    let mut client = banded.clone();
    client.linear = Linear::Client(KRYLOV);
    let (b, c) = (run_cfg(&banded), run_cfg(&client));
    for r in [&b, &c] {
        assert!(!r.remaps.is_empty());
        // the refined aligned mesh has two more cells than the base
        assert_eq!(r.final_edges.len(), BASE.n + 3);
        assert_eq!(r.final_state.len(), 2 + (BASE.n + 2) * (NM + 1));
    }
    assert_eq!(b.remaps.len(), c.remaps.len());
    assert_eq!(c.client_epoch, Some(c.remaps.len() as u64));
    // the rebuilt client carries the work of the solver it replaced
    let ops = c.client_ops.unwrap();
    assert_eq!(ops.factorizations, c.counters.direct_factorizations);
    let (sb, sc) = (
        compensated_total(&b.final_state),
        compensated_total(&c.final_state),
    );
    assert!((sb - sc).abs() <= 1.0e-5 * sb.abs());
    // unrefined client runs count every factorization too
    let plain = run(
        Arm::Hook { trigger: 0.5 },
        Linear::Client(KRYLOV),
        1.0e-6,
        outputs(),
    );
    assert_eq!(
        plain.client_ops.unwrap().factorizations,
        plain.counters.direct_factorizations
    );
}

fn run_cfg(cfg: &RunConfig) -> RunOutcome {
    let r = integrate_segments(cfg).unwrap();
    assert!(r.success, "{}", r.message);
    r
}

fn fingerprint(r: &RunOutcome) -> String {
    let outputs: Vec<(u64, u64, Vec<u64>, Vec<u64>)> = r
        .outputs
        .iter()
        .map(|o| (o.t.to_bits(), o.epoch, bits(&o.q_edges), bits(&o.state)))
        .collect();
    format!(
        "{outputs:?}|{:?}|{:?}|{}|{}",
        r.remaps,
        r.trace,
        serde_json::to_string(&r.counters).unwrap(),
        r.attempts
    )
}

#[test]
fn segmented_runs_are_deterministic_and_conserve_the_total() {
    for linear in [Linear::Banded, Linear::Client(KRYLOV)] {
        for arm in [
            Arm::Hook { trigger: 0.5 },
            Arm::Schedule {
                times: (1..12).map(|k| 0.25 * k as f64).collect(),
            },
        ] {
            let a = run(arm.clone(), linear, 1.0e-6, outputs());
            let b = run(arm.clone(), linear, 1.0e-6, outputs());
            assert_eq!(fingerprint(&a), fingerprint(&b), "{arm:?} {linear:?}");
            // every requested output once, in order, on the mesh in force
            let times: Vec<f64> = a.outputs.iter().map(|o| o.t).collect();
            assert_eq!(times, outputs());
            let total0 = a.trace[0].total;
            for p in &a.trace {
                // the operator conserves the total to rounding; every remap
                // to MAX_RELATIVE_TOTAL_CHANGE
                assert!(
                    (p.total - total0).abs() <= 1.0e-13 * total0,
                    "{arm:?}: drift at {p:?}"
                );
            }
            for e in &a.remaps {
                assert!(e.report.relative_total_change <= MAX_RELATIVE_TOTAL_CHANGE);
            }
            if let Arm::Schedule { times } = &arm {
                let at: Vec<f64> = a.remaps.iter().map(|e| e.t).collect();
                assert_eq!(&at, times);
                // an output at a remap time is the pre-remap state
                for o in &a.outputs {
                    if let Some(k) = times.iter().position(|t| *t == o.t) {
                        assert_eq!(o.epoch, k as u64, "output at {}", o.t);
                    }
                }
            }
        }
    }
}

#[test]
fn aligned_edges_are_increasing_with_the_face_at_x_chi() {
    let d = BASE.width();
    let mut out = Vec::with_capacity(BASE.n + 3);
    let base: Vec<f64> = uniform_edges(BASE.lo, BASE.hi, BASE.n);
    let mut checked = 0;
    for m in 0..=4000 {
        let x = BASE.lo - d + (BASE.hi - BASE.lo + 2.0 * d) * m as f64 / 4000.0;
        for refine in [false, true] {
            let face = aligned_edges(BASE.lo, BASE.hi, BASE.n, x, refine, &mut out);
            assert!(out.windows(2).all(|p| p[0] < p[1]), "x={x}");
            assert_eq!(out[0], BASE.lo);
            assert_eq!(out[out.len() - 1], BASE.hi);
            let inside = x - BASE.lo > 0.25 * d && BASE.hi - x > 0.25 * d;
            match face {
                None => {
                    assert!(!inside, "x={x}");
                    assert_eq!(bits(&out), bits(&base));
                }
                Some(k) => {
                    assert!(inside, "x={x}");
                    assert_eq!(out[k].to_bits(), x.to_bits());
                    assert_eq!(out.len() - 1, BASE.n + if refine { 2 } else { 0 });
                    let widths: Vec<f64> = out.windows(2).map(|p| p[1] - p[0]).collect();
                    if refine {
                        // the halves beside the face, equal up to the rounding
                        // of the midpoint and the width subtractions
                        let scale = BASE.lo.abs().max(BASE.hi.abs());
                        for (a, b) in [(k - 2, k - 1), (k, k + 1)] {
                            assert!(
                                (widths[a] - widths[b]).abs() <= 4.0 * f64::EPSILON * scale,
                                "x={x}: halves {} {}",
                                widths[a],
                                widths[b]
                            );
                        }
                    } else {
                        // only the moved edge differs from the base grid
                        for (j, e) in out.iter().enumerate() {
                            if j != k {
                                assert_eq!(e.to_bits(), base[j].to_bits());
                            }
                        }
                        let interior = x - BASE.lo >= 0.5 * d && BASE.hi - x >= 0.5 * d;
                        let (lo, hi) = if interior {
                            (0.5 * d, 1.5 * d)
                        } else {
                            (0.25 * d, 1.75 * d)
                        };
                        for w in &widths {
                            assert!(
                                *w >= lo * (1.0 - 1e-12) && *w <= hi * (1.0 + 1e-12),
                                "x={x}: width {w} outside [{lo}, {hi}]"
                            );
                        }
                        // the nearest interior base edge moved (ties to the lower)
                        let dist = (x - base[k]).abs();
                        for (j, e) in base.iter().enumerate().take(BASE.n).skip(1) {
                            let dj = (x - e).abs();
                            assert!(dj > dist || (dj == dist && j >= k), "x={x}: {j} vs {k}");
                        }
                        checked += 1;
                    }
                }
            }
        }
    }
    assert!(checked > 3000);
    // an exact tie between base edges 10 and 11 moves edge 10
    let tie = 0.5 * (base[10] + base[11]);
    assert_eq!(
        aligned_edges(BASE.lo, BASE.hi, BASE.n, tie, false, &mut out),
        Some(10)
    );
    // an exact base edge moves onto itself
    assert_eq!(
        aligned_edges(BASE.lo, BASE.hi, BASE.n, base[7], false, &mut out),
        Some(7)
    );
    assert_eq!(bits(&out), bits(&base));
    for x in [
        f64::NAN,
        BASE.lo,
        BASE.hi,
        BASE.lo + 0.25 * d,
        BASE.hi - 0.25 * d,
    ] {
        assert_eq!(
            aligned_edges(BASE.lo, BASE.hi, BASE.n, x, true, &mut out),
            None
        );
    }
    // filling a reserved buffer allocates nothing
    let ((), allocated) = allocations_during(|| {
        for m in 0..100 {
            aligned_edges(
                BASE.lo,
                BASE.hi,
                BASE.n,
                -1.0 + 0.01 * m as f64,
                true,
                &mut out,
            );
        }
    });
    assert_eq!(allocated, 0);
}
