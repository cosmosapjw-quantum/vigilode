//! Homotopy certificate contracts of re-audit R4 of 2026-10-01
//! (R4-HOM-DEV-01..06).

use rodas5p_core::{CoreResult, WorkCounters, directed::Interval, rodas5p_coefficients};
use rodas5p_integrators::{
    AdaptiveStepConfig, CERTIFICATE_CAPABILITY_UNAVAILABLE, CERTIFICATE_STRUCTURE_UNSUPPORTED,
    InverseWitness, ModelBinding, OdeProblem, OutputSchedule, ParallelExecution, Q2Admission,
    Q2CertificateSource, QuadraticModel, QuadraticStageProblem, StageTarget,
    TransactionalQ1Q2Config, TransactionalQ1Q2Lane, UnverifiedWitness, WITNESS_NOT_VERIFIED,
    WitnessCapability, blocked_doubling_certificate_with_execution, certificate_binding,
    certify_stage_target, doubling_certificate, doubling_certificate_with_execution,
    doubling_levels, integrate_transactional_q1_q2_adaptive_observed_with_admission,
    scalar_linear_problem, transactional_q1_q2_step_with_admission,
};
use serde::Deserialize;

const FIXTURES: &str = include_str!("../../../fixtures/r3_homotopy_certificate_fixtures.json");

fn scalar_problem() -> QuadraticStageProblem {
    QuadraticStageProblem {
        jacobian: vec![vec![-1.0]],
        y: vec![1.0],
        h: 0.0625,
        q: vec![0.0],
    }
}

#[test]
fn tampered_wire_witnesses_are_rejected_and_genuine_ones_rebuilt() {
    // R4-HOM-01: upper = [[0]] or [[]] after construction used to certify a
    // first-stage error of 0 (exact 0.0617).
    let target = StageTarget::sequential(rodas5p_coefficients().unwrap()).unwrap();
    let problem = scalar_problem();
    let witness = InverseWitness::diagonal(&problem, target.gamma).unwrap();
    let wire = witness.to_unverified();
    let json = serde_json::to_string(&wire).unwrap();
    let decoded: UnverifiedWitness = serde_json::from_str(&json).unwrap();
    assert_eq!(
        decoded.clone().verify(&problem, target.gamma).unwrap(),
        witness
    );
    for mutate in [
        (|w: &mut UnverifiedWitness| w.upper[0][0] = 0.0) as fn(&mut UnverifiedWitness),
        |w| w.upper[0].clear(),
        |w| w.upper.clear(),
        |w| w.upper[0][0] = -w.upper[0][0],
        |w| w.upper[0][0] = f64::NAN,
        |w| w.identity.structure = "made-up".into(),
        |w| w.residual_norm_upper = 0.5,
    ] {
        let mut tampered = decoded.clone();
        mutate(&mut tampered);
        let error = tampered.verify(&problem, target.gamma).unwrap_err();
        assert!(
            error.to_string().contains(WITNESS_NOT_VERIFIED)
                || error.to_string().contains("INVERSE_WITNESS"),
            "{error}"
        );
    }
    // Another operator: the identity no longer matches.
    let other = QuadraticStageProblem {
        h: 0.125,
        ..scalar_problem()
    };
    assert!(decoded.verify(&other, target.gamma).is_err());
    // An approximate witness round-trips with its V.
    let dense = QuadraticStageProblem {
        jacobian: vec![vec![-1.0, 0.5], vec![0.25, -2.0]],
        y: vec![1.0, 1.0],
        h: 0.1,
        q: vec![0.0, 0.0],
    };
    // V: the binary64 inverse of W = I - h gamma J (an approximate inverse
    // with ||I - V W|| of roundoff size).
    let g = dense.h * target.gamma;
    let w = [
        [1.0 - g * dense.jacobian[0][0], -g * dense.jacobian[0][1]],
        [-g * dense.jacobian[1][0], 1.0 - g * dense.jacobian[1][1]],
    ];
    let det = w[0][0] * w[1][1] - w[0][1] * w[1][0];
    let v = vec![
        vec![w[1][1] / det, -w[0][1] / det],
        vec![-w[1][0] / det, w[0][0] / det],
    ];
    let approximate =
        InverseWitness::approximate(&dense, target.gamma, &v, "dense-approximate").unwrap();
    let back = approximate
        .to_unverified()
        .verify(&dense, target.gamma)
        .unwrap();
    assert_eq!(back, approximate);
    let mut tampered = approximate.to_unverified();
    tampered.upper[1][0] *= 0.5;
    assert!(tampered.verify(&dense, target.gamma).is_err());
    assert!(InverseWitness::approximate(&dense, target.gamma, &v, "diagonal").is_err());
}

