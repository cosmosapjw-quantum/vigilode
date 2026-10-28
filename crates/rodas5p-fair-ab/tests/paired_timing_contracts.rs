//! Audit F-053 paired timing protocol on deterministic synthetic samples.

use rodas5p_fair_ab::{
    PAIRED_TIMING_MIN_INDEPENDENT_BLOCKS, PairedArm, PairedTimingCase, PairedTimingDecision,
    PairedTimingProtocol, TimingHostMetadata, abba_pair_order, assess_aa_control,
    assess_paired_timing, calibrate_batch_iterations, case_clustered_bootstrap,
    detect_timing_host_metadata, measure_paired_case, paired_timing_decision,
};

const SEED: u64 = 20_260_929;

/// Deterministic multiplicative noise in `[1 - amplitude, 1 + amplitude]`
/// (independent of the module's generator, so the tests do not share its
/// stream).
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

    fn index(&mut self, bound: usize) -> usize {
        ((0.5 * self.factor(1.0) * bound as f64) as usize).min(bound - 1)
    }
}

fn warmups() -> Vec<f64> {
    vec![5.0e-3, 1.0e-3]
}

fn protocol() -> PairedTimingProtocol {
    PairedTimingProtocol::authoritative(SEED)
}

/// Six independent processes of five consecutive pairs each.
fn six_processes(pairs: usize) -> Vec<u32> {
    (0..pairs).map(|pair| (pair * 6 / pairs) as u32).collect()
}

fn noisy_cases(ratio: f64, noise: f64, cases: usize, stream: u64) -> Vec<PairedTimingCase> {
    let protocol = protocol();
    let mut rng = Noise(stream);
    (0..cases)
        .map(|case| {
            let base = 1.0e-3 * (1.0 + case as f64);
            let candidate = (0..protocol.pairs)
                .map(|_| base * rng.factor(noise))
                .collect::<Vec<_>>();
            let reference = (0..protocol.pairs)
                .map(|_| base * ratio * rng.factor(noise))
                .collect::<Vec<_>>();
            PairedTimingCase::from_samples(
                format!("case-{case}"),
                &protocol,
                warmups(),
                candidate,
                reference,
            )
            .unwrap()
            .with_process_blocks(six_processes(protocol.pairs))
        })
        .collect()
}

/// Pair ratios spanning the v3.6 N=384 recorded range 0.547-4.43 with median
/// exactly `1.15`: fifteen log-uniform ratios below and fifteen above.
fn v36_spread_case(case_id: &str, order_seed: u64) -> PairedTimingCase {
    let protocol = protocol();
    let (low, centre, high) = (0.547_f64.ln(), 1.15_f64.ln(), 4.43_f64.ln());
    let mut logs = (0..15)
        .map(|k| low + (centre - low) * k as f64 / 14.0)
        .chain((0..15).map(|k| centre + (high - centre) * k as f64 / 14.0))
        .collect::<Vec<_>>();
    // Deterministic interleaving so the case is not trivially sorted.
    let mut rng = Noise(order_seed);
    for index in (1..logs.len()).rev() {
        logs.swap(index, rng.index(index + 1));
    }
    let candidate = vec![1.0e-3; logs.len()];
    let reference = logs.iter().map(|log| 1.0e-3 * log.exp()).collect();
    PairedTimingCase::from_samples(case_id, &protocol, warmups(), candidate, reference)
        .unwrap()
        .with_process_blocks(six_processes(logs.len()))
}

fn host() -> TimingHostMetadata {
    detect_timing_host_metadata(1)
}

#[test]
fn true_speedup_of_1_30_with_five_percent_noise_promotes() {
    let cases = noisy_cases(1.30, 0.05, 4, 1);
    let aa = noisy_cases(1.0, 0.05, 4, 2);
    let assessment = assess_paired_timing(&protocol(), &cases, Some(&aa), host()).unwrap();
    assert!(assessment.corpus.lower >= 1.15, "{:?}", assessment.corpus);
    assert!(assessment.corpus.contains(1.30));
    assert_eq!(assessment.decision, PairedTimingDecision::Promote);
    let control = assessment.aa_control.as_ref().unwrap();
    assert!(
        control.contains_unity && control.authoritative,
        "{control:?}"
    );
    assert!(assessment.timing_authoritative);
    assert_eq!(assessment.gate_decision, PairedTimingDecision::Promote);
    assert_eq!(assessment.corpus.resamples, 10_000);
}

