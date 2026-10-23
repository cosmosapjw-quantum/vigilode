use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
struct CountingAlloc;
thread_local! {
    static ENABLED: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ENABLED.with(|v| {
            if v.get() {
                ALLOCATIONS.with(|n| n.set(n.get() + 1));
            }
        });
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        ENABLED.with(|v| {
            if v.get() {
                ALLOCATIONS.with(|n| n.set(n.get() + 1));
            }
        });
        unsafe { System.realloc(ptr, layout, size) }
    }
}
#[global_allocator]
static GLOBAL: CountingAlloc = CountingAlloc;

use rodas5p_integrators::polynomial_resolvent::{
    integrate_exchange_implicit_euler, ExchangeCollisionProvider as Provider,
    ExchangeImplicitEulerWorkspace as TrajectoryWorkspace, PolynomialPhysicalNorm as Norm,
    PolynomialProviderWork as Work, PolynomialResolventError as Error,
    PolynomialResolventWorkspace as Workspace,
};

fn provider(a: f64, b: f64, blocks: usize, model: u64, epoch: u64) -> Provider {
    Provider::new(a, b, blocks, model, epoch).unwrap()
}

#[test]
fn stale_binding_refuses_before_actions_and_preserves_output() {
    let p = provider(2.0, 1.0, 2, 7, 11);
    let mut work = Work::default();
    let plan = p
        .prepare_implicit_euler(0.125, Norm::Infinity, 901, 1e-12, &mut work)
        .unwrap();
    let stale = [
        provider(2.0, 1.0, 2, 8, 11),
        provider(2.0, 1.0, 2, 7, 12),
        provider(1.0, 2.0, 2, 7, 11),
        provider(2.0, 1.0, 3, 7, 11),
    ];
    let mut out = [71.0; 4];
    for other in &stale {
        assert_eq!(
            plan.apply_provider_checked(
                other,
                0.125,
                Norm::Infinity,
                &[1.0; 4],
                &mut out,
                &mut Workspace::new(4),
                1e-12,
                1e-15,
                &mut work
            )
            .unwrap_err(),
            Error::WitnessMismatch
        );
        assert_eq!(out, [71.0; 4]);
    }
    for (step, norm) in [(0.25, Norm::Infinity), (0.125, Norm::Other(3))] {
        assert_eq!(
            plan.apply_provider_checked(
                &p,
                step,
                norm,
                &[1.0; 4],
                &mut out,
                &mut Workspace::new(4),
                1e-12,
                1e-15,
                &mut work
            )
            .unwrap_err(),
            Error::WitnessMismatch
        );
    }
    assert_eq!(work.operator_actions_attempted, 0);
    assert_eq!(work.applications_refused, 6);
    assert_eq!(work.setups_attempted, 1);
}

#[test]
fn invalid_provider_and_missing_evidence_fail_without_fabricating_witness() {
    for (a, b) in [
        (-1.0, 1.0),
        (f64::NAN, 1.0),
        (1.0, f64::INFINITY),
        (f64::MAX, f64::MAX),
    ] {
        assert!(Provider::new(a, b, 1, 1, 1).is_err());
    }
    assert!(Provider::new(1.0, 1.0, 0, 1, 1).is_err());
    let p = provider(1.0, 1.0, 1, 1, 1);
    let mut w = Work::default();
    assert!(p
        .prepare_implicit_euler(0.1, Norm::Infinity, 0, 1e-12, &mut w)
        .is_err());
    assert!(p
        .prepare_implicit_euler(0.1, Norm::Other(4), 1, 1e-12, &mut w)
        .is_err());
    assert!(p
        .prepare_implicit_euler(f64::MAX, Norm::Infinity, 1, 1e-12, &mut w)
        .is_err());
    assert_eq!((w.setups_attempted, w.setup_failures), (3, 3));
}

