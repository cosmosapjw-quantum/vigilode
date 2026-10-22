# P05: accepted-step conservative remap of the LRS photon adapter (L3 accept control, client rebind, restart)

BASS/rei DAG node P05, "accepted-step remap + cache rebind". It follows P03 (PASS, L-0150), P04 (PASS, L-0151) and
library step L3. The ledger row to be written is L-0152.

- **Code under test:** `6a16d472` on branch `research/bassrei-port-20261021`, which contains library step L3
  (`e54fdd62`, accept-control hook) and the P05 adapter.
- **Command:** `research/bassrei_p05_accepted_remap_20261022/run_p05.py`.

## Question

Can an accepted RODAS5P state be remapped, mid-integration, onto an energy mesh that tracks a moving
absorption threshold — conservatively, transactionally and without hot-loop allocation — with the driver
caches (model epoch, structured client factorization, step-size controller) rebound and the integration
restarted, such that:

- every remap conserves the total exactly to roundoff, and a refused remap changes nothing but its work
  counters (R1, R2);
- refine/coarsen cycles are exact (R3);
- a structured client bound to the old mesh is refused after the rebind, and the rebound client restarts
  (R4);
- allocations do not depend on the step count, and grow by a constant per remap (R5);
- space and time refine separately and converge as theory says, and the remap diffusion shrinks as the
  grid and the remap count are refined together (R6; the fixed-grid remap-frequency refinement is
  reported [H]);
- no threshold is wider than the approved plan's (R7)?

## Model (summary)

The P02 LRS photon transport in the **comoving** energy coordinate `x = q + ln a(t)` with outflow
boundaries, the power-law geometry `H = 0.5/(1+t)`, `s = 0.2 (1+t)^-1.5` (P02 R+), Thomson `Γ`, and the
absorber `k(E) = k0 (E/χ)^-3` above `χ`. Its threshold drifts through the fixed base grid as
`x_χ(t) = q_χ + ln a(t)` (`q_χ = −0.5`, `k0 = 0.5`), from −0.5 at t = 0 to 0.193 at T = 3.

- **Base grid:** uniform on `[−3, 2]` with `nq` cells (`D = 5/nq`), uniform μ with `nm` cells.
- **Aligned mesh** (`remesh.rs::aligned_edges`): the interior base edge nearest `x_χ` moves exactly onto
  `x_χ`, unless `x_χ` is within `D/4` of an end. `refine` halves the two cells beside the face.
- **Remap** (`Remesher`): the library's positive overlap remap (`ConservativeRemapWorkspace`,
  piecewise-constant overlap fractions, compensated sums) per angular column and for the absorbed-ledger
  column, by sign (nonnegative and negated negative parts separately). `L_lo`, `L_hi` are copied.
  The result is staged and validated (finite; compensated total change ≤ 2e-15 relative; same energy
  domain). From the staged mesh the new model and the client binding are built; only then is it committed
  with a new epoch (the commit is the last fallible step). A refusal, or a failure to build, leaves state,
  mesh, epoch, model and client unchanged and keeps its work counted. Inside an integration it ends the
  run with `success = false`, the typed reason, the accepted (t, state, mesh, epoch) from before the
  attempt and every counter (the runner reports NUMERICAL_FAILURE with them).
- **Arms** (`integrate_segments`):
  - `static`: no remap;
  - `hook`: the L3 accept control stops at the first accepted step with `|x_χ − face| > trigger·D`,
    remaps there and restarts from the controller's `h_next` (`restart = fresh` restarts from the run's
    `initial_step` instead: the plan's control);
  - `schedule` / `fixed_schedule`: segments end at given remap times (restart from the last accepted
    step's proposal, or with N fixed steps per segment).
- **Every remap** builds the new mesh's model (a new `kbar` epoch, the cell atol rule on the new
  measures) and rebinds the structured client, making its old factorization stale. With `refine` the
  shape changes (n + 2 cells) and the client is rebuilt, carrying its predecessor's op counts.
