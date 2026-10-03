# Preregistration: optional model epoch for interior-mutable callbacks (remaining-only DAG node R-NEXT-07)

## Question

PR #70 binds the matrix-free workspace's frozen data (`f(t, y)`, `f_t`, the JVP operator and, through the operator
token, the GCRO-DR/LGMRES recycle images) to the exact time and state bits and to the retained callback `Arc`
identities. A callback that reads interior-mutable parameters (for example an `Arc<AtomicU64>` or a `Mutex`) can
change its function without changing any of those, so `fresh = false` reuses stale data. That is the documented
manual-fresh contract, not a defect. Can an optional, client-supplied model epoch make that reuse safe for clients
who opt in, without changing anything for clients who do not?

## Design (written after this commit)

- `OdeProblem::with_model_epoch(epoch)` takes `Arc<dyn Fn() -> u64 + Send + Sync>`, a client-owned generation
  counter. `OdeProblem::model_epoch()` returns `Option<u64>`. The client increments it whenever any data read by
  `rhs`, `jvp`, `partial_t` (or the Jacobian) changes. The epoch is a promise by the client, not something the
  library can verify; neither the epoch nor any pointer or hash is a semantic proof that the model is unchanged.
- `Rodas5pMfFastWorkspace::attempt` reads the epoch once per attempt. Frozen data are reused only if, in addition to
  the PR #70 conditions, the epoch equals the one read when the data were built. Any difference (larger or smaller)
  invalidates `f0`, `f_t` and the JVP operator together, before any fallible callback. The new operator has a new
  token, so a GCRO-DR or LGMRES recycle state refreshes its images on the next solve through the existing
  cross-operator path.
- A problem without an epoch behaves exactly as before (manual `fresh` contract).

## Test and command

`crates/rodas5p-integrators/tests/rnext07_model_epoch.rs`, writing `RESULTS.json` here:

`RNEXT07_OUTPUT=research/rnext07_model_epoch_20261003/RESULTS.json cargo test -p rodas5p-integrators --locked --test rnext07_model_epoch -- --nocapture --test-threads=1`

Model: a nonautonomous scalar-parameter problem `y_i' = -p (y_i - sin t) + cos t + q y_(i+1) y_i` (n = 6) whose
`p` is read from an `Arc<AtomicU64>` (bits of an f64) inside `rhs`, `jvp` and `partial_t`.

## Gate

**PASS** if all hold:

1. **Epoch change invalidates everything together.** With the epoch bumped after changing `p` between two attempts
   at the same `(t, y, h)` with `fresh = false`, the second attempt evaluates `f(t, y)` and `f_t` again (counter
   deltas), builds a new operator, and, with GCRO-DR and a carried recycle state, refreshes the recycle images
   (`recycle_cross_operator_refreshes` increases). Its `y_new`, embedded error norm and stages are bitwise equal to
   a `fresh = true` attempt on the changed model from an identical workspace and recycle state.
2. **Same epoch keeps the reuse.** With an unchanged epoch, the second attempt reuses the frozen data (no
   evaluation of `f(t, y)` or `f_t` at the stage-1 point) and is bitwise equal to the same sequence on a problem
   without an epoch.
3. **No epoch, old contract.** A problem without an epoch, `p` changed, `fresh = false`: the attempt reuses the
   stale data (bitwise equal to the attempt on the unchanged model). This documents that changes are not detected
   automatically; `fresh = true` gives the changed-model result.
4. **Any change, and failure safety.** A smaller epoch also invalidates. An RHS that fails during the refresh after
   an epoch change leaves no frozen data: the next `fresh = false` attempt with the epoch restored refreshes again
   rather than pairing old data with the old epoch.
5. **No other effect.** The existing MF workspace contract tests pass unchanged.

Otherwise **FAIL**. Reported: counter deltas per case. No numerical-accuracy, speed or timing claim; timing
authority stays on HOLD.

## Stop condition (DAG)

No claim that arbitrary callback mutations are detected automatically.

## Prior information

PR #70's A-CACHE-01/02 regressions (`native_reaudit_cache_contracts.rs`) cover time, state, callback identity and
failed refresh. No epoch code exists before this commit.

## Amendment before the recorded run (development observation, disclosed)

A development run of the test showed that gate item 3's prediction was wrong for this model. The workspace's JVP
operator calls the client's `jvp` callback at every product, and that callback reads `p` live. So without an epoch,
after a change of `p` with `fresh = false`, the attempt combines a stale `f(t, y)` and `f_t` (old `p`), a live
operator (new `p`), and GCRO-DR recycle images built with the old operator. The token is unchanged, so they count
as current. In the development run, GCRO-DR then failed ("least-squares solve produced NaN/Inf for 5x4 system").
The attempt is therefore neither the unchanged-model attempt nor the changed-model attempt.

Item 3 is replaced by: **3. No epoch, old contract.** A problem without an epoch, `p` changed, `fresh = false`: the
attempt is not equal to a `fresh = true` attempt on the changed model (it fails or differs), i.e. the change is not
detected. The outcome (error text or difference) is recorded. A second case uses GMRES (no recycle state), where the
attempt must complete and differ from both the unchanged-model and the changed-model attempts (stale `f` with live
operator). `fresh = true` gives the changed-model result bitwise. Items 1, 2, 4 and 5 are unchanged. The epoch
removes exactly this mixed state for clients who opt in.

---

## Results (appended after the run at `f73cd3c`)

Output: `RESULTS.json`. Ledger row L-0044.

**Gate: PASS** (items 1-5 hold).

| Gate item | Outcome |
|---|---|
| 1. Epoch change invalidates everything | **holds**: after `p` 2 -> 5 and an epoch bump, the `fresh = false` attempt evaluates `f(t, y)` and `f_t` again (8 RHS evaluations, 1 `f_t` call, as a fresh attempt), refreshes the rank-2 GCRO-DR recycle images (1 cross-operator refresh, 2 refresh matvecs) and is bitwise equal to the `fresh = true` twin, stages and error norm included |
| 2. Same epoch keeps the reuse | **holds**: 7 RHS evaluations, no `f_t`, no refresh; bitwise equal to the same sequence without an epoch |
| 3. No epoch, old contract (amended) | **holds**: with GCRO-DR the stale `f`, `f_t` and recycle images under the live operator fail ("least-squares solve produced NaN/Inf for 5x4 system"); with GMRES the attempt completes and differs from both the unchanged-model and the changed-model attempts; `fresh = true` gives the changed-model result bitwise |
| 4. Any change, failure safety | **holds**: a smaller epoch (5 -> 3) invalidates; a refresh that fails after an epoch change leaves nothing reusable, and the next `fresh = false` attempt with the old epoch restored refreshes and equals its twin |
| 5. No other effect | **holds**: `native_reaudit_cache_contracts` 8/8 and `matrix_free_problem_contracts` 2/2 pass (all features) |

Development disclosure: before the recorded run the test ran three times. The first showed the wrong item-3
prediction (amended in `daf40ca` before the recorded run). The second had a test bug in item 4: it compared with a
lane whose recycle history differed, so the two attempts differed at rounding level; the comparison now uses a twin
lane with the same history. In the third, the recorded command failed before writing anything, because the output
path was resolved from the crate directory. That was fixed in `f73cd3c`. No gate threshold changed.

What the run shows beyond the gate: without an epoch, `fresh = false` after an interior change is not just stale.
It mixes old and new model data, and with GCRO-DR it can fail outright. The epoch removes that state for clients who
opt in. Claim ceiling: an API contract for clients who keep their epoch honest; no automatic detection of callback
mutations; no accuracy, speed or timing claim.