/// The R4-HOM-02 exact chain: `gamma = 0`, unit first subdiagonal coupling,
/// `b` selecting the last stage, `J = 1`, `y = 1`, `h = 1`, zero candidate.
/// The exact stages are `K_i = 1 + K_(i-1)`, so the output error is `s`.
fn chain(stages: usize) -> StageTarget {
    let mut target = StageTarget {
        id: "r4-exact-chain",
        snapshot_sha256: "standalone-exact-dyadic-fixture",
        gamma: 0.0,
        c: vec![0.0; stages],
        gamma_rows: vec![0.0; stages],
        alpha_rows: (0..stages).map(|i| vec![0.0; i]).collect(),
        coupling_rows: (0..stages)
            .map(|i| {
                (0..i)
                    .map(|j| Interval::point(if j + 1 == i { 1.0 } else { 0.0 }).unwrap())
                    .collect()
            })
            .collect(),
        b: vec![0.0; stages],
        btilde: vec![0.0; stages],
    };
    if stages > 0 {
        target.b[stages - 1] = 1.0;
    }
    target
}

#[test]
fn doubling_depth_follows_the_stage_count() {
    assert_eq!(
        [1, 2, 3, 4, 8, 9, 16, 17].map(doubling_levels),
        [0, 1, 2, 2, 3, 4, 4, 5]
    );
    let problem = QuadraticStageProblem {
        jacobian: vec![vec![1.0]],
        y: vec![1.0],
        h: 1.0,
        q: vec![0.0],
    };
    for s in [1_usize, 2, 4, 8, 9, 16] {
        let target = chain(s);
        let witness = InverseWitness::diagonal(&problem, target.gamma).unwrap();
        let candidate = vec![vec![0.0]; s];
        let serial = certify_stage_target(
            &target,
            &problem,
            &candidate,
            &[1.0],
            &[0.0],
            &witness,
            1.0,
            0.0,
        )
        .unwrap();
        let doubling = doubling_certificate(
            &target,
            &problem,
            &candidate,
            &[1.0],
            &[0.0],
            &witness,
            1.0,
            0.0,
            0.0,
            1,
            1,
        )
        .unwrap();
        let certificate = doubling.certificate.unwrap();
        // The exact output error is s; both bounds are sound (they were 8
        // for s = 9 and 16).
        assert!(serial.output_bound[0] >= s as f64, "s = {s}");
        assert!(
            certificate.output_bound[0] >= s as f64,
            "s = {s}: {:?}",
            certificate.output_bound
        );
    }
    // Zero stages and negative radii are typed rejections.
    let witness = InverseWitness::diagonal(&problem, 0.0).unwrap();
    assert!(
        certify_stage_target(&chain(0), &problem, &[], &[1.0], &[0.0], &witness, 1.0, 0.0).is_err()
    );
    for radius in [-1.0, f64::NAN, f64::INFINITY] {
        assert!(
            doubling_certificate(
                &chain(4),
                &problem,
                &vec![vec![0.0]; 4],
                &[1.0],
                &[0.0],
                &witness,
                1.0,
                0.0,
                radius,
                1,
                1
            )
            .is_err()
        );
    }
}

#[derive(Deserialize)]
struct Fixtures {
    atol: f64,
    rtol: f64,
    doubling_radius: f64,
    rows: Vec<Row>,
}

#[derive(Deserialize)]
struct Row {
    family: String,
    jacobian: Vec<Vec<f64>>,
    y: Vec<f64>,
    h: f64,
    q: Vec<f64>,
    candidate: Vec<Vec<String>>,
    yhat: Vec<String>,
    ehat: Vec<String>,
    stage_distance: Vec<Vec<[String; 2]>>,
}

fn bits(hex: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(hex, 16).unwrap())
}

