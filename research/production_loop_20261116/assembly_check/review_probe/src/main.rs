//! Reviewer probe of loop L1 node V26 (multi-colouring check of the assembly
//! client). Read-only use of the builder's crates. Subcommands:
//!
//!   cancel        several undeclared dependencies in one row whose
//!                 contributions cancel in pairs (constructions B and G)
//!   collapse      search for a single undeclared dependency whose entries
//!                 collapse after the subtraction and the division
//!   multi         generic rows with two or three undeclared dependencies
//!   single-g      every single-entry omission on patterns where G is chosen
//!   constructions dump of the check colourings for an independent check
//!
//! Every result is one JSON object per line behind the tag `REVIEW_JSON`.

use std::sync::Arc;

use rodas5p_core::{CoreError, CoreResult, DenseMatrix, WorkCounters};
use rodas5p_integrators::OdeProblem;
use rodas5p_integrators::stage_clients::DenseLuMode;
use rodas5p_integrators::stage_clients::assembly::{
    AssemblyError, AssemblyOptions, AssemblyStageClient, CheckColourings, CheckColumns,
    LinearBackend, RhsPurity, SparsityPattern, colour_columns, colouring_is_valid,
};
use serde_json::{Value, json};

type Rhs = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }

    fn unit(&mut self) -> f64 {
        (self.next() % (1_u64 << 40)) as f64 / (1_u64 << 40) as f64
    }

    fn below(&mut self, k: usize) -> usize {
        (self.next() % k as u64) as usize
    }
}

/// `crates/vigilode/src/fd.rs:443-455`, as in the engine.
fn increment(yj: f64) -> (f64, f64) {
    let raw = f64::EPSILON.sqrt() * yj.abs().max(1.0);
    let forward = yj + raw;
    let shifted = if forward.is_finite() {
        forward
    } else {
        yj - raw
    };
    (shifted, shifted - yj)
}

