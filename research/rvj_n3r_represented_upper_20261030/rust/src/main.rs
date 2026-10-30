use rvj_n3_window_source_locked::{Session, Source};
fn main() {
    println!("n,window,mode,re,im,ideal_bound,attempts,accepted,rejected,epoch");
    for n in [12, 24, 65, 257] {
        let source = Source::toy(n).unwrap();
        let mut s = Session::new(source).unwrap();
        for w in 1..=2 {
            let proposal = s.propose(0.25, 2).unwrap();
            assert!(proposal.projected_ideal_bound() < 5e-8);
            println!(
                "# ideal analytical max bound n={n} w={w} b={:0.17e} q={:0.12e} (NO float rounding bound)",
                proposal.projected_ideal_bound(),
                proposal.q()
            );
            s.decide(proposal, 5e-8).unwrap();
            let work = s.work();
            for (j, z) in s.state().iter().enumerate() {
                println!(
                    "{n},{w},{j},{:0.17e},{:0.17e},{:0.17e},{},{},{},{}",
                    z.re,
                    z.im,
                    s.ideal_bounds()[j],
                    work.attempted,
                    work.accepted,
                    work.rejected,
                    s.epoch()
                );
            }
        }
    }
}
