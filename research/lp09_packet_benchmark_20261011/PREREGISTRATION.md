# Preregistration: Native Fair Benchmark vs Direct Baseline (LP09)

Node LP09 of `research/promotion_loop12_20261011/PORTING_DAG.json` (pr_id `PR-LP09`). Registered on branch `pr/lp09-packet-benchmark` (base `6c3c65c`). It depends on `LP02` (merged in `L-0119`) and `LP04` (merged in `L-0121`).

**Claim boundary.**
- Native fair benchmark comparing the multi-window packet solver (`run_packet_trajectory`) against the direct stiff ODE solver (`rodas5p_fast`) on matched physical models and matched tolerances.
- Hoisting verification: the direct baseline uses a properly hoisted $O(N)$ ODE problem representing the collective mean feedback, ensuring that no artificial $N$-fold speedup is claimed from unhoisted re-evaluations.
- Strict refusal of weak or unfair baselines: unhoisted baselines repeating nonlinear mean evaluations per mode are strictly refused.
- Elimination of online linear algebra: the packet candidate arm completely eliminates online Jacobian evaluations, LU factorizations, and linear solves (`jacobian_builds == 0`, `direct_factorizations == 0`, `linear_solves == 0`), while the direct baseline executes positive solver counters.
- Zero heap allocation compliance in hot loops (Argon rule `VG-1`).
- Statistical authority hold: single-process wall-clock measurements are marked `"timing_authority": "diagnostic_only"` under R2-STAT-01 and R4-STAT-DEV-01.
- Refuses non-positive steps, reversed intervals ($t_{\text{end}} \le t_{\text{start}}$), non-positive tolerances, and unfair baseline flags.

## Question

Can the native fair benchmark module in `crates/rodas5p-fair-ab/src/packet_benchmark.rs`:
1. Execute matched-accuracy comparisons between the multi-window packet solver and direct Rosenbrock baseline (`rodas5p_fast`);
2. Demonstrate complete online elimination of Jacobian builds, LU factorizations, and linear solves on the packet candidate arm;
3. Ensure that the direct baseline uses a properly hoisted $O(N)$ evaluation structure without artificial slowdown;
4. Strictly refuse unfair or unhoisted baseline comparison requests with typed errors;
5. Enforce diagnostic-only timing authority semantics without making unwarranted production wall-clock promotion claims;
6. Refuse invalid parameter combinations with typed errors?

## Specification

### 1. Module `crates/rodas5p-fair-ab/src/packet_benchmark.rs`
- Enum `BenchmarkArm`:
  - `PacketMultiwindow`
  - `DirectSolverRodas5p`
  - `UnhoistedRhsBaseline`
- Struct `PacketBenchmarkConfig`:
  - `t_start: f64`, `t_end: f64`, `initial_h: f64`, `min_h: f64`, `max_h: f64`
  - `tolerance: f64`
  - `order: usize`, `degree: usize`, `sweeps: usize`
  - `output_policy: OutputPolicy`
  - `allow_unhoisted: bool`
- Struct `ArmRunSummary`:
  - `arm: BenchmarkArm`, `steps_taken: usize`, `final_t: f64`, `final_mean: (f64, f64)`
  - `error_vs_reference: f64`, `work_counters: WorkCounters`, `packet_counters: PacketWorkCounters`
  - `elapsed_seconds_diagnostic: f64`, `timing_authority: &'static str`, `hot_loop_allocations: usize`
- Struct `PacketBenchmarkReport`:
  - `schema: &'static str`, `node_id: &'static str`, `mode_count: usize`, `tolerance: f64`
  - `packet_candidate: ArmRunSummary`, `direct_baseline: ArmRunSummary`
  - `speedup_ratio_diagnostic: f64`, `work_reduction: WorkReductionSummary`
  - `fairness_verified: bool`, `verdict: &'static str`
- Functions:
  - `build_hoisted_collective_ode_problem(model: &CollectiveModel) -> CoreResult<OdeProblem>`
  - `run_packet_benchmark(model: &CollectiveModel, config: &PacketBenchmarkConfig) -> CoreResult<PacketBenchmarkReport>`

### 2. Contract Test `crates/rodas5p-fair-ab/tests/packet_benchmark_contracts.rs`
- `VG-1`: Zero online solver counters on packet arm; positive Jacobian and LU counters on baseline.
- `VG-2`: Matched accuracy between packet candidate and direct baseline within physical tolerance.
- `VG-3`: Hoisted baseline structure verification; refusal of unhoisted baseline; negative parameter gates.

## Commands

```bash
cargo test --offline --locked -p rodas5p-fair-ab --test packet_benchmark_contracts -- --test-threads=1
python3 tools/lp09_stage0_check.py --output research/lp09_packet_benchmark_20261011/STAGE0.json
```

## Results

Executed 2026-10-18 under commit `48269ce27ff5187a92f59337f6722ad437ab1a8f` (ledger `L-0122`).

### 1. Stage 0 Feasibility Outcome
- **VG-1 (Zero online solver counters on candidate):** Multi-window packet candidate eliminates all online Jacobian evaluations (0 vs positive), LU factorizations (0 vs positive), and linear stage solves (0 vs positive). `met = true`.
- **VG-2 (Matched physical accuracy):** Collective mean error agreement between multi-window packet candidate and direct Rosenbrock baseline (`rodas5p_fast`) matches within physical tolerance $< 10^{-4}$. `met = true`.
- **VG-3 (Fairness & Hoisting verification):** Direct baseline ODE problem confirmed to use hoisted $O(N)$ collective evaluation structure; unfair unhoisted comparison requests strictly refused; timing authority strictly marked diagnostic only (`"timing_authority": "diagnostic_only"`). `met = true`.
- **Negative parameter gates:** Reversed interval and non-positive tolerance strictly rejected with typed `CoreError::InvalidInput`. `met = true`.
- **Decision:** `FEASIBLE`.
- **Verdict:** `PASS`.
