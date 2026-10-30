//! Indicative per-attempt timing of the N3 research candidate only. No real competitor.
use rvj_n3_window_source_locked::{Session, Source};
use std::{hint::black_box, time::Instant};
fn main() {
    println!(
        "n,arm,repeats,median_ns,min_ns,max_ns,exponential_calls,moment_calls,polynomial_multiplies"
    );
    for n in [6, 8, 12, 24, 65, 257] {
        let source = Source::toy(n).unwrap();
        if n <= 8 {
            let mut s = Session::new(source).unwrap();
            let refusal = s.propose(0.25, 2).unwrap_err();
            assert!(refusal.contains("protected-small-n"));
            println!("{n},protected_skip,0,0,0,0,0,0,0");
            continue;
        }
        let mut samples = Vec::new();
        let mut total = (0, 0, 0);
        for iter in 0..35 {
            let mut s = Session::new(source.clone()).unwrap();
            let start = Instant::now();
            let a = s.propose(0.25, 2).unwrap();
            let b = a.projected_ideal_bound();
            black_box(b);
            s.decide(a, 1e-7).unwrap();
            let elapsed = start.elapsed().as_nanos() as u64;
            let w = s.work();
            if iter == 2 {
                total = (w.exponential_calls, w.moment_calls, w.polynomial_multiplies)
            }
            if iter >= 5 {
                samples.push(elapsed)
            }
        }
        samples.sort_unstable();
        let lo = samples[0];
        let hi = *samples.last().unwrap();
        let med = samples[samples.len() / 2];
        println!(
            "{n},n3_research_two_rate,{}, {med},{lo},{hi},{},{},{}",
            samples.len(),
            total.0,
            total.1,
            total.2
        );
    }
}
