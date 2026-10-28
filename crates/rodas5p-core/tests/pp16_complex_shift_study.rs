//! Case exporter of research node `research/pp16_complex_shift_gain_20261004`
//! (RVJ DAG node PP16): complex-shift certificates with the inverse gain
//! `|gamma| / Re gamma`, on seeded dissipative `J`. Pade [3/3] and [6/6]
//! shifts and weights of `e^z` are read from the JSON written by
//! `tools/pp16_pade_shifts.py` (env `PP16_PADE`); everything is written to
//! `PP16_CASES` (repo-relative, never overwritten) as IEEE-754 hex bits and
//! checked by `tools/pp16_complex_shift_check.py`. Run with
//! `--ignored --release`. No timings.

use rodas5p_core::complex_shift_jet::{
    ComplexShiftCertificate, ComplexShiftJetConfig, PartialFractionTerm,
    certify_complex_shift_candidate, certify_partial_fraction, complex_shift_jet,
};
use rodas5p_core::directed::{Interval, add_up};
use rodas5p_core::shared_shift_jet::CertificateStatus;
use rodas5p_core::{CoreResult, DenseMatrix, LuFactorization};
use serde_json::{Value, json};

const TOLERANCE: f64 = 1e-10;

fn hex(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}

fn hexes(v: &[f64]) -> Vec<String> {
    v.iter().map(|x| hex(*x)).collect()
}

fn unhex(value: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(value.as_str().unwrap(), 16).unwrap())
}

struct SplitMix(u64);

impl SplitMix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// Exact binary value in [0, 1).
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 * 2f64.powi(-53)
    }

    fn symmetric(&mut self) -> f64 {
        2.0 * self.unit() - 1.0
    }
}

/// Skew part of scale 10 plus a symmetric part of scale 1; the diagonal is
/// set below the outward symmetric-part row bound by at least 0.05.
fn dissipative(n: usize, rng: &mut SplitMix) -> DenseMatrix {
    let mut j = DenseMatrix::zeros(n, n);
    for i in 0..n {
        for k in i + 1..n {
            let skew = 10.0 * rng.symmetric();
            let sym = rng.symmetric();
            j[(i, k)] = skew + sym;
            j[(k, i)] = -skew + sym;
        }
    }
    for i in 0..n {
        let mut row = 0.0;
        for k in 0..n {
            if k != i {
                let half = Interval::exact_sum(j[(i, k)], j[(k, i)])
                    .unwrap()
                    .scale(0.5)
                    .unwrap();
                row = add_up(row, half.mag()).unwrap();
            }
        }
        j[(i, i)] = -add_up(row, 0.05 + rng.unit()).unwrap();
    }
    j
}

/// Approximate candidate from the real 2n embedding `[[P, Q], [-Q, P]]`.
fn direct_candidate(
    j: &DenseMatrix,
    h: f64,
    gamma: (f64, f64),
    b_re: &[f64],
    b_im: &[f64],
) -> CoreResult<(Vec<f64>, Vec<f64>)> {
    let n = j.nrows();
    let mut big = DenseMatrix::zeros(2 * n, 2 * n);
    for i in 0..n {
        for k in 0..n {
            let p = if i == k { 1.0 } else { 0.0 } - gamma.0 * h * j[(i, k)];
            let q = gamma.1 * h * j[(i, k)];
            big[(i, k)] = p;
            big[(i, n + k)] = q;
            big[(n + i, k)] = -q;
            big[(n + i, n + k)] = p;
        }
    }
    let rhs: Vec<f64> = b_re.iter().chain(b_im).copied().collect();
    let x = LuFactorization::new(&big)?.solve(&rhs)?;
    Ok((x[..n].to_vec(), x[n..].to_vec()))
}

fn certificate_json(c: &ComplexShiftCertificate) -> Value {
    json!({
        "status": c.status(),
        "error_upper": hex(c.error_upper()),
        "residual_l2_upper": hex(c.residual_l2_upper()),
        "gain_upper": hex(c.gain_upper()),
        "absolute_tolerance": hex(c.absolute_tolerance()),
        "error_upper_decimal": c.error_upper(),
        "gain_upper_decimal": c.gain_upper(),
    })
}

#[allow(clippy::too_many_arguments)]
fn single(
    id: String,
    family: &str,
    operator: &str,
    j: &DenseMatrix,
    h: f64,
    gamma: (f64, f64),
    b: (&[f64], &[f64]),
    u: (&[f64], &[f64]),
) -> Value {
    let c = certify_complex_shift_candidate(j, h, gamma.0, gamma.1, b.0, b.1, u.0, u.1, TOLERANCE)
        .unwrap();
    json!({
        "id": id, "family": family, "operator": operator,
        "gamma_re": hex(gamma.0), "gamma_im": hex(gamma.1),
        "b_re": hexes(b.0), "b_im": hexes(b.1), "u_re": hexes(u.0), "u_im": hexes(u.1),
        "certificate": certificate_json(&c),
    })
}

