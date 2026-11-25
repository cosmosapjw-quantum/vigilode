use rodas5p_core::DenseMatrix;
use rodas5p_integrators::retro_portfolio::{AttemptStatus, Portfolio, Route};
use std::collections::BTreeSet;
use vigilode::spatial::{SeparableStages, adaptive_stage};
fn fixture(n: usize) -> (SeparableStages, DenseMatrix, Vec<f64>) {
    // dx=1 exactly; upwind with positive velocity, and a reaction shift.
    let stage = SeparableStages::periodic_upwind_line(n, n as f64, 0.125, 0.5, -0.25).unwrap();
    let mut w = DenseMatrix::zeros(n, n);
    for i in 0..n {
        w[(i, i)] = 3.;
        w[(i, (i + n - 1) % n)] = -0.625;
        w[(i, (i + 1) % n)] = -0.125;
    }
    // z=2, c=-.25: diagonal z-c+2D+v=3.
    let b = (0..n).map(|i| ((i % 9) as f64 - 4.) / 16.).collect();
    (stage, w, b)
}
#[test]
fn paired_complex_and_direct_share_original_stage_certificate() {
    let (stage, w, b) = fixture(32);
    let mut p = Portfolio::new(1, 3).unwrap();
    let mut seen = BTreeSet::new();
    let reference = rodas5p_core::direct_solve(&w, &b).unwrap();
    for i in 0..6 {
        let r = adaptive_stage(&mut p, &stage, 2., &w, &b, 1e-10).unwrap();
        let x = r.value.as_ref().unwrap();
        assert!(
            x.iter()
                .zip(&reference)
                .all(|(&a, &b)| (a - b).abs() < 1e-10)
        );
        seen.insert(r.selected.unwrap());
        println!(
            "SPATIAL {}",
            serde_json::json!({"call":i,"selected":r.selected,"attempts":r.attempts,"total_ns":r.total_elapsed_ns()})
        );
    }
    assert_eq!(seen.len(), 3);
}
#[test]
fn wrong_operator_mass_agreement_is_not_authority() {
    let (stage, mut w, b) = fixture(32);
    let mut p = Portfolio::new(8, 3).unwrap();
    // Perturb a row by a conservative pair: zero row sum does not imply
    // correct original Jacobian or correct solution.
    w[(0, 0)] += 0.25;
    w[(0, 1)] -= 0.25;
    let r = adaptive_stage(&mut p, &stage, 2., &w, &b, 1e-10).unwrap();
    assert_eq!(r.selected, Some(Route::SpatialDirect));
    assert_eq!(r.attempts.len(), 3);
    assert!(
        r.attempts[..2]
            .iter()
            .all(|a| a.status == AttemptStatus::Rejected)
    );
    println!(
        "SPATIAL_WRONG {}",
        serde_json::json!({"selected":r.selected,"attempts":r.attempts})
    );
    w[(0, 0)] = 0.;
    assert!(adaptive_stage(&mut p, &stage, 2., &w, &b, 1e-10).is_err());
}

#[test]
fn actual_spatial_ivp_same_certified_endpoint() {
    use rodas5p_core::directed::{Interval, exp_interval};
    use vigilode::spatial::{DiffusionGrid, EndpointRequest, SpatialRows, adaptive_endpoint};
    use vigilode::{Atol, Options};
    let grid = DiffusionGrid::periodic_box([8, 4], [8., 4.], [0.125, 0.25]).unwrap();
    let problem = grid.problem(SpatialRows::Serial);
    let stage = grid.separable_stages(0.).unwrap();
    let y0: Vec<_> = (0..32)
        .map(|i| if i % 2 == 0 { 0.5 } else { -0.5 })
        .collect();
    let opts = Options {
        rtol: 1e-8,
        atol: Atol::Scalar(1e-10),
        h0: 0.001,
        ..Options::default()
    };
    // Represented periodic stencil, alternating mode along x, eigenvalue
    // -4*0.125 = -0.5 exactly; no model-dependent fit.
    let g = exp_interval(-0.25).unwrap();
    let mut portfolio = Portfolio::new(1, 3).unwrap();
    let mut seen = BTreeSet::new();
    for i in 0..3 {
        let req = EndpointRequest {
            problem: &problem,
            span: (0., 0.5),
            initial: &y0,
            options: &opts,
            stages: &stage,
            replay_safe: true,
            absolute_tolerance: 1e-7,
        };
        let r = adaptive_endpoint(&mut portfolio, req, |s| {
            let mut e = 0.0_f64;
            for (&a, &b) in s.final_state().unwrap().iter().zip(&y0) {
                e = e.max(Interval::point(a)?.sub(g.scale(b)?)?.mag());
            }
            Ok(e)
        })
        .unwrap();
        assert!(r.value.is_some());
        seen.insert(r.selected.unwrap());
        println!(
            "SPATIAL_IVP {}",
            serde_json::json!({"call":i,"selected":r.selected,"attempts":r.attempts,"total_ns":r.total_elapsed_ns()})
        );
    }
    assert_eq!(seen.len(), 3);
    let req = EndpointRequest {
        problem: &problem,
        span: (0., 0.5),
        initial: &y0,
        options: &opts,
        stages: &stage,
        replay_safe: false,
        absolute_tolerance: 1e-7,
    };
    assert!(adaptive_endpoint(&mut portfolio, req, |_| Ok(0.)).is_err());
}
