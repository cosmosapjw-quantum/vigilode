# BASS/rei P02 fixed-grid adapter: preregistration

Node `bassrei-p02-fixed-grid` (BASS/rei research DAG `research/ncp_followup_20261021/BASS_REI_DAG.json`,
node P02; plan of 2026-10-21). Ledger row **L-0148** (pre-assigned; re-checked free at append time).
This file was written, committed and pushed before the measurement run. The results are appended below
the line "Results" after the run, and nothing above that line changes.

Code under test: branch `research/bassrei-port-20261021` at `e2258191ec55fc3bae4d23e9bfc54dc526175a4c`:

- `crates/rodas5p-integrators/examples/lrs_photon_fv_support/mod.rs` (adapter);
- `crates/rodas5p-integrators/examples/lrs_photon_fv_run.rs` (runner);
- `crates/rodas5p-integrators/tests/lrs_photon_fv_contracts.rs`;
- the harness `research/bassrei_p02_fixed_grid_20261021/run_p02.py` with
  `research/bassrei_p02_fixed_grid_20261021/reference/lrs_reference.py`;
- library step L1 (`OdeProblem::eval_partial_t_into`) at `44a3e86aaf59ecf070003a9247701f0486eb6760`,
  proven by `crates/rodas5p-integrators/tests/partial_t_into_contracts.rs`.

On the L1 evidence:

- The golden digests were recorded on the pre-change tree.
- The negative control fails on the pre-change tree: 78 vs 163 allocations.
- No `Cargo.toml` or `Cargo.lock` change.

## Question

Does a client-owned fixed-grid port of the LRS Bianchi-I conservative photon transport oracle
(`research/ncp_followup_20261021/BASS_REI_FVM_REFERENCE.py`, DAG node P01) run correctly through the
actual RODAS5P stages of VigilODE (dense and banded fast drivers)? Correct means:

- exact parity with the oracle operator;
- exact semi-discrete conservation with ledgers;
- first-order spatial convergence to the exact collisionless and absorbing characteristics;
- fifth-order time convergence in nonstiff settings;
- monotone, reference-matched accuracy in stiff settings (Thomson Γ up to 1e6 and a moving absorption
  threshold);
- zero hot-loop allocation;
- determinism;
- typed refusals.

This is BASS/rei DAG node P02 ("Rust fixed-grid adapter integrated through actual VigilODE stages"). It is
unrelated to the heat-lane P02 of `research/ncp_followup_20261021/NEXT_STEPS.json`.

## Mathematics

Signature (−,+,+,+), proper time t, photon energy E, μ = cos θ, LRS shear σ = diag(−s, −s, 2s) and
S(μ) = s(3μ² − 1).

Physical coordinate q = ln(E/E_ref):

    q̇ = −H − S,   μ̇ = −3 s μ (1 − μ²),   3H + 3q̇ + ∂_μ μ̇ = 0   (Liouville; BASS_REI_PORT_RESEARCH.md §2).

The cell contents N_ij = ∫ U dq dμ with U = a³κE³f evolve conservatively. The comoving coordinate
x = ln(aE/E_ref) has ẋ = −S, and U is unchanged. A threshold χ fixed in physical energy sits at
x_χ(t) = q_χ + ln a(t) and moves with ẋ_χ = H.

