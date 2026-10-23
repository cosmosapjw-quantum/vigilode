# VigilODE: small-resource adaptive causal research

2026-10-23. Baseline `14c74978b93339a79d7228a324fa77552dae548e`.
Native implementation snapshot `548526add83a38a996875b3bd2f3cd3357465ed6`.
Persistent RHS-only correction `d4e8623bf8add9f505ec018532c4602e5a838a41`.

This research implements a bounded RHS-first switching facade, a declared
damped-rotation Lawson-DOPRI facade, causal elimination of the existing raw-U
projected stage equations, and a regular quadratic homotopy chart. Each route
retains its actual equation, derivative work and accuracy scope.

- `REPORT_KO.md`: integrated review and measured results.
- `analysis/ALGEBRAIC_TRANSFER_KO.md`: four constructive algebraic theorems.
- `analysis/OSCILLATORY_KO.md`: quotient action, semilinear accuracy and limitations.
- `analysis/CAUSAL_IMPLEMENTATION_KO.md`: finite causal stage solution and work ledger.
- `analysis/INDEPENDENT_REVIEW_KO.md`: independent findings and corrections.
- `DECISIONS.json`, `NEXT_STEPS.json`, `WORKFLOW.json`: machine-readable outcomes.
- `HANDOFF_KO.md`, `REPRODUCE.md`: local build, scientific checks and further benchmarking.
- `BENCHMARK_PLAN.json`: planned expanded local campaign, with external comparator
  families restricted to VODE/DVODE, diffsol and Julia.
- `../spherical_information_20261023/`: preceding information-geometric research,
  imported with input hashes and unchanged scientific evidence.

No NCP resources are used. The generic switching adapter currently selects
DOPRI5 or protected RODAS5P. Declared-flow and causal projected-stage APIs are
separate explicit choices. Their performance evidence must not be interpreted
as an implemented three-way automatic dispatcher.

Broad production remains HOLD. The new implementation has 205 passing scoped
native tests. The 864-run generic campaign, 324-run persistent correction,
112-record declared-flow campaign and 36-row stage-root experiment retain distinct
source identities, failures and accuracy contracts. The persistent correction
matches native observed DOPRI exactly in all 108 paired raw records.
