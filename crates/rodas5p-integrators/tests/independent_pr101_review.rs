//! Independent review cases: expanding flow, negative eta and inherited disk.
use rodas5p_core::WorkCounters;
use rodas5p_integrators::{DenseSegmentWork, Pr101Model, Rodas5pQuarticSegment, StepResult};

fn linear_fixture(h: f64) -> Rodas5pQuarticSegment {
    let step = StepResult {
        t_old: 0.0,
        t_new: h,
        h,
        y_old: vec![0.0],
        y_new: vec![h],
        stages: vec![vec![0.0]; 8],
        error_vector: vec![0.0],
        error_norm: 0.0,
        accepted: true,
        method: "review-linear".into(),
        used_fallback: false,
        certificate: None,
        counters: WorkCounters::default(),
    };
    Rodas5pQuarticSegment::from_step(&step, &mut DenseSegmentWork::default()).unwrap()
}
#[test]
fn signed_nonlinearity_and_expansion_cover_exact_sine_member() {
    for lambda in [-10.0, 0.0, 2.0] {
        for eta in [-3.0, 0.0, 3.0] {
            let model = Pr101Model::new(lambda, eta).unwrap();
            let mut work = DenseSegmentWork::default();
            let verified = model
                .verify_segment(linear_fixture(0.125), 1e-3, 0.1, &mut work)
                .unwrap();
            for j in 0..=32 {
                let t = j as f64 / 256.0;
                let out = verified.evaluate(t, &mut work).unwrap();
                assert!((out.center() - t.sin()).abs() <= out.radius());
            }
            assert!(verified.bounds().uniform_radius >= 1e-3);
        }
    }
}
