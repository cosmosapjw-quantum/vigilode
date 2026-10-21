//! Stats v4 serialization and field-complete owned-record boundary.
//! Public record fields were checked recursively; internal types occur only
//! in compatibility conversion/comparison implementations, never record fields.
//! Source audit command (no matches):
//! `rg -n 'pub .*(rodas5p_core|rodas5p_integrators)::|pub use rodas5p_' crates/vigilode/src`
//! Also manually follow every public field type in solution.rs and records.rs.
use serde::Serialize;
use vigilode::*;

fn same<S: Serialize, M: Serialize>(source: &S, mirror: &M) {
    assert_eq!(
        serde_json::to_string(source).unwrap(),
        serde_json::to_string(mirror).unwrap()
    );
}
fn all_work() -> rodas5p_core::WorkCounters {
    rodas5p_core::WorkCounters {
        rhs_calls: 1,
        rhs_batch_calls: 2,
        rhs_evaluations: 3,
        ft_calls: 4,
        jacobian_builds: 5,
        jvp_calls: 6,
        jvp_vectors: 7,
        mass_matvecs: 8,
        nonlinear_solves: 9,
        nonlinear_iterations: 10,
        nonlinear_residual_evaluations: 11,
        nonlinear_jacobian_evaluations: 12,
        nonlinear_failures: 13,
        linear_solves: 14,
        linear_iterations: 15,
        linear_matvecs: 16,
        preconditioner_apps: 17,
        direct_factorizations: 18,
        direct_solve_calls: 19,
        recycle_projection_calls: 20,
        recycle_same_operator_uses: 21,
        recycle_cross_operator_refreshes: 22,
        recycle_refresh_matvecs: 23,
        recycle_updates: 24,
        recycle_vectors_selected: 25,
        recycle_dropped_vectors: 26,
        harmonic_ritz_solves: 27,
        orthogonalization_inner_products: 28,
        orthogonalization_vector_updates: 29,
        diagnostic_matvecs: 30,
        phi_actions: 31,
        phi_krylov_vectors: 32,
        phi_projected_exponentials: 33,
        phi_restarts: 34,
        phi_dense_oracle_calls: 35,
        block_linear_solves: 36,
        block_linear_iterations: 37,
        block_matvecs: 38,
        block_preconditioner_apps: 39,
        fast_attempts: 40,
        fast_accepts: 41,
        fallback_steps: 42,
        accepted_steps: 43,
        rejected_steps: 44,
        local_error_failures: 45,
        linear_solve_failures: 46,
        nonlinear_solve_failures: 47,
        nonfinite_step_failures: 48,
        forced_stage_solves: 49,
        jacobian_matvecs: 50,
        linear_matvec_vectors: 51,
        preconditioner_vectors: 52,
        merged_unknown_vector_calls: 53,
        phi_weight_underflows: 54,
        poly_block_products: 55,
        poly_vector_products: 56,
        poly_coefficient_setups: 57,
        poly_coefficient_reuses: 58,
        poly_block_allocations: 59,
        poly_fallbacks: 60,
        recycle_update_refreshes: 61,
    }
}

#[test]
fn every_record_preserves_all_nonzero_fields_and_json_order() {
    let zero = rodas5p_core::WorkCounters::default();
    same(&zero, &WorkRecord::from(&zero));
    let work = all_work();
    let populated = serde_json::to_value(work).unwrap();
    assert_eq!(populated.as_object().unwrap().len(), 61);
    assert!(
        populated
            .as_object()
            .unwrap()
            .values()
            .all(|v| v.as_u64().unwrap() > 0)
    );
    same(&work, &WorkRecord::from(&work));
    let linear = rodas5p_core::LinearSolverConfig {
        method: rodas5p_core::LinearMethod::Gcrodr,
        rtol: 0.1,
        atol: 0.2,
        restart: 3,
        maxiter: 4,
        inner_m: 5,
        outer_k: 6,
        recycle_dim: 7,
        recycle_rank_tol: 0.8,
        preconditioner: rodas5p_core::PreconditionerKind::Direct,
        x0_strategy: rodas5p_core::InitialGuess::PreviousStepScaled,
    };
    same(&linear, &LinearSolverRecord::from(&linear));
    let adaptive = rodas5p_integrators::AdaptiveStepConfig {
        atol: 0.1,
        rtol: 0.2,
        initial_step: 0.3,
        min_step: 0.4,
        max_step: 0.5,
        max_attempts: 6,
        safety: 0.7,
        min_factor: 0.8,
        max_factor: 1.9,
        reject_max_factor: 0.9,
        controller: rodas5p_integrators::ControllerKind::PredictiveCapped2,
        max_step_policy: rodas5p_integrators::MaxStepPolicy::StrictRepresentedCap,
    };
    same(&adaptive, &AdaptiveRecord::from(&adaptive));
    let band = rodas5p_integrators::BandedWork {
        factor_operations: 1,
        solve_operations: 2,
        stored_slots: 3,
    };
    same(&band, &BandedWorkRecord::from(&band));
    let interior = rodas5p_integrators::FastInteriorWork {
        samples: 1,
        interior_rejections: 2,
        rhs_calls: 3,
        back_solves: 4,
        max_estimate: 0.5,
    };
    same(&interior, &FastInteriorRecord::from(&interior));
    let verification = rodas5p_integrators::BandVerification {
        lower: 1,
        upper: 2,
        reference: rodas5p_integrators::BandReference::DenseJacobian,
        vectors: 3,
        seed: 4,
        tolerance: 0.5,
        relative_mismatch: vec![0.1, 0.2, 0.3],
        band_fills: 6,
        band_products: 7,
        jvp_products: 8,
        dense_jacobian_builds: 9,
        dense_products: 10,
        charged: work,
    };
    same(&verification, &BandVerificationRecord::from(&verification));
    let routing = rodas5p_integrators::RoutingRecord {
        declared: rodas5p_integrators::ProblemStructure::Banded { lower: 1, upper: 2 },
        dimension: 3,
        reason: rodas5p_integrators::RoutingReason::DeclaredBand,
        driver: rodas5p_integrators::RoutedDriver::Banded,
        driver_id: "fixture-driver",
        fallback: Some(
            rodas5p_integrators::RoutingFallback::DenseDeclaredAboveSmallMax {
                dimension: 9,
                small_max: 8,
            },
        ),
        verification: Some(verification),
    };
    same(&routing, &RoutingDecisionRecord::from(&routing));
    let mismatch_storage = routing
        .verification
        .as_ref()
        .unwrap()
        .relative_mismatch
        .as_ptr();
    let expected = serde_json::to_string(&routing).unwrap();
    let moved = RoutingDecisionRecord::from(routing);
    assert_eq!(
        moved
            .verification
            .as_ref()
            .unwrap()
            .relative_mismatch
            .as_ptr(),
        mismatch_storage
    );
    assert_eq!(serde_json::to_string(&moved).unwrap(), expected);
}

