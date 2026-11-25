use rodas5p_core::{DenseMatrix, warp_cubic::CubicPacket};

#[test]
fn prepare_noncommuting_confluent_pencil() {
    let w = DenseMatrix::from_rows(&[&[4., 1.], &[0., 3.]]).unwrap();
    let d = DenseMatrix::from_rows(&[&[0., 0.], &[1., 0.]]).unwrap();
    assert!(CubicPacket::prepare(&w, &d, [0.; 3]).is_ok());
}

use rodas5p_core::shared_shift_jet::{CertificateStatus, SharedShiftJetConfig, shared_shift_jet};
use rodas5p_core::warp_cubic::{CubicCoefficients, CubicConfig, CubicPencil, MAX_UPDATES};
use rodas5p_core::{LuFactorization, LuSolveWorkspace, direct_solve};

fn norm(x: &CubicCoefficients) -> f64 {
    x.iter()
        .map(|v| v.iter().fold(0.0_f64, |a, b| a.max(b.abs())))
        .sum()
}
fn difference(x: &CubicCoefficients, y: &CubicCoefficients) -> f64 {
    let d = std::array::from_fn(|j| x[j].iter().zip(&y[j]).map(|(a, b)| a - b).collect());
    norm(&d)
}
// Independent block operator in the ordered coefficient basis. No K,
// elimination, or recurrence from the candidate is used.
fn block_matrix(w: &DenseMatrix, d: &DenseMatrix, p: [f64; 3]) -> DenseMatrix {
    let n = w.nrows();
    let mut a = DenseMatrix::zeros(3 * n, 3 * n);
    for i in 0..n {
        for j in 0..n {
            for k in 0..3 {
                a[(k * n + i, k * n + j)] = w[(i, j)];
                if k < 2 {
                    a[((k + 1) * n + i, k * n + j)] = d[(i, j)];
                }
            }
            for k in 0..3 {
                a[(k * n + i, 2 * n + j)] -= p[k] * d[(i, j)];
            }
        }
    }
    a
}
fn flat(x: &CubicCoefficients) -> Vec<f64> {
    x.iter().flatten().copied().collect()
}
fn unflat(x: &[f64], n: usize) -> CubicCoefficients {
    std::array::from_fn(|j| x[j * n..(j + 1) * n].to_vec())
}
fn oracle(
    w: &DenseMatrix,
    d: &DenseMatrix,
    p: [f64; 3],
    b: &CubicCoefficients,
) -> CubicCoefficients {
    unflat(
        &direct_solve(&block_matrix(w, d, p), &flat(b)).unwrap(),
        w.nrows(),
    )
}
fn evaluate(x: &CubicCoefficients, t: f64) -> Vec<f64> {
    (0..x[0].len())
        .map(|i| t.mul_add(t.mul_add(x[2][i], x[1][i]), x[0][i]))
        .collect()
}
fn shifted(w: &DenseMatrix, d: &DenseMatrix, t: f64) -> DenseMatrix {
    let n = w.nrows();
    DenseMatrix::new(
        n,
        n,
        w.as_slice()
            .iter()
            .zip(d.as_slice())
            .map(|(a, b)| t.mul_add(*b, *a))
            .collect(),
    )
    .unwrap()
}
fn modal_reference(
    w: &DenseMatrix,
    d: &DenseMatrix,
    b: &CubicCoefficients,
    s: f64,
) -> CubicCoefficients {
    // Three separate LU factor/solve calls, then the unique coefficient
    // interpolation at [-s,0,s]. This comparator is defined for s!=0 only.
    let l = direct_solve(&shifted(w, d, -s), &evaluate(b, -s)).unwrap();
    let z = direct_solve(w, &b[0]).unwrap();
    let r = direct_solve(&shifted(w, d, s), &evaluate(b, s)).unwrap();
    let x1 = l.iter().zip(&r).map(|(a, b)| (b - a) / (2.0 * s)).collect();
    let x2 = l
        .iter()
        .zip(&r)
        .zip(&z)
        .map(|((a, b), c)| ((a - c) + (b - c)) / (2.0 * s * s))
        .collect();
    [z, x1, x2]
}
fn fixture(n: usize) -> (DenseMatrix, DenseMatrix, CubicCoefficients) {
    let mut state = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        ((state >> 40) as f64) / (1u64 << 24) as f64 * 2.0 - 1.0
    };
    let mut w = DenseMatrix::zeros(n, n);
    let mut d = DenseMatrix::zeros(n, n);
    for i in 0..n {
        for j in 0..n {
            w[(i, j)] = if i == j {
                2.0 + 0.125 * (i % 7) as f64
            } else {
                0.5 * next() / n as f64
            };
            d[(i, j)] = if i == j {
                0.125 * next()
            } else {
                0.1 * next() / n as f64
            };
        }
    }
    let b = std::array::from_fn(|_| (0..n).map(|_| next()).collect());
    (w, d, b)
}

