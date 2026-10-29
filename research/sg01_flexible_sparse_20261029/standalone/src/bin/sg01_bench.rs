use std::hint::black_box;
use std::time::Instant;
use vigilode_sg01::{Frozen, Options, SparseFrozen, Work, certify, solve_into, solve_into_sparse};

fn main() {
    println!(
        "kind,block,n,backend,mode,elapsed_ns,columns,candidate_builds,original_checks,operator_apps,pc_apps,achieved_inf,stage_bound"
    );
    for &n in &[16_usize, 64, 128] {
        let mut j = vec![0.0; n * n];
        let mut pcdiag = vec![0.0; n];
        for i in 0..n {
            let rate = 0.2 + 9.0 * i as f64 / n as f64;
            j[i * n + i] = -rate;
            let approx = 0.2 + 9.0 * (i / 8 * 8) as f64 / n as f64;
            pcdiag[i] = 1.0 / (1.0 + 0.25 * approx);
        }
        let rhs = (0..n)
            .map(|i| 1.0 + 0.1 * (i as f64 * 0.3).sin())
            .collect::<Vec<f64>>();
        let w = vec![1.0; n];
        let p = Frozen {
            n,
            j: &j,
            sigma: 0.25,
            rhs: &rhs,
            w: &w,
            epoch: 7,
        };
        let cache = SparseFrozen::new(&p).unwrap();
        let options = Options {
            max_columns: n.min(20),
            restart: n.min(20),
            projected_trigger: 1e-9,
            original_budget: 1e-8,
            stage_budget: 1e-8,
            every_prefix: false,
        };
        let execute = |every_prefix: bool, backend: &str| -> (u128, Work, Vec<f64>, f64, f64) {
            let mut out = vec![0.0; n];
            let mut work = Work::default();
            let mut opt = options;
            opt.every_prefix = every_prefix;
            let start = Instant::now();
            let pc = |v: &[f64], z: &mut [f64]| {
                for i in 0..n {
                    z[i] = v[i] * pcdiag[i];
                }
                Ok(())
            };
            let result = if backend == "sparse" {
                solve_into_sparse(&cache, &vec![0.0; n], &mut out, opt, pc, || 7, &mut work)
            } else {
                solve_into(&p, &vec![0.0; n], &mut out, opt, pc, || 7, &mut work)
            };
            let elapsed = start.elapsed().as_nanos();
            let c = result.unwrap_or_else(|e| panic!("n={n} every={every_prefix}: {e:?} {work:?}"));
            let max_error = out
                .iter()
                .enumerate()
                .map(|(i, &x)| (x - rhs[i] / (1.0 + 0.25 * (-j[i * n + i]))).abs())
                .fold(0.0_f64, f64::max);
            let independent = certify(&p, &out, 1e-8, 1e-8).unwrap().unwrap();
            assert!(independent.rho_upper <= 1e-8 && independent.stage_error_upper <= 1e-8);
            (elapsed, work, out, max_error, c.stage_error_upper)
        };
        // Warm both, measured ABBA/BAAB; running the same binary/clock/CPU.
        for _ in 0..10 {
            black_box(execute(false, "dense"));
            black_box(execute(false, "sparse"));
            black_box(execute(true, "dense"));
            black_box(execute(true, "sparse"));
        }
        for backend in ["dense", "sparse"] {
            for block in 0..7 {
                let seq = if block % 2 == 0 {
                    [false, true, true, false]
                } else {
                    [true, false, false, true]
                };
                for &every in &seq {
                    for _ in 0..6 {
                        let (ns, work, out, err, bound) = execute(every, backend);
                        black_box(out);
                        println!(
                            "timed,{block},{n},{backend},{},{ns},{},{},{},{},{},{:.17e},{:.17e}",
                            if every { "always" } else { "trigger" },
                            work.columns,
                            work.candidate_builds,
                            work.original_checks,
                            work.operator_apps,
                            work.pc_apps,
                            err,
                            bound
                        );
                    }
                }
            }
        }
    }
}
