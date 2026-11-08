//! Comprehensive verification contract suite for the unified production solver (`VigilODE`).
//!
//! Validates the 6 core pillars of the unified solver architecture:
//! - Contract 1: Single frontend entrypoint (`VigilODE::solve`) across diverse problems.
//! - Contract 2: Zero-allocation pre-allocated workspace (`VigilodeWorkspace`).
//! - Contract 3: Autonomous multi-tier polyalgorithm routing.
//! - Contract 4: Fail-closed protected fallback guarantee.
//! - Contract 5: Physical invariant conservation and drift monitoring.
//! - Contract 6: Complete work accounting & timing authority governance.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use vigilode::prelude::*;

struct CountingAlloc;
thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static COUNT: Cell<usize> = const { Cell::new(0) };
    static BYTES: Cell<usize> = const { Cell::new(0) };
}

fn count_alloc(layout: Layout) {
    if TRACK.try_with(Cell::get).unwrap_or(false) {
        let _ = COUNT.try_with(|c| c.set(c.get() + 1));
        let _ = BYTES.try_with(|b| b.set(b.get() + layout.size()));
    }
}

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count_alloc(layout);
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count_alloc(layout);
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count_alloc(layout);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAlloc = CountingAlloc;

fn allocation_scope<T>(f: impl FnOnce() -> T) -> (T, usize, usize) {
    COUNT.with(|c| c.set(0));
    BYTES.with(|b| b.set(0));
    TRACK.with(|t| t.set(true));
    let result = f();
    TRACK.with(|t| t.set(false));
    (result, COUNT.with(Cell::get), BYTES.with(Cell::get))
}

