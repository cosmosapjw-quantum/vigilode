use rodas5p_integrators::physical_error_budget::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct CountingAlloc;
thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static ALLOCS: Cell<usize> = const { Cell::new(0) };
}
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        COUNTING.with(|enabled| {
            if enabled.get() {
                ALLOCS.with(|n| n.set(n.get() + 1));
            }
        });
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        COUNTING.with(|enabled| {
            if enabled.get() {
                ALLOCS.with(|n| n.set(n.get() + 1));
            }
        });
        unsafe { System.realloc(ptr, layout, size) }
    }
}
#[global_allocator]
static ALLOCATOR: CountingAlloc = CountingAlloc;
const ID: BudgetIdentity = BudgetIdentity {
    physical_norm: 41,
    model_epoch: 2,
    output_horizon: 9,
};
fn provenance() -> BoundProvenance {
    BoundProvenance::new(101).unwrap()
}
fn bounded(value: f64) -> ErrorMagnitude {
    ErrorMagnitude::bounded(value, provenance()).unwrap()
}
fn gain(value: f64) -> Amplification {
    Amplification::bounded(value, provenance()).unwrap()
}
fn charge(id: u64, component: ErrorComponent, value: f64) -> Contribution {
    Contribution {
        perturbation_id: id,
        component,
        magnitude: bounded(value),
        amplification: gain(1.0),
    }
}

#[test]
fn invalid_caps_amplification_and_missing_provenance_fail_closed() {
    for value in [0.0, -1.0, f64::INFINITY, f64::NAN] {
        assert_eq!(
            Amplification::bounded(value, provenance()),
            Err(BudgetError::InvalidAmplification)
        );
    }
    for value in [-1.0, f64::INFINITY, f64::NAN] {
        assert!(PhysicalErrorBudget::<8>::new(ID, value, [1.0; 6]).is_err());
        assert!(PhysicalErrorBudget::<8>::new(ID, 1.0, [value; 6]).is_err());
    }
    assert_eq!(
        BoundProvenance::new(0),
        Err(BudgetError::MissingBoundProvenance)
    );
    assert_eq!(bounded(1.0).provenance().unwrap().reference(), 101);
    // A downward local allowance must not exceed the real 1/3.
    let local = gain(3.0).local_allowance(bounded(1.0)).unwrap();
    let mut budget = PhysicalErrorBudget::<1>::new(ID, 1.0, [1.0; 6]).unwrap();
    budget
        .commit(
            ID,
            &[Contribution {
                magnitude: local,
                amplification: gain(3.0),
                ..charge(1, ErrorComponent::FlowOrRoot, 0.0)
            }],
        )
        .unwrap();
    assert!(budget.total_spent().unwrap() <= 1.0);
}

#[test]
fn duplicate_overspend_identity_and_overflow_are_transactional() {
    let mut budget = PhysicalErrorBudget::<8>::new(ID, 1.0, [0.5; 6]).unwrap();
    budget
        .commit(ID, &[charge(1, ErrorComponent::TimeResidual, 0.25)])
        .unwrap();
    let before = budget.clone();
    assert_eq!(
        budget.commit(ID, &[charge(1, ErrorComponent::FlowOrRoot, 0.1)]),
        Err(BudgetError::DuplicatePerturbation)
    );
    assert_eq!(
        budget.commit(
            ID,
            &[
                charge(2, ErrorComponent::FlowOrRoot, 0.1),
                charge(2, ErrorComponent::SpatialDefect, 0.1)
            ]
        ),
        Err(BudgetError::DuplicatePerturbation)
    );
    assert_eq!(
        budget.commit(ID, &[charge(3, ErrorComponent::TimeResidual, 0.5)]),
        Err(BudgetError::ComponentOverspent)
    );
    assert_eq!(
        budget.commit(
            ID,
            &[
                charge(4, ErrorComponent::SpatialDefect, 0.5),
                charge(5, ErrorComponent::Roundoff, 0.5)
            ]
        ),
        Err(BudgetError::TotalOverspent)
    );
    assert_eq!(
        budget.commit(
            BudgetIdentity {
                model_epoch: 3,
                ..ID
            },
            &[]
        ),
        Err(BudgetError::IdentityMismatch)
    );
    assert_eq!(
        budget.commit(
            ID,
            &[Contribution {
                amplification: gain(f64::MAX),
                ..charge(6, ErrorComponent::FlowOrRoot, 2.0)
            }]
        ),
        Err(BudgetError::ArithmeticOverflow)
    );
    assert_eq!(budget, before);
}