#[test]
fn true_speedup_of_0_90_blocks() {
    let cases = noisy_cases(0.90, 0.05, 4, 3);
    let interval = case_clustered_bootstrap(&cases, &protocol()).unwrap();
    assert!(interval.upper < 1.15);
    assert_eq!(
        paired_timing_decision(&interval, 1.15),
        PairedTimingDecision::Block
    );
}

#[test]
fn threshold_speedup_with_the_v36_recorded_spread_is_inconclusive() {
    let cases = [v36_spread_case("n384-a", 5), v36_spread_case("n384-b", 6)];
    let assessment = assess_paired_timing(&protocol(), &cases, None, host()).unwrap();
    let summary = &assessment.cases[0];
    assert!((summary.minimum_pair_speedup - 0.547).abs() < 1.0e-12);
    assert!((summary.maximum_pair_speedup - 4.43).abs() < 1.0e-12);
    assert!((summary.median_speedup - 1.15).abs() < 1.0e-12);
    assert!(assessment.corpus.contains(1.15), "{:?}", assessment.corpus);
    assert_eq!(assessment.decision, PairedTimingDecision::Inconclusive);
    // No A/A control: recorded, never authoritative.
    assert!(!assessment.timing_authoritative);
    assert_eq!(assessment.gate_decision, PairedTimingDecision::Inconclusive);

    // The same spread in an A/A control is far wider than ln(1.15)/2, so
    // even a clear candidate win would be declared non-authoritative.
    let noisy_aa = [v36_spread_case("aa-a", 7), v36_spread_case("aa-b", 8)]
        .into_iter()
        .map(|mut case| {
            // Re-centre the spread on 1.0 for the A/A arms.
            for reference in &mut case.reference_seconds {
                *reference /= 1.15;
            }
            case
        })
        .collect::<Vec<_>>();
    let control = assess_aa_control(&noisy_aa, &protocol()).unwrap();
    assert!(control.contains_unity);
    assert!(control.half_width_log > control.maximum_half_width_log);
    assert!(!control.authoritative);
    let winning = noisy_cases(1.30, 0.05, 4, 9);
    let assessment = assess_paired_timing(&protocol(), &winning, Some(&noisy_aa), host()).unwrap();
    assert_eq!(assessment.decision, PairedTimingDecision::Promote);
    assert!(!assessment.timing_authoritative);
    assert_eq!(assessment.gate_decision, PairedTimingDecision::Inconclusive);
}

#[test]
fn identical_arms_give_an_aa_interval_containing_one() {
    let protocol = protocol();
    let samples = (0..protocol.pairs)
        .map(|k| 1.0e-3 * (1.0 + 0.01 * k as f64))
        .collect::<Vec<_>>();
    let identical =
        PairedTimingCase::from_samples("identical", &protocol, warmups(), samples.clone(), samples)
            .unwrap()
            .with_process_blocks(six_processes(protocol.pairs));
    let control = assess_aa_control(&[identical], &protocol).unwrap();
    assert!(control.contains_unity);
    assert_eq!(control.interval.lower, 1.0);
    assert_eq!(control.interval.upper, 1.0);
    assert!(control.authoritative);

    let noisy = assess_aa_control(&noisy_cases(1.0, 0.05, 4, 11), &protocol).unwrap();
    assert!(noisy.contains_unity, "{:?}", noisy.interval);
    assert!(noisy.authoritative);

    // A biased "A/A" session (one arm systematically 30% slower) excludes
    // 1.0 and invalidates the timing.
    let biased = assess_aa_control(&noisy_cases(1.30, 0.05, 4, 12), &protocol).unwrap();
    assert!(!biased.contains_unity);
    assert!(!biased.authoritative);
}