- **Request options for the gates:** `record_remap_states` (every remap's old/new edges and states),
  `inject_failure_at_remap = k` (the k-th remap is refused after all its work).

**Initial data.** The P02 exact cell integrals of `U0 = C exp(−(q+0.5)²/0.18)(1 + 0.5 P2(μ))`,
`C = 4.1e8`. **atol** `= rtol·1e-6·C` × cell measure on cells, the same scale on ledgers.

**Configurations.** E: `Γ = 0` (the exact characteristic solution exists). T: `Γ = 0.5`.

**Grids.** G1 16×8, G2 32×16, G3 64×32, G4 128×64, G5 256×128 (nq × nm).

**Remap schedules.**

- `SMOOTH`: `t_k = 1.15^k − 1`, k = 1..9. Between remaps the threshold moves `0.5 ln 1.15 = 0.0699 < D/2`
  (G2: D/2 = 0.078) and the aligned face's neighbours are at least D/2 away, so the threshold stays inside
  the cell above the face: `kbar(t)` is smooth on every segment.
- `COARSE`: 0.6, 1.2, 1.8, 2.4. The threshold crosses one base edge (e17 = −0.34375 at
  t = e^0.3125 − 1 = 0.367) inside the first segment, on the base mesh before any remap. That crossing is
  a kink of `kbar(t)` (C0 in t). The four remapped segments are smooth (the harness recomputes the
  crossings: 1, 0, 0, 0, 0; SMOOTH: 0 in all 10).

## Independent references

- **`reference/p05_reference.py`** (independent of `remesh.rs`):
  - the aligned mesh;
  - the overlap remap by sign with `math.fsum`;
  - the segmented semi-discrete reference: the P02 operator `A(t)` from the read-only P02 reference and
    formula oracle on each segment's mesh, integrated by scipy (Radau with the sparse analytic Jacobian,
    or DOP853), restarted at every output and remap time, remapped at the remap times onto the aligned
    mesh of its own `x_χ`.
- **Its unittests** (15) check:
  - the mesh properties;
  - the remap against exact rationals, hand cases, identity and constant density;
  - the sign split and the layout;
  - the P02 operator on a nonuniform aligned mesh: column conservation, the absorber cell averages
    against mpmath, and the transport against a direct upwind sum;
  - the segmented reference without remaps against a plain run.
- **Exact solution** (config E): the P02 `Characteristics` (exact characteristics with the attenuation
  `exp(−τ)`):
  - cell integrals by 12×12 Gauss–Legendre on 4×4 subcells, with kink splitting;
  - `absorbed_total`.

## Gates [C] (thresholds frozen in `run_p05.py` `TH`)

**S0.** A static run (config T, G2, rtol 1e-10) provides the integrated states at t = 1.3 and 2.1 for R1 and
R3. Its L_lo, L_hi and absorbed ledgers are nonzero.

**R1. Transactional rollback.**

- **Inject mode**, two cases:
  - I1: the integrated state at t = 1.3;
  - I2: a signed version of the integrated state at t = 2.1 (every 7th cell × −0.3, every 3rd absorbed
    ledger negated), proposed refined.

| Check | Requirement |
|---|---|
| injected propose | `Err(Injected)` after all the work |
| the output buffer (prefilled with a sentinel), the committed edges, the epoch | bitwise unchanged after the injected propose and the following `commit` |
| `commit` after the refusal | `Err(NothingStaged)` |
| work after the refusal | attempts 1, refusals 1, commits 0, overlap segments > 0 (kept) |
| the next normal propose + commit, onto the **other** variant of the aligned mesh (refined ↔ unrefined) | succeeds (attempts 2, commits 1, epoch 1) and equals a fresh remesher's remap bitwise |
| allocations | 0 in the injected and the normal remap |
| both mesh variants | bitwise equal to the reference aligned meshes of the runner's `x_χ`; face indices equal |
| the committed state vs the Python remap of the same state onto that mesh | ≤ 1e-14 per column (max diff / column max) |
| `L_lo`, `L_hi` | nonzero, copied bitwise |

