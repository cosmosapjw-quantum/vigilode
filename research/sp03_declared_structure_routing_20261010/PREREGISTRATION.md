# Preregistration: routing by declared, validated problem structure (SP03)

Node SP03 of `research/reaudit_accuracy_speed_20261010/NEXT_DEVELOPMENT_DAG.json`, from the October 10 re-audit,
executed on branch `audit/rvj-accuracy-speed-wave1-20261010`. It depends on AS03 (fail-closed evidence validator).

**Claim boundary.**
- Same-binary callgrind instructions and counted work only; no wall-time claim.
- The router is opt-in: a new entry point, with no default change to any existing entry point.
- The existing banded path is not a general sparse LU, and no sparse claim is made.

## Question

The pilot and the speed cycles found two things:

- Wherever a band is declared, the banded direct driver beats every matrix-free arm by 15-270x in flops (pilot B1),
  and runs at 0.255x the dense driver's instructions at n = 400 (SPD03, L-0083).
- For n <= 8 the dense or small direct drivers beat every Krylov arm.

Yet `OdeProblem` carries no structure declaration, and the choice of driver is left to the caller. Can a router that
trusts only declared and validated structure do three things?

- Reject malformed or inconsistent declarations.
- Reproduce the direct drivers' results exactly.
- Beat the strongest applicable arm at matched accuracy, including assembly cost and without hidden O(n^2) work.

## Change

1. **`ProblemStructure` metadata in `problem.rs`:**
   - `Unstructured`;
   - `Dense`, for small n with an explicit Jacobian;
   - `Banded { lower, upper }`, attached through an explicit, validated constructor or builder. The validation checks
     bounds (`lower, upper < n`), that an explicit band Jacobian callback is present, and dimension consistency.
2. **Band verification at the first attempt.**
   - The router compares the band callback's J v with the problem's own JVP, or its dense Jacobian when only that is
     available.
   - It does so on k = 2 seeded random vectors at (t0, y0).
   - It requires a relative mismatch of at most 1e-12 times the norm.
   - The verification work is charged.
   - A mismatch rejects the declaration: the run is refused with a typed error. It never silently falls back.
3. **Router entry point** (`integrate_rodas5p_routed_observed`, in `rodas5p_fast.rs` or a new module). In order:
   1. A declared band goes to the banded fast driver.
   2. Otherwise, `Dense` with n <= 8 goes to the dense fast driver.
   3. Otherwise, a problem with an explicit Jacobian goes to the dense fast driver.
   4. Otherwise, `Unstructured` with a JVP goes to the U-form matrix-free driver (Legacy).

   The result records the routing reason and any fallback.
4. **CLI.** `rodas5p-cli` `stiff_benchmark.rs` gets a `rodas5p-routed` arm for profiling. It declares the band of the
   Brusselator problems, and declares `Dense` for robertson, hires and van-der-pol-mu1000.

## Cells

- **Brusselator-1D** with N = 50, 160, 200 and 500 (n = 100, 320, 400, 1000), at rtol 1e-6 and 1e-8. The band is
  declared.
- **Robertson, HIRES and van-der-pol-mu1000** at 1e-6 and 1e-8. `Dense` is declared.
- **Brusselator-1D-160 at 1e-6** with no declaration: the matrix-free route.
- **Malformed declarations** (validation tests):
  - a band outside the matrix;
  - a declared band narrower than the true Jacobian, which band verification must reject;
  - a missing band callback;
  - a dimension mismatch.

**Arms per cell:**
- `routed`;
- `dense` (the dense fast driver; v2);
- `dense-colext64` (SPD09);
- `banded` (direct call);
- for the Brusselators, also the U-form matrix-free `Legacy` arm.

Error is measured against the references used by `stiff_benchmark.rs`: `NATIVE.json` for N = 50 and 200; for the
others, a dense run at rtol 1e-13.

## Commands

    SP03_RUNS=research/sp03_declared_structure_routing_20261010/RUNS.json cargo test --release -p rodas5p-integrators --locked --test declared_structure_routing -- --ignored --nocapture --test-threads=1 export_runs
    cargo test --locked -p rodas5p-integrators --test declared_structure_routing
    python3 tools/sp03_routing_profile.py --output research/sp03_declared_structure_routing_20261010/PROFILE.json
    python3 tools/sp03_routing_check.py --runs research/sp03_declared_structure_routing_20261010/RUNS.json --profile research/sp03_declared_structure_routing_20261010/PROFILE.json --output research/sp03_declared_structure_routing_20261010/RESULTS.json

