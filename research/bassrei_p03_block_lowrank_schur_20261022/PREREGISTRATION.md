# P03: structured stage solvers for the LRS photon transport adapter (energy-block Thomson low rank, Krylov energy sweep, bordered Schur)

BASS/rei DAG node P03, "energy-block Thomson low-rank plus matter Schur". It follows P02b (PASS, ledger
L-0149). The ledger row to be written is L-0150.

- **Code under test:** library step L2 (`475e9abf`) and `bc912de2` on branch
  `research/bassrei-port-20261021`.
- **Command:** `research/bassrei_p03_block_lowrank_schur_20261022/run_p03.py`.

## Question

Can client-owned structured stage solvers replace the banded and dense LU of the RODAS5P fast driver on the
LRS photon transport operator? The test runs them through L2's
`integrate_rodas5p_fast_client_observed_with_atol`, the real RODAS5P stages and controller. The
requirements are:

- **Exact mode:** exact, backward stable and O(n) when energy drifts one way.
- **Mixed drift:** a typed refusal in exact mode, and convergent in Krylov mode.
- **Bordered model:** correct with the shear, ionization and matter stress as states.
- **Work:** cheaper than the banded LU in counted flops, with zero hot-loop allocation and determinism.
- **Accuracy:** trajectories accuracy-equivalent to the banded and dense LU arms.

## Mathematics

