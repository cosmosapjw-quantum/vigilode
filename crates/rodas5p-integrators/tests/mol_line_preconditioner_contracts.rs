//! Contracts of the line-tridiagonal right preconditioner of `rodas5p_integrators::mol`
//! (loop node V11, part a; check P of
//! `research/production_loop_20261116/battery_v4/PREDECLARED.json`).
//!
//! Matrix-free stage systems `W x = b`, `W = s I - J` with the operator's JVP, solved by
//! right-preconditioned restarted GMRES (`rodas5p_krylov::solve_gmres_into`):
//!
//! * 1D: the lines are the whole operator, so the preconditioner is `W^-1` and GMRES needs at
//!   most two iterations;
//! * 2D anisotropic heat (lines along the strongly coupled axis) and the sphere (longitude
//!   lines): at most half the iterations of the unpreconditioned solve;
//! * every converged solve has a true residual `|b - W x| <= 1e-9 |b|`.

use std::sync::Arc;

use rodas5p_core::{ClosureOperator, IdentityPreconditioner, Preconditioner, WorkCounters};
use rodas5p_integrators::mol::{
    Boundary, CartesianGrid, CartesianLaplacian, LineTridiagonalPreconditioner, MolOperator,
    SphereGrid, SphereLaplacian,
};
use rodas5p_krylov::{GmresCapacity, GmresConfig, GmresWorkspace, solve_gmres_into};
use serde_json::{Value, json};

fn qual(test: &str, data: Value) {
    println!("V11_QUAL {}", json!({"test": test, "data": data}));
}

fn lcg(seed: &mut u64) -> f64 {
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    (*seed >> 11) as f64 / (1u64 << 53) as f64
}

const SHIFT: f64 = 100.0;

/// `(iterations, true relative residual)` of GMRES on `W = s I - J` with `pc`.
fn solve(op: Arc<dyn MolOperator>, pc: &dyn Preconditioner, b: &[f64]) -> (u64, f64) {
    let n = b.len();
    let state = vec![0.0; n];
    let w_op = {
        let op = op.clone();
        let state = state.clone();
        ClosureOperator::new(n, move |v: &[f64], out: &mut [f64]| {
            op.jvp_into(0.0, &state, v, out)?;
            for (o, x) in out.iter_mut().zip(v) {
                *o = SHIFT * x - *o;
            }
            Ok(())
        })
    };
    let config = GmresConfig {
        restart: 40,
        max_arnoldi: 20_000,
        rtol: 1e-10,
        atol: 0.0,
    };
    let mut x = vec![0.0; n];
    let mut workspace = GmresWorkspace::default();
    let mut counters = WorkCounters::default();
    let report = solve_gmres_into(
        &w_op,
        pc,
        b,
        None,
        &config,
        None,
        &mut x,
        &mut workspace,
        GmresCapacity::unbounded(),
        &mut counters,
    )
    .unwrap();
    let mut wx = vec![0.0; n];
    op.jvp_into(0.0, &state, &x, &mut wx).unwrap();
    let mut r2 = 0.0;
    let mut b2 = 0.0;
    for i in 0..n {
        let r = b[i] - (SHIFT * x[i] - wx[i]);
        r2 += r * r;
        b2 += b[i] * b[i];
    }
    (report.iterations, (r2 / b2).sqrt())
}

fn rhs(n: usize, seed: u64) -> Vec<f64> {
    let mut s = seed;
    (0..n).map(|_| lcg(&mut s) - 0.5).collect()
}

