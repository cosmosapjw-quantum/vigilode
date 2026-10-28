//! Research node `research/pp03_rhs_compression_20261004` (RVJ DAG node
//! PP03): the shared jet on column-pivoted QR compressed RHS, certified
//! against the original columns. `tools/pp03_rhs_compression_check.py`
//! checks the n = 8 outputs against an exact rational oracle.

#![allow(clippy::should_implement_trait)]

use rodas5p_core::DenseMatrix;
use rodas5p_core::shared_shift_jet::{CertificateStatus, SharedShiftJetConfig, shared_shift_jet};
use rodas5p_core::shared_shift_policy::{compress_rhs, compressed_shift_jet, plan_shift_family};
use serde_json::{Value, json};

pub struct SplitMix(pub u64);

impl SplitMix {
    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    pub fn symmetric(&mut self) -> f64 {
        2.0 * ((self.next() >> 11) as f64 / (1u64 << 53) as f64) - 1.0
    }
    pub fn vector(&mut self, n: usize) -> Vec<f64> {
        (0..n).map(|_| self.symmetric()).collect()
    }
}

/// The PP04 fixture: symmetric-part row bound at most -0.1.
pub fn dissipative(n: usize, seed: u64) -> DenseMatrix {
    let mut rng = SplitMix(seed);
    let s: Vec<f64> = (0..n * n).map(|_| rng.symmetric()).collect();
    let mut j = DenseMatrix::zeros(n, n);
    for i in 0..n {
        let mut row = 0.0;
        for k in 0..n {
            row += (0.5 * (s[i * n + k] + s[k * n + i])).abs();
        }
        for k in 0..n {
            j[(i, k)] = s[i * n + k];
        }
        j[(i, i)] -= row + 0.1;
    }
    j
}

pub const SETS: [&str; 6] = [
    "rank-1",
    "rank-2",
    "full-rank",
    "near-dependent-1e-12",
    "near-dependent-1e-6",
    "plus-minus",
];
pub const H: f64 = 0.1;
pub const TOL: f64 = 1.0e-10;
pub const GAMMA0: f64 = 0.25;

pub fn targets() -> Vec<f64> {
    (0..9)
        .map(|i| GAMMA0 * (1.0 + 0.2 * (-1.0 + 2.0 * i as f64 / 8.0)))
        .collect()
}

fn combination(basis: &[Vec<f64>], coefficients: &[f64]) -> Vec<f64> {
    let mut out = vec![0.0; basis[0].len()];
    for (b, c) in basis.iter().zip(coefficients) {
        for (o, v) in out.iter_mut().zip(b) {
            *o += c * v;
        }
    }
    out
}

pub fn columns(set: &str, n: usize, m: usize, seed: u64) -> Vec<Vec<f64>> {
    let mut rng = SplitMix(seed);
    let q1 = rng.vector(n);
    let q2 = rng.vector(n);
    let low_rank = |rng: &mut SplitMix, rank: usize| -> Vec<Vec<f64>> {
        (0..m)
            .map(|_| {
                let c: Vec<f64> = (0..rank).map(|_| rng.symmetric()).collect();
                combination(&[q1.clone(), q2.clone()][..rank], &c)
            })
            .collect()
    };
    match set {
        "rank-1" => low_rank(&mut rng, 1),
        "rank-2" => low_rank(&mut rng, 2),
        "full-rank" => (0..m).map(|_| rng.vector(n)).collect(),
        "near-dependent-1e-12" | "near-dependent-1e-6" => {
            let eps = if set.ends_with("12") { 1.0e-12 } else { 1.0e-6 };
            let base = low_rank(&mut rng, 2);
            base.into_iter()
                .map(|b| {
                    let p = rng.vector(n);
                    b.iter().zip(&p).map(|(x, e)| x + eps * e).collect()
                })
                .collect()
        }
        "plus-minus" => (0..m)
            .map(|k| {
                let q = if (k / 2) % 2 == 0 { &q1 } else { &q2 };
                let a = if k % 2 == 0 { 3.0 } else { -0.25 };
                q.iter().map(|v| a * v).collect()
            })
            .collect(),
        _ => unreachable!(),
    }
}

fn hx(v: f64) -> String {
    format!("{:016x}", v.to_bits())
}

fn hexes(v: &[f64]) -> Vec<String> {
    v.iter().map(|x| hx(*x)).collect()
}

