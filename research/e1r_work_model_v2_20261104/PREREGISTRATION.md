# Switcher WorkModel v2 — Preregistered Re-run with Measured Callgrind Ir (Node E1R)
- written: 2026-11-04 (committed and pushed before the first run)
- question: Does WorkModel v2—defined with frozen fixed-point polynomial weights $\omega_k(n)$ over denominator $2^{16} = 65536$ for the nine solver-internal events $\mathcal K$—accurately predict measured `valgrind --tool=callgrind` instruction counts (`Ir`) per solve across routes and dimension $n$ while satisfying the original `aafc2e0b` preregistration gates verbatim?
- arms: `v1` (existing callback proxy $c_\text{rhs} + c_\text{jvp} + c_\text{ft} + n\,c_\text{jac} + c_\text{solves}$), `v2_cheap` (WorkModel v2 with default conservative $\kappa = 50$), `v2_declared` (WorkModel v2 with problem-matched declared $\kappa \in \{500, 5000\}$); everything else identical
- cells: Existing registered cells ($n \le 6$, cheap: `rotation`, `decay`, `stiff-forced`, `case5`, `transition-39`), 1-D Heat / Advection-Diffusion MOL ($n \in \{32, 64\}$, cheap), 1-D Brusselator MOL ($n = 64$, moderate), Expensive-RHS wrapper ($n = 32$, expensive)
- command: `python3 research/e1r_work_model_v2_20261104/eval_e1r.py`
- inputs: `research/e1r_work_model_v2_20261104/PREREGISTRATION.md`, `research/e1r_work_model_v2_20261104/eval_e1r.py`, `crates/vigilode/data/work_model_v2.toml` (frozen at the preregistration commit; no measured `Ir` values typed as literals into any script)
- metrics: deterministic `WorkCounters`; `valgrind --tool=callgrind` instruction count (`Ir`) parsed from callgrind output as authority; wall time diagnostic only
- PASS gate: (1) Ir Prediction Accuracy: predicted Ir per solve ($\kappa \cdot W_{v2}$) matches measured callgrind Ir within $\pm 25\%$ across calibration routes (`max_relative_error <= 0.25`); (2) Rank Correlation: Spearman rank correlation $\rho \ge 0.90$ between predicted and measured Ir across calibration cases; (3) Transition-39 Admission: `t39_w2 < 200` (as originally written in `aafc2e0b`, well below the B5 warning threshold of 240); (4) Zero-Allocation: zero heap allocations in hot loops during $W_{v2}$ evaluation (`hot_loop_allocations == 0`); (5) Exact Integer Safety: checked `u128` arithmetic returning `None` on overflow without truncating
- KILL gate: (1) WorkModel v2 fails `t39_w2 < 200` or delays transition-39 entry beyond threshold; (2) relative error $> 25\%$ or Spearman $\rho < 0.90$ against measured callgrind `Ir`; (3) any regression in registered numerical accuracy caps or any heap allocation in hot loop evaluations
- holdout: none (sealed holdout profiles `holdout-512`, `enforced-budget-holdout-320`, `stage-growth-holdout-384` and `tools/reference_v2/artifacts/*-holdout-v2.json` are not executed or inspected)
- analysis: `research/e1r_work_model_v2_20261104/eval_e1r.py`; no threshold chosen after seeing results

## Results (appended after the run; nothing above this line changes)

- **Execution Commit (`EXEC_SHA`)**: `4339facc62258b5fb9a2469f16a3af8b93763e24`
- **Verdict**: `FAIL` (`WorkModelVersion::V1` retained in `route_registry` factory)
- **Summary**: Under the verbatim `aafc2e0b` preregistration gates and live `valgrind --tool=callgrind` `Ir` measurements (`eval_e1r.py`), `transition39_n2_mf` evaluates to `w_v2 = 264.2 WU >= 200.0 WU` (`transition_39_admission_pass = false`), Spearman rank correlation is `0.7667 < 0.90`, and maximum relative error against measured callgrind `Ir` exceeds `0.25`. Accordingly, `WorkModel v2` remains opt-in only and does not return to `SwitchingConfig::route_registry`.

