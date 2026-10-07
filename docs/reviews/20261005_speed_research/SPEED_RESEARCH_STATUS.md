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
