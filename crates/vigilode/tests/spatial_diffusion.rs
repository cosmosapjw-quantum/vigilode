//! Public MOL adapter contracts at the operator and solver boundaries.
use rayon::ThreadPoolBuilder;
use std::f64::consts::{PI, TAU};
use vigilode::spatial::{DiffusionGrid, SpatialRows};
use vigilode::{Jacobian, SolverStrategy, Status, VigilODE, VigilodeOptions};

fn apply(g: &DiffusionGrid, u: &[f64], rows: SpatialRows) -> Vec<f64> {
    let mut out = vec![f64::NAN; g.dimension()];
    g.apply(u, &mut out, rows).unwrap();
    out
}

fn test_grids() -> Vec<(&'static str, DiffusionGrid, Vec<f64>, f64)> {
    let a = DiffusionGrid::periodic_box([12, 8], [2.0, 1.0], [0.2, 0.08]).unwrap();
    let u2 = (0..a.dimension())
        .map(|i| (TAU * ((i % 12) as f64 / 12.0 + (i / 12) as f64 / 8.0)).cos())
        .collect();
    let l2 =
        -4.0 * (0.2 * 36.0 * (PI / 12.0).sin().powi(2) + 0.08 * 64.0 * (PI / 8.0).sin().powi(2));
    let b = DiffusionGrid::periodic_box([6, 7, 8], [1.0, 2.0, 3.0], [0.1, 0.2, 0.3]).unwrap();
    let u3 = (0..b.dimension())
        .map(|i| {
            (TAU * ((i % 6) as f64 / 6.0 + (i / 6 % 7) as f64 / 7.0 + (i / 42) as f64 / 8.0)).cos()
        })
        .collect();
    let l3 = -4.0
        * (0.1 * 36.0 * (PI / 6.0).sin().powi(2)
            + 0.2 * 12.25 * (PI / 7.0).sin().powi(2)
            + 0.3 * (8.0_f64 / 3.0).powi(2) * (PI / 8.0).sin().powi(2));
    let s = DiffusionGrid::sphere(10, 20, 1.3, 0.15).unwrap();
    let dt = PI / 10.0;
    let us = (0..s.dimension())
        .map(|i| ((i / 20) as f64 * dt + dt / 2.0).cos())
        .collect();
    let ls = -2.0 * 0.15 / 1.3_f64.powi(2) * dt.sin() / dt;
    vec![
        ("box2", a, u2, l2),
        ("box3", b, u3, l3),
        ("sphere", s, us, ls),
    ]
}

#[test]
fn checked_grid_construction_and_buffers() {
    assert!(DiffusionGrid::periodic_box([3], [1.0], [1.0]).is_err());
    assert!(DiffusionGrid::periodic_box([1, 2], [1.0, 2.0], [1.0, 1.0]).is_err());
    assert!(DiffusionGrid::periodic_box([3, 2], [f64::NAN, 2.0], [1.0, 1.0]).is_err());
    assert!(DiffusionGrid::sphere(usize::MAX, 2, 1.0, 1.0).is_err());
    assert!(DiffusionGrid::sphere(3, 4, 0.0, 1.0).is_err());
    let g = DiffusionGrid::sphere(2, 2, 1.0, 0.0).unwrap();
    assert!(g.cell_measure(4).is_none());
    assert!(
        g.apply(&[1.0; 3], &mut [0.0; 4], SpatialRows::Serial)
            .is_err()
    );
    assert_eq!(
        apply(&g, &[0.0, 2.0, -1.0, 4.0], SpatialRows::Serial),
        [0.0; 4]
    );
}

#[test]
fn arbitrary_fields_conserve_weighted_mass_and_dissipate_energy() {
    let pool = ThreadPoolBuilder::new().num_threads(3).build().unwrap();
    for (name, g, _u, _lambda) in test_grids() {
        let u: Vec<_> = (0..g.dimension())
            .map(|i| ((i + 11) as f64 * 1.313).sin())
            .collect();
        let v: Vec<_> = (0..g.dimension())
            .map(|i| ((i + 3) as f64 * 2.17).cos())
            .collect();
        let lu = apply(&g, &u, SpatialRows::Serial);
        let lu_par = pool.install(|| apply(&g, &u, SpatialRows::Parallel));
        assert!(
            lu.iter()
                .zip(&lu_par)
                .all(|(a, b)| a.to_bits() == b.to_bits())
        );
        let lv = apply(&g, &v, SpatialRows::Serial);
        let mut mass = 0.0;
        let mut energy = 0.0;
        let mut symmetry = 0.0;
        let mut scale = 0.0;
        for i in 0..g.dimension() {
            let w = g.cell_measure(i).unwrap();
            energy += w * u[i] * lu[i];
            mass += w * lu[i];
            symmetry += w * (u[i] * lv[i] - v[i] * lu[i]);
            scale += w * (lu[i].abs() + lv[i].abs());
        }
        assert!(mass.abs() <= 1e-13 * scale && symmetry.abs() <= 1e-13 * scale && energy < 0.0);
        if name == "sphere" {
            let area: f64 = (0..g.dimension()).map(|i| g.cell_measure(i).unwrap()).sum();
            assert!((area - 4.0 * PI * 1.3_f64.powi(2)).abs() < 1e-12 * area);
        }
    }
}

