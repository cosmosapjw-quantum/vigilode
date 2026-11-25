use rodas5p_integrators::fourier_path_candidate::{
    relative_difference,
    warp::{self, Mode, Work},
};
use rodas5p_integrators::fourier_path_certificate::{
    Cplx, FourierModel, FourierPath, predictor_rhs_h,
};

#[test]
fn quotient_reconstructs_high_degree_and_complex_conjugate_channels() {
    let a = FourierPath::from_coefficients([
        ((1, 3), Cplx::new(0.25, 0.5)),
        ((-1, 0), Cplx::new(0.125, -0.25)),
    ]);
    let b = FourierPath::from_coefficients([((-1, 3), Cplx::new(0.5, -0.125))]);
    let phase = FourierPath::from_coefficients([((1, 2), Cplx::new(0.75, 0.25))]);
    let model = FourierModel::new(40., 1).unwrap();
    let path = [a, b];
    let expected = predictor_rhs_h(&path, &model, &phase, 0.125, 1.);
    assert!(
        expected
            .iter()
            .any(|p| p.coefficients().keys().any(|(_, j)| *j >= 8))
    );
    let got = warp::rhs_h(
        &path,
        &model,
        &phase,
        0.125,
        Mode::Crt,
        &mut Work::default(),
    )
    .unwrap();
    for j in 0..2 {
        assert!(relative_difference(&got[j], &expected[j]) < 1e-12);
    }
}

use rodas5p_integrators::fourier_path_candidate::{FftWork, fft_rhs_h, required_shape};
use rodas5p_integrators::fourier_path_certificate::{
    FourierState, FourierTrial, physical_error, run_driver,
};
use serde_json::{Value, json};

fn fixture(k: i32, p: u32, phase_k: i32, phase_p: u32) -> ([FourierPath; 2], FourierPath) {
    let mut state = 0x243f_6a88_85a3_08d3u64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        ((state >> 56) as i32 - 128) as f64 / 4096.
    };
    let mut make = |k: i32, p: u32| {
        FourierPath::from_coefficients(
            (-k..=k)
                .flat_map(|i| (0..=p).map(move |j| (i, j)))
                .map(|key| (key, Cplx::new(next(), next()))),
        )
    };
    ([make(k, p), make(k, p)], make(phase_k, phase_p))
}

