//! Independent probe for source-structured interval lognorm; no production solver.
extern crate chem6_lognorm;
use chem6_lognorm::{Iv, SourceInput, certify_mu};
const SOURCE: &str = "cc602e209e7deeb511ab2a1f3b2268d55206c43c";
const BINDING: &str = "8cb268e8de741e284cf494013f1e0748d27f03c5e77ba0c64413ae92af082d16";
fn fixture() -> SourceInput {
    SourceInput {
        source: SOURCE.into(),
        binding: BINDING.into(),
        clock: "x=ln(a/a_ref)".into(),
        unit: "dimensionless".into(),
        epoch: 1,
        isotope_weights: [0.125; 3],
        norm_weights: [1.0; 6],
        rates: std::array::from_fn(|a| {
            if a < 6 {
                Iv {
                    lo: 1.0 / 16.0,
                    hi: 1.0 / 8.0,
                }
            } else {
                Iv {
                    lo: 1.0 / 32.0,
                    hi: 1.0 / 16.0,
                }
            }
        }),
    }
}
fn main() {
    for id in 0..7 {
        let mut f = fixture();
        match id {
            1 => f.norm_weights = [1.0, 1.25, 1.5, 1.0, 1.25, 1.5],
            2 => f.isotope_weights = [0.25, 0.125, 0.0625],
            3 => {
                for (a, r) in f.rates.iter_mut().enumerate() {
                    if ![0, 3, 7, 11].contains(&a) {
                        r.hi = r.lo;
                    }
                }
            }
            4 => {
                for r in &mut f.rates {
                    r.lo = 0.0;
                    r.hi = 0.0;
                }
            }
            5 => {
                for (a, r) in f.rates.iter_mut().enumerate() {
                    let s = 1.0 / (64.0 + 8.0 * a as f64);
                    r.lo = s;
                    r.hi = s;
                }
            }
            6 => {
                for (a, r) in f.rates.iter_mut().enumerate() {
                    r.lo = (a + 1) as f64 / 1024.0;
                    r.hi = (a + 2) as f64 / 512.0;
                }
            }
            _ => (),
        }
        let b = certify_mu(&f).unwrap();
        println!("{id},{:.17e},{},{},{}", b.upper, b.row, b.vertex, b.signs);
    }
}