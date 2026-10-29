//! Emit actual Rust binary64 certificate inputs/outputs for an independent
//! Python Fraction.from_float oracle. Fixed seed and deterministic arithmetic.
use vigilode_prefix01::BoundModel;
fn main() {
    println!(
        "case,n,sigma,weights,jacobian,rhs,x,mu_upper,residual_upper,inverse_upper,forward_upper"
    );
    for case in 0..60usize {
        let n = 2 + (case % 4);
        let mut j = vec![0.0; n * n];
        for i in 0..n {
            for k in 0..n {
                j[i * n + k] = if i == k {
                    -2.0 - (i as f64) / 8.0
                } else {
                    (((case + 3 * i + 7 * k) % 7) as f64 - 3.0) / 32.0
                };
            }
        }
        let weights = (0..n)
            .map(|i| 1.0 + ((case + i) % 5) as f64 / 8.0)
            .collect::<Vec<_>>();
        let sigma = 0.125 + ((case % 3) as f64) / 64.0;
        let rhs = (0..n)
            .map(|i| 1.0 + (i % 3) as f64 / 8.0)
            .collect::<Vec<_>>();
        let x = (0..n)
            .map(|i| 0.5 + (i % 3) as f64 / 16.0)
            .collect::<Vec<_>>();
        let m = BoundModel {
            n,
            jacobian: j.clone(),
            weights: weights.clone(),
            sigma,
            model_id: "probe".into(),
            source_id: "synthetic".into(),
            clock_id: "unit".into(),
            norm_id: "weighted-inf".into(),
            epoch: 0,
        };
        let mu = m.mu_upper().unwrap();
        let rho = m.residual_upper(&rhs, &x).unwrap();
        let k = m.inverse_bound().unwrap();
        let c = m.certify(&rhs, &x, 100.0, 100.0).unwrap();
        let array = |v: &[f64]| {
            v.iter()
                .map(|z| z.to_bits().to_string())
                .collect::<Vec<_>>()
                .join(":")
        };
        println!(
            "{case},{n},{},{},{},{},{},{},{},{},{}",
            sigma.to_bits(),
            array(&weights),
            array(&j),
            array(&rhs),
            array(&x),
            mu.to_bits(),
            rho.to_bits(),
            k.to_bits(),
            c.forward_upper.to_bits()
        );
    }
}