The stage matrix is `W = inv I − J`, with `inv = 1/(hγ)` (the driver's unscaled convention).

**Group structure.** Per energy group `i`, `W` has a tridiagonal block `T_i + Γ I` minus the rank-two
Thomson gain `Γ P`:

- `T_i + Γ I` collects angular transport, energy outflow, absorption and Thomson loss;
- `P = d 1ᵀ/2 + (d∘p) pᵀ/4`, with `d` the angular widths and `p` the cell averages of `P2`.

Energy transport couples a cell only to its upstream neighbour in the same angular column, through a
diagonal. If no two columns drift in opposite directions (in the physical coordinate, `H > 2|s|`), `W` is
block triangular in energy.

**Exact group solve.** Each group is solved by Thomas elimination plus a two-by-two correction:

- Define `y = (T_i+ΓI)⁻¹b`, `z1 = (T_i+ΓI)⁻¹ d/2` and `z2 = (T_i+ΓI)⁻¹ (d∘p)/4`.
- Then `x = y + Γ(m0 z1 + m2 z2)`, with `m0 = 1ᵀx` and `m2 = pᵀx`.

**Conservation-form capacitance.**

- The textbook capacitance `1 − Γ 1ᵀ z1` tends to `inv/(inv+Γ)`, and it cancels when `Γ ≫ inv`.
- Angular transport conserves number, so the column sums of `T_i` are `g = inv + kbar_i + qout_i`.
- From this, `m0 = 1ᵀx` is equivalent to `(gᵀz1 + (1 − Σd/2)) m0 + (gᵀz2 − Σ(d∘p)/4) m2 = 1ᵀy`.
- The leading coefficient is a sum of positive terms, since `T_i + ΓI` is a column diagonally dominant
  M-matrix and so `z1 > 0`.
- The two grid residuals are compensated sums of the stored floats, so the identity is exact for the
  float matrix.
- The second equation is `Γ(pᵀz1) m0 + (Γ pᵀz2 − 1) m2 = −pᵀy`.

**Ledger rows.** `A_i`, `L_lo` and `L_hi` are solved by back-substitution.

**Flop counts.** Each kernel counts its own flops (`+ − × ÷` once each). The closed forms are
`factor_flops` and `exact_solve_flops` in `block_solver.rs`.

**Krylov mode, for a mixed drift.** This is restarted GMRES from `rodas5p_krylov::solve_gmres_into`,
right-preconditioned:

- **Restart length.** Its cycles stop only at a happy breakdown or at the restart length, so restart = 4
  and `max_arnoldi = 48`. The true residual is checked every 4 columns.
- **Preconditioner.** A symmetric block Gauss–Seidel energy sweep with exact group solves:
  - first the downward columns, from the top;
  - then both directions, from the bottom.
- **Two-level correction.** A group-number coarse correction, then another sweep. The coarse operator is
  `R W P`:
  - `R` takes group sums and `P` prolongates as `d_j/2` per unit;
  - the result is a tridiagonal in energy, column diagonally dominant by `Σ(inv + kbar_i) d_j/2`;
  - Thomson and angular transport vanish on it exactly.

**Bordered model** (`border.rs`): the P02 operator in the physical coordinate, plus three states:

- `ds/dt = −3Hs + k(D_γ + D_m)`, with `D_γ = Σ_i w_i Σ_j p_j N_ij` and `w_i` the cell average of `e^q`.
  The transport drifts with this `s`.
- `dx_e/dt = β(1 − x_e) − α x_e²`, a Riccati law with a closed form computed from the
  cancellation-free positive root. The Thomson rate is `Γ = γ0 x_e`.
- `dD_m/dt = −(D_m + κ s)/τ`.

The Schur client works as follows:

- `B = (−∂f/∂s, −∂f/∂x_e)` is formed at linearization.
- `Z = A⁻¹B` is computed at factorization.
- `S = D − cZ`, where `c = −k w p` is the one nonzero border row; `S` is solved by a 3×3 pivoted LU.
- Then `x_ph = y − Z x_b`.

## Scenarios

The grid is G2 (32 × 16) unless stated, with the P02 initial data (`R.initial_state`). Tolerances are a
vector atol `atol_i = rtol·1e-6·scale_i` (P02 rule), with `rtol` 1e-6 unless stated. The physical domain
is `q ∈ [−4, 2.5]` and the comoving domain `[−3, 2]`.

| id | coordinate | geometry | Γ | absorber (k0, q_χ) | T | outputs | drift |
|---|---|---|---|---|---|---|---|
| S1 | physical | R+ (`H=0.5/(1+t)`, `s=0.2(1+t)^-1.5`) | 1e4 | – | 3 | 7 | Down |
| S2 | physical | R+ | 1e6 | – | 3 | 7 | Down |
| K1 | comoving | R+ | 0.5 | (1e4, −0.5) | 3 | 7 | Mixed |
| K2 | physical | constant `H=0.1, s=0.2` | 1e4 | – | 2 | 7 | Mixed |
| B1 | physical, bordered | R+ (H only) | γ0 = 1e4 | (0.5, −0.5) | 3 | 7 | Down |

B1 parameters:

- `α = 2`, `β = 0.5`, `κ = 1`, `τ = 0.1`;
- `k = 0.02 / D_γ(0)`;
- initial border state `(s, x_e, D_m) = (0.15, 0.1, 0)`;
- border atol `rtol·1e-6·(0.15, 1, 0.01)`.

## Gates [C] (thresholds frozen in `run_p03.py` `TH`)

**C1. Exact and Schur stage systems are backward stable and equal a dense LU.**

- **Runs:** S1, S2 and B1 at rtol 1e-6.
- **Sampling:** every k-th stage solve over the whole run, with k = 6, 6, 5 (from the pilot solve counts),
  at most 100 and at least 50 per scenario.
- **Requirements** for every sample, against `W` built densely at the linearization point:
  - normwise backward error ≤ 1e-14;
  - componentwise (Oettli–Prager) backward error ≤ 1e-14;
  - `‖x − x_LU‖∞/‖x_LU‖∞` ≤ 1e-11, both overall and for the photon block;
  - each border unknown (B1) ≤ 1e-11 relative to its own magnitude.

**C2. Trajectories match the LU arms.**

- **Runs:** at rtol 1e-4, 1e-6 and 1e-8:
  - S1 and S2 with `client_exact` against `banded`;
  - B1 with `bordered_client` against `bordered_dense`.
- **Requirements:**
  - maximum over outputs of WRMS(client − LU arm) ≤ 1, with weights `atol + rtol|y|`;
  - the ratio of the errors vs Radau in [0.5, 2];
  - both arms' errors strictly decreasing.
- **References:** Radau (scipy, analytic Jacobian, restarted per output):
  - S at rtol 1e-14 (effective 2.22e-14);
  - B1 at 1e-13.
- **Reference uncertainty:** Radau 1e-13 vs 1e-14 (S) and 1e-12 vs 1e-13 (B1), required ≤ 0.25 × the
  smallest error.

**C3. Mixed drift.** K1 and K2 at rtol 1e-6. Requirements:

- the exact client is REFUSED with `STRUCTURED_EXACT_MIXED_DIRECTION`;
- the Krylov client (restart 4, `max_arnoldi` 48, GMRES rtol 1e-12, two-level) completes with
  direction Mixed and `linear_solve_failures = 0`;
- sampled stage systems (every 35 / 3, at most 100, at least 50):
  - relative residual ≤ 1e-12;
  - componentwise backward error ≤ 1e-12;
  - difference vs LU ≤ 1e-11;
- trajectory WRMS vs banded ≤ 1;
- cargo `c3_*`.

**C4. Typed refusals and epochs (cargo).** Each of these is a typed refusal:

- factor without a linearization;
- solve without a factorization of the current linearization;
- solve after a new linearization or after a rebind;
- shape mismatches, and a rebind with a different shape;
- `inv ≤ 0`, and `nm < 2`;
- for the bordered model: factor and solve out of order, a comoving grid, and a photon Thomson rate.

A rebind to a same-shape mesh with other edges and boundary must solve that mesh: it matches a dense LU,
and a fresh solver bit for bit.

Also covered here: the L2 client input refusals, and retryable client errors becoming rejections with
their work kept.

**C5. Counted flops and cost.**

- **Formula equality:** counted factor and solve flops equal the closed forms exactly for
  `nq ∈ {8,16,32,64,128} × nm ∈ {2,4,8,16,32,64}` (direction Down).
- **Cost:** structured factor+solve flops for one fixed step are below the banded driver's measured
  `factor_operations + solve_operations` for `nq ∈ {8,32,128} × nm ∈ {8,16,32,64}`.
- **Cargo:** `c5_*`.

**C6. VG-1.** Allocation counts are equal at rtol 1e-6 and 1e-9 (allocation mode, positive control),
with attempts differing at least 2×, for S1 exact, K2 Krylov and B1 bordered. `linear_solve_failures`
must also be equal. On the cargo side:

- linearize, factor and solve allocate nothing from the first cycle after construction;
- the L2 client step allocates nothing;
- client runs allocate independently of the step count.

**C7. VG-3 and L2 identity.**

- Repeated S1 exact, K2 Krylov and B1 bordered runs are bit-identical, including counters and client op
  counts.
- Cargo:
  - the exact client agrees with the banded driver;
  - the L2 `BandedStageLu` delegate reproduces the banded driver bitwise;
  - the `partial_t_into` golden digests hold;
  - the generic-scope guard holds.

**C8. Bordered model parity** (G 16 × 8; 4 states including `s < 0`; t = 0 and 0.7):

| Check | Threshold |
|---|---|
| Rust RHS vs Python reference, componentwise against the sum of \|terms\| | ≤ 1e-14 |
| Jacobian per entry (pattern and value), with term scales in the cancelling s and x_e columns | ≤ 1e-14 |
| f_t, componentwise against its \|terms\| | ≤ 1e-14 |
| D_γ | ≤ 1e-14 |
| Python transport cells vs the oracle | bitwise |
| oracle outward flux | ≤ 1e-15 |
| closed-form Riccati, Rust vs Python (a consistency check only) | ≤ 1e-14 |
| integrated x_e (B1, rtol 1e-10) vs the closed form | ≤ 1e-9 |

Cargo `c8_*` covers the Jacobian against finite differences, the Schur solve against a dense LU, and the
Riccati ODE.

**Cargo.** The binaries are:

- `lrs_block_solver_contracts`
- `client_stage_system_contracts`
- `lrs_photon_fv_contracts`
- `partial_t_into_contracts`
- `rodas5p_fast_allocations`
- `generic_scope_contracts`

Release build, `--test-threads=1`. Requirements: 0 failed, 0 ignored, no duplicate names, every named
gate test ok.

## [H] reported only

These never decide the verdict:

- the textbook-capacitance arm on S2;
- the one-level Krylov arm on K1 and K2;
- the full block/banded flop map, including nm 2 and 4;
- wall times (`diagnostic_only`, STATISTICAL_AUTHORITY_HOLD);
- reference costs.

## Design history (disclosed; non-blind)

This node is **non-blind**. Pilot runs and harness smoke runs on development code observed C1, C3, C5, C6,
C7 and C8 values, and C2 at rtol 1e-4 and 1e-6, before this file was written. The changes made after
those pilots, and why:

1. **GMRES restart.** The library GMRES cycle stops only at a happy breakdown or at the restart length,
   never on the Arnoldi residual estimate. With restart 50, every solve used up to 50 columns however good
   the preconditioner was. Restart is therefore 4 with `max_arnoldi` 48.
2. **Two-level preconditioner.** With the sweep alone, the comoving K1 needed up to 48 iterations. With
   the coarse correction it needs 1. On random right-hand sides, `‖I − W M⁻¹‖` is 1e-12 to 7e-8 at
   Γ = 1e4.
3. **Textbook capacitance.** It was planned to show catastrophic cancellation. On S2 it is only about 2–4×
   worse forward than the conservation form, since normwise backward stability holds for both. It
   remains a diagnostic [H] arm. No speed or accuracy advantage of the conservation form is claimed
   beyond these numbers.
4. **Unit tests.** The forward bound in the exact-solve unit test is κ-aware (10 κ ε): κ reaches 1e8 at
   Γ/inv = 1e8. The unit Krylov test accepts a typed `LinearSolve` rejection at inv = 0.75, where rtol
   1e-12 lies at the rounding floor `‖W‖‖x‖ε/‖b‖`. The integration gates are unaffected.
5. **Least-squares warm-up.** The GMRES least-squares workspace is grown once at construction
   (`warm_least_squares`), so allocation counts no longer depend on the largest Hessenberg size first
   reached.
6. **Pre-freeze adversarial review** (one independent reviewer; findings accepted in one fix loop):
   - M1: C8 Jacobian and f_t parity are per entry and componentwise, not normalized by a global maximum.
     Before the fix, zeroing the border row passed.
   - M2: C1 adds componentwise and per-block (border-unknown) errors.
   - M3: sampling covers whole runs.
   - M4: `rebind` refreshes the angular widths, the P2 averages and the boundary policy (a real defect),
     with a C4 test.
   - M5: provenance records git HEAD and status plus a digest of all crate sources.
   - m1: exact grid-sum corrections in the conservation form.
   - m2: factorizations are charged per attempt.
   - m3: the effective Radau rtol is recorded.
   - m6: the Riccati closed-form comparison is labelled a consistency check, and the oracle outward-flux
     comparison is added.
   - m7: zero allocations from the first cycle.
   - m8: C6 equal failure counts, and C7 repeats on all three arms.
7. **Thresholds.** Most thresholds are the approved plan's values (1e-14, 1e-11, 50 iterations,
   [0.5, 2], ≤ 1 tol unit). The componentwise backward-error (1e-14 exact, 1e-12 Krylov) and
   border-unknown (1e-11) thresholds were set after the pilot values were seen:
   - exact runs: 4.6e-16 to 2.3e-15;
   - Krylov runs: up to 6.2e-14;
   - border unknowns: up to 2e-14.

