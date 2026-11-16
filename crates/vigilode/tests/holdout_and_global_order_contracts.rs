//! Contracts for Node P9: Holdout Cases and Global Order Contracts (P9_HOLDOUT_AND_ORDER).
//!
//! Section 7 requirements:
//! 1. `stiff_nonlinear_robertson`: mass conservation < 1e-10, positive, finite, reference match, accounting complete.
//! 2. `oscillatory_prothero_robinson`: global error across h, h/2, h/4, stiff order reduction ~ 3-4, direct LU reference match.
//! 3. `nonnormal_linear_or_nonlinear`: Section 6 nonnormal problem, transient amplification, observer integration.
//! 4. `changing_stiffness_reentrant`: reentrant stiffness oscillation, cos(t) exact, hybrid controller dwell/anti-chattering.
//! 5. `global_order_smooth_nonstiff`: convergence slope >= 4.8 on smooth non-stiff problem.
//! 6. `vg1_zero_allocation`: 0 allocations in hot-loop ODE step integration.

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    f64::consts::PI,
};

use rodas5p_core::{
    LinearSolverConfig, WorkCounters,
    nonnormal_observers::{FixedHLogNormObserver, ResolventNumericalAbscissaObserver},
};
use rodas5p_integrators::{
    Dopri5Workspace, OdeProblem, oscillatory_prothero_robinson_problem, rodas5p_fast_step,
    sequential_step,
};
use vigilode::{
    Atol, CallbackError, DerivativeFallback, Jacobian, Options, Problem, Rodas5p, SwitchingConfig,
    SwitchingSolver,
    hybrid_automaton::{
        AntiChatteringPolicy, HybridRouteAutomaton, RouteEpochTracker, RouteFsmEvent,
        RouteFsmState, SpeculativeWorkTracker,
    },
};

// ---------------------------------------------------------------------------
// Zero-Allocation Tracking (VG-1)
// ---------------------------------------------------------------------------

struct CountingAlloc;
thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static COUNT: Cell<usize> = const { Cell::new(0) };
}
fn count_allocation() {
    if TRACK.try_with(Cell::get).unwrap_or(false) {
        let _ = COUNT.try_with(|count| count.set(count.get() + 1));
    }
}
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count_allocation();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count_allocation();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count_allocation();
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: CountingAlloc = CountingAlloc;

fn allocation_scope<T>(f: impl FnOnce() -> T) -> (T, usize) {
    COUNT.with(|count| count.set(0));
    TRACK.with(|track| track.set(true));
    let result = f();
    TRACK.with(|track| track.set(false));
    (result, COUNT.with(Cell::get))
}

// ---------------------------------------------------------------------------
// Test 1: stiff_nonlinear_robertson
// ---------------------------------------------------------------------------