fn bound_bits(rows: &[Vec<f64>]) -> Vec<Vec<u64>> {
    rows.iter()
        .map(|row| row.iter().map(|x| x.to_bits()).collect())
        .collect()
}

#[test]
fn the_blocked_certificate_is_the_full_one_bit_for_bit_and_encloses_the_fixtures() {
    let fixtures: Fixtures = serde_json::from_str(FIXTURES).unwrap();
    let target = StageTarget::strict_lower_projection(rodas5p_coefficients().unwrap()).unwrap();
    let sequential = ParallelExecution::sequential();
    let parallel = ParallelExecution::rayon(3).unwrap();
    let mut diagonal_rows = 0;
    for row in &fixtures.rows {
        let problem = QuadraticStageProblem {
            jacobian: row.jacobian.clone(),
            y: row.y.clone(),
            h: row.h,
            q: row.q.clone(),
        };
        let candidate = row
            .candidate
            .iter()
            .map(|stage| stage.iter().map(|x| bits(x)).collect())
            .collect::<Vec<Vec<f64>>>();
        let yhat = row.yhat.iter().map(|x| bits(x)).collect::<Vec<_>>();
        let ehat = row.ehat.iter().map(|x| bits(x)).collect::<Vec<_>>();
        let Ok(witness) = InverseWitness::diagonal(&problem, target.gamma) else {
            // Non-diagonal families: the blocked path refuses them.
            let small = InverseWitness::small(&problem, target.gamma).unwrap();
            let error = blocked_doubling_certificate_with_execution(
                &target,
                &problem,
                &candidate,
                &yhat,
                &ehat,
                &small,
                fixtures.atol,
                fixtures.rtol,
                fixtures.doubling_radius,
                4,
                &sequential,
            )
            .unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains(CERTIFICATE_STRUCTURE_UNSUPPORTED),
                "{error}"
            );
            continue;
        };
        diagonal_rows += 1;
        let full = doubling_certificate_with_execution(
            &target,
            &problem,
            &candidate,
            &yhat,
            &ehat,
            &witness,
            fixtures.atol,
            fixtures.rtol,
            fixtures.doubling_radius,
            4,
            &sequential,
        )
        .unwrap();
        for execution in [&sequential, &parallel] {
            let blocked = blocked_doubling_certificate_with_execution(
                &target,
                &problem,
                &candidate,
                &yhat,
                &ehat,
                &witness,
                fixtures.atol,
                fixtures.rtol,
                fixtures.doubling_radius,
                4,
                execution,
            )
            .unwrap();
            assert_eq!(blocked.doubling.attempts, full.attempts, "{}", row.family);
            match (&blocked.doubling.certificate, &full.certificate) {
                (Some(b), Some(f)) => {
                    assert_eq!(bound_bits(&b.stage_bound), bound_bits(&f.stage_bound));
                    for (i, stage) in row.stage_distance.iter().enumerate() {
                        for (u, distance) in stage.iter().enumerate() {
                            assert!(b.stage_bound[i][u] >= bits(&distance[1]), "{}", row.family);
                        }
                    }
                }
                (None, None) => {}
                other => panic!("{}: {other:?}", row.family),
            }
        }
    }
    assert_eq!(diagonal_rows, 12);
}

