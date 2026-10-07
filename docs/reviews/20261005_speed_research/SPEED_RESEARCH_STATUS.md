# Speed research cycle (2026-10-05 to 2026-10-07): status

Branch `audit/rvj-speed-research-20261005`, based on `4de1f88` (head of `audit/rvj-research-integration-20261004`).
Question: which methods reduce the cost of the RODAS5P solver, measured in a way that is admissible while the
timing authority is on HOLD?

**Nothing here is a wall-time claim.** The timing authority stays on HOLD (`docs/TIMING_DESIGN_CONTRACT.md`;
both pooled-pair coverage studies failed, L-0007/L-0010, and the session-cell interval, L-0018/L-0025, still waits for
its independent domain review). The admissible unit is callgrind instructions per attempted step (one integration
as the difference of 2 and 1 repetitions of `rodas5p stiff-profile-run`, with a repeated 1-repetition run as a
determinism check), plus `WorkCounters`, counted operations and allocation counts. Every default solver path is
unchanged; every change is opt-in.

## Method

1. **Map** (seven read-only readers): where the remaining instructions go in the fast drivers, the Krylov crate, the
   certified phi/polynomial paths, the measurement tooling and the timing authority; a digest of every recorded cost
   result. Before any registration the baseline was reproduced on this host: small-n driver on van der Pol
   3,568.8 instructions per attempt (L-0041 recorded 3,568.7).
2. **Hypothesize and refute** (six proposers with distinct lenses, then two independent refuters per candidate: prior
   art and code premise, measurement and research discipline): 22 candidates; 9 survived both refuters, 7 were
   contested, 4 refuted, 2 unverified (the verification agents hit a usage limit).
3. **Execute** the three strongest candidates as preregistered nodes (registration pushed before any code or run;
   base export recorded on the unmodified source), with bitwise identity gates against the legacy drivers.

## Executed nodes

| Node | Ledger | Verdict | Result (instructions per attempted step, same binary) |
|---|---|---|---|
| SPD01 fast-driver overhead (`spd01_fast_driver_overhead_20261005`) | L-0081 | **PASS** | Prevalidated controller (one `validate` per integration instead of per attempt, -171 Ir) and fused landing (one landing per attempt instead of three, -629..-678 Ir): small driver **3,580 -> 2,778** (van der Pol, 0.776x), 5,076 -> 4,264 (Robertson, 0.840x), 12,435 -> 11,586 (HIRES, 0.932x); dense v2 0.912-0.959x at n <= 8, 0.999x at n = 400. Bitwise identical on 56 points and six banded grid cases; 1,000,000-sample property test with 0 class A/B discrepancies |
| SPD02 column-extent LU (`spd02_lu_column_extents_20261005`) | L-0082 | **FAIL** | Bitwise identical on 35 points and 2,000 contract matrices; **3.90 M -> 2.02 M at n = 400 (0.517x)**, 0.773x at n = 100; but 1.036-1.062x at n <= 8 (gate <= 1.03) and the legacy path compiled 1.024x its base at n = 400 (gate +-2 %). Stays opt-in |
| SPD03 banded arm (`spd03_banded_arm_instructions_20261005`) | L-0083 | **PASS** | INT-03 banded pipeline through new CLI arms: **3.84 M -> 0.98 M at n = 400 (0.255x)**, 0.617x at n = 100, bitwise identical on 14 points; banded operations per attempt slope 1.011 (n = 100 to 400), 1.003 (to n = 1000). Secondary: a slices kernel is bitwise identical but 1.012x the indexed kernel (fails its <= 0.95 item) |

Where this leaves the drivers (cross-node, cross-binary comparison with the L-0030 profile of Hairer's RODAS, not a
gate of any node):

