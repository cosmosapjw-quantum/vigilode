use rayon::ThreadPoolBuilder;
use rodas5p_core::{
    ClosureOperator, CoreError, IdentityPreconditioner, LinearOperator, Preconditioner,
    WorkCounters,
};
use rodas5p_krylov::{GmresConfig, solve_gmres};
use std::f64::consts::{PI, TAU};
use vigilode::spatial::{DiffusionGrid, SpatialRows};

fn input(n: usize) -> Vec<f64> {
    (0..n)
        .map(|i| (0.23 * i as f64).sin() + 0.2 * (1.93 * i as f64).cos())
        .collect()
}

#[test]
fn sphere_line_inverse_matches_sparse_direct_and_contracts_area_norm() {
    let reference: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/longitude_inverse.json")).unwrap();
    let nt = reference["n_theta"].as_u64().unwrap() as usize;
    let np = reference["n_phi"].as_u64().unwrap() as usize;
    let sigma = reference["shift"].as_f64().unwrap();
    let k = reference["kappa"].as_f64().unwrap();
    let r = reference["radius"].as_f64().unwrap();
    let g = DiffusionGrid::sphere(nt, np, r, k).unwrap();
    let rhs = input(g.dimension());
    let mut x = vec![0.0; rhs.len()];
    g.longitude_preconditioner(sigma, SpatialRows::Serial)
        .unwrap()
        .apply(&rhs, &mut x)
        .unwrap();
    let mut px = x.clone();
    ThreadPoolBuilder::new()
        .num_threads(3)
        .build()
        .unwrap()
        .install(|| {
            g.longitude_preconditioner(sigma, SpatialRows::Parallel)
                .unwrap()
                .apply(&rhs, &mut px)
                .unwrap();
        });
    assert!(px.iter().zip(&x).all(|(a, b)| a.to_bits() == b.to_bits()));
    let b = PI / nt as f64;
    let p = TAU / np as f64;
    let exact = reference["inverse"].as_array().unwrap();
    assert_eq!(x.len(), exact.len());
    let mut residual = 0.0_f64;
    let mut rhsnorm = 0.0_f64;
    let mut xnorm = 0.0_f64;
    let mut error = 0.0_f64;
    for q in 0..x.len() {
        let (i, j) = (q / np, q % np);
        let area = g.cell_measure(q).unwrap();
        let rate = k * b / (((i as f64 + 0.5) * b).sin() * p * area);
        let lap =
            rate * ((x[i * np + (j + 1) % np] - x[q]) + (x[i * np + (j + np - 1) % np] - x[q]));
        residual += area * (x[q] - sigma * lap - rhs[q]).powi(2);
        rhsnorm += area * rhs[q].powi(2);
        xnorm += area * x[q].powi(2);
        error = error.max((x[q] - exact[q].as_f64().unwrap()).abs());
    }
    assert!(error < 2e-14 && (residual / rhsnorm).sqrt() < 2e-14 && xnorm <= rhsnorm);
    println!(
        "POLAR {}",
        serde_json::json!({"case":"inverse","sparse_direct_error_inf":error,
        "weighted_relative_true_residual":(residual/rhsnorm).sqrt(),"weighted_norm_ratio":(xnorm/rhsnorm).sqrt()})
    );
}

#[test]
fn edge_shifts_wraparound_and_input_validation() {
    let b = DiffusionGrid::periodic_box([3, 4], [1.0; 2], [0.2; 2]).unwrap();
    assert!(
        b.longitude_preconditioner(0.1, SpatialRows::Serial)
            .is_err()
    );
    for np in [2, 3, 4, 9] {
        let nt = 4;
        let g = DiffusionGrid::sphere(nt, np, 1.0, 0.5).unwrap();
        for shift in [-1.0, f64::INFINITY, f64::NAN, f64::MAX] {
            assert!(
                g.longitude_preconditioner(shift, SpatialRows::Serial)
                    .is_err()
            );
        }
        let rhs = input(g.dimension());
        let mut x = vec![0.0; rhs.len()];
        let ident = g
            .longitude_preconditioner(0.0, SpatialRows::Serial)
            .unwrap();
        assert!(ident.apply(&rhs[1..], &mut x).is_err());
        ident.apply(&rhs, &mut x).unwrap();
        assert_eq!(x, rhs);
        // Fourier mode on each ring, including the doubled edge of a two-cell circle.
        let harmonic: Vec<_> = (0..nt * np)
            .map(|q| (TAU * (q % np) as f64 / np as f64).cos())
            .collect();
        let b = PI / nt as f64;
        let p = TAU / np as f64;
        let sigma = 4.0;
        g.longitude_preconditioner(sigma, SpatialRows::Serial)
            .unwrap()
            .apply(&harmonic, &mut x)
            .unwrap();
        for q in 0..x.len() {
            let theta = (q / np) as f64 * b + 0.5 * b;
            let c = sigma * 0.5 * b / (theta.sin() * p * g.cell_measure(q).unwrap());
            let expect = harmonic[q] / (1.0 + 4.0 * c * (PI / np as f64).sin().powi(2));
            assert!((expect - x[q]).abs() < 3e-15);
        }
    }
}

#[test]
fn native_gmres_applies_line_preconditioner_to_the_true_spherical_stage() {
    let g = DiffusionGrid::sphere(12, 24, 1.0, 0.5).unwrap();
    let n = g.dimension();
    let sigma = 0.1;
    let p = g
        .longitude_preconditioner(sigma, SpatialRows::Serial)
        .unwrap();
    let op = ClosureOperator::new(n, |v: &[f64], out: &mut [f64]| {
        g.apply(v, out, SpatialRows::Serial)
            .map_err(|e| CoreError::Dimension(e.to_string()))?;
        for (a, b) in out.iter_mut().zip(v) {
            *a = *b - sigma * (*a);
        }
        Ok(())
    });
    let rhs = input(n);
    let config = GmresConfig {
        restart: 30,
        max_arnoldi: 600,
        rtol: 1e-9,
        atol: 0.0,
    };
    let identity = IdentityPreconditioner::new(n);
    for (name, pc) in [
        ("identity", &identity as &dyn Preconditioner),
        ("longitude", &p),
    ] {
        let mut counters = WorkCounters::default();
        let solution = solve_gmres(&op, pc, &rhs, None, &config, &mut counters).unwrap();
        assert!(solution.converged);
        let mut ax = vec![0.0; n];
        op.apply(&solution.x, &mut ax).unwrap();
        let residual = (rhs
            .iter()
            .zip(&ax)
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            / rhs.iter().map(|x| x * x).sum::<f64>())
        .sqrt();
        assert!(residual <= 1.01e-9);
        if name == "longitude" {
            assert!(counters.preconditioner_apps > 0);
        }
        println!(
            "POLAR {}",
            serde_json::json!({"case":"native_gmres","policy":name,
            "arnoldi_columns":solution.iterations,"matvecs":solution.matvecs,
            "preconditioner_apps":solution.preconditioner_apps,"true_relative_residual":residual})
        );
    }
}