**Profiling.** The profile tool runs callgrind on the release CLI with `stiff-profile-run` (2-minus-1 repetitions,
plus a determinism repeat). It profiles `routed`, `banded`, `dense` and `dense-colext64` on every Brusselator cell,
and `routed` and `dense` on the small problems.

**Checker.** The checker calls `tools/evidence_schema_v2.py` first.

## Gate

**PASS** if all of the following hold.

1. **Validation.** Every malformed declaration is rejected with a typed error before any integration step. A correct
   declaration is accepted.
2. **Parity.** `routed` is bitwise identical to its target driver called directly (states, attempts, counters except
   the charged verification JVPs), in every cell. On the Brusselators, `banded` (hence `routed`) is bitwise identical
   to `dense`, as SPD03 found.
3. **Strongest applicable arm.** On the Brusselators, `routed` Ir per trajectory is:
   - <= 0.60x min(`dense`, `dense-colext64`) at n = 400;
   - <= 1.00x min(`dense`, `dense-colext64`) at n = 100 and n = 320;
   - <= 0.05x matrix-free `Legacy`.
4. **No hidden O(n^2).** The `routed` Ir per accepted step grows with a log-log slope of at most 1.15 from n = 400 to
   n = 1000.
5. **Routing overhead.** `routed` Ir per trajectory is <= 1.02x its direct target's on every cell, verification
   included.

Everything else is **FAIL**, with every number preserved.

**Reported:** the routing reasons; the verification cost; the small-problem cells, where `routed` equals `dense`
apart from the verification.

**Predictions.**
- Gate 3: about 0.26x at n = 400 (SPD03: 0.255x of dense v2), about 0.62x at n = 100, and about 0.003-0.03x of
  matrix-free.
- Gate 4: slope about 1.0 (SPD03: 1.003-1.011).
- Gate 5: verification costs 2 J v products plus 2 banded products, which is negligible.

## Prior information

- SPD03 (L-0083) banded arm; SPD09 (L-0086) colext64; pilot B1/B2 structural rivals.
- No code of this node exists before this commit.

## Results

Appended after the recorded run; the registered text above is unchanged. Implementation `7405fe2`, profile tool and
checker `f3a5a8a` (committed before any recorded run). The recorded commands ran on `f3a5a8a` with a clean tracked
worktree, `RAYON_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1`: the release CLI with line tables (sha256
`c623ee253fe83ff64edcbe97fa94b85f0d025f1459b3fc98e81a1d0074fea047`), valgrind 3.22.0. Outputs: `RUNS.json`,
`PROFILE.json` and `RESULTS.json`. The non-ignored test command passed (5 passed, 1 ignored). Every callgrind total was
deterministic. The evidence passed the AS03-based schema, so the verdict is not INVALID. Every profiled CLI run equals
the library run of the same arm and cell in attempts, accepted steps and final state, bit for bit.

**Gate: FAIL.** Item 5 fails at n = 1000. Items 1-4 hold.

| Gate item | Outcome |
|---|---|
| 1. Validation | **Holds.** All 10 malformed declarations are refused with their typed error, with no right-hand-side call and no step: band outside the matrix (lower; upper), a band narrower than the Jacobian (dense-Jacobian reference; JVP reference), a missing band callback, a dimension mismatch (band; dense), and, reported in addition, Dense without an explicit Jacobian, a callback without a band, and one in-band value off by a relative 1e-9. The declaration-time errors come from the builder, and the band errors from verification before any step. The 3 correct declarations are accepted and integrate successfully. |
| 2. Parity | **Holds.** In all 15 cells `routed` equals its target bit for bit: output times, final state, attempts, step counts, driver id. Its counters equal the target's once the recorded verification charge (2 `jacobian_builds`, 4 `jacobian_matvecs`) is subtracted. The route is the registered one in every cell. On all 8 Brusselator cells `banded` equals `dense` bit for bit, counters included. |
| 3. Strongest applicable arm | **Holds.** routed / min(dense, dense-colext64) = **0.487 / 0.485** at n = 400 (rtol 1e-6 / 1e-8; bound 0.60), 0.790 / 0.790 at n = 100 and 0.542 / 0.540 at n = 320 (bound 1.00). routed / legacy = 0.0153 / 0.0152 (n = 100), 0.0097 / 0.0148 (n = 320), 0.0085 / 0.0134 (n = 400) and 0.0014 / 0.0038 (n = 1000); the bound is 0.05. |
| 4. No hidden O(n^2) | **Holds.** The routed Ir per accepted step grows with a slope of 1.057 / 1.031 from n = 400 to n = 1000 (bound 1.15). The banded arm alone has 0.997 / 1.002. |
| 5. Routing overhead | **Fails.** routed / banded = 1.0095, 1.0046 (n = 100), 1.0136, 1.0065 (n = 320), 1.0165, 1.0078 (n = 400), and **1.0738, 1.0347 (n = 1000)**, against a bound of 1.02. routed / dense is 1.0001-1.0024 on the six small cells, and routed / legacy = 1.0000005 on the undeclared cell. |

