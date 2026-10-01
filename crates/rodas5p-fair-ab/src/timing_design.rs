//! Design contract and coverage study of the paired timing statistic
//! (re-audit R3 of 2026-10-01, STAT-DEV-04).
//!
//! The estimand is fixed by [`TIMING_DESIGN_CONTRACT`]: the median over the
//! declared case corpus (equal case weights) of each case's population
//! median log speedup over independent sessions. Cases are the declared
//! corpus, not a sample from a larger population; sessions are the random,
//! independent unit. Each session measures exactly `protocol.pairs` pairs
//! of every case (a balanced cell); a missing cell stays missing, a case with
//! no admissible cell is excluded, and failed sessions stay in the receipt.
//!
//! [`simulate_corpus`] draws raw samples from the declared dependence model
//! `Y = theta + v_c + u_s + w_sc + e_scp` (`v_c` median-centred so the corpus
//! estimand is exactly `theta`; `e` AR(1) within a cell) and a missingness
//! model, and runs them through the production admission and
//! [`case_clustered_bootstrap`]. [`coverage_study`] measures population
//! coverage of the interval and the promotion rate at and below the
//! threshold, separately from the Monte-Carlo resolution the decision gate
//! checks.

use serde::{Deserialize, Serialize};

use crate::{
    FairError, FairResult, PAIRED_TIMING_MC_FAILURE_BUDGET, PairedTimingCase, PairedTimingDecision,
    PairedTimingProtocol, case_clustered_bootstrap,
    paired_timing::{SplitMix64, median_in_place},
    paired_timing_decision, percentile_endpoint_decision,
};

pub const TIMING_DESIGN_CONTRACT: &str =
    "vigilode-timing-design-v1/corpus-median-of-case-session-medians/balanced-cells";
pub const COVERAGE_STUDY_SCHEMA: &str = "vigilode-timing-coverage-study-v1";
/// `0.95 - 2.576 sqrt(0.95 * 0.05 / 2000)`: the nominal coverage less the
/// 99% binomial allowance of the preregistered 2000 replications.
/// (0.937446..., rounded up: the R3 package's 0.9374 rounded leniently).
pub const COVERAGE_STUDY_MIN_COVERAGE: f64 = 0.93745;
/// `0.025 + 2.576 sqrt(0.025 * 0.975 / 2000)` (0.033993..., rounded down).
pub const COVERAGE_STUDY_MAX_FALSE_PROMOTE: f64 = 0.03399;
pub const COVERAGE_STUDY_REPLICATIONS: usize = 2000;

/// Standard deviations (log scale) of the random effects.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DependenceModel {
    pub session_sd: f64,
    pub case_sd: f64,
    pub interaction_sd: f64,
    pub pair_sd: f64,
    /// AR(1) coefficient of the pair noise within a session-case cell.
    pub pair_autocorrelation: f64,
}