| Problem | Best before this cycle | Best after | Hairer RODAS (L-0030) |
|---|---|---|---|
| van der Pol (n = 2) | 3,569 (small driver, L-0041) | **2,778** (small + SPD01 `-ovh`) | 2,947 |
| HIRES (n = 8) | 12,302 (small driver) | **11,586** (small + SPD01 `-ovh`) | 9,157 |
| Brusselator 1-D (n = 400) | 3.74-3.84 M (v2, L-0033) | **0.98 M** (banded arm, SPD03) | 8.83 M |

## What did not work, and why

- **Column extents at n <= 8** (SPD02): the extent initialisation and maintenance cost more than the scans they remove
  when the dense LU itself is a few hundred instructions. At n = 400 the method halves the instruction count; a size
  threshold is a candidate for a follow-up node, not a post-hoc change.
- **The slices kernel** (SPD03 part B): the band rows are 2-5 entries long; slice setup costs as much as the bounds
  checks it replaces.
- **Refuted before execution**: compile-time stage loops for the small driver (OVH-STATIC-STAGE; the inner loops are
  already unrolled and SSE2-vectorized in the measured binary), a selective second Gram-Schmidt pass
  (KRY-MGS-SELECTIVE; on `W = I - h gamma J` the DGKS skip test fires on 97-100 % of Arnoldi columns, so nothing is
  saved), a new cache-simulation tool and a separate unprofiled-arms node (both duplicate `tools/speed_profile.py`
  and SPD03).

## Next-cycle DAG (ranked from the adversarial verdicts; not preregistered)

| Rank | Candidate | Verdict | Mechanism and corrected expectation |
|---|---|---|---|
| 1 | KRY-AUG-LS-ONCE | survives (0.85, 0.80) | The augmented Arnoldi routine shared by LGMRES re-solves the small least-squares problem after every column and uses only the last solution (`gmres.rs:110-187`); solving once per cycle should cut LGMRES `solve_into` allocations from 455-4,099 per solve to roughly 0.05-0.11x (L-0061 left exactly this). Allocation counts, bitwise identity |
| 2 | KRY-LS-NOALLOC | survives (0.75, 0.72) | `small::least_squares` allocates 11 times per call (faer column-pivoted QR in fresh storage); a workspace-owned variant should take the 48 non-HIRES `rnext02` GMRES families to 0 allocations per cycle after warm-up. Needs an explicit warm-up definition |
| 3 | SMALLN-STATIC-STAGES (corrected) | survives (0.70, 0.70) | Not the vectorized inner loops: the target is the outer `Vec<(usize, f64)>` coefficient loops, their `u[j]` bounds checks and per-entry `c/h` divisions, about 450-550 Ir per attempt at any n, plus solving directly into `u[i]` |
| 4 | KRY-STEP-X0 | survives (0.60, 0.65) | Stage-indexed warm start from the previous accepted step in the matrix-free U-form driver; 0.70-0.90x matvecs per trajectory expected on the Brusselators, about 1.0x on small problems |
| 5 | ENS-LANES | survives (0.70, 0.60) | Lane-batched (structure-of-arrays) small-n ensemble driver; ceiling about 0.42x per trajectory since clock/controller work is per member |
| 6 | SPD02 size threshold | new (from SPD02) | Column extents only above a dimension or density threshold, measured on the same five problems; plus attribution of the legacy arm's +2.4 % |
| - | CTRL-PREDICTIVE, DENSE-SAMPLE, KRY-STAGE-PROJ, CERT-IVPOINT, CERT-DOT2, CERT-MEMO, CERT-Q2-DIGEST-CAL | contested | Each needs the refuter's correction first (e.g. CERT-IVPOINT gives 8 -> 4 directed operations for point x interval, not 8 -> 2; CERT-DOT2 must treat exact zero products as exact; DENSE-SAMPLE is already implemented on the sequential path; CTRL-PREDICTIVE changes results and needs a matched-accuracy design) |

## What cannot be claimed

