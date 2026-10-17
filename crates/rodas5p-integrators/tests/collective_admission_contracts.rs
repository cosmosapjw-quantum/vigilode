use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};

use rodas5p_integrators::{
    CollectiveAdmissionError, CollectiveModel, OdeProblem, verify_collective_admission,
};

struct CountingAllocator;

static TRACK: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() && TRACK.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
    }
}

fn count_allocations<T>(operation: impl FnOnce() -> T) -> (T, u64, u64) {
    TRACK.store(false, Ordering::SeqCst);
    ALLOCATIONS.store(0, Ordering::Relaxed);
    ALLOCATED_BYTES.store(0, Ordering::Relaxed);
    TRACK.store(true, Ordering::SeqCst);
    let result = operation();
    TRACK.store(false, Ordering::SeqCst);
    (
        result,
        ALLOCATIONS.load(Ordering::Relaxed),
        ALLOCATED_BYTES.load(Ordering::Relaxed),
    )
}

fn sample_collective_model(n: usize, chi: f64, kappa: f64) -> CollectiveModel {
    let mut xi = Vec::with_capacity(n);
    let mut weights = Vec::with_capacity(n);
    let mut initial = Vec::with_capacity(n);

    let w = 1.0 / (n as f64);
    for j in 0..n {
        let detuning = if n == 1 {
            0.0
        } else {
            -1.0 + 2.0 * (j as f64) / ((n - 1) as f64)
        };
        xi.push(detuning);
        weights.push(w);
        // Normalized initial amplitude so sum w_j |z_j|^2 <= 1
        let phase = 0.5 * (j as f64);
        let r = 0.8;
        initial.push((r * phase.cos(), r * phase.sin()));
    }

    CollectiveModel::new(xi, weights, initial, 1.5, chi, 0.0, kappa)
        .expect("valid collective model")
}

#[test]
fn test_vg1_zero_allocation_in_rhs_and_jvp_hot_loops() {
    let model = sample_collective_model(8, 0.25, 0.0);
    let dim = model.dimension();
    let mut y = vec![0.0; dim];
    model.initial_state(&mut y).expect("initial state write");

    let mut dy = vec![0.0; dim];
    let v = vec![0.1; dim];
    let mut jv = vec![0.0; dim];

    // Warm up
    model.rhs(0.0, &y, &mut dy).expect("rhs call");
    model.jvp(0.0, &y, &v, &mut jv).expect("jvp call");

    // Zero-allocation verification for RHS
    let (_, allocs_rhs, _) = count_allocations(|| {
        for _ in 0..100 {
            model.rhs(0.0, &y, &mut dy).unwrap();
        }
    });
    assert_eq!(
        allocs_rhs, 0,
        "VG-1 violation: CollectiveModel::rhs allocated {allocs_rhs} times in hot loop"
    );

    // Zero-allocation verification for JVP
    let (_, allocs_jvp, _) = count_allocations(|| {
        for _ in 0..100 {
            model.jvp(0.0, &y, &v, &mut jv).unwrap();
        }
    });
    assert_eq!(
        allocs_jvp, 0,
        "VG-1 violation: CollectiveModel::jvp allocated {allocs_jvp} times in hot loop"
    );

    // Zero-allocation verification for energy and mean
    let (_, allocs_energy, _) = count_allocations(|| {
        for _ in 0..100 {
            let _ = model.energy(&y);
            let _ = model.mean(&y);
        }
    });
    assert_eq!(allocs_energy, 0, "VG-1 violation: energy/mean allocated");
}

#[test]
fn test_vg2_jvp_analytic_consistency_against_finite_difference() {
    let model = sample_collective_model(4, 0.5, 0.1);
    let dim = model.dimension();
    let mut y = vec![0.0; dim];
    model.initial_state(&mut y).unwrap();

    let v: Vec<f64> = (0..dim).map(|i| 0.1 * ((i as f64) * 0.7).sin()).collect();
    let mut jv_analytic = vec![0.0; dim];
    model.jvp(0.0, &y, &v, &mut jv_analytic).unwrap();

    // Central difference: (f(y + eps v) - f(y - eps v)) / (2 eps)
    let eps = 1e-7;
    let mut y_plus = vec![0.0; dim];
    let mut y_minus = vec![0.0; dim];
    for i in 0..dim {
        y_plus[i] = y[i] + eps * v[i];
        y_minus[i] = y[i] - eps * v[i];
    }

    let mut f_plus = vec![0.0; dim];
    let mut f_minus = vec![0.0; dim];
    model.rhs(0.0, &y_plus, &mut f_plus).unwrap();
    model.rhs(0.0, &y_minus, &mut f_minus).unwrap();

    for i in 0..dim {
        let fd = (f_plus[i] - f_minus[i]) / (2.0 * eps);
        let analytic = jv_analytic[i];
        let diff = (fd - analytic).abs();
        assert!(
            diff < 1e-6,
            "VG-2 JVP mismatch at index {i}: fd={fd}, analytic={analytic}, diff={diff}"
        );
    }
}

