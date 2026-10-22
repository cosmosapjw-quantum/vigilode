# P04: source-owned H/He photoionization, Case-A recombination, signed stresses and dynamic shear on the LRS photon adapter

BASS/rei DAG node P04, "source-owned absorption/recombination and signed stress integration". It follows
P02b (PASS, L-0149) and runs in parallel to P03 (PASS, L-0150). The ledger row to be written is L-0151.

- **Code under test:** `e3f11a34` on branch `research/bassrei-port-20261021`. Its parent `e54fdd62` (library
  step L3, the accept-control hook for P05) is in the tree; P04 does not call it.
- **Command:** `research/bassrei_p04_source_exchange_stress_20261022/run_p04.py`.
- **Model specification:** `MODEL.md`, including its "Corrections before preregistration".

## Question

Does the RODAS5P fast dense driver integrate a client-owned photon–gas model correctly? Correctly means:

- **Source fidelity:** the photon–gas model reproduces the source owner's rates. These are the Verner fit
  and the Case-A PCHIP from the rei C9 and C11 source files, read only and bound by sha256 in
  `SOURCE_BINDING.json`.
- **Exact exchange:** every exchange identity is exact (photon number ↔ ionization, photon energy ↔
  binding + heat, recombination ↔ escape and electron ledgers, work ↔ work ledgers).
- **Signed stresses:** the matter and radiation stresses are kept apart and signed. Their cancellation in
  the shear equation is exact.
- **Source ledgers:** the budgets of the source (B_src: number 5e-12 per H, energy 5e-11 eV per H) hold
  along trajectories.
- **Application budget:** the application observables meet B_app = 1e-6 relative.
- **Typed exit:** leaving the rate table ends in a typed partial result.
- **VG-1 and VG-3:** zero hot-loop allocation and determinism hold.

## Model (summary; MODEL.md is normative)

**State and units.** Comoving densities, energy in eV, time in proper seconds.

**Photon transport.** The P02 photon transport in the physical energy coordinate `q = ln(E/eV)`:

- energy faces are aligned to the Verner thresholds 13.60, 24.59 and 54.42 eV, with an upper edge of
  100 eV;
- the shear `s` is a state.

**Discrete photon work.** It is a ledger `W_γ`, the energy-weighted sum of the transport RHS.

**Photon–gas exchange.**

- Thomson scattering at `Γ = cσ_T n_e`.
- Photoionization of HI, HeI and HeII with one shared event rate per (species, cell), using
  16-point Gauss–Legendre cell averages of the Verner σ.
- Case-A radiative and dielectronic recombination, plus cooling. The source's PCHIP is in log T with a
  strict domain of 1e4 to 2.4e4 K; outside it the refusal is typed `CASE_A_T_DOMAIN`.
- An escape-energy ledger and a recombination-electron ledger.

**Matter.**

- Adiabatic and shear work on the gas, `−2HU − 2sD_m`, with the opposite floats going into the work
  ledger `W_m`.
- A matter stress relaxing to `−4τ_m s U`.
- `ds/dt = −3Hs + K_D·amp·(D_γ + D_m)a⁻³`, with a prescribed matter-dominated H (test-field
  background).

**Linear invariants.** Five are exact in algebra: number (the source's photons + electrons +
recombination electrons), H, He, photon number, and energy.

## Scenarios

The grid is `(n1, n2, n3, nm) = (16, 16, 16, 8)`, giving `n = 446` (the dense in-place LU is chosen).

**Outputs.** 8 equal output intervals, except S: S uses `0` and `T·10^(−3.5+0.5k)` for k = 0..7, because its
photons are absorbed early.

**Vector atol.** `rtol·1e-6·scale`, with:

- `nH0` for number slots;
- `|U0|` for energy slots and `D_m`;
- `|s0|` for `s`.

The common parameters are `H0 = 3e-17 s⁻¹`, `τ_m = 1e14 s`, `x_HI0 = 0.6`, `f_HeI = 0.2`,
`f_HeII = 0.5`, `s0/H0 = 0.1` and `a2 = 0.5`. Each row below overrides some of them.