Semi-discrete operator, first-order upwind on shared faces (the oracle's exact operations):

- **q faces:** velocity −H_eff − s(3⟨μ²⟩_j − 1), with H_eff = H (physical) or 0 (comoving).
- **Interior μ faces:** −3sμ(1 − μ²). The μ = ±1 faces carry no flux.
- **Boundaries:** zero inflow at the energy boundaries; outflow is booked into ledgers L_lo and L_hi.
- **Thomson:** rank-2 per energy group, Ṅ_j = Γ{(d_j/2)[M0 + ½ p_j M2] − N_j}, with p_j the exact cell
  average of P2.
- **Absorber:** k(E) = k0 (E/χ)^−3 above χ, zero below. The rate is the cell average k̄_i in the grid's
  energy coordinate (closed form); the absorbed number is booked into per-group ledgers A_i.

State layout: [L_lo, (N_i0 … N_i,nm−1, A_i)_{i<nq}, L_hi], with n = 2 + nq(nm+1) and bandwidth nm+1.

The operator is linear in N, so f(t, N) = A(t)N. Then:

- JVP(v) = A(t)v.
- f_t = A′(t)N, taken with the upwind pattern of A(t).
- Σ_k f_k = 0 exactly in exact arithmetic (linear invariant).
- With an exact Jacobian, Rosenbrock stages preserve linear invariants up to roundoff.

**Exact characteristics** (re-derived and checked by an independent ray integration in the pre-freeze
review). Define A = exp∫(H − s)dt and B = exp∫(H + 2s)dt, so that a³ = A²B. Then p⊥ = A E √(1 − μ²) and
p∥ = B E μ are conserved, and

    E0 = E √(A²(1−μ²) + B²μ²),   μ0 = B E μ / E0,   U(t, q, μ) = a³ (E/E0)³ U0(q0, μ0) e^{−τ},

where τ is the optical depth along the ray (threshold crossing found by root finding, then Gauss–Legendre
quadrature).

**Thomson discrete quadrupole:** with H = s = 0, M2 = Σ p_j N_j obeys Ṁ2 = λ2 M2 with
λ2 = −Γ(1 − ¼ Σ d_j p_j²), and M0 is conserved.

**Spatial order:** first-order upwind gives an asymptotic L1 error O(Δ). Before the asymptotic regime,
numerical diffusion ≈ Δq·∫|v|dt is a sizeable fraction of the initial variance σ² = 0.09.

## Design history (pilot run before this preregistration — disclosed)

A complete development run of the gate harness was made before this file was written. It is not a
result of this node, and its outputs are not part of the ledger.

**Pilot outcome:** C1, C2 and C5–C10 passed; C3 and C4 failed.

**C4 L1 errors G1→G4 and the observed G3→G4 orders:**

| scenario | G1 | G2 | G3 | G4 | order G3→G4 |
|---|---|---|---|---|---|
| R+ | 0.620 | 0.433 | 0.273 | 0.159 | 0.786 |
| R− | 0.595 | 0.416 | 0.263 | 0.151 | 0.799 |
| C+ | 0.717 | 0.519 | 0.350 | 0.211 | 0.727 |
| R+abs (physical) | 0.626 | 0.440 | 0.282 | 0.165 | 0.774 |
| R+abs-com (comoving) | 0.203 | 0.122 | 0.068 | 0.035 | 0.934 |

- An extra G5 point was computed for R+ only, giving error 0.0865 and order 0.875 for G4→G5.
- The orders rise with refinement (R+: 0.52 → 0.66 → 0.79 → 0.875). That is the pre-asymptotic
  approach of a first-order scheme, not an adapter defect.

**C3 invariant drift:**

- 4.67e-11 at Γ = 1e6, against the plan's bound of 1e-11.
- Below 4.5e-13 at Γ ≤ 1e4.

The drift is systematic. Roundoff in the stage solves with W = I/(hγ) − J scales like ε·stiffness·T: the
RHS's own sum residual at Γ = 1e6 is 100× that at Γ = 1e4.

**Changes made after the pilot**, all before this preregistration, with reasons:

1. **C3** uses the per-run bound 1e-11 + ε·max(Γ, k0)·T, with factor 1. The plan's 1e-11 came from a
   nonstiff roundoff estimate. This bound gives 6.7e-10 at Γ = 1e6 and T = 3, which is 14× the pilot
   drift. The pre-freeze review asked for factor 1 rather than an initial 10. C3 covers only the
   [C]-gate runs and requires status OK.
2. **C4** evaluates the plan's unchanged window [0.8, 1.25] on the finest affordable pair, G4→G5,
   instead of G3→G4. G5 is run for every scenario. Errors must still decrease monotonically over
   G1…G5, and there is an added requirement that the observed orders strictly increase over G2→G3,
   G3→G4 and G4→G5. The G1→G2 pair is excluded because errors there are 0.2–0.7. G4→G5 outcomes for
   R−, C+, R+abs and R+abs-com are unknown at registration.
3. **C6** tightens the Radau reference from rtol 1e-12 to 1e-13. A new requirement is that the
   reference uncertainty u_ref (VigilODE dense at rtol 1e-12 vs Radau) be ≤ 0.25 × the smallest
   measured error. In the pilot, S2's 1e-12 reference gap was 1/3 of the 1e-8-rung error.
4. A pre-freeze adversarial review of the code (3 independent lenses) produced fixes before the code
   was frozen:
   - **Adapter bug:** with the threshold exactly on a cell edge while moving upward (t = 0 in every
     comoving scenario), f_t used the left-sided derivative. The tie rule now follows the direction of
     motion, and a right-sided finite-difference test was added.
   - **Stricter C1:** the independent RHS and Jacobian vs Python A(t) checks, the per-boundary ledger
     check and the derivative cross-checks are now gated.
   - **Cargo gates:** mapped per test name, with `ignored == 0` required.
   - **Harness hardening:** NaN-safe maxima, fixed-step and time-alignment assertions, build provenance,
     and refusal on a dirty tree.
5. One harness bug was fixed during pilot development: the C1 fixture PT05 split points crossed an
   upwind sign change. No threshold was changed for it.
6. After the changes, only `--quick` smoke runs (G1–G2, no cargo) were made, to exercise the code
   paths. No G3–G5 outcome was observed.

## Verdict rule

- **PASS iff every [C] gate passes**, as computed by `run_p02.py` from the counters and accuracy
  numbers. The verdict is never re-scored or loosened.
- **INVALID** if the harness detects a dirty tree, a stale binary or non-empty raw outputs.
- If the run cannot complete (timeout or crash), it is reported and no ledger row is written. A rerun
  needs a new preregistration if any parameter changes.
- A FAIL stops BASS/rei DAG nodes P03–P06, per the approved plan.
- [H] items are reported and never decide the verdict.

## Holdout

None. This is a falsification gate on fixed, declared scenarios. No holdout profile is used or touched.

## Host and timing

- The run is on the local workstation (Linux x86-64, rustc 1.94.1), and host facts are recorded in
  `results.json`.
- Wall times are `timing_authority: diagnostic_only` and under STATISTICAL_AUTHORITY_HOLD. No speed
  claim is made.
- Accuracy-matched cost against VODE/DVODE, diffsol and Julia is NOT_RUN in this node; it belongs to
  the later NCP campaign node.

## Known limitations

- The spatial scheme is first order; P02 tests port correctness, not a production-accurate solver.
- Rosenbrock steps are not positivity preserving. Minimum contents are reported as [H].
- VG-1 (zero hot-loop allocation) is claimed for the dense and banded fast drivers only. The
  matrix-free driver behind the [H] `mf_identity` arm still allocates `f_t` per fresh attempt.
- The C6 dense-vs-banded sub-criteria are consistency checks that are expected to be bit-identical. On
  this problem the in-place zero-skipping dense LU performs the banded LU's arithmetic, so these checks
  carry no independent accuracy evidence. C6 accuracy rests on the Radau reference and on strict
  decrease.
- In every integration scenario the boundary outflow is negligible. Integration-level conservation
  therefore does not exercise the ledger semantics; the RHS-level contract tests and C1 per-boundary
  checks do.
- Nothing about live BASS/rei integration, physical rates or production status is claimed. Broad
  production remains HOLD.

## Rules (constants fixed now, never tuned)

All constants below are the `TH` dictionary and scenario constants of `run_p02.py` at the code-under-test
commit.

**Units:** dimensionless, with E_ref = κ = 1.

**Initial data:** U0(q, μ) = C·exp(−(q + 0.5)²/0.18)·(1 + 0.5 P2(μ)), with C = 4.1e8. The initial cell
contents are the exact cell integrals (erf in q, exact polynomial in μ, 40 digits, rounded once). The
ledgers start at 0.

**Domains:**

- physical q ∈ [−4, 2.5];
- comoving x ∈ [−3, 2];
- μ ∈ [−1, 1];
- outflow energy boundaries unless stated.

**Grids** (uniform): G1 16×8, G2 32×16, G3 64×32, G4 128×64, G5 256×128 (nq × nm).

**Geometry:**

| name | H(t) | s(t) | T |
|---|---|---|---|
| R+ | 0.5/(1+t) | 0.2(1+t)^−1.5 | 3 |
| R− | 0.5/(1+t) | −0.12(1+t)^−1.5 | 3 |
| C+ | 0.5 | 0.2 | 2 |

**Absorber:** k0 ∈ {0.5, 1e4}, with q_χ = −0.5. On the comoving grids, q_χ is an exact cell edge at t = 0
and the threshold moves upward.

**Tolerances:**

- C4: banded arm, rtol 1e-10, atol 1e-14·C·(cell measure) for cells and 1e-14·(initial total) for ledgers.
- C5: fixed_dense, N ∈ {8, 16, 32, 64, 128, 256} steps on G2.
- C6: rtol ∈ {1e-4, 1e-6, 1e-8}, atol = rtol·1e-6·C·(cell measure) for cells and rtol·1e-6·total for
  ledgers; outputs t ∈ {0, 0.5, …, 3}.
- C7: Γ = 1e4, outputs Γt ∈ {0, 1, 2, 5, 10}, banded with a tiny atol.

**References:**

- expm_multiply (autonomous).
- DOP853 at rtol 2.5e-14.
- Radau at rtol 1e-13 with analytic sparse Jacobian (C5 and C6).
- Exact characteristics: Gauss–Legendre 8×8 on 2×2 subcells, checked against 12×12 on 4×4.

## Kill gate

Every gate is [C] unless marked [H]. **PASS iff C1–C10 all pass.**

- **C1 oracle parity** (runner probe; at least 8 fixtures):
  - transport cell RHS bitwise equal to oracle `lrs_number_rhs`;
  - L_lo + L_hi vs oracle `outward`, and L_lo and L_hi each vs independent per-boundary sums, within
    4ε of the boundary-flux scale;
  - Thomson within 4·nm·ε·max|N|;
  - JVP wiring bitwise;
  - f_t vs independent Python A′(t)y within 1e-14 relative;
  - full RHS (all rows incl. ledgers) and dense Jacobian vs the independent oracle-assembled Python A(t),
    each within 1e-14;
  - k̄ and dk̄/dt vs mpmath within 1e-14; the Python closed form vs mpmath within 1e-14; mp.diff vs
    Leibniz within 1e-10;
  - tie fixtures (threshold exactly on an edge, rate > 0 and rate < 0) must exhibit the tie.
- **C2 Jacobians** (cargo `c2_dense_columns_are_unit_rhs_and_the_band_is_exact`): dense columns equal
  rhs(e_c) within 2ε relative, and the band (model and declared closure) equals the dense matrix inside
  the band and is zero outside it.
- **C3 linear invariant:** for every [C] adaptive run (tags C4_*, C6_*, C7_*), with status OK required,
  max_t |Σy(t) − Σy(0)|/Σy(0) ≤ 1e-11 + ε·max(Γ, k0)·T. [H] runs are reported only.
- **C4 spatial order vs exact characteristics** for scenarios R+, R−, C+, R+abs (physical, k0 = 0.5) and
  R+abs-com (comoving, k0 = 0.5), using the L1 relative error at T. All of the following must hold:
  - errors strictly decrease G1 > G2 > G3 > G4 > G5;
  - observed orders strictly increase: order(G2→G3) < order(G3→G4) < order(G4→G5);
  - order(G4→G5) ∈ [0.8, 1.25];
  - reference self-checks: quadrature refinement ≤ 1e-3 × error, map conservation ≤ 1e-10, PDE
    residual sanity ≤ 1e-5, initial-condition quadrature ≤ 1e-13, absorbed-number refinement ≤ 1e-10.
- **C5 nonstiff fixed-step order (VG-2):** configurations C+ (Γ = 0 and 0.5; expm) and R+ (Γ = 0 and
  0.5; DOP853/Radau).
  - error = max|y − ref|/max|ref| at T;
  - floor = max(1e-12, 100·u_ref);
  - at least 2 pre-floor pairs, and the last pre-floor slope ≥ 4.8;
  - negative control (R+, Γ = 0, f_t dropped): last pre-floor slope ≤ 3.5.
- **C6 stiff (VG-2 stiff variant)** on G2, scenarios S1 (R+, Γ = 1e4), S2 (R+, Γ = 1e6) and S3 (R+
  comoving, absorber k0 = 1e4, Γ = 0.5). Required:
  - for dense and banded, the error max_t Σ|y − ref|/Σ|ref| strictly decreases over the rtol ladder;
  - consistency criteria (expected bit-identical, still gated): dense vs banded endpoint WRMS ≤ 1 and
    error ratio ∈ [0.5, 2] at every rung;
  - fixed_banded vs fixed_dense (N = 64, S1) endpoint max relative difference ≤ 1e-10, with
    fixed_banded accepted = attempts = 64 and 0 rejected;
  - reference uncertainty u_ref (VigilODE dense at rtol 1e-12 vs Radau) ≤ 0.25 × the smallest measured
    error;
  - runner output times equal the requested times exactly.
- **C7 Thomson discrete eigenvalue:**
  - per group, |M2(t) − M2(0)e^{λ2 t}| ≤ 1e-8·max|M2(0)|;
  - M0 relative drift ≤ 1e-13;
  - output times exact.
- **C8 VG-1 allocations** (cargo):
  - `c8_drivers_allocate_independently_of_the_step_count`: dense in-place LU asserted, and banded, on a
    non-autonomous comoving absorber problem with vector atol; counts equal at rtol 1e-6 and 1e-9.
  - `c8_callbacks_allocate_nothing`: 0 allocations over 1000 calls of every callback.
  - `a_non_autonomous_fast_step_with_analytic_ft_allocates_nothing` (L1).
- **C9 VG-3** (cargo):
  - `c9_runs_are_deterministic_and_uniform_vector_atol_is_scalar`: repeat runs are bitwise identical
    incl. counters; a uniform vector atol equals the scalar atol bitwise.
  - `fast_runs_match_the_pre_change_golden_digests` (L1).
- **C10 typed refusals** (cargo `c10_invalid_declarations_and_shapes_are_typed_refusals`).

For every cargo gate:

- all tests of both binaries (`lrs_photon_fv_contracts`, `partial_t_into_contracts`) must report ok,
  including the `sanity_*` tests;
- `ignored == 0`;
- exit code 0.

**[H] (reported only):**

- minimum cell content (positivity);
- the `mf_identity` arm (R+, G2, Γ = 0.5), with its linear iteration counts and invariant drift;
- allocation-mode counts of whole integrate calls;
- wall times (`diagnostic_only`).

## Command

Run from the worktree root at the code-under-test commit plus this file, with no other changes:

    python3 research/bassrei_p02_fixed_grid_20261021/run_p02.py \
      --out research/bassrei_p02_fixed_grid_20261021/results.json

The harness:

1. builds the runner itself
   (`cargo build --profile measurement -p rodas5p-integrators --example lrs_photon_fv_run --locked --offline -j 8`,
   with `RUSTUP_TOOLCHAIN=1.94.1` and `CARGO_TARGET_DIR=/home/cosmosapjw/vigilode-bassrei-target`);
2. runs `cargo test -p rodas5p-integrators --release --locked --offline -j 8 --test lrs_photon_fv_contracts --test partial_t_into_contracts -- --test-threads=1`;
3. refuses a dirty tree or a non-empty `raw/` directory (verdict INVALID);
4. writes `results.json`, plus `raw/` with `raw/MANIFEST.json`.

Defaults: `--jobs 6`, `--run-timeout 3600`, `--cargo-timeout 3600`.

## Outputs

- `research/bassrei_p02_fixed_grid_20261021/results.json`: inputs and their sha256, host, all gate
  values and the verdict.
- `research/bassrei_p02_fixed_grid_20261021/raw/`: compact per-run request and response, cargo logs and
  the manifest.
- `research/bassrei_p02_fixed_grid_20261021/DAG_STATUS.json`: written after the run, with the node
  verdict and its downstream implication.

## Remaining known minor issues (pre-freeze verifier, not gate-affecting)

- The runner maps errors to statuses by `CoreError` variant, so a mid-run clock-resolution
  `InvalidInput` from the adaptive driver would be labelled REFUSED rather than NUMERICAL_FAILURE. Every
  gate requires status OK, so no verdict is affected.
- An OSError while spawning cargo itself aborts the harness without writing `results.json`. That is a
  run that cannot complete; no ledger row is written for it.
