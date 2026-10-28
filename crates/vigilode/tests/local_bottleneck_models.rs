//! Model identity and scientific-contract tests, independent of solver wrappers.
#[path = "../examples/local_bottleneck_support/models.rs"]
mod models;
use models::Model;

#[test]
fn all_twenty_four_models_have_finite_rhs_and_directional_derivatives() {
    for id in 1..=24 {
        let m = Model::new(id, true).unwrap();
        let y = m.initial();
        let v: Vec<f64> = (0..m.n).map(|i| 0.1 * (i as f64 + 0.4).sin()).collect();
        let mut rhs = vec![0.0; m.n];
        let mut jvp = rhs.clone();
        let mut ft = rhs.clone();
        m.apply(0.0, &y, None, false, &mut rhs, false);
        m.apply(0.0, &y, Some(&v), false, &mut jvp, false);
        m.apply(0.0, &y, None, true, &mut ft, false);
        assert!(
            rhs.iter().chain(&jvp).chain(&ft).all(|x| x.is_finite()),
            "case{id}"
        );
        let h = 1e-6;
        let yp: Vec<f64> = y.iter().zip(&v).map(|(a, b)| a + h * b).collect();
        let ym: Vec<f64> = y.iter().zip(&v).map(|(a, b)| a - h * b).collect();
        let mut plus = rhs.clone();
        let mut minus = rhs.clone();
        m.apply(0.0, &yp, None, false, &mut plus, false);
        m.apply(0.0, &ym, None, false, &mut minus, false);
        let scale = jvp.iter().map(|x| x.abs()).fold(1.0_f64, f64::max);
        let err = jvp
            .iter()
            .zip(plus.iter().zip(&minus))
            .map(|(a, (b, c))| (a - (b - c) / (2.0 * h)).abs())
            .fold(0.0_f64, f64::max);
        assert!(
            err <= 2e-5 * scale,
            "case{id} JVP error={err} scale={scale}"
        );
    }
}

#[test]
fn analytic_states_satisfy_the_semidiscrete_equations() {
    for id in [6, 7, 10, 15, 16, 19, 22] {
        let m = Model::new(id, true).unwrap();
        let t = 0.37 * m.end;
        let h = 1e-7_f64.min(m.end * 1e-4);
        let exact = m.exact(t).unwrap();
        let plus = m.exact(t + h).unwrap();
        let minus = m.exact(t - h).unwrap();
        let mut rhs = vec![0.0; m.n];
        m.apply(t, &exact, None, false, &mut rhs, false);
        let scale = rhs.iter().map(|x| x.abs()).fold(1.0_f64, f64::max);
        let err = rhs
            .iter()
            .zip(plus.iter().zip(&minus))
            .map(|(a, (b, c))| (a - (b - c) / (2.0 * h)).abs())
            .fold(0.0_f64, f64::max);
        assert!(
            err < 1e-5 * scale,
            "case{id}, equation error={err},scale={scale}"
        );
    }
}

#[test]
fn conservative_semidiscretizations_telescope() {
    for id in [1, 6, 14, 17, 22] {
        let m = Model::new(id, true).unwrap();
        let y = m.initial();
        let mut rhs = vec![0.0; m.n];
        m.apply(0.0, &y, None, false, &mut rhs, false);
        let norm = rhs.iter().map(|x| x.abs()).sum::<f64>().max(1.0);
        assert!(rhs.iter().sum::<f64>().abs() <= 1e-12 * norm, "case{id}");
    }
}

#[test]
fn hierarchy_streaming_is_skew_and_scattering_is_dissipative() {
    let m = Model::new(20, true).unwrap();
    let y: Vec<f64> = (0..m.n).map(|i| (0.7 * i as f64).sin()).collect();
    let mut rhs = vec![0.0; m.n];
    m.apply(0.0, &y, None, false, &mut rhs, false);
    let production = y.iter().zip(&rhs).map(|(a, b)| a * b).sum::<f64>();
    let dissipation = (0..m.n)
        .map(|i| {
            let l = (i as f64).sqrt().floor() as usize;
            let w = if l == 0 {
                0.0
            } else if l == 2 {
                0.9
            } else {
                1.0
            };
            -1e4 * w * y[i] * y[i]
        })
        .sum::<f64>();
    assert!((production - dissipation).abs() < 1e-9);
}

#[test]
fn data_parallel_rows_preserve_bits_and_thread_pool_budget() {
    for threads in [1, 2, 4] {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap();
        for id in 1..=24 {
            let m = Model::new(id, true).unwrap();
            let y = m.initial();
            let mut serial = vec![0.0; m.n];
            let mut parallel = serial.clone();
            m.apply(0.0, &y, None, false, &mut serial, false);
            pool.install(|| m.apply(0.0, &y, None, false, &mut parallel, true));
            assert_eq!(serial, parallel, "case{id} threads{threads}");
        }
    }
}

#[test]
fn false_switching_claims_are_negative_controls() {
    let m = Model::new(5, true).unwrap();
    let y = [1.0_f64.cos(), 1.0];
    let mut f = [0.0; 2];
    m.apply(0.0, &y, Some(&[1.0, 0.0]), false, &mut f, false);
    assert!((f[0] + 1e6 + 0.2 * 1.0_f64.sin()).abs() < 1e-9);
    let m = Model::new(7, true).unwrap();
    let mut f = [0.0];
    m.apply(0.0, &[1.0], Some(&[1.0]), false, &mut f, false);
    assert_eq!(f[0], -1.0);
    m.apply(0.0, &[0.1], None, false, &mut f, false);
    assert!(f[0] < 1e-190);
}

#[test]
fn nonautonomous_partial_time_derivative_matches_independent_difference() {
    for id in [5, 6, 15, 20] {
        let m = Model::new(id, true).unwrap();
        let mut y = m.initial();
        if id == 5 {
            y[0] = y[1].cos();
        }
        let t = m.end / 3.0;
        let h = 1e-6;
        let mut ft = vec![0.0; m.n];
        let mut plus = ft.clone();
        let mut minus = ft.clone();
        m.apply(t, &y, None, true, &mut ft, false);
        m.apply(t + h, &y, None, false, &mut plus, false);
        m.apply(t - h, &y, None, false, &mut minus, false);
        let scale = ft.iter().map(|x| x.abs()).fold(1.0_f64, f64::max);
        let err = ft
            .iter()
            .zip(plus.iter().zip(&minus))
            .map(|(a, (b, c))| (a - (b - c) / (2.0 * h)).abs())
            .fold(0.0_f64, f64::max);
        assert!(err <= 1e-5 * scale, "case{id} ft error{err}");
    }
}

#[test]
fn negligible_evolution_certificate_is_scoped_in_time() {
    let m = Model::new(7, true).unwrap();
    assert!(m.exact(10.0).is_some());
    assert!(m.exact(10.0001).is_none());
}

#[test]
fn invalid_or_overflowed_reference_is_unavailable() {
    let m = Model::new(15, true).unwrap();
    assert!(m.exact(f64::NAN).is_none());
    assert!(m.exact(-1.0).is_none());
    assert!(m.exact(1000.0).is_none());
}