#[test]
fn compression_keeps_every_column_coefficient() {
    let b = columns("plus-minus", 8, 4, 3);
    let c = compress_rhs(&b).unwrap();
    assert_eq!(c.rank, 2);
    assert_eq!(c.coefficients.len(), 4);
    // Reconstruction Q c_j reproduces b_j to rounding.
    for (bj, cj) in b.iter().zip(&c.coefficients) {
        let x = combination(&c.q_columns, cj);
        let err: f64 = x
            .iter()
            .zip(bj)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max);
        assert!(err < 1e-14, "{err}");
    }
    assert!(compress_rhs(&[]).is_err());
    assert!(compress_rhs(&[vec![f64::NAN]]).is_err());
}

#[test]
fn full_rank_abstains() {
    let j = dissipative(8, 1);
    let b = columns("full-rank", 8, 4, 2);
    assert!(
        compressed_shift_jet(&j, H, GAMMA0, &b, &targets(), 10, TOL)
            .unwrap()
            .is_none()
    );
}

#[test]
#[ignore = "recorded run of research/pp03_rhs_compression_20261004; release build"]
fn export_compression_study() {
    let gammas = targets();
    let mut configs: Vec<Value> = Vec::new();
    for (ni, n) in [8usize, 32].into_iter().enumerate() {
        let j = dissipative(n, 3000 + ni as u64);
        for m in [4usize, 16] {
            for (si, set) in SETS.iter().enumerate() {
                let b = columns(set, n, m, 4000 + 100 * n as u64 + 10 * m as u64 + si as u64);
                let max_norm = b
                    .iter()
                    .map(|c| rodas5p_core::safe_l2(c))
                    .fold(0.0_f64, f64::max);
                let plan = plan_shift_family(n, m, &gammas, max_norm, TOL).unwrap();
                let degree = plan.clusters[0].degree;
                let gamma0 = plan.clusters[0].gamma0;
                let uncompressed = shared_shift_jet(
                    &j,
                    H,
                    gamma0,
                    &b,
                    &gammas,
                    SharedShiftJetConfig {
                        degree,
                        absolute_tolerance: TOL,
                        max_stored_scalars: usize::MAX,
                        max_work_units: usize::MAX,
                    },
                )
                .unwrap();
                let compressed =
                    compressed_shift_jet(&j, H, gamma0, &b, &gammas, degree, TOL).unwrap();
                let rank = compress_rhs(&b).unwrap().rank;
                let unc: Vec<Value> = uncompressed
                    .candidates()
                    .iter()
                    .map(|c| {
                        json!({
                            "certified": c.certificate().status() == CertificateStatus::Certified,
                            "columns": c.rhs_columns().iter().map(|x| hexes(x)).collect::<Vec<_>>(),
                            "error_upper": hexes(c.certificate().rhs_error_upper()),
                        })
                    })
                    .collect();
                let comp: Option<Value> = compressed.as_ref().map(|(compression, result)| {
                    json!({
                        "rank": compression.rank,
                        "work": result.work,
                        "targets": result.targets.iter().map(|t| json!({
                            "certified": t.certified,
                            "columns": t.columns.iter().map(|x| hexes(x)).collect::<Vec<_>>(),
                            "error_upper": hexes(&t.error_upper),
                        })).collect::<Vec<_>>(),
                    })
                });
                let mut config = json!({
                    "n": n, "m": m, "set": set, "degree": degree, "gamma0": hx(gamma0),
                    "rank": rank,
                    "uncompressed_column_solves": uncompressed.work().rhs_solves,
                    "uncompressed": unc,
                    "compressed": comp,
                    "gammas": hexes(&gammas),
                });
                if n == 8 {
                    config["j"] = json!(
                        (0..n)
                            .map(|i| (0..n).map(|k| hx(j[(i, k)])).collect::<Vec<_>>())
                            .collect::<Vec<_>>()
                    );
                    config["h"] = json!(hx(H));
                    config["rhs"] = json!(b.iter().map(|x| hexes(x)).collect::<Vec<_>>());
                }
                configs.push(config);
            }
        }
    }
    let out = json!({"schema": "vigilode-pp03-rhs-compression-v1", "tolerance": hx(TOL), "configs": configs});
    if let Ok(path) = std::env::var("PP03_CASES") {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path);
        assert!(
            !path.exists(),
            "immutable output exists: {}",
            path.display()
        );
        std::fs::write(&path, serde_json::to_string(&out).unwrap() + "\n").unwrap();
        println!("wrote {}", path.display());
    }
}
