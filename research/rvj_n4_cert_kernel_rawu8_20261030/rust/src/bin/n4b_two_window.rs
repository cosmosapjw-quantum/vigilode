//! N4B source-locked two-window conditional floating carrier enclosure.
use rvj_n4_source_kernel::n3h_source_bounded_copy::{Session, Source};
fn main() -> Result<(), String> {
    println!(
        "n,window,j,float_re,float_im,picard_re_lo,picard_re_hi,picard_im_lo,picard_im_hi,float_gap_l1_upper,physical_conditional_l1_upper"
    );
    for n in [12, 24, 65, 257] {
        let mut s = Session::new(Source::toy(n)?)?;
        let a = s.propose(0.25, 2)?;
        let first = s.n4a_certify_first_pending(&a)?;
        let fst = a.n4a_research_float_state().to_vec();
        for j in 0..n {
            let x = &first[j];
            println!(
                "{n},1,{j},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e}",
                fst[j].re,
                fst[j].im,
                x.exact_first_picard.re.lo,
                x.exact_first_picard.re.hi,
                x.exact_first_picard.im.lo,
                x.exact_first_picard.im.hi,
                x.float_gap_l1_upper,
                x.initial_epoch_physical_l1_upper
            );
        }
        let first_receipt = s.n4b_commit_first_with_float_enclosure(a, 1e-8)?;
        let b = s.propose(0.25, 2)?;
        let cert = s.n4b_certify_second_pending(&b, &first_receipt)?;
        let snd = b.n4a_research_float_state().to_vec();
        for j in 0..n {
            let x = &cert[j];
            println!(
                "{n},2,{j},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e}",
                snd[j].re,
                snd[j].im,
                x.exact_first_picard.re.lo,
                x.exact_first_picard.re.hi,
                x.exact_first_picard.im.lo,
                x.exact_first_picard.im.hi,
                x.float_gap_l1_upper,
                x.initial_epoch_physical_l1_upper
            );
        }
        s.n4b_commit_second_with_float_enclosure(b, &first_receipt, 1e-8)?;
        assert_eq!(s.epoch(), 2);
        assert_eq!(s.time(), 0.5);
        assert!(s.is_balanced());
    }
    Ok(())
}