fn problem_of(id: &str, n: usize, rhs: Rhs) -> OdeProblem {
    OdeProblem::new(
        id.to_string(),
        n,
        rhs,
        None,
        Some(Arc::new(|_t: f64, _y: &[f64]| -> CoreResult<DenseMatrix> {
            Err(CoreError::InvalidInput("finite differences only".into()))
        })),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

#[derive(Clone, Copy)]
struct Arm {
    label: &'static str,
    check: CheckColumns,
    workers: Option<usize>,
}

const ARMS: [Arm; 9] = [
    Arm {
        label: "build_only@seq",
        check: CheckColumns::None,
        workers: None,
    },
    Arm {
        label: "S8@seq",
        check: CheckColumns::Sample(8),
        workers: None,
    },
    Arm {
        label: "FA@seq",
        check: CheckColumns::All,
        workers: None,
    },
    Arm {
        label: "FA_P@t2",
        check: CheckColumns::AllPooled,
        workers: Some(2),
    },
    Arm {
        label: "FA_P@t6",
        check: CheckColumns::AllPooled,
        workers: Some(6),
    },
    Arm {
        label: "MC@seq",
        check: CheckColumns::MultiColouring,
        workers: None,
    },
    Arm {
        label: "MC@t6",
        check: CheckColumns::MultiColouring,
        workers: Some(6),
    },
    Arm {
        label: "MC_P@t3",
        check: CheckColumns::MultiColouringPooled,
        workers: Some(3),
    },
    Arm {
        label: "MC_P@t6",
        check: CheckColumns::MultiColouringPooled,
        workers: Some(6),
    },
];

const FA: Arm = ARMS[2];
const MC: Arm = ARMS[5];

fn decide(
    problem: &OdeProblem,
    y0: &[f64],
    pattern: &SparsityPattern,
    arm: Arm,
) -> Result<usize, AssemblyError> {
    let options =
        AssemblyOptions::coloured(pattern.clone()).with_first_use_check(CheckColumns::None);
    let options = match arm.workers {
        None => options,
        Some(workers) => options.threaded(workers, RhsPurity::pure_and_reentrant()),
    };
    let mut client = AssemblyStageClient::new(
        pattern.n(),
        LinearBackend::DenseLu(DenseLuMode::InPlace),
        options,
    )?;
    client.verify(problem, 0.0, y0, arm.check, &mut WorkCounters::default())
}

fn detected(result: &Result<usize, AssemblyError>) -> bool {
    result
        .as_ref()
        .is_err_and(AssemblyError::is_contract_violation)
}

fn text(result: &Result<usize, AssemblyError>) -> String {
    match result {
        Ok(columns) => format!("pass({columns})"),
        Err(error) => format!("{error:?}"),
    }
}

/// Row `i` at `y0` with the columns `shifted` carrying their increments, with
/// and without the shift of column `k`.
fn row_pair(problem: &OdeProblem, y0: &[f64], shifted: &[usize], i: usize, k: usize) -> (f64, f64) {
    let mut state = y0.to_vec();
    for &j in shifted {
        state[j] = increment(y0[j]).0;
    }
    let mut out = vec![0.0; y0.len()];
    problem
        .eval_rhs_into(0.0, &state, &mut out, &mut WorkCounters::default())
        .unwrap();
    let with = out[i];
    state[k] = y0[k];
    out.fill(0.0);
    problem
        .eval_rhs_into(0.0, &state, &mut out, &mut WorkCounters::default())
        .unwrap();
    (with, out[i])
}

/// The states with column `k` shifted that MC evaluates: the client's group of
/// `k` (the build) and the group of `k` in every further colouring.
fn mc_states(pattern: &SparsityPattern, k: usize) -> Vec<Vec<usize>> {
    let (c1, _) = colour_columns(pattern);
    let mut states = vec![(0..pattern.n()).filter(|&j| c1[j] == c1[k]).collect::<Vec<_>>()];
    let plan = CheckColourings::of(pattern).unwrap();
    for r in 0..plan.colourings() {
        states.push(plan.columns(r, plan.colour(r, k)));
    }
    states
}

/// The states with column `k` shifted that FA evaluates: the build group and
/// the column alone.
fn fa_states(pattern: &SparsityPattern, k: usize) -> Vec<Vec<usize>> {
    let (c1, _) = colour_columns(pattern);
    vec![
        (0..pattern.n()).filter(|&j| c1[j] == c1[k]).collect::<Vec<_>>(),
        vec![k],
    ]
}

/// The pre-declared rule: rounding-limited for an arm if at every state the
/// arm evaluates with `k` shifted, row `i` keeps its bits without that shift.
fn limited(problem: &OdeProblem, y0: &[f64], states: &[Vec<usize>], i: usize, k: usize) -> (bool, Vec<Value>) {
    let mut all = true;
    let mut rows = Vec::new();
    for state in states {
        let (with, without) = row_pair(problem, y0, state, i, k);
        let equal = with.to_bits() == without.to_bits();
        all &= equal;
        rows.push(json!({"columns_shifted": state.len(), "with": with, "without": without, "bits_equal": equal}));
    }
    (all, rows)
}

fn band_rows(n: usize, lower: usize, upper: usize) -> Vec<Vec<usize>> {
    (0..n)
        .map(|i| (i.saturating_sub(lower)..=(i + upper).min(n - 1)).collect())
        .collect()
}

fn ring_rows(n: usize) -> Vec<Vec<usize>> {
    (0..n)
        .map(|i| {
            let mut row = vec![(i + n - 1) % n, i, (i + 1) % n];
            row.sort_unstable();
            row.dedup();
            row
        })
        .collect()
}

/// 2-D periodic 5-point grid, natural order.
fn torus_rows(nx: usize, ny: usize) -> Vec<Vec<usize>> {
    let mut rows = Vec::new();
    for y in 0..ny {
        for x in 0..nx {
            let at = |x: usize, y: usize| y * nx + x;
            let mut row = vec![
                at(x, y),
                at((x + 1) % nx, y),
                at((x + nx - 1) % nx, y),
                at(x, (y + 1) % ny),
                at(x, (y + ny - 1) % ny),
            ];
            row.sort_unstable();
            row.dedup();
            rows.push(row);
        }
    }
    rows
}

fn random_rows(n: usize, density: f64, rng: &mut Lcg) -> Vec<Vec<usize>> {
    (0..n)
        .map(|i| {
            (0..n)
                .filter(|&j| j == i || rng.unit() < density)
                .collect()
        })
        .collect()
}

/// `f_i = sum_j a_ij y_j (1 + y_j / 10)` over `rows[i]` (a generic problem
/// whose true pattern is `rows`).
fn generic(id: &str, rows: &[Vec<usize>], rng: &mut Lcg) -> (OdeProblem, Vec<f64>) {
    let n = rows.len();
    let coefficients: Vec<Vec<(usize, f64)>> = rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|&j| {
                    let magnitude = 0.5 + rng.unit();
                    (j, if rng.below(2) == 0 { magnitude } else { -magnitude })
                })
                .collect()
        })
        .collect();
    let rhs: Rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| -> CoreResult<()> {
        for (value, row) in out.iter_mut().zip(&coefficients) {
            let mut acc = 0.0;
            for &(j, a) in row {
                acc += a * y[j] * (1.0 + 0.1 * y[j]);
            }
            *value = acc;
        }
        Ok(())
    });
    let y0 = (0..n).map(|_| 0.5 + rng.unit()).collect();
    (problem_of(id, n, rhs), y0)
}