- Any wall-time speedup, on any problem: the timing authority is on HOLD. Instructions are not time; at n = 400 memory
  traffic may matter more than instruction count (the banded arm also stores n(3l + 2u + 2) slots instead of 2n^2).
- That the banded arm's gain transfers to problems without a declared band: SPD03 measured the 1-D Brusselator only.
- That the comparisons with Hairer's RODAS are like for like: they are cross-node and cross-binary (L-0030, an earlier
  binary and toolchain state) and are reported, never gated.

## Process notes

- All three nodes were registered and pushed before their code; the base export (`BASE_PROFILE.json`) was recorded on
  the unmodified source. Two amendments were made before the respective runs and are in the files: SPD01 (identity
  export moved into the CLI crate, call-count extraction added to the profile tool) and SPD03 (the same, plus a
  separate INT-03 kernel export).
- SPD01's first checker run (`RESULTS.json`, FAIL) failed on a profile row I had omitted from the registered command
  and on a checker tolerance that counted the driver's one per-integration `step_to` call as per-attempt work. Both
  were corrected (the omitted problem profiled on the same binary; integer call counts as the gate text says) and the
  corrected run is `RESULTS_CORRECTED.json`; the first run is kept. No measured number and no threshold changed.
- SPD02's LU contract test first compared the legacy extents after factorization with the new variant's initial
  extents; it was fixed before the test's recorded run (a test defect, disclosed in the results).
- The phase-2 synthesis agent did not complete (usage limit); the DAG above was synthesized from the recorded
  verdicts by the integrator.

## Independent review

A separate agent that wrote none of the code reviewed `git diff 4de1f88..a234aa3 -- crates/ tools/` read-only, with
its own counterexample searches (a Python transliteration of both LUs over 230,000 matrices with forced
cancellation, NaN and Inf; a transliteration of the landing helpers; a scratch binary against the real crate).
**No P0**: every existing entry point and default path keeps its arithmetic.

| # | Severity | Finding | Disposition |
|---|---|---|---|
| 1 | P2 | The fused landing (SPD01) is not identical below `h = 2^16 eps max(|t|, |tf|)`: a reviewer-built case (`t0 = 2^40`, unit step) takes 2 attempts instead of 1. Inside the exemption the gate registered, so the verdict stands, but "0 class A/B at any step size" overstated a sampling result | Correction appended to SPD01; the doc comment of `limit_landed_step` states both exceptions; `fused_landing` is not for promotion as is |
| 2 | P2 | The property-test classifier puts every discrepancy with a fired post-rejection shortening into class C without checking "within the residue of `tf`", and does not assert the `*_tiny` counts | Recorded in the SPD01 correction; recorded counts unchanged |
| 3 | P2 | `factor_slices` added its operation count at the end, so a factorization that failed partway dropped earlier columns from `BandedWork` | Fixed (counted per column, as in the indexed kernel); no recorded case affected |
| 4 | P2 | Adding `brusselator-1d-500` to `benchmark_problems()` made the SPD01/SPD02 identity exports non-reproducible at HEAD (42 rows instead of 35) | Fixed: moved to `profile_problems()`, used only by the profile runner and the band-fill test |
| 5 | P2 | `integrate_rodas5p_fast_banded_observed_with_kernel` lacked the band-inside-matrix check | Fixed |
| 6 | P3 | `Rodas5pFastLu` gained a variant and is not `#[non_exhaustive]` | Not changed (research API; disclosed) |
| 7 | P3 | The SPD02/SPD03 checkers do not gate `callgrind_deterministic`, and the contract-test results enter as command-line flags | Not changed after the runs; every recorded entry is deterministic (`callgrind_deterministic: true` in all profiles) |
| 8 | P3 | Carrying the second LU and the policy match costs the unchanged legacy path 0.6-2.4 % instructions (code layout; bitwise the same results) | Already disclosed in SPD02 (its gate item 5 failed on this) |

