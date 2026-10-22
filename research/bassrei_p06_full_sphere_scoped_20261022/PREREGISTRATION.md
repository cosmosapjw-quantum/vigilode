# P06 (scoped): full-sphere photon transport in triaxial Bianchi I on the (q, μ, φ) adapter

BASS/rei DAG node P06, "full sphere / polarized / class-B", in its scoped form: the unpolarized full-sphere
transport with rank-6 Thomson and the absorber, run through the RODAS5P fast banded and dense drivers. It
follows P05 (PASS, L-0152). The ledger row to be written is L-0153.

- **Code under test:** `4b2d925c` on branch `research/bassrei-port-20261021`, on top of `a0dbbeb7` (L3, P04,
  P05).
- **Command:** `research/bassrei_p06_full_sphere_scoped_20261022/run_p06.py`.
- **Theory and oracle:** `FULL_SPHERE_THEORY.md` and `FULL_SPHERE_FVM_REFERENCE.py` (this directory). The
  oracle is an independent pure-Python float implementation of the theory's bit-parity contract (§9).

## Question

Does a client-owned full-sphere `(q, μ, φ)` finite-volume adapter of triaxial Bianchi I photon transport
satisfy all of the following when it runs through real VigilODE stages?

- It reproduces its formula oracle bit for bit.
- It reduces exactly to the validated P02 LRS adapter when `H1 = H2`.
- It preserves isotropy in isotropic expansion.
- It conserves its number invariant along trajectories.
- It converges at first order to the exact triaxial characteristics.
- It meets VG-1 and VG-3.

## Model (summary; the theory file is normative)

**Variables.** `q = ln(E/E_ref)`, `μ = n_3`, `φ = atan2(n_2, n_1)`. The state is the cell contents
`N_ijk = ∫U dq dμ dφ`, plus outflow ledgers `L_lo`, `L_hi` and an absorbed ledger `A_i` per energy group.
The order is `[L_lo, (N_i·, A_i) per group, L_hi]`.

**Drift.** Prescribed `H_k(t)`:

- `q̇ = −Σ H_k n_k²`;
- `μ̇ = −μ(1−μ²)(H_3 − H_φ)`;
- `φ̇ = (H_1 − H_2) sinφ cosφ`.

These are the exact geodesic drift and Liouville form (Theorems 1 and 2). In the comoving coordinate,
`H_k` is replaced by `H_k − H̄`.

**Discretization.** First-order upwind on shared faces with exact face-averaged normal velocities; φ is
periodic; the μ = ±1 faces carry no flux.

**Collisions.**

- Rank-6 elastic Thomson: a piecewise-constant Galerkin quadrupole kernel with closed-form cell averages.
- The P02 absorber `k0 (E/χ)^-3` above a threshold, which moves in the comoving coordinate.

**Geometries.** `constant {h1, h2, h3}` and `power_law {hh_k/(1+t)}`.

## Gates [C] (thresholds frozen in `run_p06.py` `TH`)

**C1. Oracle checks and the golden fixture.** `python3 FULL_SPHERE_FVM_REFERENCE.py raw/oracle_CHECKS.json`.

- The run exits 0 with status `pass_scoped`.
- It runs 38 tests, with 0 failures, errors or skips.
- Its recorded sha256 of itself and of the LRS oracle match the files.
- The node's `CHECKS.json`, the golden fixture that cargo `a_` reads, has status `pass_scoped`.
- That fixture's `inputs.oracle_sha256` equals the oracle's sha256.
- Its `parity_fixture`, including `sha256_outputs`, equals the fresh run's.

**C2. Bit parity with the oracle.** 160 random grids (seed `1610752528`):

- nq 1–4, nm 1–5, nphi 1–7, random or uniform edges, five φ offsets;
- both coordinates, both boundaries, Γ ∈ {0, 0.7, 2.5, 1e3}, constant or power-law geometry, LRS in ~20%;
- the absorber in about half the cases, including thresholds on an edge;
- t ∈ {0, 0.37, 1.3}, two states per case.

