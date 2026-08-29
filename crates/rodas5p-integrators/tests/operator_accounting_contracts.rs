use rodas5p_core::WorkCounters;
use rodas5p_integrators::{
    BlockPreconditioner, StructuredBlockSystem, build_step_context,
    manufactured_mass_nonlinear_problem,
};

#[test]
fn block_gmres_charges_every_stage_jvp_and_mass_action_at_the_operator_site() {
    // Defect caught: the flattened block operator counted one GMRES matvec but
    // hid the s raw Jacobian and mass-vector applications performed inside it.
    let (problem, y0, _, _) = manufactured_mass_nonlinear_problem(20.0, 1.0, 0.2, 0.0).unwrap();
    let mut work = WorkCounters::default();
    let context = build_step_context(&problem, 0.0, &y0, 0.01, &mut work).unwrap();
    let system = StructuredBlockSystem::new(&context);
    let rhs = system.rhs_base();
    let before = work;

    system
        .gmres_solve(
            &rhs,
            1.0e-10,
            1.0e-12,
            16,
            64,
            BlockPreconditioner::None,
            None,
            &mut work,
        )
        .unwrap();

    let delta = work.delta(before);
    let flattened_operator_vectors = delta
        .linear_matvecs
        .saturating_add(delta.diagnostic_matvecs);
    let expected_stage_vectors = (system.s as u64) * flattened_operator_vectors;
    assert_eq!(delta.jvp_calls, expected_stage_vectors);
    assert_eq!(delta.jvp_vectors, expected_stage_vectors);
    assert_eq!(delta.mass_matvecs, expected_stage_vectors);
    assert_eq!(delta.block_matvecs, flattened_operator_vectors);
}
