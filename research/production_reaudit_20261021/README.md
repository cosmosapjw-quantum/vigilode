# Production re-audit — 2026-10-21

**Decision: broad production/global-accuracy/speed promotion HOLD; scoped corrected solver use and model-bound APIs retained.**

- [Korean integrated report](REPORT_KO.md)
- [Full branch reconciliation](BRANCH_INTEGRATION.md), [machine-readable merge resolutions](BRANCH_INTEGRATION.json), [independent integration review](INDEPENDENT_BRANCH_REVIEW.md)
- [Machine-readable decision](DECISION.json), [next steps](NEXT_STEPS.json), [all-DAG coverage](research_dag.json)
- [Codex / local handoff](CODEX_HANDOFF_KO.md)
- [Final execution](EXECUTION.json), [source identity](SOURCE_MANIFEST.json), [evidence identity](EVIDENCE_MANIFEST.json)
- [Facade review](facade.md), [dense/transaction review](dense_transaction.md), [lagged solver review](integrator.md), [benchmark review](benchmark.md)
- [Full research DAG and theorem audit](research_dag.md), [global-budget proofs](DENSE_GLOBAL_BUDGET_THEORY.md), [references and symbolic checks](REFERENCES.md)
- [RSS portability bridge](HPC_RSS_BRIDGE.md)
- [Independent code review](INDEPENDENT_CODE_REVIEW.md), [research-port review](evidence/port/PORT.md)

The root-level DAG.json/DECISION.json and imported October-18 package are historical records. Their hashes and old verdicts have not been rewritten to impersonate this source. This directory is the current audit entry point. Prior source-bound receipts remain bound to their original commits; current executions have their own manifests.

The canonical production ledger remains research/LEDGER.jsonl. Sibling ledger IDs are identified by branch+commit+ID, not overwritten where IDs collide. The source port makes the previously separate scoped solver APIs available without automatically routing the public facade through research candidates.


The execution-only follow-up is implemented here: [N01 facade](NEXT_N01_IMPLEMENTATION.md), [N02 provenance/reference](NEXT_N02_IMPLEMENTATION.md), [N03/N04 global budget and allocation](NEXT_N03_IMPLEMENTATION.md), [N05 targets and diagonal witness](NEXT_N05_IMPLEMENTATION.md), [N06/N07 comparison clients](NEXT_N06_N07_IMPLEMENTATION.md), [bounded-client witnesses](BOUND_CLIENT_BENCHMARKS.md), [N08 result gate](N08_IMPLEMENTATION.md), and [final integration review](INTEGRATION_REVIEW.md).

Use [NCP_RUNBOOK.md](NCP_RUNBOOK.md), the frozen [NCP_CONFIG.json](NCP_CONFIG.json), and the [Codex 6 Luna low handoff](CODEX_HANDOFF_KO.md) for verification and measurement only. No NCP performance campaign has been executed in this environment. Final native validation and production promotion are separate obligations.

Independent final client-scope review: [BOUND_CLIENT_INDEPENDENT_REVIEW.md](BOUND_CLIENT_INDEPENDENT_REVIEW.md).

The final equivalent facade lint correction is reviewed in [FACADE_FINAL_LINT_REVIEW.md](FACADE_FINAL_LINT_REVIEW.md). Node receipts preserve their original source snapshots; consult EXECUTION.json for the final uniform-source matrix.
