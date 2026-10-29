use rodas5p_core::WorkCounters;
use rodas5p_integrators::{
    DampedRotationAffineWorkspace, DampedRotationBlock as Block, DampedRotationFlow as Flow,
    Dopri5Config, LawsonDopri5Workspace, OutputSchedule, QuotientWork,
    integrate_affine_lawson_dopri5_observed,
};
fn flow(d: f64, w: f64) -> Flow {
    Flow::repeated(
        Block {
            damping: d,
            frequency: w,
        },
        1,
    )
    .unwrap()
}
fn close(a: &[f64], b: &[f64], relative: f64, absolute: f64) {
    for (x, y) in a.iter().zip(b) {
        assert!(
            (x - y).abs() <= absolute + relative * y.abs(),
            "{a:?} != {b:?}"
        );
    }
}
#[test]
fn decimal_oracles_cover_singular_small_period_extreme_and_signed_blocks() {
    let rows: serde_json::Value =
        serde_json::from_str(include_str!("frontier_affine_oracles.json")).unwrap();
    for row in rows.as_array().unwrap() {
        let pair = |key: &str| -> Vec<f64> {
            row[key]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect()
        };
        let f = flow(
            row["damping"].as_f64().unwrap(),
            row["frequency"].as_f64().unwrap(),
        );
        let h = row["h"].as_f64().unwrap();
        let (y, b, expected) = (pair("y"), pair("b"), pair("expected"));
        let mut actual = [0.; 2];
        f.affine_into(h, &y, &b, &mut actual).unwrap();
        close(&actual, &expected, 2e-13, 1e-300);
        let mut ws = DampedRotationAffineWorkspace::new(f).unwrap();
        let mut cached = [0.; 2];
        ws.apply_into(h, &y, &b, &mut cached).unwrap();
        assert_eq!(actual, cached, "{}", row["name"]);
    }
}
#[test]
fn affine_semigroup_and_cache_reuse() {
    for d in [0., 0.2, 1000.] {
        for w in [-30., 0., 7.] {
            let f = flow(d, w);
            let (y, b) = ([2., -3.], [4., 5.]);
            let mut direct = [0.; 2];
            let mut first = [0.; 2];
            let mut split = [0.; 2];
            f.affine_into(0.7, &y, &b, &mut direct).unwrap();
            f.affine_into(0.2, &y, &b, &mut first).unwrap();
            f.affine_into(0.5, &first, &b, &mut split).unwrap();
            close(&split, &direct, 2e-13, 1e-14);
        }
    }
    let f = Flow::repeated(
        Block {
            damping: 2.,
            frequency: 3.,
        },
        128,
    )
    .unwrap();
    let mut ws = DampedRotationAffineWorkspace::new(f).unwrap();
    let mut out = vec![0.; 256];
    for _ in 0..3 {
        ws.apply_into(0.25, &vec![1.; 256], &vec![2.; 256], &mut out)
            .unwrap();
    }
    assert_eq!(ws.work().coefficient_refreshes, 1);
    assert_eq!(ws.work().coefficient_pairs, 1);
    assert_eq!(ws.work().applied_block_actions, 384);
}
#[test]
fn validation_and_failed_refresh_do_not_poison_cache() {
    let mut ws = DampedRotationAffineWorkspace::new(flow(0., 1e308)).unwrap();
    let mut out = [0.; 2];
    ws.apply_into(1e-308, &[1., 0.], &[0., 0.], &mut out)
        .unwrap();
    let expected = out;
    assert!(ws.apply_into(10., &[1., 0.], &[0., 0.], &mut out).is_err());
    ws.apply_into(1e-308, &[1., 0.], &[0., 0.], &mut out)
        .unwrap();
    assert_eq!(out, expected);
    for h in [-1., f64::INFINITY, f64::NAN] {
        assert!(ws.apply_into(h, &[1., 0.], &[0., 0.], &mut out).is_err());
    }
    assert!(
        ws.apply_into(0., &[1., 0.], &[f64::NAN, 0.], &mut out)
            .is_err()
    );
    assert!(ws.apply_into(0., &[1., 0.], &[0.], &mut out).is_err());
    assert!(
        flow(0., 0.)
            .affine_into(2., &[0., 0.], &[f64::MAX, 0.], &mut out)
            .is_err()
    );
}
#[test]
fn affine_lawson_removes_constant_forcing_from_residual_and_embedded_error() {
    let f = flow(1000., 3000.);
    let b = [1000., -2000.];
    let y = [1., 2.];
    let mut ws = LawsonDopri5Workspace::new_affine(f.clone(), &b).unwrap();
    let mut endpoint = [0.; 2];
    let mut error = [0.; 2];
    let mut c = WorkCounters::default();
    let mut q = QuotientWork::default();
    ws.step_into(
        &|_, _, o: &mut [f64]| {
            o.fill(0.);
            Ok(())
        },
        0.,
        &y,
        1.,
        &mut endpoint,
        &mut error,
        &mut c,
        &mut q,
    )
    .unwrap();
    let mut exact = [0.; 2];
    f.affine_into(1., &y, &b, &mut exact).unwrap();
    assert_eq!(endpoint, exact);
    assert_eq!(error, [0.; 2]);
    assert_eq!(c.rhs_calls, 7);
    let mut config = Dopri5Config::default();
    config.adaptive.initial_step = 1.;
    config.adaptive.max_step = 1.;
    let result = integrate_affine_lawson_dopri5_observed(
        &f,
        &b,
        &|_, _, o: &mut [f64]| {
            o.fill(0.);
            Ok(())
        },
        (0., 1.),
        &y,
        &config,
        &OutputSchedule::new(vec![0., 1.]).unwrap(),
    )
    .unwrap();
    assert!(result.integration.observed.success);
    assert_eq!(result.integration.accepted_steps, 1);
    assert_eq!(result.integration.rejected_steps, 0);
    assert_eq!(result.integration.final_state, exact);
}
#[test]
fn affine_lawson_classical_order_on_manufactured_smooth_residual() {
    // y(t)=(cos(t),sin(t)), N(t,y)=y_exact'(t)-A*y_exact(t)-b.
    let f = flow(0.3, 0.7);
    let b = [0.4, -0.2];
    let exact = [1f64.cos(), 1f64.sin()];
    let residual = |t: f64, _: &[f64], o: &mut [f64]| {
        let (s, c) = t.sin_cos();
        o[0] = -s + 0.3 * c + 0.7 * s - b[0];
        o[1] = c - 0.7 * c + 0.3 * s - b[1];
        Ok(())
    };
    let mut errors = Vec::new();
    for steps in [4, 8, 16] {
        let mut ws = LawsonDopri5Workspace::new_affine(f.clone(), &b).unwrap();
        let mut y = [1., 0.];
        let mut z = [0.; 2];
        let mut e = [0.; 2];
        let mut c = WorkCounters::default();
        let mut q = QuotientWork::default();
        let h = 1. / steps as f64;
        for i in 0..steps {
            ws.step_into(
                &residual,
                i as f64 * h,
                &y,
                h,
                &mut z,
                &mut e,
                &mut c,
                &mut q,
            )
            .unwrap();
            y = z;
        }
        errors.push((y[0] - exact[0]).hypot(y[1] - exact[1]));
    }
    assert!(errors[0] / errors[1] > 20., "{errors:?}");
    assert!(errors[1] / errors[2] > 20., "{errors:?}");
}

