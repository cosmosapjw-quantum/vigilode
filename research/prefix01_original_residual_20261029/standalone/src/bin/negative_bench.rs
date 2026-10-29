//! Adversarial: an always-triggered false prefix pays for unsuccessful checks.
use std::hint::black_box;
use std::time::Instant;
use vigilode_prefix01::{BoundModel, SolveConfig, TriggerMode, solve_into};
fn main() {
    let n = 16;
    let mut j = vec![0.0; n * n];
    for i in 0..n {
        j[i * n + i] = -2.0 - i as f64 / 8.0;
    }
    let m = BoundModel {
        n,
        jacobian: j,
        weights: vec![1.0; n],
        sigma: 0.25,
        model_id: "negative".into(),
        source_id: "synthetic".into(),
        clock_id: "x".into(),
        norm_id: "weighted".into(),
        epoch: 0,
    };
    let b = (0..n)
        .map(|i| 1.0 + 0.25 * (i % 3) as f64)
        .collect::<Vec<_>>();
    let z = vec![0.0; n];
    println!(
        "block,order,always_ns,endonly_ns,ratio_end_over_always,always_columns,end_columns,always_checks,end_checks"
    );
    for block in 0..7 {
        let mut times = [0u128; 2];
        let mut work = [(0usize, 0usize); 2];
        let order = if block % 2 == 0 { [0, 1] } else { [1, 0] };
        for &mode in &order {
            let start = Instant::now();
            for _ in 0..100 {
                let mut out = vec![0.0; n];
                let cfg = SolveConfig {
                    max_columns: 16,
                    tau_inf: 1e-12,
                    stage_budget: 1e-12,
                    mode: if mode == 0 {
                        TriggerMode::Always
                    } else {
                        TriggerMode::EndOnly
                    },
                };
                let r = solve_into(&m, &b, &z, &mut out, cfg).unwrap();
                assert!(r.accepted);
                work[mode] = (r.work.arnoldi_columns, r.work.candidate_checks);
                black_box(out);
            }
            times[mode] = start.elapsed().as_nanos();
        }
        println!(
            "{block},{},{},{},{:.8},{},{},{},{}",
            if order[0] == 0 { "AF" } else { "FA" },
            times[0],
            times[1],
            times[1] as f64 / times[0] as f64,
            work[0].0,
            work[1].0,
            work[0].1,
            work[1].1
        );
    }
}