#[test]
fn unified_solver_tracks_semidiscrete_spectrum_without_assembling_a_jacobian() {
    let pool = ThreadPoolBuilder::new().num_threads(3).build().unwrap();
    for (name, g, u, lambda) in test_grids() {
        let lu = apply(&g, &u, SpatialRows::Serial);
        let defect = lu
            .iter()
            .zip(&u)
            .map(|(x, y)| (x - lambda * y).abs())
            .fold(0.0_f64, f64::max);
        assert!(defect < 2e-12);
        let tf = 0.12;
        let opts = VigilodeOptions::default()
            .strategy(SolverStrategy::Auto)
            .rtol(1e-7)
            .atol(1e-10);
        let s = pool
            .install(|| VigilODE::solve(&g.problem(SpatialRows::Parallel), (0.0, tf), &u, opts))
            .unwrap();
        assert_eq!(s.status, Status::Success);
        let work = &s.stats().counters;
        assert!(work.jvp_vectors > 0 && work.direct_factorizations == 0);
        let err = s
            .final_state()
            .unwrap()
            .iter()
            .zip(&u)
            .map(|(x, y)| (x - (lambda * tf).exp() * y).abs())
            .fold(0.0_f64, f64::max);
        assert!(err < 3e-7, "{name}: {err}");
        println!(
            "SPATIAL {}",
            serde_json::json!({"grid":name,"n":u.len(),"semidiscrete_eigenvalue":lambda,
               "eigenvector_defect":defect,"endpoint_max_error":err,"jvp_vectors":work.jvp_vectors,
               "direct_factorizations":work.direct_factorizations,"accepted_steps":s.stats().accepted_steps,"routes":s.route_history})
        );
    }
}

#[test]
fn dense_output_on_small_mesh_has_an_explicit_rhs_only_fallback() {
    use vigilode::{Options, Rodas5p, SolveError, Unsupported};
    let g = DiffusionGrid::periodic_box([3, 3], [1.0; 2], [0.1; 2]).unwrap();
    let p = g.problem(SpatialRows::Serial);
    let opts = Options {
        dense_output: true,
        ..Options::default()
    };
    let y = [1.0; 9];
    assert!(matches!(
        Rodas5p::solve(&p, (0.0, 0.2), &y, &opts),
        Err(SolveError::Unsupported(
            Unsupported::JvpWithMassOrDenseOutput
        ))
    ));
    let dense = p.jacobian(Jacobian::FiniteDifference);
    let s = Rodas5p::solve(&dense, (0.0, 0.2), &y, &opts).unwrap();
    assert!(s.is_success() && s.stats.counters.direct_factorizations > 0);
    assert_eq!(s.at(0.1).unwrap(), y);
}

#[test]
fn nonzonal_spherical_harmonic_refines_in_the_area_norm() {
    let mut prev: Option<f64> = None;
    for nt in [12, 24, 48] {
        let np = 2 * nt;
        let g = DiffusionGrid::sphere(nt, np, 1.7, 0.4).unwrap();
        let lambda = -0.8 / 1.7_f64.powi(2);
        let u: Vec<_> = (0..g.dimension())
            .map(|q| {
                let theta = PI * ((q / np) as f64 + 0.5) / nt as f64;
                let phi = TAU * ((q % np) as f64 + 0.5) / np as f64;
                theta.sin() * phi.cos()
            })
            .collect();
        let lu = apply(&g, &u, SpatialRows::Serial);
        let (mut err2, mut norm2) = (0.0, 0.0);
        for q in 0..g.dimension() {
            let w = g.cell_measure(q).unwrap();
            err2 += w * (lu[q] - lambda * u[q]).powi(2);
            norm2 += w * (lambda * u[q]).powi(2);
        }
        let error = (err2 / norm2).sqrt();
        if let Some(last) = prev {
            assert!(last > 1.9 * error);
        }
        prev = Some(error);
        println!(
            "SPATIAL {}",
            serde_json::json!({"grid":"sphere_Y11","n_theta":nt,"relative_spatial_residual_area_norm":error})
        );
    }
}