#[test]
fn same_seed_reproduces_identical_interval_bits() {
    let cases = noisy_cases(1.20, 0.10, 3, 13);
    let first = case_clustered_bootstrap(&cases, &protocol()).unwrap();
    let second = case_clustered_bootstrap(&cases, &protocol()).unwrap();
    for (left, right) in [
        (first.lower, second.lower),
        (first.upper, second.upper),
        (first.point, second.point),
        (first.lower_log, second.lower_log),
        (first.upper_log, second.upper_log),
    ] {
        assert_eq!(left.to_bits(), right.to_bits());
    }
    let reseeded = PairedTimingProtocol {
        seed: SEED + 1,
        ..protocol()
    };
    // With three cases in six shared sessions the two-way bootstrap has few
    // distinct replicate values, and a percentile can land on the same one
    // under another seed; twelve cases make the reseeding visible.
    let cases = noisy_cases(1.20, 0.10, 12, 13);
    let first = case_clustered_bootstrap(&cases, &protocol()).unwrap();
    // The seed also fixes the ABBA order, which raw-case admission checks
    // (re-audit R3, R3-STAT-01): the same samples under the other seed
    // carry that seed's order.
    let reordered = cases
        .iter()
        .cloned()
        .map(|mut case| {
            case.order = rodas5p_fair_ab::abba_pair_order(case.order.len(), SEED + 1);
            case
        })
        .collect::<Vec<_>>();
    let other = case_clustered_bootstrap(&reordered, &reseeded).unwrap();
    assert_eq!(other.point.to_bits(), first.point.to_bits());
    assert!(
        other.lower.to_bits() != first.lower.to_bits()
            || other.upper.to_bits() != first.upper.to_bits()
    );
}

#[test]
fn batch_size_is_calibrated_from_the_minimum_warmup() {
    let protocol = protocol();
    // Cold first sample 10 ms, fastest 1 ms, last 5 ms: the 2 ms floor
    // needs two iterations of the fastest sample, not one of the last.
    assert_eq!(
        calibrate_batch_iterations(&[1.0e-2, 1.0e-3, 5.0e-3], &protocol).unwrap(),
        2
    );
    assert_eq!(
        calibrate_batch_iterations(&[0.0, 0.0], &protocol).unwrap(),
        protocol.maximum_batch_iterations
    );
    assert!(calibrate_batch_iterations(&[1.0e-3], &protocol).is_err());
}

#[test]
fn abba_order_is_seeded_mirrored_and_balanced() {
    let order = abba_pair_order(30, SEED);
    assert_eq!(order, abba_pair_order(30, SEED));
    assert_eq!(order.len(), 30);
    for block in order.chunks_exact(2) {
        assert_eq!(block[0], [block[1][1], block[1][0]]);
    }
    let candidate_first = order
        .iter()
        .filter(|pair| pair[0] == PairedArm::Candidate)
        .count();
    assert_eq!(candidate_first, 15);
    let leads = |seed| {
        abba_pair_order(30, seed)
            .iter()
            .step_by(2)
            .map(|pair| pair[0])
            .collect::<Vec<_>>()
    };
    assert_ne!(leads(SEED), leads(SEED + 1));
    assert!(leads(SEED).contains(&PairedArm::Candidate));
    assert!(leads(SEED).contains(&PairedArm::Reference));
}