#[test]
fn exact_confluence_keeps_all_three_independent_coefficients() {
    let w = DenseMatrix::from_rows(&[&[4., 1.], &[0., 3.]]).unwrap();
    let d = DenseMatrix::from_rows(&[&[0., 0.], &[1., 0.]]).unwrap();
    let x = [vec![1., 0.5], vec![-0.25, 2.], vec![0.125, -0.5]];
    let p = [0.; 3];
    let b = unflat(&block_matrix(&w, &d, p).matvec(&flat(&x)).unwrap(), 2);
    let mut pack = CubicPacket::prepare(&w, &d, p).unwrap();
    let r = pack.solve(&b, CubicConfig::default()).unwrap();
    assert_eq!(r.certificate().status(), CertificateStatus::Certified);
    assert!(difference(r.coefficients(), &x) <= r.certificate().error_upper());
    assert_eq!(r.work().center_solves, 3);
    assert_eq!(r.work().direction_matvecs, 2);
    assert_eq!(r.work().correction_updates, 0);
    let mut wrong = r.coefficients().clone();
    wrong[2][0] += 1.;
    // The collapsed point value alone is unchanged, but the jet is wrong.
    assert_eq!(wrong[0], r.coefficients()[0]);
    let bad = pack.pencil().certify(&b, &wrong, 1e-10).unwrap();
    assert_eq!(bad.status(), CertificateStatus::Rejected);
    assert!(bad.error_upper() >= difference(&wrong, &x));
    let second = pack
        .solve(
            &[vec![0.; 2], vec![0.; 2], vec![1., 0.]],
            CubicConfig::default(),
        )
        .unwrap();
    assert_eq!(second.certificate().status(), CertificateStatus::Certified);
    assert_eq!(second.coefficients()[0], vec![0.; 2]);
    assert_eq!(second.coefficients()[1], vec![0.; 2]);
}

