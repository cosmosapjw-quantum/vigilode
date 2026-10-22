//! Adversarial API regressions: a rejected factor update must not expose
//! the previous factors, and diagnostic preconditioner calls must return
//! typed shape errors rather than panic or silently accept extra entries.

use std::sync::Arc;

use rodas5p_core::{CoreError, WorkCounters};
use rodas5p_integrators::StageLinearSystem;

#[path = "../examples/lrs_photon_fv_support/mod.rs"]
mod support;

use support::block_solver::{BlockStageSolver, Capacitance, Mode};
use support::{Boundary, Coordinate, Geometry, Layout, Model};

fn solver(two_level: bool) -> BlockStageSolver {
    let layout = Layout::new(
        vec![-2.0, -1.0, 0.0],
        vec![-1.0, -0.25, 0.5, 1.0],
        Coordinate::Physical,
        Boundary::Outflow,
    )
    .unwrap();
    let model = Arc::new(
        Model::new(
            layout,
            Geometry::Constant { h: 0.5, s: 0.1 },
            2.0,
            None,
            false,
        )
        .unwrap(),
    );
    let mut solver = BlockStageSolver::new(
        model,
        0,
        Mode::Krylov {
            restart: 4,
            max_iterations: 32,
            rtol: 1.0e-10,
            two_level,
        },
        Capacitance::Conservation,
    )
    .unwrap();
    solver.linearize_at(0.0).unwrap();
    solver.factor_with(4.0).unwrap();
    solver
}

#[test]
fn rejected_factor_update_invalidates_all_factor_consumers() {
    for bad_inv in [f64::NAN, f64::INFINITY, 0.0, -1.0] {
        let mut solver = solver(false);
        let n = solver.dimension();
        assert!(matches!(
            solver.factor_with(bad_inv),
            Err(CoreError::InvalidInput(_))
        ));
        let input = vec![1.0; n];
        let mut output = vec![7.0; n];
        assert!(matches!(
            solver.solve_with(&mut output, &mut WorkCounters::default()),
            Err(CoreError::InvalidInput(_))
        ));
        assert_eq!(output, vec![7.0; n]);
        assert!(matches!(
            solver.precondition(&input, &mut output),
            Err(CoreError::InvalidInput(_))
        ));
        assert!(matches!(
            solver.apply_w(&input, &mut output),
            Err(CoreError::InvalidInput(_))
        ));
        assert_eq!(output, vec![7.0; n]);
        // The current linearization remains usable after a rejected shift.
        solver.factor_with(4.0).unwrap();
        solver.precondition(&input, &mut output).unwrap();
        assert!(output.iter().all(|x| x.is_finite()));
    }
}

#[test]
fn preconditioner_rejects_wrong_shapes_without_touching_output() {
    for two_level in [false, true] {
        let solver = solver(two_level);
        let n = solver.dimension();
        for (ni, no) in [(n - 1, n), (n + 1, n), (n, n - 1), (n, n + 1)] {
            let input = vec![1.0; ni];
            let mut output = vec![7.0; no];
            let result = solver.precondition(&input, &mut output);
            assert!(matches!(result, Err(CoreError::Dimension(_))));
            assert_eq!(output, vec![7.0; no]);
        }
    }
}

#[test]
fn krylov_client_preserves_driver_owned_linear_solve_counter() {
    for two_level in [false, true] {
        let mut solver = solver(two_level);
        let mut rhs = vec![1.0; solver.dimension()];
        let mut counters = WorkCounters::default();
        counters.linear_solves = 13;
        solver.solve_with(&mut rhs, &mut counters).unwrap();
        assert_eq!(counters.linear_solves, 13);
        assert!(counters.linear_iterations > 0);
        assert!(counters.preconditioner_apps > 0);
        assert!(rhs.iter().all(|v| v.is_finite()));
    }
}
