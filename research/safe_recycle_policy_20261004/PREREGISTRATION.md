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
