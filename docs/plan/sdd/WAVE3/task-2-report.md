# Task 2 report: b1c-staleness

**Status: DONE.** The kill-gate verdict is **FAIL**, recorded as measured and preregistered. HIRES gives
0.588 factorizations per accepted step, over the 0.4 limit. Brusselator n = 100 passes all three criteria.

- Worktree: `/home/claude/wt/b1c-staleness`
- Branch: `production/b1c-staleness`, pushed
- Base: `75a0c6c`

The lagged driver stays unrouted from `Tier::Performance`. There is no routing, facade or
`rodas5p_fast.rs` edit.

## Commits

| sha | content |
|---|---|
| `21ed134` | RED: `tests/b1c_staleness_contracts.rs`. GMRES `Off` pins printed at base `75a0c6c` by a temporary probe, which was then deleted and never committed (B2b). Also the B2a edit of `b1b_gmres_contracts.rs`. |
| `5f5eecf` | GREEN: the implementation in `rodas5p_lagged.rs` and the exports in `lib.rs`. |
| `5572d7d` | `research/b1c_staleness_20261021/PREREGISTRATION.md`, committed and pushed before any measurement. |
| `cbcffff` | `results.json`, the numeric file, committed first. |
| `129aa46` | Ledger row L-0141 (FAIL, `supersedes: null`), results appended below the "Results" line of the preregistration, and the node report `docs/plan/nodes/b1c-staleness.md`. |

## Requirements mapped to file:line

Paths are under `/home/claude/wt/b1c-staleness/crates/rodas5p-integrators/`. Line numbers are as of `129aa46`.

| Requirement | Location |
|---|---|
| `pub enum StalenessRule { #[default] Off, Bounded }` | `src/rodas5p_lagged.rs:203` |
| `Rodas5pLaggedOptions::staleness` (struct still `Copy + Eq + Default`) | `src/rodas5p_lagged.rs:221` |
| `RODAS5P_LAGGED_STALENESS_Q_MAX = 0.5` | `src/rodas5p_lagged.rs:155` |
| `rodas5p_staleness_bound(..)` (Lemma B1c; +∞ when unbounded; B2d zero weight gives +∞; never NaN) | `src/rodas5p_lagged.rs:313` |
| `StalenessWork` (derives `Clone, Copy, Debug, Default, PartialEq, Serialize`) | `src/rodas5p_lagged.rs:274` |
| `Rodas5pLaggedResult::staleness` | `src/rodas5p_lagged.rs:293` |
| Private `Refresh::Staleness` | `src/rodas5p_lagged.rs:302`; charged at `:411` |
| Dense J_P copy at each refresh, allocated once at setup (Bounded only) | `:417`, `:1006` |
| Check before the stage loop, only when P is valid and no Initial or Forced refresh is due (B2c); refresh iff q_pred > 0.5 | `:715` |
| Stage stop ‖P⁻¹r‖_S ≤ (1 − q_pred) ρ_i: Richardson | `:657` (via `stops`, `:468`) |
| Stage stop: GMRES, at k = 0 | `:570` |
| Stage stop: GMRES, in the monitor | `:594` |
| Contraction and progress refreshes unchanged, on WRMS | unchanged code next to `:594` and `:657` |
| Driver ids `rodas5p-lagged-{richardson,gmres}-bounded-v1` | `:146`, `:151`; selected at the end of `integrate_rodas5p_lagged_observed` |
| Module docs; `preconditioner_refreshes` doc updated (B2e) | `:95`, `:229` |
| Exports | `src/lib.rs:413-421` |
| B2a | `tests/b1b_gmres_contracts.rs:159` (formerly :156) |
| Test (a) `staleness_bound_dominates_explicit_t_norm` | `tests/b1c_staleness_contracts.rs:104` |
| Test (b) `bound_is_exact_for_scalar_step_change` | `tests/b1c_staleness_contracts.rs:150` |
| Test (c) `unbounded_log_norm_refreshes_every_attempt` (FP1: J = [[−1, 1e8], [0, −1]]) | `tests/b1c_staleness_contracts.rs:182`, `:215` |
| Test (d) `bounded_off_is_wave2_bitwise` (Richardson pins `:263`; GMRES base pins `:302`) | `tests/b1c_staleness_contracts.rs:339` |
| Test (e) `forced_refresh_bounded_equals_fast` | `tests/b1c_staleness_contracts.rs:387` |
| Test (f) `staleness_ledger_identity` | `tests/b1c_staleness_contracts.rs:472` |
| `#[ignore]` `b1c_kill_gate_measurement` (writes `results.json` when `B1C_RESULTS_OUT` is set) | `tests/b1c_staleness_contracts.rs:524` |
| Research node, ledger L-0141, node report | `research/b1c_staleness_20261021/`, `research/LEDGER.jsonl` (last line), `docs/plan/nodes/b1c-staleness.md` |

