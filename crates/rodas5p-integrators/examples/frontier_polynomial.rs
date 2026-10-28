//! Deterministic correctness/work campaign; no wall-clock speed claims.
use rodas5p_integrators::polynomial_resolvent::{
    DeclaredPolynomialResolvent as Plan, PolynomialResolventWorkspace as Workspace,
};
use serde_json::json;

fn main() {
    let cases = [
        (
            "contractive_diagonal",
            vec![-0.25, 0.0, 0.0, -0.25],
            vec![0.25, 1.0],
            0.25,
        ),
        (
            "stiff_diagonal",
            vec![-1000.0, 0.0, 0.0, -1000.0],
            vec![1000.0, 1.0],
            1000.0,
        ),
        (
            "nonnormal_jordan",
            vec![-2.0, 4.0, 0.0, -2.0],
            vec![4.0, 4.0, 1.0],
            6.0,
        ),
        (
            "rapid_rotation",
            vec![-1.0, -100.0, 100.0, -1.0],
            vec![10001.0, 2.0, 1.0],
            101.0,
        ),
    ];
    let mut rows = Vec::new();
    for (name, m, q, norm) in cases {
        let rhs = [1.0, 2.0];
        let det = (1.0 - m[0]) * (1.0 - m[3]) - m[1] * m[2];
        let exact = [
            ((1.0 - m[3]) * rhs[0] + m[1] * rhs[1]) / det,
            (m[2] * rhs[0] + (1.0 - m[0]) * rhs[1]) / det,
        ];
        let apply = |x: &[f64], y: &mut [f64]| {
            y[0] = m[0] * x[0] + m[1] * x[1];
            y[1] = m[2] * x[0] + m[3] * x[1];
            Ok(())
        };
        let plan = Plan::new(&q, 0.0, norm, 1e-12).unwrap();
        let mut out = [0.0; 2];
        let report = plan
            .apply_checked(&rhs, &mut out, &mut Workspace::new(2), 1e-11, 0.0, apply)
            .unwrap();
        let error = (out[0] - exact[0]).abs().max((out[1] - exact[1]).abs());
        rows.push(json!({"case":name,"candidate":"declared_quotient","accepted":report.residual_accepted,"absolute_error":error,"residual":report.original_residual_norm,"operator_actions":report.candidate_operator_actions+report.residual_operator_actions,"declared_degree":plan.degree()}));
        // Same linear system and absolute residual target; recurrence is allowed
        // 64 corrections. Non-contractive cases are deliberately retained.
        let mut x = [0.0; 2];
        let mut bx = [0.0; 2];
        let mut calls = 0;
        let mut accepted = false;
        let mut residual = f64::INFINITY;
        for _ in 0..64 {
            apply(&x, &mut bx).unwrap();
            calls += 1;
            residual = (rhs[0] - x[0] + bx[0])
                .abs()
                .max((rhs[1] - x[1] + bx[1]).abs());
            if residual.is_finite() && residual <= 1e-11 {
                accepted = true;
                break;
            }
            x = [rhs[0] + bx[0], rhs[1] + bx[1]];
            if x.iter().any(|v| !v.is_finite()) {
                break;
            }
        }
        if !accepted && x.iter().all(|v| v.is_finite()) {
            apply(&x, &mut bx).unwrap();
            calls += 1;
            residual = (rhs[0] - x[0] + bx[0])
                .abs()
                .max((rhs[1] - x[1] + bx[1]).abs());
            accepted = residual.is_finite() && residual <= 1e-11;
        }
        let error = (x[0] - exact[0]).abs().max((x[1] - exact[1]).abs());
        rows.push(json!({"case":name,"candidate":"neumann","accepted":accepted,"absolute_error":if error.is_finite(){Some(error)}else{None},"residual":if residual.is_finite(){Some(residual)}else{None},"operator_actions":calls,"declared_contraction":norm<1.0}));
        // Specialized direct 2x2 reference remains a legitimate faster kernel;
        // zero callback actions excludes its explicit-matrix construction cost.
        rows.push(json!({"case":name,"candidate":"specialized_direct_2x2","accepted":true,"absolute_error":0.0,"operator_actions":0,"scope":"explicit known 2x2 block only; no general dense speed claim"}));
    }
    println!("{}",serde_json::to_string_pretty(&json!({"schema":"vigilode.frontier-polynomial.v1","measurement":"deterministic work/correctness, not timing","rows":rows})).unwrap());
}