#[test]
fn every_enum_variant_preserves_its_wire_representation() {
    macro_rules! variants {
        ($source:path, $mirror:ty, [$($variant:ident),+]) => {{
            use $source as Source;
            $(let value=Source::$variant; same(&value,&<$mirror>::from(&value));)+
        }};
    }
    variants!(
        rodas5p_core::LinearMethod,
        LinearMethodRecord,
        [Direct, Gmres, Lgmres, Gcrodr]
    );
    variants!(
        rodas5p_core::PreconditionerKind,
        PreconditionerRecord,
        [None, Jacobi, Direct]
    );
    variants!(
        rodas5p_core::InitialGuess,
        InitialGuessRecord,
        [Zero, Previous, PreviousStep, PreviousStepScaled]
    );
    variants!(
        rodas5p_integrators::ControllerKind,
        ControllerRecord,
        [
            Integral,
            Pi,
            Predictive,
            PredictiveCapped,
            PredictiveCapped2
        ]
    );
    variants!(
        rodas5p_integrators::MaxStepPolicy,
        MaxStepPolicyRecord,
        [StrictRepresentedCap, AllowClockResolutionSlack]
    );
    variants!(
        rodas5p_integrators::AdaptiveFailureKind,
        StepFailureKind,
        [LocalError, LinearSolve, NonlinearSolve, NonFinite]
    );
    variants!(
        rodas5p_integrators::DriverStopReason,
        StopReason,
        [AttemptBudget, MinimumStep]
    );
    variants!(
        rodas5p_integrators::Rodas5pFastLu,
        LuRecord,
        [InPlaceZeroSkipping, InPlaceColumnExtents, Faer, Banded]
    );
    variants!(
        rodas5p_integrators::DenseErrorControl,
        InteriorControlRecord,
        [
            Off,
            ReportDefect,
            Report,
            Enforce,
            VerifiedResidual,
            WFilteredReport,
            WFilteredEnforce
        ]
    );
    variants!(
        rodas5p_integrators::InteriorSampling,
        InteriorSamplingRecord,
        [OutputTimes, EveryStep]
    );
    variants!(
        rodas5p_integrators::BandReference,
        BandReferenceRecord,
        [Jvp, DenseJacobian]
    );
    variants!(
        rodas5p_integrators::RoutingReason,
        RoutingReasonRecord,
        [
            DeclaredBand,
            DeclaredDenseSmall,
            ExplicitJacobian,
            UnstructuredJvp
        ]
    );
    variants!(
        rodas5p_integrators::RoutedDriver,
        RoutedDriverRecord,
        [Banded, Dense, MatrixFree]
    );
    for value in [
        rodas5p_integrators::ProblemStructure::Unstructured,
        rodas5p_integrators::ProblemStructure::Dense,
        rodas5p_integrators::ProblemStructure::Banded { lower: 2, upper: 1 },
        rodas5p_integrators::ProblemStructure::LinearPart { nnz: 3 },
    ] {
        same(&value, &StructureRecord::from(&value));
    }
    let fallback = rodas5p_integrators::RoutingFallback::DenseDeclaredAboveSmallMax {
        dimension: 9,
        small_max: 8,
    };
    same(&fallback, &RoutingFallbackRecord::from(&fallback));
}

#[test]
fn dense_event_stats_match_the_pre_mirror_snapshot() {
    let problem = Problem::new(3, |_t, y, dy| {
        dy[0] = -0.04 * y[0] + 1e4 * y[1] * y[2];
        dy[1] = 0.04 * y[0] - 1e4 * y[1] * y[2] - 3e7 * y[1] * y[1];
        dy[2] = 3e7 * y[1] * y[1];
        Ok::<(), CallbackError>(())
    })
    .jacobian(Jacobian::dense(|_t, y, j| {
        j.copy_from_slice(&[
            -0.04,
            1e4 * y[2],
            1e4 * y[1],
            0.04,
            -1e4 * y[2] - 6e7 * y[1],
            -1e4 * y[1],
            0.0,
            6e7 * y[1],
            0.0,
        ]);
        Ok(())
    }))
    .autonomous()
    .event(|_t, y| y[0] - 0.9, Direction::Down);
    let sol = Rodas5p::solve(
        &problem,
        (0.0, 40.0),
        &[1.0, 0.0, 0.0],
        &Options {
            dense_output: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(sol.is_success());
    assert!(!sol.events.is_empty());
    assert_eq!(
        serde_json::to_string_pretty(&sol.stats).unwrap(),
        include_str!("golden/robertson_dense_event_stats.json").trim_end_matches('\n')
    );
}