**Every case must run OK.** Refusals count as failures (none expected; all inputs are valid). Every value
below is compared by its hex, so the sign of a zero counts too.

| Comparison | Requirement |
|---|---|
| inputs, the 16 grid constants and B, drift v and dv, face velocities | equal hex |
| state RHS (no absorber; the runner's absorber; explicit group rates), vs `grid_state_rhs` | equal hex |
| `f_t`, vs `grid_state_partial_t` with the runner's `dkbar` | equal hex |
| the adapter's JVP callback output (it evaluates `J s` through its RHS), vs the oracle's state RHS of `s` | equal hex |
| dense Jacobian, vs `grid_state_jacobian_dense` | equal hex |
| band | equals the dense matrix inside the band; the dense matrix is 0 outside it |
| the Jacobian action: `J_dense·s` (numpy, the runner's dense `J`) vs the oracle's state RHS of `s` | `\|Δ\|_k ≤ 1e-14·(\|J\|\|s\|)_k`; exact where that is 0 |
| `k̄` and `dk̄/dt` per group, vs the independent P02 reference `lrs_reference.kbar_closed`, evaluated at the sphere's float threshold `x = q_χ + ((ln a1 + ln a2) + ln a3)/3`, rate `H̄` | ≤ 1e-14 relative; exact where the reference is 0; bitwise at exact ties |
| cross-check of `k̄`, vs the P02 adapter's probe | physical: bitwise; comoving: ≤ 1e-12 relative to the max (that adapter rounds the mean ln a differently) |
| counts | 160/160 OK; 0 refused; 0 mismatches; 0 sign-of-zero differences; absorber cases present in both coordinates |
| cargo | `a_`, `b_`, `c_`, `d_` and both `sanity_` tests of `sphere_photon_fv_contracts` pass |

The bound on the Jacobian action is set a priori: a row has at most ~45 terms, and 45·1.11e-16 < 1e-14.

**C3. LRS reduction along fixed-step trajectories.** With `H1 = H2 = h − s` and `H3 = h + 2s`, the φ-summed
sphere state is compared with the P02 adapter on the same (q, μ) grid. Both runs are `fixed_banded`, with
64 equal steps over T = 3 and 5 outputs. The sphere initial state is the P02 state times the φ weights
`g_k = Δφ_k/2π`.

**Scenarios.**

- **L1:** physical, outflow, (h, s) = (0.3, 0.1), Γ = 0.5, absorber (0.5, −0.4), 12 × 6 × 4, φ0 = 0.3.
- **L2:** comoving (a moving threshold, non-autonomous), closed, (h, s) = (0.2, −0.15), Γ = 2, absorber
  (1.0, −0.5), 10 × 5 × 7 with random μ and φ edges, φ0 = 1.1.

**Requirements.**

- Both adapters take 64 accepted steps.
- At every output and in every energy group i, the deviation is ≤ 1e-13:
  `max(max_j |Σ_k N_ijk − N_ij|, |A_i,3D − A_i,2D|) / max(max_j |N_ij|, |A_i,2D|)`.
- The outflow ledgers satisfy `|L_3D − L_2D| / |L_2D|` ≤ 1e-13, exact where `L_2D = 0`.
- The axisymmetry defect, per group, `max_j(max_k N_ijk/g_k − min_k N_ijk/g_k) / max_j |N_ij|`, is
  ≤ 1e-13.
- cargo `e_` passes.

**C4. Isotropy.** Isotropic geometry, with isotropic data `N_ijk = f_i Δμ_j Δφ_k`.

**Scenarios.**

- **I1:** physical constant H = 0.3, Γ = 0.5, absorber; random μ and φ edges, 10 × 6 × 5.
- **I2:** comoving power law hh = 0.5, closed, Γ = 2, absorber; uniform grid, 10 × 8 × 6.

**Requirement.** Each scenario runs as banded at rtol 1e-8 and as `fixed_banded` with 64 steps. For both
runs, per group i, `(max_jk u_i − min_jk u_i) / max_jk u_i` with `u = N/(Δμ Δφ)` is ≤ 1e-13 at every
output.

**C5. Convergence to the exact triaxial characteristics.**

**Setup.**

- Physical coordinate, outflow, no collisions, constant H = (0.3, −0.1, 0.05), T = 1.
- `q ∈ [−4, 4]`, φ0 = 0.25.
- `U0 = exp(−q²/(2·0.7²))·(1 + 0.3n3 + 0.25n3² + 0.3(n1² − n2²))`.
- Levels (16,8,8), (32,16,16) and (64,32,32), integrated banded at rtol 1e-7.

**Error.** The relative L1 error `Σ|N − N_exact|/Σ|N_exact|`. `N_exact` is the 3-point tensor
Gauss–Legendre integral, in (q, θ, φ), of the exact solution `Π A_k (E/E0)³ U0(q0, n0)` over each cell
(Theorem 4, oracle `exact_cell_contents`). The initial state is the same quadrature at A = 1.

**Requirements.**

- The errors are strictly decreasing.
- The order of the finest pair is in [0.75, 1.25].
- **Time uncertainty, at every level:** `Σ|N(rtol 1e-7) − N(rtol 1e-9)| / Σ|N_exact|` ≤ 0.02·err. This
  keeps the shift in log2 ≤ ~0.03.
- **Quadrature uncertainty, at the coarsest level:**
  - |err(GL3) − err(GL5)| ≤ 0.25·err;
  - the state differences `Σ|N_exact,GL3 − N_exact,GL5| / Σ|N_exact,GL5|` at t = 0 and at T are each
    ≤ 0.02·err.

**Why this setup.** It was chosen after pilots; see the design history.

- The characteristic map is diagonal, `n0 ∝ (A1n1, A2n2, A3n3)`, so it preserves whether the data is even or
  odd in n1 and in n2.
- Data even in n1 and in n2 depends only on n1², n2² and n3. These are `(1−μ²)cos²φ`, `(1−μ²)sin²φ` and
  `μ`: polynomials in μ and trigonometric polynomials in φ. So the data and the exact solution are smooth
  in (μ, φ) at every time, including the poles.
- Data with odd powers of n1 or n2 carries `√(1−μ²)`.
- The half-width 4 = 5.7σ keeps the data at the outflow boundaries below 1e-7 relative.

**C6. VG-1.**

- **Scenario:** non-autonomous triaxial, comoving power law hh = (0.5, 0.2, −0.1), Γ = 0.5, absorber
  (0.5, −0.4), 12 × 6 × 4 (n = 302), T = 3.
- **Requirement:** allocation counts are equal at rtol 1e-6 and 1e-9. This holds for the banded and for
  the dense driver, each with a positive control and attempts ≥ 2× apart. The dense LU must be
  `InPlaceZeroSkipping`.
- **cargo:** both `f_` tests and the generic-scope guard pass.

**C7. VG-3 and the trajectory invariant.**

- **Repeats:** repeated banded and dense runs give bit-identical states and equal WorkCounters.
- **Number invariant:** it is the sum of every slot, including the cells and the absorbed and outflow
  ledgers. Its relative drift `|fsum(y(t)) − fsum(y(0))| / |fsum(y(0))|` must be ≤ 1e-11 at every output.
  This covers every C3 trajectory (3D and 2D, both arms) and every C4, C6 and C7 trajectory. The bound is
  the P02 C3 / P04 C3 one.
- **cargo `g_`:** passes. It now also asserts the same invariant bound on its dense and banded
  trajectories, and includes the uniform-vector-atol ≡ scalar check.

**C8. Typed refusals.** Each invalid input is REFUSED with the oracle's code for the same input. Where
the oracle has no such check, the code is the listed literal.

| Invalid input | Code |
|---|---|
| φ edges not a full period | `phi.not_periodic_cover` |
| μ not the full sphere | `mu.not_full_sphere` |
| q edges not increasing | `q.width` |
| φ edges not increasing | `phi.width` |
| Γ < 0 | `rate.invalid` |
| a short state through the adapter's probe | `state.length` |
| power law at 1 + t ≤ 0 | `geometry.domain` (literal) |
| k0 < 0 | `absorber.invalid` (literal) |

cargo `h_` passes.

**Cargo.** The binaries are `sphere_photon_fv_contracts`, `lrs_photon_fv_contracts` and
`generic_scope_contracts` (release, `--test-threads=1`). Requirements: 0 failed, 0 ignored, and every named
gate test ok (keyed by binary and name).

## [H] reported only

- the oracle's diagnostics, including its g4 convergence table;
- the C3 comparison for adaptive banded runs at rtol 1e-8, where the step sequences differ;
- `k̄` vs `lrs_reference.kbar_mpmath` (40 digits);
- per-output values, attempts and C5 invariant drift;
- wall times (`diagnostic_only`, STATISTICAL_AUTHORITY_HOLD).

## Design history (disclosed; non-blind)

This node is **non-blind**.

- **The oracle and the theory** were written first. They were checked by their own 38 tests (Fractions /
  mpmath identities, conservation, LRS reduction, rank-6 stationarity, polarization contract).
- **The Rust adapter** was then written to the theory's §9 bit-parity contract.
- **A development cross-check** (200 cases, 430,687 values) found no mismatch.

### Pilots on base 031cd20b

Base `031cd20b` is the P03 result, without L3, P04 and P05. The pilots observed every gate value before
any text of this file was written.

1. **C5 setup.** The setup was chosen after pilots, in which **the theory's own g4 data failed the window**.

   All runs below were on the oracle with `expm_multiply`, except the last.

   | Run | Grid levels | Errors | Orders |
   |---|---|---|---|
   | S0: g4 data (odd in n1, n2; q ∈ [−3, 3], σ = 0.5) | 12/24/48 | 0.0670, 0.0447, 0.0270 | 0.58, 0.73 |
   | S1: the even data above (q ∈ [−4, 4], σ = 0.7) | 12/24/48 | 0.0497, 0.0317, 0.0180 | 0.65, 0.81 |
   | S1 | 12/24/48/96 | adds 0.00940 | adds 0.94 |
   | S2: even data, narrow (q ∈ [−3, 3], σ = 0.5) | 12/24/48 | 0.0693, 0.0441, 0.0258 | 0.65, 0.77 |
   | S3: even data, compact C³ bump `(1−(q/2.4)²)^4` on [−3, 3] | 12/24/48 | 0.0354, 0.0216, 0.0124 | 0.71, 0.80 |
   | S1, angular-only refinement | 96×6×6 → 96×24×24 | — | −0.002, 0.22 |
   | S1, energy-only refinement | 12 → 96 at 24 × 24 | — | 0.72, 0.86, 0.74 |
   | S1, energy-heavy | 24/6/6 → 96/24/24 | — | 0.68, 0.78 |
   | S0, energy-heavy | 24/6/6 → 96/24/24 | — | 0.45, 0.68 |

   - **Conclusion:** the energy and angular errors are comparable, so all three axes are refined
     together.
   - **The grids were extended rather than the window widened.** Rust banded at 96 × 48 × 48 needs an
     8 GB band and is infeasible, so the levels are 16/32/64 (Rust).
   - **The Rust values on that sequence:**
     - errors 0.04435, 0.02574 and 0.01380; orders 0.785 and 0.899;
     - Rust 48 × 24 × 24 gives 0.018031511, equal to the oracle.
2. **C8.** The state-length case first went through the runner's own request check (`request.invalid`).
   It became a probe, so that the adapter's `state.length` is tested.
3. **Harness fix.** Two `sanity_` test names occur in both the P02 and P06 binaries, so the cargo gate keys
   tests by (binary, name). Before this fix, all 25 tests passed and only the duplicate check failed.
4. **Values on 031cd20b, before review:**
   - **C2:** 160/160, 678,654 values, 0 mismatches.
   - **C3:** fixed 2.5e-15 / 1.0e-15 (L1) and 3.1e-15 / 1.0e-15 (L2), global normalisation.
   - **C4:** ≤ 4.1e-15, global normalisation.
   - **C5:** as above.
   - **C6:** 92/92 and 85/85.
   - **C7, C8:** all pass.
   - **cargo:** 25/25.

### Pre-freeze adversarial review (four lenses, verified) and its single fix loop, re-piloted on a0dbbeb7

Each fix, with before → after:

1. **C2: no case may drop out.** `min_ok_cases` 150 → 160. Refusals were silently skipped → the refused
   list must be empty.
2. **INVALID scope (the P05 finding-D precedent).** It covered `crates/` and the node directory. It now
   also covers:
   - the LRS oracle;
   - the P02 reference directory;
   - `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml` and `.cargo/`.
   These are added to the provenance hashes too.
3. **Incomplete runs.** A runner TIMEOUT or ENVIRONMENT_FAILURE, or a nonzero exit, used to yield FAIL; it
   now makes the run INVALID (no ledger row).
4. **The golden fixture.** C1 now checks the node `CHECKS.json`: status, oracle sha256, and parity fixture
   equal to the fresh run's. It is in the provenance hashes.
5. **Trajectory invariant.** It was reported only in C5 → it is a [C] gate in C7 over 20 trajectories
   (≤ 1e-11). cargo `g_` asserts it too; that is a test-only Rust change, and `g_` passed with it at
   first run (max 1.9e-16).
6. **C5 disclosure.** The setup was described as "a priori"; it is now disclosed as pilot-selected, with
   every pilot listed above. The two pilots previously omitted are S2 and S0 energy-heavy.
7. **Re-scorability.** C2, C5 and C8 requests and responses are kept whole in `raw/`, plus
   `raw/c5_exact.json` (raw/ is 27 MB in the pilot).
8. **C5 uncertainties.**
   - **Time:** the check was |Δerr| ≤ 0.25·err at the two coarser levels. It is now a state difference
     ≤ 0.02·err at all three levels, which adds rtol 1e-9 at 64 × 32 × 32.
   - **Quadrature:** it keeps |Δerr| ≤ 0.25·err and adds state differences ≤ 0.02·err at t = 0 and T.
9. **C2 absorber.** The check was only against the P02 adapter. Now each group is checked against the
   independent P02 reference `kbar_closed`: ≤ 1e-14 relative, exact at zeros, bitwise at ties. mpmath is
   reported [H]. The adapter comparison is kept as a cross-check.
10. **C2 bitwise.** IEEE `==` with sign-of-zero differences counted [H] → hex equality for every value
    (pilot: 0 differences).
11. **C3/C4 normalisation.** It was by the global max of the output → per energy group, by that group's
    max. Ledgers are relative to themselves.
12. **C2 "JVP".** The check repeated the state RHS. It is relabelled as the callback check (hex), and the
    real Jacobian action `J_dense·s` vs the oracle's `RHS(s)` is added, with an a-priori bound of
    1e-14·(|J||s|)_k.
13. **Base.** The pilot base was 031cd20b. Re-piloted on a0dbbeb7, which adds L3, P04 and P05, a changed
    `rodas5p_fast.rs` and a changed `lrs_photon_fv_support/mod.rs`. No P06 file changed except the `g_`
    assertion and the harness.

**Values seen on a0dbbeb7** (full pilot, development tree; INVALID only because the tree is uncommitted;
0 incomplete requests):

- **C1:** 38/38; golden fixture equal (`sha256_outputs` 1514967f…).
- **C2:**
  - 160/160 cases, 691,278 values, 0 mismatches, 0 sign-of-zero differences, 0 refused;
  - absorber cases 78 (38 physical bitwise, 40 comoving);
  - vs `kbar_closed`: k 0.0, dk 4.1e-16, 8 tie cells bitwise;
  - mpmath [H]: k 5.7e-15, dk 8.9e-16;
  - Jacobian action: max ratio 6.0e-16 of the bound's scale.
- **C3, fixed, per group:**
  - L1 5.1e-15 / axisymmetry 4.7e-15;
  - L2 3.1e-15 / 3.2e-15;
  - 64/64 steps.
- **C3, adaptive [H]:** 2.8e-11 and 2.5e-8.
- **C4, per group:** I1 1.4e-14 (banded) and 7.4e-15 (fixed); I2 3.6e-15 and 2.2e-15.
- **C5:**
  - errors 0.044355, 0.025736 and 0.013798; orders 0.785 and 0.899;
  - time state differences 4.1e-11, 1.0e-10 and 1.3e-10;
  - quadrature: |Δerr| 3.6e-6, state difference 3.8e-5 at T and 2.9e-6 at 0.
- **C6:** banded 92/92 (15 vs 44 attempts); dense 85/85, `InPlaceZeroSkipping`.
- **C7:** bitwise; invariant ≤ 8.6e-16 over 20 runs.
- **C8:** 8/8.
- **cargo:** 25/25.
- **Run:** 400 s.

## Deviations from the approved plan (for the user's sign-off)

1. **C3 runs at fixed steps.** The plan says "φ-summed 3D = 2D adapter ≤ 1e-13 along trajectories". The
   gate uses fixed-step trajectories, with the same steps in both adapters. The adaptive arm is demoted to
   [H]. With adaptive steps the two adapters choose different steps (28/27 and 37/38), which gives
   2.8e-11 and 2.5e-8: that compares step choices, not the reduction.
2. **C5 grids and data are pilot-selected.** The theory's g4 data and grids give 0.73 < 0.75. The window
   was not widened.

## Verdict rule

- **PASS iff every [C] gate C1–C8 passes**, as computed by `run_p06.py`. The verdict is never re-scored.
- **INVALID** on any of:
  - a change under the watched inputs;
  - an input changed during the run;
  - a stale binary;
  - an incomplete request (TIMEOUT, ENVIRONMENT_FAILURE, a nonzero exit);
  - a non-empty `raw/`.
- **INCONCLUSIVE** in quick or skip-cargo mode. If the run cannot complete, no ledger row is written.

## Command

    python3 research/bassrei_p06_full_sphere_scoped_20261022/run_p06.py \
      --out research/bassrei_p06_full_sphere_scoped_20261022/results.json

## Outputs

`results.json`, `raw/` (with `MANIFEST.json`, the oracle's `oracle_CHECKS.json` and `c5_exact.json`) and
`DAG_STATUS.json`, all in this directory.

## Holdout

None.

## Host and timing

The run is on the local workstation. Wall times are diagnostic only, and no speed claim is made.

## Explicit HOLD (not claimed by this node)

- **Rust polarized transport:** the axisymmetric `(I, Q)` Thomson contract and coherency limiter exist in
  the oracle only.
- **Physics not covered:** tilt and aberration, recoil, class-B (and class-A non-I) geometries (theory §12).
- **Solver and mesh work not done:** a structured sphere stage solver (banded and dense LU only) and sphere
  AMR or remap.
- **Integration and claims:** live BASS/rei integration, production use and wall-time claims.

## Known limitations

- **H is prescribed:** there is no back-reaction.
- **C5 is collisionless.** The Thomson and absorber terms are gated by parity (C2), the reduction (C3),
  isotropy (C4) and the invariant (C7), not by an exact collisional solution.
- **C5's finest-pair order (0.90) is still rising**, so the claim is consistency with first order, not that
  the asymptote was reached.
- **C3 constant geometry:** the P02 power law (s ∝ (1+t)^-p, p ≠ 1) cannot equal the sphere power law
  (all H_k ∝ 1/(1+t)), so C3 uses constant geometries. L2's moving comoving threshold makes it
  non-autonomous.