#[test]
fn sampled_residual_or_estimated_growth_cannot_acquire_bounded_status() {
    for estimated_gain in [false, true] {
        let mut budget = PhysicalErrorBudget::<8>::new(ID, 1.0, [1.0; 6]).unwrap();
        let mut candidate = charge(1, ErrorComponent::TimeResidual, 0.125);
        if estimated_gain {
            candidate.amplification = Amplification::estimated(1.0).unwrap();
        } else {
            candidate.magnitude = ErrorMagnitude::sampled_residual(0.125).unwrap();
        }
        budget.commit(ID, &[candidate]).unwrap();
        budget
            .commit(ID, &[charge(2, ErrorComponent::FlowOrRoot, 0.125)])
            .unwrap();
        assert_eq!(budget.status(), EvidenceStatus::Estimated);
    }
}

#[test]
fn remap_order_changes_only_old_space_flow_charge() {
    let mut after = PhysicalErrorBudget::<4>::new(ID, 16.0, [16.0; 6]).unwrap();
    let mut before = after.clone();
    after
        .commit(
            ID,
            &remap_flow_contributions(
                1,
                bounded(0.25),
                2,
                bounded(0.5),
                gain(2.0),
                JumpOrder::RemapThenFlow,
            )
            .unwrap(),
        )
        .unwrap();
    before
        .commit(
            ID,
            &remap_flow_contributions(
                1,
                bounded(0.25),
                2,
                bounded(0.5),
                gain(2.0),
                JumpOrder::FlowThenRemap {
                    remap_norm: bounded(4.0),
                },
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(after.total_spent().unwrap(), 1.5);
    assert_eq!(before.total_spent().unwrap(), 4.5);
    assert_eq!(after.status(), EvidenceStatus::Bounded);
    assert_eq!(before.status(), EvidenceStatus::Estimated);
    assert_eq!(after.component_spent()[4], before.component_spent()[4]);
    // Signed, mass-zero remap consistency still incurs physical state error.
    let defect = [0.125_f64, -0.125];
    assert_eq!(defect.iter().sum::<f64>(), 0.0);
    assert_eq!(defect.iter().map(|x| x.abs()).sum::<f64>(), 0.25);
}

#[test]
fn accounting_has_zero_allocations_on_admission_and_refusal() {
    let mut budget = PhysicalErrorBudget::<8>::new(ID, 1.0, [1.0; 6]).unwrap();
    let candidate = [charge(1, ErrorComponent::TimeResidual, 0.25)];
    ALLOCS.with(|n| n.set(0));
    COUNTING.with(|on| on.set(true));
    let accepted = budget.commit(ID, &candidate);
    let refused = budget.commit(ID, &candidate);
    let total = budget.total_spent();
    COUNTING.with(|on| on.set(false));
    assert_eq!(ALLOCS.with(Cell::get), 0);
    assert!(accepted.is_ok());
    assert_eq!(refused, Err(BudgetError::DuplicatePerturbation));
    assert_eq!(total.unwrap(), 0.25);
}

#[test]
fn three_temporal_refinements_enclose_exact_reference_and_independent_jumps() {
    // y'=t, y(0)=0, T=1. Left Euler's continuous linear reconstruction
    // has residual -(t-t_n), exact integral h^2/2; mu=0, G=1.
    // Binary dyadic steps and reference y(1)=1/2 are exact: uncertainty=0.
    let mut previous = f64::INFINITY;
    for n in [4_u64, 8, 16] {
        let h = 1.0 / n as f64;
        let cap = h / 2.0 + 0.0625;
        let mut ledger = PhysicalErrorBudget::<32>::new(ID, cap, [cap; 6]).unwrap();
        let mut numerical = 0.0;
        for i in 0..n {
            numerical += h * (i as f64 * h);
            ledger
                .commit(ID, &[charge(i, ErrorComponent::TimeResidual, h * h / 2.0)])
                .unwrap();
        }
        // Independent negative endpoint perturbation after remap keeps bound sharp.
        numerical -= 0.03125;
        numerical -= 0.03125;
        ledger
            .commit(
                ID,
                &remap_flow_contributions(
                    n,
                    bounded(0.03125),
                    n + 1,
                    bounded(0.03125),
                    gain(1.0),
                    JumpOrder::RemapThenFlow,
                )
                .unwrap(),
            )
            .unwrap();
        let error = (numerical - 0.5).abs();
        assert!(error <= ledger.total_spent().unwrap());
        assert_eq!(error, cap);
        assert!(h / 2.0 < previous);
        previous = h / 2.0;
        assert_eq!(ledger.status(), EvidenceStatus::Bounded);
    }
}

#[test]
fn damped_rotation_growth_is_frequency_independent_and_nonnormal_growth_is_not_damping() {
    // A=-kappa I+omega R has symmetric part -kappa I. The analytic
    // amplification exp(-kappa*t) follows without an omega-dependent norm.
    let kappa = 0.5_f64;
    let t = 0.25_f64;
    let enclosure = rodas5p_core::directed::exp_interval(-kappa * t).unwrap();
    for omega in [0.0_f64, 1.0, 1000.0] {
        let angle = omega * t;
        let norm = (enclosure.lo * angle.cos()).hypot(enclosure.lo * angle.sin());
        assert!(norm <= enclosure.hi + 2.0 * f64::EPSILON);
    }
    // Eigenvalues both -1 do not imply mu<=0 for [[-1,16],[0,-1]].
    // exp(tA)*(0,1)=exp(-t)*(16t,1), which grows at t=1/8.
    let t = 0.125_f64;
    let observed = (-t).exp() * (16.0 * t).hypot(1.0);
    assert!(observed > 1.0);
    let continuous_upper = rodas5p_core::directed::exp_interval(7.0 * t).unwrap().hi;
    assert!(observed <= continuous_upper);
    let mut budget = PhysicalErrorBudget::<1>::new(ID, 4.0, [4.0; 6]).unwrap();
    budget
        .commit(
            ID,
            &[Contribution {
                amplification: gain(continuous_upper),
                ..charge(1, ErrorComponent::Initial, 1.0)
            }],
        )
        .unwrap();
    assert!(observed <= budget.total_spent().unwrap());
}

#[test]
fn positive_growth_requires_segment_supremum_not_endpoint_gain() {
    // y'=y+1, v=0 on [0,1], x(0)=0: residual magnitude 1, integral 1.
    // Actual endpoint error e-1 exceeds endpoint Gamma(1,1)*integral = 1.
    let error = 1.0_f64.exp_m1();
    assert!(error > 1.0);
    let segment_sup = rodas5p_core::directed::exp_interval(1.0).unwrap().hi;
    let mut ledger = PhysicalErrorBudget::<1>::new(ID, 3.0, [3.0; 6]).unwrap();
    ledger
        .commit(
            ID,
            &[Contribution {
                amplification: gain(segment_sup),
                ..charge(1, ErrorComponent::TimeResidual, 1.0)
            }],
        )
        .unwrap();
    assert!(error <= ledger.total_spent().unwrap());
}

fn binding(trial_id: u64) -> PhysicalCandidateBinding {
    PhysicalCandidateBinding {
        identity: ID,
        trial_id,
        reconstruction_id: 77,
    }
}

#[test]
fn candidate_refusals_are_atomic_and_provider_work_is_never_rolled_back() {
    let mut budget = PhysicalErrorBudget::<8>::new(ID, 1.0, [1.0; 6]).unwrap();
    let mut work = PhysicalAdmissionWork::default();
    let mut state = [0.0];
    let next = [0.125];
    let mut time = 0.0;
    let mut scratch = [charge(1, ErrorComponent::TimeResidual, 0.125)];
    let mut calls = 0;
    let mut good = |_: &PhysicalCandidateContext<'_>, _: &mut [Contribution; 1]| {
        calls += 1;
        Ok(1)
    };
    assert!(matches!(
        prepare_physical_candidate(
            &mut budget,
            &mut work,
            binding(1),
            &mut time,
            &mut state,
            0.5,
            &next,
            false,
            PhysicalAdmissionMode::BoundedRequired,
            &mut scratch,
            &mut good
        )
        .unwrap(),
        PhysicalCandidatePreparation::LocalRejected
    ));
    let bad = PhysicalCandidateBinding {
        identity: BudgetIdentity {
            model_epoch: 3,
            ..ID
        },
        ..binding(1)
    };
    assert_eq!(
        prepare_physical_candidate(
            &mut budget,
            &mut work,
            bad,
            &mut time,
            &mut state,
            0.5,
            &next,
            true,
            PhysicalAdmissionMode::BoundedRequired,
            &mut scratch,
            &mut good
        )
        .unwrap_err(),
        StopOnPhysicalRefusal::Budget(BudgetError::IdentityMismatch)
    );
    assert_eq!(calls, 0);
    assert_eq!(work.provider_attempts, 0);
    let before = budget.clone();
    let mut missing = |_: &PhysicalCandidateContext<'_>, _: &mut [Contribution; 1]| {
        Err(PhysicalBoundsError::MissingBound)
    };
    assert_eq!(
        prepare_physical_candidate(
            &mut budget,
            &mut work,
            binding(1),
            &mut time,
            &mut state,
            0.5,
            &next,
            true,
            PhysicalAdmissionMode::BoundedRequired,
            &mut scratch,
            &mut missing
        )
        .unwrap_err(),
        StopOnPhysicalRefusal::Provider(PhysicalBoundsError::MissingBound)
    );
    assert_eq!(budget, before);
    assert_eq!(work.provider_attempts, 1);
    assert_eq!(work.provider_completed, 0);
    let mut complete = |_: &PhysicalCandidateContext<'_>, _: &mut [Contribution; 1]| Ok(1);
    scratch[0].magnitude = ErrorMagnitude::sampled_residual(0.125).unwrap();
    assert_eq!(
        prepare_physical_candidate(
            &mut budget,
            &mut work,
            binding(1),
            &mut time,
            &mut state,
            0.5,
            &next,
            true,
            PhysicalAdmissionMode::BoundedRequired,
            &mut scratch,
            &mut complete
        )
        .unwrap_err(),
        StopOnPhysicalRefusal::BoundedEvidenceRequired
    );
    assert_eq!(budget, before);
    assert_eq!(state, [0.0]);
    assert_eq!(time, 0.0);
    match prepare_physical_candidate(
        &mut budget,
        &mut work,
        binding(1),
        &mut time,
        &mut state,
        0.5,
        &next,
        true,
        PhysicalAdmissionMode::Operational,
        &mut scratch,
        &mut complete,
    )
    .unwrap()
    {
        PhysicalCandidatePreparation::Prepared(token) => {
            assert_eq!(token.status(), EvidenceStatus::Estimated);
            token.commit();
        }
        _ => panic!("expected prepared transaction"),
    }
    let accepted = budget.clone();
    let later = [0.25];
    assert_eq!(
        prepare_physical_candidate(
            &mut budget,
            &mut work,
            binding(2),
            &mut time,
            &mut state,
            1.0,
            &later,
            true,
            PhysicalAdmissionMode::Operational,
            &mut scratch,
            &mut complete
        )
        .unwrap_err(),
        StopOnPhysicalRefusal::Budget(BudgetError::DuplicatePerturbation)
    );
    assert_eq!(budget, accepted);
    assert_eq!(state, next);
    assert_eq!(time, 0.5);
    scratch[0] = charge(2, ErrorComponent::FlowOrRoot, 2.0);
    assert_eq!(
        prepare_physical_candidate(
            &mut budget,
            &mut work,
            binding(2),
            &mut time,
            &mut state,
            1.0,
            &later,
            true,
            PhysicalAdmissionMode::Operational,
            &mut scratch,
            &mut complete
        )
        .unwrap_err(),
        StopOnPhysicalRefusal::Budget(BudgetError::ComponentOverspent)
    );
    assert_eq!(budget, accepted);
    assert_eq!(state, next);
    assert_eq!(time, 0.5);
    assert_eq!(work.committed_candidates, 1);
    assert_eq!(work.provider_attempts, 5);
    assert_eq!(work.provider_completed, 4);
}