Checked and found sound by the reviewer: the column-extent invariant under swaps and fill-in (0 mismatches and
0 invariant violations in the search), the slices kernel's offsets and operation order, `extents_of`,
`accept_prechecked` (both call sites pass the finite-checked state), the strict-cap path of the fused landing, the
SPD01 checker change (a stricter integer check, not a relaxation), the honesty of SPD03's "Ir per counted
operation", and that L-0081..L-0083 match their recorded JSON.

## Validation

The full matrix ran on `a234aa3` (all three nodes recorded). Steps 1-9 passed. Step 10 was stopped by the 2-hour
limit of the background job while two long recorded-run exporters were still running. The review fixes then
produced `77005d2`. On that head:

- re-run: fmt, the four clippy configurations, and the tests of the two changed crates (`rodas5p-integrators`,
  `rodas5p-cli`, all targets: 541 passed, 0 failed);
- run on their own: step 10 and steps 11-16.

| Step | Head | Result |
|---|---|---|
| `cargo fmt --all -- --check` | `a234aa3`, `77005d2` | pass |
| clippy `-D warnings`: workspace, `audit2-research`, `audit2-bateman-authority`, `audit2-stage-certificate` | `a234aa3`, `77005d2` | pass |
| `cargo test --workspace --all-targets` | `a234aa3` | pass |
| `cargo test -p rodas5p-integrators -p rodas5p-cli --all-targets` | `77005d2` | pass (541 tests) |
| `cargo test -p rodas5p-integrators --all-targets`, three feature configurations | `a234aa3` | pass (not re-run after the review fixes) |
| `cargo test --workspace --profile measurement -- --ignored` | `77005d2` | pass (50 ignored tests in 226 binaries) |
| `tools/check-audit2-readiness.sh` | `77005d2` | pass |
| `tools/check-research-node.py --base origin/audit/rvj-research-integration-20261004` | `77005d2` | pass (83 rows; 3 new nodes) |
| `tools/check-authority-refs.py` | `77005d2` | pass |
| `tools/check_ignored_tests_in_ci.py`, `tools/test_*.py`, `research/rvj_integration_20261004/validate_bundle.py` | `77005d2` | pass |

The three feature-configuration test runs were not repeated after the review fixes. The fixes touch only the opt-in
banded slices kernel, a doc comment, one input check and the CLI problem list, and the four clippy configurations
compiled them under every feature set.

# Second cycle (2026-10-07): SPD04-SPD09

The six candidates of the next-cycle DAG above were registered together (`6f504c0`, pushed before any code), with
base exports recorded on the unmodified solver source for the two nodes that needed one (SPD07 `BASE.json`,
SPD08 `BASE_ENSEMBLE.json`). Three implementers worked in separate worktrees (Krylov: SPD04/SPD05; fast drivers:
SPD06/SPD09; ensemble: SPD08); SPD07 was implemented by the integrator. Each node was measured once on committed
code; the same claim discipline applies (counted instructions, counters and allocator events only; no wall-time
claim).

## Executed nodes