#[test]
fn full_and_folded_match_direct_for_dense_sparse_and_folded_input() {
    // Includes constant degree, phase support larger than half the full grid,
    // positive/negative harmonics and complex asymmetric coefficients.
    for (k, p, sk, q) in [
        (0, 0, 0, 0),
        (0, 0, 0, 5),
        (1, 1, 1, 0),
        (2, 3, 1, 2),
        (3, 6, 2, 4),
        (1, 2, 0, 20),
    ] {
        let (path, phase) = fixture(k, p, sk, q);
        let model = FourierModel::new(40., -1).unwrap();
        let expected = predictor_rhs_h(&path, &model, &phase, 0.125, 1.);
        let shape = required_shape(&path, &phase);
        for mode in [Mode::Full, Mode::Crt] {
            let mut work = Work::default();
            let got = warp::rhs_h(&path, &model, &phase, 0.125, mode, &mut work).unwrap();
            for j in 0..2 {
                let e = relative_difference(&got[j], &expected[j]);
                assert!(e < 1e-12, "k={k} p={p} q={q} mode={mode:?} e={e}");
            }
            let sectors = if mode == Mode::Crt && shape.3 > 1 {
                2
            } else {
                1
            };
            assert_eq!(work.forward_2d, 3 * sectors);
            assert_eq!(work.inverse_2d, 2 * sectors);
            assert_eq!(work.grid_points, shape.2 * shape.3);
            assert_eq!(work.max_grid_points, shape.2 * shape.3 / sectors);
            assert_eq!(work.transformed_points, 5 * work.grid_points);
        }
    }
}
#[test]
fn high_degree_carry_is_not_discardable() {
    // a=b=t^3, phase=t^2, g=eps=1/8. fa and fb contain t^11.
    let a = FourierPath::from_coefficients([((1, 3), Cplx::new(1., 0.))]);
    let b = FourierPath::from_coefficients([((1, 3), Cplx::new(1., 0.))]);
    let phase = FourierPath::from_coefficients([((-1, 2), Cplx::new(1., 0.))]);
    let path = [a, b];
    let model = FourierModel::new(40., 1).unwrap();
    let target = predictor_rhs_h(&path, &model, &phase, 0.125, 1.);
    let (_, _, nk, nj) = required_shape(&path, &phase);
    assert_eq!(nj, 16);
    let mut w = FftWork::default();
    assert!(
        fft_rhs_h(
            &path,
            &model,
            &phase,
            0.125,
            Some((nk, nj / 2)),
            false,
            &mut w
        )
        .is_err()
    );
    let aliased = fft_rhs_h(
        &path,
        &model,
        &phase,
        0.125,
        Some((nk, nj / 2)),
        true,
        &mut w,
    )
    .unwrap();
    assert!(relative_difference(&aliased[0], &target[0]) > 0.5);
    let got = warp::rhs_h(
        &path,
        &model,
        &phase,
        0.125,
        Mode::Crt,
        &mut Work::default(),
    )
    .unwrap();
    assert!(relative_difference(&got[0], &target[0]) < 1e-12);
    assert!(target[0].coefficients().get(&(0, 11)).is_some());
    println!(
        "ALIAS relative={} reconstructed={}",
        relative_difference(&aliased[0], &target[0]),
        relative_difference(&got[0], &target[0])
    );
}
#[test]
fn resource_and_nonfinite_refusals() {
    let model = FourierModel::new(40., 1).unwrap();
    let a = FourierPath::constant(Cplx::new(0.25, 0.0));
    let phase = FourierPath::constant(Cplx::new(1., 0.));
    let path = [a.clone(), a];
    for key in [(i32::MIN, 0), (0, u32::MAX), (500_000, 0)] {
        let huge = FourierPath::from_coefficients([(key, Cplx::new(1., 0.))]);
        let mut w = Work::default();
        assert!(
            warp::rhs_h(
                &[huge, path[1].clone()],
                &model,
                &phase,
                0.125,
                Mode::Crt,
                &mut w
            )
            .is_err()
        );
        assert_eq!(w, Work::default());
    }
    let nan = FourierPath::constant(Cplx::new(f64::NAN, 0.));
    assert!(
        warp::rhs_h(
            &[nan, path[1].clone()],
            &model,
            &phase,
            0.125,
            Mode::Full,
            &mut Work::default()
        )
        .is_err()
    );
    assert!(
        warp::rhs_h(
            &path,
            &model,
            &phase,
            f64::NAN,
            Mode::Crt,
            &mut Work::default()
        )
        .is_err()
    );
    let mut w = Work {
        rhs_calls: usize::MAX,
        ..Work::default()
    };
    let before = w;
    assert!(warp::rhs_h(&path, &model, &phase, 0.125, Mode::Full, &mut w).is_err());
    assert_eq!(w, before);
}
fn hx(v: f64) -> String {
    format!("{:016x}", v.to_bits())
}
fn cx(z: Cplx) -> Value {
    json!([hx(z.re), hx(z.im)])
}
fn path_json(p: &FourierPath) -> Value {
    json!(
        p.coefficients()
            .iter()
            .map(|((k, j), v)| json!([k, j, hx(v.re), hx(v.im)]))
            .collect::<Vec<_>>()
    )
}
fn state_json(s: &FourierState) -> Value {
    json!({"t":hx(s.t()),"a":s.amplitudes().iter().map(|&z|cx(z)).collect::<Vec<_>>(),"error":hx(s.error()),"generation":s.generation()})
}
fn trial_json(t: &FourierTrial) -> Value {
    let c = &t.certificate;
    json!({"h":hx(t.h),"K":t.k_max,"p":t.p_max,"sweeps":t.sweeps,
        "path":[path_json(&t.path[0]),path_json(&t.path[1])],
        "accepted":c.accepted(),"error":hx(c.error()),"growth":hx(c.growth()),
        "residual":hx(c.residual()),"start_mismatch":hx(c.start_mismatch()),
        "endpoint":c.endpoint().iter().map(|&z|cx(z)).collect::<Vec<_>>()})
}
fn work_json(w: Work) -> Value {
    json!({"rhs_calls":w.rhs_calls,"forward_2d":w.forward_2d,"inverse_2d":w.inverse_2d,
        "grid_points":w.grid_points,"max_grid_points":w.max_grid_points,"sectors":w.sectors,"transformed_points":w.transformed_points})
}
#[test]
fn original_driver_and_certificate_are_used() {
    for omega in [40., 10000.] {
        for mode in [Mode::Full, Mode::Crt] {
            let model = FourierModel::new(omega, 1).unwrap();
            let mut work = Work::default();
            let mut worst = 0.0_f64;
            let mut rhs = |path: &[FourierPath; 2], m: &FourierModel, p: &FourierPath, h: f64| {
                let got = warp::rhs_h(path, m, p, h, mode, &mut work)?;
                let expected = predictor_rhs_h(path, m, p, h, 1.);
                for j in 0..2 {
                    worst = worst.max(relative_difference(&got[j], &expected[j]));
                }
                Ok(got)
            };
            let (records, counts) = run_driver(&model, 0.5, 1e-8, 0.125, Some(&mut rhs)).unwrap();
            let last = records.last().unwrap();
            assert_eq!(last.state.t(), 0.5);
            assert!(last.physical_error <= 1e-8);
            assert!(records.iter().all(|r| r.trial.certificate.accepted()));
            assert_eq!(counts.rhs_calls, work.rhs_calls);
            assert!(worst < 1e-12, "omega={omega} mode={mode:?} diff={worst}");
            let entry = json!({"omega":hx(omega),"sigma":1,"tol":hx(1e-8),"mode":format!("{mode:?}"),
                "run":{"ok":true,"counts":{"candidate_builds":counts.candidate_builds,
                "candidate_rejections":counts.candidate_rejections,"rhs_calls":counts.rhs_calls,"step_halvings":counts.step_halvings},
                "records":records.iter().map(|r|json!({"start":state_json(&r.start),"state":state_json(&r.state),"trial":trial_json(&r.trial),"physical_error":hx(r.physical_error)})).collect::<Vec<_>>()},
                "fft_work":work_json(work),"predictor_calls":work.rhs_calls,"worst_relative_difference_fft_vs_direct":worst});
            println!("DRIVER_CASE {entry}");
        }
    }
}
#[test]
fn exact_binary_fixture_export() {
    for (k, p, sk, q) in [
        (0, 0, 0, 0),
        (0, 0, 0, 5),
        (1, 1, 1, 0),
        (1, 3, 1, 2),
        (2, 2, 0, 7),
    ] {
        let (path, phase) = fixture(k, p, sk, q);
        let model = FourierModel::new(40., -1).unwrap();
        let h = 0.125;
        for mode in [Mode::Full, Mode::Crt] {
            let got = warp::rhs_h(&path, &model, &phase, h, mode, &mut Work::default()).unwrap();
            println!(
                "EXACT_CASE {}",
                json!({"a":path_json(&path[0]),"b":path_json(&path[1]),"phase":path_json(&phase),"h":hx(h),"g":hx(model.g),"eps":hx(model.eps),"q":hx(model.q()),"out":[path_json(&got[0]),path_json(&got[1])],"mode":format!("{mode:?}")})
            );
        }
    }
}
fn median(samples: &[f64]) -> f64 {
    let mut s = samples.to_vec();
    s.sort_by(f64::total_cmp);
    s[s.len() / 2]
}
#[test]
#[ignore = "paired kernel and actual driver diagnostic timing"]
fn measure() {
    use std::{hint::black_box, time::Instant};
    for (k, p, sk, q) in [(1, 3, 1, 0), (3, 6, 1, 0), (5, 10, 1, 0), (2, 5, 0, 20)] {
        let (path, phase) = fixture(k, p, sk, q);
        let model = FourierModel::new(40., 1).unwrap();
        let expected = predictor_rhs_h(&path, &model, &phase, 0.125, 1.);
        let mut times: [Vec<f64>; 4] = std::array::from_fn(|_| Vec::new());
        let mut errs = [0.; 4];
        for rep in 0..8 {
            for offset in 0..4 {
                let arm = (rep + offset) % 4;
                let start = Instant::now();
                let got = match arm {
                    0 => predictor_rhs_h(&path, &model, &phase, 0.125, 1.),
                    1 => fft_rhs_h(
                        &path,
                        &model,
                        &phase,
                        0.125,
                        None,
                        false,
                        &mut FftWork::default(),
                    )
                    .unwrap(),
                    2 => warp::rhs_h(
                        &path,
                        &model,
                        &phase,
                        0.125,
                        Mode::Full,
                        &mut Work::default(),
                    )
                    .unwrap(),
                    _ => warp::rhs_h(
                        &path,
                        &model,
                        &phase,
                        0.125,
                        Mode::Crt,
                        &mut Work::default(),
                    )
                    .unwrap(),
                };
                let ms = start.elapsed().as_secs_f64() * 1000.;
                black_box(&got);
                let e = relative_difference(&got[0], &expected[0])
                    .max(relative_difference(&got[1], &expected[1]));
                assert!(e < 1e-12);
                if rep > 0 {
                    times[arm].push(ms);
                    errs[arm] = f64::max(errs[arm], e);
                }
            }
        }
        let (_, _, nk, nj) = required_shape(&path, &phase);
        for arm in 0..4 {
            println!(
                "KERNEL {}",
                json!({"k":k,"p":p,"sk":sk,"q":q,"nk":nk,"nj":nj,"arm":(["direct","old-fft","warp-full","warp-crt"][arm]),"median_ms":median(&times[arm]),"samples_ms":times[arm],"relative_error":errs[arm]})
            );
        }
    }
    for omega in [40., 10000.] {
        let model = FourierModel::new(omega, 1).unwrap();
        let mut times: [Vec<f64>; 4] = std::array::from_fn(|_| Vec::new());
        let mut metrics: [Value; 4] = std::array::from_fn(|_| Value::Null);
        for rep in 0..6 {
            for offset in 0..4 {
                let arm = (rep + offset) % 4;
                let mut fw = FftWork::default();
                let mut ww = Work::default();
                let mut rhs =
                    |path: &[FourierPath; 2], m: &FourierModel, p: &FourierPath, h: f64| {
                        if arm == 1 {
                            fft_rhs_h(path, m, p, h, None, false, &mut fw)
                        } else {
                            warp::rhs_h(
                                path,
                                m,
                                p,
                                h,
                                if arm == 2 { Mode::Full } else { Mode::Crt },
                                &mut ww,
                            )
                        }
                    };
                let start = Instant::now();
                let (rec, c) = run_driver(
                    &model,
                    0.5,
                    1e-8,
                    0.125,
                    if arm == 0 { None } else { Some(&mut rhs) },
                )
                .unwrap();
                let ms = start.elapsed().as_secs_f64() * 1000.;
                let last = rec.last().unwrap();
                assert_eq!(last.state.t(), 0.5);
                let (_, err) = physical_error(&last.state, &model).unwrap();
                assert!(err <= 1e-8);
                if rep > 0 {
                    times[arm].push(ms);
                    metrics[arm] = json!({"steps":rec.len(),"builds":c.candidate_builds,"rejections":c.candidate_rejections,"rhs_calls":c.rhs_calls,"halvings":c.step_halvings,"final_bound":last.state.error(),"physical_bound":err,"fft_work":if arm==1{json!({"rhs_calls":fw.rhs_calls,"forward_2d":fw.forward_2d,"inverse_2d":fw.inverse_2d,"grid_points":fw.padded_grid_points,"max_grid_points":fw.max_grid_points,"transformed_points":7*fw.padded_grid_points})}else{work_json(ww)}});
                }
            }
        }
        for arm in 0..4 {
            println!(
                "DRIVER_TIMING {}",
                json!({"omega":omega,"arm":(["direct","old-fft","warp-full","warp-crt"][arm]),"median_ms":median(&times[arm]),"samples_ms":times[arm],"metrics":metrics[arm]})
            );
        }
    }
}
