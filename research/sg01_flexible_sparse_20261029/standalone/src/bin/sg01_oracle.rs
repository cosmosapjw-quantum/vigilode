use vigilode_sg01::{Frozen, SparseFrozen, certify};
fn next(s: &mut u64) -> u64 {
    *s = s
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *s
}
fn r(s: &mut u64, amp: f64) -> f64 {
    ((next(s) >> 32) % 4000) as f64 / 3999.0 * amp
}
fn bits(v: &[f64]) -> String {
    v.iter()
        .map(|x| format!("{:016x}", x.to_bits()))
        .collect::<Vec<_>>()
        .join(",")
}
fn main() {
    let mut seed = 0x726f646173357031_u64;
    for t in 0..320 {
        let n = 1 + t % 6;
        let sigma = 0.01 + r(&mut seed, 0.1);
        let mut j = vec![0.0; n * n];
        for i in 0..n {
            for k in 0..n {
                j[i * n + k] = if i == k {
                    -2.0 - r(&mut seed, 3.0)
                } else {
                    r(&mut seed, 0.25) - 0.125
                };
            }
        }
        let b = (0..n).map(|_| r(&mut seed, 3.0) - 1.5).collect::<Vec<_>>();
        let w = (0..n).map(|_| 0.5 + r(&mut seed, 2.0)).collect::<Vec<_>>();
        let x = (0..n).map(|_| r(&mut seed, 3.0) - 1.5).collect::<Vec<_>>();
        let p = Frozen {
            n,
            j: &j,
            sigma,
            rhs: &b,
            w: &w,
            epoch: 1,
        };
        let c = certify(&p, &x, 1e5, 1e5).unwrap().unwrap();
        let sp = SparseFrozen::new(&p)
            .unwrap()
            .certify(&x, 1e5, 1e5)
            .unwrap()
            .unwrap();
        println!(
            "{n}|{:016x}|{}|{}|{}|{}|{:016x},{:016x},{:016x},{:016x}|{:016x},{:016x},{:016x},{:016x}",
            sigma.to_bits(),
            bits(&j),
            bits(&b),
            bits(&w),
            bits(&x),
            c.rho_upper.to_bits(),
            c.mu_upper.to_bits(),
            c.inverse_upper.to_bits(),
            c.stage_error_upper.to_bits(),
            sp.rho_upper.to_bits(),
            sp.mu_upper.to_bits(),
            sp.inverse_upper.to_bits(),
            sp.stage_error_upper.to_bits()
        );
    }
}
