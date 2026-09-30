//! Session identity and confirmatory authority of paired timing (re-audit R2
//! of 2026-09-30: R2-STAT-01, R2-STAT-03).

use rodas5p_fair_ab::{
    PAIRED_TIMING_SCHEMA, PairedTimingCase, PairedTimingDecision, PairedTimingProtocol,
    assess_paired_timing, case_clustered_bootstrap, detect_timing_host_metadata,
    measure_paired_case, measure_paired_case_in_session,
};

const SEED: u64 = 20_260_930;

type Tamper = dyn Fn(&mut rodas5p_fair_ab::PairedTimingAssessment);

struct Noise(u64);

impl Noise {
    fn factor(&mut self, amplitude: f64) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let unit = (self.0 >> 11) as f64 / (1_u64 << 53) as f64;
        1.0 + amplitude * (2.0 * unit - 1.0)
    }
}

/// Cases of `ratio` with 2% noise; pair `k` in session `labels(k)`.
fn cases(
    protocol: &PairedTimingProtocol,
    ratio: f64,
    prefix: &str,
    stream: u64,
    labels: impl Fn(usize) -> u32,
) -> Vec<PairedTimingCase> {
    let mut noise = Noise(stream);
    (0..6)
        .map(|case| {
            let base = 1.0e-3 * (1.0 + case as f64);
            let candidate = (0..protocol.pairs)
                .map(|_| base * noise.factor(0.02))
                .collect::<Vec<_>>();
            let reference = (0..protocol.pairs)
                .map(|_| base * ratio * noise.factor(0.02))
                .collect::<Vec<_>>();
            PairedTimingCase::from_samples(
                format!("{prefix}-{case}"),
                protocol,
                vec![5.0e-3, 1.0e-3],
                candidate,
                reference,
            )
            .unwrap()
            .with_process_blocks((0..protocol.pairs).map(&labels).collect())
        })
        .collect()
}

fn six(pairs: usize) -> impl Fn(usize) -> u32 {
    move |pair| (pair * 6 / pairs) as u32
}

/// A synthetic clock: every read advances by `step` seconds.
fn clock(step: f64) -> impl FnMut() -> f64 {
    let mut now = 0.0;
    move || {
        now += step;
        now
    }
}

