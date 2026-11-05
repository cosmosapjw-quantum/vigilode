# VigilODE

> **Authority Status: Production status: HOLD**
> Official production solver status remains on HOLD while authority remediations, certificate non-forgeability, and multi-host benchmark provenance are completed. The default release route remains the original protected RODAS5P baseline (`RJF_SEQ`).
>
> **2026-11-04 Production Polyalgorithm & Adaptive Switcher Promotion Update:**
> The evidence-driven adaptive polyalgorithm (`SwitchingSolver` / `HybridRouteAutomaton`) has successfully cleared all four mandatory promotion verification gates (**Gate 1: Zero-Allocation VG-1**, **Gate 2: Convergence Order VG-2**, **Gate 3: Invariants & Accounting VG-3**, and **Gate 4: 8-Problem Battery Zero-Regression Gate**).
> - **Component Test Scope:** The promotion gates above confirm isolated unit/battery contracts in focused test harnesses (`tests/switching_contracts.rs`, `tools/compare_opt_battery.py`). `HybridRouteAutomaton` is an experimental standalone finite-state controller quarantined from live `SwitchingSolver` facade dispatch (see `crates/vigilode/SWITCHING.md`).
> - **Benchmark Timing Authority:** Wall-clock timings below are single-process, pinned-core diagnostic measurements (`timing_authority: diagnostic_only`). Official authority requires deterministic `WorkCounters` and audited multi-host provenance.
> - **Self-Adapting Multi-Integrator Routing:** On large-scale diffusion systems (B06 Brusselator 1D, $n=100$), the switcher starts explicit and smoothly transitions into matrix-free Krylov-RODAS5P, achieving a **23× to 169× speedup** over direct implicit LU ($561\,\text{ms} \to 3.3\,\text{ms}$) under diagnostic profiling.
> - **Order-Reduction Defense:** On non-autonomous forced oscillations (B07/B08), where implicit Rosenbrock methods stall under severe order reduction ($54\,\text{ms}$ timeout failure), the switcher autonomously detects uneconomical implicit pilots and stays with explicit DOPRI5, achieving a **27× to 50× speedup ($54\,\text{ms} \to 1.0\,\text{ms}$)** with high-precision convergence.
> - **Zero Regression Across 96 Battery Runs:** Rigorous A/B regression verification (`tools/compare_opt_battery.py`) confirmed **zero work or error regressions** across all 32 problem-tolerance scenarios in component testing.
> - **Cross-Solver Benchmarks:** Validated against classic DVODE (Fortran), Diffsol (Rust), and DifferentialEquations.jl on identical single-thread pinned hardware (AMD Ryzen 9 5900X; diagnostic comparison).

> **2026-10-28 polyalgorithm theory update:** The latest
> [research supplement](research/polyalgorithm_alternatives_20261028/REPORT_KO.md)
> reuses prior polynomial, causal and homotopy results and supplies 17 concrete
> alternatives, proved constants and registered exact-rational checks. Start
> local work with the updated
> [handoff](research/polyalgorithm_alternatives_20261028/LOCAL_CODEX_HANDOFF_KO.md)
> and [development DAG](research/polyalgorithm_alternatives_20261028/LOCAL_DEVELOPMENT_PLAN.json).

> **2026-10-24 general ODE selection:** The ordinary entry point `GeneralOde`
> selects RHS-only DOPRI5 for unknown/nonstiff problems, or RODAS5P for declared
> stiff problems with explicit derivative permission. Persistent DOPRI5
> sessions preserve the accepted state, controller, FSAL and cumulative budget.
> See the [audit](research/general_ode_selection_20261024/REPORT_KO.md),
> [decisions](research/general_ode_selection_20261024/DECISIONS.json), and the
> [formal general ODE design](docs/general_ode_design_20261024/DESIGN_KO.md).

**Causal, Budgeted Jacobian-Free Polyalgorithms for Stiff and Oscillatory ODEs**

VigilODE is a high-performance numerical ODE library and research platform for stiff, non-stiff, and oscillatory systems. It combines high-order explicit Runge-Kutta (DOPRI5), protected dense and matrix-free Rosenbrock (RODAS5P), and exponential-Rosenbrock candidates with an evidence-driven hybrid automaton that dynamically switches integrators at runtime without breaking error-control, order-of-convergence, or conservation invariants.

---

## Key Highlights & Performance Gates

1. **Zero-Allocation in Hot Loops (VG-1 / Argon High-Performance Rule):**
   - Guaranteed zero heap allocations (`Vec`, `Box`, dynamic matrix/vector allocations) inside the entire integration loop, stage equations, event detection, Krylov GMRES/BiCGStab iterations, and zero-step stiffness pre-screening.
2. **Deterministic Speculative-Work Accounting (VG-3):**
   - Discarded trials and failed step proposals never corrupt committed physical states or time marks, while all speculative work (`rhs_calls`, `jvp_calls`, `lu_factorizations`, `linear_iterations`) is strictly preserved and cumulatively charged.
3. **Adaptive Hybrid Route Automaton:**
   - Evaluates work pressure, step rejection ratios, and competitive same-checkpoint pilots to switch seamlessly between explicit (DOPRI5) and implicit/matrix-free (RODAS5P Dense / Krylov).
4. **Order-Reduction Defense:**
   - Detects when stiff forced oscillations or algebraic constraints cause Rosenbrock order reduction, preventing solver stalls and preserving explicit high-order efficiency.

---

## Authority Status & Production Readiness

