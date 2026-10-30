//! Descriptive research-only N4B work/cost telemetry, NOT a matched-error speedup.
use rvj_n4_source_kernel::n3h_source_bounded_copy::{Session, Source};
use std::time::Instant;
fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}
fn main() -> Result<(), String> {
    println!(
        "n,mode,replications,candidate_ms_median,certificate_and_commit_ms_median,ratio_extra_over_candidate,attempted,accepted,rejected,pending"
    );
    for n in [6, 8, 12, 24, 65, 257] {
        if n <= 8 {
            let mut s = Session::new(Source::toy(n)?)?;
            assert!(s.propose(0.25, 2).is_err());
            let w = s.work();
            println!(
                "{n},PROTECTED_BASELINE,0,0,0,0,{},{},{},{}",
                w.attempted, w.accepted, w.rejected, w.pending
            );
            continue;
        }
        let mut cand = vec![];
        let mut cert = vec![];
        let mut last = (0, 0, 0, 0);
        for _ in 0..3 {
            let mut s = Session::new(Source::toy(n)?)?;
            let now = Instant::now();
            let first = s.propose(0.25, 2)?;
            let t1 = now.elapsed();
            let now = Instant::now();
            let one = s.n4b_commit_first_with_float_enclosure(first, 1e-8)?;
            let v1 = now.elapsed();
            let now = Instant::now();
            let second = s.propose(0.25, 2)?;
            let t2 = now.elapsed();
            let now = Instant::now();
            s.n4b_commit_second_with_float_enclosure(second, &one, 1e-8)?;
            let v2 = now.elapsed();
            cand.push((t1 + t2).as_secs_f64() * 1e3);
            cert.push((v1 + v2).as_secs_f64() * 1e3);
            assert!(s.is_balanced());
            let w = s.work();
            last = (w.attempted, w.accepted, w.rejected, w.pending);
        }
        let c = median(cand);
        let v = median(cert);
        println!(
            "{n},TWO_WINDOW_RESEARCH,3,{c:.8},{v:.8},{:.6},{},{},{},{}",
            v / c,
            last.0,
            last.1,
            last.2,
            last.3
        );
    }
    Ok(())
}
