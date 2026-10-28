use rodas5p_integrators::global_budget::{
    AllocationBlock, AllocationError, AllocationLimits, GlobalBudgetError, VerifiedGlobalBudget,
    allocate_global_work, certify_global_run, integrate_verified_residual_global,
    validate_work_allocation,
};
use rodas5p_integrators::{
    AdaptiveStepConfig, CollectiveModel, Pr101Model, VerifiedContinuousProblem,
    integrate_verified_residual,
};

fn owner(chi: f64) -> VerifiedContinuousProblem {
    let model =
        CollectiveModel::new(vec![0.0], vec![1.0], vec![(1.0, 0.0)], 0.0, chi, 0.0, 0.0).unwrap();
    VerifiedContinuousProblem::from_collective(model.to_bound_continuous_problem().unwrap())
}
fn config() -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: 1e-4,
        rtol: 1e-6,
        initial_step: 0.01,
        max_step: 0.01,
        max_attempts: 2000,
        ..Default::default()
    }
}

#[test]
fn bound_stationary_trajectory_covers_whole_curve_and_stored_centres() {
    let model = owner(0.0);
    let cfg = config();
    let result = integrate_verified_residual_global(
        &model,
        (0.0, 0.02),
        &[1.0, 0.0],
        1e-8,
        &[0.0, 0.005, 0.02],
        &cfg,
    )
    .unwrap();
    assert!(result.run().success, "{:?}", result.run().failure);
    let certificate = result.global().as_ref().unwrap();
    assert_eq!(certificate.span(), (0.0, 0.02));
    assert_eq!(certificate.dimension(), 2);
    assert_eq!(certificate.absolute_l2_budget(), cfg.atol);
    let s = certificate.snapshot();
    assert!(s.maximum_curve_radius >= 1e-8 && s.maximum_curve_radius <= cfg.atol);
    assert!(s.maximum_stored_query_radius <= cfg.atol);
    assert_eq!(s.segments, result.run().segments().len());
    assert_eq!(certificate.work().segment_commits, s.segments as u64);
    assert_eq!(certificate.work().query_checks, 2);
    assert_eq!(certificate.work().workspace_items, 8);
    assert!(result.run().solver_work.rhs_evaluations > 0);
}

#[test]
fn empty_span_preserves_initial_ball_and_unforgeable_model_identity() {
    let a = owner(0.0);
    let b = owner(0.0);
    let cfg = config();
    let run = integrate_verified_residual(&a, (0.0, 0.0), &[1.0, 0.0], 1e-8, &[0.0], &cfg).unwrap();
    let other =
        integrate_verified_residual(&b, (0.0, 0.0), &[1.0, 0.0], 1e-8, &[0.0], &cfg).unwrap();
    let c1 = certify_global_run(&run, cfg.atol).unwrap();
    let c2 = certify_global_run(&run, cfg.atol).unwrap();
    let c3 = certify_global_run(&other, cfg.atol).unwrap();
    let repeated =
        integrate_verified_residual(&a, (0.0, 0.0), &[1.0, 0.0], 1e-8, &[0.0], &cfg).unwrap();
    assert!(!c1.binds_run(&repeated));
    assert!(c1.same_model_as(&c2));
    assert!(c1.binds_run(&run));
    assert!(!c1.binds_run(&other));
    assert!(!c1.same_model_as(&c3));
    assert_eq!(c1.snapshot().endpoint_radius, 1e-8);
    assert_eq!(c1.snapshot().segments, 0);
    assert!(VerifiedGlobalBudget::new(&run, 1e-9).is_err());
    assert!(VerifiedGlobalBudget::new(&run, f64::NAN).is_err());
    assert!(matches!(
        VerifiedGlobalBudget::new(&run, f64::from_bits(cfg.atol.to_bits() + 1)),
        Err(GlobalBudgetError::Invalid(_))
    ));
    assert!(certify_global_run(&run, cfg.atol * 2.0).is_err());
}

#[test]
fn refusal_keeps_scientific_prefix_and_charges_repeated_attempts() {
    let model = owner(0.2);
    let cfg = config();
    let run = integrate_verified_residual(
        &model,
        (0.0, 0.01),
        &[1.0, 0.0],
        0.0,
        &[0.0, 0.005, 0.01],
        &cfg,
    )
    .unwrap();
    assert!(run.success, "{:?}", run.failure);
    let mut replay = VerifiedGlobalBudget::new(&run, f64::from_bits(1)).unwrap();
    let before = replay.snapshot();
    for attempts in 1..=2 {
        assert!(matches!(
            replay.advance(),
            Err(GlobalBudgetError::Exceeded { .. })
        ));
        let after = replay.snapshot();
        assert_eq!(after.time, before.time);
        assert_eq!(after.segments, before.segments);
        assert_eq!(
            after.endpoint_radius.to_bits(),
            before.endpoint_radius.to_bits()
        );
        assert_eq!(replay.work().segment_attempts, attempts);
        assert_eq!(replay.work().segment_commits, 0);
        assert_eq!(replay.work().refusals, attempts);
        assert!(replay.work().scalar_bound_operations > 0);
    }
    assert!(replay.finish().is_err());
    // The original approved run and all attempted work survive replay refusal.
    assert!(run.success);
    assert!(run.solver_work.rhs_evaluations > 0);
}

