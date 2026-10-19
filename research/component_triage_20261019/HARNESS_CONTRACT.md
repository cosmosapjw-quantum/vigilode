# Component triage harness contract (CT-HARNESS v1)

This file fixes the interfaces between the Rust workload binary, the allocation
binary and the Python tooling of the component triage campaign
(`research/component_triage_20261019`). Everything that judges a component reads
raw rows produced under this contract. No tool may write a judgement that it did
not recompute from raw rows (PA-03).

## 1. Rust workload: `rodas5p` subcommands

All subcommands live in `crates/rodas5p-cli/src/triage.rs` (cells, rows,
problems) and `crates/rodas5p-cli/src/triage_arms.rs` (arm registry). The
existing `stiff_benchmark.rs` problem builders and arms are reused; nothing in
the existing subcommands changes behaviour.

### `rodas5p triage-list`

Prints one JSON document:

```json
{
  "schema": "vigilode-triage-list-v1",
  "arms": [
    {"id": "seq-direct", "components": ["drv.seq_direct", "la.faer_lu", "ctl.integral", "out.collector"],
     "wired": true, "unwired_reason": null,
     "requires": {"jacobian": "analytic", "structure": null, "mass": false, "fixed_dimension": null}}
  ],
  "problems": [
    {"id": "robertson", "tier": "G", "dimension": 3, "structure": "dense", "jacobian": "analytic",
     "jvp": true, "mass": false, "exact_solution": false, "oscillatory": false,
     "t_span": [0.0, 40.0], "atol_scale": 1e-4, "breakpoints": [], "source": "rodas5p_integrators::problems::robertson_problem"}
  ]
}
```

- `requires.jacobian` is one of `analytic`, `jvp`, `any`. `structure` is `null`,
  `banded`, `sparse_linear_part`. `fixed_dimension` is set for compile-time-N arms.
- An arm whose integrator entry point cannot be called from the CLI is still
  listed with `wired: false` and a reason. It is never silently omitted.
- A problem is applicable to an arm iff every `requires` field is satisfied.

### `rodas5p triage-run`

```
rodas5p triage-run --arm ARM --problem PROBLEM --rtol RTOL \
    --grid {endpoint|uniform101} --h0-scale {0.5|1|2} [--repetitions K] [--campaign ID]
```

Runs one cell `K` times (default 1) in-process and prints one row (below) for
the first run, with `deterministic` comparing all K final output arrays
bitwise. Exit code 0 whenever a row was printed, including solver failures and
refusals. Exit code 2 only for unknown arm/problem or inapplicable pair.

- `atol = rtol * atol_scale` (scalar; the solver has no component atol vector,
  so every arm, internal and external, uses this scalar atol).
- `grid = endpoint`: output times `[t0, tf]`. `uniform101`: 101 equispaced
  output times including both ends. Breakpoints of segmented corpus problems
  are integration-segment boundaries, not outputs.
- `h0 = h0_scale * h0_base`, `h0_base = 1e-6 * (tf - t0)` unless the problem
  declares its own base. `max_step = tf - t0`, `min_step = 1e-14`,
  `max_attempts = 1_000_000`; all other `AdaptiveStepConfig` fields default
  unless the arm itself selects them (e.g. a controller arm).

### Row schema `vigilode-triage-row-v1`

```json
{
  "schema": "vigilode-triage-row-v1",
  "campaign": "ct-20261019-smoke",
  "arm": "seq-direct",
  "components": ["drv.seq_direct", "la.faer_lu", "ctl.integral", "out.collector"],
  "problem": "robertson", "tier": "G", "dimension": 3,
  "rtol": 1e-6, "atol": 1e-10, "grid": "uniform101", "h0_scale": 1.0, "h0": 4e-5,
  "status": "ok",
  "error": null,
  "success": true,
  "attempts": 120, "accepted_steps": 110, "rejected_steps": 10,
  "output_times": [0.0, 0.4],
  "states_hex": [["3ff0000000000000", "0000000000000000", "0000000000000000"]],
  "counters": {"rhs_calls": 0},
  "extra_work": null,
  "counter_semantics": "work-counters-v2-attempted",
  "deterministic": true,
  "repetitions": 1,
  "binary": {"sha256": "...", "profile": "bench"},
  "source": {"commit": "...40hex...", "tree_clean": true}
}
```

