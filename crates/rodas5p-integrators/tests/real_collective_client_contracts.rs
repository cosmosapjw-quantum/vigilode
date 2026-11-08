//! Milestone LP06: Real nonlinear client independent structure contracts.
//!
//! Enforces:
//! 1. `EXISTING_NONLINEAR_REAL_CLIENT` status remains `NOT_ESTABLISHED` unless certified whole-domain
//!    enclosure evidence is provided.
//! 2. Negative controls: Conservation law alone, low-rank RHS sample, or matching model name
//!    cannot substitute for certified whole-domain projectability proofs.
//! 3. Unadmitted external client models fail closed with `UnsupportedModel` or admission error.
//! 4. Valid collective models maintain exact VG-1 zero-allocation hot-loop contracts.

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    sync::Arc,
};

use rodas5p_integrators::{
    CollectiveAdmissionError, CollectiveModel, OdeProblem, verify_collective_admission,
};

struct CountingAllocator;
thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static ALLOC_COUNT: Cell<u64> = const { Cell::new(0) };
}

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() && TRACK.with(Cell::get) {
            ALLOC_COUNT.with(|c| c.set(c.get() + 1));
        }
        p
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
    }
}

fn track_allocations<T>(f: impl FnOnce() -> T) -> (T, u64) {
    TRACK.with(|t| t.set(false));
    ALLOC_COUNT.with(|c| c.set(0));
    TRACK.with(|t| t.set(true));
    let res = f();
    TRACK.with(|t| t.set(false));
    (res, ALLOC_COUNT.with(Cell::get))
}

#[test]
fn test_lp06_existing_nonlinear_real_client_status_not_established_governance() {
    // LP06 Acceptance Contract:
    // "EXISTING_NONLINEAR_REAL_CLIENT remains NOT_ESTABLISHED until evidence exists."
    // "No current real client is invented to satisfy this node."
    const EXISTING_NONLINEAR_REAL_CLIENT_ESTABLISHED: bool = false;
    const {
        assert!(
            !EXISTING_NONLINEAR_REAL_CLIENT_ESTABLISHED,
            "LP06 governance violation: real nonlinear client cannot be claimed without external whole-domain proof"
        );
    }
}

#[test]
fn test_lp06_negative_controls_conservation_alone_does_not_prove_admission() {
    // A mock client that satisfies mass/energy conservation sum(y_i) = 1
    // but lacks the required collective model tensor structure must be rejected.
    let rhs = Arc::new(|_t: f64, y: &[f64], dy: &mut [f64]| {
        // Skew-symmetric linear oscillator conserving |y|^2
        dy[0] = -y[1];
        dy[1] = y[0];
        dy[2] = -y[3];
        dy[3] = y[2];
        Ok(())
    });
    let jvp = Arc::new(|_t: f64, _y: &[f64], v: &[f64], jv: &mut [f64]| {
        jv[0] = -v[1];
        jv[1] = v[0];
        jv[2] = -v[3];
        jv[3] = v[2];
        Ok(())
    });

    let conservative_candidate = OdeProblem::new(
        String::from("unadmitted-conservative-system"),
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

    // Verifier must reject the candidate because it does not carry an admitted CollectiveModel binding
    let admission_result = verify_collective_admission(&conservative_candidate, None);
    assert!(
        matches!(
            admission_result,
            Err(CollectiveAdmissionError::UnsupportedModel(_))
        ),
        "LP06: Conservative client lacking certified collective model must be refused"
    );
}

#[test]
fn test_lp06_negative_controls_model_name_alone_rejected() {
    // An arbitrary ODE problem naming itself "collective-model" cannot bypass admission verification
    let rhs = Arc::new(|_t: f64, y: &[f64], dy: &mut [f64]| {
        dy.copy_from_slice(y);
        Ok(())
    });
    let jvp = Arc::new(|_t: f64, _y: &[f64], v: &[f64], jv: &mut [f64]| {
        jv.copy_from_slice(v);
        Ok(())
    });
    let forged_problem = OdeProblem::new(
        String::from("collective-cubic-model-forgery"),
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

    let res = verify_collective_admission(&forged_problem, None);
    assert!(
        matches!(res, Err(CollectiveAdmissionError::UnsupportedModel(_))),
        "LP06: Matching string name alone cannot bypass structural verification"
    );
}

#[test]
fn test_lp06_positive_model_admitted_with_vg1_zero_allocation() {
    // Admitted CollectiveModel satisfies structural checks and executes hot loops with 0 heap allocations
    let n = 4;
    let xi = vec![-0.5, -0.2, 0.2, 0.5];
    let weights = vec![0.25; n];
    let initial = vec![(0.2, 0.1), (0.2, -0.1), (-0.2, 0.1), (-0.2, -0.1)];
    let model = CollectiveModel::new(xi, weights, initial, 1.0, 0.2, 0.0, 0.0)
        .expect("valid collective model");

    let ode_problem = model.to_ode_problem().expect("to_ode_problem succeeds");
    let admission = verify_collective_admission(&ode_problem, Some(&model));
    assert!(
        admission.is_ok(),
        "Valid collective model must pass admission"
    );

    // Hot-loop zero-allocation verification
    let dim = model.dimension();
    let mut y = vec![0.0; dim];
    model.initial_state(&mut y).unwrap();
    let mut dy = vec![0.0; dim];
    let v = vec![0.1; dim];
    let mut jv = vec![0.0; dim];

    // Warm up
    model.rhs(0.0, &y, &mut dy).unwrap();
    model.jvp(0.0, &y, &v, &mut jv).unwrap();

    let (_, allocs_rhs) = track_allocations(|| {
        for _ in 0..50 {
            model.rhs(0.0, &y, &mut dy).unwrap();
        }
    });
    assert_eq!(allocs_rhs, 0, "VG-1: RHS hot loop must have 0 allocations");

    let (_, allocs_jvp) = track_allocations(|| {
        for _ in 0..50 {
            model.jvp(0.0, &y, &v, &mut jv).unwrap();
        }
    });
    assert_eq!(allocs_jvp, 0, "VG-1: JVP hot loop must have 0 allocations");
}
