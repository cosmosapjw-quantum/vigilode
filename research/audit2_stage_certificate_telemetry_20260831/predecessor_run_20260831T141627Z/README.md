# Predecessor run receipts (stage-certificate telemetry, 2026-08-31)

Compact receipts from the external run
`~/.local/state/vigilode/stage-certificate-telemetry/20260831T141627Z`
on the owner's host. That run executed the PR #41 handoff and ended
`STOP_INVALID`. Its source and proof bytes are in the parent commit.

## Included

| Path | Content |
|---|---|
| `terminal/analysis_summary.json` | terminal disposition, unmet obligations, test counts |
| `terminal/bounded_closeout.json` | closeout decision and failed criteria |
| `terminal/failed_attempt_history.json` | classified failed attempts |
| `terminal/formal_adjudication.json` | per-backend formal dispositions |
| `terminal/publication_status.json` | why the result was not published |
| `reviews/formal_scope_review.json` | P1 findings on F01, F03, F04 |
| `reviews/software_contract_review.json` | P1 on `max_arnoldi`, two P2 findings |
| `formal/formal_receipt.json` | backend compilation receipt |
| `formal/formal_worker_contract.json`, `formal/formal_worker_failed_attempts.json` | formal worker contract and failures |

## Left out and bound by SHA-256

`RAW_DATA_POLICY.md` forbids command transcripts and caps file sizes, so these stay outside Git.

| File | Bytes | SHA-256 |
|---|---:|---|
| `terminal/command_readback.json` | 99679 | `65d4f4dbedb65befe0388009dbbbb7dabd5a9fa98cc7cea5f77480c6dd00f5e7` |
| `SHA256SUMS` | 4153211 | `057a1f5ccafc5889684caa72af0cd716edc3e04ec0f58b2b771a3971baeeb3d8` |
| `PRESEAL_SHA256SUMS` | 4149550 | `4c87502e9e372ed1ca7edae43d4a60ca01c8616e9fbcf070a874daa15b9cff84` |
| `preseal_manifest.json` | 5714368 | `1790764c24e4263df209323090e38e7030b62c7d5ed7b9938c30ba08f628e7c3` |
| `external_manifest.json` | 5719558 | `3812075cc7457f1797d512fde55b49b3202053d257e8a350a6cf48c2cea5029f` |

Raw logs, the MLflow store, Cargo targets and harness archives also stay outside Git.