fn robertson_problem() -> Problem {
    Problem::new(3, |_t, y, dy| {
        dy[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
        dy[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
        dy[2] = 3.0e7 * y[1] * y[1];
        Ok(())
    })
    .jacobian(Jacobian::dense(|_t, y, jac| {
        jac.copy_from_slice(&[
            -0.04,
            1.0e4 * y[2],
            1.0e4 * y[1],
            0.04,
            -1.0e4 * y[2] - 6.0e7 * y[1],
            -1.0e4 * y[1],
            0.0,
            6.0e7 * y[1],
            0.0,
        ]);
        Ok(())
    }))
    .invariant("mass", |y| y[0] + y[1] + y[2])
    .autonomous()
}

fn harmonic_oscillator() -> Problem {
    Problem::new(2, |_t, y, dy| {
        dy[0] = y[1];
        dy[1] = -y[0];
        Ok(())
    })
    .invariant("energy", |y| y[0] * y[0] + y[1] * y[1])
    .autonomous()
}

fn heat_equation_jvp(n: usize) -> Problem {
    let dx = 1.0 / (n as f64 + 1.0);
    let inv_dx2 = 1.0 / (dx * dx);

    Problem::new(n, move |_t, y, dy| {
        for i in 0..n {
            let left = if i == 0 { 0.0 } else { y[i - 1] };
            let right = if i + 1 == n { 0.0 } else { y[i + 1] };
            dy[i] = (left - 2.0 * y[i] + right) * inv_dx2;
        }
        Ok(())
    })
    .jvp(move |_t, _y, v, out| {
        for i in 0..n {
            let left = if i == 0 { 0.0 } else { v[i - 1] };
            let right = if i + 1 == n { 0.0 } else { v[i + 1] };
            out[i] = (left - 2.0 * v[i] + right) * inv_dx2;
        }
        Ok(())
    })
    .autonomous()
}

#[test]
fn contract_1_single_api_ergonomics() {
    // 1. Nonstiff oscillator
    let osc = harmonic_oscillator();
    let osc_sol = VigilODE::solve(
        &osc,
        (0.0, std::f64::consts::TAU),
        &[1.0, 0.0],
        SolverStrategy::Auto,
    )
    .expect("oscillator solve");
    assert!(osc_sol.success(), "oscillator should succeed");
    let y_final = osc_sol.final_state().expect("final state");
    assert!(
        (y_final[0] - 1.0).abs() < 1e-4,
        "oscillator period accuracy"
    );

    // 2. Stiff Robertson chemical kinetics
    let rob = robertson_problem();
    let rob_sol = VigilODE::solve(&rob, (0.0, 40.0), &[1.0, 0.0, 0.0], SolverStrategy::Auto)
        .expect("robertson solve");
    assert!(rob_sol.success(), "robertson should succeed");
    assert!(rob_sol.stats().accepted_steps > 0);

    // 3. High-dimensional matrix-free heat diffusion
    let heat = heat_equation_jvp(20);
    let y0 = vec![1.0; 20];
    let heat_sol = VigilODE::solve(&heat, (0.0, 0.1), &y0, SolverStrategy::HighPerformance)
        .expect("heat solve");
    assert!(heat_sol.success(), "heat equation should succeed");
}

#[test]
fn contract_2_workspace_reuse_and_retained_bytes() {
    let osc = harmonic_oscillator();
    let mut ws = VigilodeWorkspace::new(2, 20);
    let initial_bytes = ws.retained_bytes();
    assert!(initial_bytes > 0, "workspace must pre-allocate storage");

    let opts = VigilodeOptions::default().strategy(SolverStrategy::Auto);

    let sol1 = VigilODE::solve_with_workspace(&osc, (0.0, 1.0), &[1.0, 0.0], &opts, &mut ws)
        .expect("solve 1");
    assert!(sol1.success());

    // Second solve reusing the exact same workspace
    let sol2 = VigilODE::solve_with_workspace(&osc, (1.0, 2.0), &[0.5, 0.5], &opts, &mut ws)
        .expect("solve 2");
    assert!(sol2.success());

    assert_eq!(
        ws.retained_bytes(),
        initial_bytes,
        "workspace capacity should remain constant across solves"
    );

    // VG-1 hot-loop stepping zero-allocation verification: 100 consecutive steps
    let mut y = [1.0, 0.0];
    let mut y_next = [0.0, 0.0];
    let (_, allocs, bytes) = allocation_scope(|| {
        for i in 0..100 {
            ws.step_into(&osc, i as f64 * 0.01, &y, 0.01, &mut y_next)
                .expect("step");
            y = y_next;
        }
    });

    assert_eq!(
        allocs, 0,
        "VG-1: stepping hot loop must execute with exactly 0 heap allocations, measured {allocs} allocs ({bytes} bytes)"
    );
}

#[test]
fn contract_3_autonomous_tier_routing() {
    // Nonstiff problem should route to Tier1ExplicitDopri5
    let osc = harmonic_oscillator();
    let osc_sol = VigilODE::solve(&osc, (0.0, 1.0), &[1.0, 0.0], SolverStrategy::Auto)
        .expect("oscillator solve");
    assert!(osc_sol.success());
    let first_tier = &osc_sol
        .route_history
        .first()
        .expect("route history")
        .to_tier;
    assert_eq!(first_tier, "Tier1ExplicitDopri5");

    // Stiff Robertson with dense Jacobian should route to Stiff/Protected
    let rob = robertson_problem();
    let rob_sol = VigilODE::solve(&rob, (0.0, 1.0), &[1.0, 0.0, 0.0], SolverStrategy::Auto)
        .expect("robertson solve");
    assert!(rob_sol.success());
    let first_tier_rob = &rob_sol
        .route_history
        .first()
        .expect("route history")
        .to_tier;
    assert!(
        first_tier_rob.contains("Protected") || first_tier_rob.contains("Staged"),
        "expected stiff routing, got {first_tier_rob}"
    );

    // High-dimensional JVP problem should route to Tier2MatrixFreeKrylov
    let heat = heat_equation_jvp(25);
    let y0 = vec![1.0; 25];
    let heat_sol = VigilODE::solve(&heat, (0.0, 0.01), &y0, SolverStrategy::HighPerformance)
        .expect("heat solve");
    assert!(heat_sol.success());
    let heat_tier = &heat_sol
        .route_history
        .first()
        .expect("route history")
        .to_tier;
    assert_eq!(heat_tier, "Tier2MatrixFreeKrylov");
}

#[test]
fn contract_4_fail_closed_protected_fallback() {
    // Robertson without explicit Jacobian initially routes to high-speed Tier1Explicit.
    // As stiff reaction kinetics (rate 3e7) activate, DOPRI5 encounters severe step rejection.
    // The solver must detect this and fall back to Protected RODAS5P!
    let rob = Problem::new(3, |_t, y, dy| {
        dy[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
        dy[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
        dy[2] = 3.0e7 * y[1] * y[1];
        Ok(())
    })
    .invariant("mass", |y| y[0] + y[1] + y[2]);

    let opts = VigilodeOptions::default()
        .strategy(SolverStrategy::Auto)
        .max_steps(500);

    let sol = VigilODE::solve(&rob, (0.0, 10.0), &[1.0, 0.0, 0.0], opts).expect("fallback solve");

    assert!(
        sol.success(),
        "solve must succeed via fail-closed protected fallback"
    );
    // Verify fallback was triggered
    let has_fallback = sol
        .route_history
        .iter()
        .any(|r| r.to_tier == "ProtectedRODAS5P");
    assert!(
        has_fallback,
        "route history must record fallback to ProtectedRODAS5P: {:?}",
        sol.route_history
    );
    assert!(
        sol.stats().counters.fallback_steps > 0,
        "fallback step counter must be recorded"
    );
}

#[test]
fn contract_5_physical_invariant_preservation() {
    let rob = robertson_problem();
    let opts = VigilodeOptions::default()
        .rtol(1e-8)
        .atol(1e-10)
        .invariant_drift_tol(1e-6);

    let sol = VigilODE::solve(&rob, (0.0, 40.0), &[1.0, 0.0, 0.0], opts).expect("solve");
    assert!(sol.success());

    let max_drift = sol.max_invariant_drift();
    assert!(
        max_drift < 1e-6,
        "mass conservation drift must be small, got {max_drift}"
    );

    let (inv_name, drift) = &sol.invariant_drift()[0];
    assert_eq!(inv_name, "mass");
    assert!(*drift < 1e-6);
}

#[test]
fn contract_6_complete_work_accounting_and_timing_authority() {
    let rob = robertson_problem();
    let sol =
        VigilODE::solve(&rob, (0.0, 10.0), &[1.0, 0.0, 0.0], SolverStrategy::Auto).expect("solve");

    assert_eq!(
        sol.timing_authority, "diagnostic_only",
        "must adhere to R2-STAT-01 timing authority"
    );
    assert!(
        sol.stats().counters.rhs_calls > 0,
        "RHS calls must be accounted"
    );
    assert!(
        sol.stats().counters.accepted_steps > 0,
        "accepted steps must be accounted"
    );
}