fn mixed_grids() -> [(&'static str, DiffusionGrid); 3] {
    [
        (
            "box2",
            DiffusionGrid::periodic_box([6, 5], [2.0, 1.0], [0.2, 0.08]).unwrap(),
        ),
        (
            "box3",
            DiffusionGrid::periodic_box([4, 4, 5], [1.0, 2.0, 3.0], [0.1, 0.2, 0.3]).unwrap(),
        ),
        ("sphere", DiffusionGrid::sphere(6, 12, 1.3, 0.15).unwrap()),
    ]
}

fn mixed_initial(n: usize) -> Vec<f64> {
    (0..n)
        .map(|i| (0.23 * i as f64).sin() + 0.2 * (1.93 * i as f64).cos())
        .collect()
}

#[test]
fn multimode_endpoints_match_independent_face_assembly_exponential() {
    let reference: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/spatial_expm.json")).unwrap();
    for (name, grid) in mixed_grids() {
        let y0 = mixed_initial(grid.dimension());
        let opts = VigilodeOptions::default()
            .strategy(SolverStrategy::Auto)
            .rtol(reference["rtol"].as_f64().unwrap())
            .atol(reference["atol"].as_f64().unwrap());
        let tf = reference["final_time"].as_f64().unwrap();
        let s = VigilODE::solve(&grid.problem(SpatialRows::Serial), (0.0, tf), &y0, opts).unwrap();
        assert!(s.success() && s.final_time == tf);
        let exact = reference["cases"][name]["endpoint"].as_array().unwrap();
        assert_eq!(s.final_state.len(), exact.len());
        let error = exact
            .iter()
            .zip(&s.final_state)
            .map(|(x, y)| (x.as_f64().unwrap() - y).abs())
            .fold(0.0_f64, f64::max);
        assert!(error < 2e-7, "{name}: {error}");
        println!(
            "SPATIAL {}",
            serde_json::json!({"grid":name,"case":"multimode",
            "endpoint_max_error":error,"accepted_steps":s.stats.accepted_steps,
            "rhs_calls":s.stats.counters.rhs_calls,"jvp_vectors":s.stats.counters.jvp_vectors,
            "direct_factorizations":s.stats.counters.direct_factorizations})
        );
    }
}

#[test]
fn stage_gmres_policies_on_independently_referenced_diffusion() {
    use rodas5p_core::CoreError;
    use rodas5p_integrators::{
        AdaptiveStepConfig, MfStageSolvePolicy, OdeProblem, OutputSchedule, Rodas5pMfFastWorkspace,
        RoutingOptions,
    };
    use std::sync::Arc;

    let reference: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/spatial_expm.json")).unwrap();
    let tf = reference["final_time"].as_f64().unwrap();
    let schedule = OutputSchedule::new(vec![0.0, tf]).unwrap();
    let defaults = vigilode::Options::default();
    let config = AdaptiveStepConfig {
        rtol: reference["rtol"].as_f64().unwrap(),
        atol: reference["atol"].as_f64().unwrap(),
        initial_step: defaults.h0,
        max_attempts: defaults.max_steps,
        ..Default::default()
    };
    for (name, g) in mixed_grids() {
        let n = g.dimension();
        let op = g.clone();
        let p = OdeProblem::new(
            name,
            n,
            Arc::new(move |_, u, out| {
                op.apply(u, out, SpatialRows::Serial)
                    .map_err(|e| CoreError::Dimension(e.to_string()))
            }),
            None,
            None,
            Some(Arc::new(move |_, _, v, out| {
                g.apply(v, out, SpatialRows::Serial)
                    .map_err(|e| CoreError::Dimension(e.to_string()))
            })),
            None,
            true,
            None,
            None,
        )
        .unwrap();
        let y0 = mixed_initial(n);
        for policy in [
            MfStageSolvePolicy::Shipped,
            MfStageSolvePolicy::ColumnTested,
            MfStageSolvePolicy::ColumnTestedDoubling,
        ] {
            let mut ws =
                Rodas5pMfFastWorkspace::new(&p, &RoutingOptions::default().matrix_free).unwrap();
            ws.set_gmres_into(true);
            ws.set_stage_solve_policy(policy).unwrap();
            let result = ws
                .integrate_observed_with_observer(
                    &p,
                    (0.0, tf),
                    &y0,
                    &config,
                    &schedule,
                    &mut |_, _, _, _, _| {},
                )
                .unwrap()
                .0
                .observed;
            let stats = ws.stage_solve_policy_statistics();
            assert!(result.success && result.t.last() == Some(&tf));
            let exact = reference["cases"][name]["endpoint"].as_array().unwrap();
            let actual = result.y.last().unwrap();
            assert_eq!(exact.len(), actual.len());
            let err = exact
                .iter()
                .zip(actual)
                .map(|(a, b)| (a.as_f64().unwrap() - b).abs())
                .fold(0.0_f64, f64::max);
            let c = &result.counters;
            assert!(err < 2e-7 && c.direct_factorizations == 0);
            println!(
                "SPATIAL {}",
                serde_json::json!({"case":"v24_spatial","geometry":name,"policy":policy.id(),
                    "accepted_steps":c.accepted_steps, "rejected_steps":c.rejected_steps,
                    "budget_exhausted_solves":stats.budget_exhausted_solves,"other_linear_failures":stats.other_linear_failures,"failed_confirmations":stats.failed_confirmations, "arnoldi_columns":stats.columns,"linear_solve_failures":c.linear_solve_failures, "jvp_vectors":c.jvp_vectors,
                    "endpoint_max_error":err, "direct_factorizations":c.direct_factorizations})
            );
        }
    }
}
