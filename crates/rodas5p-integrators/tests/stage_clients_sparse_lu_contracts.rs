//! Contracts of the sparse direct stage client (loop node V11, part a; checks S1, S2 and S4
//! of `research/production_loop_20261116/battery_v4/PREDECLARED.json`).
//!
//! * S1: two runs with fresh clients are equal bit for bit (outputs, steps, counters);
//! * S2: against the dense reference client on small sizes the step counts are equal and
//!   `max |y_sparse - y_dense| / (atol + rtol |y_dense|) <= 1e-6` (the bound of this node);
//! * S4: construction refuses a pattern without a diagonal entry, non-finite Jacobian values
//!   and singular stage matrices surface as errors the driver rejects.

use std::f64::consts::PI;
use std::sync::Arc;

use rodas5p_core::{CoreError, WorkCounters};
use rodas5p_integrators::mol::{
    self, Boundary, Brusselator, CartesianGrid, CartesianLaplacian, GrayScott, MolOperator,
    SphereGrid, SphereLaplacian,
};
use rodas5p_integrators::stage_clients::DenseLuStageClient;
use rodas5p_integrators::stage_clients::sparse_lu::SparseLuStageClient;
use rodas5p_integrators::{
    AdaptiveStepConfig, OutputSchedule, Rodas5pFastResult, StageLinearSystem,
    integrate_rodas5p_fast_client_observed_with_atol,
};
use serde_json::{Value, json};

fn qual(test: &str, data: Value) {
    println!("V11_QUAL {}", json!({"test": test, "data": data}));
}

const TOL: f64 = 1e-6;

fn config() -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: TOL,
        rtol: TOL,
        initial_step: 1e-6,
        min_step: 1e-14,
        max_step: 10.0,
        max_attempts: 1_000_000,
        ..AdaptiveStepConfig::default()
    }
}

fn run(
    op: &Arc<dyn MolOperator>,
    client: Box<dyn StageLinearSystem>,
    span: (f64, f64),
    y0: &[f64],
) -> Rodas5pFastResult {
    let problem = mol::ode_problem(op.clone()).unwrap();
    let times: Vec<f64> = (0..5)
        .map(|k| span.0 + (span.1 - span.0) * k as f64 / 4.0)
        .collect();
    let output = OutputSchedule::new(times).unwrap();
    let atol = vec![TOL; op.dimension()];
    let (result, partial, _) = integrate_rodas5p_fast_client_observed_with_atol(
        &problem,
        client,
        span,
        y0,
        &config(),
        &output,
        &atol,
    )
    .unwrap();
    assert!(result.observed.success && partial.is_none(), "{}", result.observed.message);
    result
}

struct Case {
    name: &'static str,
    op: Arc<dyn MolOperator>,
    span: (f64, f64),
    y0: Vec<f64>,
}

fn cases() -> Vec<Case> {
    let heat = CartesianLaplacian::line(40, 1.0, 1.0, Boundary::DirichletZero).unwrap();
    let heat_y0: Vec<f64> = (0..40)
        .map(|i| {
            let x = (i as f64 + 0.5) / 40.0;
            (PI * x).sin() + 0.3 * (3.0 * PI * x).sin()
        })
        .collect();
    let grid = CartesianGrid::new(&[8, 8], &[1.0, 1.0], &[Boundary::Periodic; 2]).unwrap();
    let bruss = Brusselator::new(grid.clone(), 1.0, 3.4, 0.02, 0.02).unwrap();
    let bruss_y0: Vec<f64> = (0..64)
        .flat_map(|c| {
            let x = grid.centre(c);
            [1.0 + 0.5 * (2.0 * PI * x[0]).sin(), 3.0 + 0.2 * (2.0 * PI * x[1]).cos()]
        })
        .collect();
    let ngrid = CartesianGrid::new(&[8, 8], &[1.0, 1.0], &[Boundary::NeumannZero; 2]).unwrap();
    let gs = GrayScott::new(ngrid.clone(), 2e-3, 1e-3, 0.04, 0.06).unwrap();
    let gs_y0: Vec<f64> = (0..64)
        .flat_map(|c| {
            let x = ngrid.centre(c);
            let r2 = (x[0] - 0.5).powi(2) + (x[1] - 0.5).powi(2);
            let bump = 0.25 * (-20.0 * r2).exp();
            [1.0 - bump, bump]
        })
        .collect();
    let sgrid = SphereGrid::new(6, 12, 1.0).unwrap();
    let sphere = SphereLaplacian::new(sgrid.clone(), 1.0).unwrap();
    let sphere_y0: Vec<f64> = (0..sgrid.count())
        .map(|c| {
            let (t, p) = sgrid.centre(c);
            1.0 + t.sin() * p.cos() + 1.5 * t.cos().powi(2) - 0.5
        })
        .collect();
    vec![
        Case {
            name: "heat-1D-dirichlet-40",
            op: Arc::new(heat),
            span: (0.0, 0.1),
            y0: heat_y0,
        },
        Case {
            name: "brusselator-2D-periodic-8x8",
            op: Arc::new(bruss),
            span: (0.0, 1.0),
            y0: bruss_y0,
        },
        Case {
            name: "gray-scott-2D-neumann-8x8",
            op: Arc::new(gs),
            span: (0.0, 5.0),
            y0: gs_y0,
        },
        Case {
            name: "sphere-diffusion-6x12",
            op: Arc::new(sphere),
            span: (0.0, 0.5),
            y0: sphere_y0,
        },
    ]
}