#[test]
fn a_producer_without_a_session_is_never_authoritative() {
    // Six cases measured by one process with no declared session used to
    // count as six independent sessions and promote.
    let protocol = PairedTimingProtocol::authoritative(SEED);
    let unlabelled = (0..6)
        .map(|case| {
            measure_paired_case(
                format!("case-{case}"),
                &protocol,
                || Ok(()),
                || Ok(()),
                clock(1.0e-3),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let interval = case_clustered_bootstrap(&unlabelled, &protocol).unwrap();
    assert_eq!(interval.independent_blocks, 0);
    assert_eq!(interval.unlabeled_cases, 6);
    // The same process declared as session 77 is one session.
    let declared = (0..6)
        .map(|case| {
            measure_paired_case_in_session(
                format!("case-{case}"),
                77,
                &protocol,
                || Ok(()),
                || Ok(()),
                clock(1.0e-3),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let interval = case_clustered_bootstrap(&declared, &protocol).unwrap();
    assert_eq!(interval.independent_blocks, 1);
    assert_eq!(interval.sessions, vec![77]);
    assert_eq!(interval.unlabeled_cases, 0);

    // One unlabelled case among six labelled sessions blocks a decision.
    let mut mixed = cases(&protocol, 1.30, "case", 1, six(protocol.pairs));
    mixed[0].process_blocks.clear();
    let assessment = assess_paired_timing(
        &protocol,
        &mixed,
        Some(&cases(&protocol, 1.0, "aa", 2, six(protocol.pairs))),
        detect_timing_host_metadata(1),
    )
    .unwrap();
    assert_eq!(assessment.decision, PairedTimingDecision::Inconclusive);
    assert_eq!(assessment.gate_decision, PairedTimingDecision::Inconclusive);
}

#[test]
fn an_aa_control_in_other_sessions_confers_no_authority() {
    let protocol = PairedTimingProtocol::authoritative(SEED);
    let candidate = cases(&protocol, 1.30, "case", 3, six(protocol.pairs));
    let pairs = protocol.pairs;
    let elsewhere = cases(&protocol, 1.0, "aa", 4, move |pair| {
        100 + (pair * 6 / pairs) as u32
    });
    let assessment = assess_paired_timing(
        &protocol,
        &candidate,
        Some(&elsewhere),
        detect_timing_host_metadata(1),
    )
    .unwrap();
    assert_eq!(assessment.decision, PairedTimingDecision::Promote);
    assert!(assessment.aa_control.as_ref().unwrap().authoritative);
    assert!(!assessment.aa_sessions_match);
    assert!(!assessment.timing_authoritative);
    assert_eq!(assessment.gate_decision, PairedTimingDecision::Inconclusive);
    assert_eq!(
        assessment.verified_gate_decision().unwrap(),
        PairedTimingDecision::Inconclusive
    );
    // In the candidate's own sessions it does.
    let matched = cases(&protocol, 1.0, "aa", 4, six(protocol.pairs));
    let assessment = assess_paired_timing(
        &protocol,
        &candidate,
        Some(&matched),
        detect_timing_host_metadata(1),
    )
    .unwrap();
    assert!(assessment.aa_sessions_match && assessment.timing_authoritative);
    assert_eq!(
        assessment.verified_gate_decision().unwrap(),
        PairedTimingDecision::Promote
    );
}

#[test]
fn a_preview_protocol_or_an_inconsistent_record_never_gates() {
    // One bootstrap resample gave a zero-width interval that promoted.
    let preview = PairedTimingProtocol {
        bootstrap_resamples: 1,
        ..PairedTimingProtocol::authoritative(1)
    };
    assert!(!preview.is_confirmatory());
    let candidate = cases(&preview, 1.30, "case", 5, six(preview.pairs));
    let aa = cases(&preview, 1.0, "aa", 6, six(preview.pairs));
    let assessment = assess_paired_timing(
        &preview,
        &candidate,
        Some(&aa),
        detect_timing_host_metadata(1),
    )
    .unwrap();
    assert!(!assessment.confirmatory);
    assert_eq!(assessment.gate_decision, PairedTimingDecision::Inconclusive);
    assert!(assessment.verified_gate_decision().is_err());
    for weakened in [
        PairedTimingProtocol {
            confidence_level: 0.5,
            ..PairedTimingProtocol::authoritative(1)
        },
        PairedTimingProtocol {
            required_speedup: 1.01,
            ..PairedTimingProtocol::authoritative(1)
        },
    ] {
        assert!(!weakened.is_confirmatory());
    }

    let protocol = PairedTimingProtocol::authoritative(SEED);
    let candidate = cases(&protocol, 1.30, "case", 7, six(protocol.pairs));
    let aa = cases(&protocol, 1.0, "aa", 8, six(protocol.pairs));
    let genuine = assess_paired_timing(
        &protocol,
        &candidate,
        Some(&aa),
        detect_timing_host_metadata(1),
    )
    .unwrap();
    assert_eq!(genuine.schema, PAIRED_TIMING_SCHEMA);
    assert_eq!(
        genuine.verified_gate_decision().unwrap(),
        PairedTimingDecision::Promote
    );
    let tampered: Vec<Box<Tamper>> = vec![
        Box::new(|record| record.schema = "vigilode-paired-timing-v1".into()),
        Box::new(|record| record.protocol.required_speedup = 1.01),
        Box::new(|record| record.protocol.confidence_level = 0.5),
        Box::new(|record| record.corpus.resamples = 1),
        Box::new(|record| record.corpus.lower = record.corpus.upper * 2.0),
        Box::new(|record| record.decision = PairedTimingDecision::Block),
        Box::new(|record| record.aa_sessions_match = false),
        Box::new(|record| record.aa_control = None),
        Box::new(|record| record.corpus.independent_blocks = 60),
    ];
    for (index, tamper) in tampered.iter().enumerate() {
        let mut record = genuine.clone();
        tamper(&mut record);
        assert!(
            record.verified_gate_decision().is_err(),
            "tampering {index} was accepted"
        );
    }
    // Duplicate session labels do not count as distinct sessions.
    let mut duplicated = genuine.clone();
    duplicated.corpus.sessions = vec![5; duplicated.corpus.sessions.len()];
    assert!(duplicated.verified_gate_decision().is_err());

    // Interval values edited together with the decision are consistent, so
    // only the raw cases can expose them.
    // No speedup: the genuine record does not promote.
    let unpromoted = assess_paired_timing(
        &protocol,
        &cases(&protocol, 1.0, "case", 10, six(protocol.pairs)),
        Some(&aa),
        detect_timing_host_metadata(1),
    )
    .unwrap();
    assert_ne!(unpromoted.gate_decision, PairedTimingDecision::Promote);
    let mut forged = unpromoted.clone();
    forged.corpus.lower = 1.2;
    forged.corpus.upper = 1.3;
    forged.decision = PairedTimingDecision::Promote;
    forged.gate_decision = PairedTimingDecision::Promote;
    assert_eq!(
        forged.verified_gate_decision().unwrap(),
        PairedTimingDecision::Promote,
        "self-consistency alone cannot see this"
    );
    assert!(
        forged
            .verify_against_raw(
                &cases(&protocol, 1.0, "case", 10, six(protocol.pairs)),
                Some(&aa)
            )
            .is_err()
    );
    assert_eq!(
        genuine.verify_against_raw(&candidate, Some(&aa)).unwrap(),
        PairedTimingDecision::Promote
    );
    assert!(genuine.verify_against_raw(&aa, Some(&candidate)).is_err());
}

#[test]
fn the_interval_reports_its_simulation_band() {
    let protocol = PairedTimingProtocol::authoritative(SEED);
    let interval = case_clustered_bootstrap(
        &cases(&protocol, 1.2, "case", 9, six(protocol.pairs)),
        &protocol,
    )
    .unwrap();
    let [low, high] = interval.lower_log_simulation_band;
    assert!(low <= interval.lower_log && interval.lower_log <= high);
    let [low, high] = interval.upper_log_simulation_band;
    assert!(low <= interval.upper_log && interval.upper_log <= high);
}

#[test]
fn the_resampled_interval_matches_the_exact_six_session_bootstrap() {
    // Re-audit R2, R2-STAT-03 fixture: session ratios 0.8 .. 1.5, pairs
    // identical within a session, one case. Enumerating all 6^6 session
    // draws gives the exact percentile interval [0.848528137423857,
    // 1.449137674618944] (research/adversarial_reaudit_20260930_r2/
    // statistics/exact_bootstrap_oracle.json). The confirmatory B = 10000
    // resample must reproduce it within its reported simulation band.
    let protocol = PairedTimingProtocol::authoritative(1);
    let ratios = [0.8, 0.9, 1.0, 1.1, 1.4, 1.5];
    let labels = (0..protocol.pairs)
        .map(|pair| (pair * 6 / protocol.pairs) as u32)
        .collect::<Vec<_>>();
    let candidate = vec![1.0e-3; protocol.pairs];
    let reference = labels
        .iter()
        .map(|session| 1.0e-3 * ratios[*session as usize])
        .collect();
    let case = PairedTimingCase::from_samples(
        "fixture",
        &protocol,
        vec![5.0e-3, 1.0e-3],
        candidate,
        reference,
    )
    .unwrap()
    .with_process_blocks(labels);
    let interval = case_clustered_bootstrap(&[case], &protocol).unwrap();
    assert!((interval.point - 1.048_808_848_170_151_6).abs() <= 1.0e-12);
    for (exact, band, estimate) in [
        (
            0.848_528_137_423_857_f64,
            interval.lower_log_simulation_band,
            interval.lower,
        ),
        (
            1.449_137_674_618_944,
            interval.upper_log_simulation_band,
            interval.upper,
        ),
    ] {
        assert!(
            // 1e-12 absorbs the exp/ln round trip of the recorded endpoint.
            band[0] - 1.0e-12 <= exact.ln() && exact.ln() <= band[1] + 1.0e-12,
            "exact {exact} outside the band [{}, {}] (estimate {estimate})",
            band[0].exp(),
            band[1].exp()
        );
    }
}
