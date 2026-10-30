//! Opt-in cost instrumentation. NOT a solver speedup measurement.
use rvj_n4_source_kernel::n3h_source_bounded_copy::{Session, Source};
use std::time::Instant;
fn median(mut x: Vec<f64>) -> f64 {
    x.sort_by(|a, b| a.total_cmp(b));
    x[x.len() / 2]
}
fn main() -> Result<(), String> {
    println!("n,repeats,median_candidate_ms,median_enclosure_ms,ratio_enclosure_over_candidate");
    for n in [12, 24, 65, 257] {
        let mut s = Session::new(Source::toy(n)?)?;
        let mut candidate = Vec::new();
        let mut enclosure = Vec::new();
        for _ in 0..5 {
            let start = Instant::now();
            let a = s.propose(0.25, 2)?;
            candidate.push(start.elapsed().as_secs_f64() * 1e3);
            let start = Instant::now();
            let e = s.n4a_certify_first_pending(&a)?;
            enclosure.push(start.elapsed().as_secs_f64() * 1e3);
            if e.len() != n {
                return Err("mode count failure".into());
            }
            s.abort_pending();
            if !s.is_balanced() {
                return Err("work ledger failure".into());
            }
        }
        let c = median(candidate);
        let v = median(enclosure);
        println!("{n},5,{c:.8},{v:.8},{:.6}", v / c);
    }
    Ok(())
}