#[test]
fn test_vg2_dahlquist_linear_boundary_matches_exact_harmonic_oscillator() {
    // When chi = 0, kappa = 0: a_j'(t) = i * width * xi_j * a_j(t)
    // Exact solution: a_j(t) = exp(i * width * xi_j * t) * z_j
    let width = 2.0;
    let xi = vec![-0.75, 0.0, 0.5, 1.0];
    let weights = vec![0.25, 0.25, 0.25, 0.25];
    let initial = vec![(0.3, 0.4), (0.5, 0.0), (0.0, -0.6), (0.2, 0.2)];

    let model = CollectiveModel::new(xi.clone(), weights, initial.clone(), width, 0.0, 0.0, 0.0)
        .expect("valid model");

    let t = 0.5;
    let dim = model.dimension();
    let mut y = vec![0.0; dim];
    model.initial_state(&mut y).unwrap();

    // Compute Euler/Taylor step or evaluate RHS vector field against exact derivative
    let mut dy = vec![0.0; dim];
    model.rhs(t, &y, &mut dy).unwrap();

    for j in 0..model.num_modes() {
        let (u0, v0) = initial[j];
        let freq = width * xi[j];
        // d/dt (exp(i * freq * t) * (u0 + i v0)) at t=0
        // = i * freq * (u0 + i v0) = -freq * v0 + i * freq * u0
        let exact_du = -freq * v0;
        let exact_dv = freq * u0;

        let num_du = dy[2 * j];
        let num_dv = dy[2 * j + 1];

        assert!((num_du - exact_du).abs() < 1e-14);
        assert!((num_dv - exact_dv).abs() < 1e-14);
    }
}

#[test]
fn test_vg3_energy_invariant_conservation_derivative() {
    // Continuous energy conservation: dE/dt = sum w_j (2 u_j du_j + 2 v_j dv_j) = 0 when kappa = 0
    let model = sample_collective_model(6, 0.8, 0.0);
    let dim = model.dimension();
    let mut y = vec![0.0; dim];
    model.initial_state(&mut y).unwrap();

    let mut dy = vec![0.0; dim];
    model.rhs(0.0, &y, &mut dy).unwrap();

    let mut d_energy = 0.0;
    for j in 0..model.num_modes() {
        let w = model.weights()[j];
        let u = y[2 * j];
        let v = y[2 * j + 1];
        let du = dy[2 * j];
        let dv = dy[2 * j + 1];
        d_energy += w * (2.0 * u * du + 2.0 * v * dv);
    }

    assert!(
        d_energy.abs() < 1e-14,
        "VG-3 violation: continuous energy derivative dE/dt = {d_energy} != 0"
    );
}

#[test]
fn test_vg3_spectral_and_weighted_moments_preservation() {
    let xi = vec![-0.5, 0.5];
    let weights = vec![0.5, 0.5];
    let initial = vec![(0.3, 0.2), (-0.3, -0.2)]; // Anti-symmetric initial data

    let model = CollectiveModel::new(xi, weights, initial, 1.0, 0.1, 0.0, 0.0).unwrap();

    // mu_0 = 1.0
    assert!((model.spectral_moment(0) - 1.0).abs() < 1e-14);
    // Symmetric detunings with equal weights -> odd spectral moments vanish
    assert!(model.spectral_moment(1).abs() < 1e-14);
    assert!(model.spectral_moment(3).abs() < 1e-14);
    // Even spectral moment mu_2 = 0.5 * 0.25 + 0.5 * 0.25 = 0.25
    assert!((model.spectral_moment(2) - 0.25).abs() < 1e-14);

    // nu_0 = 0.5 * (0.3 + 0.2i) + 0.5 * (-0.3 - 0.2i) = 0
    let (nu0_re, nu0_im) = model.weighted_moment(0);
    assert!(nu0_re.abs() < 1e-14 && nu0_im.abs() < 1e-14);

    // nu_1 = 0.5 * (-0.5) * (0.3 + 0.2i) + 0.5 * (0.5) * (-0.3 - 0.2i)
    //      = -0.25 * (0.3 + 0.2i) - 0.25 * (0.3 + 0.2i) = -0.5 * (0.3 + 0.2i) = (-0.15, -0.10)
    let (nu1_re, nu1_im) = model.weighted_moment(1);
    assert!((nu1_re - (-0.15)).abs() < 1e-14);
    assert!((nu1_im - (-0.10)).abs() < 1e-14);
}

