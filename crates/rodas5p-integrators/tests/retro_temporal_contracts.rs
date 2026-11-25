use rodas5p_core::DenseMatrix;
use rodas5p_integrators::{ParallelExecution, retro_temporal::*};
#[test]
fn singular_closure_still_recovers_original_initial_value_problem() {
    let table = TemporalTable::new(16, 1.).unwrap();
    let pattern = [DenseMatrix::identity(2)];
    let p = Prepared::new(&table, &pattern).unwrap();
    let b = vec![vec![0.125, -0.25]; 16];
    let r = p
        .solve(
            &[1., 2.],
            &b,
            Mode::Crt,
            &ParallelExecution::sequential(),
            1e-10,
        )
        .unwrap();
    assert!(r.accepted);
    for (i, x) in r.values.iter().enumerate() {
        assert!((x[0] - (1. + (i + 1) as f64 * 0.125)).abs() <= r.error_upper);
        assert!((x[1] - (2. - (i + 1) as f64 * 0.25)).abs() <= r.error_upper);
    }
}

use serde_json::json;

fn difference(a: &[Vec<f64>], b: &[Vec<f64>]) -> f64 {
    a.iter()
        .zip(b)
        .flat_map(|(x, y)| x.iter().zip(y))
        .fold(0.0_f64, |e, (&a, &b)| e.max((a - b).abs()))
}

