# VigilODE

> **Research Status & Production Solver Qualification Notice (2026-11-06 / 2026-11-07):**
> VigilODE is undergoing formal production-solver qualification review (**Status: `HOLD_NOT_RELEASE_QUALIFIED`**).
> - **Live Dispatcher:** Operational adaptive routing during integration is managed by `DriveState` in `switching.rs`. `HybridRouteAutomaton` serves as an offline discrete verification model and formal contract reference (Node P8).
> - **Diagnostic Performance:** All wall-clock benchmarks on AMD Ryzen 9 5900X (pinned `taskset -c 0`) comparing VigilODE, Diffsol (Rust), SUNDIALS CVODE (C), Netlib DVODE (Fortran), Hairer RADAU5/RODAS, and Julia SciML are designated **`timing_authority: diagnostic_only`**.
> - **API & Result Semantics:** `SwitchingSolver::solve` and `ProductionSolver::solve` return `Ok(solution)` where `solution.success` can be `false` on early stops (preserving partial accepted trajectories and complete work accounting). Production callers should use `ProductionSolver::solve_checked` or `solution.ensure_completed()`.
> - **Error Characterization:** Global errors are evaluated against independent high-precision dual references (Hairer RADAU5 vs SUNDIALS CVODE at $rtol=10^{-12}$, dispute threshold $10^{-8}$). Validated continuous trajectory enclosures require verified outward intervals (`directed::Interval`) and remain under active research.

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
3. **Adaptive Route Dispatch & Verification Architecture:**
   - **Live Operational Route Controller (`DriveState` in `SwitchingSolver`):** Evaluates work pressure, step rejection ratios, and competitive same-checkpoint pilots to switch dynamically between explicit (DOPRI5) and implicit/matrix-free (RODAS5P Dense / Krylov) at runtime with zero allocations (`VG-1`).
   - **Discrete Verification Model (`HybridRouteAutomaton`):** Standalone contract reference and formal discrete-event FSM for validating route transitions, anti-chattering dwell/hysteresis, and epoch invalidation.
4. **Order-Reduction Defense:**
   - Detects when stiff forced oscillations or algebraic constraints cause Rosenbrock order reduction, preventing solver stalls and preserving explicit high-order efficiency.

---

## 8-Problem Benchmark Battery (`opt_baseline_battery`, Diagnostic Only)

Evaluated on AMD Ryzen 9 5900X (Single-Thread Pinned, Release Profile, `timing_authority: diagnostic_only`):

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

## Adversarial Spatial Grid & Spherical Coordinate Benchmarks (2026-11-07, Diagnostic Only)

Evaluated on AMD Ryzen 9 5900X (Single-Thread Pinned `taskset -c 0`, `timing_authority: diagnostic_only`).
Dual reference solutions independently verified with Hairer RADAU5 ($rtol=10^{-12}$) and cross-checked against SUNDIALS CVODE ($rtol=10^{-12}$) on a 101-point uniform grid (**0 reference disputes**, max discrepancy $\le 1.46 \times 10^{-10}$ vs $10^{-8}$ threshold).

| Problem ID | Physical System & Geometry | State Dim ($N$) | Bandwidth ($M+1$) | Winner Arm (Banded) | Banded Speedup vs Dense | VigilODE RODAS5P Max WRMS Error | Hot-Loop Allocations |
|:---|:---|:---:|:---:|:---:|:---:|:---:|:---:|
| **Sphere-2D** | Cosmological Shear Photon Transport $(q, \mu)$ | $N=86$ | $M+1=7$ | DVODE MF=24 (0.18 ms) | 1.9x ~ 2.0x | $1.46 \times 10^{-10}$ | **0 (VG-1 Pass)** |
| **Sphere-3D** | Full-Sphere Quadrupole Thomson Scattering $(q, \mu, \phi)$ | $N=302$ | $M+1=25$ | DVODE MF=24 (1.34 ms) | **2.4x ~ 4.9x** | $1.46 \times 10^{-10}$ | **0 (VG-1 Pass)** |
| **Grid-2D** | Semilinear Advection-Diffusion Grid ($8 \times 12$) | $N=96$ | $M+1=8$ | DVODE MF=24 (0.33 ms) | 1.6x ~ 2.9x | $1.34 \times 10^{-10}$ | **0 (VG-1 Pass)** |
| **Grid-3D** | Semilinear Advection-Diffusion Grid ($6 \times 6 \times 6$) | $N=216$ | $M+1=36$ | DVODE MF=24 (1.13 ms) | 2.2x ~ 2.9x | $1.07 \times 10^{-10}$ | **0 (VG-1 Pass)** |

