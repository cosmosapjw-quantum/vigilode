# Node D1: Stiffness-Gated Entry Pilots via DOPRI5 Stage Secant Quotient
- written: 2026-11-04 (committed and pushed before the first run)
- question: Does gating implicit entry pilots on the DOPRI5 stage-pair secant quotient $h\hat\lambda \le -\text{fraction} \cdot 3.306568$ (`fraction = 0.8`) preserve all registered entry decisions, lower discarded pilot work on `rotation` and `gyro` by $\ge 90\%$, and achieve $\ge 0.90$ agreement with a $10\times$-loosened-tolerance intervention test across $\ge 3$ nonlinear problems with $n \ge 50$?
- arms: `adaptive_evidence` (`stiffness_gate_fraction = None`), `adaptive_evidence_gated` (`stiffness_gate_fraction = Some(0.8)`); everything else identical
- cells: Registered cells (`stiff-forced`, `rotation`, `decay`, `relaxation`, `case5`, `transition-39`, `transition-79`, `gyro`) and 3 off-registry nonlinear PDEs with $n \ge 50$ (`brusselator_1d_n64`, `viscous_burgers_n64`, `allen_cahn_n64`) at `rtol = 1e-6`
- command: `python3 research/d1_stiffness_gate_20261104/eval_d1.py`
- inputs: `research/d1_stiffness_gate_20261104/PREREGISTRATION.md`, `research/d1_stiffness_gate_20261104/eval_d1.py`, `crates/rodas5p-integrators/src/dopri5.rs`, `crates/rodas5p-integrators/src/dopri5_stiffness.rs`, `crates/vigilode/src/switching.rs`
- metrics: deterministic `WorkCounters` (`attempts`, `rhs_calls`, `jvp_vectors`, `switches`, `discarded_pilot_jvps`) and window-level agreement fraction with the $10\times$-loosened-`rtol` step-growth intervention test; wall time diagnostic only
- PASS gate: (1) identical entry decisions on all registered cells; (2) discarded pilot work on `rotation` and `gyro` lower by $\ge 90\%$; (3) agreement of the gate with the $10\times$-loosened-`rtol` intervention test $\ge 0.90$ on $\ge 3$ nonlinear problems with $n \ge 50$
- KILL gate: any lost registered entry, or agreement $< 0.90$ off the registry (prototype saw $0.58$ on the Brusselator at `rtol = 1e-6`)
- holdout: none (sealed holdout profiles `holdout-512`, `enforced-budget-holdout-320`, `stage-growth-holdout-384` and `tools/reference_v2/artifacts/*-holdout-v2.json` are not executed or inspected)
- analysis: `research/d1_stiffness_gate_20261104/eval_d1.py`; no threshold chosen after seeing results

## Results (appended after the run; nothing above this line changes)

- **Execution Commit (`EXEC_SHA`)**: `4339facc62258b5fb9a2469f16a3af8b93763e24`
- **Verdict**: `FAIL` (`stiffness_gate_fraction = None` retained as default in `adaptive_evidence` and `route_registry`; stiffness gate remains opt-in)
- **Summary**: With `stiffness_gate_fraction = Some(0.8)`, all registered entry decisions are preserved and discarded pilot work on `rotation` and `gyro` drops by $100\%$ ($\ge 90\%$). However, on the off-registry nonlinear 1-D Brusselator ($n = 64$, `rtol = 1e-6`), agreement with the $10\times$-loosened-`rtol` step-growth intervention test is $14/24 = 0.5833 < 0.90$, triggering the KILL gate for factory promotion. Therefore `stiffness_gate_fraction` remains `None` by default.