fn plan_record(pattern: &SparsityPattern) -> Value {
    let plan = CheckColourings::of(pattern).unwrap();
    json!({
        "construction": format!("{:?}", plan.construction()),
        "colourings": plan.colourings(),
        "groups": (0..plan.colourings()).map(|r| plan.groups(r)).collect::<Vec<_>>(),
        "total_groups": plan.total_groups(),
        "total_band": plan.total_groups_band(),
        "total_refinement": plan.total_groups_refinement(),
    })
}

// ------------------------------------------------------------------ cancel

/// Declared rows `declared`, plus `(y_a - y_b) + (y_c - y_d)` in row `row` with
/// `y_a = y_b = y_c = y_d`: four undeclared dependencies whose contributions
/// cancel in pairs whenever {a, b} and {c, d} (or {a, d} and {b, c}) share
/// groups.
fn cancel_case(id: &str, declared_rows: Vec<Vec<usize>>, row: usize, quad: [usize; 4], seed: u64) {
    let n = declared_rows.len();
    let mut rng = Lcg(seed);
    let coefficients: Vec<Vec<(usize, f64)>> = declared_rows
        .iter()
        .map(|r| r.iter().map(|&j| (j, 0.5 + rng.unit())).collect())
        .collect();
    let [a, b, c, d] = quad;
    let rhs: Rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| -> CoreResult<()> {
        for (i, (value, r)) in out.iter_mut().zip(&coefficients).enumerate() {
            let mut acc = 0.0;
            for &(j, w) in r {
                acc += w * y[j] * (1.0 + 0.1 * y[j]);
            }
            if i == row {
                acc += (y[a] - y[b]) + (y[c] - y[d]);
            }
            *value = acc;
        }
        Ok(())
    });
    let problem = problem_of(id, n, rhs);
    let mut y0: Vec<f64> = (0..n).map(|_| 0.5 + rng.unit()).collect();
    for &j in &quad {
        y0[j] = 0.7;
    }
    let declared = SparsityPattern::from_rows(n, &declared_rows).unwrap();
    for &j in &quad {
        assert!(!declared.contains(row, j), "the four columns must be undeclared in the row");
    }
    let plan = CheckColourings::of(&declared).unwrap();
    let (c1, _) = colour_columns(&declared);
    let colours: Vec<Value> = quad
        .iter()
        .map(|&j| {
            json!({"column": j, "c1": c1[j],
                   "further": (0..plan.colourings()).map(|r| plan.colour(r, j)).collect::<Vec<_>>()})
        })
        .collect();
    let mut decisions = serde_json::Map::new();
    for arm in ARMS {
        let result = decide(&problem, &y0, &declared, arm);
        decisions.insert(arm.label.into(), json!({"detected": detected(&result), "decision": text(&result)}));
    }
    let mut evidence = Vec::new();
    for &k in &quad {
        let (mc_limited, mc_rows) = limited(&problem, &y0, &mc_states(&declared, k), row, k);
        let (fa_limited, fa_rows) = limited(&problem, &y0, &fa_states(&declared, k), row, k);
        evidence.push(json!({"entry": [row, k], "rounding_limited_for_MC": mc_limited, "rounding_limited_for_FA": fa_limited,
                             "mc_states": mc_rows, "fa_states": fa_rows}));
    }
    println!(
        "REVIEW_JSON {}",
        json!({"probe": "cancel", "case": id, "n": n, "row": row, "undeclared": quad,
               "plan": plan_record(&declared), "colours_of_the_four": colours,
               "decisions": decisions, "evidence": evidence})
    );
}