| id | nH0 (cm⁻³) | amp | T0 (K) | photons per H | α | τ_m (s) | T (s) | role |
|---|---|---|---|---|---|---|---|---|
| N1 | 2.4e-5 | 1 | 12000 | 0.05 | 1.5 | 1e14 | 1e12 | nominal IGM; photons survive |
| S | 2.4e-3 | 1 | 12000 | 0.05 | 1.5 | 1e14 | 1e13 | stiff (absorption ~2.6e-10 s⁻¹ over 1e13 s) |
| A | 2.4e-5 | 1e15 | 12000 | 0.05 | 1.5 | 1e14 | 1e12 | amplified stress coupling (numerical stress test, not physical) |
| R | 5 | 1 | 14000 | 0.05 | 1.5 | 1e10 | 1e12 | recombination-active (see below) |
| HOT | 2.4e-5 | 1 | 20000 | 2.0 | 0.5 | 1e14 | 1e12 | photoheating leaves the table (C8) |

**What R exercises.**

- Photoionization is very stiff: it lasts ~1e-3 s, against a span of 1e12 s.
- Photoheating raises T from 1.4e4 to 1.6e4 K.
- Case-A recombination then takes x_HII from 0.40 to 0.25.
- The matter-stress relaxation is stiff, with τ_m = 1e10 s ≪ T.
- T stays inside the table, ending at 1.7e4 K.

## Gates [C] (thresholds frozen in `run_p04.py` `TH`)

**C1. Source parity.** The source modules are imported read-only, after the binding check.

| Check | Requirement |
|---|---|
| Rust Verner σ vs the source `cross_sections`, 4010 energies including each χ and its neighbouring floats | ≤ 1e-13 relative; exact zeros agree |
| Rust Case-A values vs the source `CaseARates`, at 20001 temperatures in [T_min, T_max] and at the 129 table nodes | ≤ 1e-12 relative |
| Rust dv/dT vs the reference PCHIP derivative, at the 20001 temperatures | ≤ 1e-12 relative |
| Log-slope error `\|dv/dT_rust − dv/dT_x\|·T/v_x`, at all 20130 temperatures, for x = the source (`v·interp.derivative()(ln T)/T`) and for x = the reference | ≤ 1e-12 |
| T_min and T_max | accepted |
| next_down(T_min), next_up(T_max) and 1e4 K | refused with a typed `CASE_A_T_DOMAIN`, by both Rust and the source |
| table sha256 | matches the spec |
| cargo tests `a_`, `b_` | pass |

**C2. RHS exchange identities and parity.** There are 60 random admissible states around N1, S and A, drawn
with seed `2026102209`.

| Check | Requirement |
|---|---|
| the reference's own unittests (`python3 -m unittest discover -s reference -p 'test_*.py'`) | exit 0 |
| each of the 5 invariants' weighted RHS sum (compensated `fsum`), against the reference's weighted sum of \|terms\| | ≤ 1e-14 (the P02 C3 bound) |
| Rust RHS vs the independent reference, componentwise against \|terms\| | ≤ 1e-14 |
| Jacobian, state-weighted per entry | ≤ 1e-12 |
| f_t (photon block against its max; each matter slot) | ≤ 1e-12 |
| cargo tests `c_`, `d_`, `e_`, `pins_` | pass |

**C3. Trajectory ledgers.** On every OK trajectory of C6, C7 and C9:

- the relative drift of all five invariants is ≤ 1e-11;
- the number drift is ≤ 5e-12 per H;
- the energy drift is ≤ 5e-11 eV per H.

**C4. Transport-work consistency.**

- **Setup:** an exactly projected smooth photon distribution
  `exp(−(q−q_c)²/(2·0.15²))·(1 + 0.5 P2(μ))`, with `q_c` at the grid centre. q and μ are refined
  together: levels 4, 8, 16, 32, 64 (`n1 = n2 = n3 = nm`).
- **Measure:** the relative error of the discrete work `rhs[W_γ]` against the exact `H E_c + 2 s D_c`,
  for `s/H` = 0.1 and −0.2.
- **Requirements:** errors strictly decreasing, and the order of the finest pair in [0.8, 1.25].