#[test]
fn blocked_storage_and_work_scale_with_components() {
    let target = StageTarget::sequential(rodas5p_coefficients().unwrap()).unwrap();
    let n = 6;
    let problem = QuadraticStageProblem {
        jacobian: (0..n)
            .map(|a| {
                (0..n)
                    .map(|b| if a == b { -1.0 - a as f64 } else { 0.0 })
                    .collect()
            })
            .collect(),
        y: (0..n).map(|a| 1.0 + 0.1 * a as f64).collect(),
        h: 0.05,
        q: (0..n).map(|a| 0.01 * a as f64).collect(),
    };
    let witness = InverseWitness::diagonal(&problem, target.gamma).unwrap();
    let candidate = vec![vec![1.0e-3; n]; 8];
    let execution = ParallelExecution::sequential();
    let full = doubling_certificate_with_execution(
        &target, &problem, &candidate, &[1.0; 6], &[0.0; 6], &witness, 1.0e-6, 1.0e-6, 1.0e-3, 4,
        &execution,
    )
    .unwrap();
    let blocked = blocked_doubling_certificate_with_execution(
        &target, &problem, &candidate, &[1.0; 6], &[0.0; 6], &witness, 1.0e-6, 1.0e-6, 1.0e-3, 4,
        &execution,
    )
    .unwrap();
    assert_eq!(
        full.certificate
            .as_ref()
            .map(|c| bound_bits(&c.stage_bound)),
        blocked
            .doubling
            .certificate
            .as_ref()
            .map(|c| bound_bits(&c.stage_bound))
    );
    let work = blocked.work;
    let s = 8;
    assert_eq!((work.components, work.block_size, work.levels), (n, s, 3));
    // Per attempt: H, S and one product per level plus the squarings:
    // n (2 + 3 + 2) s^2 values, against (sn)^2 per full block matrix.
    let attempts = blocked.doubling.attempts.len() as u64;
    assert_eq!(work.allocated_values, attempts * (n * 7 * s * s) as u64);
    assert!(work.nonzeros <= (n * s * (s - 1) / 2) as u64);
    assert!(work.directed_operations > 0);
}

/// `y' = A y` with a coupled 6x6 `A`: no default witness exists.
struct Coupled(Vec<Vec<f64>>);

impl Q2CertificateSource for Coupled {
    fn stage_problem(&self, _t: f64, y: &[f64], h: f64) -> CoreResult<QuadraticStageProblem> {
        Ok(QuadraticStageProblem {
            jacobian: self.0.clone(),
            y: y.to_vec(),
            h,
            q: vec![0.0; y.len()],
        })
    }
}

fn coupled_matrix(n: usize) -> Vec<Vec<f64>> {
    (0..n)
        .map(|a| {
            (0..n)
                .map(|b| match (a as i64 - b as i64).abs() {
                    0 => -4.0,
                    1 => 1.0,
                    _ => 0.0,
                })
                .collect()
        })
        .collect()
}

#[test]
fn uncertifiable_structures_skip_the_q2_escalation() {
    let n = 6;
    let a = coupled_matrix(n);
    let model = QuadraticModel::new("coupled-linear-6", a.clone(), vec![0.0; n]).unwrap();
    let problem = model.ode_problem().unwrap();
    let source = Coupled(a);
    let y = vec![1.0; n];
    let gamma = rodas5p_coefficients().unwrap().gamma;
    let stage_problem = source.stage_problem(0.0, &y, 0.5).unwrap();
    assert_eq!(
        source.capability(&stage_problem, gamma),
        WitnessCapability::DimensionCutoff { dimension: n }
    );
    // A step large enough that q=1 is rejected: the certificate mode falls
    // back without the q=2 batches, the diagnostic mode escalates.
    let config = TransactionalQ1Q2Config {
        max_output_contraction: 1.0e-6,
        max_residual_contraction: 1.0e-6,
        ..TransactionalQ1Q2Config::default()
    };
    let run = |admission| {
        let mut counters = WorkCounters::default();
        transactional_q1_q2_step_with_admission(
            &problem,
            0.0,
            &y,
            0.5,
            &config,
            1.0e-10,
            1.0e-8,
            false,
            admission,
            &mut counters,
        )
        .unwrap()
    };
    let certified = run(Q2Admission::NativeTargetCertificate(&source));
    let diagnostic = run(Q2Admission::OperationalDiagnostic);
    assert_eq!(certified.lane, TransactionalQ1Q2Lane::SequentialFallback);
    assert!(!certified.escalated);
    assert!(
        certified
            .fallback_reason
            .as_deref()
            .unwrap()
            .starts_with(CERTIFICATE_CAPABILITY_UNAVAILABLE)
    );
    assert!(certified.q2_certificate.is_none());
    assert!(diagnostic.escalated);
    assert!(certified.work.w_solve_batches < diagnostic.work.w_solve_batches);
    // The skipped escalation leaves the protected sequential fallback.
    assert!(certified.step.used_fallback);
}