#[test]
fn full_cubic_block_oracle_and_dyadic_manufactured_certificates() {
    let w = DenseMatrix::from_rows(&[&[4., 1.], &[0., 3.]]).unwrap();
    let d = DenseMatrix::from_rows(&[&[0., 0.], &[1., 0.]]).unwrap();
    let exact = [vec![1., 0.5], vec![-0.25, 2.], vec![0.125, -0.5]];
    let ps = [
        [0.; 3],
        [0., -0.25, 0.],
        [0., 0.25, 0.],
        [0.015625, -0.15625, 0.125],
        [0.0078125, 0., -0.375],
        [0., -1., 0.],
        [0.25, 0.125, -0.25],
    ];
    for p in ps {
        let b = unflat(&block_matrix(&w, &d, p).matvec(&flat(&exact)).unwrap(), 2);
        let mut pack = CubicPacket::prepare(&w, &d, p).unwrap();
        let r = pack.solve(&b, CubicConfig::default()).unwrap();
        let err = difference(r.coefficients(), &exact);
        assert_eq!(
            r.certificate().status(),
            CertificateStatus::Certified,
            "p={p:?}, {r:?}"
        );
        assert!(
            err <= r.certificate().error_upper(),
            "p={p:?} err={err} {r:?}"
        );
        assert!(difference(r.coefficients(), &oracle(&w, &d, p, &b)) < 1e-10);
        println!(
            "MANUFACTURED p={p:?} error={err:e} bound={:e} work={:?}",
            r.certificate().error_upper(),
            r.work()
        );
    }
    for n in [3, 17, 48] {
        let (w, d, b) = fixture(n);
        for p in ps {
            let mut pack = CubicPacket::prepare(&w, &d, p).unwrap();
            let r = pack.solve(&b, CubicConfig::default()).unwrap();
            let o = oracle(&w, &d, p, &b);
            let oracle_cert = pack.pencil().certify(&b, &o, 1e-10).unwrap();
            assert_eq!(r.certificate().status(), CertificateStatus::Certified);
            assert_eq!(oracle_cert.status(), CertificateStatus::Certified);
            assert!(
                difference(r.coefficients(), &o)
                    <= 1.01 * (r.certificate().error_upper() + oracle_cert.error_upper())
            );
        }
    }
}

#[test]
fn real_and_complex_modal_equations_and_repeated_root_derivative() {
    let (w, d, b) = fixture(17);
    for (p, roots) in [
        ([0., -0.25, 0.], vec![(-0.5, 0.), (0., 0.), (0.5, 0.)]),
        ([0., 0.25, 0.], vec![(0., -0.5), (0., 0.), (0., 0.5)]),
        (
            [0.015625, -0.15625, 0.125],
            vec![(-0.5, 0.), (0.125, 0.), (0.25, 0.)],
        ),
        ([0.0078125, 0., -0.375], vec![(-0.125, 0.), (0.25, 0.)]),
    ] {
        let mut pack = CubicPacket::prepare(&w, &d, p).unwrap();
        let r = pack.solve(&b, CubicConfig::default()).unwrap();
        assert_eq!(r.certificate().status(), CertificateStatus::Certified);
        for (re, im) in roots {
            let (xr, xi) = r.evaluate(re, im).unwrap();
            let wr = shifted(&w, &d, re);
            let ar = wr.matvec(&xr).unwrap();
            let ai = wr.matvec(&xi).unwrap();
            let dr = d.matvec(&xr).unwrap();
            let di = d.matvec(&xi).unwrap();
            for i in 0..xr.len() {
                let br = b[0][i] + re * b[1][i] + (re * re - im * im) * b[2][i];
                let bi = im * b[1][i] + 2.0 * re * im * b[2][i];
                assert!((ar[i] - im * di[i] - br).abs() < 1e-10);
                assert!((ai[i] + im * dr[i] - bi).abs() < 1e-10);
            }
        }
        if p[2] == -0.375 {
            let t = 0.25;
            let (x, _) = r.evaluate(t, 0.).unwrap();
            let xp: Vec<_> = r.coefficients()[1]
                .iter()
                .zip(&r.coefficients()[2])
                .map(|(a, b)| a + 2.0 * t * b)
                .collect();
            let wx = shifted(&w, &d, t).matvec(&xp).unwrap();
            let dx = d.matvec(&x).unwrap();
            for i in 0..x.len() {
                assert!((wx[i] + dx[i] - b[1][i] - 2.0 * t * b[2][i]).abs() < 1e-10);
            }
        }
    }
}

