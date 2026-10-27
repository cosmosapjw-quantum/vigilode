//! Isolate proactive comparator dominance stopping; no incumbent integration changes.
use serde_json::{Value, json};
use std::{hint::black_box, time::Instant};
use vigilode::{
    Atol, DerivativeFallback, Jacobian, Options, Problem, SwitchingConfig, SwitchingEvidence,
    SwitchingSolution, SwitchingSolver,
};

fn problem(tau: f64, frequency: f64, stiffness: f64, dimension: usize) -> Problem {
    Problem::new(dimension, move |t, y, o| {
        let l = 1.0 + 0.5 * stiffness * (1.0 - (40.0 * (t - tau)).tanh());
        let (s, c) = (frequency * t).sin_cos();
        for (out, yi) in o.iter_mut().zip(y) {
            *out = -l * (yi - c) - frequency * s;
        }
        Ok(())
    })
    .jacobian(Jacobian::dense(move |t, _, j| {
        let d = -(1.0 + 0.5 * stiffness * (1.0 - (40.0 * (t - tau)).tanh()));
        j.fill(0.0);
        for i in 0..dimension {
            j[i * dimension + i] = d;
        }
        Ok(())
    }))
    .time_derivative(move |t, y, o| {
        let q = (40.0 * (t - tau)).tanh();
        let l = 1.0 + 0.5 * stiffness * (1.0 - q);
        let lp = -20.0 * stiffness * (1.0 - q * q);
        let (s, c) = (frequency * t).sin_cos();
        for (out, yi) in o.iter_mut().zip(y) {
            *out = -lp * (yi - c) - l * frequency * s - frequency * frequency * c;
        }
        Ok(())
    })
}
fn summarize(r: &SwitchingSolution, frequency: f64) -> Value {
    let trajectory_error =
        r.t.iter()
            .zip(&r.y)
            .flat_map(|(t, y)| y.iter().map(move |yi| (yi - (frequency * t).cos()).abs()))
            .fold(0.0f64, f64::max);
    let lower_bound_returns = r
        .stats
        .segments
        .iter()
        .filter(|s| s.evidence == SwitchingEvidence::ExplicitCheaperThanComparatorLowerBound)
        .count();
    json!({"success":r.success,"state":r.final_state,"time":r.final_time,
        "trajectory_max_error":trajectory_error,"accuracy_cap":1e-5,
        "accuracy_pass":r.success && r.final_time==1.0 && trajectory_error<=1e-5,
        "attempts":r.stats.attempts,"rhs":r.stats.rhs_calls,
        "work":r.stats.segments.iter().map(|s|s.work_units).sum::<f64>(),
        "lower_bound_returns":lower_bound_returns,
        "accounting_complete":r.stats.accounting_complete,
        "counter_sum_pass":r.stats.attempts==r.stats.segments.iter().map(|s|s.attempts).sum::<usize>()
            && r.stats.rhs_calls==r.stats.segments.iter().map(|s|s.rhs_calls).sum::<u64>(),
        "segments":r.stats.segments})
}
fn solve(p: &Problem, o: &Options, c: &SwitchingConfig) -> SwitchingSolution {
    SwitchingSolver::solve(p, (0.0, 1.0), &vec![1.0; p.dimension()], o, c).unwrap()
}
fn batch(p: &Problem, o: &Options, c: &SwitchingConfig, n: usize) -> (f64, SwitchingSolution) {
    let start = Instant::now();
    let mut last = None;
    for _ in 0..n {
        last = Some(black_box(solve(black_box(p), black_box(o), black_box(c))));
    }
    (start.elapsed().as_secs_f64(), last.unwrap())
}
fn calibrate(p: &Problem, o: &Options, c: &SwitchingConfig) -> usize {
    let mut n = 1;
    loop {
        let (s, r) = batch(p, o, c, n);
        black_box(r);
        if s >= 0.08 || n >= 100000 {
            return n;
        }
        n *= 2;
    }
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let tau = args.get(1).map_or(0.4, |s| s.parse::<f64>().unwrap());
    let frequency = args.get(2).map_or(1.0, |s| s.parse::<f64>().unwrap());
    let stiffness = args.get(3).map_or(9999.0, |s| s.parse::<f64>().unwrap());
    let h0 = args.get(4).map_or(1e-4, |s| s.parse::<f64>().unwrap());
    let sentinel = args.get(5).map_or(0, |s| s.parse::<usize>().unwrap());
    let blocks = args.get(6).map_or(5, |s| s.parse::<usize>().unwrap());
    let windows = args.get(7).map_or(12, |s| s.parse::<usize>().unwrap());
    let dimension = args.get(8).map_or(1, |s| s.parse::<usize>().unwrap());
    let p = problem(tau, frequency, stiffness, dimension);
    let o = Options {
        rtol: 1e-7,
        atol: Atol::Scalar(1e-10),
        h0,
        max_steps: 50000,
        ..Default::default()
    };
    let a = SwitchingConfig {
        window_count: windows,
        implicit_residence: 1,
        probe_interval: 1,
        probe_attempt_budget: 64,
        return_cost_ratio: 0.99,
        return_sentinel_interval: sentinel,
        ..SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly)
    };
    let b = SwitchingConfig {
        stop_dominated_return_comparator: true,
        ..a.clone()
    };
    let ra = solve(&p, &o, &a);
    let rb = solve(&p, &o, &b);
    let sa = summarize(&ra, frequency);
    let sb = summarize(&rb, frequency);
    let accepted_equal = ra.t == rb.t && ra.y == rb.y && ra.final_state == rb.final_state;
    let mut samples = Vec::new();
    let mut deterministic = true;
    if blocks > 0 {
        let na = calibrate(&p, &o, &a);
        let nb = calibrate(&p, &o, &b);
        for block in 0..blocks {
            let order = if block % 2 == 0 {
                [false, true, true, false]
            } else {
                [true, false, false, true]
            };
            for (slot, optimized) in order.into_iter().enumerate() {
                let (c, n) = if optimized { (&b, nb) } else { (&a, na) };
                let (s, r) = batch(&p, &o, c, n);
                deterministic &=
                    summarize(&r, frequency) == if optimized { sb.clone() } else { sa.clone() };
                samples.push(json!({"block":block,"slot":slot,"optimized":optimized,
                    "repetitions":n,"seconds":s,"seconds_per_solve":s/n as f64}));
            }
        }
    }
    println!(
        "{}",
        json!({"schema":"vigilode.comparator-dominance-benchmark.v1",
        "tau":tau,"frequency":frequency,"stiffness":stiffness,"h0":h0,"sentinel_interval":sentinel,
        "windows":windows,"dimension":dimension,"options":o,"baseline_config":a,"optimized_config":b,"baseline":sa,"optimized":sb,"accepted_trajectory_equal":accepted_equal,
        "deterministic":deterministic,"samples":samples,
        "source_commit":std::env::var("VIGILODE_SOURCE_COMMIT").ok(),
        "timing_scope":"complete solve including probes, setup, allocation, counters; excludes JSON",
        "error_scope":"all adapter returned states including uniform requested windows; not continuous supremum"})
    );
}
