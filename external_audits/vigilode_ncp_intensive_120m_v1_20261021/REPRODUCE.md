# Reproduction and artifact map

Protocol: `vigilode_ncp_intensive_120m_v1`. Source commit/tree are fixed in `IDENTITY.json`. The original NCP checkout remains at `/root/vigilode-reaudit-20261021T0549Z/checkout`; campaign directory was `/dev/shm/vigilode_ncp_intensive_120m_v1_20261021T062109Z`. The returned local copy is the current artifact directory. Do not re-run into this directory or overwrite raw streams.

## Reproduce

See `CONFIG.json` and `harness/`. Tool versions and installed dependencies are in `DEPENDENCIES.json`; host topology and initial pinning caveats are in `HOST.json`, `HOST.lscpu.json`, `HOST.cgroup-limits.txt`, `HOST.memory.txt`, and disk snapshot files. Exact commands and worker receipts are preserved in `logs/` and `harness/`. The campaign was executed once, with screening, confirmation, throughput, tolerance, nonlinear, and recovery blocks. Raw files retain their initial bytes. Files ending `_RECOVERED` are derived parsing/aggregation products; original malformed/partial sources are preserved and their issue is documented in `DIFFSOL_RECOVERY.json` / `ROTATION_TOLERANCE_SUMMARY.json`.

## Identity / validation

`PACKAGE_VERIFY.json` status PASS means file identity and coverage checks only. Source commit/tree: `FINAL_COMMIT.txt`, `FINAL_TREE.txt`; clean status snapshot: `FINAL_SOURCE_STATUS.txt`. No source or Cargo.lock changes were made by this campaign. The experiment-specific diffsol scratch crate is isolated under `harness/diffsol-adapter/`.

## Main evidence

- Screening raw: `RAW_RESULTS.jsonl`; `SCREENING_COVERAGE.json`.
- Confirmation raw plus corrected rotation branch: `CONFIRMATION_RESULTS.jsonl`, `CONFIRMATION_ROTATION_V2.jsonl`, derived concatenation `CONFIRMATION_RESULTS_RECOVERED.jsonl`.
- Julia: `JULIA_RESULTS.jsonl`; diffsol original/recovered plus `DIFFSOL_RECOVERY.json`.
- Throughput original and correction: `THROUGHPUT_*`; tolerance original and recovered rotation: `TOLERANCE_RESULTS.jsonl`, `ROTATION_TOLERANCE_*`.
- Nonlinear runs/invariants: `NONLINEAR_CONFIRMATION_*`.
- Host samples: `HOST_TELEMETRY.jsonl`; stop-file parse recovery kept separate.
- Human/machine analysis: `REPORT_KO.md`, `SUMMARY.json`, `DECISIONS.json`, `OPTIMIZATION_PLAN.json`, `PARETO_FRONTIERS.json`, `COVERAGE.json`, `WORK_PRECISION_SUMMARY.json`, `figures/`.
- `GATE.json` is intentionally absent: this campaign protocol is not input to the frozen production promotion gate. broad production remains HOLD.

Raw hashes are in `RAW_SHA256SUMS.txt`; harness, adapter, request, patch, binary, and figure digests are in `HARNESS_SHA256SUMS.json`. Hash manifests do not hash themselves.
