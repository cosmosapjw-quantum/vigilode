# Switcher Admission 2026-11-03 — Preregistration (E0 Admission Gate)

*Prospective admission of post-audit adapter hardening commits (68a28357, 206298b4, 78bb8775, 5c27e3af, 7fce5a4c) and Phase 0/1 speed hardening under the switching-economy protocol.*

- **Date**: 2026-11-03
- **Branch**: `research/switcher-v2-20261103` (ancestor: `569ce026`, base reference: `46939bbe` / `596dcdfa`)
- **Protocol**: `research/adapter_switcher_economy_20261026/tools/` campaign harness
- **Classification**: Confirmatory admission gate (Node E0 of Switcher v2 Plan)

---

## 1. Question

Do the post-audit adapter commits (`68a28357` strict comparisons & dominated-comparator stop, `206298b4` checkpoint-seeded entry, `78bb8775` production setup & strict return, `5c27e3af` FD scratch reuse, `7fce5a4c` output ownership) and Phase 0/1 bitwise-safe speed hardening prospectively maintain:
1. Exact bitwise identity on all untouched direct solver paths (G1)?
2. Monotonic non-increasing deterministic work on all registered switching cells (G2)?
3. All registered numerical accuracy caps, solver success, and multi-run determinism (G3)?
4. All 12 switching-related contract test suites (81 `#[test]` functions) in `crates/vigilode/tests`?

---

## 2. Benchmark Corpus and Cells

### Direct Paths (G1 Control Arm — 9 cells)
1. `bench_rotation_dopri`
2. `bench_rotation_rodas`
3. `bench_stiff-forced_dopri`
4. `bench_stiff-forced_rodas`
5. `suite_c5_rodas5p-mf` (Case 5, matrix-free)
6. `suite_c5_rodas5p-dense` (Case 5, dense)
7. `suite_c10t_dopri5` (Case 10 Gyromotion, tight tolerance `rtol=1e-9, atol=1e-12`)
8. `suite_c10t_rodas5p-mf` (Case 10 Gyromotion, tight tolerance `rtol=1e-9, atol=1e-12`)
9. `suite_c10_dopri5` (Case 10 Gyromotion, default tolerance)

### Switching Paths (G2/G3 Test Arm)
1. `bench_rotation_evidence`
2. `bench_rotation_screened-reset`
3. `bench_rotation_pressure-only`
4. `bench_stiff-forced_evidence`
5. `bench_stiff-forced_screened-reset`
6. `bench_stiff-forced_pressure-only`
7. `suite_c5_switching` (Case 5, switching)
8. `suite_c10t_switching` (Case 10 Gyromotion tight, switching)
9. `suite_c10_switching` (Case 10 Gyromotion default, switching — reported only, known phase drift)

### Switching Contract Test Suites (12 files)
- `crates/vigilode/tests/switching_contracts.rs`
- `crates/vigilode/tests/evidence_switch_contracts.rs`
- `crates/vigilode/tests/return_sentinel_contracts.rs`
- `crates/vigilode/tests/comparator_dominance_contracts.rs`
- `crates/vigilode/tests/strict_return_work_contracts.rs`
- `crates/vigilode/tests/frontier_fatal_session_contracts.rs`
- `crates/vigilode/tests/switching_persistent_explicit.rs`
- `crates/vigilode/tests/production_setup_contracts.rs`
- `crates/vigilode/tests/adaptive_entry_contracts.rs`
- `crates/vigilode/tests/entry_checkpoint_seed_contracts.rs`
- `crates/vigilode/tests/frontier_switch_contracts.rs`
- `crates/vigilode/tests/switching_economy_contracts.rs`

---

## 3. Pass / Kill Gates

### PASS Gate
- **Gate G1 (Direct Bit-Identity)**: `G1 == true`. For every direct cell $k \in \text{DI}$, `stats` (`attempts`, `rhs_calls`, `jvp_vectors`, `linear_solves`) and `final_state` (`f64::to_bits()`) must match baseline `AFTER_596dcdfa.json` exactly.
- **Gate G2 (Deterministic Work)**: `G2 == true`. On all switching cells, `attempts`, `rhs_calls`, `implicit_pilot_discarded.attempts`, `explicit_probe_discarded.attempts`, and `implicit_committed.count` must be $\le$ baseline.
- **Gate G3 (Accuracy & Determinism)**: `G3 == true`. `success == true`, `reconciled == true`, identical results across 5 timed runs, and accuracy cap met on all cells where the cap is attainable.
- **Unit/Contract Suites**: All 12 switching contract files pass with 0 failures.

### KILL Gate
- Any bitwise discrepancy on any direct path cell.
- Any regression in accuracy cap on any attainable cell.
- Any test failure in the switching contracts suite.

---

## 4. Sealed Holdout Isolation

In strict compliance with `docs/HOLDOUT_HYGIENE.md` and `AGENTS.md` §2.6:
- Sealed holdout profiles (`holdout-512`, `enforced-budget-holdout-320`, `stage-growth-holdout-384`) are NOT executed.
- Sealed artifacts (`tools/reference_v2/artifacts/*-holdout-v2.json`) are NOT opened or inspected.
- Only the pre-registered open calibration cells defined above are evaluated.

---

## 5. Execution Command

```bash
cargo build --release --locked --offline --example evidence_switch_bench --example local_bottleneck_suite
bash research/adapter_switcher_economy_20261026/tools/run_campaign.sh target/release research/switcher_admission_20261103/raw 0
python3 research/adapter_switcher_economy_20261026/tools/analyze.py research/switcher_admission_20261103/raw research/switcher_admission_20261103/AFTER.json target/release $EXEC_SHA
python3 research/adapter_switcher_economy_20261026/tools/compare.py research/adapter_switcher_economy_20261026/evidence/AFTER_596dcdfa.json research/switcher_admission_20261103/AFTER.json research/switcher_admission_20261103/COMPARISON.md research/switcher_admission_20261103/GATES.json
```


---

## 6. Execution Results (Node E0 Admission PASS)

- **Execution Commit (`EXEC_SHA`)**: `a4b08c5970247ab4fc435bb6ef269518fc1d3c3d` (reachable on branch `origin/research/switcher-v2-20261103`)
- **Direct Bit-Identity Gate (G1)**: **PASS** (`G1 == true`). All 9 direct paths (rotation dopri/rodas, stiff-forced dopri/rodas, case5 rodas5p-mf/dense, case10t dopri5/rodas5p-mf, case10 dopri5) are bit-identical in all work counters (`attempts`, `rhs_calls`, `jvp_vectors`, `linear_solves`) and final state (`float::to_bits()`) against baseline `AFTER_596dcdfa.json`.
- **Deterministic Work Gate (G2)**: **PASS** (`G2 == true`). Deterministic work across all switching cells is $\le$ baseline.
- **Accuracy & Determinism Gate (G3)**: **PASS** (`G3 == true`). All accuracy caps met on attainable cells, solver success, reconciled records, multi-run determinism confirmed across 5 timed runs.
- **Switching Contracts Suite**: 12/12 contract test files in `crates/vigilode/tests` pass with 0 failures (81 tests).
- **Verdict**: **PASS**. Confirmatory admission of post-audit adapter hardening commits (`68a28357`, `206298b4`, `78bb8775`, `5c27e3af`, `7fce5a4c`) and Phase 0/1 speed hardening (`B9a`, `C8`).
