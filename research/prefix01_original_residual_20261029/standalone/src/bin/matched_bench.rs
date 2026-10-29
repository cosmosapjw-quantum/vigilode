//! Matched-output benchmark: both modes stop at the same certified prefix.
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
        model_id: "matched".into(),
        source_id: "synthetic".into(),
        clock_id: "declared".into(),
        norm_id: "w-inf".into(),
        epoch: 0,
    };
    let b = (0..n)
        .map(|i| 1.0 + 0.25 * (i % 3) as f64)
        .collect::<Vec<_>>();
    (m, b, vec![0.0; n])
}
fn run(
    m: &BoundModel,
    b: &[f64],
    x0: &[f64],
    selective: bool,
    repeats: usize,
) -> (u128, usize, usize, Vec<u64>) {
    let mode = if selective {
        TriggerMode::Prefix {
            projected_tau: 1e-7,
            ratio: 1.5,
        }
    } else {
        TriggerMode::Always
    };
    let start = Instant::now();
    let mut checks = 0;
    let mut columns = 0;
    let mut bits = vec![];
    for _ in 0..repeats {
        let mut out = vec![0.0; m.n];
        let r = solve_into(
            m,
            b,
            x0,
            &mut out,
            SolveConfig {
                max_columns: 16,
                tau_inf: 1e-7,
                stage_budget: 1e-7,
                mode,
            },
        )
        .unwrap();
        assert!(r.accepted);
        checks = r.work.candidate_checks;
        columns = r.work.arnoldi_columns;
        bits = out.iter().map(|x| x.to_bits()).collect();
        black_box(out);
    }
    (start.elapsed().as_nanos(), checks, columns, bits)
}
fn main() {
    println!(
        "n,block,order,selective_ns,always_ns,ratio_always_over_selective,selective_checks,always_checks,columns,bit_identical"
    );
    for &(n, repeats) in &[(16, 100), (64, 40), (128, 15)] {
        let (m, b, x0) = fixture(n);
        for _ in 0..5 {
            let _ = run(&m, &b, &x0, true, 1);
            let _ = run(&m, &b, &x0, false, 1);
        }
        for block in 0..7 {
            let (s, a) = if block % 2 == 0 {
                (
                    run(&m, &b, &x0, true, repeats),
                    run(&m, &b, &x0, false, repeats),
                )
            } else {
                let a = run(&m, &b, &x0, false, repeats);
                let s = run(&m, &b, &x0, true, repeats);
                (s, a)
            };
            assert_eq!(s.3, a.3);
            assert_eq!(s.2, a.2);
            println!(
                "{n},{block},{},{},{},{:.8},{},{},{},{}",
                if block % 2 == 0 { "SA" } else { "AS" },
                s.0,
                a.0,
                a.0 as f64 / s.0 as f64,
                s.1,
                a.1,
                s.2,
                s.3 == a.3
            );
        }
    }
}
