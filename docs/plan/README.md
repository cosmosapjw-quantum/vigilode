# Production development plan

`PRODUCTION_PLAN_20261020.md` is the Markdown export of the planning document
"VigilODE Production Solver Development Plan" (Claude Docs artifact,
exported 2026-10-21 KST). It is the authority for the phases, workstreams,
kill gates and theorems T1–T9 that the `production/*` branches implement.

A block of unrelated text (a hand-off prompt for the separate `rec_bianchi`
repository) that had been pasted into the middle of section 5, T9, was
removed before committing; nothing else was changed.

Execution log: `docs/plan/EXECUTION_LOG.md` (one row per pushed node).
- `handoff/LOCAL_TEST_HANDOFF_W2.md` — heavy test runs delegated to the owner's workstation; results return on `results/local-wave2-<sha>` branches.
- `IMPLEMENTATION_WAVE3.md` — wave-3 plan (9 tasks: T0 residuals, T1–T6 parallel, T7 p0-api, T8 integration); heavy runs via LOCAL_*_W3.md handoffs.
- `handoff/LOCAL_TEST_HANDOFF_W3.md` — wave-3 full suites at 4945683; results on `results/local-wave3-<sha>`. E1b measurement handoff lives on branch production/e1b-dense (`docs/plan/handoff/LOCAL_E1B_HANDOFF_W3.md`, SHA 33e60cf).
- `sdd/` — SDD records (ledgers, briefs, reports, reviews) for waves 1–3.
- `WAVE3_REPORT.md` — wave-3 close-out report (verdicts, E1b, B1c, global-error observation, workstation results).
- `OPEN_ITEMS.md` — consolidated parked/deferred items, waves 1–3.
- `IMPLEMENTATION_WAVE4.md` — wave-4 plan (base f2f20c1; Tasks 0–7; ledger from L-0143).
