//! The q=2 eighth-batch diagnostic replaced by a native-target certificate
//! admission (re-audit R3 of 2026-10-01, HOM-05).

use std::sync::Arc;

use rodas5p_core::{CoreError, CoreResult, WorkCounters};
use rodas5p_integrators::{
    CERTIFICATE_CAPABILITY_UNAVAILABLE, InverseWitness, OdeProblem, Q2Admission,
    Q2CertificateSource, QuadraticStageProblem, TransactionalQ1Q2Config, TransactionalQ1Q2Lane,
    TransactionalQ1Q2StepReport, WitnessCapability, scalar_linear_problem,
    transactional_q1_q2_step, transactional_q1_q2_step_with_admission,
};

/// `y' = a y + c y^2`: `J = a + 2 c y`, `q = c`.
struct Quadratic {
    a: f64,
    c: f64,
}

impl Q2CertificateSource for Quadratic {
    fn stage_problem(&self, _t: f64, y: &[f64], h: f64) -> CoreResult<QuadraticStageProblem> {
        Ok(QuadraticStageProblem {
            jacobian: vec![vec![self.a + 2.0 * self.c * y[0]]],
            y: y.to_vec(),
            h,
            q: vec![self.c],
        })
    }
}

fn quadratic_problem(a: f64, c: f64) -> OdeProblem {
    OdeProblem::new(
        format!("quadratic-{a}-{c}"),
        1,
        Arc::new(move |_t, y: &[f64], out: &mut [f64]| {
            out[0] = a * y[0] + c * y[0] * y[0];
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(
            move |_t, y: &[f64], v: &[f64], out: &mut [f64]| {
                out[0] = (a + 2.0 * c * y[0]) * v[0];
                Ok(())
            },
        )),
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

fn step(
    problem: &OdeProblem,
    h: f64,
    config: &TransactionalQ1Q2Config,
    admission: Q2Admission<'_>,
) -> TransactionalQ1Q2StepReport {
    let mut counters = WorkCounters::default();
    transactional_q1_q2_step_with_admission(
        problem,
        0.0,
        &[1.0],
        h,
        config,
        1.0e-10,
        1.0e-7,
        false,
        admission,
        &mut counters,
    )
    .unwrap()
}

fn bits(values: &[f64]) -> Vec<u64> {
    values.iter().map(|value| value.to_bits()).collect()
}

#[test]
fn the_certificate_replaces_only_the_eighth_batch() {
    let config = TransactionalQ1Q2Config::default();
    let (linear, _) = scalar_linear_problem(-20.0, 1.0);
    let quadratic = quadratic_problem(-20.0, -2.0);
    for (problem, source) in [
        (&linear, Quadratic { a: -20.0, c: 0.0 }),
        (&quadratic, Quadratic { a: -20.0, c: -2.0 }),
    ] {
        let diagnostic = step(problem, 0.01, &config, Q2Admission::OperationalDiagnostic);
        let certified = step(
            problem,
            0.01,
            &config,
            Q2Admission::NativeTargetCertificate(&source),
        );
        let label = &problem.name;
        assert_eq!(
            diagnostic.lane,
            TransactionalQ1Q2Lane::Q2Escalated,
            "{label}"
        );
        assert_eq!(
            certified.lane,
            TransactionalQ1Q2Lane::Q2Escalated,
            "{label}"
        );
        assert_eq!(diagnostic.q2_admission, "operational-diagnostic");
        assert_eq!(certified.q2_admission, "native-target-certificate");
        // The q=1 and q=2 candidates are the same bits in both modes.
        assert_eq!(
            bits(&diagnostic.q1_candidate_y),
            bits(&certified.q1_candidate_y)
        );
        assert_eq!(
            bits(diagnostic.q2_candidate_y.as_ref().unwrap()),
            bits(certified.q2_candidate_y.as_ref().unwrap()),
            "{label}"
        );
        // Both admit it here, so the accepted step is bit-identical too.
        let admission = certified.q2_certificate.as_ref().unwrap();
        assert!(admission.accepted, "{label}: {}", admission.reason);
        let certificate = admission.certificate.as_ref().unwrap();
        assert!(certificate.output_wrms_upper <= admission.budget_lower);
        assert!(diagnostic.fast_accepted && certified.fast_accepted);
        assert_eq!(bits(&diagnostic.step.y_new), bits(&certified.step.y_new));
        for (a, b) in diagnostic.step.stages.iter().zip(&certified.step.stages) {
            assert_eq!(bits(a), bits(b));
        }
        // Seven W batches instead of eight; the certificate is its own line.
        assert_eq!(diagnostic.work.w_solve_batches, 8);
        assert_eq!(certified.work.w_solve_batches, 7);
        assert_eq!(certified.critical_path_depth, 7);
        assert_eq!(diagnostic.work.certificate_attempts, 0);
        assert_eq!(certified.work.certificate_attempts, 1);
        assert_eq!(
            certified.work.certificate_operations,
            certificate.directed_operations
        );
        assert!(certified.work.certificate_operations > 0);
    }
}

#[test]
fn the_default_entry_point_keeps_the_operational_diagnostic() {
    let (problem, y0) = scalar_linear_problem(-20.0, 1.0);
    let mut counters = WorkCounters::default();
    let report = transactional_q1_q2_step(
        &problem,
        0.0,
        &y0,
        0.01,
        &TransactionalQ1Q2Config::default(),
        1.0e-10,
        1.0e-7,
        false,
        &mut counters,
    )
    .unwrap();
    assert_eq!(report.q2_admission, "operational-diagnostic");
    assert!(report.q2_certificate.is_none());
    assert_eq!(report.work.w_solve_batches, 8);
}

struct Shifted;
impl Q2CertificateSource for Shifted {
    fn stage_problem(&self, _t: f64, y: &[f64], h: f64) -> CoreResult<QuadraticStageProblem> {
        Ok(QuadraticStageProblem {
            jacobian: vec![vec![-20.0]],
            y: vec![y[0] + 1.0e-3],
            h,
            q: vec![0.0],
        })
    }
}

struct Failing;
impl Q2CertificateSource for Failing {
    fn stage_problem(&self, _t: f64, _y: &[f64], _h: f64) -> CoreResult<QuadraticStageProblem> {
        Err(CoreError::InvalidInput("no certificate structure".into()))
    }
}

/// A witness built for another step size.
struct ForeignWitness;
impl Q2CertificateSource for ForeignWitness {
    fn stage_problem(&self, _t: f64, y: &[f64], h: f64) -> CoreResult<QuadraticStageProblem> {
        Quadratic { a: -20.0, c: 0.0 }.stage_problem(0.0, y, h)
    }

    fn witness(&self, problem: &QuadraticStageProblem, gamma: f64) -> CoreResult<InverseWitness> {
        let other = QuadraticStageProblem {
            h: problem.h * 2.0,
            ..problem.clone()
        };
        InverseWitness::diagonal(&other, gamma)
    }
}

#[test]
fn a_wrong_or_unverified_certificate_cannot_fast_accept() {
    let (linear, _) = scalar_linear_problem(-20.0, 1.0);
    let wrong_operator = Quadratic { a: -10.0, c: 0.0 };
    let right = Quadratic { a: -20.0, c: 0.0 };
    let tight = TransactionalQ1Q2Config {
        absolute_output_budget: 1.0e-300,
        ..TransactionalQ1Q2Config::default()
    };
    let default = TransactionalQ1Q2Config::default();
    let cases: [(&str, &dyn Q2CertificateSource, &TransactionalQ1Q2Config); 4] = [
        ("another state", &Shifted, &default),
        ("another operator", &wrong_operator, &default),
        ("foreign witness", &ForeignWitness, &default),
        ("budget", &right, &tight),
    ];
    for (label, source, config) in cases {
        let report = step(
            &linear,
            0.01,
            config,
            Q2Admission::NativeTargetCertificate(source),
        );
        let admission = report.q2_certificate.as_ref().unwrap();
        assert!(!admission.accepted, "{label}");
        assert!(
            admission
                .reason
                .starts_with("ADMISSION_EQUIVALENCE_NOT_ESTABLISHED"),
            "{label}: {}",
            admission.reason
        );
        assert_eq!(
            report.lane,
            TransactionalQ1Q2Lane::SequentialFallback,
            "{label}"
        );
        assert!(!report.fast_accepted, "{label}");
        assert!(report.step.used_fallback, "{label}");
        // The fallback transaction is unchanged; no eighth batch was spent.
        assert_eq!(report.work.w_solve_batches, 7, "{label}");
        assert_eq!(report.work.certificate_attempts, 1, "{label}");
        assert_eq!(report.critical_path_depth, 7 + 8, "{label}");
    }
    // A source with no certificate problem is known before the escalation
    // (re-audit R4, R4-HOM-DEV-06): the q=2 batches are not spent on a
    // candidate no certificate can admit, and the fallback is the same.
    let report = step(
        &linear,
        0.01,
        &default,
        Q2Admission::NativeTargetCertificate(&Failing),
    );
    assert!(matches!(
        report.q2_capability,
        Some(WitnessCapability::ProblemUnavailable { .. })
    ));
    assert!(report.q2_certificate.is_none());
    assert!(!report.escalated);
    assert_eq!(report.lane, TransactionalQ1Q2Lane::SequentialFallback);
    assert!(
        report
            .fallback_reason
            .as_deref()
            .unwrap()
            .starts_with(CERTIFICATE_CAPABILITY_UNAVAILABLE)
    );
    assert!(report.work.w_solve_batches < 7);
    assert_eq!(report.work.certificate_attempts, 0);
}

/// `y' = a y + q y^2 + c y^3` declared as the quadratic model with the true
/// `J = a + 2 q y + 3 c y^2` and `q' = q + 2 c y`: it reproduces `f(y)` and
/// every JVP at `y` exactly, so only the stage states expose the cubic term
/// (re-audit R3 review).
struct CubicAsQuadratic {
    a: f64,
    q: f64,
    c: f64,
}

impl Q2CertificateSource for CubicAsQuadratic {
    fn stage_problem(&self, _t: f64, y: &[f64], h: f64) -> CoreResult<QuadraticStageProblem> {
        let y0 = y[0];
        Ok(QuadraticStageProblem {
            jacobian: vec![vec![self.a + 2.0 * self.q * y0 + 3.0 * self.c * y0 * y0]],
            y: y.to_vec(),
            h,
            q: vec![self.q + 2.0 * self.c * y0],
        })
    }
}

#[test]
fn a_model_that_matches_only_at_y_is_rejected() {
    let (a, q, c) = (-20.0, -2.0, -5.0);
    let cubic = OdeProblem::new(
        "cubic",
        1,
        Arc::new(move |_t, y: &[f64], out: &mut [f64]| {
            out[0] = a * y[0] + q * y[0] * y[0] + c * y[0] * y[0] * y[0];
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(
            move |_t, y: &[f64], v: &[f64], out: &mut [f64]| {
                out[0] = (a + 2.0 * q * y[0] + 3.0 * c * y[0] * y[0]) * v[0];
                Ok(())
            },
        )),
        None,
        true,
        None,
        None,
    )
    .unwrap();
    let source = CubicAsQuadratic { a, q, c };
    let report = step(
        &cubic,
        0.01,
        &TransactionalQ1Q2Config::default(),
        Q2Admission::NativeTargetCertificate(&source),
    );
    assert_eq!(report.lane, TransactionalQ1Q2Lane::SequentialFallback);
    let admission = report.q2_certificate.as_ref().unwrap();
    assert!(!admission.accepted);
    assert!(
        admission.reason.contains("at stage"),
        "{}",
        admission.reason
    );
    assert!(!report.fast_accepted);
}
