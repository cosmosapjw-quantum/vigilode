#![cfg(feature = "audit2-research")]

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use rodas5p_core::{
    CoreError, CoreResult, ExactPreconditionerIdentity, Preconditioner, WorkCounters,
};
use rodas5p_integrators::{
    Audit2ExternalOutputReference, Audit2FrozenWSemanticIdentity, Audit2IndependentStepBudget,
    Audit2MatrixFreeCommonWConfig, Audit2MatrixFreeCorrectionOutcome,
    Audit2ReferenceUncertaintyTreatment, Audit2ReusablePreconditionerCache,
    Audit2ReusablePreconditionerIdentity, Audit2TransactionCommitRule,
    Audit2TransactionalAttemptConfig, Audit2TransactionalAttemptOutcome,
    Audit2TransactionalSelection, OdeProblem, assess_audit2_reference_aware_output,
    audit2_conservative_l2_difference_upper, audit2_conservative_output_budget_lower,
    build_step_context_matrix_free, manufactured_vector_problem,
    run_audit2_matrix_free_common_w_correction,
    run_audit2_reusable_preconditioner_transactional_attempt,
};

#[derive(Debug)]
struct DiagonalPreconditioner {
    inverse: Vec<f64>,
}

#[derive(Debug)]
struct MutableDiagonalPreconditioner {
    inverse: Mutex<Vec<f64>>,
}

#[derive(Debug)]
struct FailingDiagonalPreconditioner {
    inverse: Vec<f64>,
    fail_on_attempt: usize,
    attempts: AtomicUsize,
}

impl Preconditioner for FailingDiagonalPreconditioner {
    fn dimension(&self) -> usize {
        self.inverse.len()
    }

    fn apply(&self, input: &[f64], output: &mut [f64]) -> CoreResult<()> {
        let attempt = self.attempts.fetch_add(1, Ordering::SeqCst);
        if attempt == self.fail_on_attempt {
            return Err(CoreError::LinearSolve(
                "injected reusable preconditioner apply failure".into(),
            ));
        }
        for ((value, inverse), input) in output.iter_mut().zip(&self.inverse).zip(input) {
            *value = inverse * input;
        }
        Ok(())
    }

    fn exact_identity(&self) -> Option<ExactPreconditionerIdentity> {
        Some(ExactPreconditionerIdentity::Jacobi {
            inverse_diagonal_bits: self.inverse.iter().map(|value| value.to_bits()).collect(),
        })
    }
}

impl Preconditioner for DiagonalPreconditioner {
    fn dimension(&self) -> usize {
        self.inverse.len()
    }

    fn apply(&self, input: &[f64], output: &mut [f64]) -> CoreResult<()> {
        if input.len() != self.inverse.len() || output.len() != input.len() {
            return Err(CoreError::Dimension(
                "test diagonal preconditioner shape mismatch".into(),
            ));
        }
        for ((value, inverse), input) in output.iter_mut().zip(&self.inverse).zip(input) {
            *value = inverse * input;
        }
        Ok(())
    }

    fn exact_identity(&self) -> Option<ExactPreconditionerIdentity> {
        Some(ExactPreconditionerIdentity::Jacobi {
            inverse_diagonal_bits: self.inverse.iter().map(|value| value.to_bits()).collect(),
        })
    }
}

impl Preconditioner for MutableDiagonalPreconditioner {
    fn dimension(&self) -> usize {
        self.inverse.lock().unwrap().len()
    }

    fn apply(&self, input: &[f64], output: &mut [f64]) -> CoreResult<()> {
        let inverse = self.inverse.lock().unwrap();
        if input.len() != inverse.len() || output.len() != input.len() {
            return Err(CoreError::Dimension(
                "test mutable diagonal preconditioner shape mismatch".into(),
            ));
        }
        for ((value, inverse), input) in output.iter_mut().zip(inverse.iter()).zip(input) {
            *value = inverse * input;
        }
        Ok(())
    }

    fn exact_identity(&self) -> Option<ExactPreconditionerIdentity> {
        Some(ExactPreconditionerIdentity::Jacobi {
            inverse_diagonal_bits: self
                .inverse
                .lock()
                .unwrap()
                .iter()
                .map(|value| value.to_bits())
                .collect(),
        })
    }
}

fn frozen_w_identity(hex_digit: char) -> Audit2FrozenWSemanticIdentity {
    Audit2FrozenWSemanticIdentity {
        schema: "audit2-test-frozen-w-v1".into(),
        sha256: std::iter::repeat_n(hex_digit, 64).collect(),
    }
}

fn diagonal_identity(
    provider: &str,
    revision: u64,
    inverse: &[f64],
) -> Audit2ReusablePreconditionerIdentity {
    Audit2ReusablePreconditionerIdentity {
        provider: provider.into(),
        revision,
        configuration_bits: vec![0x3ff0_0000_0000_0000],
        expected_inverse_diagonal_bits: inverse.iter().map(|value| value.to_bits()).collect(),
    }
}

#[test]
fn cache_reuses_only_the_same_frozen_w_and_declared_preconditioner_identity() -> CoreResult<()> {
    // Breaks if exact frozen-W identity is ignored, or if a valid same-W
    // binding is needlessly rebuilt.
    let (problem, y0) = manufactured_vector_problem(6, 100.0, 0.5, 0.1, 0.0)?;
    let context =
        build_step_context_matrix_free(&problem, 0.0, &y0, 0.01, &mut WorkCounters::default())?;
    let changed_h =
        build_step_context_matrix_free(&problem, 0.0, &y0, 0.02, &mut WorkCounters::default())?;
    let identity = diagonal_identity("analytic-diagonal", 7, &[0.5; 6]);
    let setup_calls = AtomicUsize::new(0);
    let mut cache = Audit2ReusablePreconditionerCache::default();

    let first = cache.begin_attempt(
        &context,
        frozen_w_identity('a'),
        identity.clone(),
        |frozen, setup_work| {
            setup_calls.fetch_add(1, Ordering::Relaxed);
            setup_work.jvp_calls = setup_work.jvp_calls.saturating_add(2);
            Ok(Arc::new(DiagonalPreconditioner {
                inverse: vec![0.5; frozen.problem.dimension],
            }) as Arc<dyn Preconditioner>)
        },
    )?;
    assert_eq!(first.dimension(), 6);
    cache.commit_attempt()?;

    let reused = cache.begin_attempt(
        &context,
        frozen_w_identity('a'),
        identity.clone(),
        |_, _| panic!("the same frozen W and identity must not rerun setup"),
    )?;
    assert!(Arc::ptr_eq(&first, &reused));
    cache.commit_attempt()?;

    let changed_identity = diagonal_identity("analytic-diagonal", 7, &[0.25; 6]);
    let rebuilt = cache.begin_attempt(
        &changed_h,
        frozen_w_identity('b'),
        changed_identity,
        |frozen, setup_work| {
            setup_calls.fetch_add(1, Ordering::Relaxed);
            setup_work.jvp_calls = setup_work.jvp_calls.saturating_add(3);
            Ok(Arc::new(DiagonalPreconditioner {
                inverse: vec![0.25; frozen.problem.dimension],
            }) as Arc<dyn Preconditioner>)
        },
    )?;
    assert!(!Arc::ptr_eq(&reused, &rebuilt));
    cache.commit_attempt()?;

    let snapshot = cache.snapshot();
    assert_eq!(setup_calls.load(Ordering::Relaxed), 2);
    assert_eq!(snapshot.attempts, 3);
    assert_eq!(snapshot.setup_attempts, 2);
    assert_eq!(snapshot.setup_completed, 2);
    assert_eq!(snapshot.same_binding_reuses, 1);
    assert_eq!(snapshot.changed_operator_invalidations, 1);
    assert_eq!(snapshot.changed_preconditioner_invalidations, 1);
    assert_eq!(snapshot.commits, 3);
    assert_eq!(snapshot.rollbacks, 0);
    assert!(snapshot.committed_binding.is_some());
    assert!(snapshot.pending_binding.is_none());
    assert_eq!(snapshot.setup_work.jvp_calls, 5);
    let serialized = serde_json::to_value(&snapshot).unwrap();
    let binding = &serialized["committed_binding"];
    assert!(binding.get("operator_token").is_none());
    assert!(binding.get("exact_operator_identity_available").is_none());
    assert_eq!(binding["frozen_w_semantic"]["sha256"], "b".repeat(64));
    Ok(())
}