- **I3, integrate level.** Hook (trigger 0.5, config T, G2, rtol 1e-6), banded and `client_krylov`, with
  `inject_failure_at_remap` k = 1 and k = 3.
  - The run ends NUMERICAL_FAILURE with the typed `Injected` refusal.
  - Its final (t, state, edges, epoch) equal bitwise the uninjected run's k-th remap record (t, y_before,
    old_edges, epoch k − 1).
  - Its attempts and every WorkCounters field equal the uninjected run's `attempts_before` /
    `counters_before` of that remap.
  - Remesh work: attempts k, refusals 1, commits k − 1.
  - Outputs before the refusal are bitwise equal.
  - Krylov: client epoch k − 1, and factorizations = `direct_factorizations`.
- **Cargo:** `r1_refusals_…`, `r1_a_refusal_inside_a_run_…`.

**R2. Conservation.**

- `MAX_RELATIVE_TOTAL_CHANGE` in `remesh.rs` is `2.0e-15`.
- **Status:** every integrate run of every gate is OK, except the preregistered R4 `client_exact` (REFUSED)
  and the R1 I3 injected runs (NUMERICAL_FAILURE). OK runs have 0 remesh refusals.
- **Every OK run:**
  - the runner's per-remap changes are ≤ 2e-15;
  - the trace drift is ≤ 1e-11, and so is the Python `fsum` of every returned output state against
    `fsum(y0)`.
- **Independent per-remap check.** It runs on the runs with `record_remap_states`:
  - G2 hook, banded and Krylov;
  - G2 refined hook, banded and Krylov;
  - the G2 SMOOTH schedule at rtol 1e-8;
  - the G3 config E hook.

  For every remap, Python recomputes:
  - `|fsum(y_after) − fsum(y_before)| / |fsum(y_before)| ≤ 2e-15`;
  - the `p05_reference` remap of y_before onto new_edges against y_after, ≤ 1e-14 per column;
  - new_edges, bitwise equal to the Python aligned mesh of the reported face;
  - old_edges, equal to the previous mesh.

  The runner's own per-remap value is checked at its source (a larger change is refused), so this
  independent recomputation is the evidence.
- The inject per-column changes are ≤ 2e-15.
- **Cargo:** `r2_`, `segmented_runs_`.

**R3. Refine/coarsen cycles.** Cycle mode, on the integrated state at `x_χ(1.3)` and on the I2 signed state
at `x_χ(2.1)`. Both the helpers (`split_all`/`merge_pairs`) and the remesher (aligned mesh ↔ its split)
must meet:

- one cycle within 2 ulp; 100 cycles within 1e-13, with exact zeros kept;
- 0 allocations after warm-up;
- for the remesher: every remap ≤ 2e-15, drift ≤ 1e-13, 0 refusals, 201 commits;
- cargo tests `r3_` and `aligned_edges_` pass.

**R4. Stale factorization, rebind, restart.** Hook, trigger 0.5, config T, G2, rtol 1e-6, unrefined and
refined (`refine = true`).

- **Krylov vs banded** (restart 4, max 48, rtol 1e-12, two-level):
  - the same remap count; client epoch = remaps; segments = remaps + 1;
  - `|X_krylov − X_banded| / |X_banded| ≤ 10·rtol` for each of photons, absorbed, L_lo and L_hi, every
    quantity against its own banded magnitude;
  - Krylov factorizations = `direct_factorizations`.
- **Refined runs:** they end on n + 2 cells, the client is rebuilt at the shape change, and its work is
  carried over.
- **Restart:**
  - in every remap of these hook runs, `h_restart` equals bitwise `control_h_next` (the driver's stop
    record against the accept view);
  - the `restart = fresh` controls (banded, Krylov) use `initial_step` at every remap.
- **`client_exact`:** REFUSED with `STRUCTURED_EXACT_MIXED_DIRECTION`.
- **Cargo:** `r4_rebind_…`, `r4_restarts_…`, `r4_refined_…`, the L3 accept-control tests and the vigilode
  record wire test.