#[test]
fn measured_case_uses_warmups_minimum_calibration_and_abba_pairs() {
    // Synthetic clock: candidate costs 1.0 ms and reference 1.3 ms per call,
    // except the very first (cold) call, which costs 10 ms.
    let protocol = protocol();
    let now = std::cell::Cell::new(0.0_f64);
    let calls = std::cell::Cell::new(0_usize);
    let log = std::cell::RefCell::new(Vec::new());
    let tick = |cost: f64, arm: PairedArm| {
        let cold = calls.get() == 0;
        calls.set(calls.get() + 1);
        now.set(now.get() + if cold { 1.0e-2 } else { cost });
        log.borrow_mut().push(arm);
        Ok(())
    };
    let case = measure_paired_case(
        "synthetic",
        &protocol,
        || tick(1.0e-3, PairedArm::Candidate),
        || tick(1.3e-3, PairedArm::Reference),
        || now.get(),
    )
    .unwrap();
    assert_eq!(case.warmup_seconds.len(), 2 * protocol.warmups);
    assert_eq!(case.warmup_seconds[0], 1.0e-2);
    // Fastest warmup is 1 ms, so the 2 ms floor needs a batch of two.
    assert_eq!(case.batch_iterations, 2);
    assert_eq!(case.candidate_seconds.len(), protocol.pairs);
    assert_eq!(case.order, abba_pair_order(protocol.pairs, protocol.seed));
    let measured = &log.borrow()[2 * protocol.warmups..];
    let expected = case
        .order
        .iter()
        .flat_map(|pair| pair.iter().flat_map(|&arm| [arm, arm]))
        .collect::<Vec<_>>();
    assert_eq!(measured, expected.as_slice());
    for ratio in case
        .reference_seconds
        .iter()
        .zip(&case.candidate_seconds)
        .map(|(reference, candidate)| reference / candidate)
    {
        assert!((ratio - 1.3).abs() < 1.0e-9);
    }
}

#[test]
fn protocol_and_cases_below_the_minimum_are_rejected() {
    let mut weak = protocol();
    weak.warmups = 1;
    assert!(weak.validate().is_err());
    let mut weak = protocol();
    weak.pairs = 29;
    assert!(weak.validate().is_err());
    let protocol = protocol();
    let short = PairedTimingCase::from_samples(
        "short",
        &protocol,
        warmups(),
        vec![1.0e-3; 29],
        vec![1.0e-3; 29],
    )
    .unwrap();
    assert!(case_clustered_bootstrap(&[short], &protocol).is_err());
    let duplicate = noisy_cases(1.0, 0.05, 1, 14).pop().unwrap();
    assert!(case_clustered_bootstrap(&[duplicate.clone(), duplicate.clone()], &protocol).is_err());
    // Fewer than two warmups can neither calibrate nor validate a case.
    assert!(
        PairedTimingCase::from_samples(
            "cold",
            &protocol,
            vec![1.0e-3],
            vec![1.0e-3; 30],
            vec![1.0e-3; 30],
        )
        .is_err()
    );
    let mut stripped = duplicate;
    stripped.warmup_seconds.truncate(1);
    assert!(case_clustered_bootstrap(&[stripped], &protocol).is_err());
}

#[test]
fn host_metadata_records_every_field_or_unknown() {
    let host = detect_timing_host_metadata(3);
    assert_eq!(host.thread_count, 3);
    for field in [
        &host.cpu_model,
        &host.scaling_governor,
        &host.load_average,
        &host.cpus_allowed,
        &host.pinning,
    ] {
        assert!(!field.is_empty());
    }
    assert!(["single-cpu", "multi-cpu", "unknown"].contains(&host.pinning.as_str()));
}

#[test]
fn pairs_of_one_process_are_not_resampled_as_independent() {
    // External re-audit, 6.1: repeats inside one process share its state.
    // Declared as one process, thirty noisy pairs are one resampling unit:
    // the interval collapses to the case median and no decision is made.
    let protocol = protocol();
    let one_process = noisy_cases(1.30, 0.05, 1, 21)
        .into_iter()
        .map(|case| case.with_process_blocks(Vec::new()))
        .collect::<Vec<_>>();
    let interval = case_clustered_bootstrap(&one_process, &protocol).unwrap();
    // Unlabelled, it is resampled as one unit and counts as no session
    // (re-audit R2, R2-STAT-01).
    assert_eq!(interval.independent_blocks, 0);
    assert_eq!(interval.unlabeled_cases, 1);
    assert_eq!(interval.lower.to_bits(), interval.upper.to_bits());
    assert_eq!(
        paired_timing_decision(&interval, 1.15),
        PairedTimingDecision::Inconclusive
    );
    let control = assess_aa_control(&one_process, &protocol).unwrap();
    assert!(!control.authoritative);

    // The same samples from six processes give a real interval.
    let six = noisy_cases(1.30, 0.05, 1, 21);
    let interval = case_clustered_bootstrap(&six, &protocol).unwrap();
    assert_eq!(
        interval.independent_blocks,
        PAIRED_TIMING_MIN_INDEPENDENT_BLOCKS
    );
    assert!(interval.lower < interval.upper);

    // A block label per pair is required.
    let mismatched = noisy_cases(1.30, 0.05, 1, 22)
        .into_iter()
        .map(|case| case.with_process_blocks(vec![0; 3]))
        .collect::<Vec<_>>();
    assert!(case_clustered_bootstrap(&mismatched, &protocol).is_err());
}