*Key Findings:*
- **Banded Scaling Advantage:** On 3D spherical geometry ($N=302, M+1=25$), banded Jacobian factorization achieves substantial scaling speedups across all solvers (Hairer RADAU5 $4.9\times$, Hairer RODAS $4.0\times$, CVODE $4.0\times$, DVODE $3.3\times$, Diffsol $2.4\times$).
- **Zero-Allocation Compliance (`VG-1`):** VigilODE RODAS5P integration hot loops execute with exactly 0 heap allocations across all spatial grid topologies.
- **High-Order Accuracy:** 5th-order L-stable RODAS5P maintains tight global errors $\le 1.46 \times 10^{-10}$ across all tolerances.
- **Report & Data:** Full 144-row multi-solver results available in `bench/out/ADVERSARIAL_SPATIAL_GRID_REPORT.md` and `bench/out/adversarial_spatial_grid_rust_results.json`.

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

    if !solution.success {
        eprintln!("Integration terminated early: {}", solution.message);
    }
    println!("Success: {}", solution.success);
    println!("Final time: {}", solution.final_time);
    println!("Final state: {:?}", solution.final_state);
    println!("Accounting complete: {}", solution.stats.accounting_complete);
    println!("RHS calls: {}", solution.stats.rhs_calls);
    println!("Switches: {}", solution.stats.switches);

    // Checked outcome helpers:
    // `solve` returns Ok(solution) even when integration stops early (e.g. attempt budget exhaustion
    // or callback failure) to preserve the partial accepted prefix and full work accounting.
    // Callers must inspect `solution.success`, `solution.final_time`, `solution.message`, and
    // `solution.stats.accounting_complete`, or use checked outcome helpers:
    assert!(solution.is_successful_endpoint(1.0e5));
    solution.ensure_completed()?;

    Ok(())
}
```

### 2. Production Solver & Checked Outcome API (`ProductionSolver`)

For production deployment contracts, [`ProductionSolver`] narrows permissions and enforces strict scheduling without implicit defaults:

```rust
use vigilode::{
    Options, Problem, ProductionConfig, ProductionSolver,
};

fn run_production(problem: &Problem, config: &ProductionConfig) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Unchecked `solve`: preserves partial solution on early stop, returning Ok(solution) with success == false.
    let partial_ok = ProductionSolver::solve(problem, (0.0, 1.0), &[1.0], config)?;
    if !partial_ok.success {
        eprintln!(
            "Early stop at t = {} (accounting complete: {}): {}",
            partial_ok.final_time,
            partial_ok.stats.accounting_complete,
            partial_ok.message
        );
    }

    // 2. Checked `solve_checked`: returns Ok only when success && accounting_complete,
    // and returns typed Err(ProductionSolveError::Unsuccessful(partial)) otherwise.
    match ProductionSolver::solve_checked(problem, (0.0, 1.0), &[1.0], config) {
        Ok(solution) => {
            assert!(solution.is_successful_endpoint(1.0));
            println!("Target endpoint reached: {:?}", solution.final_state);
        }
        Err(err) => {
            if let Some(partial) = err.partial_solution() {
                eprintln!("Incomplete run preserved: final_time = {}, work = {} RHS",
                    partial.final_time, partial.stats.rhs_calls);
            }
        }
    }
    Ok(())
}
```

### 3. Running Benchmarks

```bash
# Run 8-problem battery benchmark
cargo run --release --example opt_baseline_battery > bench/out/opt_candidate.json

# Run A/B regression validator against baseline
python3 tools/compare_opt_battery.py bench/out/opt_candidate_current.json bench/out/opt_candidate.json

# Run adversarial multi-solver benchmark
cargo run --manifest-path crates/vigilode-bench/Cargo.toml --release -- wp \
  --corpus bench/protocols/adversarial_spatial_grid_v1.json \
  --out bench/out/adversarial_spatial_grid_rust_results.json
```

---

## Build reproducibility

A normal fresh clone uses Cargo's default configuration:

```bash
cargo metadata --locked --format-version 1
cargo test --workspace --all-targets --no-run --locked
```

No repository-external sibling directory is required by default. Network access is needed only when the pinned registry artifacts are not already present in the local Cargo cache.

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

`tools/cargo-offline.sh` accepts either `--vendor-dir PATH` or the `VIGILODE_CARGO_VENDOR_DIR` environment variable. The vendor directory must be a standard Cargo directory source created by `cargo vendor`; the wrapper validates its structure, uses an isolated temporary Cargo home with an absolute vendor path, and leaves `Cargo.toml`, `Cargo.lock`, and tracked Cargo configuration unchanged.

The repository keeps `vendor/` untracked. `.cargo/config.offline.toml` is a documented template for users who prefer a repository-local `vendor/` directory and an explicit manual Cargo configuration.

---

## License

MIT
