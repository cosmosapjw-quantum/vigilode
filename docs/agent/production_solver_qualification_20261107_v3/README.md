# VigilODE production-solver qualification v3

**Decision: `HOLD_NOT_RELEASE_QUALIFIED`**

Pinned GitHub snapshot:
- Repository: https://github.com/cosmosapjw-quantum/vigilode
- Branch: `research/switcher-v2-20261103`
- HEAD: `92d975dd4abfebb924f9a691dac1e5f87fb69943`
- Tree: `2fd16cb90a9e75f9fde87de3584f1feab2668aca`
- Audit date: 2026-11-07

This package is additive. It preserves the v2 historical qualification bundle and does not change solver source code.

## Conclusion

The latest source contains substantive remediation: sealed/scoped certification evidence, explicit partial-result API semantics, persistent-session and causal-contract tests, negative controls, and a new spatial-grid comparison. However, the new spatial campaign does not yet qualify cross-solver work-precision: all 520 matched-error comparison entries are `DIAGNOSTIC_ONLY`. The selected VigilODE arms are grid-clipped continuity controls, while endpoint-only dense-output arms already exist in the benchmark code but are absent from the v1 spatial corpus. The reference check uses one tolerance level and 101 sample points; it is empirical agreement, not a continuous-time enclosure. Whole-run allocation counts do not isolate the solver hot loop. Current HEAD also has no observed GitHub status contexts or workflow runs in this audit.

The allowed claim remains: a constrained adaptive floating-point ODE solver research implementation with locally verified components and diagnostic benchmark data. It is not a validated ODE solver with guaranteed continuous global error bounds, nor release-qualified for general production use.

## Highest-priority findings

| Finding | Severity | Correction |
|---|---|---|
| F-BENCH-001 | BLOCK_NOW | The 520/520 matched-error entries are diagnostic-only. Rebuild a work-precision corpus with bracketed achieved errors. |
| F-BENCH-002 | BLOCK_NOW | Use existing endpoint-only `vigilode-dense` and `vigilode-banded-dense` arms as primary baselines; retain grid-clipped arms separately. |
| F-REF-001 | BLOCK_NOW for guarantees | Add a preregistered reference refinement ladder and uncertainty budget. |
| F-CI-001 | BLOCK_NOW | Run full tests/build/clippy on a clean current-HEAD checkout. |
| F-SCOPE-001 | BLOCK_NOW | Keep continuous validated-enclosure and general solver claims unqualified. |
| F-ALLOC-001 | FIX_SOON | Measure calibrated complete hot-loop allocation separately from setup/teardown. |
| F-PROV-001 | FIX_SOON | Rebuild from a clean exact current HEAD; the v1 benchmark is content-bound but built with dirty source context. |

## Package map

- `CURRENT_AUDIT.json`: source identity, observations, blockers and caveats.
- `PRODUCTION_VERDICT.json`: claim-by-claim status.
- `BENCHMARK_AUDIT.json`: machine-readable reanalysis of spatial v1 results.
- `ACCEPTANCE_MATRIX.json`: claims, gates and carried-forward legacy thresholds.
- `TASK_QUEUE.json` / `TASK_GRAPH.json`: bounded tasks and dependencies.
- `TASK.schema.json` / `EVIDENCE.schema.json`: machine-readable contracts.
- `DESIGN_CONTRACT.md`: mathematical, numerical and physical-model invariants.
- `MUTATION_POLICY.json` / `ANTI_DRIFT.json`: mutation gates and anti-hallucination rules.
- `VALIDATION_PROTOCOL.md`: reproducible commands and evidence semantics.
- `SOURCE_REFERENCES.json`: exact Git blob identities and primary literature.
- `AGENT_PROMPT_KO.txt`: standalone execution prompt.
- `validate_package.py`: standard-library validator.
- `MANIFEST.json`: file identity index. It excludes itself by design.

## Execution

Read `AGENT_PROMPT_KO.txt`. Execute tasks in dependency order. Source edits are permitted only in isolated task worktrees and only at exact paths in each task's `write_allowlist`. Do not relax tolerances or inherited thresholds. Do not commit/push/release solver changes.

## Audit limitations

The connected local checkout at `/home/cosmosapjw/vigilode` was not accessible because Remote Desktop Commander had exhausted its monthly usage allowance. This package was pushed to GitHub; no claim is made that the local path was updated or synced. No Cargo tests or builds were run in this audit turn. Existing receipts and GitHub-hosted artifacts were read and independently reanalysed; they are not new current-HEAD executions.