#[test]
fn contraction_and_accuracy_refusals_are_not_singularity_claims() {
    let w = DenseMatrix::identity(2);
    let d = DenseMatrix::from_rows(&[&[0., 0.], &[0.5, 0.]]).unwrap();
    for p in [[f64::NAN, 0., 0.], [1., 0.25, 0.]] {
        assert!(CubicPacket::prepare(&w, &d, p).is_err());
    }
    let singular = DenseMatrix::zeros(2, 2);
    assert!(CubicPacket::prepare(&singular, &d, [0.; 3]).is_err());
    assert!(CubicPacket::prepare(&w, &w, [0.; 3]).is_err());
    let nonsdd = DenseMatrix::from_rows(&[&[0., 1.], &[1., 0.]]).unwrap();
    assert!(CubicPacket::prepare(&nonsdd, &d, [0.; 3]).is_err());
    let (w, d, b) = fixture(8);
    let mut pack = CubicPacket::prepare(&w, &d, [0.25, 0.125, -0.25]).unwrap();
    let low = pack
        .solve(
            &b,
            CubicConfig {
                max_updates: 0,
                ..CubicConfig::default()
            },
        )
        .unwrap();
    assert_eq!(low.certificate().status(), CertificateStatus::Rejected);
    assert!(
        pack.solve(
            &b,
            CubicConfig {
                max_updates: MAX_UPDATES + 1,
                ..CubicConfig::default()
            }
        )
        .is_err()
    );
    let mut bad = b.clone();
    bad[2][0] = f64::INFINITY;
    assert!(pack.solve(&bad, CubicConfig::default()).is_err());
    assert!(pack.pencil().certify(&b, &bad, 1e-10).is_err());
    let mut bad = b.clone();
    bad[1].pop();
    assert!(pack.solve(&bad, CubicConfig::default()).is_err());
    assert!(
        pack.solve(
            &b,
            CubicConfig {
                absolute_tolerance: 0.,
                ..CubicConfig::default()
            }
        )
        .is_err()
    );
    let tiny = pack
        .solve(
            &b,
            CubicConfig {
                absolute_tolerance: f64::MIN_POSITIVE,
                ..CubicConfig::default()
            },
        )
        .unwrap();
    assert_eq!(tiny.certificate().status(), CertificateStatus::Rejected);
}

#[test]
fn confluence_sweep_preserves_derivative_information() {
    let (w, d, b) = fixture(17);
    for s in [0.5_f64, 1e-2, 1e-4, 1e-6, 1e-8, 0.] {
        let p = [0., -s * s, 0.];
        let mut pack = CubicPacket::prepare(&w, &d, p).unwrap();
        let r = pack.solve(&b, CubicConfig::default()).unwrap();
        let o = oracle(&w, &d, p, &b);
        assert_eq!(r.certificate().status(), CertificateStatus::Certified);
        assert!(difference(r.coefficients(), &o) < 1e-10);
        let direct = if s > 0. {
            let x = modal_reference(&w, &d, &b, s);
            let c = pack.pencil().certify(&b, &x, 1e-10).unwrap();
            Some((difference(&x, &o), c.error_upper(), c.status()))
        } else {
            None
        };
        println!(
            "CONFLUENCE s={s:e} packet_error={:e} packet_bound={:e} direct={direct:?}",
            difference(r.coefficients(), &o),
            r.certificate().error_upper()
        );
    }
}

#[test]
fn shared_shift_compatible_pencil_agrees_at_the_same_targets() {
    let n = 16;
    let (w, _, b) = fixture(n);
    let mut j = DenseMatrix::identity(n);
    for (v, a) in j.as_mut_slice().iter_mut().zip(w.as_slice()) {
        *v -= a;
    }
    let d = DenseMatrix::new(n, n, j.as_slice().iter().map(|a| -0.25 * a).collect()).unwrap();
    let b = [b[0].clone(), vec![0.; n], vec![0.; n]];
    let p = [0., -0.25, 0.];
    let mut pack = CubicPacket::prepare(&w, &d, p).unwrap();
    let r = pack.solve(&b, CubicConfig::default()).unwrap();
    assert_eq!(r.certificate().status(), CertificateStatus::Certified);
    let old = shared_shift_jet(
        &j,
        1.,
        1.,
        &[b[0].clone()],
        &[0.875, 1., 1.125],
        SharedShiftJetConfig::default(),
    )
    .unwrap();
    for (c, t) in old.candidates().iter().zip([-0.5, 0., 0.5]) {
        assert_eq!(c.certificate().status(), CertificateStatus::Certified);
        let (x, _) = r.evaluate(t, 0.).unwrap();
        assert!(
            x.iter()
                .zip(&c.rhs_columns()[0])
                .all(|(a, b)| (a - b).abs() < 1e-10)
        );
    }
}