fn ordered_pattern() -> Vec<DenseMatrix> {
    vec![
        DenseMatrix::from_rows(&[&[0.75, 0.125, 0.], &[0., 0.75, 0.125], &[0., 0., 0.5]]).unwrap(),
        DenseMatrix::from_rows(&[&[0.5, 0., 0.], &[-0.125, 0.75, 0.], &[0.125, 0., 0.75]]).unwrap(),
        DenseMatrix::from_rows(&[&[0.75, 0., -0.125], &[0., 0.5, 0.], &[0., 0.125, 0.75]]).unwrap(),
    ]
}
fn manufactured(pattern: &[DenseMatrix], m: usize) -> (Vec<f64>, Vec<Vec<f64>>, Vec<Vec<f64>>) {
    let n = pattern[0].nrows();
    let y = (0..n).map(|i| (i as f64 - 1.) / 8.).collect::<Vec<_>>();
    let mut exact: Vec<Vec<f64>> = Vec::new();
    let mut rhs = Vec::new();
    for i in 0..m * pattern.len() {
        let x: Vec<_> = (0..n)
            .map(|j| (((i * 7 + j * 5) % 19) as f64 - 9.) / 16.)
            .collect();
        // dyadic small coefficients, no rounded products in this fixture.
        let gy = pattern[i % pattern.len()]
            .matvec(exact.last().map_or(&y, |v| v))
            .unwrap();
        rhs.push(x.iter().zip(gy).map(|(&a, b)| a - b).collect());
        exact.push(x);
    }
    (y, rhs, exact)
}
#[test]
fn ordered_noncommuting_period_and_both_closure_signs() {
    let pattern = ordered_pattern();
    let comm = pattern[0]
        .matmul(&pattern[1])
        .unwrap()
        .sub(&pattern[1].matmul(&pattern[0]).unwrap())
        .unwrap();
    assert!(comm.as_slice().iter().any(|x| x.abs() > 0.01));
    let exec = ParallelExecution::rayon(3).unwrap();
    for m in [2, 8, 64] {
        let (y, b, exact) = manufactured(&pattern, m);
        for alpha in [1., 0.25, 1. / 4096.] {
            let table = TemporalTable::new(m, alpha).unwrap();
            let prep = Prepared::new(&table, &pattern).unwrap();
            assert_eq!(table.dyadic_levels(), m.trailing_zeros() as usize);
            let seq = solve_sequential(&pattern, &y, &b, 1e-10).unwrap();
            assert_eq!(seq.values, exact);
            for mode in [Mode::Scan, Mode::Padded, Mode::Crt] {
                let r = prep.solve(&y, &b, mode, &exec, 1e-10).unwrap();
                assert!(
                    r.accepted,
                    "m={m} alpha={alpha} mode={mode:?} bound={}",
                    r.error_upper
                );
                assert_eq!(r.values.len(), b.len());
                let e = difference(&r.values, &exact);
                assert!(e <= r.error_upper);
                for (i, (a, b)) in r.values.iter().zip(&exact).enumerate() {
                    assert!(
                        a.iter()
                            .zip(b)
                            .all(|(a, b)| (a - b).abs() <= r.certificate.node_error_upper[i])
                    );
                }
                if mode == Mode::Crt {
                    assert_eq!(r.work.forward_ffts, 2 * y.len());
                    assert_eq!(r.work.inverse_ffts, 2 * y.len());
                }
                println!(
                    "ORDERED {}",
                    json!({"m":m,"alpha":alpha,"mode":mode,"error":e,"bound":r.error_upper,"residual":r.certificate.residual_upper,"work":r.work})
                );
            }
        }
    }
}
#[test]
fn period_prefix_bound_preserves_signed_cancellation_and_rejects_wrong_path() {
    let pattern = vec![
        DenseMatrix::from_rows(&[&[1., 8.], &[0., 1.]]).unwrap(),
        DenseMatrix::from_rows(&[&[0.5, -4.], &[0., 0.25]]).unwrap(),
    ];
    let table = TemporalTable::new(64, 1.).unwrap();
    let prep = Prepared::new(&table, &pattern).unwrap();
    assert_eq!(prep.transfer().as_slice(), &[0.5, 0., 0., 0.25]);
    let (y, b, exact) = manufactured(&pattern, 64);
    let r = prep
        .solve(&y, &b, Mode::Crt, &ParallelExecution::sequential(), 1e-10)
        .unwrap();
    assert!(r.accepted);
    assert!(difference(&r.values, &exact) <= r.error_upper);
    let mut wrong = r.values.clone();
    wrong[17][1] += 1. / 1024.;
    let c = prep.certify(&y, &b, &wrong, 1e-10).unwrap();
    assert!(!c.accepted);
    assert!(difference(&wrong, &exact) <= c.error_upper);
    let mut wrong_ic = y.clone();
    wrong_ic[0] += 0.1;
    assert!(
        !prep
            .certify(&wrong_ic, &b, &r.values, 1e-10)
            .unwrap()
            .accepted
    );
    println!(
        "CANCEL {}",
        json!({"error":difference(&r.values,&exact),"bound":r.error_upper,"wrong_bound":c.error_upper,"wrong_residual":c.residual_upper})
    );
}
#[test]
fn nonfinite_shape_and_budget_refusals() {
    for (m, a) in [
        (1, 1.),
        (3, 1.),
        (8192, 1.),
        (16, 0.),
        (16, f64::NAN),
        (16, 1e-10),
    ] {
        assert!(TemporalTable::new(m, a).is_err());
    }
    let table = TemporalTable::new(8, 1.).unwrap();
    assert!(Prepared::new(&table, &[]).is_err());
    let bad = DenseMatrix::from_rows(&[&[1e200]]).unwrap();
    assert!(Prepared::new(&table, &[bad]).is_err());
    let pattern = ordered_pattern();
    let prep = Prepared::new(&table, &pattern).unwrap();
    let (y, mut b, _) = manufactured(&pattern, 8);
    assert!(
        prep.solve(&y, &b, Mode::Crt, &ParallelExecution::sequential(), 0.)
            .is_err()
    );
    assert!(
        prep.solve(
            &y,
            &b[..b.len() - 1],
            Mode::Scan,
            &ParallelExecution::sequential(),
            1e-10
        )
        .is_err()
    );
    b[2][1] = f64::INFINITY;
    assert!(
        prep.solve(&y, &b, Mode::Crt, &ParallelExecution::sequential(), 1e-10)
            .is_err()
    );
}
fn bits(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}
fn encode(v: &[Vec<f64>]) -> Vec<Vec<String>> {
    v.iter()
        .map(|x| x.iter().map(|&x| bits(x)).collect())
        .collect()
}
#[test]
fn source_owned_piecewise_shear_flow_and_exact_binary_audit() {
    // A=-4 I + S, S^2=0, alternating upper/lower shear. h=1/16.
    // Exact analytic step map is represented in binary64. This certificate
    // applies to that represented recurrence, not to exp/forcing roundoff.
    let h = 1. / 16.;
    let decay = (-0.25_f64).exp();
    let phi1 = -(-0.25_f64).exp_m1() / 0.25;
    let j1 = (1. - 1.25 * decay) / (0.25 * 0.25);
    let s = [
        DenseMatrix::from_rows(&[&[0., 2.], &[0., 0.]]).unwrap(),
        DenseMatrix::from_rows(&[&[0., 0.], &[-3., 0.]]).unwrap(),
    ];
    let pattern: Vec<_> = s
        .iter()
        .map(|s| {
            DenseMatrix::new(
                2,
                2,
                (0..4)
                    .map(|i| decay * (f64::from(i / 2 == i % 2) + h * s.as_slice()[i]))
                    .collect(),
            )
            .unwrap()
        })
        .collect();
    let y = vec![0.5, -0.25];
    let m = 64;
    let forcing: Vec<Vec<f64>> = (0..2 * m)
        .map(|i| {
            vec![
                ((3 * i % 11) as f64 - 5.) / 16.,
                ((7 * i % 13) as f64 - 6.) / 8.,
            ]
        })
        .collect();
    let b: Vec<Vec<f64>> = forcing
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let sf = s[i % 2].matvec(f).unwrap();
            f.iter()
                .zip(sf)
                .map(|(a, b)| h * phi1 * a + h * h * j1 * b)
                .collect()
        })
        .collect();
    let table = TemporalTable::new(m, 0.25).unwrap();
    let prep = Prepared::new(&table, &pattern).unwrap();
    let seq = solve_sequential(&pattern, &y, &b, 1e-10).unwrap();
    for mode in [Mode::Sequential, Mode::Scan, Mode::Padded, Mode::Crt] {
        let r = prep
            .solve(&y, &b, mode, &ParallelExecution::rayon(4).unwrap(), 1e-10)
            .unwrap();
        assert!(r.accepted);
        assert!(difference(&seq.values, &r.values) <= 1.01 * (seq.error_upper + r.error_upper));
        let directed = prep.certify_directed(&y, &b, &r.values, 1e-10).unwrap();
        assert!(directed.accepted);
        println!(
            "EXACT_CASE {}",
            json!({"m":m,"mode":mode,"pattern":pattern.iter().map(|a|a.as_slice().iter().map(|&x|bits(x)).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "y":y.iter().map(|&x|bits(x)).collect::<Vec<_>>(),"b":encode(&b),"forcing":encode(&forcing),"x":encode(&r.values),"certificate":r.certificate,"directed_certificate":directed,"work":r.work})
        );
    }
}
fn dense_fixture(n: usize, p: usize, m: usize) -> (Vec<DenseMatrix>, Vec<f64>, Vec<Vec<f64>>) {
    let mut seed = 0x6a09_e667_f3bc_c909_u64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        ((seed >> 40) as f64) / (1u64 << 24) as f64 * 2. - 1.
    };
    let pattern = (0..p)
        .map(|k| {
            let mut g = DenseMatrix::zeros(n, n);
            for i in 0..n {
                for j in 0..n {
                    g[(i, j)] = if i == j {
                        0.86 - 0.02 * k as f64 + 0.02 * next()
                    } else {
                        0.06 * next() / n as f64
                    };
                }
            }
            g
        })
        .collect();
    let y = (0..n).map(|_| next()).collect();
    let b = (0..m * p)
        .map(|_| (0..n).map(|_| 0.1 * next()).collect())
        .collect();
    (pattern, y, b)
}
fn median(v: &[f64]) -> f64 {
    let mut x = v.to_vec();
    x.sort_by(f64::total_cmp);
    x[x.len() / 2]
}
#[test]
#[ignore = "owner-host diagnostic timing; shared output and independent residual certificate"]
fn measure_temporal() {
    use std::{hint::black_box, time::Instant};
    faer::set_global_parallelism(faer::Par::Seq);
    let one = ParallelExecution::sequential();
    let four = ParallelExecution::rayon(4).unwrap();
    let modes = [Mode::Sequential, Mode::Scan, Mode::Padded, Mode::Crt];
    for (n, p, m) in [(4, 1, 64), (16, 2, 256), (32, 2, 1024), (64, 2, 1024)] {
        let (pattern, y, b) = dense_fixture(n, p, m);
        let table = TemporalTable::new(m, 1.).unwrap();
        let prepared = Prepared::new(&table, &pattern).unwrap();
        let exact = solve_sequential(&pattern, &y, &b, 1e-10).unwrap();
        let mut raw: [Vec<f64>; 4] = std::array::from_fn(|_| Vec::new());
        let mut cert: [Vec<f64>; 4] = std::array::from_fn(|_| Vec::new());
        let mut fresh: [Vec<f64>; 4] = std::array::from_fn(|_| Vec::new());
        let mut meta: [serde_json::Value; 4] = std::array::from_fn(|_| serde_json::Value::Null);
        for rep in 0..6 {
            for off in 0..4 {
                let i = (rep + off) % 4;
                let mode = modes[i];
                let exec = if i == 0 { &one } else { &four };
                let t = Instant::now();
                let (x, w) = prepared.candidate(&y, &b, mode, exec).unwrap();
                black_box(&x);
                let dt = t.elapsed().as_secs_f64() * 1000.;
                let t = Instant::now();
                let c = prepared.certify(&y, &b, &x, 1e-10).unwrap();
                let dc = t.elapsed().as_secs_f64() * 1000.;
                assert!(c.accepted);
                assert!(
                    difference(&exact.values, &x) <= 1.01 * (exact.error_upper + c.error_upper)
                );
                let t = Instant::now();
                let r = if i == 0 {
                    solve_sequential(&pattern, &y, &b, 1e-10).unwrap()
                } else {
                    let tb = TemporalTable::new(m, 1.).unwrap();
                    let pr = Prepared::new(&tb, &pattern).unwrap();
                    pr.solve(&y, &b, mode, exec, 1e-10).unwrap()
                };
                let df = t.elapsed().as_secs_f64() * 1000.;
                assert!(r.accepted);
                assert_eq!(r.values, x);
                if rep > 0 {
                    raw[i].push(dt);
                    cert[i].push(dc);
                    fresh[i].push(df);
                }
                meta[i] = json!({"difference_to_sequential":difference(&x,&exact.values),"error_upper":c.error_upper,"residual_upper":c.residual_upper,
                    "candidate_work":w,"certified_rows":r.work.residual_rows,"prepared_candidate_matrix_products":if i==0{0}else{prepared.prepared_matrix_products()}});
            }
        }
        for i in 0..4 {
            println!(
                "WP {}",
                json!({"n":n,"p":p,"m":m,"mode":modes[i],"threads":if i==0{1}else{4},"tol":1e-10,
                "raw_ms":median(&raw[i]),"certificate_ms":median(&cert[i]),"fresh_ms":median(&fresh[i]),
                "prepared_total_ms":median(&raw[i].iter().zip(&cert[i]).map(|(a,b)|a+b).collect::<Vec<_>>()),
                "raw_samples":raw[i],"certificate_samples":cert[i],"fresh_samples":fresh[i],"metrics":meta[i]})
            );
        }
    }
}

