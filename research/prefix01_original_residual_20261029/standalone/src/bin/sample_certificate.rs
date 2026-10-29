use vigilode_prefix01::{BoundModel, SolveConfig, TriggerMode, solve_into};
fn main() {
    println!(
        "n,mode,mu_upper,inverse_upper,residual_upper,forward_upper,columns,checks,projected_last"
    );
    for n in [16, 64, 128] {
        let mut jacobian = vec![0.0; n * n];
        for i in 0..n {
            jacobian[i * n + i] = -2.0 - (i % 16) as f64 / 8.0;
        }
        let m = BoundModel {
            n,
            jacobian,
            weights: vec![1.0; n],
            sigma: 0.25,
            model_id: "diag-rate".into(),
            source_id: "synthetic-16-rate".into(),
            clock_id: "declared-clock".into(),
            norm_id: "w-inf".into(),
            epoch: 0,
        };
        let b = (0..n)
            .map(|i| 1.0 + 0.25 * (i % 3) as f64)
            .collect::<Vec<_>>();
        let x0 = vec![0.0; n];
        for (name, mode) in [
            (
                "prefix",
                TriggerMode::Prefix {
                    projected_tau: 1e-7,
                    ratio: 1.5,
                },
            ),
            ("endonly", TriggerMode::EndOnly),
        ] {
            let mut out = vec![0.0; n];
            let r = solve_into(
                &m,
                &b,
                &x0,
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
            let c = r.certificate.unwrap();
            println!(
                "{n},{name},{:.17e},{:.17e},{:.17e},{:.17e},{},{},{:.17e}",
                m.mu_upper().unwrap(),
                c.inverse_upper,
                c.residual_upper,
                c.forward_upper,
                r.work.arnoldi_columns,
                r.work.candidate_checks,
                r.projected_last
            );
        }
    }
}