// A plain coefficient Picard baseline, using the same center factorization but
// updating all three independent channels every iteration. No shared cubic
// elimination or point interpolation is used.
fn full_picard(
    w: &DenseMatrix,
    d: &DenseMatrix,
    p: [f64; 3],
    b: &CubicCoefficients,
    levels: usize,
) -> CubicCoefficients {
    let lu = LuFactorization::new(w).unwrap();
    let mut scratch = LuSolveWorkspace::new(w.nrows());
    let mut x = b.clone();
    for v in &mut x {
        lu.solve_in_place_with_workspace(v, &mut scratch).unwrap();
    }
    let seed = x.clone();
    let n = w.nrows();
    for _ in 0..levels {
        let mut v: CubicCoefficients = std::array::from_fn(|j| {
            (0..n)
                .map(|i| {
                    if j == 0 {
                        -p[j] * x[2][i]
                    } else {
                        x[j - 1][i] - p[j] * x[2][i]
                    }
                })
                .collect()
        });
        for j in 0..3 {
            v[j] = d.matvec(&v[j]).unwrap();
            lu.solve_in_place_with_workspace(&mut v[j], &mut scratch)
                .unwrap();
            for (a, s) in v[j].iter_mut().zip(&seed[j]) {
                *a = *s - *a;
            }
        }
        x = v;
    }
    x
}

#[test]
#[ignore = "single-core paired diagnostic kernel timings"]
fn measure_work_precision() {
    use std::time::Instant;
    faer::set_global_parallelism(faer::Par::Seq);
    let p = [0., -0.25, 0.];
    for n in [32, 128, 512, 1024] {
        let (w, d, b) = fixture(n);
        let pencil = CubicPencil::new(&w, &d, p).unwrap();
        let refx = modal_reference(&w, &d, &b, 0.5);
        for tol in [1e-10, 2e-12] {
            assert_eq!(
                pencil.certify(&b, &refx, tol).unwrap().status(),
                CertificateStatus::Certified
            );
            // Select smallest measured integer Picard depth attaining the same
            // coefficient tolerance; this tuning is not included in timings.
            let picard_levels = (0..=24)
                .find(|&l| {
                    pencil
                        .certify(&b, &full_picard(&w, &d, p, &b, l), tol)
                        .unwrap()
                        .status()
                        == CertificateStatus::Certified
                })
                .unwrap();
            let mut samples: [Vec<f64>; 3] = std::array::from_fn(|_| Vec::new());
            let mut bounds = [0.; 3];
            let mut diffs = [0.; 3];
            let mut work = 0;
            let mut corrections = 0;
            for rep in 0..6 {
                for offset in 0..3 {
                    let arm = (rep + offset) % 3;
                    let start = Instant::now();
                    let (x, c) = match arm {
                        0 => {
                            let mut pack = CubicPacket::prepare(&w, &d, p).unwrap();
                            let r = pack
                                .solve(
                                    &b,
                                    CubicConfig {
                                        absolute_tolerance: tol,
                                        ..CubicConfig::default()
                                    },
                                )
                                .unwrap();
                            work = r.work().center_solves;
                            corrections = r.work().correction_updates;
                            (r.coefficients().clone(), *r.certificate())
                        }
                        1 => {
                            let x = modal_reference(&w, &d, &b, 0.5);
                            let c = CubicPencil::new(&w, &d, p)
                                .unwrap()
                                .certify(&b, &x, tol)
                                .unwrap();
                            (x, c)
                        }
                        _ => {
                            let x = full_picard(&w, &d, p, &b, picard_levels);
                            let c = CubicPencil::new(&w, &d, p)
                                .unwrap()
                                .certify(&b, &x, tol)
                                .unwrap();
                            (x, c)
                        }
                    };
                    let ms = start.elapsed().as_secs_f64() * 1000.;
                    assert_eq!(c.status(), CertificateStatus::Certified);
                    let diff = difference(&x, &refx);
                    if rep > 0 {
                        samples[arm].push(ms);
                    }
                    bounds[arm] = c.error_upper();
                    diffs[arm] = diff;
                }
            }
            for arm in 0..3 {
                let mut sorted = samples[arm].clone();
                sorted.sort_by(f64::total_cmp);
                println!(
                    "WARP_WP {}",
                    serde_json::json!({"n":n,"arm":(["warp-elimination","direct-three-modal-lu","full-coefficient-picard"][arm]),"tol":tol,"bound":bounds[arm],"difference_to_modal":diffs[arm],"median_ms":sorted[2],"samples_ms":samples[arm],"picard_levels":picard_levels,"warp_center_solves":work,"warp_corrections":corrections,"q_upper":pencil.contraction_upper()})
                );
            }
        }
    }
}