**C5. Signed stresses and exchange signs.** N1 states at t = 0.3 T, probed at `amp` = 1e30 and at 1e22.
At each amp:

- **Cancellation, bitwise:** with `D_m = −D_γ`, `ṡ` equals `(−3H)·s` bitwise. H is the runner's own
  background H, and the Python `D_γ` must equal the runner's diagnostic `D_γ` bitwise. The coupling is
  1.1e19 × |3Hs| at 1e30 and 1.1e11 × at 1e22. A dropped or altered `−3Hs` term therefore fails, as does
  a residual coupling.
- **Control:** with `D_m = +D_γ`, `|ṡ + 3Hs|` exceeds 1e6 × 4ε(K_D·amp·|D_γ|a⁻³ + |3Hs|).
- **Bitwise pairs:**
  - `ṅ_HI = −ṅ_HII` for neutral gas with photons;
  - `U̇ = −Ẇ_m` for neutral gas without photons, with `s = 0.2H0` and `D_m = −2τ_m s U`, so that both the
    adiabatic and the shear work are nonzero;
  - `ṅ_HI = −ṅ_HII` for the ionized initial gas with photons (photoionization and recombination).

**C6. Stiff accuracy.** Scenarios S and R, at rtol 1e-4, 1e-6 and 1e-8.

- **Error:** the maximum over outputs t > 0 and over the C7 observables, against Radau (scipy, analytic
  dense Jacobian, restarted at every output, rtol 1e-12).
- **Requirements, per scenario:**
  - strictly decreasing;
  - reference uncertainty u_ref (Radau 1e-11 vs 1e-12, same error measure) ≤ 0.25 × the smallest error.

**C7. B_app at the preregistered rung rtol 1e-8.** For N1, S, A and R:

- every observable at every output t > 0 is within 1e-6 of the Radau 1e-12 reference;
- u_ref ≤ 1e-7.