#[test]
fn prepared_token_drop_preserves_checkpoint_and_controller_shadow_and_commit_advances_both_state_and_budget(
) {
    use rodas5p_integrators::{
        adaptive_next_step_after_attempt, AdaptiveControllerState, AdaptiveStepConfig,
    };
    let mut budget = PhysicalErrorBudget::<8>::new(ID, 1.0, [1.0; 6]).unwrap();
    let before_budget = budget.clone();
    let mut work = PhysicalAdmissionWork::default();
    let mut state = [0.0];
    let mut time = 0.0;
    let next = [0.125];
    let mut controller = AdaptiveControllerState::default();
    let original = controller.clone();
    let mut shadow = controller.clone();
    let expected_next = adaptive_next_step_after_attempt(
        &mut shadow,
        &AdaptiveStepConfig::default(),
        0.5,
        0.5,
        0.25,
        5,
        true,
        false,
    )
    .unwrap();
    let mut scratch = [charge(1, ErrorComponent::TimeResidual, 0.125)];
    let mut provider = |ctx: &PhysicalCandidateContext<'_>, _: &mut [Contribution; 1]| {
        // The old endpoint was reborrowed internally from the exclusively held
        // checkpoint, rather than supplied as a separate caller assertion.
        assert_eq!(ctx.old_time, 0.0);
        assert_eq!(ctx.old_state, [0.0]);
        assert_eq!(ctx.candidate_time, 0.5);
        assert_eq!(ctx.candidate_state, [0.125]);
        Ok(1)
    };
    let prepared = prepare_physical_candidate(
        &mut budget,
        &mut work,
        binding(1),
        &mut time,
        &mut state,
        0.5,
        &next,
        true,
        PhysicalAdmissionMode::BoundedRequired,
        &mut scratch,
        &mut provider,
    )
    .unwrap();
    // A rejected downstream output operation drops the prepared checkpoint.
    drop(prepared);
    assert_eq!(budget, before_budget);
    assert_eq!(state, [0.0]);
    assert_eq!(time, 0.0);
    assert_eq!(controller, original);
    assert_eq!(work.provider_attempts, 1);
    assert_eq!(work.committed_candidates, 0);
    if let PhysicalCandidatePreparation::Prepared(token) = prepare_physical_candidate(
        &mut budget,
        &mut work,
        binding(1),
        &mut time,
        &mut state,
        0.5,
        &next,
        true,
        PhysicalAdmissionMode::BoundedRequired,
        &mut scratch,
        &mut provider,
    )
    .unwrap()
    {
        // External controller/output/cache adoption remains the driver's scope.
        controller = shadow.clone();
        token.commit();
    } else {
        panic!("expected prepared transaction");
    }
    assert_eq!(controller, shadow);
    assert!(expected_next.is_finite());
    assert_eq!(state, next);
    assert_eq!(time, 0.5);
    assert_eq!(budget.total_spent().unwrap(), 0.125);
    assert_eq!(work.committed_candidates, 1);
}