| Node | Ledger | Verdict | Result |
|---|---|---|---|
| SPD04 allocation-free least squares (`spd04_ls_workspace_20261007`) | L-0087 | **PASS** | A workspace-owned faer in-place column-pivoted QR with the high-level path's parameters is bitwise identical on 1,024 contract systems and 408 GMRES-into solves and removes exactly 11 allocations per least-squares solve; matrix-free driver allocations per attempt 0.018-0.096x on five problems (HIRES 0.48x: its JVP allocates) |
| SPD05 one least-squares solve per LGMRES-into cycle (`spd05_lgmres_ls_once_20261007`) | L-0088 | **PASS** | Bitwise identical on all 1,216 rev04 solves (344 failures rolled back); allocations per solve 0.027-0.042x of L-0061 (CDR 4,059 -> 110, Brusselator 455.7 -> 15.4); with SPD04's workspace too, 3.0-4.7 per solve |
| SPD06 fixed stage structure, small driver (`spd06_small_static_stages_20261007`) | L-0085 | **PASS** | Bitwise identical on 21 points and the ensemble; Ir per attempt **0.838x** (van der Pol), 0.848x (Robertson), **0.736x** (HIRES) of the legacy arm in the same binary; 0.882x / 0.872x / 0.737x of the base binary. The index-loop solves give most of the HIRES gain (stages alone 0.951x) |
| SPD07 stage-indexed warm start, matrix-free driver (`spd07_mf_step_warm_start_20261007`) | L-0084 | **FAIL** | Starting stage i from stage i of the last accepted step: 0.963-1.001x the linear matvecs of the default start on the Brusselators (gate <= 0.85) and 1.03-1.05x on Robertson and van der Pol |
| SPD08 lane-batched ensemble (`spd08_small_ensemble_lanes_20261007`) | L-0089 | **FAIL (kill)** | All 64 members bitwise identical, failures masked per lane, but 0.924x (B = 8) / 0.937x (B = 4) Ir per trajectory (gate <= 0.80, kill > 0.90); the scalar arm drifted +8.6 % from its base export |
| SPD09 column extents above n = 64 (`spd09_colext_threshold_20261007`) | L-0086 | **PASS** | Bitwise identical on 37 points; 1.0000-1.0001x at n = 2, 3, 8, 60; **0.833x** (n = 80), 0.787x (n = 100), **0.531x** (n = 400) |

Where the drivers stand now (cross-node, cross-binary; reported, never gated):

| Problem | Before the speed research | After cycle 1 | After cycle 2 | Hairer RODAS (L-0030) |
|---|---|---|---|---|
| van der Pol (n = 2) | 3,569 (small driver) | 2,778 (SPD01 `-ovh`) | **2,348** (SPD06 `-static-ovh`; 2,955 for `-ovh` in the same binary) | 2,947 |
| HIRES (n = 8) | 12,302 | 11,586 | **8,224** (SPD06 `-static-ovh`) | 9,157 |
| Brusselator 1-D (n = 100, dense storage) | 395 k (v2) | 306 k (SPD02 colext, opt-in) | 316 k (SPD09 colext64, no small-n loss) | - |
| Brusselator 1-D (n = 400) | 3.74-3.84 M (v2) | 0.98 M (banded arm) | 0.98 M (banded); 2.03 M dense storage (SPD09) | 8.83 M |

## What did not work, and why

- **Step-indexed warm start** (SPD07): iterations per stage solve fall by at most 4 % (Brusselator-160 at 1e-8:
  50.6 -> 48.5), so the last step's stages are not a much better start than the previous stage of the same attempt,
  and on the small problems the extra true-residual matvec of a nonzero stage-0 start dominates. Reported: the
  driver's default `Previous` start costs 1.02-1.57x the matvecs of a zero start in all 14 cases; zero is the
  cheapest start everywhere. Changing the default is a separate question (its own node, with GCRO-DR and LGMRES
  measured separately).
- **Lane batching at N = 2** (SPD08): batching the elementwise stage arithmetic saves about 250-300 Ir per attempt
  out of about 3,900; the per-lane LU, solves, controller and output clock dominate. The predicted 0.65-0.75x was
  wrong; the direction is closed for N = 2.
- **The legacy arms drift**: carrying the new opt-in code moved the unchanged default paths by +0.3 to +5.2 %
  (small driver) and +0.6 to +2.2 % (dense driver) in instructions per attempt, with bitwise identical results. The
  cause is unverified (code layout of the whole binary; the dense arm drifted although its file barely changed).
  Gates measured against the same binary carry this drift in their denominators; cross-binary figures are given
  next to them.

## Next steps (not preregistered)

