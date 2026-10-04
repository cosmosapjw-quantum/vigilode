# SAFE-RECYCLE — explicit GCRO-DR recycle policy in the U-form driver (prospective registration)

RVJ development DAG node `SAFE-RECYCLE`
(`research/rvj_integration_20261004/NEXT_DEVELOPMENT_DAG.json`), finding
`INTAKE-NATIVE-03`. Base commit `01535400` (head of
`audit/rvj-research-integration-20261004` when this node started).

## Question

The REV-01c repair (`refresh_after_update`, L-0059) restores
`M^-1 A U = C` after every recycle update, but the opt-in matrix-free U-form
driver (`rodas5p_matrix_free_fast.rs`) calls `solve_gcrodr_with_workspace`,
which uses the default options (no refresh). Can the driver take an explicit
recycle policy, with the old behaviour kept bit for bit under its own
identifier, the refresh work charged, and fewer recycle-induced stage-solve
failures at driver level?

## Base export (recorded before any source change)

`crates/rodas5p-integrators/tests/safe_recycle_policy.rs::export_legacy_base`
was run on the unmodified base source:

    SAFE_RECYCLE_BASE=research/safe_recycle_policy_20261004/BASE_LEGACY.json \
      cargo test --release -p rodas5p-integrators --locked \
      --test safe_recycle_policy -- --ignored --nocapture --test-threads=1

`BASE_LEGACY.json` sha256
`6b7eadba309c935b2d4d37e38cb1d041450d5db28f45a6e62744a0aa9112e6c0`:
7 problems (the six L-0038 cases and `brusselator-1d-160` on [0, 10]) x
rtol {1e-6, 1e-8}, GCRO-DR (recycle_dim 8, restart default), linear rtol
1e-10, atol 1e-14, no preconditioner, max_attempts 5000. Observed there
(not a gate input): linear-solve failures 2453/2475 on `brusselator-1d-50`
(both runs end at the attempt cap, success=false), 102/102 on
`brusselator-1d-160`, 0 elsewhere.

## Change (opt-in; default unchanged)

- `GcrodrRecyclePolicy { Legacy, RefreshAfterUpdate, Cold }` with stable
  identifiers `gcrodr-recycle-legacy-v1`, `gcrodr-recycle-refresh-after-update-v1`,
  `gcrodr-cold-v1`. Default `Legacy`, which keeps the existing call.
- `Rodas5pMfFastWorkspace::set_gcrodr_policy`, and a driver entry point
  taking the policy; the result reports the policy identifier.
- `RefreshAfterUpdate` calls `solve_gcrodr_with_options` with
  `refresh_after_update = true` and the same config, workspace and carried
  state. `Cold` solves every stage from a fresh `GcrodrState` and leaves the
  carried state untouched.
- New counter `WorkCounters::recycle_update_refreshes` (events; serialized
  only when nonzero, so existing ledgers keep their bytes). The refresh
  products themselves are already charged as `recycle_refresh_matvecs`.

## Gate (all required for PASS)

G1. The `Legacy` arm reproduces `BASE_LEGACY.json` exactly: for every row,
    the JSON of the run (t, last y as IEEE bits, attempts, accepted,
    rejected, all counters, message) equals the base row. The default entry
    point equals the `Legacy` arm.
G2. `RefreshAfterUpdate`: `recycle_update_refreshes > 0` in every row with
    `recycle_updates > 0`; `Legacy` and `Cold` rows have
    `recycle_update_refreshes == 0`. A unit contract shows one refreshed
    stage solve through the driver policy equals a direct
    `solve_gcrodr_with_options(refresh_after_update = true)` call bit for bit
    (solution and counters).
G3. `Cold`: a unit contract shows the carried state after an attempt equals
    the state before it.
G4. Prediction: in every row, `RefreshAfterUpdate` linear-solve failures
    <= `Legacy` failures, and strictly fewer in every row where `Legacy`
    has any.

Reported, not gated: success, attempts, rejections and charged
shifted-operator applications (linear + diagnostic + refresh) of all three
arms; whether refresh or cold reaches the end on `brusselator-1d-50`. No
timing; no claim that recycling saves work (L-0059 found it does not on
these sets). The default method and policy stay unchanged.

## Kill / abstain

Any silent reuse of a stale pair under `RefreshAfterUpdate`, uncharged
refresh work, or a change of the default path is a FAIL regardless of G4.
The REV-01c attribution campaign is not rerun.

## Results (append only after the recorded run)

### Executed result — 2026-10-04 (source `65a16dd`)

Command: `SAFE_RECYCLE_OUTPUT=research/safe_recycle_policy_20261004/RUNS.json
cargo test --release -p rodas5p-integrators --locked --test
safe_recycle_policy export_policies -- --ignored --nocapture
--test-threads=1`, then `python3 .../summarize.py` (RESULTS.json). The
contract tests `refreshed_stage_solve_equals_traced_rev01c_call` and
`cold_stage_solve_leaves_carried_state` pass.

**Verdict: PASS.** G1: the `Legacy` arm and the default entry point equal
`BASE_LEGACY.json` in all 14 rows (bits, counters, attempts, messages).
G2: every refresh row with recycle updates has
`recycle_update_refreshes > 0` (equal to the number of updates), Legacy and
Cold have 0; the driver's refreshed stage solve equals the REV-01c traced
call bit for bit, counters and state included. G3: a cold stage solve
leaves the carried state unchanged. G4: refresh failures <= legacy failures
in every row and strictly fewer wherever legacy has any.

| case | rtol | legacy: ok, attempts, lin. failures, operator apps | refresh | cold |
|---|---|---|---|---|
| brusselator-1d-50 | 1e-6 | no (attempt cap), 5000, 2453, 1,363,561 | yes, 92, 0, 32,925 | yes, 92, 0, 31,783 |
| brusselator-1d-50 | 1e-8 | no (attempt cap), 5000, 2475, 1,376,525 | yes, 193, 0, 68,755 | yes, 193, 0, 66,076 |
| brusselator-1d-160 | 1e-6 | yes, 320, 102, 121,099 | yes, 92, 0, 72,278 | yes, 92, 0, 58,383 |
| brusselator-1d-160 | 1e-8 | yes, 420, 102, 147,204 | yes, 193, 0, 105,053 | yes, 193, 0, 80,986 |

On the other five problems no arm has a linear-solve failure; refresh is
within 1 % of legacy in charged shifted-operator applications (at most 0.9 %, robertson 1e-6), cold
costs 1.3x to 2.6x more (e.g. hires 1e-8: 23,515 / 23,522 / 61,896).
On the Brusselator sets cold is 4 % to 23 % cheaper than refresh. These
are counted applications, not time; no recycling speed claim. The
default policy stays `Legacy`; changing it is not part of this node.
