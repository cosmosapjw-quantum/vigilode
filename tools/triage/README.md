# Component triage tools

Tools of the component triage campaign (`research/component_triage_20261019`). The interfaces of the Rust
binaries are fixed by `HARNESS_CONTRACT.md` (CT-HARNESS v1). Every threshold is read from `THRESHOLDS.json`.
Every judgement is recomputed from raw rows by `triage_check.py` (PA-03). A missing measurement is `null` with
a reason string. It is never 0 and never counts as a pass.

All tools are Python 3 standard library except `triage_references.py`, `triage_parity.py` and the SciPy arms
of `triage_external.py`, which need NumPy and SciPy. Every tool has `--help`. Every output is
byte-deterministic for fixed inputs. A tool refuses to overwrite an existing output unless given `--force`.
The JSONL producers (runner, profile, alloc, external) append and resume instead: a cell already present (same
campaign, arm, problem, rtol, grid, h0_scale) is skipped. Rows are written in plan order even with `-j N`.
With these producers, `--force` truncates the file and starts over.

## Pipeline

```
N=research/component_triage_20261019; C=ct-20261019-main
python3 tools/triage/triage_manifest.py --list LIST.json --dry-run --seconds-per-cell runs=0.5,profile=40,alloc=3
python3 tools/triage/triage_manifest.py --list LIST.json --static-reach $N/STATIC_REACH.json
python3 tools/triage/triage_references.py --corpus $N/CORPUS.json --binary BIN --output $N/REFERENCES.json
python3 tools/triage/triage_parity.py --binary BIN --corpus $N/CORPUS.json --output $N/PARITY.json \
    [--diffsol-binary DIFFSOL_BIN [--diffsol-only]]
tools/triage/host_prep.sh --output $N/HOST.json            # wall-clock sessions only (diagnostic)
python3 tools/triage/triage_runner.py --binary BIN --corpus $N/CORPUS.json --campaign $C --output $N/RUNS.jsonl -j 4
python3 tools/triage/triage_runner.py ... --output $N/PROBES.jsonl   # probe rows, see "PROBES.jsonl"
python3 tools/triage/triage_profile.py --binary BENCH_BIN --corpus $N/CORPUS.json --campaign $C \
    --output $N/PROFILE.jsonl --scratch /tmp/ct-callgrind [--cachegrind]
python3 tools/triage/triage_alloc.py --binary ALLOC_BIN --list LIST.json --corpus $N/CORPUS.json --campaign $C \
    --output $N/ALLOCATIONS.jsonl
python3 tools/triage/triage_external.py --corpus $N/CORPUS.json --campaign $C --output $N/EXTERNAL.jsonl \
    [--native-driver BUILD/native_stiff] [--diffsol-binary DIFFSOL_BIN] [--diffsol-timeout 3600]
python3 tools/triage/triage_check.py --campaign $C --runs $N/RUNS.jsonl --runs $N/EXTERNAL.jsonl \
    --references $N/REFERENCES.json --profile $N/PROFILE.jsonl --allocations $N/ALLOCATIONS.jsonl \
    --probes $N/PROBES.jsonl [--timing $N/TIMING.json] [--attribution $N/ATTRIBUTION.json] [--list LIST.json] \
    --results $N/RESULTS.json --classification $N/CLASSIFICATION.json
python3 tools/triage/triage_report.py --results $N/RESULTS.json --classification $N/CLASSIFICATION.json \
    --output $N/REPORT_KO.md            # later: same command with --check (PA-06 guard)
```

`LIST.json` is the stdout of `rodas5p triage-list`. Tools that take `--binary` can produce it themselves.
`DIFFSOL_BIN` is `diffsol-triage`, built from `tools/benchmarks/diffsol/triage` (see the README there).

## Files and formats