fn robertson_problem() -> Problem {
    Problem::new(3, |_t, y, dy| {
        dy[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
        dy[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
        dy[2] = 3.0e7 * y[1] * y[1];
        Ok::<(), CallbackError>(())
    })
    .jacobian(Jacobian::dense(|_t, y, jac| {
        jac[0] = -0.04;
        jac[1] = 1.0e4 * y[2];
        jac[2] = 1.0e4 * y[1];
        jac[3] = 0.04;
        jac[4] = -1.0e4 * y[2] - 6.0e7 * y[1];
        jac[5] = -1.0e4 * y[1];
        jac[6] = 0.0;
        jac[7] = 6.0e7 * y[1];
        jac[8] = 0.0;
        Ok(())
    }))
    .autonomous()
}

#[test]
fn stiff_nonlinear_robertson() {
    let problem = robertson_problem();
    let opts = Options {
        rtol: 1e-6,
        atol: Atol::Scalar(1e-10),
        h0: 1e-4,
        max_steps: 50_000,
        ..Default::default()
    };

    // 1. Solve with protected Rodas5p driver
    let sol = Rodas5p::solve(&problem, (0.0, 40.0), &[1.0, 0.0, 0.0], &opts)
        .expect("Rodas5p solve should succeed on Robertson");
    assert!(sol.is_success(), "solution should be successful");
    let y = sol.final_state().expect("final state exists");

    // Invariant mass conservation: |y1 + y2 + y3 - 1.0| < 1e-10
    let mass = y[0] + y[1] + y[2];
    let mass_drift = (mass - 1.0).abs();
    assert!(
        mass_drift < 1e-10,
        "mass drift {mass_drift:e} exceeds tolerance 1e-10"
    );

    // Solution is finite and positive
    for (i, &val) in y.iter().enumerate() {
        assert!(val.is_finite(), "component y[{i}] is not finite");
        assert!(val >= 0.0, "component y[{i}] is negative: {val}");
    }

    // Matches high-accuracy reference at t = 40.0:
    // published reference: y0 = 0.7158270687, y1 = 9.185535e-6, y2 = 0.2841637457
    let reference = [0.7158270687, 9.185535e-6, 0.2841637457];
    for (i, &r) in reference.iter().enumerate() {
        let rel_err = (y[i] - r).abs() / r;
        assert!(
            rel_err < 1e-3,
            "y[{i}] = {} vs reference {r}: rel error {rel_err:e}",
            y[i]
        );
    }

    // 2. Solve with Switching / Hybrid solver
    let config = SwitchingConfig {
        fallback: DerivativeFallback::ProvidedOnly,
        window_count: 20,
        explicit_attempt_budget: 8,
        pressure_steps: 4,
        ..Default::default()
    };
    let switched = SwitchingSolver::solve(&problem, (0.0, 40.0), &[1.0, 0.0, 0.0], &opts, &config)
        .expect("SwitchingSolver solve should succeed");
    assert!(switched.success, "hybrid solve must succeed");
    assert!(
        switched.stats.accounting_complete,
        "stats.accounting_complete must be true"
    );

    let y_sw = &switched.final_state;
    let sw_drift = (y_sw[0] + y_sw[1] + y_sw[2] - 1.0).abs();
    assert!(
        sw_drift < 1e-10,
        "switched mass drift {sw_drift:e} exceeds tolerance 1e-10"
    );
    for (i, &val) in y_sw.iter().enumerate() {
        assert!(val.is_finite() && val >= 0.0);
        let rel = (val - reference[i]).abs() / reference[i];
        assert!(
            rel < 1e-2,
            "switched y[{i}] = {} vs ref {}: rel error {rel:e}",
            val,
            reference[i]
        );
    }

    // Verify SpeculativeWorkTracker accounting completeness
    let mut tracker = SpeculativeWorkTracker::new();
    tracker.record_committed(sol.stats.counters.into());
    assert!(tracker.accounting_complete());
}

// ---------------------------------------------------------------------------
// Test 2: oscillatory_prothero_robinson
// ---------------------------------------------------------------------------

fn prothero_robinson_problem(lambda: f64, omega: f64) -> Problem {
    Problem::new(1, move |t, y, dy| {
        dy[0] = lambda * (y[0] - (omega * t).sin()) + omega * (omega * t).cos();
        Ok(())
    })
    .jacobian(Jacobian::dense(move |_t, _y, jac| {
        jac[0] = lambda;
        Ok(())
    }))
    .time_derivative(move |t, _y, out| {
        out[0] = -lambda * omega * (omega * t).cos() - omega * omega * (omega * t).sin();
        Ok(())
    })
}

#[test]
fn oscillatory_prothero_robinson() {
    let lambda = -1.0e4;
    let omega = 10.0;
    let problem = prothero_robinson_problem(lambda, omega);
    let (ode, _) = oscillatory_prothero_robinson_problem(lambda, 0.0, omega, 0.0)
        .expect("oscillatory Prothero-Robinson problem creation should succeed");

    // Fixed-step integration across h, h/2, h/4 over [0, 1]
    let t_final = 1.0;
    let exact_final = (omega * t_final).sin();

    let step_sizes = [0.04, 0.02, 0.01];
    let mut errors = Vec::new();
    let atol = 1e-12;
    let rtol = 1e-10;
    let direct_cfg = LinearSolverConfig::default();

    for &h in &step_sizes {
        let n_steps = (t_final / h).round() as usize;
        let mut t = 0.0;
        let mut y = vec![0.0];
        let mut counters = WorkCounters::default();
        let mut seq_counters = WorkCounters::default();

        for _ in 0..n_steps {
            // Direct LU sequential reference step
            let seq_res = sequential_step(
                &ode,
                t,
                &y,
                h,
                &direct_cfg,
                None,
                atol,
                rtol,
                false,
                &mut seq_counters,
            )
            .expect("sequential step should succeed");

            // Fast step
            let (y_fast, _, _) = rodas5p_fast_step(&ode, t, &y, h, atol, rtol, &mut counters)
                .expect("fast step should succeed");

            // Verify bit-level / ultra-high precision match with direct LU reference
            let diff = (y_fast[0] - seq_res.y_new[0]).abs();
            assert!(
                diff < 1e-12,
                "fast step and direct LU reference should match within 1e-12, got diff {diff:e}"
            );

            y = y_fast;
            t += h;
        }

        let err = (y[0] - exact_final).abs();
        errors.push(err);
    }

    // Monotonicity: error must decrease strictly with smaller h
    assert!(
        errors[1] < errors[0],
        "error must decrease: e(h/2)={} vs e(h)={}",
        errors[1],
        errors[0]
    );
    assert!(
        errors[2] < errors[1],
        "error must decrease: e(h/4)={} vs e(h/2)={}",
        errors[2],
        errors[1]
    );

    // Convergence rate adhering to stiff order reduction (order ~ 3 to 4 under extreme stiffness)
    let order_1 = (errors[0] / errors[1]).log2();
    let order_2 = (errors[1] / errors[2]).log2();
    assert!(
        (2.8..=4.5).contains(&order_1),
        "stiff order 1 should be ~ 3-4, got {order_1:.2}"
    );
    assert!(
        (2.8..=4.5).contains(&order_2),
        "stiff order 2 should be ~ 3-4, got {order_2:.2}"
    );

    // Also verify adaptive tolerance regime with Rodas5p
    let mut adaptive_errors = Vec::new();
    for &tol in &[1e-4, 1e-6, 1e-8] {
        let opts = Options {
            rtol: tol,
            atol: Atol::Scalar(tol * 1e-3),
            h0: 1e-4,
            max_steps: 10_000,
            ..Default::default()
        };
        let sol = Rodas5p::solve(&problem, (0.0, t_final), &[0.0], &opts)
            .expect("adaptive solve should succeed");
        assert!(sol.is_success());
        let y_end = sol.final_state().unwrap()[0];
        let err = (y_end - exact_final).abs();
        adaptive_errors.push(err);
    }
    assert!(
        adaptive_errors[1] < adaptive_errors[0] && adaptive_errors[2] < adaptive_errors[1],
        "adaptive error should decrease monotonically with tolerance: {adaptive_errors:?}"
    );
}

// ---------------------------------------------------------------------------
// Test 3: nonnormal_linear_or_nonlinear
// ---------------------------------------------------------------------------

#[test]
fn nonnormal_linear_or_nonlinear() {
    // Section 6 nonnormal operator J = [[-1.0, 10.0], [0.0, -2.0]]
    let j_mat = [[-1.0, 10.0], [0.0, -2.0]];

    // 1. Nonnormal observer integration: verify transient amplification is flagged
    let mut observer = ResolventNumericalAbscissaObserver::new();
    let analysis = observer
        .analyze_2x2(&j_mat)
        .expect("observer analysis should succeed");

    assert!(
        analysis.has_transient_amplification,
        "transient amplification must be flagged"
    );
    assert!(
        analysis.spectral_abscissa < 0.0,
        "spectral abscissa must be negative (asymptotically stable)"
    );
    assert!(
        analysis.numerical_abscissa > 0.0,
        "numerical abscissa must be positive (finite-time transient growth)"
    );

    // Theoretical Euclidean numerical abscissa: (-3 + sqrt(101)) / 2 ≈ 3.5249378106
    let expected_mu2 = (-3.0 + 101.0_f64.sqrt()) / 2.0;
    assert!(
        (analysis.numerical_abscissa - expected_mu2).abs() < 1e-10,
        "mu_2 = {} vs expected {}",
        analysis.numerical_abscissa,
        expected_mu2
    );
    assert!(
        analysis.kreiss_lower_bound > 1.0,
        "Kreiss bound must exceed 1.0 due to transient growth"
    );

    // 2. Solve IVP y' = J y with y0 = [1.0, 1.0] over t in [0.0, 5.0]
    // Exact solution: y(t) = [exp(-t) + 10*(exp(-t) - exp(-2t)), exp(-2t)]
    let problem = Problem::new(2, |_t, y, dy| {
        dy[0] = -y[0] + 10.0 * y[1];
        dy[1] = -2.0 * y[1];
        Ok(())
    })
    .jacobian(Jacobian::dense(|_t, _y, jac| {
        jac[0] = -1.0;
        jac[1] = 10.0;
        jac[2] = 0.0;
        jac[3] = -2.0;
        Ok(())
    }))
    .autonomous();

    let opts = Options {
        rtol: 1e-8,
        atol: Atol::Scalar(1e-11),
        h0: 1e-3,
        dense_output: true,
        ..Default::default()
    };

    let sol =
        Rodas5p::solve(&problem, (0.0, 5.0), &[1.0, 1.0], &opts).expect("solve should succeed");
    assert!(sol.is_success());

    // Check transient amplification along trajectory: y[0] should grow well above y0[0] = 1.0
    let mut peak_y0 = 0.0;
    for step in 0..=50 {
        let t = step as f64 * 0.1;
        let y_t = sol.at(t).expect("dense output inside span");
        if y_t[0] > peak_y0 {
            peak_y0 = y_t[0];
        }

        // Compare against exact solution
        let exact_0 = (-t).exp() + 10.0 * ((-t).exp() - (-2.0 * t).exp());
        let exact_1 = (-2.0 * t).exp();
        let err_0 = (y_t[0] - exact_0).abs();
        let err_1 = (y_t[1] - exact_1).abs();
        assert!(
            err_0 < 1e-6,
            "t={t}: y[0] err {err_0:e} exceeds tolerance 1e-6"
        );
        assert!(
            err_1 < 1e-6,
            "t={t}: y[1] err {err_1:e} exceeds tolerance 1e-6"
        );
    }

    // Verify peak amplified state is indeed > 2.5
    assert!(
        peak_y0 > 2.5,
        "peak y[0] should show transient amplification > 2.5, got {peak_y0}"
    );

    // Also check y0 = [1.0, 0.0] pure eigenmode: y(t) = [exp(-t), 0.0]
    let sol_mode = Rodas5p::solve(&problem, (0.0, 2.0), &[1.0, 0.0], &opts)
        .expect("eigenmode solve should succeed");
    assert!(sol_mode.is_success());
    let y_end = sol_mode.final_state().unwrap();
    let exact_end_0 = (-2.0_f64).exp();
    assert!((y_end[0] - exact_end_0).abs() < 1e-6);
    assert!(y_end[1].abs() < 1e-10);
}

// ---------------------------------------------------------------------------
// Test 4: changing_stiffness_reentrant
// ---------------------------------------------------------------------------

#[test]
fn changing_stiffness_reentrant() {
    // Reentrant stiffness problem:
    // y' = -lam(t) * (y - cos(t)) - sin(t),
    // where lam(t) = 1.0 + 1.0e4 * (0.5 * (1.0 + (5.0 * t).sin())).
    // Stiffness smoothly oscillates between ~1 and ~10,000.
    // Exact solution: y(t) = cos(t).
    let lam = |t: f64| 1.0 + 1.0e4 * (0.5 * (1.0 + (5.0 * t).sin()));
    let lam_prime = |t: f64| 5.0e4 * 0.5 * (5.0 * t).cos();

    let problem = Problem::new(1, move |t, y, dy| {
        dy[0] = -lam(t) * (y[0] - t.cos()) - t.sin();
        Ok(())
    })
    .jacobian(Jacobian::dense(move |t, _y, jac| {
        jac[0] = -lam(t);
        Ok(())
    }))
    .time_derivative(move |t, y, out| {
        out[0] = -lam_prime(t) * (y[0] - t.cos()) - lam(t) * t.sin() - t.cos();
        Ok(())
    });

    let opts = Options {
        rtol: 1e-6,
        atol: Atol::Scalar(1e-9),
        h0: 1e-4,
        max_steps: 50_000,
        ..Default::default()
    };

    // 1. Solve with Rodas5p across t in [0.0, 2 * PI]
    let t_final = 2.0 * PI;
    let sol =
        Rodas5p::solve(&problem, (0.0, t_final), &[1.0], &opts).expect("solve should succeed");
    assert!(sol.is_success());

    let y_final = sol.final_state().unwrap()[0];
    let exact_final = t_final.cos(); // cos(2*pi) = 1.0
    let err = (y_final - exact_final).abs();
    assert!(
        err < 1e-5,
        "error vs exact solution {err:e} exceeds limit 1e-5"
    );

    // 2. Solve with Hybrid / Switching solver and verify no Zeno / chattering
    let config = SwitchingConfig {
        fallback: DerivativeFallback::ProvidedOnly,
        window_count: 32,
        explicit_attempt_budget: 16,
        pressure_steps: 8,
        implicit_residence: 3,
        probe_interval: 4,
        return_cost_ratio: 0.5,
        ..Default::default()
    };
    let switched = SwitchingSolver::solve(&problem, (0.0, t_final), &[1.0], &opts, &config)
        .expect("switching solve should succeed");
    assert!(switched.success);
    let sw_final = switched.final_state[0];
    let sw_err = (sw_final - exact_final).abs();
    assert!(sw_err < 1e-5, "hybrid error {sw_err:e} exceeds limit 1e-5");

    // Check that switches occurred adaptively and remain bounded (no chattering)
    assert!(
        switched.stats.switches <= 16,
        "switches {} should be bounded to prevent Zeno chattering",
        switched.stats.switches
    );

    // 3. Verify HybridRouteAutomaton dwell/hysteresis and anti-chattering policy
    let anti_chatter = AntiChatteringPolicy::default();
    assert_eq!(anti_chatter.min_dwell_batches, 3);
    assert!(!anti_chatter.can_switch_to_implicit(1, 0.9));
    assert!(!anti_chatter.can_switch_to_explicit(1, 0.2));
    assert!(anti_chatter.can_switch_to_implicit(3, 0.85));
    assert!(anti_chatter.can_switch_to_explicit(3, 0.35));

    let mut automaton = HybridRouteAutomaton::with_anti_chattering(anti_chatter);
    let mut epoch_tracker = RouteEpochTracker::new();

    // Trace cycle
    automaton.handle_event(RouteFsmEvent::StartSolve).unwrap();
    let ticket_0 = epoch_tracker.issue_ticket();
    assert!(epoch_tracker.is_evidence_valid(ticket_0));

    automaton
        .handle_event(RouteFsmEvent::PressureOrStiffnessDetected)
        .unwrap();
    automaton
        .handle_event(RouteFsmEvent::ScreenPassedWithEvidence)
        .unwrap();
    automaton
        .handle_event(RouteFsmEvent::PilotCompletedCheaper)
        .unwrap();
    assert_eq!(automaton.state(), RouteFsmState::ImplicitActive);

    // Epoch increments on route switch, invalidating earlier explicit ticket
    epoch_tracker.increment_epoch();
    assert!(!epoch_tracker.is_evidence_valid(ticket_0));

    // Dwell in implicit mode
    automaton
        .handle_event(RouteFsmEvent::ImplicitWindowAccepted)
        .unwrap();
    automaton
        .handle_event(RouteFsmEvent::ImplicitWindowAccepted)
        .unwrap();
    automaton
        .handle_event(RouteFsmEvent::ImplicitWindowAccepted)
        .unwrap();

    // Return sentinel due -> ExplicitReturnPilot -> ExplicitActive
    automaton
        .handle_event(RouteFsmEvent::ReturnSentinelDue)
        .unwrap();
    automaton
        .handle_event(RouteFsmEvent::ReturnPilotCheaper)
        .unwrap();
    assert_eq!(automaton.state(), RouteFsmState::ExplicitActive);
}

// ---------------------------------------------------------------------------
// Test 5: global_order_smooth_nonstiff
// ---------------------------------------------------------------------------

fn nonstiff_exact(t: f64) -> f64 {
    -(-t).exp() + t * t - 2.0 * t + 2.0
}

fn nonstiff_ode() -> OdeProblem {
    OdeProblem::new(
        "nonstiff-order",
        1,
        std::sync::Arc::new(|t, y, dy| {
            dy[0] = -y[0] + t * t;
            Ok(())
        }),
        None,
        Some(std::sync::Arc::new(|_t, _y| {
            rodas5p_core::DenseMatrix::new(1, 1, vec![-1.0])
        })),
        None,
        Some(std::sync::Arc::new(|t, _y, out| {
            out[0] = 2.0 * t;
            Ok(())
        })),
        false,
        None,
        Some(std::sync::Arc::new(|t| vec![nonstiff_exact(t)])),
    )
    .unwrap()
}

#[test]
fn global_order_smooth_nonstiff() {
    let ode = nonstiff_ode();
    let t_final: f64 = 1.0;

    let step_sizes: [f64; 3] = [0.2, 0.1, 0.05];
    let mut errors = Vec::new();

    for &h in &step_sizes {
        let n_steps = (t_final / h).round() as usize;
        let mut t = 0.0;
        let mut y = vec![1.0];
        let mut counters = WorkCounters::default();

        for _ in 0..n_steps {
            let (y_new, _, _) = rodas5p_fast_step(&ode, t, &y, h, 1e-14, 1e-12, &mut counters)
                .expect("fast step should succeed");
            y = y_new;
            t += h;
        }

        let err = (y[0] - nonstiff_exact(t_final)).abs();
        errors.push(err);
    }

    let slope_1 = (errors[0] / errors[1]).log2();
    let slope_2 = (errors[1] / errors[2]).log2();

    assert!(
        slope_1 >= 4.8,
        "convergence slope 1 must be >= 4.8 for 5th order method, got {slope_1:.3}"
    );
    assert!(
        slope_2 >= 4.8,
        "convergence slope 2 must be >= 4.8 for 5th order method, got {slope_2:.3}"
    );
}

// ---------------------------------------------------------------------------
// Test 6: vg1_zero_allocation
// ---------------------------------------------------------------------------

#[test]
fn vg1_zero_allocation() {
    // Verify that hot-loop step integration maintains exactly 0 allocations

    // 1. Nonnormal observer evaluation: stack arrays [[f64; 2]; 2]
    let j_mat = [[-1.0, 10.0], [0.0, -2.0]];
    let mut observer = ResolventNumericalAbscissaObserver::new();

    let (_res, obs_allocs) = allocation_scope(|| {
        for _ in 0..10 {
            let _ = observer.analyze_2x2(&j_mat).unwrap();
            let _ = FixedHLogNormObserver::compute_mu_euclidean_2x2(&j_mat);
        }
    });
    assert_eq!(
        obs_allocs, 0,
        "VG-1: Nonnormal observer evaluation must have 0 hot-loop allocations, got {obs_allocs}"
    );

    // 2. Hybrid route automaton state transitions and work tracking
    let mut automaton = HybridRouteAutomaton::new();
    let mut tracker = SpeculativeWorkTracker::new();
    let counters = WorkCounters {
        rhs_calls: 1,
        linear_solves: 1,
        ..Default::default()
    };

    let (_res, auto_allocs) = allocation_scope(|| {
        for _ in 0..10 {
            let _ = automaton.handle_event(RouteFsmEvent::PressureOrStiffnessDetected);
            let _ = automaton.handle_event(RouteFsmEvent::ScreenPassedWithEvidence);
            tracker.record_speculative(counters);
            tracker.commit_speculative();
            let _ = automaton.handle_event(RouteFsmEvent::PilotCompletedCheaper);
            let _ = automaton.handle_event(RouteFsmEvent::ImplicitWindowAccepted);
            let _ = automaton.handle_event(RouteFsmEvent::ReturnSentinelDue);
            let _ = automaton.handle_event(RouteFsmEvent::ReturnPilotCheaper);
        }
    });
    assert_eq!(
        auto_allocs, 0,
        "VG-1: Hybrid automaton transitions must have 0 hot-loop allocations, got {auto_allocs}"
    );

    // 3. DOPRI5 workspace step: non-stiff problem
    let mut dopri_work = Dopri5Workspace::new(1).unwrap();
    let mut y_dopri = [1.0];
    let mut y_next = [0.0];
    let mut error_buf = [0.0];
    let mut dopri_counters = WorkCounters::default();

    let (_res, dopri_allocs) = allocation_scope(|| {
        for _ in 0..10 {
            let _ = dopri_work.step_into(
                &|_t, y, dy| {
                    dy[0] = -y[0];
                    Ok(())
                },
                0.0,
                &y_dopri,
                0.01,
                &mut y_next,
                &mut error_buf,
                &mut dopri_counters,
            );
            y_dopri[0] = y_next[0];
        }
    });
    assert_eq!(
        dopri_allocs, 0,
        "VG-1: DOPRI5 step_into must have 0 hot-loop allocations, got {dopri_allocs}"
    );
}
