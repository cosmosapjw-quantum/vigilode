use rodas5p_core::CoreResult;
use rodas5p_integrators::{MatrixFreeCommonWProfile, run_matrix_free_common_w_gate};

#[test]
fn smoke_common_w_gate_is_strictly_jacobian_free_and_deterministic() -> CoreResult<()> {
    let first = run_matrix_free_common_w_gate(MatrixFreeCommonWProfile::Smoke)?;
    let second = run_matrix_free_common_w_gate(MatrixFreeCommonWProfile::Smoke)?;

    assert_eq!(first.schema, "rodas5p-matrix-free-common-w-gate-v1");
    assert_eq!(first.scientific_checksum, second.scientific_checksum);
    assert!(first.strict_jacobian_free);
    assert!(first.explicit_jacobian_builds == 0);
    assert!(first.factorization_builds == 0);
    assert!(
        first
            .rows
            .iter()
            .any(|row| row.solver == "independent-gmres-serial")
    );
    assert!(
        first
            .rows
            .iter()
            .any(|row| row.solver == "independent-gmres-rayon")
    );
    assert!(first.rows.iter().any(|row| row.solver == "block-gmres"));
    assert!(
        first
            .rows
            .iter()
            .any(|row| row.solver == "seeded-shared-gmres")
    );
    assert!(first.rows.iter().filter(|row| row.success).all(|row| {
        row.maximum_relative_residual
            .is_some_and(|value| value <= 2.0e-9)
            && row.maximum_solution_difference_vs_serial <= 2.0e-8
            && row.timing_repetitions >= 3
            && row.timing_batch_iterations > 0
            && row.explicit_jacobian_builds == 0
            && row.factorization_builds == 0
    }));
    Ok(())
}

#[test]
fn block_common_w_rows_use_fewer_block_calls_than_vector_operator_actions() -> CoreResult<()> {
    let report = run_matrix_free_common_w_gate(MatrixFreeCommonWProfile::Smoke)?;
    let block_rows = report
        .rows
        .iter()
        .filter(|row| row.solver == "block-gmres" && row.success)
        .collect::<Vec<_>>();
    assert!(!block_rows.is_empty());
    assert!(block_rows.iter().all(|row| {
        row.block_operator_calls > 0 && row.operator_vectors >= row.block_operator_calls
    }));
    Ok(())
}