#[test]
fn s1_two_runs_are_equal_bit_for_bit() {
    let grid = CartesianGrid::new(&[16, 16], &[1.0, 1.0], &[Boundary::Periodic; 2]).unwrap();
    let op: Arc<dyn MolOperator> = Arc::new(Brusselator::new(grid.clone(), 1.0, 3.4, 2e-3, 2e-3).unwrap());
    let y0: Vec<f64> = (0..grid.count())
        .flat_map(|c| {
            let x = grid.centre(c);
            [1.0 + 0.5 * (2.0 * PI * x[0]).sin(), 3.0]
        })
        .collect();
    let runs: Vec<Rodas5pFastResult> = (0..2)
        .map(|_| {
            let client = Box::new(SparseLuStageClient::for_operator(op.clone()).unwrap());
            run(&op, client, (0.0, 1.0), &y0)
        })
        .collect();
    let (a, b) = (&runs[0], &runs[1]);
    let states_equal = a.observed.y.len() == b.observed.y.len()
        && a.observed.y.iter().zip(&b.observed.y).all(|(p, q)| {
            p.iter()
                .zip(q)
                .all(|(x, y)| x.to_bits() == y.to_bits())
        });
    let equal = states_equal
        && a.observed.t == b.observed.t
        && a.attempts == b.attempts
        && a.accepted_steps == b.accepted_steps
        && a.rejected_steps == b.rejected_steps
        && a.observed.counters == b.observed.counters;
    qual(
        "s1_sparse_lu_bit_reproducibility",
        json!({"problem": op.name(), "n": op.dimension(), "accepted": a.accepted_steps,
               "rejected": a.rejected_steps, "attempts": a.attempts,
               "direct_factorizations": a.observed.counters.direct_factorizations,
               "direct_solve_calls": a.observed.counters.direct_solve_calls,
               "states_equal_bitwise": states_equal, "pass": equal}),
    );
    assert!(equal, "two sparse-LU runs differ");
    assert_eq!(
        a.observed.counters.direct_factorizations,
        a.attempts as u64,
        "one numeric factorisation per attempt"
    );
}

