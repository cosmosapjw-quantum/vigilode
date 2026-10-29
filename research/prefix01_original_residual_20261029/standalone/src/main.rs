use std::hint::black_box;
use std::time::Instant;
use vigilode_prefix01::{BoundModel, SolveConfig, TriggerMode, solve_into};
fn fixture(n: usize) -> (BoundModel, Vec<f64>, Vec<f64>) {
    let mut j = vec![0.0; n * n];
    for i in 0..n {
        j[i * n + i] = -2.0 - (i % 16) as f64 / 8.0;
    }
    let m = BoundModel {
        n,
        jacobian: j,
        weights: vec![1.0; n],
        sigma: 0.25,
        model_id: "diagonal-rate-family".into(),
        source_id: "synthetic-rate-box-v1".into(),
        clock_id: "declared-source-clock".into(),
        norm_id: "weighted-infinity".into(),
        epoch: 0,
    };
    let b = (0..n).map(|i| 1.0 + 0.25 * (i % 3) as f64).collect();
    (m, b, vec![0.0; n])
}
fn run(
    m: &BoundModel,
    b: &[f64],
    x0: &[f64],
    prefix: bool,
    repeats: usize,
) -> (u128, usize, usize, f64) {
    let mut work = 0;
    let mut checks = 0;
    let mut maxerr = 0.0f64;
    let now = Instant::now();
    for _ in 0..repeats {
        let mut output = vec![0.0; m.n];
        let mode = if prefix {
            TriggerMode::Prefix {
                projected_tau: 1e-7,
                ratio: 1.5,
            }
        } else {
            TriggerMode::EndOnly
        };
        let r = solve_into(
            m,
            b,
            x0,
            &mut output,
            SolveConfig {
                max_columns: 16,
                tau_inf: 1e-7,
                stage_budget: 1e-7,
                mode,
            },
        )
        .unwrap();
        assert!(r.accepted);
        work += r.work.arnoldi_columns;
        checks += r.work.candidate_checks;
        for i in 0..m.n {
            let exact = b[i] / (1.0 - m.sigma * m.jacobian[i * m.n + i]);
            maxerr = maxerr.max((output[i] - exact).abs())
        }
        black_box(output);
    }
    (now.elapsed().as_nanos(), work, checks, maxerr)
}
fn main() {
    println!(
        "n,block,order,prefix_ns,full_ns,ratio_full_over_prefix,prefix_columns,full_columns,prefix_checks,full_checks,max_error"
    );
    for &(n, repeats) in &[(16, 100), (64, 40), (128, 15)] {
        let (m, b, x0) = fixture(n);
        for _ in 0..5 {
            let _ = run(&m, &b, &x0, true, 1);
            let _ = run(&m, &b, &x0, false, 1);
        }
        for block in 0..7 {
            let pfirst = block % 2 == 0;
            let (p, f) = if pfirst {
                let p = run(&m, &b, &x0, true, repeats);
                let f = run(&m, &b, &x0, false, repeats);
                (p, f)
            } else {
                let f = run(&m, &b, &x0, false, repeats);
                let p = run(&m, &b, &x0, true, repeats);
                (p, f)
            };
            println!(
                "{n},{block},{},{},{},{:.8},{},{},{},{},{:.17e}",
                if pfirst { "PF" } else { "FP" },
                p.0,
                f.0,
                f.0 as f64 / p.0 as f64,
                p.1 / repeats,
                f.1 / repeats,
                p.2 / repeats,
                f.2 / repeats,
                p.3.max(f.3)
            );
        }
    }
}
