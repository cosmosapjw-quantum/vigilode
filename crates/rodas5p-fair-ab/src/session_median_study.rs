//! Coverage and power study of the exact session-median interval (re-audit
//! R4 of 2026-10-01, R4-STAT-DEV-04). Research only; preregistered in
//! `research/r4_session_median_coverage_20261001/PREREGISTRATION.md`.
//!
//! Each replication draws `S` sessions of `C` cases: the session-cell median
//! is `Z_sc = theta + v_c + u_s(c) + median(e_sc1, .., e_scP)` with fixed
//! symmetric case offsets `v_c`, a session effect `u_s` and `P` pair
//! errors. In every in-domain law `u_s` and `e` are symmetric about 0 and
//! independent of each other, so each case's population median of `Z_sc` is
//! `theta + v_c` and the target `median_c(theta + v_c)` is `theta` (the
//! offsets are symmetric about 0). The session vectors are iid in every
//! in-domain law; the out-of-domain law adds a drift across sessions that
//! breaks it and is reported apart from the verdict.

use serde::{Deserialize, Serialize};

use crate::paired_timing::SplitMix64;
use crate::{
    FairError, FairResult, PairedTimingDecision, SessionCells, exact_session_median_interval,
    session_median_design,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SessionMedianLaw {
    /// Normal session effect (sd 0.05) and pair error (sd 0.2), case
    /// offsets of sd 0.1.
    Additive,
    /// [`Self::Additive`] with all case offsets 0 (`case_sd = 0`).
    CaseSdZero,
    /// Every case of a session has the same `Z` (one shared draw).
    IdenticalCases,
    /// A session effect of sd 0.3 shared by all cases: strong dependence
    /// between cases, iid sessions.
    SharedSessionEffect,
    /// Pair errors on the atoms {-0.2, 0, 0.2} (probabilities 1/4, 1/2,
    /// 1/4): ties, conservative coverage.
    Atoms,
    /// Cauchy pair errors (scale 0.05) and session effects (scale 0.02).
    HeavyTails,
    /// Out of domain: a linear drift of 0.02 per session, not iid.
    Drift,
}

impl SessionMedianLaw {
    pub const IN_DOMAIN: [Self; 6] = [
        Self::Additive,
        Self::CaseSdZero,
        Self::IdenticalCases,
        Self::SharedSessionEffect,
        Self::Atoms,
        Self::HeavyTails,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Additive => "additive",
            Self::CaseSdZero => "case-sd-zero",
            Self::IdenticalCases => "identical-cases",
            Self::SharedSessionEffect => "shared-session-effect",
            Self::Atoms => "atoms",
            Self::HeavyTails => "heavy-tails",
            Self::Drift => "drift-out-of-domain",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionMedianScenario {
    pub id: String,
    pub sessions: usize,
    pub cases: usize,
    pub law: SessionMedianLaw,
    pub theta_log: f64,
    pub in_domain: bool,
}

/// Pairs per session cell (odd, so the cell median is one pair).
pub const SESSION_MEDIAN_STUDY_PAIRS: usize = 15;

/// The preregistered grid: S in {6, 8, 12, 24}, C in {1, 5}, the six
/// in-domain laws and the drift law, theta in {ln 1.15, ln 1.3}.
pub fn session_median_study_grid() -> Vec<SessionMedianScenario> {
    let mut grid = Vec::new();
    for sessions in [6, 8, 12, 24] {
        for cases in [1, 5] {
            for law in SessionMedianLaw::IN_DOMAIN
                .into_iter()
                .chain([SessionMedianLaw::Drift])
            {
                for (label, theta) in [("theta1.15", 1.15_f64), ("theta1.3", 1.3)] {
                    grid.push(SessionMedianScenario {
                        id: format!("s{sessions}-c{cases}-{}-{label}", law.name()),
                        sessions,
                        cases,
                        law,
                        theta_log: theta.ln(),
                        in_domain: law != SessionMedianLaw::Drift,
                    });
                }
            }
        }
    }
    grid
}

fn uniform(rng: &mut SplitMix64) -> f64 {
    ((rng.next_u64() >> 11) as f64 + 0.5) * (1.0 / (1u64 << 53) as f64)
}

fn normal(rng: &mut SplitMix64, sd: f64) -> f64 {
    let radius = (-2.0 * uniform(rng).ln()).sqrt();
    sd * radius * (std::f64::consts::TAU * uniform(rng)).cos()
}

fn cauchy(rng: &mut SplitMix64, scale: f64) -> f64 {
    scale * (std::f64::consts::PI * (uniform(rng) - 0.5)).tan()
}

fn pair_error(rng: &mut SplitMix64, law: SessionMedianLaw) -> f64 {
    match law {
        SessionMedianLaw::Atoms => match rng.below(4) {
            0 => -0.2,
            1 | 2 => 0.0,
            _ => 0.2,
        },
        SessionMedianLaw::HeavyTails => cauchy(rng, 0.05),
        _ => normal(rng, 0.2),
    }
}

fn cell_median(rng: &mut SplitMix64, law: SessionMedianLaw) -> f64 {
    let mut errors = (0..SESSION_MEDIAN_STUDY_PAIRS)
        .map(|_| pair_error(rng, law))
        .collect::<Vec<_>>();
    errors.sort_by(f64::total_cmp);
    errors[SESSION_MEDIAN_STUDY_PAIRS / 2]
}

/// Symmetric case offsets `sd * {-1.2816, -0.5244, 0, 0.5244, 1.2816}`
/// (normal quantiles at 0.1, 0.3, 0.5, 0.7, 0.9) for five cases, 0 for one.
fn case_offsets(cases: usize, sd: f64) -> Vec<f64> {
    match cases {
        1 => vec![0.0],
        5 => [-1.2816, -0.5244, 0.0, 0.5244, 1.2816]
            .iter()
            .map(|q| q * sd)
            .collect(),
        _ => (0..cases)
            .map(|c| sd * (c as f64 - (cases as f64 - 1.0) / 2.0))
            .collect(),
    }
}

/// One replication's session cells.
pub fn simulate_session_cells(scenario: &SessionMedianScenario, seed: u64) -> Vec<SessionCells> {
    let mut rng = SplitMix64(seed);
    let law = scenario.law;
    let offsets = case_offsets(
        scenario.cases,
        if law == SessionMedianLaw::CaseSdZero {
            0.0
        } else {
            0.1
        },
    );
    (0..scenario.sessions)
        .map(|s| {
            let session_effect = match law {
                SessionMedianLaw::SharedSessionEffect => normal(&mut rng, 0.3),
                SessionMedianLaw::HeavyTails => cauchy(&mut rng, 0.02),
                SessionMedianLaw::Drift => 0.02 * s as f64,
                _ => normal(&mut rng, 0.05),
            };
            let shared = cell_median(&mut rng, law);
            let cells = offsets
                .iter()
                .enumerate()
                .map(|(c, offset)| {
                    let noise = if law == SessionMedianLaw::IdenticalCases {
                        shared
                    } else {
                        cell_median(&mut rng, law)
                    };
                    let offset = if law == SessionMedianLaw::IdenticalCases {
                        0.0
                    } else {
                        *offset
                    };
                    (
                        format!("case-{c}"),
                        scenario.theta_log + offset + session_effect + noise,
                    )
                })
                .collect();
            SessionCells {
                session: s as u32,
                cells,
                failed: false,
            }
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionMedianScenarioResult {
    pub scenario: SessionMedianScenario,
    pub replications: usize,
    pub k: Option<usize>,
    /// `1 - C q(S, k)` as a float, 1 when unbounded.
    pub design_coverage_lower: f64,
    pub covered: usize,
    pub coverage: f64,
    pub promote: usize,
    pub block: usize,
    pub unbounded: usize,
    /// One case, continuous law, finite design: `|miss rate - q| <=
    /// tolerance`; `None` where the bound is not exact.
    pub exact_check: Option<bool>,
    /// `coverage >= design_coverage_lower - tolerance` and no failed exact
    /// check.
    pub pass: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionMedianStudyReport {
    pub schema: String,
    pub seed: u64,
    pub replications: usize,
    pub alpha: String,
    /// Hoeffding half-width at `delta = 0.01 / (in-domain scenarios)`.
    pub tolerance: f64,
    pub scenarios: Vec<SessionMedianScenarioResult>,
    pub in_domain_failures: usize,
    pub out_of_domain_failures: usize,
    pub verdict: String,
}

/// FNV-1a of the scenario id and the replication index, so a scenario run
/// alone reproduces its rows.
pub fn session_median_replication_seed(seed: u64, scenario_id: &str, replication: usize) -> u64 {
    let id_hash = scenario_id
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
        });
    SplitMix64(seed ^ id_hash ^ (replication as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)).next_u64()
}

/// Run the study. `threads` changes only the wall time.
pub fn session_median_study(
    scenarios: &[SessionMedianScenario],
    replications: usize,
    seed: u64,
    threads: usize,
) -> FairResult<SessionMedianStudyReport> {
    if replications == 0 || scenarios.is_empty() {
        return Err(FairError::Invalid(
            "session-median study needs scenarios and replications".into(),
        ));
    }
    let in_domain = scenarios.iter().filter(|s| s.in_domain).count().max(1);
    let delta = 0.01 / in_domain as f64;
    let tolerance = ((2.0 / delta).ln() / (2.0 * replications as f64)).sqrt();
    let run = |scenario: &SessionMedianScenario| -> FairResult<SessionMedianScenarioResult> {
        let design = session_median_design(scenario.sessions, scenario.cases, 1, 20)?;
        let lower = design.simultaneous_coverage_lower.as_f64();
        let per_case_failure = design.per_case_failure.as_f64();
        let cases = (0..scenario.cases)
            .map(|c| format!("case-{c}"))
            .collect::<Vec<_>>();
        let (mut covered, mut promote, mut block, mut unbounded) = (0, 0, 0, 0);
        for replication in 0..replications {
            let rows = simulate_session_cells(
                scenario,
                session_median_replication_seed(seed, &scenario.id, replication),
            );
            let interval =
                exact_session_median_interval(&rows, &cases, scenario.sessions, 1, 20, 1.15)?;
            match (interval.lower, interval.upper) {
                (Some(l), Some(u)) => {
                    if l <= scenario.theta_log && scenario.theta_log <= u {
                        covered += 1;
                    }
                }
                _ => {
                    covered += 1;
                    unbounded += 1;
                }
            }
            match interval.decision {
                PairedTimingDecision::Promote => promote += 1,
                PairedTimingDecision::Block => block += 1,
                PairedTimingDecision::Inconclusive => {}
            }
        }
        let coverage = covered as f64 / replications as f64;
        // For one case and a continuous law the per-case bound is exact:
        // the miss rate must equal q(S, k) within the tolerance, both ways,
        // so an over-conservative implementation fails too.
        let exact_check = (scenario.cases == 1
            && design.k.is_some()
            && !matches!(
                scenario.law,
                SessionMedianLaw::Atoms | SessionMedianLaw::Drift
            ))
        .then(|| ((1.0 - coverage) - per_case_failure).abs() <= tolerance);
        Ok(SessionMedianScenarioResult {
            scenario: scenario.clone(),
            replications,
            k: design.k,
            design_coverage_lower: lower,
            covered,
            coverage,
            promote,
            block,
            unbounded,
            exact_check,
            pass: coverage >= lower - tolerance && exact_check != Some(false),
        })
    };
    // Scenarios are independent and seeded by id: a strided split over
    // scoped threads, reassembled in grid order.
    let threads = threads.clamp(1, scenarios.len());
    let mut slots: Vec<Option<FairResult<SessionMedianScenarioResult>>> =
        (0..scenarios.len()).map(|_| None).collect();
    std::thread::scope(|scope| {
        let handles = (0..threads)
            .map(|worker| {
                let run = &run;
                scope.spawn(move || {
                    (worker..scenarios.len())
                        .step_by(threads)
                        .map(|index| (index, run(&scenarios[index])))
                        .collect::<Vec<_>>()
                })
            })
            .collect::<Vec<_>>();
        for handle in handles {
            for (index, result) in handle.join().expect("study worker panicked") {
                slots[index] = Some(result);
            }
        }
    });
    let results = slots
        .into_iter()
        .map(|slot| slot.expect("every scenario ran"))
        .collect::<FairResult<Vec<_>>>()?;
    let in_domain_failures = results
        .iter()
        .filter(|r| r.scenario.in_domain && !r.pass)
        .count();
    let out_of_domain_failures = results
        .iter()
        .filter(|r| !r.scenario.in_domain && !r.pass)
        .count();
    Ok(SessionMedianStudyReport {
        schema: "vigilode-r4-session-median-study-v1".into(),
        seed,
        replications,
        alpha: "1/20".into(),
        tolerance,
        scenarios: results,
        in_domain_failures,
        out_of_domain_failures,
        verdict: if in_domain_failures == 0 {
            "PASS"
        } else {
            "FAIL"
        }
        .into(),
    })
}