#[test]
fn incomplete_run_cannot_be_promoted_by_changing_public_telemetry() {
    let model =
        VerifiedContinuousProblem::from_pr101(Pr101Model::new(-1e6, 10.0).unwrap()).unwrap();
    let cfg = AdaptiveStepConfig {
        atol: 1e-11,
        rtol: 1e-8,
        initial_step: 1.0,
        max_step: 1.0,
        max_attempts: 1,
        ..Default::default()
    };
    let mut run =
        integrate_verified_residual(&model, (0.0, 1.0), &[0.0], 0.0, &[0.0, 1.0], &cfg).unwrap();
    assert!(!run.success);
    assert!(run.solver_work.rhs_evaluations > 0);
    run.success = true;
    run.failure = None; // Public diagnostics confer no capability.
    let refusal = certify_global_run(&run, cfg.atol).unwrap_err();
    assert!(matches!(
        refusal.error,
        GlobalBudgetError::Incomplete { .. }
    ));
}

#[test]
fn nonlinear_positive_gain_is_replayed_without_changing_solver_work() {
    let model = owner(0.2);
    let cfg = config();
    let result = integrate_verified_residual_global(
        &model,
        (0.0, 0.01),
        &[1.0, 0.0],
        0.0,
        &[0.0, 0.005, 0.01],
        &cfg,
    )
    .unwrap();
    assert!(result.run().success, "{:?}", result.run().failure);
    let c = result.global().as_ref().unwrap();
    assert!(c.work().exponential_enclosures > 0);
    assert!(c.snapshot().maximum_stored_query_radius <= cfg.atol);
    // One-mode chi=0.2 with unit initial modulus: exact a(t)=exp(i*chi*t).
    for output in result.run().outputs() {
        let theta = 0.2 * output.time();
        let e = ((output.center()[0] - theta.cos()).powi(2)
            + (output.center()[1] - theta.sin()).powi(2))
        .sqrt();
        assert!(e <= c.snapshot().maximum_stored_query_radius);
    }
    let direct = integrate_verified_residual(
        &model,
        (0.0, 0.01),
        &[1.0, 0.0],
        0.0,
        &[0.0, 0.005, 0.01],
        &cfg,
    )
    .unwrap();
    assert_eq!(result.run().solver_work, direct.solver_work);
    assert_eq!(result.run().attempts, direct.attempts);
}

#[test]
fn exact_integer_allocation_and_heuristic_seed_are_both_outward_validated() {
    let blocks = [
        AllocationBlock {
            error_coefficient: 1.0,
            step_cost_upper: 1.0,
        },
        AllocationBlock {
            error_coefficient: 9.0,
            step_cost_upper: 1.0,
        },
    ];
    let limits = AllocationLimits::default();
    let exact = validate_work_allocation(&blocks, 1, 0.5, &[8, 24], limits).unwrap();
    assert_eq!(exact.error_upper(), 0.5);
    assert_eq!(exact.work_upper(), 32.0);
    assert!(matches!(
        validate_work_allocation(&blocks, 1, 0.5, &[1, 1], limits),
        Err(AllocationError::Budget { .. })
    ));
    let planned = allocate_global_work(&blocks, 1, 0.5, limits).unwrap();
    assert!(planned.error_upper() <= 0.5);
    assert!(planned.work_upper() < 40.0);
    assert_eq!(planned.counts().len(), 2);
    // Separate 1/20+9/20 rounding exceeds0.5; equal-denominator grouping
    // recovers the exact upper bound (1+9)/20 without any budget slack.
    let uniform = validate_work_allocation(&blocks, 1, 0.5, &[20, 20], limits).unwrap();
    assert_eq!(uniform.error_upper(), 0.5);
    assert_eq!(uniform.work_upper(), 40.0);
}