## Test evidence

**RED.** At `21ed134`, the `b1c_staleness_contracts` target failed to compile (E0432/E0560/E0609),
because `StalenessRule`, the bound, `StalenessWork`, the `staleness` field and the bounded ids did not
exist.

**GREEN.** At `5f5eecf` (debug, `-j 1 --locked`):

| target | result |
|---|---|
| `b1c_staleness_contracts` | 6 passed, 1 ignored |
| `b1b_gmres_contracts` | 8 passed, 1 ignored |
| `b1_lagged_contracts` | 5 passed, 1 ignored |
| `a1_zero_tolerance_scale` | 9 passed |

`b1c_staleness_contracts` also passes in release (6 passed).

During GREEN, one test-side fix went into `5f5eecf`. It concerned my own extra assertion in (b), not a
preregistered item: `h_P = 1/γ` gave `h_Pγ = 1 − ulp`, so the assertion now uses `h_P = 2/γ`.

**Hygiene.**
- `rustfmt --edition 2024 --check` is clean on the 4 touched Rust files.
- clippy (`--lib --test b1c_staleness_contracts --test b1b_gmres_contracts`) reports no warning in the
  touched files.
- `python3 tools/check-research-node.py --base a5a48f77e7ac7a80b8ff86cd897110790066f7ee` prints PASS
  (141 rows) and exits 0.

## Measurement (rtol 1e-6, reference = fast at rtol 1e-11)

The run used release mode with `-j 1` and took about 3 minutes, most of it the build. A debug re-run
printed identical counters.

| problem | arm | accepted (fast) | fact/acc | step ratio | error ratio | checks / staleness refr. / unbounded | verdict |
|---|---|---|---|---|---|---|---|
| HIRES | Gmres+Bounded (gated) | 204 (204) | **0.588** | 1.000 | 1.004 | 210 / 119 / 75 | **FAIL** |
| HIRES | Richardson+Bounded (context) | 204 | 0.789 | 1.000 | 0.997 | 209 / 101 / 75 | – |
| Brusselator n=100 | Gmres+Bounded (gated) | 82 (82) | 0.244 | 1.000 | 0.998 | 91 / 19 / 0 | PASS |
| Brusselator n=100 | Richardson+Bounded (context) | 82 | 0.890 | 1.000 | 1.000 | 91 / 8 / 0 | – |

**Overall: FAIL.** The node report says "B1 family closed until a new mechanism".

Interpretation (not tested here): the corrected stop removes B1b's step inflation. Brusselator goes from
980 to 82 accepted steps, and the error is at fast's level. On HIRES the bound itself is the cost.
Unbounded checks (h_Pγμ_S ≥ 1) alone are 0.37 refreshes per accepted step, while GMRES needed no
contraction or budget refresh. The preflight proxy predicted 0.39.

## Deviations

1. **Profile.** The measurement ran in release, not debug (dispatch permission). Counters were verified
   identical in debug. The deviation is stated in the PREREGISTRATION before the run.
2. **Test (c).** Test (c) uses FP1's non-normal Jacobian with y0 = (0, 1e-8) and atol = rtol, so the
   weights stay near atol and every check is unbounded. Both stage solvers are covered at rtol 1e-4 and 1e-6.
3. **Test (a).** Test (a) uses outputs 1, 4 and 8 of a fast run as the three states, with J_P from
   outputs 0, 2 and 6.
4. **Test (e).** Test (e) covers both stage solvers, not only the default.
5. **`q_pred_max`.** `q_pred_max` records the largest finite q_pred. Unbounded checks are counted in
   `unbounded_checks`. This avoids an `inf`, which serde_json would write as `null`.
6. **Exports.** The new exports in `lib.rs` include two driver-id constants that the brief did not name.

## Handoff commands

None required: everything ran here. To reproduce on the workstation, use the environment of the
dispatch contract and run:

    cargo test -p rodas5p-integrators --locked -j 1 --test b1c_staleness_contracts --test b1b_gmres_contracts --test b1_lagged_contracts --test a1_zero_tolerance_scale
    B1C_RESULTS_OUT=$PWD/research/b1c_staleness_20261021/results.json cargo test -p rodas5p-integrators --release --locked -j 1 --test b1c_staleness_contracts b1c_kill_gate_measurement -- --ignored --nocapture

The `wall_ms_diagnostic` fields will differ; all counters and errors should be bit-identical.

## Open questions

- **T8 seam (iii).** It should add `b1c_staleness_contracts` to the `production.yml` `--test` list.
  `ignored-tests.yml` will run `b1c_kill_gate_measurement`. It always passes as a test, because it
  computes and prints the verdict and does not assert PASS.
- **L-0141 is in place.** e1b-dense may append L-0142 once its branch contains `129aa46` (R-C6).
