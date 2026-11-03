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
