# Machine-readable interface

UTF-8 JSON, schema_version `1.0`. IDs are stable within this review. All aggregate evidence paths resolve relative to this folder unless `evidence_base` or `detail_file` states the lane context. Source locations resolve relative to the audited repository; line numbers refer to the source commit, not future edits. Native JSONL rows are observations, not test-count units.

| File | Meaning |
|---|---|
| REPORT_STATUS.json | Source identity, aggregate disposition, publication constraints and execution pointers |
| FINDINGS.json | Four confirmed items, novelty/regression distinctions, source/observed/expected evidence and seven original closures |
| RESEARCH_CLAIMS.json | Bounded claims, evidence level, exclusions, independent decision and production hold |
| NEXT_DEVELOPMENT_DAG.json | Executable planning contracts with dependencies, implementation, acceptance and failure semantics |
| PACKAGE_SCHEMA.json | JSON Schema 2020-12 definitions for the four aggregate document shapes |
| evidence/runtime/FINAL_RUNTIME_SUMMARY.json | Final actual runtime disposition; build status separate from native executions |
| decision/INDEPENDENT_DECISION.json | Independent scoped decision, not whole-repository certification |

`P1/P2` on findings describes defect urgency. Priorities on research development nodes describe prerequisite importance and do not add new defects to the finding count. `implementation-verified` means the named bounded implementation/observation was checked. It does not mean the production solver was modified. `INCONCLUSIVE` is the research-process ledger verdict because the exact protocol was not committed before runs; bounded scientific evidence is separately assessed. A closure is limited to the named original reproducer.

Use the relevant `#/$defs/status`, `findings`, `claims`, or `dag` schema as the root. `validate_review_package.py` adds source, manifest, counts, target semantics, process and DAG cross-checks using only the Python standard library. It does not replace an independent scientific oracle or a fresh native run.

Lane-local receipts retain their creation-time states, including pending-review labels and hashes of initial drafts. The final authority for review disposition is `decision/INDEPENDENT_DECISION.json`, and for execution totals it is `evidence/runtime/FINAL_RUNTIME_SUMMARY.json`. Historical failed/incomplete build receipts are evidence of those attempts, not the final suite verdict.
