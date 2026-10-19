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
python3 tools/triage/triage_parity.py --binary BIN --corpus $N/CORPUS.json --output $N/PARITY.json
tools/triage/host_prep.sh --output $N/HOST.json            # wall-clock sessions only (diagnostic)
python3 tools/triage/triage_runner.py --binary BIN --corpus $N/CORPUS.json --campaign $C --output $N/RUNS.jsonl -j 4
python3 tools/triage/triage_runner.py ... --output $N/PROBES.jsonl   # probe rows, see "PROBES.jsonl"
python3 tools/triage/triage_profile.py --binary BENCH_BIN --corpus $N/CORPUS.json --campaign $C \
    --output $N/PROFILE.jsonl --scratch /tmp/ct-callgrind [--cachegrind]
python3 tools/triage/triage_alloc.py --binary ALLOC_BIN --list LIST.json --corpus $N/CORPUS.json --campaign $C \
    --output $N/ALLOCATIONS.jsonl
python3 tools/triage/triage_external.py --corpus $N/CORPUS.json --campaign $C --output $N/EXTERNAL.jsonl \
    [--native-driver BUILD/native_stiff] [--diffsol-rows A07.jsonl]
python3 tools/triage/triage_check.py --campaign $C --runs $N/RUNS.jsonl --runs $N/EXTERNAL.jsonl \
    --references $N/REFERENCES.json --profile $N/PROFILE.jsonl --allocations $N/ALLOCATIONS.jsonl \
    --probes $N/PROBES.jsonl [--timing $N/TIMING.json] [--attribution $N/ATTRIBUTION.json] [--list LIST.json] \
    --results $N/RESULTS.json --classification $N/CLASSIFICATION.json
python3 tools/triage/triage_report.py --results $N/RESULTS.json --classification $N/CLASSIFICATION.json \
    --output $N/REPORT_KO.md            # later: same command with --check (PA-06 guard)
```

`LIST.json` is the stdout of `rodas5p triage-list`. Tools that take `--binary` can produce it themselves.

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

These are the contract's row keys. The Python drivers may add only two keys: `process` and `probe`.

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
    rows also keep `raw`; diffsol rows keep `solver_counters` and `callback_counters`.
  - Status `unavailable` (with the reason in `error`) marks a cell the external code cannot run. Such a
    cell is written, never skipped.
  - Each (arm, problem, rtol, grid) has exactly one row, at the nominal `h0_scale` 1.0. For native and
    diffsol rows `h0` is null, because they use the solver's default first step
    (`extra_work.h0_policy = "solver_default"`).
  - Wall time is never recorded.

### `PROBES.jsonl` (C3)

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
- A probe with `must_refuse: false` passes unless it aborts.

Verdict per component:

- C3 = `fail` if any of its probes fails, `pass` if all pass.
- C3 = `null` if no probe covers it (C3_MISSING).

### Assumed Rust outputs not in the contract

- **`rodas5p triage-exact --problem P --grid G`:** prints
  `{"schema": "vigilode-triage-exact-v1", "problem", "grid", "output_times": [..], "states_hex": [[..], ..]}`.
  This is the closed-form solution, not the solver. The output times must equal the reference grid.
- **`rodas5p triage-parity --problem P`:** prints
  `{"schema": "vigilode-triage-parity-v1", "problem", "dimension", "y0_hex": [..]?, "samples": [{"t_hex",
  "y_hex": [..], "f_hex": [..], "jacobian_hex": [[row 0], ..]}]}`. All values are 16-digit binary64 hex.
  The Jacobian is dense and row-major.

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
  - `attribution`: every arm a′ whose component set equals a's with c replaced by another member of c's slot.
    If a′ is not applicable to the problem, there is no rival for that cell.
  - `strongest`: used when no a′ exists. The rival is the cheapest applicable arm that contains another
    slot member and not c.
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
   - C1, C2 and allocation pass (warm max ≤ `warm_window_allocations_max`, or a documented n-independent
     constant).
   - If all criteria hold and the component is not provisional: `PROMOTE`.
   - If all criteria hold and it is provisional: `PROMOTE_BLOCKED_PROVISIONAL` gives OPTIMIZE (flag
     PROMOTE_BLOCKED_PROVISIONAL). It never reaches PRUNE by fall-through.
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
| `C1_FAIL` | correctness | integrity failure: process_abort, `deterministic=false`, or a non-finite/malformed ok row. A failure is blamed on c unless another arm containing c ran the same cell cleanly. Gives PRUNE unless `C_FAIL_REPAIRABLE` follows |
| `C3_FAIL` | correctness | a fail-closed probe failed: PRUNE (unless `C_FAIL_REPAIRABLE` follows) |
| `C_FAIL_REPAIRABLE` | contract | C1/C3 failed, `repair_path` is non-null and `unique_capability` is true: OPTIMIZE, provisional |
| `CORRECTNESS_OK` | correctness | neither C1 nor C3 failed (null counts as not failed) |
| `NO_EVIDENCE` | evidence | no usable comparison: OPTIMIZE if unique_capability, else PRUNE; provisional |
| `PROMOTE` | frontier | all PROMOTE criteria and not provisional |
| `NOT_PROMOTE` | frontier | lists the failed PROMOTE criteria |
| `SPECIALIZE` | frontier | validated predicate (stored in `predicate`) |
| `NOT_SPECIALIZE` | frontier | why there is no predicate: empty win set, search failed, or holdout verdict |
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
- **Diffsol:** the a07 harness covers Robertson at rtol 1e-8 on the uniform101 grid only. Its atol is
  compared within 4 ulps.
- **NumPy models:** exist for `robertson`, `hires`, `van-der-pol-mu<MU>`, `brusselator-1d-<CELLS>` and the
  ScientificCorpusV2.1 ids. Any other problem gets `no_numpy_model` in references and `not_checked` in
  parity.
- **Static reach:** found by name (unique pub items and module paths), not by type resolution.
- **Repair path:** `repair_path` is free text taken from COMPONENTS. The engine checks only that it is set,
  not that the repair is feasible.

Tests: `tools/test_triage_check.py`, `tools/test_triage_runner.py`, `tools/test_triage_profile.py`,
`tools/test_triage_tools.py`, and `TriageKind` in `tools/test_evidence_schema_v2.py`.