impl DependenceModel {
    /// The preregistered model.
    pub const PREREGISTERED: Self = Self {
        session_sd: 0.05,
        case_sd: 0.10,
        interaction_sd: 0.03,
        pair_sd: 0.05,
        pair_autocorrelation: 0.5,
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum Missingness {
    None,
    /// Each cell is missing independently with this probability.
    Mcar {
        probability: f64,
    },
    /// A cell whose systematic effect `u_s + w_sc` is negative (unfavourable
    /// to the candidate) is missing with this probability: missingness that
    /// flatters the candidate.
    Mnar {
        probability: f64,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TimingDesign {
    pub sessions: usize,
    pub cases: usize,
    pub pairs_per_cell: usize,
    pub theta_log: f64,
    pub dependence: DependenceModel,
    pub missingness: Missingness,
}

impl TimingDesign {
    fn validate(&self, protocol: &PairedTimingProtocol) -> FairResult<()> {
        let dependence = &self.dependence;
        let probability = match self.missingness {
            Missingness::None => 0.0,
            Missingness::Mcar { probability } | Missingness::Mnar { probability } => probability,
        };
        if self.sessions == 0
            || self.cases == 0
            || self.pairs_per_cell < protocol.pairs
            || !self.theta_log.is_finite()
            || ![
                dependence.session_sd,
                dependence.case_sd,
                dependence.interaction_sd,
                dependence.pair_sd,
            ]
            .iter()
            .all(|sd| sd.is_finite() && *sd >= 0.0)
            || !(0.0..1.0).contains(&dependence.pair_autocorrelation)
            || !(0.0..1.0).contains(&probability)
        {
            return Err(FairError::Invalid(
                "timing design needs sessions, cases, full cells and finite effects".into(),
            ));
        }
        Ok(())
    }
}

/// A simulated raw corpus.
#[derive(Clone, Debug)]
pub struct SimulatedCorpus {
    pub cases: Vec<PairedTimingCase>,
    /// The corpus estimand: exactly `theta_log`.
    pub target_log: f64,
    pub missing_cells: usize,
    pub excluded_cases: usize,
}

impl SplitMix64 {
    fn uniform(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64 + 0.5) * (1.0 / (1u64 << 53) as f64)
    }

    fn normal(&mut self, sd: f64) -> f64 {
        let radius = (-2.0 * self.uniform().ln()).sqrt();
        let angle = std::f64::consts::TAU * self.uniform();
        sd * radius * angle.cos()
    }
}

/// Raw samples from the design, through the production admission.
pub fn simulate_corpus(
    design: &TimingDesign,
    protocol: &PairedTimingProtocol,
    seed: u64,
) -> FairResult<SimulatedCorpus> {
    design.validate(protocol)?;
    let model = design.dependence;
    let mut rng = SplitMix64(seed);
    let session_effects = (0..design.sessions)
        .map(|_| rng.normal(model.session_sd))
        .collect::<Vec<_>>();
    let mut case_effects = (0..design.cases)
        .map(|_| rng.normal(model.case_sd))
        .collect::<Vec<_>>();
    let centre = median_in_place(&mut case_effects.clone());
    for effect in &mut case_effects {
        *effect -= centre;
    }
    let innovation = (1.0 - model.pair_autocorrelation.powi(2)).sqrt();
    let mut cases = Vec::with_capacity(design.cases);
    let mut missing_cells = 0;
    let mut excluded_cases = 0;
    for (case, case_effect) in case_effects.iter().enumerate() {
        let mut candidate = Vec::new();
        let mut reference = Vec::new();
        let mut labels = Vec::new();
        for (session, session_effect) in session_effects.iter().enumerate() {
            let cell_effect = session_effect + rng.normal(model.interaction_sd);
            let draw = rng.uniform();
            let missing = match design.missingness {
                Missingness::None => false,
                Missingness::Mcar { probability } => draw < probability,
                Missingness::Mnar { probability } => cell_effect < 0.0 && draw < probability,
            };
            let mut noise = rng.normal(model.pair_sd);
            for pair in 0..design.pairs_per_cell {
                if pair > 0 {
                    noise =
                        model.pair_autocorrelation * noise + innovation * rng.normal(model.pair_sd);
                }
                if !missing {
                    let log = design.theta_log + case_effect + cell_effect + noise;
                    candidate.push(1.0e-3);
                    reference.push(1.0e-3 * log.exp());
                    labels.push(session as u32);
                }
            }
            if missing {
                missing_cells += 1;
            }
        }
        if candidate.len() < protocol.pairs {
            excluded_cases += 1;
            continue;
        }
        cases.push(
            PairedTimingCase::from_samples(
                format!("case-{case}"),
                protocol,
                vec![5.0e-3, 1.0e-3],
                candidate,
                reference,
            )?
            .with_process_blocks(labels),
        );
    }
    Ok(SimulatedCorpus {
        cases,
        target_log: design.theta_log,
        missing_cells,
        excluded_cases,
    })
}

/// One replication of the study.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoverageReplication {
    /// False when no case was admissible: counted as not covered and not
    /// promoted.
    pub interval: bool,
    pub covered: bool,
    pub decision: PairedTimingDecision,
    /// The percentile-endpoint rule alone (diagnostic).
    pub endpoint_decision: PairedTimingDecision,
    pub independent_blocks: usize,
    pub width_log: f64,
    pub missing_cells: usize,
    pub excluded_cases: usize,
}

pub fn coverage_replication(
    design: &TimingDesign,
    protocol: &PairedTimingProtocol,
    seed: u64,
) -> FairResult<CoverageReplication> {
    let corpus = simulate_corpus(design, protocol, seed)?;
    if corpus.cases.is_empty() {
        return Ok(CoverageReplication {
            interval: false,
            covered: false,
            decision: PairedTimingDecision::Inconclusive,
            endpoint_decision: PairedTimingDecision::Inconclusive,
            independent_blocks: 0,
            width_log: f64::INFINITY,
            missing_cells: corpus.missing_cells,
            excluded_cases: corpus.excluded_cases,
        });
    }
    let interval = case_clustered_bootstrap(&corpus.cases, protocol)?;
    Ok(CoverageReplication {
        interval: true,
        covered: interval.lower_log <= corpus.target_log && corpus.target_log <= interval.upper_log,
        decision: paired_timing_decision(&interval, protocol.required_speedup),
        endpoint_decision: percentile_endpoint_decision(&interval, protocol.required_speedup),
        independent_blocks: interval.independent_blocks,
        width_log: interval.upper_log - interval.lower_log,
        missing_cells: corpus.missing_cells,
        excluded_cases: corpus.excluded_cases,
    })
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoverageScenario {
    pub id: String,
    pub design: TimingDesign,
    /// Primary scenarios decide the verdict; the others are sensitivity.
    pub primary: bool,
}

/// The preregistered grid: sessions {6, 12} x cases {1, 5} x missingness
/// {none, MCAR 0.1, MCAR 0.3, MNAR 0.3} x theta {1, 1.15, 1.3}; MNAR is
/// sensitivity, the rest is primary.
pub fn preregistered_coverage_grid(protocol: &PairedTimingProtocol) -> Vec<CoverageScenario> {
    let mut grid = Vec::new();
    for sessions in [6, 12] {
        for cases in [1, 5] {
            for (missing_id, missingness) in [
                ("none", Missingness::None),
                ("mcar0.1", Missingness::Mcar { probability: 0.1 }),
                ("mcar0.3", Missingness::Mcar { probability: 0.3 }),
                ("mnar0.3", Missingness::Mnar { probability: 0.3 }),
            ] {
                for speedup in [1.0_f64, 1.15, 1.3] {
                    grid.push(CoverageScenario {
                        id: format!("s{sessions}-c{cases}-{missing_id}-theta{speedup}"),
                        design: TimingDesign {
                            sessions,
                            cases,
                            pairs_per_cell: protocol.pairs,
                            theta_log: speedup.ln(),
                            dependence: DependenceModel::PREREGISTERED,
                            missingness,
                        },
                        primary: !matches!(missingness, Missingness::Mnar { .. }),
                    });
                }
            }
        }
    }
    grid
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoverageScenarioResult {
    pub id: String,
    pub design: TimingDesign,
    pub primary: bool,
    pub replications: usize,
    pub intervals: usize,
    pub covered: usize,
    pub coverage: f64,
    pub promote: usize,
    pub block: usize,
    pub inconclusive: usize,
    /// Promotion rate when `theta <= ln(required)`; `None` otherwise.
    pub false_promote_rate: Option<f64>,
    pub endpoint_promote: usize,
    /// Replications whose endpoint decision the Monte-Carlo or session gate
    /// withheld: resampling-resolution, not population coverage.
    pub gate_withheld: usize,
    pub below_min_sessions: usize,
    pub mean_missing_cells: f64,
    pub mean_excluded_cases: f64,
    pub median_width_log: f64,
    pub coverage_pass: bool,
    pub false_promote_pass: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoverageStudyReport {
    pub schema: String,
    pub contract: String,
    pub seed: u64,
    pub replications: usize,
    pub bootstrap_resamples: usize,
    pub confidence_level: f64,
    pub required_speedup: f64,
    pub mc_failure_budget: f64,
    pub min_coverage: f64,
    pub max_false_promote: f64,
    pub scenarios: Vec<CoverageScenarioResult>,
    pub primary_pass: bool,
    pub sensitivity_pass: bool,
    /// `PASS` iff every primary scenario passes.
    pub verdict: String,
}

/// Independent data and bootstrap seeds of one replication, keyed by the
/// scenario id (so a scenario run alone reproduces its grid rows) and the
/// replication index. The data and the resampling never share a stream
/// (re-audit R3 review: the first study reused one seed for both).
pub fn replication_seeds(study_seed: u64, scenario_id: &str, replication: usize) -> (u64, u64) {
    // FNV-1a of the id.
    let id_hash = scenario_id
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
        });
    let mut rng =
        SplitMix64(study_seed ^ id_hash ^ (replication as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15));
    // Two consecutive SplitMix64 outputs seed two separate streams.
    (rng.next_u64(), rng.next_u64())
}

/// Runs every scenario; `threads` only changes the wall time, not a bit of
/// the report.
pub fn coverage_study(
    scenarios: &[CoverageScenario],
    replications: usize,
    seed: u64,
    threads: usize,
    base: &PairedTimingProtocol,
) -> FairResult<CoverageStudyReport> {
    base.validate()?;
    if replications == 0 {
        return Err(FairError::Invalid(
            "coverage study needs at least one replication".into(),
        ));
    }
    let threads = threads.max(1);
    let threshold_log = base.required_speedup.ln();
    let mut results = Vec::with_capacity(scenarios.len());
    for scenario in scenarios {
        let runs = std::thread::scope(|scope| {
            let workers = (0..threads)
                .map(|worker| {
                    scope.spawn(move || {
                        (worker..replications)
                            .step_by(threads)
                            .map(|replication| {
                                let (data_seed, bootstrap_seed) =
                                    replication_seeds(seed, &scenario.id, replication);
                                let protocol = PairedTimingProtocol {
                                    seed: bootstrap_seed,
                                    ..base.clone()
                                };
                                coverage_replication(&scenario.design, &protocol, data_seed)
                                    .map(|run| (replication, run))
                            })
                            .collect::<FairResult<Vec<_>>>()
                    })
                })
                .collect::<Vec<_>>();
            let mut runs = Vec::with_capacity(replications);
            for worker in workers {
                runs.extend(worker.join().expect("coverage worker panicked")?);
            }
            runs.sort_by_key(|(replication, _)| *replication);
            Ok::<_, FairError>(runs.into_iter().map(|(_, run)| run).collect::<Vec<_>>())
        })?;
        let count = |predicate: &dyn Fn(&CoverageReplication) -> bool| {
            runs.iter().filter(|run| predicate(run)).count()
        };
        let total = replications as f64;
        let covered = count(&|run| run.covered);
        let promote = count(&|run| run.decision == PairedTimingDecision::Promote);
        let coverage = covered as f64 / total;
        let false_promote_rate =
            (scenario.design.theta_log <= threshold_log).then_some(promote as f64 / total);
        let mut widths = runs
            .iter()
            .filter(|run| run.interval)
            .map(|run| run.width_log)
            .collect::<Vec<_>>();
        let coverage_pass = coverage >= COVERAGE_STUDY_MIN_COVERAGE;
        let false_promote_pass =
            false_promote_rate.is_none_or(|rate| rate <= COVERAGE_STUDY_MAX_FALSE_PROMOTE);
        results.push(CoverageScenarioResult {
            id: scenario.id.clone(),
            design: scenario.design.clone(),
            primary: scenario.primary,
            replications,
            intervals: count(&|run| run.interval),
            covered,
            coverage,
            promote,
            block: count(&|run| run.decision == PairedTimingDecision::Block),
            inconclusive: count(&|run| run.decision == PairedTimingDecision::Inconclusive),
            false_promote_rate,
            endpoint_promote: count(&|run| run.endpoint_decision == PairedTimingDecision::Promote),
            gate_withheld: count(&|run| run.interval && run.decision != run.endpoint_decision),
            below_min_sessions: count(&|run| {
                run.independent_blocks < crate::PAIRED_TIMING_MIN_INDEPENDENT_BLOCKS
            }),
            mean_missing_cells: runs.iter().map(|run| run.missing_cells as f64).sum::<f64>()
                / total,
            mean_excluded_cases: runs
                .iter()
                .map(|run| run.excluded_cases as f64)
                .sum::<f64>()
                / total,
            median_width_log: if widths.is_empty() {
                f64::INFINITY
            } else {
                median_in_place(&mut widths)
            },
            coverage_pass,
            false_promote_pass,
        });
    }
    let passes = |primary: bool| {
        results
            .iter()
            .filter(|result| result.primary == primary)
            .all(|result| result.coverage_pass && result.false_promote_pass)
    };
    let primary_pass = passes(true);
    let sensitivity_pass = passes(false);
    Ok(CoverageStudyReport {
        schema: COVERAGE_STUDY_SCHEMA.into(),
        contract: TIMING_DESIGN_CONTRACT.into(),
        seed,
        replications,
        bootstrap_resamples: base.bootstrap_resamples,
        confidence_level: base.confidence_level,
        required_speedup: base.required_speedup,
        mc_failure_budget: PAIRED_TIMING_MC_FAILURE_BUDGET,
        min_coverage: COVERAGE_STUDY_MIN_COVERAGE,
        max_false_promote: COVERAGE_STUDY_MAX_FALSE_PROMOTE,
        scenarios: results,
        primary_pass,
        sensitivity_pass,
        verdict: if primary_pass { "PASS" } else { "FAIL" }.into(),
    })
}