struct Pade {
    k: u64,
    c0: (f64, f64),
    poles: Vec<((f64, f64), (f64, f64))>,
}

fn load_pade(path: &std::path::Path) -> Vec<Pade> {
    let data: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(data["schema"], "vigilode-pp16-pade-shifts-v1");
    data["approximants"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| Pade {
            k: a["k"].as_u64().unwrap(),
            c0: (unhex(&a["c0_re_hex"]), unhex(&a["c0_im_hex"])),
            poles: a["poles"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| {
                    (
                        (unhex(&p["gamma_re_hex"]), unhex(&p["gamma_im_hex"])),
                        (unhex(&p["weight_re_hex"]), unhex(&p["weight_im_hex"])),
                    )
                })
                .collect(),
        })
        .collect()
}

fn outcome<T>(id: &str, result: CoreResult<T>, status: impl Fn(&T) -> CertificateStatus) -> Value {
    match result {
        Ok(value) => {
            let s = status(&value);
            json!({"id": id, "rejected": s == CertificateStatus::Rejected, "outcome": format!("{s:?}")})
        }
        Err(error) => json!({"id": id, "rejected": true, "outcome": format!("error: {error}")}),
    }
}

/// (gamma_re, gamma_im, weight_re, weight_im, u_re, u_im)
type OwnedTerm = (f64, f64, f64, f64, Vec<f64>, Vec<f64>);