fn cancel() {
    // Construction B: tridiagonal declaration of order 60 (c1 = j mod 3, moduli 3, 4, 5).
    // {0, 45} and {1, 16} share groups mod 3 and mod 5, {0, 16} and {45, 1} mod 4.
    cancel_case("cancel-B-tridiagonal-n60", band_rows(60, 1, 1), 30, [0, 45, 1, 16], 11);
    // Construction G: the true periodic ring of order 84 (c1 = j mod 3, classes of 28,
    // primes 2, 3, 5); ranks 0, 10, 1, 21 in class 0 are the columns 0, 30, 3, 63.
    cancel_case("cancel-G-ring-n84", ring_rows(84), 40, [0, 30, 3, 63], 12);
    // Control: the same four columns without the pairing (MC is expected to detect).
    cancel_case("control-B-tridiagonal-n60-unpaired", band_rows(60, 1, 1), 30, [0, 44, 1, 17], 13);
}

// ------------------------------------------------------------------ collapse

/// One undeclared dependency `(i, k)` of size about one ulp of the entries it
/// is aliased into. Declared tridiagonal of order 60; `k = i + 21` shares the
/// group of `i` mod 3 (c1 and the first further colouring) and of `i + 1`
/// mod 4 and mod 5.
fn collapse() {
    let n = 60;
    let i = 20;
    let k = i + 21;
    let rows = band_rows(n, 1, 1);
    let declared = SparsityPattern::from_rows(n, &rows).unwrap();
    let mut rng = Lcg(0x5eed_0026);
    let mut hits = 0;
    let mut trials = 0;
    let mut tally = std::collections::BTreeMap::<String, u64>::new();
    while trials < 20_000 && hits < 3 {
        trials += 1;
        let mut base: Vec<f64> = (0..n).map(|_| 0.5 + rng.unit()).collect();
        base[i] = 1.55 + 0.43 * rng.unit();
        base[i + 1] = 1.55 + 0.43 * rng.unit();
        base[i - 1] = 1.3;
        base[k] = 1.0;
        let c0 = 1.0 + 0.04 * rng.unit();
        let c1 = 1.0 + 0.04 * rng.unit();
        let c2 = 0.75;
        let delta_i = increment(base[i]).1;
        let f_prime = c0 * delta_i;
        let ulp = f64::from_bits(f_prime.to_bits() + 1) - f_prime;
        let delta_k = increment(base[k]).1;
        let d = ulp / delta_k * (0.55 + 0.9 * rng.unit());
        let mut generic_rng = Lcg(trials as u64);
        let coefficients: Vec<Vec<(usize, f64)>> = rows
            .iter()
            .map(|r| r.iter().map(|&j| (j, 0.5 + generic_rng.unit())).collect())
            .collect();
        let s = base.clone();
        let rhs: Rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| -> CoreResult<()> {
            for (m, (value, r)) in out.iter_mut().zip(&coefficients).enumerate() {
                if m == i {
                    let mut acc = c0 * (y[i] - s[i]);
                    acc += c1 * (y[i + 1] - s[i + 1]);
                    acc += c2 * (y[i - 1] - s[i - 1]);
                    acc += d * (y[k] - s[k]);
                    *value = acc;
                } else {
                    let mut acc = 0.0;
                    for &(j, w) in r {
                        acc += w * y[j] * (1.0 + 0.1 * y[j]);
                    }
                    *value = acc;
                }
            }
            Ok(())
        });
        let problem = problem_of("collapse", n, rhs);
        let full = decide(&problem, &base, &declared, FA);
        let multi = decide(&problem, &base, &declared, MC);
        let key = format!("FA {} / MC {}", detected(&full), detected(&multi));
        *tally.entry(key).or_default() += 1;
        if detected(&full) && !detected(&multi) {
            hits += 1;
            let (mc_limited, mc_rows) = limited(&problem, &base, &mc_states(&declared, k), i, k);
            let (fa_limited, fa_rows) = limited(&problem, &base, &fa_states(&declared, k), i, k);
            let mut decisions = serde_json::Map::new();
            for arm in ARMS {
                let result = decide(&problem, &base, &declared, arm);
                decisions.insert(arm.label.into(), json!({"detected": detected(&result), "decision": text(&result)}));
            }
            println!(
                "REVIEW_JSON {}",
                json!({"probe": "collapse", "trial": trials, "n": n, "entry": [i, k],
                       "y_i": base[i], "y_i_plus_1": base[i + 1], "c0": c0, "c1": c1, "d": d,
                       "omitted_entry_value_d": d,
                       "plan": plan_record(&declared), "decisions": decisions,
                       "rounding_limited_for_MC": mc_limited, "rounding_limited_for_FA": fa_limited,
                       "mc_states": mc_rows, "fa_states": fa_rows})
            );
        }
    }
    println!("REVIEW_JSON {}", json!({"probe": "collapse-summary", "trials": trials, "hits": hits, "tally": tally}));
}