#[test]
fn one_model_generates_the_ode_and_the_certificate_and_stale_ones_are_rejected() {
    let model = QuadraticModel::new("scalar-quadratic", vec![vec![-20.0]], vec![-2.0]).unwrap();
    assert!(matches!(
        model.binding(),
        ModelBinding::GeneratedFromModel { .. }
    ));
    let problem = model.ode_problem().unwrap();
    let mut counters = WorkCounters::default();
    let report = transactional_q1_q2_step_with_admission(
        &problem,
        0.0,
        &[1.0],
        0.01,
        &TransactionalQ1Q2Config::default(),
        1.0e-10,
        1.0e-7,
        false,
        Q2Admission::NativeTargetCertificate(&model),
        &mut counters,
    )
    .unwrap();
    assert_eq!(report.lane, TransactionalQ1Q2Lane::Q2Escalated);
    let admission = report.q2_certificate.as_ref().unwrap();
    assert_eq!(admission.model_binding, model.binding());
    // The same model as the source of another integrated problem: the
    // structural binding is not claimed, only sampled agreement.
    let (foreign, _) = scalar_linear_problem(-20.0, 1.0);
    let linear_model = QuadraticModel::new("linear", vec![vec![-20.0]], vec![0.0]).unwrap();
    let report = transactional_q1_q2_step_with_admission(
        &foreign,
        0.0,
        &[1.0],
        0.01,
        &TransactionalQ1Q2Config::default(),
        1.0e-10,
        1.0e-7,
        false,
        Q2Admission::NativeTargetCertificate(&linear_model),
        &mut counters,
    )
    .unwrap();
    assert_eq!(
        report.q2_certificate.as_ref().unwrap().model_binding,
        ModelBinding::SampledAgreement
    );
    // Binding: every input of the certificate changes the digest.
    let target = StageTarget::sequential(rodas5p_coefficients().unwrap()).unwrap();
    let stage_problem = model.stage_problem(0.0, &[1.0], 0.01).unwrap();
    let witness = InverseWitness::diagonal(&stage_problem, target.gamma).unwrap();
    let candidate = vec![vec![-0.2]; 8];
    let certificate = certify_stage_target(
        &target,
        &stage_problem,
        &candidate,
        &[0.8],
        &[0.0],
        &witness,
        1.0e-10,
        1.0e-7,
    )
    .unwrap();
    let id = witness.identity();
    let (y_hat, e_hat) = ([0.8], [0.0]);
    let bound = |target: &StageTarget,
                 problem: &QuadraticStageProblem,
                 candidate: &[Vec<f64>],
                 y_hat: &[f64],
                 e_hat: &[f64],
                 identity: &rodas5p_integrators::WitnessIdentity,
                 atol: f64,
                 rtol: f64| {
        certificate.is_bound_to(
            target, problem, candidate, y_hat, e_hat, identity, atol, rtol,
        )
    };
    assert!(bound(
        &target,
        &stage_problem,
        &candidate,
        &y_hat,
        &e_hat,
        id,
        1.0e-10,
        1.0e-7
    ));
    assert_eq!(
        certificate.binding_sha256,
        certificate_binding(
            &target,
            &stage_problem,
            &candidate,
            &y_hat,
            &e_hat,
            id,
            1.0e-10,
            1.0e-7
        )
    );
    let mut stale_target = target.clone();
    stale_target.b[0] = stale_target.b[0].next_up();
    let mut stale_coupling = target.clone();
    stale_coupling.coupling_rows[3][1] =
        Interval::point(stale_coupling.coupling_rows[3][1].hi.next_up()).unwrap();
    let mut stale_candidate = candidate.clone();
    stale_candidate[7][0] = stale_candidate[7][0].next_up();
    let mut stale_identity = id.clone();
    stale_identity.tolerance_bits ^= 1;
    let stale_problems = [
        QuadraticStageProblem {
            h: 0.02,
            ..stage_problem.clone()
        },
        QuadraticStageProblem {
            y: vec![1.0_f64.next_up()],
            ..stage_problem.clone()
        },
        QuadraticStageProblem {
            q: vec![-2.5],
            ..stage_problem.clone()
        },
        QuadraticStageProblem {
            jacobian: vec![vec![-21.0]],
            ..stage_problem.clone()
        },
    ];
    let p = &stage_problem;
    assert!(!bound(
        &stale_target,
        p,
        &candidate,
        &y_hat,
        &e_hat,
        id,
        1.0e-10,
        1.0e-7
    ));
    assert!(!bound(
        &stale_coupling,
        p,
        &candidate,
        &y_hat,
        &e_hat,
        id,
        1.0e-10,
        1.0e-7
    ));
    assert!(!bound(
        &target,
        p,
        &stale_candidate,
        &y_hat,
        &e_hat,
        id,
        1.0e-10,
        1.0e-7
    ));
    assert!(!bound(
        &target,
        p,
        &candidate,
        &[0.8_f64.next_up()],
        &e_hat,
        id,
        1.0e-10,
        1.0e-7
    ));
    assert!(!bound(
        &target,
        p,
        &candidate,
        &y_hat,
        &[1.0e-300],
        id,
        1.0e-10,
        1.0e-7
    ));
    assert!(!bound(
        &target,
        p,
        &candidate,
        &y_hat,
        &e_hat,
        &stale_identity,
        1.0e-10,
        1.0e-7
    ));
    assert!(!bound(
        &target, p, &candidate, &y_hat, &e_hat, id, 2.0e-10, 1.0e-7
    ));
    assert!(!bound(
        &target, p, &candidate, &y_hat, &e_hat, id, 1.0e-10, 2.0e-7
    ));
    for stale in &stale_problems {
        assert!(!bound(
            &target, stale, &candidate, &y_hat, &e_hat, id, 1.0e-10, 1.0e-7
        ));
    }
    // The other model differs in digest.
    let other = QuadraticModel::new("scalar-quadratic", vec![vec![-20.0]], vec![-2.5]).unwrap();
    assert_ne!(model.model_sha256(), other.model_sha256());
}

