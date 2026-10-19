# Final repository-wide status, 2026-10-20 KST

This note complements the numerical results. It does not modify an observed value, a historical ledger row, or an acceptance threshold.

## Scoped numerical result versus whole-repository gates

The successful measured source is `f8e89b70528da8490fbe1febf0b0ede9ff84a01e`, with numerical implementation from `b51c051b045e977c038e5eec468de0004840872b`. Run `39304607635` retains 432 accuracy rows, 324 exact output comparisons, 576 timed processes and 36 paired comparisons. The separate readback and frozen closeout succeeded. Closeout verified 122 archived file hashes and appended the scoped evidence row `L-0137`.

The repository-wide research-policy checker returned exit 1. Its two findings were:

- `L-0124`: SHA-256 mismatch for `research/production_audit_20261018/CONTINUATION_DAG.json`.
- `L-0124`: SHA-256 mismatch for `research/production_audit_20261018/CONTINUATION_FINDINGS.json`.

These are not new numerical failures. Both files were compared through the GitHub contents API at the numerical parent `b56bb19ce85bc28a021138ee29689701a81bc4f0` and the closed publication commit `0da4c8a579a28f19a4e681b856aad53ced890ef2`. The exact Git blob IDs agree:

| File | Parent and publication Git blob |
|---|---|
| `CONTINUATION_DAG.json` | `d2332f1bcd01bbf83486a0b3fda604c74e7bc6b7` |
| `CONTINUATION_FINDINGS.json` | `d9d6ac662e58c332470f5e5c0fd30292142201ce` |

The ledger closeout preserves the original byte prefix. The historical `L-0124` records were not rewritten to force a green policy result. Their reconciliation remains a separate repository provenance obligation. See `RESEARCH_POLICY_CHECK.log`, `RESEARCH_POLICY_STATUS.json`, and `LEDGER_APPEND_RECEIPT.json`.

The full-workspace test also remains incomplete: exit 124 after the bounded 660-second run. Its 14 completed target summaries report 142 passed and 33 ignored tests, with no failed tests in those completed summaries; this does not count unfinished targets as passed. Raw log lines containing `error:` are retained, including command-line error probes, and are not automatically equivalent to a failed test result. See `VALIDATION.json` for the exact summary and terminal log section.

## Concurrent upstream update

During this task, `claude/nice-dijkstra-208xq0` advanced from `b56bb19ce85bc28a021138ee29689701a81bc4f0` to `c03ca90f19a9e64ba7751c069325a48b54b39c2e`. The one-commit comparison changed triage policy/documentation, schema/checker/reporting and their tests, not the production numerical crate modified here.

The final PR read showed Draft PR #73 open, not merged, and mergeable. No automatic merge or retargeting was performed. Measurements are still attributed to their frozen sources, not to the later upstream tip or a hypothetical merge commit.

**Final interpretation:** scoped correctness repair and opt-in numerical implementation study are complete; whole-workspace validation and repository-wide provenance-policy clearance are not complete. Default-kernel promotion and general performance claims remain on hold.
