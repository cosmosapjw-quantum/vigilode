# VigilODE NCP follow-up audit bundle (source `5fd6e459`, 2026-10-21)

This bundle holds the evidence from executing the handoff in
`research/ncp_followup_20261021/CODEX_HANDOFF_KO.md` against branch
`research/ncp-followup-fvm-20261021`. It is published for independent audit.
**Broad production remains HOLD.** All timing is `diagnostic_only` and under
`STATISTICAL_AUTHORITY_HOLD`. The hard evidence is the deterministic counters and
the full-trajectory accuracy against an independent reference.

Machine-readable report: `CAMPAIGN_REPORT_v2.json`. `CAMPAIGN_REPORT.json` is the
superseded v1 draft, kept unmodified. v2 incorporates the adversarial review in
`verification/`.

## Provenance

| item | value |
|---|---|
| source commit (pinned at fetch, tip not followed) | `5fd6e459a3f67cee90e002821b9b4b740c47475b` |
| source tree | `1df97f74d0e32e8192ba10b21337b588997f0784` (recomputed on NCP before and after build) |
| artifact manifest | 102/102 files match `ARTIFACT_MANIFEST.json` and git blobs (`MANIFEST_VERIFY.json`) |
| toolchain | rustc/cargo 1.94.1 (`RUSTUP_TOOLCHAIN=1.94.1`), existing `Cargo.lock`, `--locked --offline` |
| `component_benchmark_native` | sha256 `47f0ecbe46fa79defc790c39be772d2376af399e2d14a6d2b40ea137bef5467a` |
| `bordered_probe` | sha256 `d4e80f17ec01e87f72f57e07003aa7ce91428bb94749677d921df63ac25a4d3d` |
| `diffsol-adapter-v2` (archived binary) | sha256 `8042284844ef5ea6ca512290ef0ca0926555820c3f5a03b5af728acc1eddd220`, equal to the archived `DIFFSOL_SHA256SUMS.txt` entry |
| NCP host | KVM guest, Xeon Gold 5220, 64 vCPU visible. Campaign ran 2026-10-21 10:31:00Z to 10:47:51Z |

The binaries are not committed. They are reproducible from the commit with the
`b1` and `b2` commands in `CAMPAIGN_REPORT_v2.json`, and their hashes are recorded
above and in `ncp_campaign/BINARY_REQUEST_HARNESS_SHA256.txt`.

## Layout

- `local/`: `run_followup_checks.py` executed on the pinned source on the
  workstation. Result: PASS_SCOPED 9/9; Rust tests 19 (core 4, krylov 1,
  adapter 6, remap 8); Python tests 15. The `remap_tests` binary is omitted.
- `ncp_campaign/`: NCP campaign directory, with `bin/` omitted.
  - `results/`: `SMOKE.jsonl`, `PAIRS_A0_c08.jsonl`, `PAIRS_A_c16..c31.jsonl`,
    `COMP_<ARM>.jsonl`, `BORDERED.jsonl`, `TELEMETRY.jsonl`, `REFERENCE.json`,
    `APPLICATION_BUDGET.json`, `ANALYSIS_SUMMARY.json`.
  - `raw/<phase>/<run_id>.json`: complete solver stdout for every run, including
    all output times and states.
  - `harness/`: the scripts that produced the rows (`common.py` holds the
    evaluator and `run_child`). `julia_solver_v3.jl` is the archived v2 plus
    `sol.t` and separately named stats; its diff is in `julia_solver_v2_to_v3.diff`.
  - `logs/`, `host/`: build logs, phase log, per-phase harness hash snapshots, and
    host snapshots.
- `ncp_posthoc/`: post-campaign identity re-check: tar commit id, tree after
  build, interpreter hashes, and a full `ps`.
- `verification/`: raw output of three independent adversarial verifiers
  (numerics, pairing/timing, identity/coverage), plus their re-derivation scripts.

## How to verify

```sh
cd ncp_campaign
# bin/* and harness/__pycache__/*.pyc are intentionally absent; every other file must match
sha256sum -c --ignore-missing OUTPUT_SHA256SUMS.txt
cd .. && sha256sum -c SHA256SUMS.txt
python3 ncp_campaign/harness/analyze.py ncp_campaign   # rebuilds results/ANALYSIS_SUMMARY.json
```

`OUTPUT_SHA256SUMS.txt` was written on NCP. `ANALYSIS_SUMMARY.json` was later
regenerated locally with byte-identical content.

## Problem and acceptance

The problem is `periodic_heat`, n = 128, D = 0.01·n², integrated over t ∈ [0, 1]
with 41 output times. The application budget comes from the original request:
rtol 1e-6, atol 1e-9.

The acceptance metric is max over all times (including t = 0) and all components
of |y − ref| / (atol + rtol·|ref|) ≤ 1.

The reference is an exact Fourier solution of the float64 IVP in mpmath. A
verifier re-derived it independently with mp Taylor propagation of exp(Ah).
Internal solver tolerances equal the application budget for every arm.

## Headline results

| arm | max scaled error (only \|ref\|>1e-3) | budget | counters |
|---|---|---|---|
| VigilODE basis-JVP (BASE) | 1.21e-7 (~6.7e-9) | pass 524/524 | 42 steps, rhs 336 + 5376 derivative RHS |
| VigilODE analytic stencil (STEN) | identical bits to BASE | pass 569/569 | 42 steps, rhs 336 + 0 derivative RHS |
| VODE MF20 (archived functional BDF) | 0.990 | pass, 1% margin | NST 1060, NFE 2115 |
| VODE MF21 (analytic Jacobian BDF) | 4.72 | **fail** | NST 23, NLU 12 |
| VODE MF22 (FD Jacobian BDF) | 14.65 (5.97) | **fail** | NST 28, NFE 307 |
| Julia Rodas5P | 1.30e-3 | pass | 5 steps |
| diffsol BDF | 5.49e-3 (states only; times not returned) | pass | not exposed |

The BASE and STEN pass counts include the paired phases, the smoke runs and the
STEN runs inside the comparator blocks.

Paired STEN/BASE blocks: 9 in A0 (one core) and 480 in A (16 independent
single-thread workers, not a multi-thread run). STEN is faster in all 489 blocks.
The median ratios are 0.586 for warmed solve time, 0.620 for process wall and
0.616 for CPU. These ratios are descriptive only, because the blocks were run
concurrently on one host and are not independent.

Cross-solver timer scopes differ (see `timing_scopes` in the report). Process-wall
ratios across solvers do not measure solver speed. Ratios for arms that fail the
budget are suppressed.

`bordered_probe` is a manufactured linear solve, not an ODE or BASS benchmark.
Residuals are 1.1e-13 (n = 1600) and 8.9e-16 (n = 4096). Vec lengths equal the
design formula 8n + 4.

## Verdicts, coverage, deviations

See `verdicts`, `coverage.NOT_RUN` and `deviations` in `CAMPAIGN_REPORT_v2.json`.

**Not run in this pass:**
- the VG-1 allocation gate
- heat at n = 32, 512, 1600 and 4096
- banded and matrix-free arms
- tightened internal-tolerance arms
- a full-trajectory rerun of the rotation case

**Disclosed deviations:**
- Julia loads touched the mtime of 3 cache files in a prior campaign's depot.
  Sizes are unchanged; the pre-change hashes were not captured.
- The harness was authored during the campaign. The evaluator was frozen before
  any measurement.
- An external host job ran on cpu63 from 10:46:39Z, overlapping Julia blocks
  b06–b08 and Phase C.

No research ledger row was appended. This bundle is evidence for audit, not a
node closure.