- `status`: `ok` (integration returned and `success`), `solver_failure`
  (integration returned with `success=false`, e.g. step-size underflow,
  attempt budget), `refused` (entry point returned a typed error before any
  work), `error` (typed error after work started). A process abort (the release
  profile is `panic = "abort"`) produces no row; the Python driver records it as
  `process_abort` with the exit status.
- `states_hex`: every observed output state, each f64 as 16 lowercase hex digits
  of its IEEE-754 bits (exact and NaN-safe; decoded by
  `tools/evidence_schema_v2.py::decode_hex64`). Rows with fewer observed
  outputs than requested keep what was observed and have `success=false`.
- `counters`: every field of `rodas5p_core::WorkCounters`, serialized by serde,
  raw. Arm-specific ledgers (`BandedWork`, `AttemptWorkTotals`, ...) go in the
  optional `extra_work` object.
- `counter_semantics`: `work-counters-v2-attempted` after the DP-02 residue fix
  (callbacks counted when attempted, not only when they succeed).
- `source.commit`/`tree_clean` come from the environment variables
  `VIGILODE_SOURCE_COMMIT` and `VIGILODE_TREE_CLEAN` set by the Python driver
  (the binary does not shell out to git); `null` when unset.

### `rodas5p triage-profile-run`

Same flags as `triage-run`, with `--repetitions K` where K may be 0. Runs the
cell K times and prints a compact JSON summary (`success`, `attempts`, the
counters of the first run, `deterministic`). This is the callgrind workload:
Ir of one integration = Ir(K=2) - Ir(K=1) (warm) and Ir(K=1) - Ir(K=0) (cold),
with Ir(K=1) re-measured as `1b` for determinism. Nothing else may run inside
the measured process.

## 2. Allocation binary: `rodas5p-allocation-audit triage`

```
rodas5p-allocation-audit triage --arm ARM --problem P --rtol R --grid G \
    --h0-scale S --warmups K --windows M
```

Runs one integration as `cold`, then K untimed integrations, then M
integrations each inside its own counting window, and prints:

```json
{"schema": "vigilode-triage-alloc-v1", "arm": "...", "problem": "...", "rtol": 1e-6,
 "grid": "uniform101", "h0_scale": 1.0, "warmups": 2,
 "cold": {"allocations": 0, "deallocations": 0, "allocated_bytes": 0, "peak_live_bytes": 0},
 "windows": [{"allocations": 0, "deallocations": 0, "allocated_bytes": 0, "peak_live_bytes": 0}]}
```

Counts are raw per window; no subtraction between windows and no saturation.
`peak_live_bytes` is the peak of live bytes inside the window minus live bytes
at window start (signed i64).

## 3. Python tooling (`tools/triage/`)

| Script | Input | Output |
|---|---|---|
| `triage_manifest.py` | `COMPONENTS.json`, `CORPUS.json`, `triage-list` JSON | validation report; applicability matrix; `--dry-run` cell count and budget; `--static-reach` caller graph |
| `triage_runner.py` | manifest + cell selection | `RUNS.jsonl` (one row per cell, typed `process_abort` rows) |
| `triage_references.py` | problems | `REFERENCES.json`: per (problem, grid) reference states, radius `rho`, provenance |
| `triage_profile.py` | cells | `PROFILE.jsonl`: Ir cold/warm, determinism, optional cachegrind LL misses |
| `triage_alloc.py` | cells | `ALLOCATIONS.jsonl` |
| `triage_external.py` | cells | external rows in the row schema with `arm = ext.<solver>` and raw upstream counters in `extra_work` |
| `triage_check.py` | all of the above + `THRESHOLDS.json` | `RESULTS.json`, `CLASSIFICATION.json` |
| `triage_report.py` | `RESULTS.json`, `CLASSIFICATION.json` | `REPORT_KO.md` (generated, never edited by hand) |

Missing measurements are `null` with a reason string, never 0. Thresholds live
only in `THRESHOLDS.json`, frozen with the preregistration.