## Verdict rule

- **PASS iff every [C] gate C1–C8 passes**, as computed by `run_p03.py`. The verdict is never re-scored.
- **INVALID** on any of:
  - a change under `crates/` or this node directory (other than the harness outputs) at start or end;
  - an input changed during the run;
  - a stale binary;
  - a non-empty `raw/`.
- If the run cannot complete, no ledger row is written.
- **If P03 FAILs:** the NCP campaign runs without the client arm, and P05 is blocked (approved plan).
  P04 is independent of P03.

## Command

Run from the worktree root on the clean tree at the commit that adds this file:

    python3 research/bassrei_p03_block_lowrank_schur_20261022/run_p03.py \
      --out research/bassrei_p03_block_lowrank_schur_20261022/results.json

## Outputs

`results.json`, `raw/` (with `MANIFEST.json`) and `DAG_STATUS.json`, all in this directory.

## Holdout

None. This is a falsification gate on fixed, declared scenarios. No holdout profile is touched.

## Host and timing

- The run is on the local workstation (Linux x86-64, rustc 1.94.1).
- Wall times are `timing_authority: diagnostic_only`. No speed claim is made; the cost gate is in counted
  flops.

## Known limitations

- **Exact mode:** requires a one-directional energy drift. The mixed case relies on Krylov.
- **Bordered model:** dimensionless and not physical. Its H is prescribed (a test-field background).
- **Flop counts:** counted by the kernels themselves against closed forms, which is close to
  tautological. Only the banded side is measured independently by the library.