#[test]
fn harmonic_forcing_matches_manufactured_original_negative_control() {
    // Exact y=(cos t,sin t): b_c=(d,1-w), b_s=(w-1,d).
    for damping in [0., 0.1, 500., 1e6] {
        for frequency in [-500., 0., 1., 500.] {
            let f = flow(damping, frequency);
            let c = [damping, 1. - frequency];
            let d = [frequency - 1., damping];
            for start in [0., 0.7] {
                for dt in [1e-8, 0.01, 1.] {
                    let mut y = [0.; 2];
                    f.harmonic_into(start, dt, &[start.cos(), start.sin()], 1., &c, &d, &mut y)
                        .unwrap();
                    close(&y, &[(start + dt).cos(), (start + dt).sin()], 2e-10, 2e-12);
                }
            }
        }
    }
}
#[test]
fn harmonic_resonance_zero_frequency_and_validation() {
    let mut out = [0.; 2];
    // y'=i*w*y+exp(i*w*t), y(0)=0 has y=t*exp(i*w*t).
    for frequency in [-1000., 0., 1000.] {
        let f = flow(0., frequency);
        f.harmonic_into(
            0.,
            0.75,
            &[0., 0.],
            frequency,
            &[1., 0.],
            &[0., 1.],
            &mut out,
        )
        .unwrap();
        close(
            &out,
            &[
                0.75 * (frequency * 0.75).cos(),
                0.75 * (frequency * 0.75).sin(),
            ],
            2e-13,
            1e-14,
        );
    }
    let f = flow(1., 2.);
    f.harmonic_into(2., 0.4, &[3., 4.], 0., &[5., 6.], &[100., 200.], &mut out)
        .unwrap();
    let mut exact = [0.; 2];
    f.affine_into(0.4, &[3., 4.], &[5., 6.], &mut exact)
        .unwrap();
    close(&out, &exact, 2e-14, 1e-14);
    assert!(
        f.harmonic_into(
            f64::INFINITY,
            1.,
            &[0., 0.],
            1.,
            &[0., 0.],
            &[0., 0.],
            &mut out
        )
        .is_err()
    );
    assert!(
        f.harmonic_into(0., 1., &[0., 0.], 1., &[f64::NAN, 0.], &[0., 0.], &mut out)
            .is_err()
    );
}

#[test]
fn harmonic_subnormal_forcing_is_not_halved_before_amplification() {
    let mut out = [0.; 2];
    let smallest = f64::from_bits(1);
    let frequency = 1e-308;
    let h = 1e308;
    flow(0., 0.)
        .harmonic_into(
            0.,
            h,
            &[0., 0.],
            frequency,
            &[smallest, 0.],
            &[0., 0.],
            &mut out,
        )
        .unwrap();
    let expected = (smallest / frequency) * (frequency * h).sin();
    close(&out, &[expected, 0.], 2e-14, 1e-30);
    // At nu=0 the sine coefficient has no effect, even if it dwarfs cosine.
    flow(0., 0.)
        .harmonic_into(0., 1., &[0., 0.], 0., &[1., 2.], &[1e308, 1e308], &mut out)
        .unwrap();
    assert_eq!(out, [1., 2.]);
}