#[test]
fn candidate_consumer_exact_dyadic_temporal_refinements_and_jump() {
    let mut previous = f64::INFINITY;
    for n in [4_u64, 8, 16] {
        let h = 1.0 / n as f64;
        let mut ledger = PhysicalErrorBudget::<64>::new(ID, 1.0, [1.0; 6]).unwrap();
        let mut work = PhysicalAdmissionWork::default();
        let mut state = [0.0];
        let mut time = 0.0;
        let mut scratch = [charge(0, ErrorComponent::TimeResidual, 0.0); 2];
        for i in 0..n {
            let start = i as f64 * h;
            let end = (i + 1) as f64 * h;
            let mut next = [state[0] + h * start];
            let jump = i == n - 1;
            if jump {
                next[0] -= 0.03125;
            }
            let mut bound = |ctx: &PhysicalCandidateContext<'_>, out: &mut [Contribution; 2]| {
                assert_eq!(ctx.old_time, start);
                out[0] = charge(i, ErrorComponent::TimeResidual, h * h / 2.0);
                if jump {
                    out[1] = charge(n, ErrorComponent::FlowOrRoot, 0.03125);
                }
                Ok(if jump { 2 } else { 1 })
            };
            match prepare_physical_candidate(
                &mut ledger,
                &mut work,
                binding(i),
                &mut time,
                &mut state,
                end,
                &next,
                true,
                PhysicalAdmissionMode::BoundedRequired,
                &mut scratch,
                &mut bound,
            )
            .unwrap()
            {
                PhysicalCandidatePreparation::Prepared(token) => token.commit(),
                _ => panic!("expected prepared transaction"),
            }
        }
        // y'=t, exact y(1)=1/2, dyadic arithmetic: reference uncertainty0.
        let actual = (state[0] - 0.5).abs();
        assert_eq!(actual, h / 2.0 + 0.03125);
        assert_eq!(actual, ledger.total_spent().unwrap());
        assert!(actual < previous);
        previous = actual;
        assert_eq!(time, 1.0);
        assert_eq!(work.committed_candidates, n);
        assert_eq!(work.provider_attempts, n);
    }
}