- **Krylov hot loop:** one uncontended mutex lock per preconditioner application. It does not allocate.
- **Scope:** nothing about physical rates, live BASS/rei integration or production is claimed. Broad
  production remains HOLD.

---

## Results

Run on the clean tree at `557afd6385c2553c02ed8c038e91ffd88021cbae` (code `bc912de2`, L2 `475e9abf`). The
command was exactly the preregistered one. Wall time was 172 s on the local workstation and is diagnostic
only. The harness built the runner itself and ran the cargo contract tests, 33/33 ok with 0 ignored. The
tree was clean at start and end, and no input changed during the run.

Output hashes:

- `results.json`: `c539adfc3df0680d992abf78d38fc2786ae4dece3182efd5f0251d50fde20221`
- `raw/MANIFEST.json`: `3c488b393fef78ec1f05e27a1ef2eb098f964e0242dd78a4120222ba77a07191`; it covers
  108 raw files.

**Verdict: PASS**: every [C] gate C1–C8 passed.

| gate | pass | key numbers |
|---|---|---|
| C1 | True | S1 92, S2 100, B1 100 samples over whole runs; backward error ≤ 3.4e-16; componentwise ≤ 2.3e-15 (B1); vs dense LU ≤ 2.3e-12 (S2); B1 border unknowns ≤ 2.0e-14 |
| C2 | True | errors (client vs LU arm) at rtol 1e-4, 1e-6, 1e-8, all strictly decreasing; ratios S1 1.000, 1.000, 1.005; S2 0.999, 0.993, **1.880**; B1 1.000, 1.000, 0.996; max WRMS 5.7e-3 |
| C3 | True | K1 and K2: exact client REFUSED (typed); Krylov 0 linear-solve failures, max 1 and 4 iterations per solve (means 1.0 and 2.29); sampled residual ≤ 1.5e-13, componentwise ≤ 6.2e-14, vs LU ≤ 1.8e-13 |
| C4 | True | cargo (typed refusals, epochs, rebind to a new mesh) |
| C5 | True | 30/30 formula equalities; block/banded flop ratio 0.24–0.26 at nm 8, 0.105–0.116 at nm 16, 0.040–0.044 at nm 32, 0.013–0.014 at nm 64 |
| C6 | True | allocations equal at rtol 1e-6 and 1e-9: exact 117/117 (69 vs 234 attempts), Krylov 151/151, bordered 120/120; 0 failures |
| C7 | True | three arms repeat bit-identically (states, counters, op counts); L2 delegate bitwise; golden digests |
| C8 | True | RHS parity 2.8e-16, Jacobian per entry 1.7e-16, f_t identical, D_γ 5.3e-16; transport = oracle bitwise; integrated x_e vs Riccati 8.2e-14 |

**[H] observations** (not gated):

- The textbook-capacitance arm on S2 is 6.5e-12 vs LU, against 2.3e-12 for the conservation form.
- The one-level sweep needs up to 4 iterations on K1.
- The S2 rtol-1e-8 error ratio of 1.88 is near the RODAS5P accuracy floor at Γ = 1e6 that P02b reported
  (about 1.2e-11). Both arms are within 3.6e-11 to 6.8e-11 of Radau.

**Consequence (approved plan):** P03 passes. Together with P04, this unblocks NCP campaign C1 with the
client arm, and P05 (after L3). No physical-rate, live BASS/rei or production claim is made. Broad
production remains HOLD.