| File | Producer | Schema |
|---|---|---|
| `COMPONENTS.json` | lead (`build_manifests.py`) | `vigilode-triage-components-v1`, plus the optional per-component keys `external_peers`, `documented_warm_allocations` and `repair_path` (nonempty string or null; absent = null) |
| `CORPUS.json` | lead | `vigilode-triage-corpus-v1`, plus the optional per-problem key `numpy_model` |
| `THRESHOLDS.json` | lead (frozen) | `vigilode-triage-thresholds-v1` |
| triage-list | `rodas5p triage-list` | `vigilode-triage-list-v1` (contract section 1) |
| manifest report | `triage_manifest.py` | `vigilode-triage-manifest-report-v1` |
| `STATIC_REACH.json` | `triage_manifest.py --static-reach` | `vigilode-triage-static-reach-v1` |
| `REFERENCES.json` | `triage_references.py` | `vigilode-triage-references-v1` |
| `PARITY.json` | `triage_parity.py` | `vigilode-triage-parity-report-v1` |
| `HOST.json` | `host_prep.sh` | `vigilode-triage-host-v1` |
| `RUNS.jsonl`, `PROBES.jsonl` | `triage_runner.py` | `vigilode-triage-row-v1`, one row per line |
| `PROFILE.jsonl` | `triage_profile.py` | `vigilode-triage-profile-v1` |
| `ALLOCATIONS.jsonl` | `triage_alloc.py` | `vigilode-triage-allocations-v1` |
| `EXTERNAL.jsonl` | `triage_external.py` | `vigilode-triage-row-v1` (arm `ext.*`) |
| `TIMING.json` | wall-clock sessions | `vigilode-triage-timing-v1` |
| `ATTRIBUTION.json` | profile attribution | `vigilode-triage-attribution-v1` |
| `RESULTS.json`, `CLASSIFICATION.json` | `triage_check.py` | `vigilode-triage-results-v1`, `vigilode-triage-classification-v1`; validated by `tools/evidence_schema_v2.py::validate_triage` |
| `REPORT_KO.md` | `triage_report.py` | generated Markdown; never edited by hand |

### Rows (`vigilode-triage-row-v1`)