#[test]
fn one_shared_process_is_one_session_however_many_cases() {
    // Audit 2026-09-30, B-01: six cases all measured in global process 77
    // were counted as six independent blocks and promoted.
    let protocol = protocol();
    for cases in [1, 6, 60] {
        let shared = noisy_cases(1.30, 0.05, cases, 31)
            .into_iter()
            .map(|case| {
                let pairs = case.candidate_seconds.len();
                case.with_process_blocks(vec![77; pairs])
            })
            .collect::<Vec<_>>();
        let interval = case_clustered_bootstrap(&shared, &protocol).unwrap();
        assert_eq!(interval.independent_blocks, 1, "{cases} cases");
        assert_eq!(
            paired_timing_decision(&interval, 1.15),
            PairedTimingDecision::Inconclusive
        );
        let aa = identical_arm_cases(cases, 77);
        let assessment = assess_paired_timing(&protocol, &shared, Some(&aa), host()).unwrap();
        assert!(!assessment.timing_authoritative, "{cases} cases");
        assert_ne!(assessment.gate_decision, PairedTimingDecision::Promote);
    }
}

#[test]
fn replicating_cases_does_not_narrow_a_session_level_interval() {
    // Six sessions with a common per-session effect on every case. The
    // interval is about the sessions; ten times more cases measured in the
    // same six sessions must not make it much narrower.
    let protocol = protocol();
    let session_factor = [0.85, 0.95, 1.0, 1.05, 1.2, 1.35];
    let build = |cases: usize| {
        let mut noise = Noise(41);
        (0..cases)
            .map(|case| {
                let base = 1.0e-3 * (1.0 + case as f64);
                let labels = six_processes(protocol.pairs);
                let candidate = (0..protocol.pairs)
                    .map(|_| base * noise.factor(0.01))
                    .collect::<Vec<_>>();
                let reference = labels
                    .iter()
                    .map(|&session| {
                        base * 1.3 * session_factor[session as usize] * noise.factor(0.01)
                    })
                    .collect::<Vec<_>>();
                PairedTimingCase::from_samples(
                    format!("case-{case}"),
                    &protocol,
                    warmups(),
                    candidate,
                    reference,
                )
                .unwrap()
                .with_process_blocks(labels)
            })
            .collect::<Vec<_>>()
    };
    let few = case_clustered_bootstrap(&build(6), &protocol).unwrap();
    let many = case_clustered_bootstrap(&build(60), &protocol).unwrap();
    assert_eq!(few.independent_blocks, 6);
    assert_eq!(many.independent_blocks, 6);
    assert!(
        many.half_width_log() >= 0.8 * few.half_width_log(),
        "6 cases {:.4}, 60 cases {:.4}",
        few.half_width_log(),
        many.half_width_log()
    );
}

/// Reference-against-reference cases, all pairs in one global process.
fn identical_arm_cases(cases: usize, process: u32) -> Vec<PairedTimingCase> {
    let protocol = protocol();
    let mut noise = Noise(51);
    (0..cases)
        .map(|case| {
            let base = 1.0e-3 * (1.0 + case as f64);
            let a = (0..protocol.pairs)
                .map(|_| base * noise.factor(0.01))
                .collect::<Vec<_>>();
            let b = (0..protocol.pairs)
                .map(|_| base * noise.factor(0.01))
                .collect::<Vec<_>>();
            PairedTimingCase::from_samples(format!("aa-{case}"), &protocol, warmups(), a, b)
                .unwrap()
                .with_process_blocks(vec![process; protocol.pairs])
        })
        .collect()
}