#[test]
fn full_space_exchange_relation_and_actual_stage_residual_use_two_actions() {
    for blocks in [1, 3, 17] {
        let p = provider(4.0, 1.0, blocks, 3, 5);
        let mut w = Work::default();
        let h = 0.25;
        let plan = p
            .prepare_implicit_euler(h, Norm::Infinity, 23, 1e-12, &mut w)
            .unwrap();
        assert_eq!(plan.degree(), 2);
        let rhs: Vec<f64> = (0..2 * blocks)
            .map(|i| if i % 2 == 0 { 0.75 } else { 0.25 })
            .collect();
        let mut out = vec![0.0; rhs.len()];
        let r = plan
            .apply_provider_checked(
                &p,
                h,
                Norm::Infinity,
                &rhs,
                &mut out,
                &mut Workspace::new(rhs.len()),
                1e-12,
                1e-15,
                &mut w,
            )
            .unwrap();
        assert!(r.residual_accepted);
        assert_eq!(
            (r.candidate_operator_actions, r.residual_operator_actions),
            (1, 1)
        );
        assert_eq!(w.operator_actions_attempted, 2);
        // Independent 2x2 inverse from determinant of I-hA.
        let det = (1.0 + 4.0 * h) * (1.0 + h) - 4.0 * h * h;
        let expected = [
            ((1.0 + h) * 0.75 + h * 0.25) / det,
            (4.0 * h * 0.75 + (1.0 + 4.0 * h) * 0.25) / det,
        ];
        for pair in out.chunks_exact(2) {
            for j in 0..2 {
                assert!((pair[j] - expected[j]).abs() < 1e-14);
            }
            assert!((pair[0] + pair[1] - 1.0).abs() < 1e-14);
            assert!(pair.iter().all(|x| *x >= 0.0));
        }
        let mut direct = vec![0.0; rhs.len()];
        p.direct_implicit_euler(h, &rhs, &mut direct, &mut w)
            .unwrap();
        for (x, y) in out.iter().zip(direct) {
            assert!((x - y).abs() < 1e-14);
        }
        assert_eq!(w.direct_blocks_solved, blocks as u64);
        assert_eq!(w.operator_actions_attempted, 2); // direct arithmetic is recorded separately
    }
}

#[test]
fn accepted_ie_trajectory_refines_and_preserves_number_and_positivity() {
    let p = provider(2.0, 1.0, 1, 1, 1);
    let exact = [
        1.0 / 3.0 + (2.0 / 3.0) * (-3.0_f64).exp(),
        2.0 / 3.0 - (2.0 / 3.0) * (-3.0_f64).exp(),
    ];
    let mut prior = f64::INFINITY;
    for steps in [16, 32, 64] {
        let h = 1.0 / steps as f64;
        let mut work = Work::default();
        let plan = p
            .prepare_implicit_euler(h, Norm::Infinity, 67, 1e-12, &mut work)
            .unwrap();
        let mut out = [99.0; 2];
        let r = integrate_exchange_implicit_euler(
            &p,
            &plan,
            h,
            steps,
            &[1.0, 0.0],
            &mut out,
            &mut TrajectoryWorkspace::new(2),
            1e-12,
            1e-15,
            &mut work,
        )
        .unwrap();
        assert_eq!(r.completed_steps, steps);
        assert_eq!(r.last_accepted_time, 1.0);
        assert!(r.failure.is_none());
        assert!(out.iter().all(|x| *x >= 0.0));
        assert!((out.iter().sum::<f64>() - 1.0).abs() < 2e-14);
        let e = (out[0] - exact[0]).abs().max((out[1] - exact[1]).abs());
        assert!(e < prior);
        if prior.is_finite() {
            assert!(prior / e > 1.8);
        }
        prior = e;
        assert_eq!(work.operator_actions_attempted, 2 * steps as u64);
        assert_eq!(work.stages_accepted, steps as u64);
    }
}

#[test]
fn stage_refusal_preserves_accepted_prefix_and_all_attempted_work() {
    let p = provider(2.0, 1.0, 1, 1, 1);
    let mut w = Work::default();
    let plan = p
        .prepare_implicit_euler(0.125, Norm::Infinity, 1, 1e-12, &mut w)
        .unwrap();
    let mut out = [0.0; 2];
    // A known residual-evaluation allowance larger than the target must refuse,
    // even when the symbolic full-space relation has zero defect.
    let r = integrate_exchange_implicit_euler(
        &p,
        &plan,
        0.125,
        8,
        &[1.0, 0.0],
        &mut out,
        &mut TrajectoryWorkspace::new(2),
        1e-12,
        1e-6,
        &mut w,
    )
    .unwrap();
    assert_eq!(r.completed_steps, 0);
    assert!(r.failure.is_some());
    assert_eq!(out, [1.0, 0.0]);
    assert_eq!(w.operator_actions_attempted, 2);
    assert_eq!(w.stages_attempted, 1);
    assert_eq!(w.stages_accepted, 0);
}