**R5. VG-1, VG-3.** Allocation mode on config T, G2, outputs [0, 3], banded and `client_krylov`:

- schedule K = 2, 4, 8 (`T k/(K+1)`): counts equal at rtol 1e-6 and 1e-9 for every K, with ≥ 2× attempts
  and remap count K;
- `A(8) − A(4) = 2 (A(4) − A(2))` exactly;
- hook triggers 1, 0.5, 0.25: equal increments per remap;
- two hook runs: outputs, remaps and counters bitwise equal;
- cargo: `r5_`, `the_control_allocates_nothing`, `generic_solver_core_…`.

**R6. Separate refinement.**

- **(a) Space, without remap.**
  - Static, config E, G2→G5 (q and μ together), rtol 1e-8, against the exact solution at T.
  - Gated metrics: cell L1 `Σ|N − N_ex| / Σ|N_ex|` and absorbed total `|ΣA − A_ex| / A_ex`. Each is
    strictly decreasing, with the G4→G5 order in [0.8, 1.25].
  - The photon-total metric is [H]: by conservation it is the absorbed metric times `A_ex / ΣN_ex`.
  - Quadrature: cell refinement ≤ 1e-3 × the error; absorbed refinement ≤ 1e-10.
- **(b) Time, at a fixed remap schedule.** Schedule `SMOOTH`, config T, G2, rtol 1e-4, 1e-6, 1e-8.
  - Error: the max over outputs 1, 2, 3 of `max|y − y_ref| / max|y_ref|`, against segmented Radau 1e-12.
  - Strictly decreasing; u_ref (Radau 1e-11 vs 1e-12) ≤ 0.25 × the smallest error.
  - Output meshes equal the reference's to 4ε.
- **(c) Fixed-step order, at a fixed remap schedule.** `fixed_schedule` `SMOOTH`, N = 1…64, against
  segmented DOP853 2.5e-14.
  - Floor: max(1e-12, 100·u_ref).
  - Requirements:
    - ≥ 2 pre-floor pairs, last slope ≥ 4.8;
    - 0 rejections and accepted = attempts;
    - attempts within [segments·N, segments·(N+1)];
    - 9 remaps.
  - **Negative control:** `COARSE` against its own reference. Every run must be OK with the same steps
    rule, and the last pre-floor slope must be ≤ 3.5: the one C0 crossing in the first segment breaks the
    classical order. This shows the gate is sensitive to C0 forcing, not to remap behaviour.
- **(d) Remap diffusion (grid and remap count co-refined).** Hook trigger 0.5 (in base widths), config E,
  rtol 1e-8: Σδ (the summed reconstruction L1 change) strictly decreasing over G3, G4, G5, while the
  remap count roughly doubles per level. The fixed-grid remap-frequency refinement (G4, triggers 1, 0.5,
  0.25: Σδ and the errors against the exact solution) is reported [H].

**R7. Documentation meta-gate (no tolerance widening).** It checks two things:

- the R2/R3/R6 thresholds equal the plan literal `PLAN`: per-remap 2e-15, drift 1e-11, 2 ulp, 100 cycles,
  1e-13, [0.8, 1.25] and 4.8;
- the reused constants equal their sources, read by AST, read-only:
  - from the P02 C5 harness: the floors 1e-12 and 100, 2 pre-floor pairs, 4.8, and 3.5;
  - from P02 C4: the order window, 1e-3 and 1e-10;
  - from P02b C6: 0.25.

**Cargo.**

- Binaries:
  - `lrs_photon_remap_contracts`, `accept_control_contracts`, `lrs_block_solver_contracts`,
    `lrs_photon_fv_contracts`, `generic_scope_contracts`;
  - the post-L3 driver regression suites `rodas5p_fast_allocations`, `fast_vector_atol_contracts`,
    `fixed_step_order_contracts`, `client_stage_system_contracts` and `partial_t_into_contracts`;
  - vigilode `records_contracts`.
