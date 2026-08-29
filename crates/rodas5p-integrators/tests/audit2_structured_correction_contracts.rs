//! Actual crate integration test, not a stand-in solver or production activation.
#[path = "support/common_w_target_correction.rs"]
mod candidate;
use candidate::common_w_target_correction;
use rodas5p_core::{LinearSolverConfig, LuFactorization, WorkCounters, inverse, safe_l2};
use rodas5p_integrators::{
    StructuredBlockSystem, build_step_context, manufactured_vector_problem, sequential_stages,
};
#[test]
fn common_w_matches_full_target_correction_without_stage_jacobians() {
    for n in [4, 8, 16] {
        for h in [0.001, 0.01, 0.05, 0.1] {
            let (problem, y0) = manufactured_vector_problem(n, 50.0, 5.0, 0.1, 0.0).unwrap();
            let mut setup = WorkCounters::default();
            let context = build_step_context(&problem, 0.0, &y0, h, &mut setup).unwrap();
            let block = StructuredBlockSystem::new(&context);
            let mut stages =
                sequential_stages(&context, &LinearSolverConfig::default(), None, &mut setup)
                    .unwrap()
                    .stages;
            for (i, row) in stages.iter_mut().enumerate() {
                for (j, v) in row.iter_mut().enumerate() {
                    *v += 1e-5 * ((i * n + j + 1) as f64).sin();
                }
            }
            let snapshot = block
                .nonlinear_remainder_snapshot(&stages, &mut setup)
                .unwrap();
            let residual = block.target_residual(&stages, &mut setup).unwrap();
            let rhs: Vec<_> = residual.iter().flatten().copied().collect();
            let mut oracle = WorkCounters::default();
            let a = block
                .target_jacobian_matrix(&stages, &snapshot, &mut oracle)
                .unwrap();
            let reference = LuFactorization::new(&a).unwrap().solve(&rhs).unwrap();
            let mut work = WorkCounters::default();
            let z: Vec<_> =
                common_w_target_correction(&context, &snapshot.states, &residual, &mut work)
                    .unwrap()
                    .into_iter()
                    .flatten()
                    .collect();
            let anorm = safe_l2(a.as_slice());
            let condition_f = anorm * safe_l2(inverse(&a).unwrap().as_slice());
            let difference = safe_l2(
                &z.iter()
                    .zip(&reference)
                    .map(|(x, y)| x - y)
                    .collect::<Vec<_>>(),
            ) / safe_l2(&reference);
            let backward = |v: &[f64]| {
                safe_l2(
                    &a.matvec(v)
                        .unwrap()
                        .iter()
                        .zip(&rhs)
                        .map(|(x, y)| x - y)
                        .collect::<Vec<_>>(),
                ) / (anorm * safe_l2(v) + safe_l2(&rhs))
            };
            let eta = backward(&z);
            assert!(eta <= 4096.0 * f64::EPSILON);
            assert!(backward(&reference) <= 4096.0 * f64::EPSILON);
            assert!(difference <= 8192.0 * f64::EPSILON * condition_f);
            assert_eq!(work.jacobian_builds, 0);
            assert_eq!(work.direct_factorizations, 1);
            assert_eq!(work.direct_solve_calls, 8);
            assert_eq!(work.jvp_vectors, 14);
            assert!(backward(&z.iter().map(|x| -x).collect::<Vec<_>>()) > 1e-7);
            println!(
                "AUDIT2_CORRECTION {}",
                serde_json::json!({"n":n,"h":h,"state_relative_difference":difference,"backward_error":eta,"condition_f":condition_f,"stage_jacobian_builds":work.jacobian_builds,"oracle_stage_jacobian_builds":oracle.jacobian_builds,"common_w_factorizations":work.direct_factorizations,"common_w_solve_vectors":work.direct_solve_calls,"jvp_vectors":work.jvp_vectors,"production_activation":false})
            );
        }
    }
}
#[test]
fn invalid_shape_fails_before_factorization() {
    let (p, y) = manufactured_vector_problem(4, 50.0, 5.0, 0.1, 0.0).unwrap();
    let c = build_step_context(&p, 0.0, &y, 0.01, &mut WorkCounters::default()).unwrap();
    let mut w = WorkCounters::default();
    assert!(common_w_target_correction(&c, &[], &[], &mut w).is_err());
    assert_eq!(w.direct_factorizations, 0);
}
