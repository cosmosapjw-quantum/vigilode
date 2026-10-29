//! This binary proves only the PR112 N03 mathematical interval certificate
//! conditional on the supplied BASS21 represented-rate box. Not a physical
//! source/epoch owner or a RODAS stage. No default GMRES behavior is altered.
use std::{env, fs};
use vigilode_n03_interval::{Interval, SourceStamp, StageIntervalBox};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args()
        .nth(1)
        .ok_or("supply BASS21 source-box input path")?;
    let file = fs::read_to_string(path)?;
    let mut v = file.split_whitespace();
    if v.next() != Some("BASS21-FROZEN-PROPER-TIME-INTERVAL-V1") {
        return Err("source fixture schema".into());
    }
    let n: usize = v.next().ok_or("n missing")?.parse()?;
    if n != 21 {
        return Err("expected full BASS21 owner".into());
    }
    let sigma: f64 = v.next().ok_or("sigma missing")?.parse()?;
    let mut j = Vec::with_capacity(n * n);
    for _ in 0..(n * n) {
        let lo: f64 = v.next().ok_or("J lower missing")?.parse()?;
        let hi: f64 = v.next().ok_or("J upper missing")?.parse()?;
        j.push(Interval::new(lo, hi)?);
    }
    let mut b = Vec::with_capacity(n);
    for _ in 0..n {
        let lo: f64 = v.next().ok_or("b lower missing")?.parse()?;
        let hi: f64 = v.next().ok_or("b upper missing")?.parse()?;
        b.push(Interval::new(lo, hi)?);
    }
    fn read_vec<'a>(
        v: &mut impl Iterator<Item = &'a str>,
        n: usize,
    ) -> Result<Vec<f64>, Box<dyn std::error::Error>> {
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            out.push(v.next().ok_or("vector missing")?.parse()?);
        }
        Ok(out)
    }
    let w = read_vec(&mut v, n)?;
    let p = read_vec(&mut v, n)?;
    let x = read_vec(&mut v, n)?;
    if v.next().is_some() {
        return Err("trailing untrusted data".into());
    }
    let source = StageIntervalBox::new(
        n,
        j,
        b,
        w,
        p,
        sigma,
        SourceStamp {
            model: "BASS21_REPRESENTED_SOURCE_FROZEN".into(),
            source: "BASS_PR234_NUMERIC_COEFFICIENT_PROJECTION".into(),
            clock: "proper_seconds".into(),
            norm: "weighted_infty_to_p".into(),
            epoch: 0,
        },
    )?;
    let out = source.certify(&x, 1.0e-6, 1.0e-6)?;
    println!(
        "{{\"accepted_within_given_box\":{},\"mu_up\":{:.17e},\"inverse_up\":{:.17e},\"rho_up\":{:.17e},\"p_error_up\":{:.17e}}}",
        out.admitted_within_given_box,
        out.signed_lognorm_upper,
        out.weighted_inverse_upper,
        out.original_residual_upper,
        out.physical_error_upper
    );
    Ok(())
}