| Observable | Error measure |
|---|---|
| n_γ, E_γ | relative, with floors of 1e-3 × the initial value |
| D_γ/E_γ (the plan's ratio) | absolute / max(\|D/E\|, 1e-2), weighted by min(1, E_γ,ref/(1e-3·E_γ(0))) |
| D_γ | relative, floor 1e-3·\|D_γ(0)\| |
| D_m | relative, floor 1e-3·max_t \|D_m,ref\| |
| E_esc | relative, floor 1e-3·U(0) |
| x_HII, x_HeII, x_HeIII | relative, floor 1e-3 |
| U | relative |
| s/H | relative, floor 1e-3·\|s0/H0\| |

**C8. Typed refusal, partial result.** HOT, with `max_attempts` 3000.

- **Outcome:** status NUMERICAL_FAILURE with a `DriverPartial` at `t < T`, and `domain_refusals ≥ 1`.
- **Partial:** its reason is `attempt-budget` or `minimum-step`, and its `last_failure` is `non-finite`
  or None.
- **Temperatures:** the partial state's T is in [0.99, 1] × T_max, and every returned T is inside the
  table.
- **Ledgers:** across the returned states and the partial state:
  - relative drift ≤ 1e-11;
  - number drift ≤ 5e-12 per H;
  - energy drift ≤ 5e-11 eV per H.
- **Cargo:** `h_` tests pass.

**C9. VG-1 and VG-3.**

- For N1 and for S, allocation counts are equal at rtol 1e-6 and 1e-9 (allocation mode, positive control).
  The attempts must differ at least 2×, and the in-place zero-skipping LU must be used in both.
- Repeated N1 runs are bit-identical, including counters.
- Cargo:
  - callbacks allocate nothing;
  - the dense driver allocates independently of the step count;
  - uniform vector atol equals scalar atol;
  - the generic-scope guard holds.

**Cargo.** The binaries are `lrs_photon_chem_contracts`, `lrs_photon_fv_contracts` and
`generic_scope_contracts` (release, `--test-threads=1`). Requirements: 0 failed, 0 ignored, and every named
gate test ok.

## [H] reported only

These never decide the verdict:

- wall times (`diagnostic_only`, STATISTICAL_AUTHORITY_HOLD) and reference costs;
- the per-observable error breakdown;
- the C7 errors at the rtol 1e-6 rung;
- the unfloored relative errors of every observable (unbounded for depleted photons in S and R);
- the per-slot photon f_t parity;
- the C1 dt→0 limit of the source's own `rec_step`, against the runner's recombination rows (Richardson
  over dt = 1e-6 and 5e-7 recombination times; it is limited by the source's 1e-12 midpoint iteration);
- C9 rejected steps and domain refusals.

## Design history (disclosed; non-blind)

This node is **non-blind**. Two independent implementations were written from one specification
(`MODEL.md`):

- the Rust adapter, by one implementer;
- the Python reference, by another implementer who never saw the Rust.

Pilot runs on development code observed every gate value before this file was written. A pre-freeze
adversarial review then audited the harness, and its findings were fixed. The fixed harness was run once
more in quick mode, and that run was seen too. Changes, and why:

1. **Specification corrections, now in `MODEL.md`.**
   - The shear-row `f_t` term `K_D·amp·(D_γ+D_m)·d(a⁻³)/dt` was missing from the §8 list. The
     independent reference found it; the Rust adds it, pinned bitwise and with an FD negative control.
   - The partial-run ending is described correctly.
   - The integration grid is (16,16,16,8), so the dense driver keeps its in-place LU. On (4,4,4,4) it
     picks the allocating faer LU, and the runner refuses that with `LU_NOT_IN_PLACE`.
2. **Scenario choices.**
   - A uses `amp` = 1e15. A pilot with 1e11 left `s/H` unchanged at 4 digits; with 1e15, `s/H` goes from
     0.100 to 0.177.
   - HOT uses `max_attempts` = 3000. With the default 1e6, the driver alternates accepted and refused steps
     at the table edge for over 10 minutes.
3. **Harness fixes during the pilots.**
   - The Radau reference process pool uses `spawn`; `fork` beside the live runner threads deadlocked.
   - C5 uses `dataclasses.replace`.
   - The references are saved to `raw/references.json`.
4. **Pre-freeze review: major findings and their fixes.**
   - **M1, C5 was blind to the `−3Hs` term.** At amp 1e30 the old tolerance, 4ε(coupling + |3Hs|), was
     ~1e4 × |3Hs|. C5 now requires bitwise equality at amp 1e30 and at 1e22. It adds a shear-work pair with
     `D_m ≠ 0` and an ionized pair in which recombination is active.
   - **M2, thresholds were changed after seeing the pilot.**
     - **C2:** the pilot set the bound at 8ε, just above its 5ε residual. C2 now uses the P02 C3 bound
       (1e-14 × the sum of |terms|), which was fixed a priori, and draws from a new seed (`2026102209`;
       the pilot used `2026102204`).
     - **The plan's `D_γ/ρ_γ`:** the pilot had replaced it with `D_γ`. It is restored as an observable,
       with the same depletion floor that n and E carry: its error is weighted by
       min(1, E_ref/(1e-3·E_γ(0))). `D_γ` is kept besides it.
     - Both items depart from the plan text. They are **disclosed deviations for the user's sign-off**
       (see below).
   - **M3, coverage was mostly photoionization.**
     - Added scenario R, with recombination, cooling and stiff matter-stress relaxation, to C6 and C7.
     - Added the observables `D_m` and `E_esc`.
     - S uses log-spaced outputs.
     - u_ref is gated for every C7 scenario.
     - The unfloored errors and the 1e-6 rung are reported.
5. **Pre-freeze review: minor fixes.**
   - C1 adds the 129 table nodes and the source's own PCHIP derivative, and adds the `rec_step` limit [H].
   - C8 gates the per-H ledgers, the partial's reason and its `last_failure`.
   - C9 adds the stiff S pair.
   - C2 runs the reference unittests.
   - Provenance:
     - the source binding is re-checked at the end;
     - `rustc -V` is recorded;
     - BLAS is pinned to one thread (`OMP/OPENBLAS/MKL_NUM_THREADS=1`) in the harness and in its spawned
       reference workers, which also cut the Radau cost from ~1100 s to 5–50 s per reference;
     - every C2 state records its metrics and the sha256 of the Rust Jacobian.
   - The CASE_A refusal allocates its message `String` on the rejection path. This is disclosed in TH C8.
     The C9 allocation runs stay inside the table (0 refusals).
6. **Values seen before freezing.** These come from the quick smoke of the fixed harness on development
   code (C6 at the first two rungs only).
   - **C1:** every parity is 0, bitwise to the source. The `rec_step` limit [H] agrees to 1e-12–8e-11.
   - **C2:** invariants ≤ 1.2e-15, RHS ≤ 1.1e-15, Jacobian ≤ 8.0e-16, f_t ≤ 8.7e-16; unittests 24/24.
   - **C3:** 15 runs; ≤ 3.0e-14 relative, 1.7e-15 per H and 1.0e-13 eV per H.
   - **C4:** finest-pair orders 1.00 and 0.98.
   - **C5:** bitwise at both amps, with all three pairs bitwise.
   - **C6:**
     - S: 6.5e-7 and 2.3e-8, u_ref 1.5e-13.
     - R: 2.4e-6 and 5.4e-7, u_ref 6.9e-13; the R rung at 1e-8 gives 1.1e-8 (from C7).
   - **C7:**
     - errors: N1 1.2e-10, S 4.0e-10, A 1.2e-10, R 1.1e-8;
     - u_ref ≤ 6.9e-13.
   - **C8:** a partial at t = 2.6e11 s with T = T_max, reason `attempt-budget`, `last_failure`
     `non-finite`, 1505 refusals, and 1.1e-14 per H.
   - **C9:** 89/89 allocations for both N1 (14 vs 38 attempts) and S (80 vs 293).
   - **Cargo:** 32/32 locally (including P03's binary).

## Deviations from the approved plan (for the user's sign-off)

These are reported in the final summary. None of them is a widening against a frozen gate of this node.

1. **The C2 invariant bound.** It is 1e-14 × the sum of |terms|, the P02 C3 bound, in place of the plan's
   1e-15. 1e-15 is 4.5ε, below what the RHS accumulation of these floats delivers (5ε seen on another
   seed).
2. **The `D_γ/ρ_γ` error.** It is weighted by the photon depletion min(1, E_ref/(1e-3·E_γ(0))). Unweighted,
   the ratio of depleted photons is atol-limited and does not converge: 1.93e-5 at both rtol 1e-4 and
   1e-6 in the pilot. The unweighted values are reported [H].
3. **Thomson coverage.** Thomson is weak at every density here (τ_T < 0.1 over each run). The
   Thomson-dominated regimes were gated in P02/P03 (Γ up to 1e6).

## Verdict rule

- **PASS iff every [C] gate C1–C9 passes**, as computed by `run_p04.py`. The verdict is never re-scored.
- **INVALID** on any of:
  - a dirty tree;
  - an input changed during the run;
  - a stale binary;
  - a source-binding mismatch at start or end;
  - a non-empty `raw/`.
- If the run cannot complete, no ledger row is written.
- **If P04 FAILs:** P05 is blocked (approved plan).

## Command

    python3 research/bassrei_p04_source_exchange_stress_20261022/run_p04.py \
      --out research/bassrei_p04_source_exchange_stress_20261022/results.json

## Outputs

`results.json`, `raw/` (with `MANIFEST.json`) and `DAG_STATUS.json`, all in this directory.

## Holdout

None.

## Host and timing

The run is on the local workstation. Wall times are diagnostic only, and no speed claim is made.

## Known limitations

- **Geometry:** H is prescribed (no Friedmann back-reaction).
- **Scenario A:** its amplified coupling is a numerical stress test of the shear border, not physics.
- **Thomson:** physically weak at these densities. It is present, is checked through the Jacobian, RHS and
  invariants, and is not otherwise exercised (deviation 3).
- **Recombination:** no recombination radiation is tracked (Case A: binding and cooling go to the escape
  ledger, as the source does). There are no collisional processes and no secondary ionization.
- **Photon absorption energy:** an absorbed photon carries the cell-average energy `Ebar_i`, a discrete
  modelling choice of O(Δq²).
- **Source rates:** taken read-only from the rei source files, bound by sha256. Live BASS/rei integration
  and production remain HOLD.
