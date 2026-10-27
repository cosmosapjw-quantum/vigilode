//! Reuse the existing measured-evidence switcher with a supplied JVP only.
//! Every pilot is included; no finite differences or matrix assembly available.
use serde_json::json;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use vigilode::{
    Atol, DerivativeFallback, Jacobian, Options, Problem, SwitchingConfig, SwitchingSolver,
};

fn main() {
    let mut rows = Vec::new();
    for rate in [1.0_f64, 1000.0, 10000.0] {
        for (mode, selective, checkpoint_seed, strict_entry_work_improvement) in [
            ("derivative-free", false, false, false),
            ("selective-unseeded", true, false, false),
            ("selective-checkpoint-seeded", true, true, false),
            ("selective-seeded-strict-work", true, true, true),
        ] {
            let rhs = Arc::new(AtomicU64::new(0));
            let jvp = Arc::new(AtomicU64::new(0));
            let rhs_seen = rhs.clone();
            let jvp_seen = jvp.clone();
            let p = Problem::new(16, move |_, y, out| {
                rhs_seen.fetch_add(1, Ordering::SeqCst);
                for (o, y) in out.iter_mut().zip(y) {
                    *o = -rate * y;
                }
                Ok(())
            })
            .jacobian(Jacobian::jvp(move |_, _, v, out| {
                jvp_seen.fetch_add(1, Ordering::SeqCst);
                for (o, v) in out.iter_mut().zip(v) {
                    *o = -rate * v;
                }
                Ok(())
            }))
            .autonomous();
            let config = if selective {
                SwitchingConfig {
                    window_count: 8,
                    stop_dominated_return_comparator: true,
                    seed_entry_pilot_from_explicit_checkpoint: checkpoint_seed,
                    strict_entry_work_improvement,
                    ..SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly)
                }
            } else {
                SwitchingConfig {
                    window_count: 8,
                    ..Default::default()
                }
            };
            let options = Options {
                rtol: 1e-7,
                atol: Atol::Scalar(1e-10),
                h0: 1e-4,
                max_steps: 50000,
                ..Default::default()
            };
            let out =
                SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0; 16], &options, &config).unwrap();
            let error = out
                .t
                .iter()
                .zip(&out.y)
                .flat_map(|(t, y)| y.iter().map(move |x| (x - (-rate * t).exp()).abs()))
                .fold(0.0_f64, f64::max);
            let work = out.stats.segments.iter().map(|s| s.work_units).sum::<f64>();
            let qualified = out.success && out.final_time == 1.0 && error <= 1e-6;
            assert!(
                qualified,
                "frozen all-state accuracy cap: rate {rate}, selective {selective}, error {error}"
            );
            assert_eq!(rhs.load(Ordering::SeqCst), out.stats.rhs_calls);
            assert_eq!(jvp.load(Ordering::SeqCst), out.stats.jvp_vectors);
            assert_eq!(out.stats.jacobian_builds, 0);
            assert_eq!(out.stats.ft_calls, 0);
            if !selective {
                assert_eq!(out.stats.jvp_vectors, 0);
            }
            rows.push(json!({"rate":rate,"mode":mode,"selective":selective,"checkpoint_seed":checkpoint_seed,"entry_cost_ratio":config.entry_cost_ratio,"strict_entry_work_improvement":strict_entry_work_improvement,"success":out.success,"accuracy_pass":qualified,
                "maximum_returned_state_error":error,"accuracy_cap":1e-6,"work_units":work,"rhs":out.stats.rhs_calls,
                "jvp":out.stats.jvp_vectors,"attempts":out.stats.attempts,"jacobian_builds":out.stats.jacobian_builds,
                "ft_calls":out.stats.ft_calls,"complete_accounting":out.stats.accounting_complete,"segments":out.stats.segments,
                "times":out.t,"states":out.y}));
        }
    }
    println!("{}",serde_json::to_string_pretty(&json!({"dimension":16,"span":[0.0,1.0],
        "equation":"y'=-rate*y; all components y(0)=1", "source":"existing adaptive_evidence ProvidedOnly policy + PR90 dominance stop + opt-in observed checkpoint seed + exact integer strict entry comparison",
        "derivative_scope":"supplied JVP only after explicit-work pressure; no AD, FD, full matrix or Newton",
        "cost_scope":"all completed/discarded probes and native work; deterministic proxy, not elapsed time","rows":rows})).unwrap());
}
