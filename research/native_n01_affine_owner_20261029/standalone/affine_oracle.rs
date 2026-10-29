//! Independent Python Fraction-from-bits oracle consumes this file; every
//! numeric cell is a raw binary64 bit pattern printed as decimal u64.
use vigilode_n01_core_source_mirror::{DenseMatrix, n01_affine_owner::N01AffineOwner};
fn next(s: &mut u64) -> f64 {
    *s = s
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (((*s >> 11) as f64) / ((1u64 << 53) as f64)) * 2.0 - 1.0
}
fn emit(x: f64) -> String {
    x.to_bits().to_string()
}
fn main() {
    let fields = [
        "ref", "clock", "sigma", "j00", "j01", "j10", "j11", "k00", "k01", "k10", "k11", "b0",
        "b1", "q0", "q1", "w0", "w1", "p0", "p1", "x0", "x1", "y0", "y1", "delta_j", "delta_b",
        "eta", "mu", "margin", "rho", "error", "accepted",
    ];
    println!("{}", fields.join(","));
    let mut seed = 0x1a83_91e2_7d3f_e3a5_u64;
    for i in 0..192 {
        let j = [
            -1.1 - 0.6 * next(&mut seed).abs(),
            0.15 * next(&mut seed),
            0.12 * next(&mut seed),
            -1.3 - 0.5 * next(&mut seed).abs(),
        ];
        let k = [
            -0.2 * next(&mut seed).abs(),
            0.1 * next(&mut seed),
            0.1 * next(&mut seed),
            -0.15 * next(&mut seed).abs(),
        ];
        let b = [0.5 + 0.3 * next(&mut seed), -0.7 + 0.4 * next(&mut seed)];
        let q = [0.2 * next(&mut seed), 0.1 * next(&mut seed)];
        let w = [1.0 + next(&mut seed).abs(), 1.0 + next(&mut seed).abs()];
        let p = [0.6 + next(&mut seed).abs(), 0.6 + next(&mut seed).abs()];
        let x = [0.3 * next(&mut seed), 0.3 * next(&mut seed)];
        let tref = 0.125;
        let clock = if i % 3 == 0 {
            0.1
        } else if i % 3 == 1 {
            0.2
        } else {
            -0.075
        };
        let sigma = 0.125 + 0.1 * (next(&mut seed).abs());
        let mut owner = N01AffineOwner::new(
            DenseMatrix::from_rows(&[&j[..2], &j[2..]]).unwrap(),
            DenseMatrix::from_rows(&[&k[..2], &k[2..]]).unwrap(),
            b.to_vec(),
            q.to_vec(),
            sigma,
            w.to_vec(),
            p.to_vec(),
            tref,
        )
        .unwrap();
        owner.advance_time(clock).unwrap();
        let mut out = [f64::NAN; 2];
        let rep = owner
            .try_commit(&owner.freeze(), &x, 1.0e6, 1.0e6, &mut out)
            .unwrap();
        assert!(rep.accepted);
        assert_eq!(out, x);
        let y = [
            x[0] - sigma * (j[0] * x[0] + j[1] * x[1]),
            x[1] - sigma * (j[2] * x[0] + j[3] * x[1]),
        ];
        let values = [
            tref,
            clock,
            sigma,
            j[0],
            j[1],
            j[2],
            j[3],
            k[0],
            k[1],
            k[2],
            k[3],
            b[0],
            b[1],
            q[0],
            q[1],
            w[0],
            w[1],
            p[0],
            p[1],
            x[0],
            x[1],
            y[0],
            y[1],
            rep.delta_j_upper,
            rep.delta_b_upper,
            rep.eta_apply_upper,
            rep.mu_upper,
            rep.margin_lower,
            rep.original_residual_upper,
            rep.physical_error_upper,
        ];
        let mut record: Vec<String> = values.iter().copied().map(emit).collect();
        record.push((rep.accepted as u8).to_string());
        println!("{}", record.join(","));
    }
}
