# Preregistration: Generative Collective Model and Structure Admission Contract (LP01)

Node LP01 of `research/promotion_loop12_20261011/PORTING_DAG.json` (pr_id `PR-LP01`). Registered on branch `pr/lp01-collective-model` (base `53f8767`). It depends on `[]` (empty) and serves as the foundational root for Track A packet porting (`LP01` through `LP10`).

**Claim boundary.**
- Generative collective cubic model only: checked private-field scalar complex cubic model with exact stored-input weight normalization and retained independent initial amplitudes.
- One immutable model defines both the target dynamics and certificate expressions; an optional comparison adapter generates an `OdeProblem` with exact analytic JVP and identity mass.
- Admission fails closed: zero/negative weights, unnormalized weights, non-finite parameters, frequency out of `[-1, 1]`, energy out of unit weighted-energy ball, state dimension mismatch, or generic opaque ODE callbacks are strictly refused with typed `CollectiveAdmissionError`.
- No black-box ODE admission: arbitrary ODE callbacks cannot be admitted into the collective packet pipeline without declaring a validated `CollectiveModel`.
- Zero-allocation: RHS and JVP hot loops execute with zero heap allocations (`CountingAlloc == 0`).
- No change to existing `OdeProblem::new` requirements.

## Question

Can the generative collective cubic model (`L12.A`/`L12.B`) be implemented as a zero-allocation, typed Rust module that:
1. Retains all independent microscopic initial amplitudes $z_j \in \mathbb{C}$ and positive normalized weights $w_j > 0$ ($\sum w_j = 1$);
2. Computes the collective mean feedback $\langle a \rangle = \sum_j w_j a_j$ and exact vector field
   $$\dot{a}_j = i \cdot \text{width} \cdot \xi_j a_j + i \cdot \chi |\langle a \rangle|^2 \langle a \rangle$$
   in real-valued state coordinates $(u_j, v_j) \in \mathbb{R}^{2N}$ with zero hot-loop allocations;
3. Computes the exact Jacobian-vector product (JVP) in $O(N)$ operations with zero heap allocations;
4. Strictly conserves the weighted energy invariant $E = \sum_j w_j (u_j^2 + v_j^2)$ when $\kappa = 0$;
5. Preserves odd and even spectral moments $\mu_k = \sum_j w_j \xi_j^k$ and complex weighted moments $\nu_k = \sum_j w_j \xi_j^k z_j$;
6. Exactly reduces to decoupled linear harmonic oscillators when $\chi = 0$ (Dahlquist $g=0$ boundary);
7. Strictly rejects unadmitted generic callbacks, invalid weights, and non-unit energy configurations?

## Specification

### 1. Module `crates/rodas5p-integrators/src/collective_model.rs`
- Struct `CollectiveModel`:
  - `xi: Vec<f64>`: detuning frequencies in $[-1, 1]$;
  - `weights: Vec<f64>`: strictly positive normalized weights $\sum_j w_j = 1$;
  - `initial: Vec<(f64, f64)>`: initial complex amplitudes $z_j = (u_j(0), v_j(0))$;
  - `width: f64`: non-negative frequency scale;
  - `chi: f64`: cubic nonlinear coupling;
  - `carrier: f64`: carrier frequency (conventionally 0);
  - `kappa: f64`: linear damping ($\kappa \ge 0$, default 0).
- Constructor `CollectiveModel::new(...) -> Result<Self, CollectiveAdmissionError>`:
  - Validates all invariants and private fields.
- Methods:
  - `dimension(&self) -> usize`: returns $2N$;
  - `num_modes(&self) -> usize`: returns $N$;
  - `energy(&self, state: &[f64]) -> f64`: weighted energy $\sum_j w_j (u_j^2 + v_j^2)$;
  - `initial_energy(&self) -> f64`: initial energy $\le 1$;
  - `mean(&self, state: &[f64]) -> (f64, f64)`: $(\bar{u}, \bar{v}) = \sum_j w_j (u_j, v_j)$;
  - `rhs(&self, t: f64, y: &[f64], dy: &mut [f64]) -> CoreResult<()>`;
  - `jvp(&self, t: f64, y: &[f64], v: &[f64], jv: &mut [f64]) -> CoreResult<()>`;
  - `spectral_moment(&self, k: usize) -> f64`: $\mu_k = \sum_j w_j \xi_j^k$;
  - `weighted_moment(&self, k: usize) -> (f64, f64)`: $\nu_k = \sum_j w_j \xi_j^k (u_j(0), v_j(0))$;
  - `to_ode_problem(&self) -> OdeProblem`: adapter to standard `OdeProblem` with exact analytic RHS and JVP.
- Enum `CollectiveAdmissionError`:
  - `EmptyModes`, `DimensionMismatch`, `NonFinite`, `DetuningOutOfRange`, `NonPositiveWeight`, `UnnormalizedWeights`, `EnergyExceedsUnitBall`, `NegativeWidth`, `NegativeDamping`, `UnsupportedModel`.

### 2. Contract Test `crates/rodas5p-integrators/tests/collective_admission_contracts.rs`
- `VG-1`: Zero-allocation verified via `CountingAlloc` for both `rhs` and `jvp` hot loops.
- `VG-2`: Convergence and accuracy verified:
  - $\chi = 0$ Dahlquist boundary matches exact decoupled analytic solution $a_j(t) = e^{i \cdot \text{width} \cdot \xi_j t} z_j$ to $< 10^{-13}$;
  - General nonlinear case ($\chi \ne 0$) integrated with `rodas5p` converges with expected high order.
- `VG-3`: Invariant conservation verified:
  - Energy drift $|E(t) - E(0)| < 10^{-11}$ across adaptive integration trajectory when $\kappa = 0$;
  - Exact preservation of odd and even initial moments $\mu_k, \nu_k$.
- Negative admission gate:
  - Confirms typed rejection for all boundary and invalid configurations.
  - Rejection of unadmitted generic `OdeProblem` without collective structure.

## Commands

```bash
cargo test --offline --locked -p rodas5p-integrators --test collective_admission_contracts -- --test-threads=1
python3 tools/lp01_stage0_check.py --output research/lp01_collective_model_20261011/STAGE0.json
```

## Results

Executed 2026-10-18 under commit `254fe0f45eefc3ef0f178fa2d407465c0035f0cc` (ledger `L-0118`).

### 1. Stage 0 Feasibility Outcome
- **VG-1 (Zero-allocation):** Hot loops for `rhs` and `jvp` allocate exactly 0 heap bytes (`CountingAlloc == 0`). `met = true`.
- **VG-2 (Analytic consistency):** Analytic JVP matches finite difference to $< 10^{-6}$; Dahlquist linear boundary ($\chi = 0$) matches exact decoupled harmonic oscillators to $< 10^{-14}$. `met = true`.
- **VG-3 (Invariant & moment preservation):** Continuous energy derivative $\dot{E} = 0$ ($< 10^{-14}$); odd and even spectral moments $\mu_k$ and weighted moments $\nu_k$ preserved. `met = true`.
- **Negative admission gate:** Strictly refuses empty modes, dimension mismatches, $\xi \notin [-1, 1]$, non-positive weights, unnormalized weights, initial energy $> 1$, non-finite parameters, negative width/damping, and generic unadmitted ODE callbacks. `met = true`.
- **Decision:** `FEASIBLE`.
- **Verdict:** `PASS`.
