//! Contracts for the switcher-v2 remediation tasks (design:
//! docs/agent/switcher_v2_remediation/DESIGN.md). Each task appends its own
//! test functions below the marker for that task. Helpers are shared.
use vigilode::{
    Atol, DerivativeFallback, Jacobian, Options, Problem, SwitchingConfig, SwitchingMethod,
    SwitchingSolution, SwitchingSolver,
};

/// Registered `rotation` model of `examples/evidence_switch_bench.rs`.
fn rotation() -> Problem {
    Problem::new(2, |_, y, o| {
        o[0] = -100.0 * y[1];
        o[1] = 100.0 * y[0];
        Ok(())
    })
    .jacobian(Jacobian::jvp(|_, _, v, o| {
        o[0] = -100.0 * v[1];
        o[1] = 100.0 * v[0];
        Ok(())
    }))
    .autonomous()
}

/// Registered `stiff-forced` model of `examples/evidence_switch_bench.rs`.
fn stiff_forced() -> Problem {
    Problem::new(1, |t, y, o| {
        o[0] = -10000.0 * (y[0] - t.cos()) - t.sin();
        Ok(())
    })
    .jacobian(Jacobian::dense(|_, _, j| {
        j[0] = -10000.0;
        Ok(())
    }))
    .time_derivative(|t, _, o| {
        o[0] = -10000.0 * t.sin() - t.cos();
        Ok(())
    })
}

/// Options of the registered campaign (`evidence_switch_bench`, h0 = 1e-7).
fn bench_options() -> Options {
    Options {
        rtol: 1e-7,
        atol: Atol::Scalar(1e-10),
        h0: 1e-7,
        max_steps: 20000,
        ..Default::default()
    }
}

fn solve(p: &Problem, y0: &[f64], cfg: &SwitchingConfig) -> SwitchingSolution {
    SwitchingSolver::solve(p, (0.0, 1.0), y0, &bench_options(), cfg).unwrap()
}

/// Sum of JVP vectors spent in discarded (uncommitted) RODAS pilots/comparators.
fn discarded_rodas_jvp(sol: &SwitchingSolution) -> u64 {
    sol.stats
        .segments
        .iter()
        .filter(|s| s.method == SwitchingMethod::Rodas5p && s.is_probe && !s.committed)
        .map(|s| s.jvp_vectors)
        .sum()
}

fn provided() -> DerivativeFallback {
    DerivativeFallback::ProvidedOnly
}

// ---- T-F1 ------------------------------------------------------------------

// ---- T-F2 ------------------------------------------------------------------

// ---- T-F3 ------------------------------------------------------------------

// ---- T-F4 ------------------------------------------------------------------

// ---- T-F5 ------------------------------------------------------------------

// ---- T-F6 ------------------------------------------------------------------

// ---- T-F7 ------------------------------------------------------------------

// ---- T-F8 ------------------------------------------------------------------

// ---- T-F9 ------------------------------------------------------------------

#[test]
fn skeleton_helpers_reproduce_registered_counters() {
    // Guards the helpers themselves: the evidence-driven factory must give the
    // registered L-0171 counters on both models (COMPARISON.md, "after" column).
    let rot = solve(
        &rotation(),
        &[1.0, 0.0],
        &SwitchingConfig::evidence_driven(provided()),
    );
    assert!(rot.success);
    assert_eq!((rot.stats.attempts, rot.stats.rhs_calls), (892, 5371));
    assert_eq!(discarded_rodas_jvp(&rot), 355);
    let sf = solve(
        &stiff_forced(),
        &[1.0],
        &SwitchingConfig::evidence_driven(provided()),
    );
    assert!(sf.success);
    assert_eq!((sf.stats.attempts, sf.stats.rhs_calls), (66, 456));
}
