//! Paired complete certified solves, all interval checks and allocations timed.
use serde_json::json;
use std::hint::black_box;
use std::time::Instant;
use vigilode_coupled::certified_fastfirst::{CertifiedConfig, FrozenDiagonalAffine};

fn model(n: usize, mixed: bool) -> FrozenDiagonalAffine {
    let a = (0..n)
        .map(|i| if mixed && i % 2 == 1 { -0.8 } else { 0.99 })
        .collect();
    let b = (0..n)
        .map(|i| if mixed && i % 2 == 1 { 0.2 } else { 0.01 })
        .collect();
    FrozenDiagonalAffine::new(a, b).unwrap()
}

fn measured(source: &FrozenDiagonalAffine, fast_first: bool, reps: usize) -> (u128, u64, u64, u64) {
    let cfg = CertifiedConfig {
        fast_first,
        root_error_target: 1e-10,
        ..CertifiedConfig::default()
    };
    let anchor = vec![0.0; source.dimension()];
    let start = Instant::now();
    let (mut total_calls, mut intervals, mut checksum) = (0, 0, 0_u64);
    for _ in 0..reps {
        let result = black_box(source.solve(black_box(&anchor), black_box(&cfg)));
        assert!(result.failure.is_none(), "{:?}", result.failure);
        assert!(result.points[0].exact_root_error_upper <= cfg.root_error_target);
        total_calls += result.work.map_attempted;
        intervals += result.work.exact_residual_interval_evaluations;
        checksum = checksum.wrapping_add(result.root.unwrap()[0].to_bits());
    }
    (start.elapsed().as_nanos(), total_calls, intervals, checksum)
}

fn main() {
    // Independent Fraction oracle can replay the exact mathematical affine
    // fixed points against every reported binary64 state/error-bound.
    for &(n, mixed) in &[(1, false), (3, true), (12, true), (256, true)] {
        let owned = model(n, mixed);
        let conf = CertifiedConfig::default();
        let out = owned.solve(&vec![0.0; n], &conf);
        assert!(out.failure.is_none(), "{:?}", out.failure);
        println!(
            "CFF_ORACLE={}",
            json!({
                "n": n,
                "source_bits": owned.represented_source_bits(),
                "root_bits": out.root.unwrap().iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
                "certified_upper_bits": out.points[0].exact_root_error_upper.to_bits(),
                "exact_residual_lower_bits": out.points[0].exact_residual_l2_lower.to_bits(),
                "exact_residual_upper_bits": out.points[0].exact_residual_l2_upper.to_bits(),
                "root_error_target_bits": conf.root_error_target.to_bits(),
                "source_q_bits": owned.represented_contraction().to_bits(),
                "calls": out.work.map_attempted
            })
        );
    }
    for &(n, mixed) in &[(1, false), (256, false), (256, true)] {
        let owned = model(n, mixed);
        for fast in [false, true] {
            black_box(measured(&owned, fast, 15));
        }
        for block in 0..5 {
            let order = if block % 2 == 0 {
                [false, true, true, false]
            } else {
                [true, false, false, true]
            };
            for fast in order {
                let (ns, calls, intervals, checksum) = measured(&owned, fast, 200);
                println!(
                    "CFF_BENCH n={n} mixed={mixed} block={block} fast={fast} ns={ns} calls={calls} intervals={intervals} checksum={checksum}"
                );
            }
        }
    }
}
