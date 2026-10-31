//! Work and physical-certification paired timings, no production speedup claim.
use serde_json::json;
use std::hint::black_box;
use std::time::Instant;
use vigilode_coupled::nonlinear_be::{Config, SearchOrder, Source};

fn measure(src: &Source, cfg: Config, reps: usize) -> (u128, u64, u64, u64) {
    let start = Instant::now();
    let mut rhs = 0_u64;
    let mut ints = 0_u64;
    let mut checksum = 0_u64;
    for _ in 0..reps {
        let r = black_box(src.run(black_box(cfg)));
        assert!(r.completed, "{:?}", r.failure);
        rhs += r.work.rhs_attempted;
        ints += r.work.interval_residual_evaluations;
        checksum = checksum.wrapping_add(r.points.last().unwrap().state.to_bits());
        black_box(&r);
    }
    (start.elapsed().as_nanos(), rhs, ints, checksum)
}
fn main() {
    for (name, k, g, y, t, n, eps) in [
        ("stiff_fast", 100.0, 1e6, 0.01, 0.1, 64, 0.0015),
        ("relaxation", 10.0, 1e5, 0.01, 0.3, 32, 0.0009),
        ("scale_covariant", 1000.0, 1e9, 0.001, 0.01, 64, 0.00015),
    ] {
        let src = Source::new(k, g, y).unwrap();
        for order in [SearchOrder::BaselineFirst, SearchOrder::FastFirst] {
            let cfg = Config {
                steps: n,
                horizon: t,
                physical_tolerance: eps,
                max_source_evaluations: 100_000,
                max_iterations_per_step: 120,
                order,
            };
            let run = src.run(cfg);
            assert!(run.completed, "{:?}", run.failure);
            println!(
                "NLB_TRACE={}",
                json!({"name":name,"fast":order==SearchOrder::FastFirst,
    "source_bits":src.source_bits(),"h_bits":(t/n as f64).to_bits(),"epsilon_bits":eps.to_bits(),
    "steps":run.points.iter().map(|p|json!([p.time.to_bits(),p.state.to_bits(),p.physical_error_upper.to_bits(),p.original_residual_upper.to_bits(),p.source_q_upper.to_bits()])).collect::<Vec<_>>(),
    "total_rhs":run.work.rhs_attempted,"interval":run.work.interval_residual_evaluations})
            );
        }
        let left = Config {
            steps: n,
            horizon: t,
            physical_tolerance: eps,
            max_source_evaluations: 100_000,
            max_iterations_per_step: 120,
            order: SearchOrder::BaselineFirst,
        };
        let right = Config {
            order: SearchOrder::FastFirst,
            ..left
        };
        black_box(measure(&src, left, 4));
        black_box(measure(&src, right, 4));
        for block in 0..5 {
            let pattern = if block % 2 == 0 {
                [false, true, true, false]
            } else {
                [true, false, false, true]
            };
            for fast in pattern {
                let cfg = if fast { right } else { left };
                let (nanos, rhs, interval, checksum) = measure(&src, cfg, 40);
                println!(
                    "NLB_BENCH name={name} block={block} fast={fast} ns={nanos} rhs={rhs} interval={interval} checksum={checksum}"
                );
            }
        }
    }
}