#[test]
fn rounded_polynomial_coefficients_get_full_space_defect_and_failed_actions_remain_charged() {
    let p = provider(0.1, 0.2, 3, 1, 1);
    let mut w = Work::default();
    let plan = p
        .prepare_implicit_euler(0.3, Norm::Infinity, 51, 1e-12, &mut w)
        .unwrap();
    assert!(plan.relation_defect_ratio() > 0.0);
    let p = provider(f64::MAX, 0.0, 1, 1, 1);
    let h = f64::MIN_POSITIVE;
    let plan = p
        .prepare_implicit_euler(h, Norm::Infinity, 52, 1e-12, &mut w)
        .unwrap();
    let before = w.operator_actions_attempted;
    let mut out = [4.0, 5.0];
    assert_eq!(
        plan.apply_provider_checked(
            &p,
            h,
            Norm::Infinity,
            &[f64::MAX, 0.0],
            &mut out,
            &mut Workspace::new(2),
            1e-12,
            1e-15,
            &mut w
        )
        .unwrap_err(),
        Error::NonFinite
    );
    assert_eq!(out, [4.0, 5.0]);
    assert_eq!(w.operator_actions_attempted, before + 1);
    assert_eq!(
        p.direct_implicit_euler(h, &[f64::MAX, 0.0], &mut out, &mut w)
            .unwrap_err(),
        Error::NonFinite
    );
    assert_eq!(out, [4.0, 5.0]);
    assert_eq!(w.direct_block_formula_evaluations, 1);
    assert_eq!(w.direct_blocks_solved, 0);
    assert_eq!(w.direct_solves_failed, 1);
}

#[test]
fn every_accepted_ie_checkpoint_has_the_discrete_analytic_solution() {
    let p = provider(2.0, 1.0, 1, 1, 1);
    let h = 0.0625;
    let mut w = Work::default();
    let plan = p
        .prepare_implicit_euler(h, Norm::Infinity, 71, 1e-12, &mut w)
        .unwrap();
    let mut state = [1.0, 0.0];
    let mut next = [0.0; 2];
    let mut workspace = Workspace::new(2);
    for k in 1..=16 {
        let r = plan
            .apply_provider_checked(
                &p,
                h,
                Norm::Infinity,
                &state,
                &mut next,
                &mut workspace,
                1e-12,
                1e-15,
                &mut w,
            )
            .unwrap();
        assert!(r.residual_accepted);
        let u = 1.0 / 3.0 + (2.0 / 3.0) * (1.0 + 3.0 * h).powi(-k);
        assert!((next[0] - u).abs() < 2e-14);
        assert!((next[0] + next[1] - 1.0).abs() < 2e-14);
        assert!(next.iter().all(|x| *x >= 0.0));
        state = next;
    }
}

#[test]
fn provider_stages_trajectory_and_stale_refusal_allocate_nothing_after_setup() {
    let p = provider(2.0, 1.0, 2, 1, 1);
    let mut work = Work::default();
    let plan = p
        .prepare_implicit_euler(0.125, Norm::Infinity, 1, 1e-12, &mut work)
        .unwrap();
    let mut state = [0.0; 4];
    let mut ws = TrajectoryWorkspace::new(4);
    let mut stage_ws = Workspace::new(4);
    ALLOCATIONS.with(|n| n.set(0));
    ENABLED.with(|v| v.set(true));
    let trajectory = integrate_exchange_implicit_euler(
        &p,
        &plan,
        0.125,
        8,
        &[1.0, 0.0, 0.0, 1.0],
        &mut state,
        &mut ws,
        1e-12,
        1e-15,
        &mut work,
    );
    let refused = plan.apply_provider_checked(
        &p,
        0.25,
        Norm::Infinity,
        &[1.0; 4],
        &mut state,
        &mut stage_ws,
        1e-12,
        1e-15,
        &mut work,
    );
    ENABLED.with(|v| v.set(false));
    assert_eq!(ALLOCATIONS.with(Cell::get), 0);
    assert!(trajectory.unwrap().failure.is_none());
    assert_eq!(refused.unwrap_err(), Error::WitnessMismatch);
}