#[test]
fn mathematical_identity_diagonal_is_rejected_even_when_provider_self_reports_nonidentity()
-> CoreResult<()> {
    // Breaks if the cache trusts the default `is_identity() == false` instead
    // of classifying the exact returned Jacobi bits.
    let (problem, y0) = manufactured_vector_problem(4, 60.0, 0.0, 0.1, 0.0)?;
    let context =
        build_step_context_matrix_free(&problem, 0.0, &y0, 1.0e-3, &mut WorkCounters::default())?;
    let mut cache = Audit2ReusablePreconditionerCache::default();
    let result = cache.begin_attempt(
        &context,
        frozen_w_identity('1'),
        diagonal_identity("self-reported-nonidentity", 1, &[1.0; 4]),
        |_, _| {
            Ok(Arc::new(DiagonalPreconditioner {
                inverse: vec![1.0; 4],
            }) as Arc<dyn Preconditioner>)
        },
    );
    let error = match result {
        Ok(_) => panic!("mathematical identity diagonal was accepted"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("exact nonidentity diagonal map"));
    let snapshot = cache.snapshot();
    assert_eq!(snapshot.setup_attempts, 1);
    assert_eq!(snapshot.setup_completed, 0);
    assert_eq!(snapshot.setup_failures, 1);
    assert!(snapshot.committed_binding.is_none());
    assert!(snapshot.pending_binding.is_none());
    Ok(())
}

#[test]
fn mutable_provider_cannot_change_the_committed_effective_diagonal_map() -> CoreResult<()> {
    // Breaks if the cache retains the provider object itself instead of a
    // cache-owned immutable map bound to the verified exact bits.
    let (problem, y0) = manufactured_vector_problem(4, 60.0, 0.0, 0.1, 0.0)?;
    let context =
        build_step_context_matrix_free(&problem, 0.0, &y0, 1.0e-3, &mut WorkCounters::default())?;
    let declared = diagonal_identity("mutable-provider", 1, &[0.5; 4]);
    let provider = Arc::new(MutableDiagonalPreconditioner {
        inverse: Mutex::new(vec![0.5; 4]),
    });
    let setup_calls = AtomicUsize::new(0);
    let mut cache = Audit2ReusablePreconditionerCache::default();
    let first = cache.begin_attempt(
        &context,
        frozen_w_identity('2'),
        declared.clone(),
        |_, _| {
            setup_calls.fetch_add(1, Ordering::SeqCst);
            Ok(Arc::clone(&provider) as Arc<dyn Preconditioner>)
        },
    )?;
    cache.commit_attempt()?;

    *provider.inverse.lock().unwrap() = vec![0.25; 4];
    let reused = cache.begin_attempt(&context, frozen_w_identity('2'), declared, |_, _| {
        setup_calls.fetch_add(1, Ordering::SeqCst);
        Ok(Arc::new(DiagonalPreconditioner {
            inverse: vec![0.5; 4],
        }) as Arc<dyn Preconditioner>)
    })?;
    assert!(!Arc::ptr_eq(&first, &reused));
    let mut output = vec![0.0; 4];
    reused.apply(&[2.0; 4], &mut output)?;
    assert_eq!(output, vec![1.0; 4]);
    cache.commit_attempt()?;

    let snapshot = cache.snapshot();
    assert_eq!(setup_calls.load(Ordering::SeqCst), 2);
    assert_eq!(snapshot.setup_attempts, 2);
    assert_eq!(snapshot.setup_completed, 2);
    assert_eq!(snapshot.same_binding_reuses, 0);
    assert_eq!(snapshot.changed_preconditioner_invalidations, 1);
    Ok(())
}

#[test]
fn changed_semantic_w_digest_invalidates_even_the_same_runtime_operator() -> CoreResult<()> {
    // Breaks if a caller-declared dependency change is ignored merely because
    // the same live ShiftedOperator instance is still present.
    let (problem, y0) = manufactured_vector_problem(4, 60.0, 0.0, 0.1, 0.0)?;
    let context =
        build_step_context_matrix_free(&problem, 0.0, &y0, 1.0e-3, &mut WorkCounters::default())?;
    let identity = diagonal_identity("semantic-diagonal", 1, &[0.5; 4]);
    let setup_calls = AtomicUsize::new(0);
    let mut cache = Audit2ReusablePreconditionerCache::default();
    let first = cache.begin_attempt(
        &context,
        frozen_w_identity('5'),
        identity.clone(),
        |_, _| {
            setup_calls.fetch_add(1, Ordering::Relaxed);
            Ok(Arc::new(DiagonalPreconditioner {
                inverse: vec![0.5; 4],
            }) as Arc<dyn Preconditioner>)
        },
    )?;
    cache.commit_attempt()?;
    let rebuilt = cache.begin_attempt(&context, frozen_w_identity('6'), identity, |_, _| {
        setup_calls.fetch_add(1, Ordering::Relaxed);
        Ok(Arc::new(DiagonalPreconditioner {
            inverse: vec![0.5; 4],
        }) as Arc<dyn Preconditioner>)
    })?;
    assert!(!Arc::ptr_eq(&first, &rebuilt));
    cache.commit_attempt()?;
    let snapshot = cache.snapshot();
    assert_eq!(setup_calls.load(Ordering::Relaxed), 2);
    assert_eq!(snapshot.same_binding_reuses, 0);
    assert_eq!(snapshot.changed_operator_invalidations, 1);
    assert_eq!(snapshot.changed_preconditioner_invalidations, 0);
    Ok(())
}

#[test]
fn returned_preconditioner_map_must_match_its_declared_exact_identity() -> CoreResult<()> {
    // Breaks if a provider can label one map and return another map that is
    // then cached under the false declaration.
    let (problem, y0) = manufactured_vector_problem(4, 60.0, 0.0, 0.1, 0.0)?;
    let context =
        build_step_context_matrix_free(&problem, 0.0, &y0, 1.0e-3, &mut WorkCounters::default())?;
    let mut cache = Audit2ReusablePreconditionerCache::default();
    let result = cache.begin_attempt(
        &context,
        frozen_w_identity('7'),
        diagonal_identity("mismatched-diagonal", 1, &[0.5; 4]),
        |_, _| {
            Ok(Arc::new(DiagonalPreconditioner {
                inverse: vec![0.25; 4],
            }) as Arc<dyn Preconditioner>)
        },
    );
    let error = match result {
        Ok(_) => panic!("mismatched exact preconditioner map was accepted"),
        Err(error) => error,
    };
    assert!(
        error
            .to_string()
            .contains("different from its declared identity")
    );
    let snapshot = cache.snapshot();
    assert_eq!(snapshot.setup_attempts, 1);
    assert_eq!(snapshot.setup_completed, 0);
    assert_eq!(snapshot.setup_failures, 1);
    assert!(snapshot.committed_binding.is_none());
    assert!(snapshot.pending_binding.is_none());
    Ok(())
}

#[test]
fn failed_preconditioner_setup_is_terminal_and_preserves_committed_binding() -> CoreResult<()> {
    // Breaks if a failed rebuild overwrites a committed PC, loses partial
    // setup work, or leaves an unterminated pending lease.
    let (problem, y0) = manufactured_vector_problem(4, 60.0, 0.0, 0.1, 0.0)?;
    let context =
        build_step_context_matrix_free(&problem, 0.0, &y0, 1.0e-3, &mut WorkCounters::default())?;
    let changed_h =
        build_step_context_matrix_free(&problem, 0.0, &y0, 2.0e-3, &mut WorkCounters::default())?;
    let identity = diagonal_identity("exact-diagonal", 1, &[0.5; 4]);
    let mut cache = Audit2ReusablePreconditionerCache::default();
    let committed = cache.begin_attempt(
        &context,
        frozen_w_identity('c'),
        identity.clone(),
        |_, _| {
            Ok(Arc::new(DiagonalPreconditioner {
                inverse: vec![0.5; 4],
            }) as Arc<dyn Preconditioner>)
        },
    )?;
    cache.commit_attempt()?;
    let committed_binding = cache.snapshot().committed_binding;

    let error = match cache.begin_attempt(
        &changed_h,
        frozen_w_identity('d'),
        identity.clone(),
        |_, setup_work| {
            setup_work.jvp_calls = setup_work.jvp_calls.saturating_add(3);
            Err(CoreError::LinearSolve("injected zero pivot".into()))
        },
    ) {
        Ok(_) => panic!("injected setup failure must be preserved"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("zero pivot"));
    let failed = cache.snapshot();
    assert_eq!(failed.setup_attempts, 2);
    assert_eq!(failed.setup_completed, 1);
    assert_eq!(failed.setup_failures, 1);
    assert_eq!(failed.commits, 1);
    assert_eq!(failed.rollbacks, 0);
    assert_eq!(failed.setup_work.jvp_calls, 3);
    assert_eq!(failed.committed_binding, committed_binding);
    assert!(failed.pending_binding.is_none());
    assert!(
        failed
            .last_setup_failure
            .as_deref()
            .unwrap()
            .contains("zero pivot")
    );

    let reused = cache.begin_attempt(&context, frozen_w_identity('c'), identity, |_, _| {
        panic!("failed rebuild must preserve the old committed entry")
    })?;
    assert!(Arc::ptr_eq(&committed, &reused));
    cache.commit_attempt()?;
    Ok(())
}

#[test]
fn transaction_setup_failure_preserves_partial_setup_work_before_candidate_solve() -> CoreResult<()>
{
    // Breaks if setup work is held only inside the cache snapshot and omitted
    // from the monotone whole-attempt WorkCounters ledger.
    let (problem, y0) = manufactured_vector_problem(4, 60.0, 0.0, 0.1, 0.0)?;
    let context =
        build_step_context_matrix_free(&problem, 0.0, &y0, 1.0e-3, &mut WorkCounters::default())?;
    let config = Audit2TransactionalAttemptConfig {
        common_w: Audit2MatrixFreeCommonWConfig::default(),
        outer_atol: 1.0e-9,
        outer_rtol: 1.0e-7,
        commit_rule: Audit2TransactionCommitRule::Causal,
        reuse_certificate: false,
    };
    let budget = Audit2IndependentStepBudget {
        identifier: "fixed-setup-failure-budget-v1".into(),
        output_atol_l2: 1.0,
        output_rtol: 0.0,
        max_embedded_l2: 1.0,
        max_original_target_residual_l2: 1.0,
        max_original_target_contraction: 1.0,
    };
    let reference = Audit2ExternalOutputReference {
        source: "manufactured-exact-v1".into(),
        state: problem.exact(context.t + context.h).unwrap(),
        uncertainty_l2: 0.0,
        uncertainty_treatment: Audit2ReferenceUncertaintyTreatment::DeclaredUpperBound,
    };
    let mut cache = Audit2ReusablePreconditionerCache::default();
    let mut work = WorkCounters::default();
    let outcome = run_audit2_reusable_preconditioner_transactional_attempt(
        &context,
        &zero_trial(context.coeffs.stages(), problem.dimension),
        &config,
        &budget,
        &reference,
        &mut cache,
        frozen_w_identity('f'),
        diagonal_identity("failing-diagonal", 1, &[0.5; 4]),
        |_, setup_work| {
            setup_work.jvp_calls = setup_work.jvp_calls.saturating_add(3);
            setup_work.jvp_vectors = setup_work.jvp_vectors.saturating_add(3);
            Err(CoreError::LinearSolve("injected setup failure".into()))
        },
        &mut work,
    );
    let failed = match outcome {
        Audit2TransactionalAttemptOutcome::Failed(value) => *value,
        Audit2TransactionalAttemptOutcome::Completed(value) => {
            panic!("setup failure unexpectedly completed: {value:?}")
        }
    };
    assert_eq!(
        failed.phase,
        rodas5p_integrators::Audit2TransactionalFailurePhase::PreconditionerSetup
    );
    assert_eq!(failed.cache.setup_attempts, 1);
    assert_eq!(failed.cache.setup_completed, 0);
    assert_eq!(failed.cache.setup_failures, 1);
    assert_eq!(failed.cache.setup_work.jvp_calls, 3);
    assert_eq!(failed.work, work);
    assert_eq!(work.jvp_calls, 2 * context.coeffs.stages() as u64 + 3);
    assert_eq!(work.jvp_vectors, work.jvp_calls);
    assert_eq!(work.linear_solves, 0);
    assert_eq!(work.accepted_steps, 0);
    assert_eq!(work.rejected_steps, 0);
    Ok(())
}

#[test]
fn invalid_common_w_config_fails_before_attempt_setup_or_work() -> CoreResult<()> {
    // Breaks if malformed inner-solver policy is discovered only after target
    // preparation or reusable-PC setup has already spent work.
    let (problem, y0) = manufactured_vector_problem(4, 60.0, 0.0, 0.1, 0.0)?;
    let context =
        build_step_context_matrix_free(&problem, 0.0, &y0, 1.0e-3, &mut WorkCounters::default())?;
    let config = Audit2TransactionalAttemptConfig {
        common_w: Audit2MatrixFreeCommonWConfig {
            restart: 0,
            ..Audit2MatrixFreeCommonWConfig::default()
        },
        outer_atol: 1.0e-9,
        outer_rtol: 1.0e-7,
        commit_rule: Audit2TransactionCommitRule::Causal,
        reuse_certificate: false,
    };
    let budget = Audit2IndependentStepBudget {
        identifier: "fixed-invalid-config-budget-v1".into(),
        output_atol_l2: 1.0,
        output_rtol: 0.0,
        max_embedded_l2: 1.0,
        max_original_target_residual_l2: 1.0,
        max_original_target_contraction: 1.0,
    };
    let reference = Audit2ExternalOutputReference {
        source: "manufactured-exact-v1".into(),
        state: problem.exact(context.t + context.h).unwrap(),
        uncertainty_l2: 0.0,
        uncertainty_treatment: Audit2ReferenceUncertaintyTreatment::DeclaredUpperBound,
    };
    let setup_calls = AtomicUsize::new(0);
    let mut cache = Audit2ReusablePreconditionerCache::default();
    let mut work = WorkCounters::default();
    let outcome = run_audit2_reusable_preconditioner_transactional_attempt(
        &context,
        &zero_trial(context.coeffs.stages(), problem.dimension),
        &config,
        &budget,
        &reference,
        &mut cache,
        frozen_w_identity('1'),
        diagonal_identity("must-not-run", 1, &[0.5; 4]),
        |_, _| {
            setup_calls.fetch_add(1, Ordering::Relaxed);
            Ok(Arc::new(DiagonalPreconditioner {
                inverse: vec![0.5; 4],
            }) as Arc<dyn Preconditioner>)
        },
        &mut work,
    );
    let failed = match outcome {
        Audit2TransactionalAttemptOutcome::Failed(value) => *value,
        Audit2TransactionalAttemptOutcome::Completed(value) => {
            panic!("invalid config unexpectedly completed: {value:?}")
        }
    };
    assert_eq!(
        failed.phase,
        rodas5p_integrators::Audit2TransactionalFailurePhase::InputValidation
    );
    assert_eq!(setup_calls.load(Ordering::Relaxed), 0);
    assert_eq!(cache.snapshot().attempts, 0);
    assert_eq!(work, WorkCounters::default());
    assert_eq!(failed.work, WorkCounters::default());
    Ok(())
}

fn zero_trial(stages: usize, dimension: usize) -> Vec<Vec<f64>> {
    vec![vec![0.0; dimension]; stages]
}

#[test]
fn asserted_reference_uncertainty_is_consumed_inside_the_output_budget() {
    // Killing case for the former E_ref <= B + u gate: E_ref=0.9 would
    // incorrectly pass against B=1 and u=0.2 even though the proved true-error
    // upper bound is 1.1.  Estimate-only uncertainty cannot admit a candidate.
    let rejected = assess_audit2_reference_aware_output(
        0.9,
        1.0,
        0.2,
        Audit2ReferenceUncertaintyTreatment::DeclaredUpperBound,
    );
    assert!(rejected.output_error_upper_l2 >= 1.1);
    assert!(!rejected.accepted);

    let admitted = assess_audit2_reference_aware_output(
        0.7,
        1.0,
        0.2,
        Audit2ReferenceUncertaintyTreatment::DeclaredUpperBound,
    );
    assert!((admitted.output_error_upper_l2 - 0.9).abs() <= f64::EPSILON);
    assert!(admitted.accepted);

    let unresolved = assess_audit2_reference_aware_output(
        0.0,
        1.0,
        0.0,
        Audit2ReferenceUncertaintyTreatment::EstimateOnly,
    );
    assert!(!unresolved.accepted);

    let half_ulp = 2.0f64.powi(-54);
    let rounding_boundary = assess_audit2_reference_aware_output(
        1.0,
        1.0,
        half_ulp,
        Audit2ReferenceUncertaintyTreatment::DeclaredUpperBound,
    );
    assert!(rounding_boundary.output_error_upper_l2 > 1.0);
    assert!(!rounding_boundary.accepted);

    let hidden_norm_increment = 2.0f64.powi(-27);
    let distance_upper =
        audit2_conservative_l2_difference_upper(&[1.0, hidden_norm_increment], &[0.0, 0.0]);
    assert!(distance_upper > 1.0);
    let budget_lower =
        audit2_conservative_output_budget_lower(0.0, 1.0, &[1.0, hidden_norm_increment]);
    assert!(budget_lower < 1.0);

    let exact_boundary = assess_audit2_reference_aware_output(
        1.0,
        1.0,
        0.0,
        Audit2ReferenceUncertaintyTreatment::DeclaredUpperBound,
    );
    assert!(exact_boundary.accepted);
    for invalid in [f64::NAN, f64::INFINITY, -1.0] {
        assert!(
            !assess_audit2_reference_aware_output(
                0.0,
                1.0,
                invalid,
                Audit2ReferenceUncertaintyTreatment::DeclaredUpperBound,
            )
            .accepted
        );
    }
}

#[test]
fn reference_verdict_does_not_treat_reference_uncertainty_as_extra_tolerance() -> CoreResult<()> {
    // This exercises the transaction seam, not only the pure assessment
    // helper: the recorded reference verdict must reject, where the former
    // E_ref <= B + u formula would accept. Since audit F-014 the verdict is
    // audit-only, so the candidate commits on its causal gates.
    let (problem, y0) = manufactured_vector_problem(4, 80.0, 0.0, 0.2, 0.0)?;
    let context =
        build_step_context_matrix_free(&problem, 0.0, &y0, 1.0e-3, &mut WorkCounters::default())?;
    let reference = Audit2ExternalOutputReference {
        source: "manufactured-exact-with-declared-bound-v1".into(),
        state: problem.exact(context.t + context.h).unwrap(),
        uncertainty_l2: 2.0,
        uncertainty_treatment: Audit2ReferenceUncertaintyTreatment::DeclaredUpperBound,
    };
    let budget = Audit2IndependentStepBudget {
        identifier: "reference-uncertainty-inside-budget-killer-v1".into(),
        output_atol_l2: 1.0,
        output_rtol: 0.0,
        max_embedded_l2: 10.0,
        max_original_target_residual_l2: 10.0,
        max_original_target_contraction: 10.0,
    };
    let config = Audit2TransactionalAttemptConfig {
        common_w: Audit2MatrixFreeCommonWConfig::default(),
        outer_atol: 1.0,
        outer_rtol: 0.0,
        commit_rule: Audit2TransactionCommitRule::Causal,
        reuse_certificate: false,
    };
    let mut cache = Audit2ReusablePreconditionerCache::default();
    let mut work = WorkCounters::default();
    let outcome = run_audit2_reusable_preconditioner_transactional_attempt(
        &context,
        &zero_trial(context.coeffs.stages(), problem.dimension),
        &config,
        &budget,
        &reference,
        &mut cache,
        frozen_w_identity('0'),
        diagonal_identity("uncertainty-killer-diagonal", 1, &[0.5; 4]),
        |_, _| {
            Ok(Arc::new(DiagonalPreconditioner {
                inverse: vec![0.5; 4],
            }) as Arc<dyn Preconditioner>)
        },
        &mut work,
    );
    let completed = match outcome {
        Audit2TransactionalAttemptOutcome::Completed(value) => *value,
        Audit2TransactionalAttemptOutcome::Failed(failure) => {
            panic!("protected fallback unexpectedly failed: {failure:?}")
        }
    };
    assert_eq!(completed.selection, Audit2TransactionalSelection::Candidate);
    let candidate = completed.candidate.expect("candidate budget receipt");
    assert!(candidate.budget.accepted, "causal gates pass");
    assert_eq!(candidate.budget.output_budget_l2, 1.0);
    assert!(candidate.budget.output_error_l2 < 1.0);
    assert!(candidate.budget.output_error_upper_l2 >= 2.0);
    assert!(candidate.budget.output_error_upper_l2 > candidate.budget.output_budget_l2);
    assert_eq!(candidate.budget.reference_uncertainty_l2, 2.0);
    assert_eq!(
        candidate.budget.uncertainty_treatment,
        Audit2ReferenceUncertaintyTreatment::DeclaredUpperBound
    );
    assert!(!candidate.budget.output_accepted);
    assert_eq!(cache.snapshot().commits, 1);
    assert_eq!(cache.snapshot().rollbacks, 0);
    Ok(())
}

#[test]
fn estimate_only_reference_is_not_upgraded_in_the_reference_verdict() -> CoreResult<()> {
    // Breaks if a numerically zero but non-authoritative uncertainty is
    // silently upgraded into a declared upper bound by the transaction. Since
    // audit F-014 the verdict is audit-only: the candidate commits on its
    // causal gates while the recorded reference verdict rejects.
    let (problem, y0) = manufactured_vector_problem(4, 80.0, 0.0, 0.2, 0.0)?;
    let context =
        build_step_context_matrix_free(&problem, 0.0, &y0, 1.0e-3, &mut WorkCounters::default())?;
    let reference = Audit2ExternalOutputReference {
        source: "manufactured-estimate-only-v1".into(),
        state: problem.exact(context.t + context.h).unwrap(),
        uncertainty_l2: 0.0,
        uncertainty_treatment: Audit2ReferenceUncertaintyTreatment::EstimateOnly,
    };
    let budget = Audit2IndependentStepBudget {
        identifier: "estimate-only-killer-v1".into(),
        output_atol_l2: 1.0,
        output_rtol: 0.0,
        max_embedded_l2: 10.0,
        max_original_target_residual_l2: 10.0,
        max_original_target_contraction: 10.0,
    };
    let config = Audit2TransactionalAttemptConfig {
        common_w: Audit2MatrixFreeCommonWConfig::default(),
        outer_atol: 1.0,
        outer_rtol: 0.0,
        commit_rule: Audit2TransactionCommitRule::Causal,
        reuse_certificate: false,
    };
    let mut cache = Audit2ReusablePreconditionerCache::default();
    let mut work = WorkCounters::default();
    let outcome = run_audit2_reusable_preconditioner_transactional_attempt(
        &context,
        &zero_trial(context.coeffs.stages(), problem.dimension),
        &config,
        &budget,
        &reference,
        &mut cache,
        frozen_w_identity('9'),
        diagonal_identity("estimate-only-diagonal", 1, &[0.5; 4]),
        |_, _| {
            Ok(Arc::new(DiagonalPreconditioner {
                inverse: vec![0.5; 4],
            }) as Arc<dyn Preconditioner>)
        },
        &mut work,
    );
    let completed = match outcome {
        Audit2TransactionalAttemptOutcome::Completed(value) => *value,
        Audit2TransactionalAttemptOutcome::Failed(failure) => {
            panic!("estimate-only protected fallback unexpectedly failed: {failure:?}")
        }
    };
    assert_eq!(completed.selection, Audit2TransactionalSelection::Candidate);
    let candidate = completed.candidate.expect("candidate budget receipt");
    assert!(candidate.budget.accepted, "causal gates pass");
    assert!(!candidate.budget.output_accepted);
    assert_eq!(
        candidate.budget.uncertainty_treatment,
        Audit2ReferenceUncertaintyTreatment::EstimateOnly
    );
    assert_eq!(cache.snapshot().commits, 1);
    assert_eq!(cache.snapshot().rollbacks, 0);
    Ok(())
}

#[test]
fn prepared_target_samples_stateful_batch_rhs_exactly_once() -> CoreResult<()> {
    // Breaks if residual and stage-linearization snapshots can observe two
    // different calls of a stateful batched RHS callback.
    let batch_calls = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&batch_calls);
    let problem = OdeProblem::new(
        "audit2-single-prepared-target",
        1,
        Arc::new(|_: f64, _: &[f64], output: &mut [f64]| {
            output[0] = 0.0;
            Ok(())
        }),
        Some(Arc::new(move |_: &[f64], states: &[Vec<f64>]| {
            let call_index = observed.fetch_add(1, Ordering::SeqCst);
            if call_index != 0 {
                return Err(CoreError::InvalidInput(format!(
                    "prepared target RHS sampled again at call {call_index}"
                )));
            }
            Ok((0..states.len())
                .map(|stage| vec![(stage + 1) as f64])
                .collect())
        })),
        None,
        Some(Arc::new(
            |_: f64, _: &[f64], _: &[f64], output: &mut [f64]| {
                output[0] = 0.0;
                Ok(())
            },
        )),
        None,
        true,
        None,
        None,
    )?;
    let h = 0.125;
    let context =
        build_step_context_matrix_free(&problem, 0.0, &[0.0], h, &mut WorkCounters::default())?;
    let outcome = run_audit2_matrix_free_common_w_correction(
        &context,
        &zero_trial(context.coeffs.stages(), problem.dimension),
        Audit2MatrixFreeCommonWConfig::default(),
    );
    let success = match outcome {
        Audit2MatrixFreeCorrectionOutcome::Completed(value) => *value,
        Audit2MatrixFreeCorrectionOutcome::Failed(failure) => {
            panic!("single prepared target unexpectedly failed: {failure:?}")
        }
    };
    assert_eq!(batch_calls.load(Ordering::SeqCst), 1);
    assert_eq!(success.work.preparation_counters.rhs_batch_calls, 1);
    assert_eq!(
        success.work.preparation_counters.rhs_evaluations,
        context.coeffs.stages() as u64
    );
    assert_eq!(success.work.preparation_counters.block_matvecs, 1);
    assert_eq!(
        success.work.preparation_counters.jvp_calls,
        2 * context.coeffs.stages() as u64
    );
    for (stage, row) in success.projected_residual.iter().enumerate() {
        assert_eq!(row[0].to_bits(), (-h * (stage + 1) as f64).to_bits());
    }
    Ok(())
}

#[test]
fn admitted_candidate_commits_state_and_nonidentity_preconditioner_atomically() -> CoreResult<()> {
    // Breaks if the candidate can commit without all independent gates, if the
    // provisional PC is not committed with it, or if the supplied PC is ignored.
    let (problem, y0) = manufactured_vector_problem(8, 80.0, 0.0, 0.2, 0.0)?;
    let context =
        build_step_context_matrix_free(&problem, 0.0, &y0, 1.0e-3, &mut WorkCounters::default())?;
    let reference = Audit2ExternalOutputReference {
        source: "manufactured-exact-v1".into(),
        state: problem.exact(context.t + context.h).unwrap(),
        uncertainty_l2: 0.0,
        uncertainty_treatment: Audit2ReferenceUncertaintyTreatment::DeclaredUpperBound,
    };
    let budget = Audit2IndependentStepBudget {
        identifier: "fixed-permissive-linear-probe-v1".into(),
        output_atol_l2: 1.0,
        output_rtol: 0.0,
        max_embedded_l2: 1.0,
        max_original_target_residual_l2: 1.0e-8,
        max_original_target_contraction: 1.0e-8,
    };
    let config = Audit2TransactionalAttemptConfig {
        common_w: Audit2MatrixFreeCommonWConfig::default(),
        outer_atol: 1.0e-9,
        outer_rtol: 1.0e-7,
        commit_rule: Audit2TransactionCommitRule::Causal,
        reuse_certificate: false,
    };
    let identity = diagonal_identity("scaled-diagonal-test", 1, &[0.5; 8]);
    let mut cache = Audit2ReusablePreconditionerCache::default();
    let mut work = WorkCounters::default();
    let outcome = run_audit2_reusable_preconditioner_transactional_attempt(
        &context,
        &zero_trial(context.coeffs.stages(), problem.dimension),
        &config,
        &budget,
        &reference,
        &mut cache,
        frozen_w_identity('e'),
        identity,
        |frozen, _| {
            Ok(Arc::new(DiagonalPreconditioner {
                inverse: vec![0.5; frozen.problem.dimension],
            }) as Arc<dyn Preconditioner>)
        },
        &mut work,
    );
    let completed = match outcome {
        Audit2TransactionalAttemptOutcome::Completed(value) => *value,
        Audit2TransactionalAttemptOutcome::Failed(failure) => {
            panic!("transactional candidate unexpectedly failed: {failure:?}")
        }
    };

    assert_eq!(completed.selection, Audit2TransactionalSelection::Candidate);
    assert!(completed.committed);
    assert_eq!(
        completed.committed_state,
        completed
            .selected_step
            .as_ref()
            .expect("committed candidate step")
            .y_new
    );
    assert_ne!(completed.committed_state, context.y);
    assert!(completed.fallback_step.is_none());
    let candidate = completed.candidate.expect("candidate receipt");
    assert_eq!(
        candidate
            .correction
            .work
            .preparation_counters
            .rhs_batch_calls,
        1
    );
    assert_eq!(
        candidate
            .correction
            .work
            .preparation_counters
            .rhs_evaluations,
        context.coeffs.stages() as u64
    );
    assert!(candidate.budget.accepted);
    assert!(candidate.budget.output_accepted);
    assert!(candidate.budget.embedded_accepted);
    assert!(candidate.budget.original_target_accepted);
    assert!(candidate.original_target_residual_after_l2 <= 1.0e-8);
    let session = candidate
        .correction
        .work
        .session
        .as_ref()
        .expect("candidate session receipt");
    assert_eq!(session.identity_preconditioner_setups, 0);
    assert_eq!(session.reusable_preconditioner_setups, 1);
    assert!(session.preconditioner_apply_attempts > 0);
    assert_eq!(
        session.preconditioner_apply_attempts,
        session.preconditioner_apply_completed
    );
    assert_eq!(session.counters.direct_factorizations, 0);
    assert_eq!(session.counters.direct_solve_calls, 0);
    let cache = cache.snapshot();
    assert_eq!(cache.setup_attempts, 1);
    assert_eq!(cache.setup_completed, 1);
    assert_eq!(cache.commits, 1);
    assert_eq!(cache.rollbacks, 0);
    assert!(cache.committed_binding.is_some());
    assert!(cache.pending_binding.is_none());
    assert_eq!(work.linear_solves, session.counters.linear_solves);
    assert_eq!(work.linear_matvecs, session.counters.linear_matvecs);
    assert_eq!(
        work.preconditioner_apps,
        session.counters.preconditioner_apps
    );
    assert_eq!(completed.work, work);
    assert_eq!(work.accepted_steps, 1);
    assert_eq!(work.rejected_steps, 0);
    Ok(())
}

#[test]
fn late_preconditioner_apply_failure_rolls_back_lease_and_runs_isolated_fallback() -> CoreResult<()>
{
    // Breaks if partial candidate work is erased, if the provisional PC leaks
    // into fallback, or if the candidate lease commits after a solve failure.
    let (problem, y0) = manufactured_vector_problem(6, 80.0, 0.0, 0.2, 0.0)?;
    let context =
        build_step_context_matrix_free(&problem, 0.0, &y0, 1.0e-3, &mut WorkCounters::default())?;
    let reference = Audit2ExternalOutputReference {
        source: "manufactured-exact-v1".into(),
        state: problem.exact(context.t + context.h).unwrap(),
        uncertainty_l2: 0.0,
        uncertainty_treatment: Audit2ReferenceUncertaintyTreatment::DeclaredUpperBound,
    };
    let budget = Audit2IndependentStepBudget {
        identifier: "fixed-permissive-failure-probe-v1".into(),
        output_atol_l2: 10.0,
        output_rtol: 0.0,
        max_embedded_l2: 10.0,
        max_original_target_residual_l2: 10.0,
        max_original_target_contraction: 10.0,
    };
    let config = Audit2TransactionalAttemptConfig {
        common_w: Audit2MatrixFreeCommonWConfig::default(),
        outer_atol: 1.0,
        outer_rtol: 0.0,
        commit_rule: Audit2TransactionCommitRule::Causal,
        reuse_certificate: false,
    };
    let failing = Arc::new(FailingDiagonalPreconditioner {
        inverse: vec![0.5; problem.dimension],
        fail_on_attempt: 1,
        attempts: AtomicUsize::new(0),
    });
    let mut cache = Audit2ReusablePreconditionerCache::default();
    let mut work = WorkCounters::default();
    let outcome = run_audit2_reusable_preconditioner_transactional_attempt(
        &context,
        &zero_trial(context.coeffs.stages(), problem.dimension),
        &config,
        &budget,
        &reference,
        &mut cache,
        frozen_w_identity('2'),
        diagonal_identity("failing-diagonal", 1, &[0.5; 6]),
        |_, _| Ok(Arc::clone(&failing) as Arc<dyn Preconditioner>),
        &mut work,
    );
    let completed = match outcome {
        Audit2TransactionalAttemptOutcome::Completed(value) => *value,
        Audit2TransactionalAttemptOutcome::Failed(failure) => {
            panic!("protected fallback unexpectedly failed: {failure:?}")
        }
    };
    assert_eq!(
        completed.selection,
        Audit2TransactionalSelection::ProtectedFallback
    );
    assert!(completed.committed);
    assert!(completed.candidate.is_none());
    let candidate_failure = completed
        .candidate_failure
        .expect("candidate failure receipt");
    let session = candidate_failure
        .work
        .session
        .expect("partial session receipt");
    assert!(session.preconditioner_apply_attempts > session.preconditioner_apply_completed);
    assert_eq!(session.preconditioner_apply_attempts, 2);
    assert_eq!(session.preconditioner_apply_completed, 1);
    assert_eq!(session.counters.preconditioner_apps, 1);
    assert!(work.preconditioner_apps > session.counters.preconditioner_apps);
    assert_eq!(failing.attempts.load(Ordering::SeqCst), 2);
    assert_eq!(work.linear_solve_failures, 1);
    assert_eq!(work.fallback_steps, 1);
    assert_eq!(work.accepted_steps, 1);
    assert_eq!(work.rejected_steps, 0);
    let cache = cache.snapshot();
    assert_eq!(cache.commits, 0);
    assert_eq!(cache.rollbacks, 1);
    assert!(cache.committed_binding.is_none());
    assert!(cache.pending_binding.is_none());
    Ok(())
}

#[test]
fn nonfinite_derived_output_budget_fails_before_setup_or_candidate_work() -> CoreResult<()> {
    // Breaks if finite inputs whose derived arithmetic overflows to +infinity
    // are discovered only after reusable setup or candidate work.
    let (problem, y0) = manufactured_vector_problem(6, 80.0, 0.0, 0.2, 0.0)?;
    let context =
        build_step_context_matrix_free(&problem, 0.0, &y0, 1.0e-3, &mut WorkCounters::default())?;
    let reference = Audit2ExternalOutputReference {
        source: "manufactured-exact-v1".into(),
        state: problem.exact(context.t + context.h).unwrap(),
        uncertainty_l2: 0.0,
        uncertainty_treatment: Audit2ReferenceUncertaintyTreatment::DeclaredUpperBound,
    };
    let budget = Audit2IndependentStepBudget {
        identifier: "fixed-overflow-killer-v1".into(),
        output_atol_l2: f64::MAX,
        output_rtol: f64::MAX,
        max_embedded_l2: 10.0,
        max_original_target_residual_l2: 10.0,
        max_original_target_contraction: 10.0,
    };
    let config = Audit2TransactionalAttemptConfig {
        common_w: Audit2MatrixFreeCommonWConfig::default(),
        outer_atol: 1.0,
        outer_rtol: 0.0,
        commit_rule: Audit2TransactionCommitRule::Causal,
        reuse_certificate: false,
    };
    let mut cache = Audit2ReusablePreconditionerCache::default();
    let mut work = WorkCounters::default();
    let outcome = run_audit2_reusable_preconditioner_transactional_attempt(
        &context,
        &zero_trial(context.coeffs.stages(), problem.dimension),
        &config,
        &budget,
        &reference,
        &mut cache,
        frozen_w_identity('3'),
        diagonal_identity("overflow-killer-diagonal", 1, &[0.5; 6]),
        |_, _| {
            Ok(Arc::new(DiagonalPreconditioner {
                inverse: vec![0.5; 6],
            }) as Arc<dyn Preconditioner>)
        },
        &mut work,
    );
    let failed = match outcome {
        Audit2TransactionalAttemptOutcome::Failed(value) => *value,
        Audit2TransactionalAttemptOutcome::Completed(value) => {
            panic!("nonfinite derived budget unexpectedly completed: {value:?}")
        }
    };
    assert_eq!(
        failed.phase,
        rodas5p_integrators::Audit2TransactionalFailurePhase::InputValidation
    );
    assert!(failed.message.contains("output budget is nonfinite"));
    assert_eq!(cache.snapshot().attempts, 0);
    assert_eq!(work, WorkCounters::default());
    assert_eq!(failed.work, WorkCounters::default());
    Ok(())
}

#[test]
fn candidate_and_fallback_rejection_preserves_base_state_and_exposes_no_selected_step()
-> CoreResult<()> {
    // Breaks if an uncommitted fallback y_new is exposed as the selected step,
    // or if numerical state/work disposition are rolled back together.
    let (problem, y0) = manufactured_vector_problem(6, 80.0, 0.0, 0.2, 0.0)?;
    let context =
        build_step_context_matrix_free(&problem, 0.0, &y0, 1.0e-3, &mut WorkCounters::default())?;
    let reference = Audit2ExternalOutputReference {
        source: "manufactured-exact-v1".into(),
        state: problem.exact(context.t + context.h).unwrap(),
        uncertainty_l2: 0.0,
        uncertainty_treatment: Audit2ReferenceUncertaintyTreatment::DeclaredUpperBound,
    };
    let budget = Audit2IndependentStepBudget {
        identifier: "fixed-rejection-killer-v1".into(),
        output_atol_l2: 0.0,
        output_rtol: 0.0,
        max_embedded_l2: 0.0,
        max_original_target_residual_l2: 0.0,
        max_original_target_contraction: 0.0,
    };
    let config = Audit2TransactionalAttemptConfig {
        common_w: Audit2MatrixFreeCommonWConfig::default(),
        outer_atol: 1.0e-30,
        outer_rtol: 0.0,
        commit_rule: Audit2TransactionCommitRule::Causal,
        reuse_certificate: false,
    };
    let mut cache = Audit2ReusablePreconditionerCache::default();
    let mut work = WorkCounters::default();
    let outcome = run_audit2_reusable_preconditioner_transactional_attempt(
        &context,
        &zero_trial(context.coeffs.stages(), problem.dimension),
        &config,
        &budget,
        &reference,
        &mut cache,
        frozen_w_identity('4'),
        diagonal_identity("rejection-diagonal", 1, &[0.5; 6]),
        |_, _| {
            Ok(Arc::new(DiagonalPreconditioner {
                inverse: vec![0.5; 6],
            }) as Arc<dyn Preconditioner>)
        },
        &mut work,
    );
    let completed = match outcome {
        Audit2TransactionalAttemptOutcome::Completed(value) => *value,
        Audit2TransactionalAttemptOutcome::Failed(failure) => {
            panic!("bounded rejection unexpectedly failed: {failure:?}")
        }
    };
    assert_eq!(completed.selection, Audit2TransactionalSelection::Rejected);
    assert!(!completed.committed);
    assert_eq!(completed.committed_state, context.y);
    assert!(completed.selected_step.is_none());
    let fallback = completed.fallback_step.expect("rejected fallback receipt");
    assert!(!fallback.accepted);
    assert_ne!(fallback.y_new, context.y);
    assert_eq!(cache.snapshot().commits, 0);
    assert_eq!(cache.snapshot().rollbacks, 1);
    assert_eq!(work.accepted_steps, 0);
    assert_eq!(work.rejected_steps, 1);
    assert!(work.linear_solves > 0);
    Ok(())
}

/// One transactional attempt on the 8-dimensional manufactured problem with
/// the permissive linear-probe budget, against `reference_state`.
fn causal_probe_attempt(
    reference_state: Vec<f64>,
    max_embedded_l2: f64,
    commit_rule: Audit2TransactionCommitRule,
) -> CoreResult<(
    rodas5p_integrators::Audit2TransactionalAttemptSuccess,
    Audit2ReusablePreconditionerCache,
)> {
    let (problem, y0) = manufactured_vector_problem(8, 80.0, 0.0, 0.2, 0.0)?;
    let context =
        build_step_context_matrix_free(&problem, 0.0, &y0, 1.0e-3, &mut WorkCounters::default())?;
    let reference = Audit2ExternalOutputReference {
        source: "manufactured-exact-v1".into(),
        state: reference_state,
        uncertainty_l2: 0.0,
        uncertainty_treatment: Audit2ReferenceUncertaintyTreatment::DeclaredUpperBound,
    };
    let budget = Audit2IndependentStepBudget {
        identifier: "causal-commit-probe-v1".into(),
        output_atol_l2: 1.0,
        output_rtol: 0.0,
        max_embedded_l2,
        max_original_target_residual_l2: 1.0e-8,
        max_original_target_contraction: 1.0e-8,
    };
    let config = Audit2TransactionalAttemptConfig {
        common_w: Audit2MatrixFreeCommonWConfig::default(),
        outer_atol: 1.0e-9,
        outer_rtol: 1.0e-7,
        commit_rule,
        reuse_certificate: false,
    };
    let mut cache = Audit2ReusablePreconditionerCache::default();
    let outcome = run_audit2_reusable_preconditioner_transactional_attempt(
        &context,
        &zero_trial(context.coeffs.stages(), problem.dimension),
        &config,
        &budget,
        &reference,
        &mut cache,
        frozen_w_identity('c'),
        diagonal_identity("causal-probe-diagonal", 1, &[0.5; 8]),
        |frozen, _| {
            Ok(Arc::new(DiagonalPreconditioner {
                inverse: vec![0.5; frozen.problem.dimension],
            }) as Arc<dyn Preconditioner>)
        },
        &mut WorkCounters::default(),
    );
    match outcome {
        Audit2TransactionalAttemptOutcome::Completed(value) => Ok((*value, cache)),
        Audit2TransactionalAttemptOutcome::Failed(failure) => {
            panic!("causal probe unexpectedly failed: {failure:?}")
        }
    }
}

#[test]
fn transaction_commit_is_invariant_to_reference_state() -> CoreResult<()> {
    // Audit F-014: the commit decision must not read the reference. A
    // reference displaced by 1e3 x the output budget changes only the
    // recorded reference verdict.
    let (problem, _) = manufactured_vector_problem(8, 80.0, 0.0, 0.2, 0.0)?;
    let exact = problem.exact(1.0e-3).unwrap();
    let displaced = exact.iter().map(|value| value + 1.0e3).collect::<Vec<_>>();
    let (near, near_cache) = causal_probe_attempt(exact, 1.0, Audit2TransactionCommitRule::Causal)?;
    let (far, far_cache) =
        causal_probe_attempt(displaced, 1.0, Audit2TransactionCommitRule::Causal)?;

    assert_eq!(near.selection, far.selection);
    assert_eq!(near.committed, far.committed);
    assert_eq!(
        near.committed_state
            .iter()
            .map(|v| v.to_bits())
            .collect::<Vec<_>>(),
        far.committed_state
            .iter()
            .map(|v| v.to_bits())
            .collect::<Vec<_>>()
    );
    assert_eq!(near_cache.snapshot().commits, far_cache.snapshot().commits);
    let near_budget = &near.candidate.as_ref().unwrap().budget;
    let far_budget = &far.candidate.as_ref().unwrap().budget;
    assert_eq!(near_budget.accepted, far_budget.accepted);
    assert!(near_budget.output_accepted);
    assert!(!far_budget.output_accepted);
    assert_eq!(
        far_budget.commit_rule,
        rodas5p_integrators::AUDIT2_TRANSACTION_COMMIT_RULE_CAUSAL
    );
    Ok(())
}

#[test]
fn an_embedded_estimate_above_budget_is_not_committed_with_an_exact_reference() -> CoreResult<()> {
    let (problem, _) = manufactured_vector_problem(8, 80.0, 0.0, 0.2, 0.0)?;
    let exact = problem.exact(1.0e-3).unwrap();
    let (completed, cache) = causal_probe_attempt(exact, 0.0, Audit2TransactionCommitRule::Causal)?;
    let budget = &completed.candidate.as_ref().unwrap().budget;
    assert!(
        budget.output_accepted,
        "the reference verdict is favourable"
    );
    assert!(!budget.embedded_accepted);
    assert!(!budget.accepted);
    assert_eq!(
        completed.selection,
        Audit2TransactionalSelection::ProtectedFallback
    );
    assert_eq!(cache.snapshot().commits, 0);
    Ok(())
}

#[test]
fn reference_gated_rule_is_explicit_and_keeps_the_frozen_receipt_schema() -> CoreResult<()> {
    // The pre-F-014 rule survives only as an explicit mode, for frozen
    // research scenarios; its receipts omit `commit_rule` so their field set
    // is the one the frozen verifier expects.
    let (problem, _) = manufactured_vector_problem(8, 80.0, 0.0, 0.2, 0.0)?;
    let displaced = problem
        .exact(1.0e-3)
        .unwrap()
        .iter()
        .map(|value| value + 1.0e3)
        .collect::<Vec<_>>();
    let (gated, cache) =
        causal_probe_attempt(displaced, 1.0, Audit2TransactionCommitRule::ReferenceGated)?;
    assert_eq!(
        gated.selection,
        Audit2TransactionalSelection::ProtectedFallback
    );
    assert_eq!(cache.snapshot().commits, 0);
    let budget = &gated.candidate.as_ref().unwrap().budget;
    assert_eq!(
        budget.commit_rule,
        rodas5p_integrators::AUDIT2_TRANSACTION_COMMIT_RULE_REFERENCE_GATED
    );
    let json = serde_json::to_value(budget).unwrap();
    assert!(json.get("commit_rule").is_none());
    let legacy: rodas5p_integrators::Audit2IndependentBudgetReceipt =
        serde_json::from_value(json).unwrap();
    assert_eq!(legacy.commit_rule, budget.commit_rule);
    Ok(())
}

#[test]
fn reuse_certificate_evaluates_the_preconditioner_defect() -> CoreResult<()> {
    // Audit F-062: epsilon = ||I - P W||_1. W = [[2, 1], [0, 4]].
    let w = rodas5p_core::DenseOperator::new(rodas5p_core::DenseMatrix::from_rows(&[
        &[2.0, 1.0],
        &[0.0, 4.0],
    ])?)?;
    let exact_diagonal = DiagonalPreconditioner {
        inverse: vec![0.5, 0.25],
    };
    let certificate =
        rodas5p_integrators::audit2_preconditioner_reuse_certificate(&w, &exact_diagonal, true)?;
    // I - P W = [[0, -0.5], [0, 0]].
    assert_eq!(certificate.epsilon_l1, 0.5);
    assert_eq!(certificate.delta_w_l1, 0.0);
    assert_eq!(certificate.bound, 0.5);
    assert!(certificate.certified);
    assert_eq!(certificate.preconditioner_l1, 0.5);
    assert_eq!(certificate.work.diagnostic_matvecs, 2);

    let poor_diagonal = DiagonalPreconditioner {
        inverse: vec![1.5, 1.0],
    };
    let certificate =
        rodas5p_integrators::audit2_preconditioner_reuse_certificate(&w, &poor_diagonal, true)?;
    // I - P W = [[-2, -1.5], [0, -3]]: column sums 2 and 4.5.
    assert_eq!(certificate.epsilon_l1, 4.5);
    assert!(!certificate.certified);
    Ok(())
}

#[test]
fn transactional_attempts_report_the_reuse_certificate_on_request() -> CoreResult<()> {
    let (problem, y0) = manufactured_vector_problem(8, 80.0, 0.0, 0.2, 0.0)?;
    let context =
        build_step_context_matrix_free(&problem, 0.0, &y0, 1.0e-3, &mut WorkCounters::default())?;
    let reference = Audit2ExternalOutputReference {
        source: "manufactured-exact-v1".into(),
        state: problem.exact(context.t + context.h).unwrap(),
        uncertainty_l2: 0.0,
        uncertainty_treatment: Audit2ReferenceUncertaintyTreatment::DeclaredUpperBound,
    };
    let budget = Audit2IndependentStepBudget {
        identifier: "reuse-certificate-probe-v1".into(),
        output_atol_l2: 1.0,
        output_rtol: 0.0,
        max_embedded_l2: 1.0,
        max_original_target_residual_l2: 1.0e-8,
        max_original_target_contraction: 1.0e-8,
    };
    let mut cache = Audit2ReusablePreconditionerCache::default();
    let mut run = |reuse_certificate: bool| {
        let config = Audit2TransactionalAttemptConfig {
            common_w: Audit2MatrixFreeCommonWConfig::default(),
            outer_atol: 1.0e-9,
            outer_rtol: 1.0e-7,
            commit_rule: Audit2TransactionCommitRule::Causal,
            reuse_certificate,
        };
        match run_audit2_reusable_preconditioner_transactional_attempt(
            &context,
            &zero_trial(context.coeffs.stages(), problem.dimension),
            &config,
            &budget,
            &reference,
            &mut cache,
            frozen_w_identity('d'),
            diagonal_identity("reuse-certificate-diagonal", 1, &[0.5; 8]),
            |frozen, _| {
                Ok(Arc::new(DiagonalPreconditioner {
                    inverse: vec![0.5; frozen.problem.dimension],
                }) as Arc<dyn Preconditioner>)
            },
            &mut WorkCounters::default(),
        ) {
            Audit2TransactionalAttemptOutcome::Completed(value) => *value,
            Audit2TransactionalAttemptOutcome::Failed(failure) => {
                panic!("reuse-certificate attempt failed: {failure:?}")
            }
        }
    };
    let first = run(true);
    let first_certificate = first.reuse_certificate.as_ref().unwrap();
    assert!(
        !first_certificate.reused,
        "fresh setup on the first attempt"
    );
    assert!(first.committed);
    let second = run(true);
    let second_certificate = second.reuse_certificate.as_ref().unwrap();
    assert!(second_certificate.reused, "same binding is reused");
    assert_eq!(second_certificate.delta_w_l1, 0.0);
    assert_eq!(
        second_certificate.epsilon_l1.to_bits(),
        first_certificate.epsilon_l1.to_bits()
    );
    assert!(second_certificate.epsilon_l1.is_finite());
    assert_eq!(second_certificate.certified, second_certificate.bound < 1.0);
    // The certificate's work is its own: an attempt without it charges the
    // same attempt work.
    let third = run(false);
    assert!(third.reuse_certificate.is_none());
    assert_eq!(third.work, second.work);
    Ok(())
}