**Why item 5 fails.** The CLI Brusselators have no JVP, so the registered rule makes band verification compare against
the problem's **dense** Jacobian. Building and multiplying that Jacobian is a one-time O(n^2) cost. The difference
routed - banded (Ir per trajectory) is:

| n | routed - banded |
|---|---|
| 100 | 0.216 M |
| 320 | 0.981 M |
| 400 | 1.480 M |
| 1000 | 16.50 M |

The same cost appears at both tolerances. At n = 1000 the function attribution splits it into two parts:
- 8.08 M in `memset`, zeroing the 8 MB dense Jacobian;
- 8.35 M in the inlined verification, the two dense products.

The `memset` share depends on glibc's dynamic mmap threshold. In a first call in a fresh process, the allocation is
served by mmap without zeroing. Even without it the ratio is 1.038 at rtol 1e-6, so the item fails either way.

This one-time quadratic term is also what raises the slope of item 4 from about 1.00 (banded) to 1.03-1.06. It stays
under the bound only because it is spread over 82-190 accepted steps.

The prediction "2 J v products plus 2 banded products, negligible" assumed a JVP reference, which is O(n). That is a
statement about a problem with a JVP. It was not measured here and is not a result of this node.

**Reported.**
- **Routes.** `declared-band`, driver `rodas5p-fast-banded-v1`, on the 8 Brusselator cells. `declared-dense-small`,
  driver `rodas5p-fast-transformed-v2`, on robertson, hires and van-der-pol-mu1000. `unstructured-jvp`, driver
  `rodas5p-mf-fast-transformed-v1`, on the undeclared Brusselator-160. No fallback was recorded.
- **Verification.** It used the dense-Jacobian reference on every Brusselator cell, with relative mismatch 0 on both
  vectors (the band fill and the dense fill are the same expressions).
- **Small problems.** routed - dense is 1,194-1,251 Ir per trajectory, which is the routing decision and the record;
  the small problems have no verification.
- **Against dense v2 alone.** routed / dense = 0.259 / 0.257 at n = 400 (predicted about 0.26) and 0.622 / 0.622 at
  n = 100 (predicted about 0.62). Gate item 3 divides by the stronger dense-colext64 (SPD09), which is why its ratios
  are higher than these predictions.
- **Accuracy.**
  - Routed, banded, dense and dense-colext64 are bitwise identical, so their endpoint errors are equal (for example
    2.14e-6 and 2.2e-8 on the Brusselators).
  - Legacy matches their step counts and errors to the printed digits at n <= 400.
  - At n = 1000 Legacy takes 321 and 288 accepted steps instead of 82 and 190, with errors 1.08e-7 and 1.42e-8 instead
    of 2.14e-6 and 2.19e-8. The n = 1000 comparison with Legacy is therefore not at matched error. Legacy also costs
    700x and 270x more there.

**Deviations and disclosures.**
- **Legacy profiling.** The registered profiling sentence lists routed, banded, dense and dense-colext64. Gate item 3
  needs the Legacy Ir, so the profile tool also profiles `rodas5p-mf-legacy` on every Brusselator cell. The Legacy
  entry at (n = 320, 1e-6) is the direct target of the undeclared cell.
- **brusselator-1d-160.** It is not in `profile_problems()`. It is reachable from `stiff-profile-run` only through a
  new `routing_profile_problems()`, so no existing problem set changed.
- **Banded arm.** The small problems have no band, so their arms are routed, dense and dense-colext64.
- **Transcribed problems.** The integration test cannot link the CLI crate, so it transcribes the CLI problems. The
  checker binds the CLI and library runs, as above.
- **Rehearsal.** Before the recorded run, the RUNS export was run once to a scratch file. Its purpose was to test the
  checker's plumbing against a synthetic profile with made-up instruction counts. No callgrind number was seen before
  the recorded profile, and the recorded `RUNS.json` is byte-identical to that rehearsal.
- **`correct-dense-small`.** Its right-hand-side counter is not attached, so it reports `rhs_calls` 0 although the run
  integrates.

**Claim boundary.** Same-binary callgrind instructions and counted work only, with no wall-time claim. The router is
opt-in, the banded path is not a general sparse LU, and no default changed.