- Release, `--test-threads=1`. Requirements: 0 failed, 0 ignored, and every named gate test ok.
- The reference unittests must exit 0. This is gated in R1, R6b and R6c.

## [H] reported only

- wall times (`diagnostic_only`);
- the hook arm against the exact solution on its final mesh (G2–G5);
- the fixed-grid remap-frequency refinement on G4;
- nq-only refinement at nm = 16;
- the photon-total metric;
- the restart controls' attempts and their final differences against the h_next runs;
- `h_restart` values;
- L_lo/total0 and L_hi/total0;
- `client_krylov` schedule errors and the `COARSE` adaptive errors;
- Krylov op counts;
- fixed-step attempts per N;
- `x_χ` ulps between Rust (`ln_1p`) and Python (`log1p`).

## Design history (disclosed; non-blind)

This node is **non-blind**. The Rust adapter (`remesh.rs`, the runner, the contract tests) was written
and smoke-tested first. Then:

- exploratory pilots;
- pilot 1 (full, `--skip-cargo`);
- a pre-freeze adversarial review (4 lenses, verified) and one fix loop;
- pilot 2 (full, with cargo).

All of them ran on the uncommitted development tree, hence INVALID, and observed every gate value before
this file was written.

### Pilot-informed choices (before the review)

1. **R6c remap schedule.**
   - An exploratory fixed-step run on `COARSE` gave slopes 4.18, −0.01, 3.68, 0.17, 2.54, 2.03.
   - The cause is that `kbar(t)` is C0 where the threshold crosses an edge inside a segment. The review
     verified that this happens once, in the first, unremapped segment.
   - `SMOOTH` was derived from that theory, so the threshold moves less than D/2 between remaps.
   - The slope bound 4.8 is unchanged. `COARSE` is kept as a negative control (≤ 3.5).
2. **R6a grids.** q and μ are refined together (G2→G5, as P02 C4), because the μ error does not shrink
   with nq alone; the [H] nq-only orders are 0.71, 0.58 and 0.34. G1 is pre-asymptotic.
3. **R6a L_lo/L_hi.** These are not gated: the exact leakage (≈ 2.4e-5 out of 6.2e7) cannot be separated
   from the exact absorbed total.
4. **The structured client.** `client_exact` cannot run this scenario (mixed comoving drift). R4 gates its
   typed refusal, and R4/R5 use `client_krylov`.
5. **Initial data.** The P02 exact cell integrals replace the smoke runs' midpoint Gaussian, because the
   exact solution requires them.

### Pre-freeze review and fix loop (every change below was made because of the review)

