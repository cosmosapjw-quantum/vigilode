# B7: First-Batch Length Preregistered Comparison (`max` vs `min`)
- written: 2026-11-04 (committed and pushed before the first run)
- question: Does replacing the B7 first explicit batch rule `max(budget, k_d0)` with `min(budget, k_d0)` or `min(budget, max(k_d0, 24))` lower total callbacks by >= 10% on >= 3 of 5 cells without raising any cell by > 5% or losing entry decisions?
- arms: `max_budget_kd0` (`max(budget, k_d0)`, current `route_registry` baseline), `min_budget_kd0` (`min(budget, k_d0)`), `min_budget_max_kd0_24` (`min(budget, max(k_d0, 24))`); everything else identical
- cells: `stiff_forced`, `transition_39`, `transition_79`, `dense_n64`, `dense_n256` under `SwitchingConfig::route_registry`
- command: `python3 research/b7_first_batch_20261104/eval_b7.py`
- inputs: `research/b7_first_batch_20261104/PREREGISTRATION.md`, `research/b7_first_batch_20261104/eval_b7.py`, `crates/vigilode/src/switching.rs`
- metrics: deterministic total callbacks (`rhs_calls + jvp_calls + n * jacobian_builds`), attempts, and entry time `t_entry`; wall time diagnostic only (`"timing_authority": "diagnostic_only"`)
- PASS gate: a candidate `min` arm lowers total callbacks by >= 10% on >= 3 of 5 cells and raises none by > 5%, with identical entry decisions
- KILL gate: every `min` arm raises `stiff_forced` work or delays/loses entry on a stiff cell
- holdout: none
- analysis: `research/b7_first_batch_20261104/eval_b7.py` evaluates all 5 cells across the 3 arms and checks the PASS and KILL gates without post-hoc modification

## Results (appended after the run; nothing above this line changes)
- verdict: FAIL (KILL gate triggered; current `max(budget, k_d0)` rule from F5 retained)
- summary: Both `min(budget, k_d0)` and `min(budget, max(k_d0, 24))` shrink the first explicit batch on `stiff_forced`, delaying implicit entry from t=0.0082 to t=0.0741 / 0.0315 and raising RHS calls from 400 to 1840 / 892.