#[test]
fn scaled_mirror_weighted_cubic_packet_is_not_an_affine_root_count() {
    // R3 handoff B: 3F0(x)=x^3+(9/2) E x^2+9M x+6U.
    // x=3 xi, so the normalized polynomial is monic with
    // p=[2U/9, M, 3E/2]. This is the weighted model, not the full radical
    // incidence or a count of regular real witnesses.
    let (w, d, b) = fixture(24);
    for m in [-0.4, 0.4] {
        let e = 0.2;
        let u = 0.08;
        let p = [2.0 * u / 9.0, m, 1.5 * e];
        let mut packet = CubicPacket::prepare(&w, &d, p).unwrap();
        let r = packet.solve(&b, CubicConfig::default()).unwrap();
        let o = oracle(&w, &d, p, &b);
        assert_eq!(r.certificate().status(), CertificateStatus::Certified);
        assert!(difference(r.coefficients(), &o) < 1e-10);
    }
}

// Exact binary64 inputs for a separate rational arithmetic audit. These
// include poor candidates, underflow-scale data and disparate magnitudes.
#[test]
fn emit_exact_binary_residual_fixtures() {
    let (w, d, b) = fixture(3);
    let mut count = 0;
    for scale in [2f64.powi(-500), 1., 2f64.powi(500)] {
        let ws = DenseMatrix::new(3, 3, w.as_slice().iter().map(|a| a * scale).collect()).unwrap();
        let ds = DenseMatrix::new(3, 3, d.as_slice().iter().map(|a| a * scale).collect()).unwrap();
        let bs: CubicCoefficients =
            std::array::from_fn(|j| b[j].iter().map(|a| a * scale).collect());
        for p in [[0.; 3], [0.015625, -0.15625, 0.125], [0., 0.25, 0.]] {
            let pencil = CubicPencil::new(&ws, &ds, p).unwrap();
            let mut packet = pencil.prepare().unwrap();
            let r = packet.solve(&bs, CubicConfig::default()).unwrap();
            let mut bad = r.coefficients().clone();
            bad[2][0] += 0.125;
            for x in [r.coefficients(), &bad] {
                let c = pencil.certify(&bs, x, 1e-10).unwrap();
                println!(
                    "EXACT_CASE {}",
                    serde_json::json!({"n":3,"w":ws.as_slice(),"d":ds.as_slice(),"p":p,"b":bs,"x":x,"residual_upper":c.residual_upper(),"error_upper":c.error_upper(),"status":c.status()})
                );
                count += 1;
            }
        }
    }
    let w = DenseMatrix::new(1, 1, vec![2.]).unwrap();
    let d = DenseMatrix::new(1, 1, vec![0.5]).unwrap();
    let p = [0., -0.25, 0.];
    let small = f64::from_bits(1);
    let b = [vec![small], vec![-small], vec![small]];
    let x = [vec![small], vec![small], vec![-small]];
    let c = CubicPencil::new(&w, &d, p)
        .unwrap()
        .certify(&b, &x, 1e-10)
        .unwrap();
    println!(
        "EXACT_CASE {}",
        serde_json::json!({"n":1,"w":w.as_slice(),"d":d.as_slice(),"p":p,"b":b,"x":x,"residual_upper":c.residual_upper(),"error_upper":c.error_upper(),"status":c.status()})
    );
    assert_eq!(count + 1, 19);
}

