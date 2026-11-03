# Progress log — switcher v2 remediation

Append-only (the gate rejects edits to earlier lines). One entry per task, format in DESIGN.md section 13.1.

## G0 — Verify the environment and the baseline at the design commit — 2026-11-03
- result: done
- commit: uncommitted
- toolchain: pinned 1.94.1
- commands: gate-validate: 0, contracts: 0, vigilode-suite: 0, clippy: 0, fmt: 0
- red: none
- mutation: none
- notes: full suite passed in 131s; skeleton helpers reproduced registered counters (rotation 892 attempts, stiff-forced 66)

## G1 — Supersede ledger row L-0206 with INVALID and keep the as-preregistered rerun — 2026-11-03
- result: done
- commit: uncommitted
- toolchain: pinned 1.94.1
- commands: reproduce: 0, research-node-check: 0
- red: none
- mutation: none
- notes: reproduced preregistered rerun verdict FAIL with transition39_n2_mf relative_error 0.5015; appended L-0207 INVALID


## G2 — Re-scope ledger row L-0205 to default-path neutrality — 2026-11-03
- result: done
- commit: uncommitted
- toolchain: pinned 1.94.1
- commands: research-node-check: 0
- red: none
- mutation: none
- notes: output hashes of L-0205 verified; appended row L-0208 PASS superseding L-0205 to narrow claim to default-path neutrality
## F1 — Restore adaptive_evidence without loss-margin backoff (A-4) — 2026-11-03
- result: done
- commit: uncommitted
- toolchain: pinned 1.94.1
- commands: contracts: 0, lib-unit-tests: 0, vigilode-suite: 0, clippy: 0, fmt: 0
- red: f1_adaptive_evidence_factory_has_no_loss_margin_backoff (failed on base)
- mutation: caught 2, missed 0, waived 0
- notes: restored EntryProbeAdaptation::default() in adaptive_evidence; route_registry retains backoff
## F2 — Convert WorkModel V2 ceilings to callback units (A-5) — 2026-11-03
- result: done
- commit: uncommitted
- toolchain: pinned 1.94.1
- commands: contracts: 0, lib-unit-tests: 0, vigilode-suite: 0, clippy: 0, fmt: 0
- red: f2_v2_entry_ceiling_stops_losing_rotation_pilots_like_v1 (failed on base)
- mutation: caught 18, missed 0, waived 0
- notes: armed V2 pilot ceilings in callback units; pilot decisions match V1 economics
## F3 — route_registry uses WorkModel V1 until E1 is re-run (A-1) — 2026-11-03
- result: done
- commit: uncommitted
- toolchain: pinned 1.94.1
- commands: contracts: 0, lib-unit-tests: 0, vigilode-suite: 0, clippy: 0, fmt: 0
- red: f3_route_registry_defaults_to_work_model_v1 (failed on base)
- mutation: caught 2, missed 1, waived 1
- notes: route_registry defaults to WorkModel V1; updated admission contract assertion
## F4 — Sound progress screen for RODAS5P: c_min = 7 callbacks (A-6) — 2026-11-03
- result: done
- commit: uncommitted
- toolchain: pinned 1.94.1
- commands: contracts: 0, lib-unit-tests: 0, vigilode-suite: 0, clippy: 0, fmt: 0
- red: f4_progress_reach_upper_bound_boundary_table (does not compile on base)
- mutation: caught 31, missed 0, waived 0
- notes: implemented progress_reach_upper_bound with c_min = 7 callbacks; updated economy contracts
## F5 — Keep the B7 first-batch rule; document why and pin it (A-7) — 2026-11-03
- result: done
- commit: uncommitted
- toolchain: pinned 1.94.1
- commands: contracts: 0, lib-unit-tests: 0, vigilode-suite: 0, clippy: 0, fmt: 0
- red: none
- mutation: none
- notes: documented B7 first-batch rule and pinned with contract test
