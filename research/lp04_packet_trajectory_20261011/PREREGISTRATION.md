# Preregistration: Uncertainty Propagation and Adaptive Window Transaction (LP04)

Node LP04 of `research/promotion_loop12_20261011/PORTING_DAG.json` (pr_id `PR-LP04`). Registered on branch `pr/lp04-packet-trajectory` (base `a702b3e`). It depends on `LP02` (merged and verified in `L-0119`).

**Claim boundary.**
- Adaptive multi-window packet integration carrying mode radii and weighted initial moment enclosures across sequential windows.
- Inherited error transfer theorem:
  \[
  \rho_{\text{mean}}^{(k)} = \sum_j w_j \rho_j^{(k)}, \quad E_{\text{mean}}^{(k+1)} = \frac{D + \tau(A_0 + h G / (p+2)) + \rho_{\text{mean}}^{(k)}}{1 - h L}
  \]
  \[
  E_{\text{mode}, j}^{(k+1)} = \tau(|z_j| + h G / (p+2)) + h L E_{\text{mean}}^{(k+1)} + \rho_j^{(k)}
  \]
- Transactional atomicity: accepted state $(t_k, \text{model}_k, \rho_j^{(k)})$ is committed only after local bound and global budget checks; rejected trials reduce step size but strictly preserve accepted data unmodified; all work counters accumulate monotonically across retries.
- Separate output policies: `OutputPolicy::MeanOnly` records only $(t, m(t), \rho_{\text{mean}})$; `OutputPolicy::AllModes` records full microscopic amplitudes and mode radii.
- Zero online Jacobian, JVP, Krylov, and linear solver calls across the entire multi-window trajectory.
- Refuses non-positive step, $t_{\text{end}} \le t_{\text{start}}$, non-positive tolerances, dimension mismatches, and step underflow below $\text{min\_h}$.

## Question

Can the uncertainty propagation and adaptive window transaction module in `crates/rodas5p-integrators/src/packet_trajectory.rs`:
1. Advance collective models across multiple windows while propagating inherited mode radii and mean uncertainty;
2. Maintain strict transactional atomicity where rejected attempts never mutate accepted time, state, or error bounds;
3. Accumulate work counters monotonically across all attempts while maintaining zero online solver counters;
4. Reproduce exact decoupled harmonic oscillator solutions within certified mode bounds in the linear limit ($\chi = 0$);
5. Support clean output policy separation between mean-only and full-mode trajectory logs;
6. Refuse invalid parameter combinations with typed errors?

## Specification

### 1. Module `crates/rodas5p-integrators/src/packet_trajectory.rs`
- Enum `OutputPolicy`:
  - `MeanOnly`
  - `AllModes`
- Struct `TrajectoryConfig`:
  - `t_start: f64`, `t_end: f64`, `initial_h: f64`, `min_h: f64`, `max_h: f64`
  - `order: usize`, `degree: usize`, `sweeps: usize`
  - `mean_tolerance: f64`, `mode_tolerance: f64`
  - `output_policy: OutputPolicy`
  - `max_steps: usize`
- Struct `TrajectoryStep`:
  - `t: f64`, `h: f64`, `mean: (f64, f64)`, `mean_radius: f64`
  - `modes: Option<Vec<(f64, f64)>>`, `mode_radii: Option<Vec<f64>>`
  - `local_defect: f64`, `total_mean_bound: f64`
- Struct `PacketTrajectoryResult`:
  - `final_t: f64`, `final_mean: (f64, f64)`, `final_mean_radius: f64`
  - `final_modes: Vec<(f64, f64)>`, `final_mode_radii: Vec<f64>`
  - `steps: Vec<TrajectoryStep>`, `accepted_steps: usize`, `rejected_steps: usize`
  - `work: PacketWorkCounters`
- Function:
  - `run_packet_trajectory(model: &CollectiveModel, initial_radii: Option<&[f64]>, config: &TrajectoryConfig, work: &mut PacketWorkCounters) -> CoreResult<PacketTrajectoryResult>`

### 2. Contract Test `crates/rodas5p-integrators/tests/packet_trajectory_contracts.rs`
- `VG-1`: Zero online solver counters; strictly positive packet work counters.
- `VG-2`: Multi-window convergence and agreement with exact Dahlquist linear limit.
- `VG-3`: Transactional atomicity under rejection; output policy separation; inherited radii propagation.
- Negative admission gates: $t_{\text{end}} \le t_{\text{start}}$, non-positive step/tolerances, radii dimension mismatch.

## Commands

```bash
cargo test --offline --locked -p rodas5p-integrators --test packet_trajectory_contracts -- --test-threads=1
python3 tools/lp04_stage0_check.py --output research/lp04_packet_trajectory_20261011/STAGE0.json
```

## Results

Executed 2026-10-18 under commit `e9f32326953d6caf1973cd53f314dd8e0e4a4bd0` (ledger `L-0121`).

### 1. Stage 0 Feasibility Outcome
- **VG-1 (Zero online solver counters):** Online Jacobian calls = 0, JVP calls = 0, Krylov iterations = 0, linear solves = 0 across multi-window trajectory. Moment terms, polynomial multiply terms, Volterra terms, and nonlinear evaluations strictly tracked and positive. `met = true`.
- **VG-2 (Multi-window convergence & linear limit):** Multi-window trajectory advances continuously; matches exact decoupled harmonic oscillator modes within certified mode bounds to $< 10^{-12}$. `met = true`.
- **VG-3 (Transactional atomicity & uncertainty transfer):** Rejected attempts strictly preserve accepted state and time unmodified; work counters accumulate monotonically during retries; clean separation between `OutputPolicy::MeanOnly` and `OutputPolicy::AllModes`; inherited radii propagation rigorously verified. `met = true`.
- **Negative admission gate:** Strictly refuses reversed time interval ($t_{\text{end}} \le t_{\text{start}}$), non-positive step/tolerances, radii dimension mismatches, and step underflows below $\text{min\_h}$. `met = true`.
- **Decision:** `FEASIBLE`.
- **Verdict:** `PASS`.

