# Adversarial audit, 2026-09-27

Audit target: Draft PR #42 head `b3e8165c8dc3b5016702821d280daea1a3f1feb7`. The audit changed no source code.

| Path | Content |
|---|---|
| `VIGILODE_ADVERSARIAL_AUDIT_20260927.md` | Report, in Korean |
| `VIGILODE_ADVERSARIAL_AUDIT_LEDGER_20260927.json` | Machine-readable ledger |
| `VIGILODE_ADVERSARIAL_AUDIT_LEDGER_SCHEMA_20260927.json` | JSON Schema, draft-07 |
| `HANDOFF_PROMPT.md` | Prompt for the session that implements the fixes |
| `fixes/` | Fix designs per group, roadmap proposal, completeness critique |
| `refute/` | Severity, provenance and second-refuter records for the top 11 findings, narrowings, final number and anchor checks |
| `experiments/` | E-01 to E-10: README, results, scripts. Files above 300 KB are left out |
| `harness/` | Standalone crate with the experiment binaries |
| `baseline/` | Baseline test timeline and the note on the failing ignored test |

Result: 94 confirmed findings, of which 4 are P1, 41 are P2 and 49 are P3. 10 findings were refuted and 5 merged as duplicates. No P0.

Claim scope: this directory records an audit. It admits no performance, accuracy, ranking or release claim for the solver.