#[test]
fn candidate_consumer_has_zero_allocations_for_checkpoint_commit_and_physical_refusal() {
    let mut ledger = PhysicalErrorBudget::<8>::new(ID, 1.0, [1.0; 6]).unwrap();
    let mut work = PhysicalAdmissionWork::default();
    let mut state = [0.0];
    let mut time = 0.0;
    let next = [0.25];
    let later = [0.5];
    let mut scratch = [charge(1, ErrorComponent::TimeResidual, 0.25)];
    let mut provider = |_: &PhysicalCandidateContext<'_>, _: &mut [Contribution; 1]| Ok(1);
    ALLOCS.with(|n| n.set(0));
    COUNTING.with(|on| on.set(true));
    let result = prepare_physical_candidate(
        &mut ledger,
        &mut work,
        binding(1),
        &mut time,
        &mut state,
        0.5,
        &next,
        true,
        PhysicalAdmissionMode::BoundedRequired,
        &mut scratch,
        &mut provider,
    );
    if let Ok(PhysicalCandidatePreparation::Prepared(token)) = result {
        token.commit();
    }
    let duplicate = prepare_physical_candidate(
        &mut ledger,
        &mut work,
        binding(2),
        &mut time,
        &mut state,
        1.0,
        &later,
        true,
        PhysicalAdmissionMode::BoundedRequired,
        &mut scratch,
        &mut provider,
    );
    let refused = matches!(
        duplicate,
        Err(StopOnPhysicalRefusal::Budget(
            BudgetError::DuplicatePerturbation
        ))
    );
    COUNTING.with(|on| on.set(false));
    assert_eq!(ALLOCS.with(Cell::get), 0);
    assert!(refused);
    assert_eq!(state, next);
    assert_eq!(time, 0.5);
    assert_eq!(work.committed_candidates, 1);
    assert_eq!(work.provider_attempts, 2);
}