#[test]
fn row_rounding_encloses_small_and_large_represented_inputs() {
    let pattern = [DenseMatrix::from_rows(&[&[0.5, 0.125], &[-0.125, 0.5]]).unwrap()];
    let table = TemporalTable::new(16, 1.).unwrap();
    let p = Prepared::new(&table, &pattern).unwrap();
    for scale in [f64::from_bits(1), 2f64.powi(-500), 1., 2f64.powi(500)] {
        let y = vec![scale, -scale];
        let b = vec![vec![scale, 0.5 * scale]; 16];
        let (mut x, _) = p
            .candidate(&y, &b, Mode::Crt, &ParallelExecution::sequential())
            .unwrap();
        x[4][0] += scale * 0.125;
        let tol = 1e-10 * scale.max(1.);
        let c = p.certify(&y, &b, &x, tol).unwrap();
        let d = p.certify_directed(&y, &b, &x, tol).unwrap();
        if scale >= 1. {
            assert!(!c.accepted && !d.accepted);
        }
        println!(
            "SCALE_CASE {}",
            json!({"scale":bits(scale),"pattern":[pattern[0].as_slice().iter().map(|&v|bits(v)).collect::<Vec<_>>()],"y":y.iter().map(|&v|bits(v)).collect::<Vec<_>>(),"b":encode(&b),"x":encode(&x),"certificate":c,"directed_certificate":d})
        );
    }
}