These are the contract's row keys, plus the binary's optional `error_kind` (string or null; the typed
error's kind: dimension, invalid-input, non-finite, linear-solve, nonlinear-solve, coefficients, ...),
`segments` (int ≥ 1) and `segments_completed` (0..segments; equal to `segments` on an ok row). Any other
unknown key is rejected. The Python drivers may add only the keys `process`, `probe`,
`accuracy_inline` and the state-sidecar keys `states_sha256`, `states_shape` and `states_file` (which replace
`states_hex`; see "State sidecars").

- **`process_abort`:** written by `triage_runner.py` when no valid row exists for a cell. Its `status` is
  `process_abort`. It carries the arm's `components` from triage-list and `success` false. `states_hex` and
  `output_times` are `[]`. `counters`, `deterministic` and `binary.profile` are null. It adds
  `process: {kind, exit_status, signal, stderr_tail, stdout_tail}`. The `kind` is one of:
  - `abort`: a nonzero exit or a signal.
  - `usage_exit_2`: unknown arm/problem, or an inapplicable pair.
  - `unparseable_row`: exit 0 without a JSON row.
  - `cell_mismatch`: the row is for a different arm, problem, rtol, grid, h0 scale or campaign.
  - `timeout`.
- **External rows:**
  - `components` is `[]` and `counters` is null.
  - `counter_semantics` is `external-upstream-raw` and `binary.profile` is `external`.
  - `extra_work` is `{nfev, njev, nlu, upstream, ...}`, normalized from the raw upstream counters. Native
    rows also keep `raw`.
  - Diffsol rows (`ext.diffsol-bdf`, `ext.diffsol-esdirk34`, `ext.diffsol-tr-bdf2`) are printed by
    `diffsol-triage run` itself. Their `extra_work` keeps the verbatim `solver_statistics`
    (`OdeSolverStatistics`) and `rhs_op_statistics` (`OpStatistics`). It also has `derived_mapping`, the exact
    upstream source of `nfev`, `njev`, `nlu` and the step counts. `accepted_steps` is `number_of_steps`;
    `rejected_steps` is error-test failures plus nonlinear failures. `harness_counters` and `mapping_check`
    cross-check `nfev` and `nlu` against closure and LU counts.
  - `triage_external.py` checks a Diffsol row before keeping it: the cell identity, the key set, the atol
    (exactly `rtol * atol_scale`) and the output times (bit for bit the requested grid). It also reads
    `diffsol-triage list` once and compares the harness's problem definitions with CORPUS.
  - Status `unavailable` (with the reason in `error`) marks a cell the external code cannot run. Such a
    cell is written, never skipped.
  - Each (arm, problem, rtol, grid) has exactly one row, at the nominal `h0_scale` 1.0. For native and
    diffsol rows `h0` is null, because they use the solver's default first step
    (`extra_work.h0_policy = "solver_default"`).
  - Wall time is never recorded.

### State sidecars (large rows)

Rows of dimension greater than `--inline-states-max-dimension` (default 64) can move their states out of the
JSONL: `triage_runner.py` (RUNS and PROBES) and `triage_external.py` (EXTERNAL) do this when given
`--states-dir DIR` and `--references REFERENCES.json` (repeatable; `.gz` allowed).

- **Sidecar file:** `DIR/<sha256>.f64le.gz`, the `gzip -n` of the little-endian float64 matrix, row-major
  [time][component]. The sha256 is over the uncompressed bytes, so the file is content addressed and shared
  by identical states.
- **Row changes:** `states_hex` is replaced by `states_sha256`, `states_shape` = [times, dimension] and
  `states_file`, the name relative to DIR.
- **`accuracy_inline`:** `{class, E, E_rho, E_up, R, R_rho, R_up, reason, integrity, reference_sha256,
  function}`, computed by `triage_common.row_accuracy` (the checker's own code) against the matching
  reference entry. `reference_sha256` is the sha256 of that entry's canonical JSON. Without a reference the
  class is `unknown` with a reason.
- **Without `--states-dir`:** large rows keep `states_hex`, and a warning is printed.

`triage_check.py --states-dir DIR` (repeatable) takes each row's accuracy from the first source available:

1. `states_hex`: recomputed (`accuracy_source` `recomputed_hex`).
2. The row's status, for non-ok rows (`recomputed_status`).
3. A sidecar whose sha256 and shape verify: recomputed (`recomputed_sidecar`). A sidecar that fails the
   check is *refused*, never read.
4. `accuracy_inline`, if well formed, produced by `triage_common.row_accuracy`, and bound to the same
   reference entry (`runner_inline`).
5. Otherwise `accuracy: null` (`none`; class `unknown`, with a reason).

`RESULTS.accuracy_summary` counts each source plus `sidecars_missing` and `sidecars_refused`, and
`validate_triage` checks the counts.

### `PROBES.jsonl` (C3)

Probe rows are produced by `triage_runner.py --probes` (with `--list` or `--binary`; no CORPUS needed) from the
top-level `probes` array of triage-list. Each entry has problem fields plus `probe: {id, base, must_refuse,
expect}` and `applicable_arms`, computed by the binary.

- **Plan:** every wired internal arm in `applicable_arms` × the probe, at one fixed cell (rtol 1e-4, grid
  endpoint, h0 1.0). `--probe-rtol`, `--probe-grid` and `--probe-h0-scale` override the cell, and `--arm` and
  `--problem` narrow the plan.
- **Run:** `triage-run --problem <probe entry id>`.
- **Rows:** each row gets `probe: {id, must_refuse, components}`, where `components` are the arm's
  components. Process aborts become `process_abort` probe rows, as for any cell.
- **Errors:** an applicable arm unknown to triage-list is an error (exit 2).

Probe rows are ordinary rows of the row schema with the same `campaign`. Their problem ids are special probe
ids, which need not be CORPUS problems. Each row carries one extra key:

```json
{"schema": "vigilode-triage-row-v1", "campaign": "ct-...", "arm": "seq-direct", "components": ["..."],
 "problem": "probe-nan-y0", "tier": "V", "dimension": 3, "rtol": 1e-6, "atol": 1e-10, "grid": "endpoint",
 "h0_scale": 1.0, "h0": 4e-5, "status": "refused", "error": "typed error text", "success": false,
 "attempts": 0, "accepted_steps": 0, "rejected_steps": 0, "output_times": [], "states_hex": [],
 "counters": null, "extra_work": null, "counter_semantics": "work-counters-v2-attempted",
 "deterministic": true, "repetitions": 1, "binary": {"sha256": "...", "profile": "bench"},
 "source": {"commit": "...", "tree_clean": true},
 "probe": {"id": "nan-y0", "must_refuse": true, "components": ["drv.seq_direct"]}}
```

Key rules:

- `probe.id` is a string and `probe.must_refuse` is a bool.
- `probe.components` is optional; it defaults to the row's `components`. It names the components the probe
  tests.
- All row-schema type rules apply, for example `rtol` and `h0_scale` must be positive floats.
- A `process_abort` probe row, written by the runner, is the same with the `process` key added.

Verdict per probe:

- A `process_abort` row always fails.
- A must-refuse probe passes only with `status` `refused` or `error` and `success` false. Any other status,
  including `solver_failure` and `ok`, fails.
- A probe with `must_refuse: false` (fault injection: NaN/Err right-hand side, ...) must fail closed: it passes
  only with `status` `solver_failure`, `error` or `refused` and `success` false. A `success: true` row is a
  silent failure and fails C3.

Verdict per component:

- C3 = `fail` if any of its probes fails, `pass` if all pass.
- C3 = `null` if no probe covers it (C3_MISSING).

### Rust outputs not in the contract (layouts of `target/release/rodas5p` at 10ada53)

- **`rodas5p triage-exact --problem P --grid G`:** prints `{"schema": "vigilode-triage-exact-v1", "problem",
  "grid", "dimension", "output_times", "output_times_hex", "states_hex"}`. This is the closed-form solution,
  not the solver. `triage_references.py` requires `output_times_hex` to equal the reference grid bit for bit.
  At 10ada53 the binary's grids equal `triage_common.grid_times` bit for bit.
- **`rodas5p triage-parity --problem P`:** prints `{"schema": "vigilode-triage-parity-v1", "problem",
  "dimension", "autonomous", "jacobian": "analytic"|"jvp", "jvp_direction_hex": [v], "mass_hex": null|rows,
  "samples": [...]}`. Each sample is `{"t_hex", "y_hex", "f_hex"|null, "f_error": null|str, "jacobian":
  null | {"format": "dense", "rows_hex"} | {"format": "coo", "nonzeros_hex": [[i, k, hex]]}, "jvp_hex"}`.
  - The samples are y0 and a perturbation of it, at t0 and the span midpoint. Sample 0 is y0, so the model's
    initial state is checked against it bit for bit.
  - `triage_parity.py` compares f on every sample, and the Jacobian when present. When only `jvp_hex` is
    present it compares J·v instead, using the model JVP if the model has one, otherwise its Jacobian times v.
  - An `f_error` sample must also fail in the model. The mass matrix is compared too.
- **`binary.profile`:** a row reports `"release-or-bench"` with `profile_source: "inferred"`.
  `triage_profile.py` refuses such a binary unless given `--assert-bench-profile`, which is recorded in
  every row as `binary.profile_asserted: "bench"`.
- **CORPUS method names:** `reference.methods` uses `scipy-radau`, `scipy-lsoda` and `scipy-bdf`; the bare
  SciPy names are also accepted. `profile_subset.grids` and `profile_subset.h0_scales` restrict the
  profile and alloc plans.

### Other inputs and outputs

- **`TIMING.json`:** `{"schema": "vigilode-triage-timing-v1", "host": <HOST.json>?, "entries": [{"arm",
  "counterpart", "problem", "grid", "rtol", "wall_ratio", "interval": [lo, hi]}]}`. `wall_ratio` is
  wall(arm) / wall(counterpart). Timing is diagnostic: it can only raise TIMING_CONFLICT. `RESULTS.timing.diagnostic`
  is true unless `host.wall_clock_admissible` is true.
- **`ATTRIBUTION.json`:** `{"schema": "vigilode-triage-attribution-v1", "entries": [{"component", "arm",
  "problem", "grid", "rtol", "removable_ir": int, "basis": str}]}`.
  - `removable_ir` is the instructions of the arm's h0 = 1 profile cell that optimizing the component
    would remove.
  - `rtol` is the rtol of the arm's selected group in the LOSS comparison (`RESULTS.comparisons[].rtol`).
- **`PROFILE.jsonl` row:**
  - Cell keys: `campaign`, `arm`, `problem`, `rtol`, `grid`, `h0_scale` (= 1.0).
  - `status` (`ok` | `failed`) and `reason`.
  - `ir: {"0", "1", "2", "1b"}`, `ir_warm` = Ir(2)−Ir(1), `ir_cold` = Ir(1)−Ir(0), and
    `deterministic` = (Ir(1) == Ir(1b)).
  - `summary`: the K=1 JSON summary.
  - `cachegrind`: null, or `{status, reason, totals: {"1", "2"}, warm: {event: 2−1}}`.
  - `valgrind`, `binary: {sha256, profile}`, `source`.
- **`ALLOCATIONS.jsonl` row:**
  - Cell keys, plus `warmups` and `windows_requested`.
  - `status`: `ok`, `process_abort` or `invalid`, with `reason`.
  - `cold` and `windows`: the raw counts. `warm` = the per-field maximum over the windows.
  - `warm_rule`, `process` (on failure), `binary`, `source`.
- **`REFERENCES.json`:** `{schema, generator, runtime: {python, numpy, scipy, machine}, corpus_sha256,
  times_source, reference_rtol, references: [...]}`.
  - Each entry has `problem`, `grid`, `kind` (`exact` | `two_method`), `status` (`ok` | `null`) and `reason`.
  - Data: `output_times`, `states_hex` (one hex vector per time), and `rho_hex` (one hex vector per time).
  - Provenance: `rho_rule`, `methods` (`{method, rtol, atol, nfev, njev, nlu, success, message,
    segments}`), `canonical_method`, `model_source`.
  - `rho` = 2 × max over the non-canonical methods of |canonical − method|, per time and component.
- **`PARITY.json`:**
  - `limit` comes from THRESHOLDS `parity.max_relative_error`. If that key is absent, the
    `stiff_benchmark_scipy` PARITY_LIMIT is used, and `limit_source` records which.
  - Also `limit_source`, `tiny`, `runtime`, `binary_sha256` and `corpus_sha256`.
  - `problems[]`: `{problem, status: pass|fail|error|not_checked, reason, samples, rhs_max_rel,
    jacobian_max_rel, y0_match, model_source}`.
  - With `--diffsol-binary`: `diffsol: {binary_sha256, problems: [...]}`. Each entry is the same kind of record
    plus `jvp_max_rel` and `definition_match`. It compares `diffsol-triage parity`, run on the same Rust
    document, with that document: f always, the dense Jacobian when Rust has one, and J v when Rust has
    `jvp_hex`. The y0, t_span and atol_scale must match exactly. Problems the harness does not implement are
    `not_checked`. `--diffsol-only` skips the NumPy models: `problems` is `[]`, `numpy_models` says so, and
    SciPy is not needed.

### RESULTS.json and CLASSIFICATION.json

- **`RESULTS.json`:**
  - Header: `schema`, `campaign`, `generator`.
  - `inputs`: per role, `{name, sha256}` or null; `runs` is a list.
  - `thresholds`: a copy of THRESHOLDS. Also `ignored_rows_other_campaigns`.
  - `references[]`: `{problem, grid, status, reason, E_rho, reference_limited_targets}`.
  - `rows[]`: `{arm, problem, grid, rtol, h0_scale, status, external, class, E, E_rho, E_up, R, R_rho, R_up,
    reason, integrity}`.
  - `groups[]`: `{arm, problem, grid, rtol, external, targets: [{target, state, reason}], E_up_max,
    R_up_max, R_rho_max, in_profile_subset, cost, cost_reason, secondary, secondary_reasons, counted_work,
    counted_work_reason}`.
  - `comparisons[]`: `{component, arm, kind, counterpart, problem, grid, target, tier, partition, outcome,
    ratio, cost, counterpart_cost, rtol, counterpart_rtol, reason, secondary}`.
  - `specialized_rivals[]`: the same shape, with kind `specialized_rival` (comparisons against narrow rivals;
    informational).
  - `external_gaps[]`: `{component, arm, peer, problem, grid, target, work, peer_work, rtol, peer_rtol, gap,
    reason}`.
  - `timing`: `{present, diagnostic, entries}`.
- **`CLASSIFICATION.json`:**
  - Header: `schema`, `campaign`, `generator`, `inputs` (identical to RESULTS), and `results_sha256` (the
    SHA-256 of the RESULTS bytes).
  - `summary`: the counts per label, plus `provisional` and `components`.
  - `components[]`: `{id, family, slot, loc, reach, label, prune_kind, provisional, blocked_by, flags,
    predicate, rule_path, evidence, criteria: {c1, c2, c3, allocation, promote, specialize, optimize, timing},
    reasons, insufficient_evidence}`.

## Measures

- **Accuracy:** computed on `ok` rows only. The output times must equal the reference times bit for bit.
  - E = max over output times of the WRMS of (y − ref)·w, with w = 1/(atol_scale_P + |ref|).
  - E_rho is the same norm of rho, and E_up = E + E_rho.
  - R, R_rho and R_up are the same with w = 1/(atol_row + rtol_row·|ref|).
  - Non-finite or malformed `ok` rows are integrity failures (C1).
- **Groups:** a group is (arm, problem, grid, rtol). At target ε it is admissible iff every required row is
  `ok` with E_up ≤ ε.
  - Required rows: internal arms need every h0 in `accuracy.h0_scales_required`. External arms need only
    their single nominal h0 = 1 row (`THRESHOLDS.external.admissibility`).
  - A group is inadmissible if a required row is not ok or exceeds ε. It is unknown if a row or the
    reference is missing.
  - (problem, grid) is reference-limited at ε when E_rho > `reference_limited_factor`·ε.
- **Cost:** the PROFILE `ir_warm` of the group's h0 = 1 cell. Only groups in the CORPUS `profile_subset`
  are candidates.
  - The configuration cost at ε is the minimum over admissible candidates, ordered by (cost, E_up, rtol).
  - It is *incomplete* if an admissible candidate has no cost or a candidate group is unknown.
  - Secondary costs are reported separately and never combined.
- **Counted work (OPTIMIZE b):** the sum of the `THRESHOLDS.external.counted_work` units on the nominal row.

  | unit | internal WorkCounters | external `extra_work` |
  |---|---|---|
  | `rhs_evaluations` | `rhs_evaluations` | `nfev` |
  | `jacobian_evaluations` | `jacobian_builds` | `njev` |
  | `lu_factorizations` | `direct_factorizations` | `nlu` |

  - The arm's work at ε is the minimum over admissible groups.
  - A cell is excluded, with its reason, when a unit is unavailable on either side or a group is unknown.
  - Gap = work(component arm) / work(peer) per (problem, grid, ε). The component-level value is the median
    over matched cells.
- **Comparisons:** for each arm a containing component c:
  - `attribution`: every arm a′ that equals a outside c's slot and does not contain c. This covers a slot
    whose content is replaced (one arm may hold several members of a slot, e.g. the fast driver's Legacy LU
    policy) and an opt-in feature switched off.
    If a′ is not applicable to the problem, there is no rival for that cell.
  - `strongest`: used when no a′ exists. The rival is the cheapest applicable arm that contains another
    slot member and not c, excluding *narrow* rivals.
    - A rival is narrow when its arm-level coverage, computed over universe(c) of the component being judged,
      is below `applicability.coverage_min`.
    - Comparisons against narrow rivals go to `RESULTS.specialized_rivals` (kind `specialized_rival`). They
      are routing data and never enter PROMOTE/PRUNE fractions.
    - Attribution counterparts are not filtered.
  - Outcomes for ratio = cost(a)/cost(rival):
    - GAIN when ratio ≤ `gain_max`; NEUTRAL when ratio ≤ `neutral_max`; LOSS otherwise.
    - `capability_gain` / `capability_loss`: only one side is admissible.
    - `excluded`, with a reason: neither side is admissible, the reference is missing or limited, a state
      is unknown, or a cost is missing or incomplete.
  - Only problems whose tier is in c's `tiers` are compared.

## Decision tree (as implemented in `triage_check.py::classify`)

Each step appends `{rule, kind, detail}` to `rule_path`. The last *deciding* rule fixes the label, and
`validate_triage` checks this. The steps run in this order:

1. **Provisional causes**, recorded first:
   - `BLOCKED_BY` when `blocked_by` is non-empty.
   - `TIMING_CONFLICT` when a timing entry conflicts.
   - `C3_MISSING` when no probe covers the component. This is not applied to static components.
   - Each cause sets `provisional`. TIMING_CONFLICT and C3_MISSING also set the matching flag.
2. **Static:** `STATIC_PRUNE_RELOCATE` gives PRUNE with `prune_kind = "relocate"`.
3. **Correctness:** `C1_FAIL` or `C3_FAIL` is recorded, then:
   - If `repair_path` is non-null and `unique_capability` is true: `C_FAIL_REPAIRABLE` (detail = the
     `repair_path`) gives OPTIMIZE, provisional (prereg rule 1).
   - Otherwise PRUNE. A `repair_path` without `unique_capability` is noted in `reasons`.
   - With neither failure, `CORRECTNESS_OK` is recorded.
4. **No evidence:** `NO_EVIDENCE` applies when there is no non-excluded comparison. The label is OPTIMIZE if
   `unique_capability`, else PRUNE. Both are provisional, with flag NO_EVIDENCE and an
   insufficient_evidence reason.
5. **PROMOTE criteria:**
   - Over the non-excluded G/P comparisons, the fraction of GAIN, NEUTRAL and capability_gain is
     ≥ `promote_cell_fraction`.
   - Every ratio is ≤ `promote_max_ratio`.
   - C1, C2 and allocation pass.
     - **C2** is the requested-tolerance ratio check. A failing group counts against c only if no other arm
       containing c passes C2 on the same (problem, grid, rtol). Exonerated groups are counted in
       `criteria.c2.exonerated_groups`.
     - **Allocation** is the hot-loop rate. For each (arm, problem, grid) of c's arms, warm allocations =
       a + b·attempts is fitted by least squares. The points are the h0 = 1 ALLOCATIONS rows (warm = max over
       their raw windows) at the profiled rtols, with `attempts` from the matching RUNS row. A fit needs at
       least 2 distinct attempts; otherwise it is insufficient_evidence.
     - Allocation passes iff every fit has b ≤ `allocation.per_attempt_max`, and fails if one exceeds it.
       It is null when no fit exists. The constant term a (output storage, setup) is not gated.
     - The raw max warm window is still reported. `allocation.warm_window_allocations_max` and
       `allow_documented_constant_independent_of_n` remain in THRESHOLDS for back-compatibility but are unused.
   - **Applicability coverage:** coverage(c) = |G/P problems where some wired arm containing c applies| /
     |universe(c)|. It must be ≥ `applicability.coverage_min`.
     - universe(c) = the G/P problems where some wired arm of c's slot applies *and* that satisfy c's
       Jacobian class. Jacobian availability is user-declared and routed on, so it partitions the universe.
     - c's class is the union of `requires.jacobian` over its wired arms: `analytic` means problem.jacobian
       == "analytic"; `jvp` means problem.jvp; `any` means all problems. Without `--list` the class is `any`.
     - Arms and problems come from the binary's `applicable_problems` (triage-list), or from RUNS rows without
       `--list`.
     - `criteria.coverage` records the coverage, `jacobian_class`, `universe_gp_problems` and
       `slot_gp_problems`.
   - If all criteria hold and the component is not provisional: `PROMOTE`.
   - If all criteria hold and it is provisional: `PROMOTE_BLOCKED_PROVISIONAL` gives OPTIMIZE (flag
     PROMOTE_BLOCKED_PROVISIONAL). It never reaches PRUNE by fall-through.
   - If every criterion holds except coverage: `SPECIALIZE_APPLICABILITY` (kind `regime`) gives SPECIALIZE,
     provisional or not.
     - The predicate is `{kind: "applicability", rule: [{arm, requires, applicable_problems}], coverage, ...}`,
       the arms' API requirement itself. It is deterministic, so no calibration search and no holdout verdict
       is used.
   - If every criterion holds except allocation (coverage may also fail): `OPTIMIZE_ALLOCATION` (kind `alloc`,
     detail = the worst arm-problem b) gives OPTIMIZE. If the component is also provisional, it is preceded
     by `PROMOTE_BLOCKED_PROVISIONAL`.
   - Otherwise `NOT_PROMOTE` is recorded and the tree continues.
6. **SPECIALIZE:** the win set (GAIN or capability_gain cells) is non-empty, and a predicate exists:
   - Search on calibration cells: a single feature from `specialize.features`, either a numeric `le`/`ge`
     threshold (dimension; `rtol` means the target ε) or an `in` category set. It must admit no loss and the
     most wins.
   - Holdout check with `tools/holdout_verdict.py`: positives are holdout loss cells; at least
     `min_positives` are needed, and the verdict must be stable under a one-rank threshold shift. A category
     set has no threshold to shift, so its verdict is the raw verdict under `min_positives`. The predicate
     must also recommend at least one holdout win.
   - If so: `SPECIALIZE`. A provisional component keeps SPECIALIZE. Otherwise `NOT_SPECIALIZE`.
7. **OPTIMIZE**, checked in this order:
   - `OPTIMIZE_HEADROOM`: every LOSS ratio adjusted by `removable_ir` is ≤ `headroom_ratio_max`.
   - `OPTIMIZE_EXTERNAL_GAP`: `unique_capability`, and the median counted-work gap to `external_peers` is
     greater than `external_counted_work_gap`.
   - `OPTIMIZE_UNCHARACTERIZED_WINS`: `unique_capability` with a non-empty win set that no validated
     predicate describes.
8. **Default:** otherwise `PRUNE_DEFAULT` gives PRUNE.
9. **Tie-break** (post-pass over each slot): take two non-PRUNE components whose mutual comparisons are all
   NEUTRAL. The one with more LOC gets `NEUTRAL_TIEBREAK_LOC` and becomes PRUNE. With equal LOC both get
   `NEUTRAL_TIE_EQUAL_LOC` and keep their labels.

| rule | kind | meaning |
|---|---|---|
| `BLOCKED_BY` | contract | open obligation (`blocked_by`): provisional, PROMOTE forbidden |
| `TIMING_CONFLICT` | contract | a wall ratio deviates from 1 by more than `conflict_threshold`, opposite to the Ir ratio of the same cell, with an interval excluding 1: provisional |
| `C3_MISSING` | contract | no PROBES row covers the component: C3 null, provisional, PROMOTE forbidden |
| `PROMOTE_BLOCKED_PROVISIONAL` | contract | PROMOTE criteria met while provisional: OPTIMIZE |
| `STATIC_PRUNE_RELOCATE` | static | `static_class` PRUNE_RELOCATE: PRUNE(relocate) |
| `C1_FAIL` | correctness | integrity failure: a process_abort other than a runner timeout (a timeout only makes the cell inadmissible), `deterministic=false`, or a non-finite/malformed ok row. A failure is blamed on c unless another arm containing c ran the same cell cleanly. Gives PRUNE unless `C_FAIL_REPAIRABLE` follows |
| `C3_FAIL` | correctness | a fail-closed probe failed: PRUNE (unless `C_FAIL_REPAIRABLE` follows) |
| `C_FAIL_REPAIRABLE` | contract | C1/C3 failed, `repair_path` is non-null and `unique_capability` is true: OPTIMIZE, provisional |
| `CORRECTNESS_OK` | correctness | neither C1 nor C3 failed (null counts as not failed) |
| `NO_EVIDENCE` | evidence | no usable comparison: OPTIMIZE if unique_capability, else PRUNE; provisional |
| `PROMOTE` | frontier | all PROMOTE criteria and not provisional |
| `NOT_PROMOTE` | frontier | lists the failed PROMOTE criteria |
| `SPECIALIZE` | frontier | validated predicate (stored in `predicate`) |
| `NOT_SPECIALIZE` | frontier | why there is no predicate: empty win set, search failed, or holdout verdict |
| `SPECIALIZE_APPLICABILITY` | regime | PROMOTE fails only on coverage: SPECIALIZE with the arms' applicability as the predicate |
| `OPTIMIZE_ALLOCATION` | alloc | PROMOTE fails only on the allocation rate (and possibly provisional): OPTIMIZE |
| `OPTIMIZE_HEADROOM` | frontier | attribution-adjusted LOSS ratios ≤ `headroom_ratio_max` |
| `OPTIMIZE_EXTERNAL_GAP` | frontier | unique capability, median counted-work gap > `external_counted_work_gap` |
| `OPTIMIZE_UNCHARACTERIZED_WINS` | frontier | unique capability, wins without a validated predicate |
| `PRUNE_DEFAULT` | frontier | none of the above |
| `NEUTRAL_TIEBREAK_LOC` | tiebreak | all-NEUTRAL slot alternative with larger LOC: PRUNE |
| `NEUTRAL_TIE_EQUAL_LOC` | tiebreak | all-NEUTRAL alternatives with equal LOC: labels kept |

Flags: `TIMING_CONFLICT`, `C3_MISSING`, `NO_EVIDENCE` and `PROMOTE_BLOCKED_PROVISIONAL`. Each one implies
`provisional`.

## Known limits

- **Native solvers:** `tools/native_stiff` reports final states only and uses its own default first step.
  Its uniform101 cells are `unavailable`. The t0 state of its endpoint rows comes from the NumPy model.
- **Diffsol:**
  - `tools/benchmarks/diffsol/triage` implements nine problems: robertson, hires, van-der-pol-mu1000,
    oregonator, pollution, prothero-robinson-nl, scalar-linear, brusselator-1d-50 and brusselator-1d-250.
    Every other cell is `unavailable`.
  - Diffsol picks its own first step (`h0` null) and keeps its upstream defaults. For example, at most 50
    nonlinear failures over the whole integration are allowed; more is a typed `solver_failure`.
  - Counted work is comparable; timing is not recorded.
  - The a07 rows (Robertson, rtol 1e-8, atol 1e-11 = atol_scale 1e-3, which is not a CORPUS cell) are no
    longer read.
- **NumPy models:** exist for `robertson`, `hires`, `van-der-pol-mu<MU>`, `brusselator-1d-<CELLS>`,
  `gray-scott-<N>`, `mass-affine` and the ScientificCorpusV2.1 ids. CORPUS `numpy_model` aliases map the
  others onto these. `mass-affine` is integrated as y' = M⁻¹f.
  - The model JVPs of the 1D Brusselator and the v2.1 semilinear problem keep the Rust operation order. A
    plain J @ v differs by rounding where the product cancels.
  - Problems with no model get `no_numpy_model` in references and `not_checked` in parity.
- **Static reach:** found by name (unique pub items and module paths), not by type resolution.
- **Repair path:** `repair_path` is free text taken from COMPONENTS. The engine checks only that it is set,
  not that the repair is feasible.

Tests: `tools/test_triage_check.py`, `tools/test_triage_runner.py`, `tools/test_triage_profile.py`,
`tools/test_triage_tools.py`, `tools/test_triage_diffsol.py` (fake `diffsol-triage`; the real binary too when
`VIGILODE_DIFFSOL_BINARY` is set), and `TriageKind` in `tools/test_evidence_schema_v2.py`.