| id | Finding | Change | Before → after |
|---|---|---|---|
| A | R4 compared against 10·rtol·total0, which is 1.1e3 × L_lo and 2.9e4 × L_hi. The draft's "~1e-5 of total0" was wrong: L_lo/total0 = 8.9e-9 and L_hi/total0 = 3.5e-10 | Each quantity is now compared against its own banded value, `≤ 10·rtol` (tightened) | L_lo 4.4e-15, L_hi 1.1e-14 (relative to themselves); photons 2.8e-12, absorbed 2.2e-11 |
| B | The R2 per-remap check reused the guard's own value, and non-OK runs were skipped | R2 fails any unexpected non-OK run, and requires 0 refusals. New `record_remap_states` with an independent Python recomputation of the total change, the remap parity and the meshes | 6 runs, 52 remaps: max fsum change 1.9e-16, parity 0.0, meshes bitwise; non-OK runs are exactly the 5 expected ones |
| C | A refusal inside `integrate_segments` dropped every counter and the state; the commit came before the new model and client were built | Rust (`remesh.rs`): build from the staged proposal, commit last; a refusal or build failure ends the run with the accepted (t, y, edges, epoch) and all counters. Runner: `final_state`, `final_epoch`, the per-remap `attempts_before` and `counters_before`, and `inject_failure_at_remap`. New test `r1_a_refusal_inside_a_run_…` and gate R1 I3 | (new) k = 1, 3 × banded/Krylov: state, edges, epoch bitwise; attempts 8 and 16 equal; counters equal; work k/1/k−1 |
| D | INVALID did not cover the P02 reference, the oracle, Cargo.toml/lock, rust-toolchain.toml or .cargo/; timeouts counted as FAIL | Added all of them to WATCHED (git status). Any TIMEOUT, ENVIRONMENT_FAILURE or nonzero exit gives INVALID | (new) 10 watched paths; 0 incomplete requests |
| E | The plan's fresh-h0 control was missing | Runner option `restart: h_next\|fresh`. Gated: `h_restart == control_h_next` bitwise (h_next runs) and `== initial_step` (fresh). The attempts are [H]. New test `r4_restarts_…` | h_next 30 attempts vs fresh 55 (7 remaps each) |
| F | Local ×2 refinement was never integrated, and a rebuilt client dropped its OpCounts | Refined hook runs (banded, Krylov) are now in R2/R4. The rebuilt client carries `ops`. New test `r4_refined_…` | refined Krylov factorizations 59 = direct 59 (before the fix the review measured 51 vs 59) |
| G | The post-L3 driver suites were missing | Added 5 binaries | 37 + 3 → 68 tests in 11 binaries |
| H | R6 could not be re-scored from raw/ | All runner states are kept, and raw/references.json holds the exact cells, absorbed totals and segmented reference states/meshes | raw 3.0 → 16 MB |
| I | The negative controls ignored run failures; the no-remap control could pass on mesh mismatch alone; the COARSE text was wrong | The no-remap control is dropped. COARSE requires every run OK with the steps rule, and its text is corrected (one crossing, first segment); the harness recomputes the crossings | COARSE last slope 2.03, all runs OK |
| J | R6d co-refines grid and remap count | Described as such; the fixed-grid remap-frequency report on G4 is [H] | triggers 1/0.5/0.25: remaps 16/28/43, Σδ 8.5e6/3.4e7/5.6e7, cell L1 3.66e-2/3.78e-2/3.88e-2, absorbed 1.59e-2/2.03e-2/2.14e-2 (the error grows with the remap count at a fixed grid) |
| K | R1 I1 used the t = 0 data | I1 and R3 C1 use the integrated state at 1.3, and I2/C2 a signed version of the state at 2.1 (S0) | ledgers nonzero; parity 0.0 |
| L | `state_unchanged` was a tautology, and the post-refusal commit reused the refused inputs | A sentinel output buffer, and the normal commit onto the other mesh variant compared with a fresh remesher | buffer unchanged; equals fresh: true |
| M | R7 compared against a literal in the same file | Described as a documentation meta-gate; it now also compares 9 reused constants with the P02/P02b TH (AST) | all equal |
| N | The photon metric duplicated the absorbed metric | The photon metric is [H]; the cell L1 and absorbed metrics stay gated | unchanged values |

### Pilot 2 values (full, with cargo, 663 s; development tree)

- **R1:**
  - I1 faces 19/20 and I2 faces 21/20: parity 0.0; meshes bitwise; buffer unchanged; equals a fresh
    remesher; 0/0 allocations; `x_χ` 0 ulp;
  - I3: all 4 cases pass.
- **R2:**
  - 66 runs, 495 remaps;
  - max per-remap 1.9e-16; Python drift 5.8e-16;
  - inject per-column 0.0;
  - independent 52 remaps: fsum change ≤ 1.9e-16, parity 0.0.
- **R3:** 0 ulp; 0.0 after 100 cycles; 0 allocations.
- **R4:**
  - unrefined: 7/7 remaps, epoch 7, relative differences ≤ 2.2e-11;
  - refined: 34 cells, 7/7 remaps, ≤ 7.2e-12, factorizations 59 = 59;
  - restart equality true; fresh controls OK;
  - `client_exact` refused.
- **R5:**
  - schedule 211/339/595 (64 per remap) and Krylov 258/378/618 (60 per remap);
  - hook 331/517/827 (62) and 370/544/834 (58);
  - repeat bitwise.