| Candidate | Why |
|---|---|
| Zero start as the matrix-free driver's default (GMRES-into) | SPD07's reported finding: 1.02-1.57x fewer matvecs than `Previous` in 14/14 cases with equal attempts and errors; needs its own registration and the LGMRES/GCRO-DR interaction measured |
| Wire SPD04's workspace into the matrix-free driver by default, and SPD05 into a driver | Both are bitwise identical; the remaining question is adoption, not effect |
| Column-extent threshold below 64 | SPD09 reports 0.888x at n = 60 for the always-on variant; the crossover between n = 8 and 60 is unmeasured |
| Keep the legacy small-driver function separate from the static variants | Would test whether the +5.2 % drift of the default small driver is the shared generic function (unverified) |
| Promote SPD06 + SPD01 options to the small driver's default | 2,348 Ir per attempt on van der Pol, below Hairer's RODAS (2,947, cross-binary); needs a promotion review (SPD01's fused landing has a documented exception below `h = 2^16 eps |t|`) |

## What cannot be claimed (cycle 2)

- Any wall-time effect (timing authority on HOLD). Allocation removal (SPD04/SPD05) is an allocator-event count, not
  a time.
- SPD05's result for any driver: no driver calls `solve_lgmres_into`.
- SPD04's identity for a faer version other than 0.24.4, or another global parallelism setting.
- That the batch driver (SPD08) or the static stages (SPD06) generalize beyond N = 2, 3, 8 and the van der Pol
  ensemble.

## Independent review (cycle 2)

A separate agent that wrote none of the code reviewed `6f504c0..9aab6f3` read-only, ran the new non-ignored tests in
its own worktree, and probed `LeastSquaresWorkspace` with 200 further tall and shrinking shapes in one reused
workspace (all bitwise equal). **No blocking finding**; every default path keeps its results bit for bit; all six
verdicts are supported by the recorded JSON.

| # | Severity | Finding | Disposition |
|---|---|---|---|
| 1 | should-fix | SPD06/SPD08 state as fact that the legacy small arm's drift comes from the shared generic function; the dense arm drifted 1.4-2.2 % in the same binary, so the cause is unverified | Corrections appended to SPD06 and SPD08 |
| 2 | should-fix | SPD07: the doc comment claimed every other stage solver refuses the step-indexed starts (only the sequential one does); a reused workspace kept the previous integration's stages; the test only checked that the work changes | Fixed in `c26a235` (record cleared per integration, contract stated, a test requiring zero iterations on the recorded system); recorded runs unaffected (fresh workspace per integration); correction appended |
| 3 | should-fix | Three reported (not gated) numbers misstated: SPD07 scaled-arm minimum 0.958 (not 0.963), SPD05 workspace ratio minimum 0.0009 (not 0.002), SPD08 "batch8 1.005x of base" mixed two protocols (1.0008x with one protocol) | Corrections appended; the ledger rows are append-only and keep their text, the node files carry the corrected figures |
| 4 | minor | SPD04 checker exempts whole solves with growth and does not gate the contract's growth flag; SPD06 gate item 2 relies on a `--contract-passed` flag; SPD05's reading of "non-failing sequence" undisclosed; SPD07 checker stricter on exclusions; L-0084 claim omits Prothero-Robinson 1e-8; no n = 64/65 boundary test in SPD09 | Disclosed in the corrections; no verdict depends on them (the SPD09 boundary is `n > 64` in both places, checked by reading) |

Checked and found sound by the reviewer: the SPD04 bitwise argument against the faer 0.24.4 source (parameters,
`split_LU`, `Mat` stride and alignment, zeroing of every reused buffer, monotone growth, error paths); SPD05's
ls-once routine as the legacy loop without the per-column solve; that every default path is unchanged; SPD06's
operation order and tables check; SPD08's per-lane freshness, failure counters and reset; that no pre-existing test
file was modified; that all registrations are append-only; checker thresholds; and that every ledger input matches its
blob at the ledger commit.
