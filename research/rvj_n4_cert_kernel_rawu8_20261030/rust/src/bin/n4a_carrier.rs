//! Source-owned first-window N4A one-Picard ideal interval versus actual f64.
//! NO adaptive state mutation, NO production acceptance.
use rvj_n4_source_kernel::n3h_source_bounded_copy::{Session, Source};
fn main() -> Result<(), String> {
    println!(
        "n,j,source_re,source_im,float_re,float_im,exact_picard_re_lo,exact_picard_re_hi,exact_picard_im_lo,exact_picard_im_hi,float_gap_l1_upper,physical_l1_conditional_upper"
    );
    for n in [12, 24, 65, 257] {
        let mut s = Session::new(Source::toy(n)?)?;
        let old = s.state().to_vec();
        let a = s.propose(0.25, 2)?;
        let rec = s.n4a_certify_first_pending(&a)?;
        for j in 0..n {
            let float = a.n4a_research_float_state()[j];
            let r = &rec[j];
            println!(
                "{},{},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e}",
                n,
                j,
                old[j].re,
                old[j].im,
                float.re,
                float.im,
                r.exact_first_picard.re.lo,
                r.exact_first_picard.re.hi,
                r.exact_first_picard.im.lo,
                r.exact_first_picard.im.hi,
                r.float_gap_l1_upper,
                r.initial_epoch_physical_l1_upper
            );
        }
        assert_eq!(s.state(), old.as_slice());
        s.abort_pending();
        assert!(s.is_balanced());
    }
    Ok(())
}