- **R6a:**
  - cell L1 1.22e-1 … 1.81e-2 (0.85, 0.93, **0.97**);
  - absorbed 7.19e-2 … 7.11e-3 (1.13, 1.13, **1.08**);
  - photons [H] 1.08.
- **R6b:** 3.02e-6, 1.29e-7, 2.61e-9; u_ref 5.6e-15; meshes identical.
- **R6c:**
  - SMOOTH slopes 4.47, 4.80, 4.92, 4.96, 4.97 (last pre-floor **4.96**), attempts 10…643;
  - COARSE control 2.03;
  - crossings SMOOTH 0 × 10, COARSE 1, 0, 0, 0, 0.
- **R6d:** Σδ 7.79e7, 3.39e7, 1.70e7 (15/28/56 remaps).
- **R7:** equal (plan and 9 reused constants).
- **Cargo:** 68/68 (11 binaries); reference unittests 15/15.

## Deviations from the approved plan

No frozen threshold is wider than the plan's (R7). Disclosed choices of measurement:

1. **R6 time and fixed-step refinement** use `SMOOTH`, a fixed remap schedule chosen by theory so that
   `kbar(t)` is smooth on every segment. On `COARSE`, one C0 threshold crossing in the first (unremapped)
   segment breaks the classical order, which makes it the negative control. This is a property of the
   first-order threshold cell average, not of the remap.
2. **The structured client** is exercised in Krylov mode only; exact mode is refused (typed) on the mixed
   comoving drift.
3. **R6a** refines q and μ together (as P02 C4), with G4→G5 as the window pair.
4. **Remap frequency** is refined together with the grid in the gate (R6d). The fixed-grid refinement is
   reported [H]. At fixed grid the error grows with the remap count, because first-order remap diffusion
   accumulates.

## Verdict rule

- **PASS iff every [C] gate R1–R7 passes** (R6 = R6a, R6b, R6c, R6d), as computed by `run_p05.py`. The
  verdict is never re-scored.
- **INVALID** on any of:
  - a dirty watched path (`crates/`, this node, the P02 reference and harnesses, the oracle, Cargo.toml,
    Cargo.lock, rust-toolchain.toml, `.cargo/`);
  - an input changed during the run;
  - a stale binary;
  - a non-empty `raw/`;
  - any runner request that timed out, failed in its environment or exited nonzero (an incomplete run).
- If the run cannot complete, no ledger row is written.
- **If P05 FAILs:** P06 is blocked (approved plan).

## Command

    python3 research/bassrei_p05_accepted_remap_20261022/run_p05.py \
      --out research/bassrei_p05_accepted_remap_20261022/results.json

## Outputs

`results.json`, `raw/` (with `MANIFEST.json`) and `DAG_STATUS.json`, all in this directory.

## Holdout

None.

## Known limitations

- **Coverage:** one geometry family (LRS power law) and one absorber, in the comoving coordinate only.
  The physical coordinate (fixed threshold) needs no remap.
- **Accuracy order:**
  - The remap is first order (piecewise-constant overlap).
  - The hook arm's errors against the exact solution are a little larger than static's at equal grids
    ([H]). At a fixed grid they grow with the remap count ([H] remap frequency), although they converge
    under co-refinement.
  - Alignment removes the kinks of `kbar(t)`; it does not raise the spatial order.
- **Fixed-step micro-steps:** `fixed_schedule` takes N steps of `(t_end − t)/N`. When rounding leaves the
  segment end unreached, the driver adds one ulp-scale step, so attempts may exceed `segments·N` by up to
  one per segment. This is bounded in R6c.
- **Exact client mode:** `client_exact` cannot run the comoving scenario (mixed drift). Only Krylov is
  exercised with remaps.
- **Driver errors:** a driver error inside a segment (for example a client refusal at linearization) still
  ends the run as an error without the accumulated counters. Only remap and remap-build failures carry the
  partial record (R1 I3).
- **Hook timing:** remap times follow the accepted steps. Temporal and fixed-step refinement therefore use
  the schedule arms.