#[test]
fn equal_denominator_grouping_preserves_outward_refusals_and_overflow_fallback() {
    let limits = AllocationLimits::default();
    let blocks = |a| {
        [
            AllocationBlock {
                error_coefficient: a,
                step_cost_upper: 2.0,
            },
            AllocationBlock {
                error_coefficient: a,
                step_cost_upper: 3.0,
            },
        ]
    };
    // Nearest binary64 2/3 lies strictly below the rational value. Grouping
    // cannot turn this into acceptance by suppressing the outward rounding.
    let lower = 2.0 / 3.0;
    assert!(matches!(
        validate_work_allocation(&blocks(1.0), 1, lower, &[3, 3], limits),
        Err(AllocationError::Budget { .. })
    ));
    let upper = f64::from_bits(lower.to_bits() + 1);
    let nonexact = validate_work_allocation(&blocks(1.0), 1, upper, &[3, 3], limits).unwrap();
    assert_eq!(nonexact.error_upper(), upper);
    assert_eq!(nonexact.work_upper(), 15.0);
    // The unscaled grouped numerator overflows, while the pointwise scaled
    // sum is representable. Preserve the original route's valid certificate.
    let large = validate_work_allocation(&blocks(f64::MAX), 1, f64::MAX, &[2, 2], limits).unwrap();
    assert_eq!(large.error_upper(), f64::MAX);
    assert_eq!(large.work_upper(), 10.0);
    // Subnormal division deliberately inflates uncertain rounding. Retain it.
    let tiny = f64::from_bits(1);
    assert!(matches!(
        validate_work_allocation(&blocks(tiny), 1, tiny, &[2, 2], limits),
        Err(AllocationError::Budget { .. })
    ));
    let sub = validate_work_allocation(&blocks(tiny), 1, 2.0 * tiny, &[2, 2], limits).unwrap();
    assert_eq!(sub.error_upper(), 2.0 * tiny);
    assert_eq!(sub.work_upper(), 10.0);
}

#[test]
fn allocator_checks_underflow_overflow_zero_blocks_and_resource_caps() {
    let tiny = f64::from_bits(1);
    let limits = AllocationLimits::default();
    let sub = allocate_global_work(
        &[AllocationBlock {
            error_coefficient: tiny,
            step_cost_upper: 1.0,
        }],
        1,
        tiny,
        limits,
    )
    .unwrap();
    assert!(sub.error_upper() <= tiny && sub.error_upper() > 0.0);
    let zero = allocate_global_work(
        &[AllocationBlock {
            error_coefficient: 0.0,
            step_cost_upper: 2.0,
        }],
        5,
        1e-12,
        limits,
    )
    .unwrap();
    assert_eq!(zero.counts(), &[1]);
    assert_eq!(zero.error_upper(), 0.0);
    assert_eq!(zero.work_upper(), 2.0);
    let impossible = [AllocationBlock {
        error_coefficient: f64::MAX,
        step_cost_upper: 1.0,
    }];
    assert!(allocate_global_work(&impossible, 1, tiny, limits).is_err());
    assert!(allocate_global_work(&impossible, 0, 1.0, limits).is_err());
    let bad_limits = AllocationLimits {
        max_steps_per_block: (1 << 53) + 1,
        ..limits
    };
    assert!(allocate_global_work(&impossible, 1, 1.0, bad_limits).is_err());
    let capped = AllocationLimits {
        max_steps_per_block: 3,
        max_total_steps: 3,
        max_refinements: 0,
    };
    assert!(
        allocate_global_work(
            &[AllocationBlock {
                error_coefficient: 1.0,
                step_cost_upper: 1.0
            }],
            1,
            1e-10,
            capped
        )
        .is_err()
    );
    assert!(
        validate_work_allocation(
            &[AllocationBlock {
                error_coefficient: 0.0,
                step_cost_upper: f64::MAX
            }],
            1,
            1.0,
            &[2],
            limits
        )
        .is_err()
    );
}

#[test]
fn gray_scott_factory_uses_the_same_sealed_global_route() {
    let (model, mut initial) =
        rodas5p_integrators::gray_scott_verified::gray_scott_2d_verified(2).unwrap();
    let cells = initial.len() / 2;
    initial[..cells].fill(1.0);
    initial[cells..].fill(0.0);
    let cfg = config();
    let result = integrate_verified_residual_global(
        &model,
        (0.0, 0.01),
        &initial,
        0.0,
        &[0.0, 0.005, 0.01],
        &cfg,
    )
    .unwrap();
    assert!(result.run().success, "{:?}", result.run().failure);
    let certificate = result.global().as_ref().unwrap();
    assert!(certificate.binds_run(result.run()));
    assert_eq!(certificate.dimension(), 8);
    assert!(certificate.snapshot().maximum_stored_query_radius <= cfg.atol);
    for output in result.run().outputs() {
        assert_eq!(output.center(), initial.as_slice());
    }
}
