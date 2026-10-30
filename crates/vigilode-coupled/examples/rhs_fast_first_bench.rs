//! Equal-target, paired complete map-solve benchmark, not certified wall speedup.
use std::cell::Cell;
use std::hint::black_box;
use std::time::Instant;
use vigilode_coupled::rhs_homotopy::{
    RhsAcceleration, RhsHomotopyConfig, RhsOnlyMap, solve_rhs_homotopy,
    solve_rhs_homotopy_contraction_fast_first,
};

struct Linear {
    n: usize,
    calls: Cell<u64>,
}
impl RhsOnlyMap for Linear {
    fn dimension(&self) -> usize {
        self.n
    }
    fn evaluate(&self, x: &[f64], out: &mut [f64]) -> Result<(), String> {
        self.calls.set(self.calls.get() + 1);
        for (v, &xi) in out.iter_mut().zip(x) {
            *v = 0.99 * xi + 0.01;
        }
        Ok(())
    }
}
fn run(n: usize, fast: bool, reps: usize) -> (u128, u64, u64) {
    let map = Linear {
        n,
        calls: Cell::new(0),
    };
    let cfg = RhsHomotopyConfig {
        acceleration: RhsAcceleration::ResidualDominance,
        residual_l2_tolerance: 1e-12,
        ..RhsHomotopyConfig::default()
    };
    let anchor = vec![0.0; n];
    let start = Instant::now();
    let mut checksum = 0u64;
    for _ in 0..reps {
        let result = if fast {
            solve_rhs_homotopy_contraction_fast_first(
                black_box(&map),
                black_box(&anchor),
                &cfg,
                0.99,
            )
        } else {
            solve_rhs_homotopy(black_box(&map), black_box(&anchor), &cfg)
        };
        assert!(result.failure.is_none(), "{:?}", result.failure);
        let root = result.root.unwrap();
        assert!(root.iter().all(|v| (*v - 1.0).abs() < 1e-10));
        assert!(result.points[0].residual_l2 <= cfg.residual_l2_tolerance);
        checksum = checksum.wrapping_add(root[0].to_bits());
        black_box(&root);
    }
    (start.elapsed().as_nanos(), map.calls.get(), checksum)
}
fn main() {
    for n in [1, 256] {
        for fast in [false, true] {
            black_box(run(n, fast, 10));
        }
        for block in 0..5 {
            let arms = if block % 2 == 0 {
                [false, true, true, false]
            } else {
                [true, false, false, true]
            };
            for &fast in &arms {
                let (ns, calls, check) = run(n, fast, 600);
                println!(
                    "FAST_FIRST_BENCH n={} block={} fast={} ns={} calls={} checksum={}",
                    n, block, fast, ns, calls, check
                );
            }
        }
    }
}