// ------------------------------------------------------------------ multi / single-g

#[derive(Default)]
struct Sweep {
    cases: u64,
    at_build: u64,
    both: u64,
    neither: u64,
    fa_only: u64,
    mc_only: u64,
    fa_only_not_limited: Vec<Value>,
    mc_only_fa_not_limited: Vec<Value>,
    pooled_mismatch: u64,
}

/// One wrong declaration: `truth` minus `omitted`, on the generic problem of `truth`.
fn one_case(
    problem: &OdeProblem,
    y0: &[f64],
    truth: &[Vec<usize>],
    omitted: &[(usize, usize)],
    pooled: bool,
    sweep: &mut Sweep,
) {
    let n = truth.len();
    let mut wrong = truth.to_vec();
    for &(i, k) in omitted {
        wrong[i].retain(|&j| j != k);
    }
    let pattern = SparsityPattern::from_rows(n, &wrong).unwrap();
    sweep.cases += 1;
    let build = decide(problem, y0, &pattern, ARMS[0]);
    if detected(&build) {
        sweep.at_build += 1;
        sweep.both += 1;
        return;
    }
    let full = decide(problem, y0, &pattern, FA);
    let multi = decide(problem, y0, &pattern, MC);
    if pooled {
        let full_p = decide(problem, y0, &pattern, ARMS[4]);
        let multi_p = decide(problem, y0, &pattern, ARMS[8]);
        if text(&full_p) != text(&full) || text(&multi_p) != text(&multi) {
            sweep.pooled_mismatch += 1;
        }
    }
    let limited_all = |states_of: &dyn Fn(usize) -> Vec<Vec<usize>>| -> bool {
        omitted
            .iter()
            .all(|&(i, k)| limited(problem, y0, &states_of(k), i, k).0)
    };
    match (detected(&full), detected(&multi)) {
        (true, true) => sweep.both += 1,
        (false, false) => sweep.neither += 1,
        (true, false) => {
            sweep.fa_only += 1;
            if !limited_all(&|k| mc_states(&pattern, k)) {
                sweep.fa_only_not_limited.push(json!({"omitted": omitted, "n": n,
                    "fa": text(&full), "plan": plan_record(&pattern)}));
            }
        }
        (false, true) => {
            sweep.mc_only += 1;
            if !limited_all(&|k| fa_states(&pattern, k)) {
                sweep.mc_only_fa_not_limited.push(json!({"omitted": omitted, "n": n, "mc": text(&multi)}));
            }
        }
    }
}

fn report(label: &str, plan: Value, sweep: Sweep) {
    println!(
        "REVIEW_JSON {}",
        json!({"probe": label, "plan_of_truth": plan, "cases": sweep.cases, "at_build": sweep.at_build,
               "both": sweep.both, "neither": sweep.neither, "fa_only": sweep.fa_only, "mc_only": sweep.mc_only,
               "fa_only_not_rounding_limited_for_MC": sweep.fa_only_not_limited.len(),
               "first_fa_only_not_limited": sweep.fa_only_not_limited.iter().take(3).collect::<Vec<_>>(),
               "mc_only_fa_not_rounding_limited": sweep.mc_only_fa_not_limited.len(),
               "pooled_mismatch": sweep.pooled_mismatch})
    );
}

fn patterns_for_sweeps(rng: &mut Lcg) -> Vec<(String, Vec<Vec<usize>>)> {
    let mut out = Vec::new();
    for n in [9, 17, 31] {
        for (lower, upper) in [(1, 1), (2, 2), (1, 3), (0, 2), (3, 0)] {
            out.push((format!("band-n{n}-{lower}-{upper}"), band_rows(n, lower, upper)));
        }
    }
    for n in [30, 50, 84] {
        out.push((format!("ring-n{n}"), ring_rows(n)));
    }
    out.push(("torus-8x8".into(), torus_rows(8, 8)));
    out.push(("torus-6x10".into(), torus_rows(6, 10)));
    for (n, density) in [(40, 0.05), (60, 0.04), (90, 0.03)] {
        out.push((format!("random-n{n}-d{density}"), random_rows(n, density, rng)));
    }
    out
}