#[test]
fn s2_sparse_and_dense_clients_agree_within_the_stated_bound() {
    let mut rows = Vec::new();
    let mut ok = true;
    for case in cases() {
        let problem = mol::ode_problem(case.op.clone()).unwrap();
        let dense =
            Box::new(DenseLuStageClient::for_protected_run(&problem, case.span.0, &case.y0).unwrap());
        let mode = format!("{:?}", dense.mode());
        let d = run(&case.op, dense, case.span, &case.y0);
        let sparse = Box::new(SparseLuStageClient::for_operator(case.op.clone()).unwrap());
        let s = run(&case.op, sparse, case.span, &case.y0);
        let mut worst = 0.0f64;
        let mut max_abs_difference = 0.0f64;
        for (ys, yd) in s.observed.y.iter().zip(&d.observed.y) {
            for (a, b) in ys.iter().zip(yd) {
                worst = worst.max((a - b).abs() / (TOL + TOL * b.abs()));
                max_abs_difference = max_abs_difference.max((a - b).abs());
            }
        }
        let steps_equal =
            s.accepted_steps == d.accepted_steps && s.rejected_steps == d.rejected_steps;
        let row_ok = steps_equal && worst <= 1e-6 && s.observed.y.len() == d.observed.y.len();
        ok &= row_ok;
        rows.push(json!({
            "case": case.name, "n": case.op.dimension(), "dense_mode": mode,
            "accepted": [s.accepted_steps, d.accepted_steps],
            "rejected": [s.rejected_steps, d.rejected_steps],
            "max_scaled_difference": worst, "max_abs_difference": max_abs_difference,
            "bound": 1e-6, "pass": row_ok,
        }));
    }
    qual(
        "s2_sparse_vs_dense",
        json!({"rtol": TOL, "atol": TOL, "rows": rows,
               "rule": "equal accepted and rejected counts and max |ys - yd| / (atol + rtol |yd|) <= 1e-6",
               "pass": ok}),
    );
    assert!(ok, "sparse and dense clients differ beyond the stated bound");
}

#[test]
fn s4_refusals_and_error_kinds() {
    let fill = Arc::new(|_t: f64, _y: &[f64], v: &mut [f64]| {
        v.fill(1.0);
        Ok(())
    });
    // Column 1 lacks its diagonal entry.
    let missing = SparseLuStageClient::new(2, vec![0, 1, 2], vec![0, 0], fill.clone());
    assert!(matches!(missing, Err(CoreError::InvalidInput(_))));
    // Rows out of order are refused by the solver's own check.
    assert!(SparseLuStageClient::new(2, vec![0, 2, 3], vec![1, 0, 1], fill.clone()).is_err());

    let op: Arc<dyn MolOperator> =
        Arc::new(CartesianLaplacian::line(4, 1.0, 1.0, Boundary::Periodic).unwrap());
    let problem = mol::ode_problem(op.clone()).unwrap();
    let pattern = op.structure().pattern().clone();
    let mut counters = WorkCounters::default();

    let nan_fill = Arc::new(|_t: f64, _y: &[f64], v: &mut [f64]| {
        v.fill(f64::NAN);
        Ok(())
    });
    let mut client = SparseLuStageClient::new(
        4,
        pattern.col_ptr().to_vec(),
        pattern.row_idx().to_vec(),
        nan_fill,
    )
    .unwrap();
    let nonfinite = client.linearize(&problem, 0.0, &[0.0; 4], &mut counters);
    assert!(matches!(nonfinite, Err(CoreError::NonFinite(_))));
    let mut b = vec![1.0; 4];
    let unfactored = client.solve_in_place(&mut b, &mut counters);
    assert!(matches!(unfactored, Err(CoreError::LinearSolve(_))));
    assert_eq!(counters.direct_solve_calls, 1, "a failed solve stays charged");

    // J = 2 I and inv_h_gamma = 2 give W = 0 exactly: the factorisation or the solve reports
    // an error the driver rejects.
    let two = Arc::new(|_t: f64, _y: &[f64], v: &mut [f64]| {
        v.fill(2.0);
        Ok(())
    });
    let mut client = SparseLuStageClient::new(4, vec![0, 1, 2, 3, 4], vec![0, 1, 2, 3], two).unwrap();
    client
        .linearize(&problem, 0.0, &[0.0; 4], &mut counters)
        .unwrap();
    let factor = client.factor(2.0, &mut counters);
    let outcome = factor.and_then(|_| client.solve_in_place(&mut b, &mut counters));
    assert!(
        matches!(
            outcome,
            Err(CoreError::LinearSolve(_)) | Err(CoreError::NonFinite(_))
        ),
        "{outcome:?}"
    );
    qual(
        "s4_sparse_lu_errors",
        json!({"missing_diagonal": "InvalidInput", "nonfinite_jacobian": "NonFinite",
               "solve_without_factors": "LinearSolve", "singular_w": format!("{outcome:?}"),
               "pass": true}),
    );
}