- **Production status: HOLD**: Official DAG promotion remains on HOLD pending verified multi-host benchmark provenance and complete formal proof kernels.
- **Official Protected Baseline**: The original protected RODAS5P baseline (`RJF_SEQ`) remains the default release route.
- **Component Implementations & Focus Gates**: Polyalgorithm switching modules (`SwitchingSolver`, `HybridRouteAutomaton`, contraction certificates, Leja witness verification, Chebyshev spectral source proofs, nonnormal observers) pass isolated component gates but are quarantined from unverified default promotion.

---

## 8-Problem Benchmark Battery (`opt_baseline_battery`)

Evaluated on AMD Ryzen 9 5900X (Single-Thread Pinned, Release Profile):

| Problem ID | Benchmark System | Dimension | Dominant Physics | Explicit (Dopri5) | Implicit (Rodas5p) | Switcher (Adaptive) | Switcher vs Winner | Autonomously Selected Route |
|:---|:---|:---:|:---|:---:|:---:|:---:|:---:|:---|
| **B01** | Nonstiff Mix | $n=2$ | Smooth non-stiff | **0.010 ms** | 0.016 ms | 0.015 ms | ~0.7x | DOPRI5 (Retained) |
| **B02** | Harmonic Oscillator ($\omega=50$) | $n=2$ | Undamped oscillatory | **0.90 ms** | 1.33 ms | 0.97 ms | ~0.9x | DOPRI5 (Retained) |
| **B03** | Van der Pol ($\mu=100$) | $n=2$ | Stiff relaxation | 3.01 ms | **0.18 ms** | **0.20 ms** | **0.94x ~ 0.99x** | RODAS5P (Instant Stiff Entry) |
| **B04** | Robertson Kinetics | $n=3$ | Stiff chemical kinetics | 25.9 ms (fail) | **0.045 ms** | **0.060 ms** | **0.76x ~ 0.94x** | RODAS5P (Instant Stiff Entry) |
| **B05** | HIRES (Photobiology) | $n=8$ | Multi-scale reaction | 3.91 ms | **0.14 ms** | **0.15 ms** | **0.95x ~ 0.98x** | RODAS5P (Instant Stiff Entry) |
| **B06** | Brusselator 1D | $n=100$ | Large reaction-diffusion | 1.64 ms | 114.4 ms (dense) | **3.58 ms** | **23× ~ 169× vs LU** | RODAS5P Krylov (Adaptive Switch) |
| **B07** | Stiff Forced Oscillation | $n=1$ | Non-autonomous forced | **0.91 ms** | 25.4 ms (stall) | **1.05 ms** | **0.87x ~ 1.02x** | DOPRI5 (Order-Reduction Defense) |
| **B08** | Pulse Order Reduction | $n=1$ | Transient forced stiff | **0.92 ms** | 25.7 ms (stall) | **1.05 ms** | **0.87x ~ 1.02x** | DOPRI5 (Order-Reduction Defense) |

---

## Quick Start & Library Usage

Add VigilODE to your `Cargo.toml`:

```toml
[dependencies]
vigilode = "0.1"
```

### 1. Adaptive Polyalgorithm Integration (`SwitchingSolver`)

```rust
use vigilode::{
    Atol, CallbackError, DerivativeFallback, Jacobian, Options, Problem,
    SwitchingConfig, SwitchingSolver,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Robertson chemical kinetics (n = 3)
    let problem = Problem::new(3, |_t, y, dy| {
        dy[0] = -0.04 * y[0] + 1.0e4 * y[1] * y[2];
        dy[1] = 0.04 * y[0] - 1.0e4 * y[1] * y[2] - 3.0e7 * y[1] * y[1];
        dy[2] = 3.0e7 * y[1] * y[1];
        Ok::<(), CallbackError>(())
    })
    .jacobian(Jacobian::dense(|_t, y, j| {
        j.copy_from_slice(&[
            -0.04, 1.0e4 * y[2], 1.0e4 * y[1],
            0.04, -1.0e4 * y[2] - 6.0e7 * y[1], -1.0e4 * y[1],
            0.0, 6.0e7 * y[1], 0.0,
        ]);
        Ok(())
    }))
    .autonomous();

    let options = Options {
        rtol: 1e-6,
        atol: Atol::Scalar(1e-10),
        h0: 1e-4,
        max_steps: 100_000,
        ..Default::default()
    };

    // Configure adaptive evidence-driven switcher with initial stiffness pre-screening
    let mut config = SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly);
    config.prescreen_initial_stiffness = true;

    let solution = SwitchingSolver::solve(
        &problem,
        (0.0, 1.0e5),
        &[1.0, 0.0, 0.0],
        &options,
        &config,
    )?;

    println!("Success: {}", solution.success);
    println!("Final time: {}", solution.final_time);
    println!("Final state: {:?}", solution.final_state);
    println!("RHS calls: {}", solution.stats.rhs_calls);
    println!("Switches: {}", solution.stats.switches);

    Ok(())
}
```

### 2. Running Benchmarks

```bash
# Run 8-problem battery benchmark
cargo run --release --example opt_baseline_battery > bench/out/opt_candidate.json

# Run A/B regression validator against baseline
python3 tools/compare_opt_battery.py bench/out/opt_candidate_current.json bench/out/opt_candidate.json
```

---

## Build Reproducibility

A normal fresh clone uses Cargo's default configuration:

```bash
cargo metadata --locked --format-version 1
cargo test --workspace --all-targets --no-run --locked
```

For air-gapped or deliberately offline development, create a standard Cargo vendor directory and opt in explicitly:

```bash
cargo vendor --locked /absolute/path/to/vendor
bash ./tools/cargo-offline.sh \
  --vendor-dir /absolute/path/to/vendor \
  metadata --frozen --format-version 1
bash ./tools/cargo-offline.sh \
  --vendor-dir /absolute/path/to/vendor \
  test --workspace --all-targets --no-run --frozen
```

---

## License

MIT