fn negative_controls(pade: &[Pade]) -> Vec<Value> {
    let mut controls = Vec::new();
    let j = DenseMatrix::from_rows(&[&[-1.0, 3.0], &[-3.0, -2.0]]).unwrap();
    let b = ([1.0, -0.5], [0.25, 0.75]);
    let h = 0.5;
    let gamma = (0.25, 1.5);
    let (u_re, u_im) = direct_candidate(&j, h, gamma, &b.0, &b.1).unwrap();
    let cert = |id: &str,
                j: &DenseMatrix,
                h: f64,
                g: (f64, f64),
                bb: ([f64; 2], [f64; 2]),
                u: (&[f64], &[f64])| {
        outcome(
            id,
            certify_complex_shift_candidate(j, h, g.0, g.1, &bb.0, &bb.1, u.0, u.1, TOLERANCE),
            |c| c.status(),
        )
    };
    let u = (u_re.as_slice(), u_im.as_slice());
    controls.push(cert("re_gamma_zero", &j, h, (0.0, 1.5), b, u));
    controls.push(cert("re_gamma_negative_zero", &j, h, (-0.0, 1.5), b, u));
    controls.push(cert("re_gamma_negative", &j, h, (-0.25, 1.5), b, u));
    controls.push(cert("re_gamma_negative_real", &j, h, (-0.25, 0.0), b, u));
    let nondissipative = DenseMatrix::from_rows(&[&[-1.0, 100.0], &[0.0, -1.0]]).unwrap();
    controls.push(cert(
        "nondissipative_j_stable_spectrum",
        &nondissipative,
        h,
        gamma,
        b,
        u,
    ));
    let positive = DenseMatrix::from_rows(&[&[0.5, 0.0], &[0.0, -1.0]]).unwrap();
    controls.push(cert(
        "nondissipative_j_positive_entry",
        &positive,
        h,
        gamma,
        b,
        u,
    ));
    // DenseMatrix constructors refuse NaN; the entry is set afterwards.
    let mut nan_j = j.clone();
    nan_j[(0, 1)] = f64::NAN;
    controls.push(cert("nonfinite_j", &nan_j, h, gamma, b, u));
    controls.push(cert("nonfinite_h", &j, f64::INFINITY, gamma, b, u));
    controls.push(cert("nonfinite_gamma_im", &j, h, (0.25, f64::NAN), b, u));
    controls.push(cert(
        "nonfinite_gamma_re",
        &j,
        h,
        (f64::INFINITY, 1.5),
        b,
        u,
    ));
    controls.push(cert(
        "nonfinite_b_im",
        &j,
        h,
        gamma,
        (b.0, [0.25, f64::INFINITY]),
        u,
    ));
    let nan_u = [u_re[0], f64::NAN];
    controls.push(cert("nonfinite_u_re", &j, h, gamma, b, (&nan_u, u.1)));
    controls.push(cert("negative_h", &j, -0.5, gamma, b, u));
    // Wrong candidates: the exact error exceeds the tolerance.
    let zero = [0.0, 0.0];
    controls.push(cert(
        "wrong_candidate_zero",
        &j,
        h,
        gamma,
        b,
        (&zero, &zero),
    ));
    let mut perturbed = u_re.clone();
    perturbed[1] += 1e-6;
    controls.push(cert(
        "wrong_candidate_perturbed",
        &j,
        h,
        gamma,
        b,
        (&perturbed, u.1),
    ));
    let (c_re, c_im) = direct_candidate(&j, h, (gamma.0, -gamma.1), &b.0, &b.1).unwrap();
    controls.push(cert(
        "wrong_candidate_conjugate_shift",
        &j,
        h,
        gamma,
        b,
        (&c_re, &c_im),
    ));
    controls.push(cert(
        "wrong_candidate_swapped_parts",
        &j,
        h,
        gamma,
        b,
        (u.1, u.0),
    ));

    let config = ComplexShiftJetConfig::default();
    let rhs_re = vec![b.0.to_vec()];
    let rhs_im = vec![b.1.to_vec()];
    let jet = |id: &str, j: &DenseMatrix, g0: f64, targets: &[(f64, f64)], config| {
        outcome(
            id,
            complex_shift_jet(j, h, g0, &rhs_re, &rhs_im, targets, config),
            |r| {
                if r.candidates()
                    .iter()
                    .all(|c| c.status() == CertificateStatus::Certified)
                {
                    CertificateStatus::Certified
                } else {
                    CertificateStatus::Rejected
                }
            },
        )
    };
    controls.push(jet(
        "jet_z_modulus_one_imaginary",
        &j,
        1.0,
        &[(1.0, 1.0)],
        config,
    ));
    controls.push(jet(
        "jet_z_modulus_one_real",
        &j,
        1.0,
        &[(2.0, 0.0)],
        config,
    ));
    controls.push(jet(
        "jet_z_modulus_above_one",
        &j,
        1.0,
        &[(0.5, 0.9)],
        config,
    ));
    controls.push(jet(
        "jet_z_modulus_one_mixed",
        &j,
        1.0,
        &[(0.4, 0.8)],
        config,
    ));
    controls.push(jet(
        "jet_z_one_among_valid",
        &j,
        1.0,
        &[(1.25, 0.25), (1.0, -1.0)],
        config,
    ));
    controls.push(jet("jet_re_gamma_zero", &j, 1.0, &[(0.0, 0.5)], config));
    controls.push(jet(
        "jet_nondissipative",
        &nondissipative,
        1.0,
        &[(1.0, 0.25)],
        config,
    ));
    controls.push(jet(
        "jet_nonfinite_target",
        &j,
        1.0,
        &[(1.0, f64::NAN)],
        config,
    ));
    controls.push(jet("jet_nonfinite_j", &nan_j, 1.0, &[(1.0, 0.25)], config));
    controls.push(jet("jet_center_zero", &j, 0.0, &[(1.0, 0.25)], config));
    controls.push(jet(
        "jet_degree_zero_under_resolved",
        &j,
        1.0,
        &[(1.5, 0.25)],
        ComplexShiftJetConfig {
            degree: 0,
            ..config
        },
    ));
    controls.push(jet(
        "jet_resource_cap",
        &j,
        1.0,
        &[(1.0, 0.25)],
        ComplexShiftJetConfig {
            max_stored_scalars: 0,
            ..config
        },
    ));

    // Partial fraction with the Pade [3/3] shifts on this J.
    let p3 = pade.iter().find(|p| p.k == 3).unwrap();
    let b_im0 = [0.0, 0.0];
    let solved: Vec<_> = p3
        .poles
        .iter()
        .map(|(g, _)| direct_candidate(&j, h, *g, &b.0, &b_im0).unwrap())
        .collect();
    let terms = |solved: &[(Vec<f64>, Vec<f64>)]| -> Vec<OwnedTerm> {
        p3.poles
            .iter()
            .zip(solved)
            .map(|((g, w), (ur, ui))| (g.0, g.1, w.0, w.1, ur.clone(), ui.clone()))
            .collect()
    };
    let run = |id: &str, j: &DenseMatrix, c0: (f64, f64), owned: &[OwnedTerm]| {
        let terms: Vec<PartialFractionTerm<'_>> = owned
            .iter()
            .map(|t| PartialFractionTerm {
                gamma_re: t.0,
                gamma_im: t.1,
                weight_re: t.2,
                weight_im: t.3,
                candidate_re: &t.4,
                candidate_im: &t.5,
            })
            .collect();
        outcome(
            id,
            certify_partial_fraction(j, h, c0.0, c0.1, &terms, &b.0, &b_im0, TOLERANCE),
            |c| c.status(),
        )
    };
    let good = terms(&solved);
    let mut wrong = good.clone();
    wrong[1].4[0] += 1e-6;
    controls.push(run("partial_fraction_wrong_candidate", &j, p3.c0, &wrong));
    let mut swapped = good.clone();
    let (first, second) = (swapped[1].4.clone(), swapped[1].5.clone());
    swapped[1].4 = swapped[2].4.clone();
    swapped[1].5 = swapped[2].5.clone();
    swapped[2].4 = first;
    swapped[2].5 = second;
    controls.push(run(
        "partial_fraction_conjugate_candidates_swapped",
        &j,
        p3.c0,
        &swapped,
    ));
    controls.push(run(
        "partial_fraction_nondissipative",
        &nondissipative,
        p3.c0,
        &good,
    ));
    let mut nan_weight = good.clone();
    nan_weight[0].2 = f64::NAN;
    controls.push(run(
        "partial_fraction_nonfinite_weight",
        &j,
        p3.c0,
        &nan_weight,
    ));
    controls.push(run(
        "partial_fraction_nonfinite_c0",
        &j,
        (f64::NAN, 0.0),
        &good,
    ));
    let mut negative = good.clone();
    negative[1].0 = -negative[1].0;
    controls.push(run(
        "partial_fraction_re_gamma_negative",
        &j,
        p3.c0,
        &negative,
    ));
    controls.push(run("partial_fraction_empty", &j, p3.c0, &[]));
    controls
}