- **Scope:** live BASS/rei integration, other remap orders, sphere AMR (P06 HOLD), and wall-time claims
  remain HOLD.

---

## Results

The run was on the clean tree at `2d24dd6f11ca236f0e93e32529cf4831b0a1be56` (code `6a16d472`, L3 `e54fdd62`).
The command was exactly the preregistered one.

- Wall time was 575 s on the local workstation, diagnostic only.
- The harness built the runner itself and ran the cargo contract tests: 68/68 ok, 0 ignored, 11 binaries.
- The reference unittests passed.
- The watched tree was clean at start and end, no input changed, and 0 requests were incomplete.

Output hashes:

- `results.json`: `567d6a8e3fa864cbe291e644bab6c7a92e458d720d99278a0cc142788624ccb6`
- `raw/MANIFEST.json`: `ef54ab393096e7582de96fa85bd0ec922e8fae6a267745e494a07d2bdb4cfe8b`; it covers
  79 raw files.

**Verdict: PASS**: every [C] gate R1–R7 passed.

| gate | pass | key numbers |
|---|---|---|
| R1 | True | I1/I2: injected refusal typed; buffer, edges and epoch unchanged; NothingStaged after refusal; commit to the other mesh equals a fresh remesher bitwise; remap parity vs the independent Python remap 0.0 on all 17 columns; 0/0 allocations. I3 (refusal inside an integration, k = 1 and 3, banded and Krylov): partial state, edges and epoch bitwise equal to the uninjected run at that remap; counters equal (8 and 16 attempts); refused attempt kept in the remesh work |
| R2 | True | 66 runs, 495 remaps: max per-remap 1.9e-16; Python fsum drift ≤ 5.8e-16. Non-OK runs are exactly the 5 preregistered ones. Independent recomputation on 52 remaps (6 runs): fsum change ≤ 1.9e-16, parity 0.0, meshes bitwise |
| R3 | True | integrated and signed states: one cycle 0 ulp; 0.0 after 100 cycles; 0 allocations after warm-up; 201 commits |
| R4 | True | Krylov vs banded, per-quantity relative: unrefined photons 2.8e-12, absorbed 2.2e-11, L_lo 4.4e-15, L_hi 1.1e-14; refined (34 cells) ≤ 7.2e-12; remaps 7 = 7, client epoch 7; h_restart == controller proposal bitwise; `client_exact` refused as typed (mixed drift) |
| R5 | True | allocations equal at rtol 1e-6/1e-9 for K = 2, 4, 8 (banded 211/339/595, 64 per remap; Krylov 258/378/618, 60 per remap); hook 62 and 58 per remap; repeat bitwise |
| R6a | True | static, config E, G2→G5: cell L1 0.122, 0.068, 0.035, 0.018 (orders 0.85, 0.93, 0.97); absorbed 0.072 → 0.0071 (last order 1.08) |
| R6b | True | SMOOTH schedule: 3.02e-6, 1.29e-7, 2.61e-9; u_ref 5.6e-15; meshes match |
| R6c | True | SMOOTH fixed steps N = 1…64: slopes 4.47, 4.80, 4.92, 4.96 (pre-floor), last pre-floor slope 4.96; COARSE negative control, last slope 2.03 (≤ 3.5), all runs OK |
| R6d | True | hook Σδ 7.79e7, 3.39e7, 1.70e7 (G3, G4, G5); remaps 15, 28, 56 |
| R7 | True | thresholds equal the plan constants; 9 reused constants equal their P02/P02b sources |

**[H] observations** (not gated):

- **Fresh-h0 restart control:** it costs 55 attempts, against 30 with h_next.
- **Hook vs static accuracy:** the hook arm is slightly less accurate than static at equal grids. At a fixed grid (G4, triggers 1/0.5/0.25), the error grows with the remap count, because the first-order remap diffuses the solution.
- **Krylov on the schedule arm:** its errors equal banded's to 3e-16 relative.

**Consequence (approved plan):** P05 passes, which unblocks P06. No live BASS/rei or production claim is
made. Broad production remains HOLD.
