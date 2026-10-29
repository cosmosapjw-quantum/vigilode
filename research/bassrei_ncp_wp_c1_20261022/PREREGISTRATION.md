# C1: accuracy-matched work-precision of the BASS/rei VigilODE arms against VODE, DOP853, Julia and diffsol on NCP

BASS/rei DAG campaign C1 of the approved plan ("C1 NCP campaign", after P03 and P04). The ledger row to be
written is L-0154 (after P05 L-0152 and P06 L-0153).

- **Code under test:** `5a7850bc` + `79a39b8e` (Amendment 1) on branch `research/bassrei-port-20261021`, on top of `451452dc` (L4; harness piloted on `6d5ba7ff`, head after
  P06).
  - **Verified delta of the runner sources** from `d0de4f34` (where the harness was first written) to
    `6d5ba7ff`:
    - `examples/lrs_photon_fv_support/mod.rs` gains one line, `pub mod remesh;` (P05). As a result, the
      1301-line `remesh.rs` is compiled into all three runners and, through `#[path]`, into the diffsol
      adapter.
    - New P05/P06 examples and tests are added, which the campaign does not use.
    - `crates/*/src` is unchanged in that range (L3 `e54fdd62` is already an ancestor of `d0de4f34`).
  - **Verified effect:** the runners and the 0.16.1 diffsol stand-in rebuilt from the 6d5ba7ff tree
    reproduce every RHS bitwise; see the dry run below.
  - **The EXEC base.** EXEC will sit on top of `451452dc` (library step L4). Relative to 6d5ba7ff, L4
    changes only:
    - the accept-control path of `src/rodas5p_fast.rs`: the `AcceptedStepView`/restart `h_next` is now
      capped at `max_step`;
    - two test files. One of them, `fast_driver_golden_digest_contracts`, pins every pre-existing
      fast-driver entry point to its 5fd6e459 output.

    The campaign runners do not use accept control, and no runner or adapter source changes. The default
    dense, banded and client paths are therefore bit-identical, as the golden-digest test proves. The
    local pilot ran on 6d5ba7ff.
- **Command:**
  `bash external_audits/vigilode_bassrei_ncp_c1_20261022/launch_from_local.sh <EXEC>`, where `<EXEC>` is the commit that adds this file (recorded in the ledger row).
  The launcher refuses to start unless EXEC is on a remote branch and contains this file. It runs the
  preflight gates below, uploads, starts the campaign detached on NCP, waits, copies the results back and
  verifies them.
- **Import:** `python3 -s harness/import_results.py <copy> research/bassrei_ncp_wp_c1_20261022` (see
  "Ledger import").

## Question

Do the VigilODE arms meet the application budget B_app = 1e-6 (and the source ledger budget B_src) at the
preregistered rung, on fixed physical scenarios on the NCP host, while bit-identical across cores and
allocation-free? At what budget-matched cost do they run against the user-authorized comparators: scipy
VODE and DOP853, Julia (Rodas5P, FBDF, CVODE_BDF, Tsit5, Vern7) and diffsol?

Cost is **diagnostic_only** (STATISTICAL_AUTHORITY_HOLD). No speed claim is made.

## Cases (harness/scenarios.py is normative)

| case | model | grid | n | stiffness |
|---|---|---|---|---|
| S1-G2, S1-G3 | P02 photon operator, physical q in [-4, 2.5], outflow, R+ geometry, Γ = 0.5, T = 3 | 32×16, 64×32 | 546, 2114 | nonstiff |
| S2-G2, S2-G3, S2-G4 | as S1 with Γ = 1e4 (the P02b/P03 S1) | 32×16, 64×32, 128×64 | 546, 2114, 8322 | stiff |
| S3-G2 | P03 bordered model: dynamic shear, Riccati x_e, Γ = 1e4·x_e, matter stress | 32×16 | 549 | stiff |
| S4N1, S4S, S4R | P04 photon–gas chemistry, scenarios N1 / S / R | (16,16,16,8) | 446 | N1 nonstiff; S, R stiff |
| S1-G1, S2-G1, S3-G1 | auxiliary 16×8 cases (dense VODE, n ≤ 512), reported [H] | 16×8 | 146–149 | — |

