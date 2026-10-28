//! Contracts of the stage-coordinate candidate adapter (research node
//! `research/int04_stage_chart_20261003`, integrated DAG node INT-04): chart
//! identities, singular charts, domain exits and invalid inputs.

#[path = "int04_common/mod.rs"]
mod common;

use common::{Cube, PositiveSquare, Scaling, Triangular, families};
use rodas5p_core::rodas5p_coefficients;
use rodas5p_integrators::{ChartStatus, StageChart, StageTarget, stage_chart_candidate};

fn splitmix(state: &mut u64) -> f64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^= z >> 31;
    2.0 * ((z >> 11) as f64 / (1u64 << 53) as f64) - 1.0
}

#[test]
fn regular_charts_invert_and_differentiate() {
    let (s, n) = (8, 4);
    let charts: [Box<dyn StageChart>; 2] = [
        Box::new(Triangular { s, n, beta: 0.5 }),
        Box::new(Scaling { s, n }),
    ];
    let mut seed = 2024_u64;
    for chart in &charts {
        for _ in 0..20 {
            let z: Vec<f64> = (0..s * n).map(|_| splitmix(&mut seed)).collect();
            let back = chart.inverse(&chart.forward(&z).unwrap()).unwrap();
            let scale = z.iter().fold(0.0_f64, |m, v| m.max(v.abs()));
            for (a, b) in back.iter().zip(&z) {
                assert!((a - b).abs() <= 1.0e-14 * scale, "{}", chart.name());
            }
            let dz: Vec<f64> = (0..s * n).map(|_| splitmix(&mut seed)).collect();
            let jvp = chart.jvp(&z, &dz).unwrap();
            let eps = 1.0e-6;
            let plus: Vec<f64> = z.iter().zip(&dz).map(|(a, b)| a + eps * b).collect();
            let minus: Vec<f64> = z.iter().zip(&dz).map(|(a, b)| a - eps * b).collect();
            let (fp, fm) = (
                chart.forward(&plus).unwrap(),
                chart.forward(&minus).unwrap(),
            );
            let jscale = jvp.iter().fold(0.0_f64, |m, v| m.max(v.abs()));
            for p in 0..s * n {
                let fd = (fp[p] - fm[p]) / (2.0 * eps);
                assert!((fd - jvp[p]).abs() <= 1.0e-6 * jscale, "{}", chart.name());
            }
        }
    }
}

#[test]
fn singular_charts_and_domain_exits_are_typed() {
    let target = StageTarget::sequential(rodas5p_coefficients().unwrap()).unwrap();
    let s = target.stages();
    for (label, problem) in families() {
        let n = problem.dimension();
        let cube =
            stage_chart_candidate(&target, &problem, &Cube, &vec![0.0; s * n], 20, 1e-13).unwrap();
        assert_eq!(cube.status, ChartStatus::SingularChart, "{label}");
        let square = stage_chart_candidate(
            &target,
            &problem,
            &PositiveSquare,
            &vec![1.0; s * n],
            20,
            1e-13,
        )
        .unwrap();
        assert_eq!(square.status, ChartStatus::DomainExit, "{label}");
        // The last iterate inside the domain is returned, with positive K.
        assert!(square.stages.iter().flatten().all(|k| *k > 0.0), "{label}");
    }
}

#[test]
fn invalid_inputs_are_errors() {
    let target = StageTarget::sequential(rodas5p_coefficients().unwrap()).unwrap();
    let s = target.stages();
    let (_, problem) = families().into_iter().nth(1).unwrap(); // n = 4
    let n = problem.dimension();
    let chart = Triangular { s, n, beta: 0.5 };
    let ok = vec![0.0; s * n];
    assert!(stage_chart_candidate(&target, &problem, &chart, &ok, 20, 1e-13).is_ok());
    // An n-vector endpoint offered in place of s n stages.
    assert!(stage_chart_candidate(&target, &problem, &chart, &vec![0.0; n], 20, 1e-13).is_err());
    let mut nan = ok.clone();
    nan[3] = f64::NAN;
    assert!(stage_chart_candidate(&target, &problem, &chart, &nan, 20, 1e-13).is_err());
    for tolerance in [0.0, -1.0, f64::NAN] {
        assert!(stage_chart_candidate(&target, &problem, &chart, &ok, 20, tolerance).is_err());
    }
    assert!(
        stage_chart_candidate(
            &target,
            &problem,
            &PositiveSquare,
            &vec![-1.0; s * n],
            20,
            1e-13
        )
        .is_err()
    );
}