#[test]
fn test_negative_admission_gate_contract() {
    // 1. Empty modes
    let res = CollectiveModel::new(vec![], vec![], vec![], 1.0, 0.1, 0.0, 0.0);
    assert_eq!(res, Err(CollectiveAdmissionError::EmptyModes));

    // 2. Dimension mismatch
    let res = CollectiveModel::new(
        vec![0.0, 0.5],
        vec![1.0],
        vec![(0.1, 0.1)],
        1.0,
        0.1,
        0.0,
        0.0,
    );
    assert!(matches!(
        res,
        Err(CollectiveAdmissionError::DimensionMismatch { .. })
    ));

    // 3. Detuning out of range [-1, 1]
    let res = CollectiveModel::new(vec![1.5], vec![1.0], vec![(0.1, 0.1)], 1.0, 0.1, 0.0, 0.0);
    assert!(matches!(
        res,
        Err(CollectiveAdmissionError::DetuningOutOfRange { .. })
    ));

    // 4. Non-positive weight
    let res = CollectiveModel::new(vec![0.0], vec![0.0], vec![(0.1, 0.1)], 1.0, 0.1, 0.0, 0.0);
    assert!(matches!(
        res,
        Err(CollectiveAdmissionError::NonPositiveWeight { .. })
    ));

    // 5. Unnormalized weights
    let res = CollectiveModel::new(
        vec![0.0, 0.1],
        vec![0.6, 0.6],
        vec![(0.1, 0.1), (0.1, 0.1)],
        1.0,
        0.1,
        0.0,
        0.0,
    );
    assert!(matches!(
        res,
        Err(CollectiveAdmissionError::UnnormalizedWeights { .. })
    ));

    // 6. Energy exceeds unit ball
    let res = CollectiveModel::new(vec![0.0], vec![1.0], vec![(1.5, 0.0)], 1.0, 0.1, 0.0, 0.0);
    assert!(matches!(
        res,
        Err(CollectiveAdmissionError::EnergyExceedsUnitBall { .. })
    ));

    // 7. Non-finite parameter
    let res = CollectiveModel::new(
        vec![0.0],
        vec![1.0],
        vec![(0.1, 0.1)],
        f64::NAN,
        0.1,
        0.0,
        0.0,
    );
    assert!(matches!(res, Err(CollectiveAdmissionError::NonFinite(_))));

    // 8. Negative width
    let res = CollectiveModel::new(vec![0.0], vec![1.0], vec![(0.1, 0.1)], -1.0, 0.1, 0.0, 0.0);
    assert!(matches!(
        res,
        Err(CollectiveAdmissionError::NegativeWidth(_))
    ));

    // 9. Negative damping
    let res = CollectiveModel::new(vec![0.0], vec![1.0], vec![(0.1, 0.1)], 1.0, 0.1, 0.0, -0.5);
    assert!(matches!(
        res,
        Err(CollectiveAdmissionError::NegativeDamping(_))
    ));
}

#[test]
fn test_generic_ode_problem_rejection_by_collective_verifier() {
    let rhs = std::sync::Arc::new(|_t: f64, _y: &[f64], dy: &mut [f64]| {
        dy.fill(0.0);
        Ok(())
    });
    let jvp = std::sync::Arc::new(|_t: f64, _y: &[f64], _v: &[f64], jv: &mut [f64]| {
        jv.fill(0.0);
        Ok(())
    });
    let generic_problem = OdeProblem::new(
        String::from("generic-ode"),
        4,
        rhs,
        None,
        None,
        Some(jvp),
        None,
        true,
        None,
        None,
    )
    .unwrap();

    // Verifier without CollectiveModel binding must refuse with UnsupportedModel
    let res = verify_collective_admission(&generic_problem, None);
    assert!(matches!(
        res,
        Err(CollectiveAdmissionError::UnsupportedModel(_))
    ));

    // Valid CollectiveModel with matching dimension passes
    let model = sample_collective_model(2, 0.1, 0.0);
    let problem = model.to_ode_problem().expect("to_ode_problem succeeds");
    let res = verify_collective_admission(&problem, Some(&model));
    assert!(res.is_ok());
}