**Outputs.** Every 0.5 for S1–S3. For S4, the P04 outputs: 8 equal intervals, and log-spaced for S.

**Vector atol, identical for every solver.** It is `rtol·1e-6·scale`, with the P02b/P03/P04 scale vectors.

**Ladder.** rtol 1e-3 … 1e-10 (8 rungs) for every applicable arm.

## Arms

| family | arms | applicability (a priori, `arms_for`) |
|---|---|---|
| VigilODE | `vg_dense` (n ≤ 2048), `vg_banded`, `vg_client_exact`, `vg_client_krylov` (P02 / P03 block runners); `vg_bordered_dense`, `vg_bordered_client` (S3); `vg_chem_dense` (S4) | all cases |
| scipy VODE 1.11.4 | MF10 Adams (nonstiff); MF21/MF22 dense user/FD Jacobian (n ≤ 512, plan); MF24/MF25 banded user/FD, `lband = uband = nm+1` (P02, S3: S3's border coupling outside the band is dropped) | MF20 not rerun (archived ncp_intensive result) |
| scipy DOP853 | `py_dop853`: the ladder on nonstiff cases; rtol 1e-4 and 1e-8 on stiff cases (300 s cap, censored) | not on G4 |
| scipy DOP853 | `py_dop853_tight`: rtol 3e-11, max_step = 1e-3·T, atol by the campaign rule | nonstiff cases only |
| Julia 1.12.7 | `jl_rodas5p_sparse`, `jl_fbdf_sparse` (sparse Jacobian by coloured dual-number AD of the bitwise Julia port; the exact f_t as tgrad); `jl_cvode_bdf_klu` (the same Jacobian); `jl_cvode_bdf_band` (CVODE's own banded DQ Jacobian; P02/S3); `jl_tsit5`, `jl_vern7` (nonstiff) | all cases |
| diffsol 0.17.1 | `ds_bdf_dense` (NalgebraLU; the Jacobian from the adapter's exact JVP or dense Jacobian), `ds_tsit45` (nonstiff); built by `#[path]` on the VigilODE adapter kernels | n ≤ 2200 (not G4) |

**The `py_dop853_tight` settings.** rtol and the relative step cap follow the DOP853 call of the rei C15
source: `research/rei_c15_sigma_pi_feedback_20261015/coupled.py`, lines 329–332, file sha256
`a920d0e86c0badf395bfabc61dafaee3449a34e4cfe8ccdc28a833674c1c4b1e`, in the rei source worktree. That call uses rtol 3e-11 and
max_step 0.001 in its own ln a time. The source's atol belongs to a different state vector and is not
used. The arm is therefore labelled "tight", not "the source configuration".

## References

For every case:

- **Primary:** the VigilODE LU arm at rtol 1e-12: dense where n ≤ 2048, banded on G3/G4.
- **Cross-check:** scipy Radau at 1e-12 on the independent Python references of P02/P03/P04, with the
  reference Jacobian, restarted at every output.

**Scheduling.** The references run first and alone, before any ladder work, each with an 1800 s cap.
The runners integrate twice per call (a warm run, then the measured run).

## B_app metric and floors (scenarios.bapp_error, floors_of)

The error is the maximum over outputs t > 0 and over the observables of |a − b| / max(|b|, floor). A
non-finite observable error counts as +∞. The floors are taken per case from its reference:

| observable | floor |
|---|---|
| n_γ, ρ_γ | 1e-3 × the initial value |
| D_γ/ρ_γ | 1e-3 × max over the evaluated outputs (t > 0) of \|D/ρ_ref\| |
| D_γ | 1e-3·\|D_γ(0)\| (P04 precedent) |
| x_HII (x_e in S3) | 1e-3 |
| u | none (relative) |
| s/H | 1e-3·\|s0/H0\| |

- **Depletion weight:** the D/ρ error is also multiplied by min(1, ρ_ref/(1e-3·ρ_γ(0))).
- **Unfloored errors:** the unfloored relative error of every observable is reported [H].
- **B_src:** applies to the S4 trajectories: number ≤ 5e-12 per H, energy ≤ 5e-11 eV per H.
- **Matched rung:** for each (case, arm), the loosest rung meeting B_app (and B_src for S4) with valid
  parity. A match at the loosest ladder rung (1e-3) is flagged as censored, since it could be looser.

## Gates [C] (analyze.py; thresholds in scenarios.py)

- **G1 references:** every main case has both references, with u_ref ≤ 1e-7.
- **G2 reference independence:** the Python reference RHS equals the VigilODE kernel's to ≤ 1e-13 against
  the per-component sum of |terms|, at 4 states per case.
- **G3 VigilODE at the preregistered rung rtol 1e-8:**
  - every VigilODE arm of every main case returns exactly the requested outputs, with B_app error
    ≤ 1e-6;
  - the S4 arms also hold B_src;
  - the margin B_app/error is reported.

  This is a sanity gate: the pilot margins are 2500×–6e5×.
- **G4 NCP cross-core bit-identity:** each of the 8 preregistered groups (`vg.IDENTITY`) runs on 3
  distinct cores of 60–63. The outputs must validate, and the sha256 digests of the states and of the
  counters must be equal.
- **G5 zero hot-loop allocation:**
  - 10 allocation groups (`vg.ALLOCATION`): equal counts at rtol 1e-6 and 1e-9, with the counter
    calibrated and the attempts differing at least 2×;
  - 6 rejection groups (`vg.REJECTION`: the four P02 arms on S1-G2 and the two bordered arms on S3-G2):
    equal counts with and without `initial_step = T`, with more rejected steps in the second run.

  The chemistry rejections are covered by the S4S allocation pair, whose rungs differ in rejected steps.
- **G6 coverage:** PLAN.json equals the preregistered plan (`ladder.expected_plan`, with no override),
  and every item has a row with a typed status. The typed statuses are:
  - OK, NUMERICAL_FAILURE, TIMEOUT (the run's own cap);
  - DEADLINE (cut at a deadline-capped timeout, with the effective timeout recorded);
  - HOLD, ADAPTER_FAILURE, NOT_APPLICABLE (a priori), NOT_RUN, INVALID_PARITY.

  No VigilODE or reference item of a main case may be NOT_RUN, DEADLINE or ENVIRONMENT_FAILURE.
- **G7 provenance:**
  - the source subset's git blob ids equal the EXEC ls-tree, both at the start and at the end;
  - the binaries, Cargo.lock, the Julia project files and the diffsol lock are unchanged between the start
    and the end;
  - Cargo.toml and Cargo.lock are never edited (diffsol is a standalone crate).

**Verdict.**

- **INVALID** (no ledger row) when the run is incomplete or its infrastructure failed:
  - G6 or G7 fail;
  - PARITY.json is missing or partial;
  - a preregistered VG group or a VigilODE/reference item is missing, NOT_RUN, DEADLINE or
    ENVIRONMENT_FAILURE;
  - the setup or the runner build fails;
  - the plan is unofficial.
- **PASS** iff G1–G7 all pass.
- **FAIL** when a completed [C] measurement misses its threshold.
- **Comparator parity:** an adapter whose parity exceeds 1e-13 makes its arms INVALID_PARITY for that case.
  The rows are preserved. This does not affect the campaign verdict.

## [H] reported only

These never decide the verdict:

- comparator errors, unfloored errors, matched rungs (censored flags) and counters;
- the work-precision table;
- the ABBA cost ratios:
  - pairing: per main case, the fastest matched VigilODE arm against the fastest matched arm of each
    comparator family;
  - blocks: 2 warm-up + 7 confirmation, alternating AB/BA on one pinned core, comparing in-process solve
    times (median, range, and counts per status);
  - matching: the pairs are **budget-matched**, not error-matched. Both errors and their ratio are
    recorded, and censored matches are flagged;
  - timing scopes differ. VigilODE times the runner's second in-process integrate call, including the
    problem build and output; scipy times the integration call; Julia times `solve` after warm-up;
    diffsol times solver construction plus `solve_dense`;
  - skip rule: a pair whose matched ladder run took more than 120 s is skipped (NOT_RUN), because 9
    blocks would not fit the window;
- telemetry;
- the Julia AD Jacobian against the reference Jacobian (n ≤ 3000);
- the runner's initial state against the Python one;
- setup times.

## Host, budget and timeline (≤ 120 min wall from the start on NCP; enforced)

**Preflight gates.** The launcher stops and reports, without launching, if any of these fails:

- the effective cpuset must contain 8–63;
- no live C1 `run_campaign.sh`;
- no foreign process at ≥ 25 % CPU in a 1 s `top` sample;
- 1-min load < 4;
- MemAvailable ≥ 48 GiB: 16 Julia servers × 1.5 GiB + 36 workers × 0.4 GiB + margin;
- `/root` free ≥ 20 GiB + the measured size of the Julia depot to be copied.

A host-key mismatch is reported as such (exit 5), and ssh runs with `BatchMode=yes`.

| minutes | phase | cores |
|---|---|---|
| 0–20 (cap) | verify source; build the runners (-j 24); in parallel, build diffsol (-j 16, 8 min cap, lock seeds merged/archived only, else HOLD) and copy the Julia depot privately (after a du-vs-free check, 10 min cap), then load the packages (12 precompile tasks, 12 min cap, else HOLD) | 8–63 |
| –25 | parity: VigilODE probes; Python, Julia (private depot, archived project) and diffsol RHS, all pinned | 8–15 |
| –70 | references first and alone, then ladders; VG-1/VG-3 in parallel | 8–59 (16 pinned Julia servers); VG on 60–63 |
| –105 | ABBA | 8–59 |
| 112 | measurement end: the watchdog TERMs the driver, which kills the phase drivers and every registered child session (solvers, Julia servers, diffsol) | — |
| 112–120 | finalize: analysis (clipped to the time left), hashes, OUT.tar.gz (temp name, then rename), CAMPAIGN_DONE | 8–63 |

**Rules that apply throughout.**

- The campaign script runs under `taskset -c 8-63`, so every descendant inherits the mask; cores 0–7 stay
  free.
- BLAS uses 1 thread.
- Every run's timeout is min(its own cap, the phase deadline − 15 s), so nothing runs past 120 min.
- Every row is fsync'ed. On abort the EXIT trap analyses the partial data (verdict INVALID) and packs it.
- Campaign outputs go only to CAMP. Cargo updates its home cache. The Julia temp files go to CAMP/tmp.

## Ledger import

The steps after a run:

1. `fetch_results.sh` copies OUT.tar.gz back to `~/vigilode-campaign/bassrei-ncp-c1-<UTC>/ncp/` and
   verifies it.
2. `import_results.py` copies the following into `research/bassrei_ncp_wp_c1_20261022/`:
   - `results.json`, the VERDICT;
   - `campaign/`, holding PLAN, PARITY, LADDER, VG, ABBA(_PLAN), WORK_PRECISION, MATCHED, ABBA_SUMMARY,
     IDENTITY(_END), SETUP, DIFFSOL/JULIA_SETUP, the full diffsol lock, REPORT.md, OUTPUT_SHA256SUMS.txt
     (the raw manifest), IDENTITY_LOCAL.json, SOURCE_LS_TREE.txt and the tarball's sha256 file.
3. `import_results.py` also writes `IMPORT_MANIFEST.json`, which holds the sha256 of each file (the ledger
   row's outputs_sha256) and the tarball's local path and sha256.

The rows store CAMP-relative stdout paths, and the raw stdout files are inside the tarball. A copied-back
directory therefore re-scores identically with `analyze.py`.

## Design history (disclosed)

The harness was developed and dry-run locally on development code, never on NCP. Every value below was
seen before freezing.

1. **The first harness, worktree at d0de4f34.**
   - **Dry run:** S1-G1, S3-G1 and S4N1 at rtol {1e-4, 1e-8}.
     - The S3-G1 VigilODE bordered arms gave **1.13e-6 at rtol 1e-4, which exceeds B_app**, and 7.5e-10
       at 1e-8.
     - S1-G1 gave 6.0e-7 at 1e-4 and 3.9e-10 at 1e-8.
     - S4N1 gave 8.1e-9 at 1e-4 and 1.2e-10 at 1e-8.
   - **Parity, all 12 cases:**
     - the Python references are ≤ 9.8e-16;
     - the Julia ports are bitwise, except S4S at 8.5e-16;
     - the diffsol 0.16.1 stand-in is bitwise;
     - the Julia AD Jacobian matches to ≤ 5.5e-16.
2. **The main-case pilot (camp_heavy).** Full 8-rung ladders of every non-Julia arm on S2-G3, S2-G4, S4S and
   S4R (4 of the 9 main cases), with 20 jobs in parallel on the 24-core workstation.
   - **G1 u_ref:** 9.8e-14, 1.0e-13, 4.3e-14 and 1.8e-14.
   - **G3 at 1e-8:**
     - S2-G3: 1.6e-12–2.0e-12;
     - S2-G4: 2.6e-12–2.7e-12;
     - S4S: 4.0e-10;
     - S4R: 1.5e-10.

     These are margins of 2500×–6e5×.
   - **Matched rungs (under the old D/ρ floor):**

     | case | VigilODE (error) | comparators (error) |
     |---|---|---|
     | S2-G3 | 1e-3 (5.4e-8) | VODE MF24/25 1e-5 (6.3e-7); DOP853 1e-4 (1.8e-9); diffsol 1e-5 (5.4e-7) |
     | S2-G4 | 1e-3 (6.3e-8) | VODE 1e-4 (8.4e-7) |
     | S4S | 1e-4 (6.5e-7) | VODE MF21/22 1e-7 (9.2e-7); DOP853 1e-8 (1.5e-7); diffsol 1e-8 (1.3e-7) |
     | S4R | 1e-3 (4.5e-8) | VODE MF21 1e-7 (9.6e-8); MF22 1e-10 (8.8e-11); diffsol 1e-6 (8.9e-7) |

     The VigilODE matches at 1e-3 sit at the ladder's loose end and are now flagged as censored.
   - **S2-G4 reference cost:** the VigilODE banded 1e-12 reference took 171 s wall when run alone on an idle
     machine. In the pilot, with 20 jobs in parallel, it took 508.7 s wall (503 s user), and Radau took
     486 s. Under that load, single jobs slow down about 3×.
   - **Parameters changed after this pilot:**
     - the reference cap, raised from 600 s to 1800 s;
     - reference scheduling: the references now run first and alone;
     - the D/ρ floor, tightened (item 3).

     No gate threshold was loosened.
3. **The pre-freeze adversarial review (3 lenses).** It drove these changes, each of which tightens a check
   or fixes operations:
   - **D/ρ floor:** the absolute 1e-2 floor was 500–2300× the signal on S2/S3. It is now case-scaled (see
     the floors table), and D_γ is added as an observable. Unfloored errors are reported, and a NaN
     error counts as +∞.
   - **INVALID verdict:** incomplete runs and infrastructure failures now give INVALID, not FAIL.
   - **Completeness:**
     - G4 and G5 require the preregistered group lists, and G4 requires distinct cores and validated
       outputs;
     - G5 gains the forced-rejection groups (local pilot: 0 → 3 rejected steps on S1-G2 and 0 → 5 on
       S3-G2, with equal counts; S4N1 showed no rejection with `initial_step = T`, so it is not used);
     - G6 requires the preregistered plan;
     - G7 adds the end-of-run tree check and the Julia/diffsol lock hashes.
   - **Deadline enforcement:**
     - every timeout is clipped and typed DEADLINE when the deadline cut it;
     - the measurement ends at 112 min, with an 8-min finalize reserve;
     - the Julia READY wait is clipped;
     - the driver kills every registered child session (they run in their own sessions) on abort or at
       the deadline.
   - **Cores and Julia:**
     - cores 0–7 are really reserved (affinity inherited, BLAS 1 thread, pinned parity);
     - Julia parity uses the private depot copy (a single JULIA_DEPOT_PATH entry, never stacked);
     - Julia warm-up uses the timed solve's callback type;
     - the Julia server count is 16, consistent between this document and the harness.
   - **Launch and fetch:**
     - a real detach (verified locally: the start returns in 1.0 s while the job runs);
     - the preflight gates above;
     - host-key reporting;
     - fetch liveness by PID file and `kill -0`, with ssh 255 kept distinct;
     - OUT.tar.gz is complete when fetched.
   - **Ledger readiness:** the EXEC-pushed check, the import step and the relative evidence paths.
   - **DOP853 arm:** relabelled and cited (see Arms).
   - **diffsol provenance:** no unpinned lock resolution, and the full lock is kept.
   - **Parity isolation:** a Julia or diffsol exception now invalidates only that adapter.
4. **The dry run after the fixes**, on the 6d5ba7ff tree with its rebuilt runners, through
   `run_campaign.sh`. Locally Julia and diffsol take their HOLD paths.
   - **Plan:** S1-G1, S3-G1, S4N1 and S2-G4 at rtol {1e-4, 1e-8}.
   - **Outcome:**
     - wall 303 s;
     - the verdict is INVALID only because the plan is unofficial (dry-run overrides);
     - G1–G5 and G7 pass.
   - **G1:**
     - u_ref is 1.1e-14 on S4N1 and 3.5e-12 on S2-G4 (the new floor);
     - the S2-G4 references, run alone, took 130 s (VigilODE banded) and 102 s (Radau).
   - **G3 at 1e-8:**
     - S4N1: 1.24e-10 (margin 8045);
     - S2-G4: 2.05e-9 (margin 487), under the new D/ρ floor (2.6e-12 under the old one).
   - **G4/G5:**
     - 8 identity groups on 3 distinct cores;
     - 16 allocation and rejection groups pass (rejections 0 → 3 and 0 → 5, with equal counts).
   - **Matched rungs:**
     - on S2-G4, VigilODE matched at 1e-4 (the reduced ladder) and VODE MF24/25 at 1e-8 (1e-4 under the
       old floor);
     - on S3-G1, no comparator matches at either rung under the new floor.
   - **Re-scoring:** the extracted tarball reproduces VERDICT exactly.
   - **Parity**, with a private empty depot and the 0.16.1 stand-in rebuilt on 6d5ba7ff: unchanged, and
     nothing was written to `~/.julia`.
   - **Abort test:** a TERM 5 s into the ladder, with 7 registered child sessions:
     - finalize took 7 s;
     - no child and no campaign process was left;
     - the verdict was INVALID (references missing), and OUT.tar.gz was written.

## Deviations from the approved plan (for the user's sign-off)

1. **Dense VODE.** MF21/22 run only where n ≤ 512, so only S4 among the main cases. The G1 auxiliary
   cases cover S1–S3 as [H].
2. **Julia "sparse analytic Jacobian".** It is the coloured dual-number AD of the bitwise port: exact for
   the linear P02 operator, AD for S3/S4. CVODE Band uses its own DQ Jacobian. Rodas5P/FBDF use the
   default sparse LU, and CVODE KLU uses Sundials' KLU.
3. **VG-1/VG-3 placement.** They run during the ladder window, not at 105–120.
4. **DOP853 on stiff cases.** It runs 2 rungs with a 300 s cap.
5. **diffsol.** It is dense-only, so it is not run on G4.
6. **S3 grids.** S3 runs on G2 only; S3-G3 is dropped. The bordered dense LU arm needs n ≤ 2048, and the
   G3 bordered model is not in the P03 node.
7. **S4 grids.** S4 runs only on the P04 grid (16,16,16,8). The P04 chemistry is defined there, and S4
   is not run on G2/G3.
8. **No dense LU on G3.** G3 has n = 2114 > 2048, so it has no dense VigilODE arm and no dense-LU
   reference. The reference there is the banded LU at 1e-12, cross-checked by Radau.
9. **ABBA skip rule.** Pairs whose matched run took more than 120 s are skipped. The plan says 2 warm-up
   + 7 confirmation for every pair.
10. **ABBA matching.** The pairs are budget-matched, not error-matched: errors and ratios are reported,
    and censored matches are flagged.
11. **Julia servers.** There are 16 concurrent servers, about 24 GiB of RAM, inside the 48 GiB preflight
    requirement.
12. **D/ρ floor.** The case-scaled floor and the added D_γ observable tighten the P04 metric. P04's
    absolute D/E floor is still awaiting sign-off.
13. **DOP853 settings.** The tight DOP853 arm uses the source's rtol and step cap, but not its atol.
14. **diffsol lock.** The adapter lock is a merge of the checkout lock and the archived diffsol lock, or
    the archived lock alone. The full resolved lock is kept for provenance.

## Not changed (disclosed)

- **The double integration.** The runners at EXEC always integrate twice per call (a warm run, then the
  measured run). The references therefore cost 2 integrations. This is mitigated by the 1800 s cap and by
  running the references first and alone.
- **G3 stays at rung 1e-8** as a labelled sanity gate. Its pilot margins are disclosed above. No looser
  [C] rung was added after seeing the pilot.

## Holdout

None.

## Amendment 1 (before any measurement): attempt 1 INVALID in setup

Attempt 1 was launched at EXEC `6ed8f526` on 2026-10-22 at 06:33:59Z, into
`/root/vigilode-bassrei-ncp-c1-20261022T063359Z`.

- **Preflight passed:** cpuset 0-63, 64 CPUs, MemAvailable 124 GiB, `/root` free 478 GB, load 0.08, no
  live C1 run and no heavy process.
- **Setup failed after 1 s.** The detached non-login shell lacks `/root/.cargo/bin` on its PATH, so the
  runner build stopped with `cargo: command not found` (exit 127). As preregistered, the campaign
  finalized as **INVALID** (incomplete, no measurement phase ran, no ledger row).
- **Evidence:** the OUT tarball, its sha256, `campaign.log`, `REPORT.md`, the output sums and the
  preflight are kept in `attempts/attempt1_20261022T063359Z/`.

**Fix.** Commit `79a39b8e` changes only `run_campaign.sh`. It prepends `/root/.cargo/bin` to PATH when that
directory exists and exports `RUSTUP_TOOLCHAIN=1.94.1`. A read-only probe found rustup there, with
1.94.1 installed. No gate, threshold, scenario, arm or schedule changes.

Attempt 2 runs at the commit that adds this amendment (the new EXEC). Its 120-minute cap starts afresh at
its own setup, and attempt 1 used 9 s of NCP time.

## Amendment 2 (before attempt 3): user-authorized online Julia install

**Cause.** Attempt 2 (`/root/vigilode-bassrei-ncp-c1-20261022T063627Z`, EXEC `e3341309`) passed setup,
built the runners and diffsol, and passed parity. Its Julia setup returned **HOLD**: no depot with
OrdinaryDiffEq and Sundials was found. The only such depot on NCP had been
`/dev/shm/vigilode_ncp_intensive_120m_v1_20261021T062109Z/julia-depot`, on tmpfs, and the power cycle
before this campaign cleared it. The archived `env/vigilode-component-julia-depot-20261018` holds only a
Project, a Manifest and artifacts (56 KB). Under the offline rule this is the preregistered environment
HOLD, so attempt 2 continues unchanged and is scored by the unchanged `analyze.py`.

**Instruction.** On 2026-10-22 at 15:44 KST the user wrote: "julia는 너가 직접 설치하고 관련 비교 벤치마크까지
시행해" (install Julia yourself and run the related comparison benchmarks). That explicitly authorizes
network access for this one dependency, a **deviation from the offline-dependency rule** that is recorded
here.

**Install.**

- **Target:** a fresh private depot at `/root/vigilode-julia-c1-20261022/depot`, on persistent disk. It
  is a single `JULIA_DEPOT_PATH` entry, and nothing is stacked.
- **Method:** `Pkg.instantiate()` and `Pkg.precompile()` with Julia 1.12.7, using the harness's own pinned
  `external_audits/vigilode_ncp_intensive_120m_v1_20261021/harness/julia-env/{Project,Manifest}.toml`:
  - Project sha256 `984c9c52…a6c3306`;
  - Manifest sha256 `66296b7c…d9ce48`.

  The Manifest pins exact versions and tree hashes, so the installed packages equal the archived
  environment.
- **Execution:** run on cores 0-7 at nice 10, so it does not compete with attempt 2's cores 8-63. Timing is
  diagnostic only.
- **Evidence:** `install.log`, `ENV_SHA256SUMS.txt` and the start/end times are kept on NCP and copied
  back into this node.

**Harness change.** Commit `15e17192` lists that depot first among the Julia depot candidates, in
`setup_ncp.sh` and in the launch preflight. The campaign still copies the depot into its own directory
first. No gate, threshold, scenario, arm, rung, schedule or analysis changes.

**Attempt 3.** Attempt 3 is the full preregistered campaign, with all arms. It is launched from the
commit that adds this amendment, under three conditions:

- attempt 2 has finished;
- the install has logged `JULIA_INSTALL_OK`;
- the launch preflight passes, which requires no live C1 run and no heavy process.

**Ledger.**

- **Attempt 2:** its verdict goes to L-0154, with Julia HOLD.
- **Attempt 3:** its verdict goes to L-0155, with the full comparator set.
- Each verdict is computed by the unchanged `analyze.py` and never re-scored.
- If attempt 3 ends INVALID, that is recorded, and attempt 2 stands as the C1 result.

---

## Results

Three attempts were made. Attempt 1 was INVALID in setup (Amendment 1) and has no ledger row. Attempts 2
and 3 completed and were scored by the unchanged `analyze.py`. Both were fetched with sha256 verification
and imported verbatim into `attempt2/` and `attempt3/`, each with an `IMPORT_MANIFEST.json`.

| attempt | EXEC | NCP directory | wall | verdict | rows (OK / HOLD / TIMEOUT / n.a.) | tarball sha256 |
|---|---|---|---|---|---|---|
| 2 | `e3341309` | `…T063627Z` | 1034 s | **PASS** | 720 / 424 (all Julia) / 2 / 66 | `9aa4033b…35f032` |
| 3 | `2064366e` | `…T070004Z` | 1130 s | **PASS** | 1144 / 0 / 2 / 66 | `ebd3b90f…72c965` |

**Gates (both attempts): G1–G7 all True.**

- **G1:** references within limits.
- **G2:** every adapter matches the VigilODE kernels to ≤ 3.0e-15.
- **G3:** all 22 VigilODE rung items meet B_app at rtol 1e-8. The maximum error is 1.2e-8 (chemistry
  R), giving a minimum margin of 81×; most margins are 3e4–7e4×. The source ledgers hold.
- **G4:** 8 groups are bitwise identical across 3 cores.
- **G5:** 16 groups, the 6 forced-rejection groups among them, allocate the same number at both rtols.
- **G6:** the preregistered plan is complete; the 2 TIMEOUTs are DOP853 on S4R, typed.
- **G7:** provenance holds at start and end.

The Julia setup in attempt 3 was OK: it used a private copy of the user-authorized depot (Amendment 2;
evidence in `julia_install/`).

**Matched cost** (`diagnostic_only`, a single host, STATISTICAL_AUTHORITY_HOLD, no speed claim). The
ratios are comparator solve time divided by VigilODE solve time, as medians over 7 AB/BA blocks:

| case family | best comparator (matched rung) | ratio vs the best VigilODE arm |
|---|---|---|
| S1 non-stiff | Julia Tsit5 | 0.22–0.27: the explicit RK is about 4× faster |
| S4N1 non-stiff chemistry | Julia Tsit5 | 0.05: about 20× faster |
| S2 stiff transport, G2/G3/G4 | Julia CVODE_BDF Band | 2.9×, 5.8×, 15.9×: VigilODE structured client faster |
| S2, VigilODE banded vs Julia Rodas5P sparse (same method family) | n/a | about 1.8–1.9× (matched-rung times) |
| S3-G2 bordered | Julia FBDF sparse | 5.6× |
| S4R stiff chemistry | Julia CVODE_BDF KLU | 1.26× |
| S4S stiff chemistry | Julia CVODE_BDF KLU | 0.89×: CVODE about 1.1× faster |

Against the older comparators of attempt 2, the stiff ratios are:

- VODE BDF 18–32×;
- diffsol BDF 17–127×;
- DOP853 76–5800×.

**Reading of the cost results.** The VigilODE matched rungs are mostly censored at the loosest ladder rung
(1e-3), so the ratios are conservative in VigilODE's favour. The Jacobian strategies differ by arm (see
Deviations). n ≤ 8322.

**Verdicts.** Attempt 2 is recorded as L-0154 and attempt 3 as L-0155. Neither supersedes the other.
Broad production, live BASS/rei integration and every wall-time claim remain HOLD.
