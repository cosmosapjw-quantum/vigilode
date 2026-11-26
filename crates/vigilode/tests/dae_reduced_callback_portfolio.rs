#![cfg(feature = "polynomial-events")]
use rodas5p_core::directed::Interval as Iv;
use rodas5p_integrators::retro_dae_evolution as de;
use serde::Serialize;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    hint::black_box,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
struct Counts {
    alloc: u64,
    zeroed: u64,
    realloc: u64,
    requested_bytes: u64,
    dealloc: u64,
    dealloc_bytes: u64,
}
impl Counts {
    fn requests(self) -> u64 {
        self.alloc + self.zeroed + self.realloc
    }
    fn add(&mut self, o: Self) {
        self.alloc += o.alloc;
        self.zeroed += o.zeroed;
        self.realloc += o.realloc;
        self.requested_bytes += o.requested_bytes;
        self.dealloc += o.dealloc;
        self.dealloc_bytes += o.dealloc_bytes;
    }
}
thread_local! {
    static TRACK:Cell<bool>=const{Cell::new(false)};
    static COUNT:Cell<Counts>=const{Cell::new(Counts{alloc:0,zeroed:0,realloc:0,requested_bytes:0,dealloc:0,dealloc_bytes:0})};
}
fn record(kind: u8, bytes: usize) {
    if TRACK.try_with(Cell::get).unwrap_or(false) {
        let _ = COUNT.try_with(|v| {
            let mut c = v.get();
            match kind {
                0 => {
                    c.alloc += 1;
                    c.requested_bytes += bytes as u64;
                }
                1 => {
                    c.zeroed += 1;
                    c.requested_bytes += bytes as u64;
                }
                2 => {
                    c.realloc += 1;
                    c.requested_bytes += bytes as u64;
                }
                _ => {
                    c.dealloc += 1;
                    c.dealloc_bytes += bytes as u64;
                }
            }
            v.set(c);
        });
    }
}
struct CountingAllocator;
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        record(0, l.size());
        unsafe { System.alloc(l) }
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        record(1, l.size());
        unsafe { System.alloc_zeroed(l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        record(2, n);
        unsafe { System.realloc(p, l, n) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        record(3, l.size());
        unsafe { System.dealloc(p, l) }
    }
}
#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator;
struct Tracking;
impl Drop for Tracking {
    fn drop(&mut self) {
        TRACK.with(|v| v.set(false));
    }
}
fn measure<T>(f: impl FnOnce() -> T) -> (T, Counts) {
    assert!(!TRACK.with(Cell::get));
    COUNT.with(|v| v.set(Counts::default()));
    TRACK.with(|v| v.set(true));
    let guard = Tracking;
    let r = f();
    drop(guard);
    (r, COUNT.with(Cell::get))
}
#[test]
fn allocator_positive_and_zero_control() {
    // Explicit direct calls with escaping pointer; no optimization-dependent Vec count.
    let (_, c) = measure(|| unsafe {
        let l = Layout::from_size_align(64, 8).unwrap();
        let p = std::alloc::alloc(l);
        assert!(!p.is_null());
        black_box(p);
        p.write(7);
        let q = std::alloc::realloc(p, l, 128);
        assert!(!q.is_null());
        black_box(q);
        assert_eq!(q.read(), 7);
        std::alloc::dealloc(q, Layout::from_size_align(128, 8).unwrap());
        let z = std::alloc::alloc_zeroed(l);
        assert!(!z.is_null());
        black_box(z);
        assert_eq!(z.read(), 0);
        std::alloc::dealloc(z, l);
    });
    assert_eq!(
        c,
        Counts {
            alloc: 1,
            zeroed: 1,
            realloc: 1,
            requested_bytes: 256,
            dealloc: 2,
            dealloc_bytes: 192
        }
    );
    let (_, z) = measure(|| black_box([1., 2., 3.]));
    assert_eq!(z, Counts::default());
    println!(
        "CONTROL {}",
        serde_json::json!({"positive":c,"stack_zero":z})
    );
}

use vigilode::dae_evolution::*;
use vigilode::polynomial_events::{
    AffineFactor as F, DeclaredPolynomial as P, EventTerm as T, retro_dae_init::*,
};
use vigilode::{Atol, InteriorError, Options};
fn p(v: Vec<T>) -> P {
    P::new(v).unwrap()
}
fn tr(coefficient: f64, factors: Vec<F>) -> T {
    T {
        coefficient,
        factors,
    }
}
fn opts() -> Options {
    Options {
        rtol: 1e-8,
        atol: Atol::Scalar(1e-11),
        dense_output: true,
        interior_error: InteriorError::Off,
        max_steps: 4096,
        ..Default::default()
    }
}
fn policy() -> EvolutionPolicy {
    EvolutionPolicy {
        init: InitPolicy {
            mode: InitMode::ProjectAlgebraic,
            state_tol: 1e-11,
            residual_tol: 1e-11,
            derivative_tol: 1e-6,
            ..Default::default()
        },
        tube_radius: 1e-3,
        global_tol: 1e-3,
        green: true,
    }
}
fn mp(method: MultiGuardRoute) -> MultiGuardPolicy {
    MultiGuardPolicy {
        method,
        tolerance: 1e-6,
        root_route: ThresholdRoute::DictionaryHint,
        derivative: GuardDerivativeRoute::Schur,
        value: GuardValueRoute::Transport,
        pending: PendingPolicy {
            method: PendingMethod::PiecewiseProbe,
            max_slabs: 8,
        },
        max_slabs: 128,
    }
}
fn rp(sync: VectorSyncRoute) -> VectorResetPolicy {
    VectorResetPolicy {
        sync,
        derivative: GuardDerivativeRoute::Schur,
        branch_route: InitRoute::Reuse,
        subdivisions: 1,
    }
}
fn cycle(
    overlap: bool,
    route: EvolutionRoute,
    cert: ConservationRoute,
    sync: VectorSyncRoute,
) -> ConservedMode {
    let v = 1. / 8192.;
    let q = 1. / 16384.;
    let h = |q: f64| p(vec![tr(q, vec![]), tr(-1., vec![F::component(1, 0.)])]);
    let reset = SimplexReset::new([[1. - v; 3], [v; 3], [0.; 3]]).unwrap();
    let mut pol = rp(sync);
    pol.branch_route = match route {
        EvolutionRoute::Prepared5 => InitRoute::PreparedBatch,
        EvolutionRoute::Scalar5 => InitRoute::ScalarRoot,
        EvolutionRoute::Affine5 => InitRoute::AffineExact,
        _ => InitRoute::Reuse,
    };
    ConservedMode {
        rate: 3e7,
        route,
        certificate: cert,
        guards: vec![
            h(q),
            h(q - if overlap {
                2f64.powi(-28)
            } else {
                2f64.powi(-18)
            }),
        ],
        transitions: (0..2)
            .map(|_| ConservedTransition {
                target: 0,
                reset: reset.clone(),
                policy: pol,
                sync: SimplexSyncRoute::JointTransport,
            })
            .collect(),
    }
}
fn initial(rad: f64) -> RobertsonInitial {
    let v = 1. / 8192.;
    RobertsonInitial::simplex_ball([1. - v, v], rad).unwrap()
}
fn pm(order: MultiGuardRoute) -> MultiGuardPolicy {
    let mut p = mp(order);
    p.derivative = GuardDerivativeRoute::ConservedSchur;
    p
}
fn limits() -> PhaseCycleBudget {
    PhaseCycleBudget {
        native_attempts: 4096,
        scan_slabs: 512,
        transitions: 3,
        min_dwell: 1e-5,
        absolute_width: 1e-6,
    }
}
fn constant(v: f64) -> SimplexReset {
    SimplexReset::new([[1. - v; 3], [v; 3], [0.; 3]]).unwrap()
}
fn graph(case: &str, solver: EvolutionRoute, cert: ConservationRoute) -> Vec<ConservedMode> {
    let mut a = cycle(case == "overlap", solver, cert, VectorSyncRoute::Centered);
    let mut b = cycle(case == "overlap", solver, cert, VectorSyncRoute::Centered);
    b.rate = 6e7;
    if case == "distinct" {
        let mut c = cycle(false, solver, cert, VectorSyncRoute::Centered);
        for e in &mut a.transitions {
            e.target = 1;
            e.reset = constant(1. / 8192.);
        }
        for e in &mut b.transitions {
            e.target = 2;
            e.reset = constant(5. / 32768.);
        }
        for e in &mut c.transitions {
            e.target = 1;
            e.reset = constant(3. / 16384.);
        }
        {
            a.transitions[1].target = 0;
            a.transitions[1].reset = constant(3. / 16384.);
            vec![a, b, c]
        }
    } else {
        for e in &mut a.transitions {
            e.target = 1;
            e.reset = constant(3. / 16384.);
        }
        for e in &mut b.transitions {
            e.target = 0;
            e.reset = constant(5. / 32768.);
        }
        {
            a.transitions[1].target = 0;
            a.transitions[1].reset = constant(1. / 8192.);
            vec![a, b]
        }
    }
}
fn atlas_budget() -> PhaseAtlasBudget {
    PhaseAtlasBudget {
        limits: PhaseCycleBudget {
            transitions: 10,
            ..limits()
        },
        reset_checks: 64,
        cache_entries: 10,
    }
}

fn power(s: &SlabRecord) -> Vec<Vec<Iv>> {
    s.power
        .iter()
        .map(|row| row.iter().map(|a| Iv::new(a[0], a[1]).unwrap()).collect())
        .collect()
}
#[test]
fn failed_picard_retains_branch_initialization() {
    let src = DaeSource::new(
        vec![p(vec![tr(1., vec![])])],
        vec![p(vec![
            tr(1., vec![F::component(1, 0.)]),
            tr(-1., vec![F::component(0, 0.)]),
        ])],
    )
    .unwrap();
    let (r, w) = de::certify_vector_tube_audited(
        &src,
        [0., 1.],
        &[[0., 0.]],
        &[0.],
        &[[-0.25, 0.25]],
        policy().init,
        InitRoute::Reuse,
    );
    assert!(r.is_err());
    assert_eq!(w.tubes.started, 1);
    assert_eq!(w.tubes.returned, 0);
    assert!(w.branches.started > w.branches.returned);
    assert!(w.branches.returned >= 1);
    assert!(w.branch_initialization.calls.returned >= 1);
    assert!(w.branch_initialization.rhs_returned > 0);
    println!(
        "PICARD_RED_GREEN {}",
        serde_json::json!({"error":r.unwrap_err().to_string(),"work":w})
    );
}

fn check_count(c: AttemptCount) {
    assert!(c.returned <= c.started);
}
fn check_projection(w: &ProjectionAttemptWork) {
    for c in [w.calls, w.evaluations, w.linear, w.frozen, w.roots] {
        check_count(c);
    }
    assert!(w.rhs_returned <= w.rhs_requested);
    assert_eq!(
        w.rhs_requested,
        w.successful_rhs_requested + w.refused_rhs_requested
    );
}
fn check_guard(w: &de::GuardKernelAudit) {
    for c in [
        w.builds,
        w.evaluations,
        w.preconditioners,
        w.reduced_columns,
    ] {
        check_count(c);
    }
    assert!(w.rhs_returned <= w.rhs_requested);
}
fn check_kernel(w: &de::ResetKernelAudit) {
    for c in [
        w.components,
        w.branches,
        w.branch_evaluations,
        w.branch_preconditioners,
        w.branch_columns,
        w.tubes,
        w.picard_images,
        w.synchronizations,
        w.sync_components,
        w.green,
    ] {
        check_count(c);
    }
    check_guard(&w.component_guards);
    check_projection(&w.branch_initialization);
    assert!(w.branch_rhs_returned <= w.branch_rhs_requested);
    assert!(w.strict_images <= w.picard_images.returned);
}
fn check_native(n: &NativeResourceAudit) {
    check_guard(&n.guard_kernel);
    check_kernel(&n.difference_kernel);
    check_count(n.difference_comparisons);
    assert_eq!(n.guard_kernel.builds, n.guard_build);
    assert!(n.positive_differences <= n.difference_comparisons.returned);
    if let Some(s) = &n.stats {
        assert!(n.driver_invoked);
        assert_eq!(s.attempts, s.accepted_steps + s.rejected_steps);
    }
}
fn validate_chain(r: &ConservedChainRun, a: &ConservedChainResourceAudit) {
    assert!(a.admitted);
    let w = &r.work().common;
    assert_eq!(
        a.native
            .iter()
            .map(|v| v.charged_native_attempts)
            .sum::<usize>(),
        w.charged_native_attempts
    );
    assert_eq!(
        a.native.iter().map(|v| v.charged_scan_slabs).sum::<usize>(),
        w.charged_scan_slabs
    );
    assert_eq!(a.transitions.len(), w.transitions_attempted);
    assert_eq!(
        a.transitions.iter().map(|v| v.charged_cells).sum::<usize>(),
        w.charged_reset_cells
    );
    assert_eq!(
        a.transitions
            .iter()
            .map(|v| v.charged_picard_attempts)
            .sum::<usize>(),
        r.work().charged_picard_attempts
    );
    assert_eq!(
        a.transitions
            .iter()
            .map(|v| v.work.gap_checks.started)
            .sum::<usize>(),
        w.gap_checks
    );
    assert_eq!(
        a.transitions.iter().filter(|v| v.work.committed).count(),
        w.transitions_committed
    );
    for n in &a.native {
        check_native(&n.native);
        assert_eq!(n.measured_attempts, n.native.measured_attempts());
        if n.returned {
            assert_eq!(Some(n.charged_native_attempts), n.measured_attempts);
        } else {
            assert_eq!(n.charged_native_attempts, n.native_quota);
        }
    }
    for (t, e) in a.transitions.iter().zip(r.transitions()) {
        assert_eq!(
            (t.from, t.target, t.guard, t.root),
            (e.from, e.target, e.guard, e.event_time)
        );
        assert_eq!(t.work.committed, e.committed);
        let q = &t.work;
        check_kernel(&q.kernel);
        for c in [
            q.preparation,
            q.table_setup,
            q.cells,
            q.restart_branch,
            q.transport,
            q.transport_cells,
            q.gap_checks,
        ] {
            check_count(c);
        }
        assert!(q.negative_gaps <= q.gap_checks.returned);
        assert!(q.kernel.branches.started <= t.charged_picard_attempts);
        if let Some(prep) = &e.reset {
            assert_eq!(q.cells.started, prep.cells.len());
            let proved = prep
                .cells
                .iter()
                .filter_map(|c| c.tube.as_ref())
                .map(|v| v.attempts.len())
                .sum::<usize>();
            assert!(
                q.kernel.branches.returned >= proved + usize::from(prep.restart_branch.is_some())
            );
            assert!(
                q.kernel.green.returned
                    >= prep
                        .cells
                        .iter()
                        .filter_map(|c| c.sync.as_ref())
                        .map(|v| v.green_calls)
                        .sum()
            );
        }
        if e.committed {
            assert_eq!(q.cells.started, q.cells.returned);
            assert_eq!(q.gap_checks.returned, q.negative_gaps);
            assert_eq!(q.restart_branch.returned, 1);
        }
    }
}
#[test]
fn branch_and_green_partial_failures() {
    use rodas5p_core::retro_dictionary::RealMode;
    use rodas5p_integrators::retro_green::RetroTable;
    let src = DaeSource::new(
        vec![p(vec![tr(1., vec![])])],
        vec![p(vec![
            tr(
                1.,
                vec![
                    F {
                        constant: 1.,
                        time: 0.,
                        state: vec![(0, 1.)],
                    },
                    F::component(1, 0.),
                ],
            ),
            tr(-1., vec![F::component(0, 0.)]),
        ])],
    )
    .unwrap();
    let mut a = de::ResetKernelAudit::default();
    let r = de::certify_vector_branch_recorded(
        &src,
        [0., 1.],
        &[[-2., 0.]],
        &[0.],
        &[0.],
        &[[-0.25, 0.25]],
        policy().init,
        InitRoute::Reuse,
        &mut a,
    );
    assert!(r.is_err());
    assert_eq!(a.branch_initialization.calls.returned, 1);
    assert_eq!(a.branches.started, 1);
    assert_eq!(a.branches.returned, 0);
    assert_eq!(a.branch_rhs_returned, 1);
    println!(
        "BRANCH_FAILURE {}",
        serde_json::json!({"error":r.unwrap_err().to_string(),"work":a})
    );
    let mut m = graph(
        "alternating",
        EvolutionRoute::Mass,
        ConservationRoute::Integral4,
    );
    for mode in &mut m {
        for edge in &mut mode.transitions {
            edge.policy.sync = VectorSyncRoute::GreenMoment;
        }
    }
    let (r, w) = evolve_conserved_chain_audited(
        &m,
        0,
        initial(2f64.powi(-26)),
        [0., 0.002],
        &opts(),
        policy(),
        pm(MultiGuardRoute::Correlated),
        chain_limits(),
        SimplexGapRoute::AffineDual,
    );
    let r = r.unwrap();
    validate_chain(&r, &w);
    let e = &r.transitions()[0];
    assert!(e.committed);
    let prep = e.reset.as_ref().unwrap();
    let c = &prep.cells[0];
    let tube = c.tube.as_ref().unwrap();
    let table = RetroTable::with_dictionary(RealMode::Hybrid).unwrap();
    let mut a = de::ResetKernelAudit::default();
    let s = de::synchronize_vector_recorded(
        &c.reset,
        c.times,
        e.event_time[1],
        tube,
        VectorSyncRoute::GreenMoment,
        &table,
        &mut a,
    )
    .unwrap();
    assert_eq!(
        serde_json::to_string(&s).unwrap(),
        serde_json::to_string(c.sync.as_ref().unwrap()).unwrap()
    );
    assert_eq!(a.green.returned, 8);
    assert_eq!(a.sync_components.returned, 2);
    // Explicit malformed-certificate negative control, never used as physical authority.
    let mut malformed = tube.clone();
    malformed.attempts.last_mut().unwrap().nominal_field[1] = [f64::NAN, f64::NAN];
    let mut b = de::ResetKernelAudit::default();
    let v = de::synchronize_vector_recorded(
        &c.reset,
        c.times,
        e.event_time[1],
        &malformed,
        VectorSyncRoute::GreenMoment,
        &table,
        &mut b,
    );
    assert!(v.is_err());
    assert_eq!(b.synchronizations.started, 1);
    assert_eq!(b.synchronizations.returned, 0);
    assert_eq!(b.sync_components.started, 2);
    assert_eq!(b.sync_components.returned, 1);
    assert_eq!(b.green.returned, 4);
    println!(
        "GREEN_FAILURE {}",
        serde_json::json!({"error":v.unwrap_err().to_string(),"work":b,"control":a})
    );
    let s = r.epochs()[0]
        .original_slab_diagnostics()
        .iter()
        .find(|s| s.times[0] <= c.times[0] && c.times[1] <= s.times[1])
        .unwrap();
    let model = RobertsonModel::new(m[0].rate).unwrap();
    let y = power(s);
    let cert = s.certificate.as_ref().unwrap();
    let pol = m[0].transitions[0].reset.polynomials().unwrap();
    let mut a = de::ResetKernelAudit::default();
    assert!(
        de::vector_reset_component_recorded(
            model.source(),
            &pol[0],
            &y,
            cert,
            [s.times[1], s.times[1] + 1.],
            VectorSyncRoute::Centered,
            GuardDerivativeRoute::PreparedSchur,
            &mut a
        )
        .is_err()
    );
    assert_eq!(a.components.started, 1);
    assert_eq!(a.components.returned, 0);
    assert_eq!(a.component_guards.builds.started, 0);
    // Invalid original branch defect is deliberately injected only to test
    // the diagnostic path after PreparedSchur has already solved its RHS.
    let mut invalid = cert.clone();
    invalid.branch_defect = 1.;
    let mut a = de::GuardKernelAudit::default();
    let q = de::GuardProof::new_recorded(
        model.source(),
        &pol[0],
        &y,
        &invalid,
        GuardDerivativeRoute::PreparedSchur,
        GuardValueRoute::Transport,
        &mut a,
    );
    assert!(q.is_err());
    assert_eq!(a.builds.started, 1);
    assert_eq!(a.builds.returned, 0);
    assert_eq!(a.rhs_returned, 1);
    println!(
        "GUARD_FAILURE {}",
        serde_json::json!({"work":a,"error":"late original branch defect"})
    );
}
#[test]
fn chain_refusals() {
    use serde_json::json;
    for name in [
        "native",
        "slab",
        "transition",
        "reset",
        "picard",
        "gap",
        "active_target",
        "tie",
        "warp",
        "driver_options",
        "terminal",
        "terminal_warp",
    ] {
        let route = if name == "warp" || name == "terminal_warp" {
            EvolutionRoute::WarpFolded
        } else {
            EvolutionRoute::Mass
        };
        let mut m = graph("alternating", route, ConservationRoute::Integral4);
        let mut b = chain_limits();
        let mut o = opts();
        match name {
            "native" => b.limits.native_attempts = 1,
            "slab" => b.limits.scan_slabs = 1,
            "transition" => b.limits.transition_attempts = 0,
            "reset" => b.limits.reset_cells = 0,
            "picard" => b.picard_attempts = 8,
            "gap" => b.limits.gap_checks = 1,
            "active_target" => m[0].transitions[0].reset = constant(0.),
            "tie" => m[0].guards[1] = m[0].guards[0].clone(),
            "driver_options" => o.rtol = -1.,
            "terminal" | "terminal_warp" => {
                m[0].guards.clear();
                m[0].transitions.clear();
            }
            _ => {}
        }
        let (r, a) = evolve_conserved_chain_audited(
            &m,
            0,
            initial(2f64.powi(-26)),
            [0., 0.002],
            &o,
            policy(),
            pm(MultiGuardRoute::Correlated),
            b,
            SimplexGapRoute::AffineDual,
        );
        let r = r.unwrap();
        validate_chain(&r, &a);
        let old = evolve_conserved_chain(
            &m,
            0,
            initial(2f64.powi(-26)),
            [0., 0.002],
            &o,
            policy(),
            pm(MultiGuardRoute::Correlated),
            b,
            SimplexGapRoute::AffineDual,
        )
        .unwrap();
        assert_eq!(r.stop(), old.stop());
        assert_eq!(r.final_mode(), old.final_mode());
        assert_eq!(
            serde_json::to_string(r.work()).unwrap(),
            serde_json::to_string(old.work()).unwrap()
        );
        assert_eq!(
            serde_json::to_string(r.transitions()).unwrap(),
            serde_json::to_string(old.transitions()).unwrap()
        );
        if matches!(name, "warp" | "terminal_warp") {
            assert_eq!(a.native[0].measured_attempts, Some(0));
            assert!(!a.native[0].native.driver_invoked);
        }
        if name == "driver_options" {
            assert_eq!(a.native[0].measured_attempts, None);
        }
        if name == "gap" {
            assert_eq!(r.stop(), ChainStop::GapBudget);
            assert!(!a.transitions[0].work.committed);
            assert_eq!(a.transitions[0].work.gap_checks.started, 1);
        }
        println!(
            "CHAIN_REFUSAL {}",
            json!({"case":name,"stop":r.stop(),"final_mode":r.final_mode(),"work":r.work(),"audit":a})
        );
    }
}

fn validate_atlas(r: &PhaseAtlasRun, a: &AtlasResourceAudit) {
    assert!(a.admitted);
    let w = &r.work().phase;
    assert_eq!(
        w.charged_native_attempts,
        a.native
            .iter()
            .map(|v| v.charged_native_attempts)
            .sum::<usize>()
    );
    assert_eq!(
        w.charged_scan_slabs,
        a.native.iter().map(|v| v.charged_scan_slabs).sum::<usize>()
    );
    assert_eq!(w.reset_checks, a.reset_guards.started);
    assert_eq!(w.committed, a.committed);
    assert_eq!(w.cache_hits, a.cache_hits);
    assert!(a.negative_reset_guards <= a.reset_guards.returned);
    let mut idx = 0;
    for n in &a.native {
        check_native(&n.native);
        assert_eq!(n.measured_attempts, n.native.measured_attempts());
        if n.returned {
            let p = &r.phases()[idx];
            idx += 1;
            let mut it = p.ordered().native_stats();
            let s = it.next().unwrap();
            assert!(it.next().is_none());
            assert_eq!(
                serde_json::to_value(s).unwrap(),
                serde_json::to_value(n.native.stats.as_ref().unwrap()).unwrap()
            );
            assert_eq!(n.charged_native_attempts, s.attempts);
            assert_eq!(n.charged_scan_slabs, s.accepted_steps);
            assert_eq!(
                n.native.point_projection.successful_rhs_requested
                    + n.native.replay_nodes.successful_rhs_requested,
                p.ordered().work.replay_projection_rhs
            );
            // On a fully returned reconstruction, all node reports contribute to
            // the legacy scan count. A failed reconstruction can discard a prefix.
            assert!(
                n.native.scan_nodes.successful_rhs_requested
                    >= p.ordered().work.shared_projection.rhs_count
            );
        } else {
            assert_eq!(n.charged_native_attempts, n.native_quota);
            assert_eq!(n.charged_scan_slabs, n.slab_quota);
        }
    }
    assert_eq!(idx, r.phases().len());
}

#[derive(Default, Serialize)]
struct QueryWork {
    calls: usize,
    refused: usize,
    cells: usize,
    allocation: Counts,
}
impl QueryWork {
    fn query<T>(
        &mut self,
        f: impl FnOnce() -> Result<T, rodas5p_core::CoreError>,
    ) -> Result<T, rodas5p_core::CoreError> {
        let (r, a) = measure(f);
        self.calls += 1;
        self.refused += usize::from(r.is_err());
        self.allocation.add(a);
        r
    }
}

fn chain_limits() -> ConservedChainBudget {
    ConservedChainBudget {
        limits: ChainBudget {
            native_attempts: 4096,
            scan_slabs: 512,
            transition_attempts: 10,
            reset_cells: 64,
            gap_checks: 256,
            min_dwell: 1e-5,
        },
        picard_attempts: 512,
    }
}

fn all_training_impl(audited: bool) {
    let m = graph(
        "alternating",
        EvolutionRoute::Affine5,
        ConservationRoute::Integral4,
    );
    let _ = evolve_robertson_phase_atlas(
        &m,
        0,
        initial(2f64.powi(-26)),
        [0., 0.002],
        &opts(),
        policy(),
        pm(MultiGuardRoute::Correlated),
        atlas_budget(),
        PhaseReuseRoute::Reuse,
    )
    .unwrap();
    use serde_json::json;
    for case in ["alternating", "distinct", "overlap"] {
        for solver in [EvolutionRoute::Reduced5] {
            for cert in [ConservationRoute::Max, ConservationRoute::Integral4] {
                for order in [MultiGuardRoute::Shared, MultiGuardRoute::Correlated] {
                    for method in ["JointCentered", "JointGreen", "AtlasFresh", "AtlasReuse"] {
                        let mut m = graph(case, solver, cert);
                        if method == "JointGreen" {
                            for a in &mut m {
                                for e in &mut a.transitions {
                                    e.policy.sync = VectorSyncRoute::GreenMoment;
                                }
                            }
                        }
                        let mut row = json!({"case":case,"solver":solver,"cert":cert,"order":order,"method":method,"radius":2f64.powi(-26)});
                        if method.starts_with("Atlas") {
                            let ((r, audit), run_alloc) = measure(|| {
                                if audited {
                                    evolve_robertson_phase_atlas_audited(
                                        &m,
                                        0,
                                        initial(2f64.powi(-26)),
                                        [0., 0.002],
                                        &opts(),
                                        policy(),
                                        pm(order),
                                        atlas_budget(),
                                        if method == "AtlasReuse" {
                                            PhaseReuseRoute::Reuse
                                        } else {
                                            PhaseReuseRoute::Fresh
                                        },
                                    )
                                } else {
                                    (
                                        evolve_robertson_phase_atlas(
                                            &m,
                                            0,
                                            initial(2f64.powi(-26)),
                                            [0., 0.002],
                                            &opts(),
                                            policy(),
                                            pm(order),
                                            atlas_budget(),
                                            if method == "AtlasReuse" {
                                                PhaseReuseRoute::Reuse
                                            } else {
                                                PhaseReuseRoute::Fresh
                                            },
                                        ),
                                        AtlasResourceAudit::default(),
                                    )
                                }
                            });
                            let r = r.unwrap();
                            if audited {
                                validate_atlas(&r, &audit);
                                println!(
                                    "ATLAS_AUDIT {}",
                                    json!({"case":case,"solver":solver,"cert":cert,"order":order,"method":method,"audit":audit})
                                );
                            }
                            let mut hwork = QueryWork::default();
                            let mut cwork = QueryWork::default();
                            let mut qs = vec![];
                            for (i, e) in r.epochs().iter().enumerate() {
                                let ep = &e.phase;
                                if ep.absolute_safe <= ep.clock[1] {
                                    continue;
                                }
                                for j in 0..=4 {
                                    let t = if j == 4
                                        && r.stop() == PhaseAtlasStop::Horizon
                                        && ep.absolute_safe == 0.002
                                    {
                                        0.002
                                    } else {
                                        ep.clock[1]
                                            + (ep.absolute_safe - ep.clock[1])
                                                * ((j as f64 + 0.5) / 5.)
                                    };
                                    let a = hwork
                                        .query(|| r.query(i, t, PhaseQueryRoute::Horner))
                                        .unwrap();
                                    hwork.cells += a.cells.len();
                                    let b = cwork
                                        .query(|| r.query(i, t, PhaseQueryRoute::Centered))
                                        .unwrap();
                                    cwork.cells += b.cells.len();
                                    qs.push(json!({"epoch":i,"horner":a,"centered":b}));
                                }
                            }
                            let driver_stats = r
                                .phases()
                                .iter()
                                .map(|p| p.ordered().event_authority().map(|a| a.stats()))
                                .collect::<Vec<_>>();
                            println!(
                                "RESOURCE {}",
                                json!({"case":case,"solver":solver,"cert":cert,"order":order,"method":method,"solve_alloc":run_alloc,"horner":hwork,"centered":cwork,"driver_stats":driver_stats})
                            );
                            row["atlas"] = json!({"stop":r.stop(),"reason":r.reason(),"final_mode":r.final_mode(),"work":r.work(),"phases":r.phases().iter().map(|p|json!({"mode":p.mode,"initial":p.initial,"winner":p.ordered().winner(),"safe":p.ordered().committed_until(),"reason":p.ordered().reason(),"guards":p.ordered().guards(),"comparisons":p.ordered().comparisons(),"work":p.ordered().work,"slabs":p.original_slab_diagnostics()})).collect::<Vec<_>>(),"epochs":r.epochs(),"events":r.events(),"queries":qs});
                        } else {
                            let ((r, audit), run_alloc) = measure(|| {
                                if audited {
                                    evolve_conserved_chain_audited(
                                        &m,
                                        0,
                                        initial(2f64.powi(-26)),
                                        [0., 0.002],
                                        &opts(),
                                        policy(),
                                        pm(order),
                                        chain_limits(),
                                        SimplexGapRoute::AffineDual,
                                    )
                                } else {
                                    (
                                        evolve_conserved_chain(
                                            &m,
                                            0,
                                            initial(2f64.powi(-26)),
                                            [0., 0.002],
                                            &opts(),
                                            policy(),
                                            pm(order),
                                            chain_limits(),
                                            SimplexGapRoute::AffineDual,
                                        ),
                                        ConservedChainResourceAudit::default(),
                                    )
                                }
                            });
                            let r = r.unwrap();
                            if audited {
                                validate_chain(&r, &audit);
                                println!(
                                    "CHAIN_AUDIT {}",
                                    json!({"case":case,"solver":solver,"cert":cert,"order":order,"method":method,"audit":audit})
                                );
                            }
                            let mut qwork = QueryWork::default();
                            let mut es = vec![];
                            for (i, e) in r.epochs().iter().enumerate() {
                                let qs = if e.safe() > e.start() {
                                    (0..=4)
                                        .filter_map(|j| {
                                            qwork
                                                .query(|| {
                                                    r.query(
                                                        i,
                                                        if j == 4 {
                                                            e.safe()
                                                        } else {
                                                            e.start()
                                                                + (e.safe() - e.start())
                                                                    * (j as f64 / 4.)
                                                        },
                                                    )
                                                })
                                                .ok()
                                        })
                                        .collect::<Vec<_>>()
                                } else {
                                    vec![]
                                };
                                es.push(json!({"mode":e.mode(),"start":e.start(),"safe":e.safe(),"radius":e.incoming_l1(),"center":e.center(),"slabs":e.original_slab_diagnostics(),"queries":qs}));
                            }
                            let driver_stats = r
                                .epochs()
                                .iter()
                                .map(|e| {
                                    e.ordered()
                                        .and_then(|a| a.event_authority())
                                        .map(|a| a.stats())
                                })
                                .collect::<Vec<_>>();
                            println!(
                                "RESOURCE {}",
                                json!({"case":case,"solver":solver,"cert":cert,"order":order,"method":method,"solve_alloc":run_alloc,"point":qwork,"driver_stats":driver_stats})
                            );
                            row["joint"] = json!({"stop":r.stop(),"reason":r.reason(),"final_mode":r.final_mode(),"work":r.work(),"events":r.transitions(),"epochs":es});
                        }
                        println!("CELL {}", row);
                    }
                }
            }
        }
    }
}

#[test]
#[ignore = "fixed192 physical replay"]
fn training() {
    all_training_impl(false);
}
#[test]
#[ignore = "fixed192 audited Joint and Atlas candidates"]
fn audited_training() {
    all_training_impl(true);
}
