# Local follow-up reproduction and receipt index

All commands below ran in `/tmp/vigilode-adapter-review.rjwwJ4Cn/checkout` from measurement commit `d914b4c8a53405ea846f710e07f77fff5f45b16f`, tree `438b9602dae0728121363fa0a40a8acb58bfb2c3`. The original checkout `/home/cosmosapjw/vigilode` was not altered. No NCP, SSH, or cloud command was run. The target was `/tmp/vigilode-adapter-local-target-20261025`; new raw remains `/tmp/vigilode-adapter-local-followup.1PRDjIEs`. Existing raw remains at the path and archive in `BACKUP_RECEIPT_N01.json`.

## Focused build and tests

Rust 1.94.1, Cargo jobs 2, locked offline dependency graph. The exact focused switching and VG1 commands, stdout, stderr and exit 0 are in `tests/switch_budget_repair/` and `tests/vg1_enabled/`. The release measurement build was:

```bash
timeout 1800 env CARGO_TARGET_DIR=/tmp/vigilode-adapter-local-target-20261025 CARGO_BUILD_JOBS=2 cargo build --locked --offline --profile measurement -p vigilode --example adapter_phase_grid --example adapter_attempt_trace --example adapter_gyro_grid --example evidence_switch_bench --example local_bottleneck_suite
```

Build exit 0, stdout/stderr: `tests/build_final_measurement.*`. The five executable SHA-256 values are in `BINARY_SHA256_FINAL.json`; the host, branch, lockfile and compiler record is `IDENTITY.json`. One case5 external-reference smoke ran before the campaign, exit 0, with local command/stdout/stderr files `case5_smoke.*` under the new raw root.

## New bounded runs

The following command ran once with exit 0; the script creates its result directory exclusively and cannot overwrite it:

```bash
python3 -B /tmp/vigilode-adapter-local-followup.1PRDjIEs/run_narrow.py
```

It ran 5 cohorts × 2 candidates × (warmup 1 + timed 5) = 60 sequential processes, each with timeout 90 seconds, CPU affinity 0 and one solver thread. `narrow_measurement/INDEX.json` plus each `.receipt.json`, `.stdout`, `.stderr` and `.resource` are locally retained. The 50 timed rows were converted to `SELECTION_WINDOWS.json` by:

```bash
python3 -B external_reports/adapter_local_followup_20261025/make_selection_packet.py /tmp/vigilode-adapter-local-followup.1PRDjIEs /tmp/vigilode-adapter-review.rjwwJ4Cn/checkout /tmp/vigilode-adapter-local-followup.1PRDjIEs/SELECTION_WINDOWS.json
python3 -B research/adapter_research_20261025/selection_benchmark/analyze_selection.py /tmp/vigilode-adapter-local-followup.1PRDjIEs/SELECTION_WINDOWS.json --output /tmp/vigilode-adapter-local-followup.1PRDjIEs/SELECTION_ANALYSIS.json
```

Both exit 0, stdout/stderr/exit in `tests/`; the analyzer returned 25 `COMPARABLE` blocks, zero rejected rows, and `performance_claim_authorized=false`. Analyzer inputs and outputs are copied here. These commands have create-only output behavior: a fresh replay must use fresh paths. `run_narrow.py` contains this host's absolute paths, so adapt a copy when replaying elsewhere and record the new source/binary/reference identity.

Further source-locked diagnostics ran `adapter_attempt_trace --case 5/12 --max-attempts 2000`, `adapter_phase_grid --mode both --grid-points 129 --physical-atol 1e-6`, and `adapter_gyro_grid` with default and tight tolerances. They produced 5 exit-0 local files under `final_diagnostics/`; their commands, hashes and binary SHA are in `final_diagnostics/INDEX.json`. `check_analytic_reference.py` and `analyze_attempt_trace.py` generated the compact published summaries. The first KS linear-limit analysis output with a sign error and its corrected retry are retained locally as `analyze_attempt_trace.stdout` and `analyze_attempt_trace_retry.stdout`, respectively. The 24-process `narrow_persistence/` control compares `evidence` with `screened-reset` for rotation and stiff-forced; every attempt is indexed and hashed locally.

`RAW_MANIFEST_FINAL.json` inventories 358 local files and their hashes, including five binaries, with zero missing/index-hash mismatches. It excludes the later derived `SELECTION_WINDOWS.json`, `SELECTION_ANALYSIS.json`, `ENDPOINT_ANALYTIC_CROSSCHECK.json`, and this publication packet; those are bound by this Git commit. The historical raw archive remains separately protected by `BACKUP_RECEIPT_N01.json` and `REUSED_EVIDENCE.json`.