#[test]
#[ignore = "paired prepared-cache diagnostic; both arms reuse their factors"]
fn measure_prepared_cache() {
    use std::time::Instant;
    faer::set_global_parallelism(faer::Par::Seq);
    let p = [0., -0.25, 0.];
    for n in [128, 512, 1024] {
        let (w, d, b) = fixture(n);
        let pencil = CubicPencil::new(&w, &d, p).unwrap();
        let mut packet = pencil.prepare().unwrap();
        let factors = [
            LuFactorization::new(&shifted(&w, &d, -0.5)).unwrap(),
            LuFactorization::new(&w).unwrap(),
            LuFactorization::new(&shifted(&w, &d, 0.5)).unwrap(),
        ];
        let mut scratch = LuSolveWorkspace::new(n);
        let mut times: [Vec<f64>; 2] = std::array::from_fn(|_| Vec::new());
        let mut bounds = [0.; 2];
        for rep in 0..6 {
            // New independent RHS vectors by fixed deterministic scaling and
            // rotation, not a result cache. Input construction outside timing.
            let rhs: CubicCoefficients = std::array::from_fn(|j| {
                (0..n)
                    .map(|i| b[j][(i + rep * 7) % n] * (1.0 + rep as f64 / 16.0))
                    .collect()
            });
            for arm in [rep % 2, 1 - rep % 2] {
                let start = Instant::now();
                let c = if arm == 0 {
                    *packet
                        .solve(&rhs, CubicConfig::default())
                        .unwrap()
                        .certificate()
                } else {
                    let mut l = evaluate(&rhs, -0.5);
                    let mut z = rhs[0].clone();
                    let mut r = evaluate(&rhs, 0.5);
                    factors[0]
                        .solve_in_place_with_workspace(&mut l, &mut scratch)
                        .unwrap();
                    factors[1]
                        .solve_in_place_with_workspace(&mut z, &mut scratch)
                        .unwrap();
                    factors[2]
                        .solve_in_place_with_workspace(&mut r, &mut scratch)
                        .unwrap();
                    let x1 = l.iter().zip(&r).map(|(a, b)| b - a).collect();
                    let x2 = l
                        .iter()
                        .zip(&r)
                        .zip(&z)
                        .map(|((a, b), c)| 2.0 * ((a - c) + (b - c)))
                        .collect();
                    pencil.certify(&rhs, &[z, x1, x2], 1e-10).unwrap()
                };
                let ms = start.elapsed().as_secs_f64() * 1000.;
                assert_eq!(c.status(), CertificateStatus::Certified);
                if rep > 0 {
                    times[arm].push(ms);
                    bounds[arm] = f64::max(bounds[arm], c.error_upper());
                }
            }
        }
        for arm in 0..2 {
            let mut sorted = times[arm].clone();
            sorted.sort_by(f64::total_cmp);
            println!(
                "WARP_CACHE {}",
                serde_json::json!({"n":n,"arm":(["warp-prepared","three-lu-prepared"][arm]),"median_ms":sorted[2],"samples_ms":times[arm],"max_bound":bounds[arm],"tol":1e-10})
            );
        }
    }
}