#[test]
#[ignore = "research exporter; run with PP16_PADE and PP16_CASES set"]
fn export_pp16_cases() {
    // Without PP16_CASES (the workspace's ignored-test run) nothing is read or written.
    let Ok(path) = std::env::var("PP16_CASES") else {
        println!("PP16_CASES not set: cases not written");
        return;
    };
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let target = root.join(path);
    assert!(
        !target.exists(),
        "immutable output exists: {}",
        target.display()
    );
    let pade_path = std::env::var("PP16_PADE").expect("PP16_PADE must name the Pade JSON");
    let pade = load_pade(&root.join(&pade_path));

    let mut operators = Vec::new();
    let mut singles = Vec::new();
    let mut partial_fractions = Vec::new();
    let mut jets = Vec::new();
    for (index, n) in [4_usize, 6, 16, 24].into_iter().enumerate() {
        let mut rng = SplitMix(0x5050_3136_0000_0000 + index as u64);
        let j = dissipative(n, &mut rng);
        for h in [0.05, 0.5] {
            let op = format!("n{n}_h{h}");
            operators.push(json!({"id": op, "n": n, "h": hex(h), "h_decimal": h, "j_row_major": hexes(j.as_slice())}));

            // Pade [k/k] partial fractions of e^{hJ} b, b real.
            let b_re: Vec<f64> = (0..n).map(|_| rng.symmetric()).collect();
            let b_im = vec![0.0; n];
            for p in &pade {
                let family = format!("pade{}", p.k);
                let mut owned = Vec::new();
                let mut term_records = Vec::new();
                for (i, (g, w)) in p.poles.iter().enumerate() {
                    let (ur, ui) = direct_candidate(&j, h, *g, &b_re, &b_im).unwrap();
                    singles.push(single(
                        format!("{op}_{family}_pole{i}"),
                        &family,
                        &op,
                        &j,
                        h,
                        *g,
                        (&b_re, &b_im),
                        (&ur, &ui),
                    ));
                    owned.push((*g, *w, ur, ui));
                }
                let terms: Vec<PartialFractionTerm<'_>> = owned
                    .iter()
                    .map(|(g, w, ur, ui)| PartialFractionTerm {
                        gamma_re: g.0,
                        gamma_im: g.1,
                        weight_re: w.0,
                        weight_im: w.1,
                        candidate_re: ur,
                        candidate_im: ui,
                    })
                    .collect();
                let pf = certify_partial_fraction(
                    &j, h, p.c0.0, p.c0.1, &terms, &b_re, &b_im, TOLERANCE,
                )
                .unwrap();
                for ((g, w, ur, ui), c) in owned.iter().zip(pf.term_certificates()) {
                    term_records.push(json!({
                        "gamma_re": hex(g.0), "gamma_im": hex(g.1),
                        "weight_re": hex(w.0), "weight_im": hex(w.1),
                        "u_re": hexes(ur), "u_im": hexes(ui),
                        "certificate": certificate_json(c),
                    }));
                }
                println!(
                    "{op} {family}: {:?} bound {:e}",
                    pf.status(),
                    pf.error_upper()
                );
                partial_fractions.push(json!({
                    "id": format!("{op}_{family}"), "family": family, "operator": op,
                    "c0_re": hex(p.c0.0), "c0_im": hex(p.c0.1),
                    "b_re": hexes(&b_re), "b_im": hexes(&b_im),
                    "terms": term_records,
                    "output_re": hexes(pf.output_re()), "output_im": hexes(pf.output_im()),
                    "status": pf.status(),
                    "error_upper": hex(pf.error_upper()),
                    "error_upper_decimal": pf.error_upper(),
                    "output_radius_l2_upper": hex(pf.output_radius_l2_upper()),
                    "weighted_term_error_upper": hex(pf.weighted_term_error_upper()),
                    "absolute_tolerance": hex(pf.absolute_tolerance()),
                    "work": pf.work(),
                }));
            }

            // Random complex shifts: Re gamma log-uniform in [1e-3, 2], |Im gamma| <= 5.
            for i in 0..10 {
                let re = 1e-3 * (2000.0_f64).powf(rng.unit());
                let im = 5.0 * rng.symmetric();
                let b_re: Vec<f64> = (0..n).map(|_| rng.symmetric()).collect();
                let b_im: Vec<f64> = (0..n).map(|_| rng.symmetric()).collect();
                let (ur, ui) = direct_candidate(&j, h, (re, im), &b_re, &b_im).unwrap();
                singles.push(single(
                    format!("{op}_random{i}"),
                    "random",
                    &op,
                    &j,
                    h,
                    (re, im),
                    (&b_re, &b_im),
                    (&ur, &ui),
                ));
            }

            // Jet around gamma0 = 1: z = rho e^{i theta}, rho in {0.25, 0.6}.
            let rhs_re: Vec<Vec<f64>> = (0..2)
                .map(|_| (0..n).map(|_| rng.symmetric()).collect())
                .collect();
            let rhs_im: Vec<Vec<f64>> = (0..2)
                .map(|_| (0..n).map(|_| rng.symmetric()).collect())
                .collect();
            let mut targets = Vec::new();
            for rho in [0.25_f64, 0.6] {
                for k in 0..8 {
                    let theta = 0.3 + std::f64::consts::TAU * f64::from(k) / 8.0;
                    targets.push((1.0 + rho * theta.cos(), rho * theta.sin()));
                }
            }
            let config = ComplexShiftJetConfig {
                degree: 32,
                absolute_tolerance: TOLERANCE,
                ..ComplexShiftJetConfig::default()
            };
            let report = complex_shift_jet(&j, h, 1.0, &rhs_re, &rhs_im, &targets, config).unwrap();
            let certified = report
                .candidates()
                .iter()
                .filter(|c| c.status() == CertificateStatus::Certified)
                .count();
            println!("{op} jet: {certified}/{} targets certified", targets.len());
            jets.push(json!({
                "id": format!("{op}_jet"), "family": "jet", "operator": op,
                "gamma0": hex(1.0), "config": config,
                "rhs_re": rhs_re.iter().map(|c| hexes(c)).collect::<Vec<_>>(),
                "rhs_im": rhs_im.iter().map(|c| hexes(c)).collect::<Vec<_>>(),
                "candidates": report.candidates().iter().map(|c| json!({
                    "gamma_re": hex(c.gamma().0), "gamma_im": hex(c.gamma().1),
                    "z_modulus_upper": hex(c.z_modulus_upper()),
                    "columns_re": c.columns_re().iter().map(|v| hexes(v)).collect::<Vec<_>>(),
                    "columns_im": c.columns_im().iter().map(|v| hexes(v)).collect::<Vec<_>>(),
                    "certificates": c.certificates().iter().map(certificate_json).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
                "work": report.work(),
            }));
        }
    }
    let controls = negative_controls(&pade);
    for control in &controls {
        println!("{}: {}", control["id"], control["outcome"]);
    }
    std::fs::write(
        &target,
        serde_json::to_string_pretty(&json!({
            "schema": "vigilode-pp16-complex-shift-cases-v1",
            "pade_input": pade_path,
            "absolute_tolerance": hex(TOLERANCE),
            "operators": operators,
            "singles": singles,
            "partial_fractions": partial_fractions,
            "jets": jets,
            "negative_controls": controls,
        }))
        .unwrap()
            + "\n",
    )
    .unwrap();
    println!("wrote {}", target.display());
}