#[test]
fn an_integration_builds_at_most_one_pool_and_threads_change_no_bit() {
    let model = QuadraticModel::new("scalar-quadratic", vec![vec![-20.0]], vec![-2.0]).unwrap();
    let problem: OdeProblem = model.ode_problem().unwrap();
    let adaptive = AdaptiveStepConfig {
        atol: 1.0e-8,
        rtol: 1.0e-6,
        initial_step: 1.0e-3,
        max_attempts: 200,
        ..Default::default()
    };
    let output = OutputSchedule::new(vec![0.0, 0.05, 0.1]).unwrap();
    let run = |threads: usize| {
        integrate_transactional_q1_q2_adaptive_observed_with_admission(
            &problem,
            (0.0, 0.1),
            &[1.0],
            &TransactionalQ1Q2Config {
                threads,
                ..TransactionalQ1Q2Config::default()
            },
            &adaptive,
            &output,
            Q2Admission::NativeTargetCertificate(&model),
        )
        .unwrap()
    };
    let one = run(1);
    let three = run(3);
    assert_eq!(one.transactional.pool_creations, 0);
    assert_eq!(three.transactional.pool_creations, 1);
    assert!(three.diagnostics.attempts > 1);
    assert_eq!(one.observed.y, three.observed.y);
    // The doubling certificate builds one pool per call, none when the
    // caller owns the execution.
    let target = chain(8);
    let stage_problem = QuadraticStageProblem {
        jacobian: vec![vec![1.0]],
        y: vec![1.0],
        h: 1.0,
        q: vec![0.0],
    };
    let witness = InverseWitness::diagonal(&stage_problem, target.gamma).unwrap();
    let candidate = vec![vec![0.0]; 8];
    let owned = doubling_certificate(
        &target,
        &stage_problem,
        &candidate,
        &[1.0],
        &[0.0],
        &witness,
        1.0,
        0.0,
        0.0,
        3,
        4,
    )
    .unwrap();
    assert_eq!(owned.pool_creations, 1);
    let pool = ParallelExecution::rayon(4).unwrap();
    let borrowed = doubling_certificate_with_execution(
        &target,
        &stage_problem,
        &candidate,
        &[1.0],
        &[0.0],
        &witness,
        1.0,
        0.0,
        0.0,
        3,
        &pool,
    )
    .unwrap();
    assert_eq!(borrowed.pool_creations, 0);
    assert_eq!(
        owned.certificate.map(|c| bound_bits(&c.stage_bound)),
        borrowed.certificate.map(|c| bound_bits(&c.stage_bound))
    );
}
