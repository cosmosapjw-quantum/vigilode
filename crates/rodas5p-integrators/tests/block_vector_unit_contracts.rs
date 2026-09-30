//! Block GMRES work in state-vector units (audit F-051).
//!
//! One application of the s*n block operator performs s stage products, and
//! one block-direct preconditioner application performs s LU back-solves.
//! `linear_matvecs` and `preconditioner_apps` stay solver-level application
//! counts; the `*_vectors` counters carry the state-vector units, so block
//! and sequential lanes compare on the same unit.

use rodas5p_core::{DenseMatrix, DenseOperator, DirectPreconditioner, WorkCounters};
use rodas5p_integrators::{
    BlockPreconditioner, StructuredBlockSystem, build_step_context,
    manufactured_mass_nonlinear_problem,
};
use rodas5p_krylov::{GmresConfig, solve_gmres};

#[test]
fn block_gmres_charges_s_state_vectors_per_application() {
    let (problem, y0, _, _) = manufactured_mass_nonlinear_problem(20.0, 1.0, 0.2, 0.0).unwrap();
    for preconditioner in [BlockPreconditioner::Direct, BlockPreconditioner::None] {
        let mut work = WorkCounters::default();
        let context = build_step_context(&problem, 0.0, &y0, 0.01, &mut work).unwrap();
        let system = StructuredBlockSystem::new(&context);
        let s = system.s as u64;
        let rhs = system.rhs_base();
        let before = work;
        let report = system
            .gmres_solve(
                &rhs,
                1.0e-10,
                1.0e-12,
                16,
                64,
                preconditioner,
                None,
                &mut work,
            )
            .unwrap();
        let delta = work.delta(before);
        assert!(delta.linear_matvecs > 0);
        assert_eq!(
            delta.linear_matvec_vectors,
            s * (delta.linear_matvecs + delta.diagnostic_matvecs),
            "{preconditioner:?}"
        );
        assert_eq!(
            delta.preconditioner_vectors,
            s * delta.preconditioner_apps,
            "{preconditioner:?}"
        );
        assert_eq!(report.matvec_vectors, s * report.matvecs);
        assert_eq!(
            report.preconditioner_vectors,
            s * report.preconditioner_apps
        );
        // The stage products themselves were already charged per stage.
        assert_eq!(delta.jacobian_matvecs, delta.linear_matvec_vectors);
    }
}

#[test]
fn a_sequential_application_is_one_state_vector() {
    let matrix = DenseMatrix::from_rows(&[&[4.0, 1.0], &[1.0, 3.0]]).unwrap();
    let operator = DenseOperator::new(matrix.clone()).unwrap();
    let preconditioner = DirectPreconditioner::new(matrix).unwrap();
    let mut work = WorkCounters::default();
    solve_gmres(
        &operator,
        &preconditioner,
        &[1.0, 2.0],
        Some(&[0.5, 0.5]),
        &GmresConfig::default(),
        &mut work,
    )
    .unwrap();
    assert!(work.linear_matvecs > 0 && work.preconditioner_apps > 0);
    assert_eq!(
        work.linear_matvec_vectors,
        work.linear_matvecs + work.diagnostic_matvecs
    );
    assert_eq!(work.preconditioner_vectors, work.preconditioner_apps);
}