/// Two (and, on random rows, three) undeclared dependencies in one row.
fn multi() {
    let mut rng = Lcg(0x0026_5eed_0002);
    for (id, truth) in patterns_for_sweeps(&mut rng) {
        let (problem, y0) = generic(&id, &truth, &mut rng);
        let plan = plan_record(&SparsityPattern::from_rows(truth.len(), &truth).unwrap());
        let mut sweep = Sweep::default();
        for (i, row) in truth.iter().enumerate() {
            for (p, &a) in row.iter().enumerate() {
                for &b in &row[p + 1..] {
                    one_case(&problem, &y0, &truth, &[(i, a), (i, b)], false, &mut sweep);
                }
            }
            if row.len() >= 3 {
                for _ in 0..3 {
                    let mut picks: Vec<usize> = row.clone();
                    while picks.len() > 3 {
                        picks.remove(rng.below(picks.len()));
                    }
                    let omitted: Vec<(usize, usize)> = picks.iter().map(|&k| (i, k)).collect();
                    one_case(&problem, &y0, &truth, &omitted, false, &mut sweep);
                }
            }
        }
        report(&format!("multi:{id}"), plan, sweep);
    }
}

/// Every single-entry omission (with pooled-arm equality) on the same patterns.
fn single_g() {
    let mut rng = Lcg(0x0026_5eed_0003);
    for (id, truth) in patterns_for_sweeps(&mut rng) {
        let (problem, y0) = generic(&id, &truth, &mut rng);
        let plan = plan_record(&SparsityPattern::from_rows(truth.len(), &truth).unwrap());
        let mut sweep = Sweep::default();
        for (i, row) in truth.iter().enumerate() {
            for &k in row {
                one_case(&problem, &y0, &truth, &[(i, k)], true, &mut sweep);
            }
        }
        report(&format!("single:{id}"), plan, sweep);
    }
}

// ------------------------------------------------------------------ constructions

fn constructions() {
    let mut rng = Lcg(0x0026_5eed_0004);
    let mut patterns: Vec<(String, Vec<Vec<usize>>)> = Vec::new();
    for n in [1, 2, 3, 4, 7, 12, 29, 60, 61, 100, 257] {
        for (lower, upper) in [(0, 0), (1, 0), (0, 1), (1, 1), (2, 2), (3, 1), (5, 5), (n - 1, 0), (0, n - 1), (n - 1, n - 1)] {
            if lower < n && upper < n {
                patterns.push((format!("band-n{n}-{lower}-{upper}"), band_rows(n, lower, upper)));
            }
        }
    }
    for n in [5, 30, 84, 85, 200] {
        patterns.push((format!("ring-n{n}"), ring_rows(n)));
    }
    for (nx, ny) in [(4, 4), (8, 8), (12, 7), (16, 16)] {
        patterns.push((format!("torus-{nx}x{ny}"), torus_rows(nx, ny)));
    }
    for (n, density) in [(10, 0.2), (40, 0.05), (60, 0.1), (120, 0.02), (300, 0.01)] {
        patterns.push((format!("random-n{n}-d{density}"), random_rows(n, density, &mut rng)));
    }
    // An arrow pattern (dense last row and column): c1 gives every column its own colour.
    let n = 20;
    let mut arrow: Vec<Vec<usize>> = (0..n).map(|i| if i + 1 < n { vec![i, n - 1] } else { (0..n).collect() }).collect();
    arrow[n - 1].dedup();
    patterns.push(("arrow-n20".into(), arrow));
    for (id, rows) in patterns {
        let n = rows.len();
        let pattern = SparsityPattern::from_rows(n, &rows).unwrap();
        let plan = CheckColourings::of(&pattern).unwrap();
        let (c1, colours) = colour_columns(&pattern);
        let further: Vec<Vec<usize>> = (0..plan.colourings())
            .map(|r| (0..n).map(|j| plan.colour(r, j)).collect())
            .collect();
        let valid: Vec<bool> = further.iter().map(|c| colouring_is_valid(&pattern, c)).collect();
        let members: Vec<Vec<Vec<usize>>> = (0..plan.colourings())
            .map(|r| (0..plan.groups(r)).map(|g| plan.columns(r, g)).collect())
            .collect();
        println!(
            "REVIEW_JSON {}",
            json!({"probe": "constructions", "case": id, "n": n, "rows": rows, "c1": c1, "c1_colours": colours,
                   "bandwidth": pattern.bandwidth(), "plan": plan_record(&pattern),
                   "further": further, "valid_by_crate": valid, "members": members})
        );
    }
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("cancel") => cancel(),
        Some("collapse") => collapse(),
        Some("multi") => multi(),
        Some("single-g") => single_g(),
        Some("constructions") => constructions(),
        _ => eprintln!("usage: v26-review-probe <cancel|collapse|multi|single-g|constructions>"),
    }
}