#[test]
fn p_line_preconditioner_inverts_1d_and_cuts_iterations_in_2d_and_on_the_sphere() {
    let mut rows = Vec::new();
    let mut ok = true;
    for bc in [
        Boundary::Periodic,
        Boundary::NeumannZero,
        Boundary::DirichletZero,
    ] {
        let op: Arc<dyn MolOperator> =
            Arc::new(CartesianLaplacian::line(64, 1.0, 1.0, bc).unwrap());
        let mut pc = LineTridiagonalPreconditioner::new(op.structure(), 0).unwrap();
        let state = vec![0.0; op.dimension()];
        pc.update(op.as_ref(), 0.0, &state, SHIFT).unwrap();
        let b = rhs(op.dimension(), 11);
        let (iterations, residual) = solve(op.clone(), &pc, &b);
        let row_ok = iterations <= 2 && residual <= 1e-9;
        ok &= row_ok;
        rows.push(json!({"case": format!("1D-{bc:?}"), "n": op.dimension(),
                         "preconditioned_iterations": iterations,
                         "true_relative_residual": residual, "rule": "iterations <= 2", "pass": row_ok}));
    }
    let cases: Vec<(&str, Arc<dyn MolOperator>, usize)> = vec![
        (
            "2D-anisotropic-64x64-D(1,0.01)-PN-lines-x",
            Arc::new(
                CartesianLaplacian::new(
                    CartesianGrid::new(
                        &[64, 64],
                        &[1.0, 1.0],
                        &[Boundary::Periodic, Boundary::NeumannZero],
                    )
                    .unwrap(),
                    &[1.0, 0.01],
                )
                .unwrap(),
            ),
            0,
        ),
        (
            "sphere-32x64-longitude-lines",
            Arc::new(SphereLaplacian::new(SphereGrid::new(32, 64, 1.0).unwrap(), 1.0).unwrap()),
            0,
        ),
    ];
    for (name, op, axis) in cases {
        let n = op.dimension();
        let mut pc = LineTridiagonalPreconditioner::new(op.structure(), axis).unwrap();
        pc.update(op.as_ref(), 0.0, &vec![0.0; n], SHIFT).unwrap();
        let b = rhs(n, 23);
        let (with, residual_with) = solve(op.clone(), &pc, &b);
        let (without, residual_without) = solve(op.clone(), &IdentityPreconditioner::new(n), &b);
        let ratio = with as f64 / without as f64;
        let row_ok = ratio <= 0.5 && residual_with <= 1e-9 && residual_without <= 1e-9;
        ok &= row_ok;
        rows.push(json!({"case": name, "n": n, "preconditioned_iterations": with,
                         "unpreconditioned_iterations": without, "ratio": ratio,
                         "true_relative_residual_preconditioned": residual_with,
                         "true_relative_residual_unpreconditioned": residual_without,
                         "rule": "ratio <= 0.5", "pass": row_ok}));
    }
    qual(
        "p_line_preconditioner",
        json!({"shift": SHIFT, "gmres": {"restart": 40, "rtol": 1e-10}, "rows": rows, "pass": ok}),
    );
    assert!(ok, "a line preconditioner check failed");
}

#[test]
fn line_preconditioner_refuses_missing_axes_and_unfactored_use() {
    let op = CartesianLaplacian::line(8, 1.0, 1.0, Boundary::Periodic).unwrap();
    assert!(LineTridiagonalPreconditioner::new(op.structure(), 1).is_err());
    let pc = LineTridiagonalPreconditioner::new(op.structure(), 0).unwrap();
    let mut y = vec![0.0; 8];
    assert!(pc.apply(&[1.0; 8], &mut y).is_err());
    let mut pc = pc;
    assert!(pc.update(&op, 0.0, &[0.0; 8], f64::NAN).is_err());
    assert!(pc.apply(&[1.0; 8], &mut y).is_err());
    assert!(pc.update(&op, 0.0, &[0.0; 7], 1.0).is_err());
    pc.update(&op, 0.0, &[0.0; 8], 1.0).unwrap();
    assert!(pc.apply(&[1.0; 7], &mut y).is_err());
    pc.apply(&[1.0; 8], &mut y).unwrap();
    // (I - L) 1 = 1 for the periodic Laplacian: the preconditioner is exact in 1D.
    assert!(y.iter().all(|v| (v - 1.0).abs() < 1e-12), "{y:?}");
}
