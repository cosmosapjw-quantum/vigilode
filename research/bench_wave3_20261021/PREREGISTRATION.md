# E1b-dense: preregistration

Research node `e1b_dense` (wave 3, task 1; plan §7). Directory `research/bench_wave3_20261021/`. Written and
pushed before any measurement of the corpus. Ledger row `L-0142` is pre-assigned and is appended by the
controller after the workstation results return; its verdict is fixed in advance as **INCONCLUSIVE**.

## Question

Wave-2 E1 (`research/bench_wave2_20261021/`, ledger `L-0139`, head of its chain `L-0140`) compared VigilODE
with the external comparators while the two VigilODE arms landed a step on every grid time, whereas every
comparator stepped freely and interpolated. The wave-2 ruling is that no solver ranking is admitted until the
facade's dense output is benchmarked. This node asks: how does VigilODE compare with the comparators when
**every arm steps freely and interpolates**? VigilODE enters through `Rodas5p::solve` with
`dense_output = true`, and the grid values are `Solution::at(t_k)`.

## What is measured

One run of `vigilode-bench wp` over `corpus_v2.json` (sha256
`f85db9641ae9e7cc126913005e29475eae5f4ee633b3c0d46c147991a18d2623`). Schema id is unchanged
(`vigilode-bench.corpus.v1`).

- Corpus v2 equals corpus v1 in problems (Robertson, HIRES, van der Pol mu = 1000, Brusselator 1D n = 100),
  `rtols` {1e-4, 1e-6, 1e-8}, `error_levels`, reference rule, atol scales, 101-point grid, 2 warm-ups and 21
  repetitions. The arm list is v1's 13 plus four new arms.
- New arms: `vigilode-dense`, `vigilode-dense-off`, `vigilode-banded-dense`, `vigilode-banded-dense-off`.
  Each calls `Rodas5p::solve` with `Options { rtol, atol: Atol::Scalar(rtol x atol_scale), dense_output: true,
  interior_error: WFilteredEnforce | Off, ..Default::default() }`. The analytic Jacobian enters through
  `Jacobian::dense` or `Jacobian::banded(lower, upper, ..)`. Scalar atol keeps the run comparable with v1.
  Banded arms run on the Brusselator only.
- The v1 arms `vigilode` and `vigilode-banded` stay as continuity arms and are labelled `grid-clipped`.
- Comparators (re-measured, not inherited from v1): Diffsol 0.16.1 BDF (dense, sparse), SUNDIALS CVODE 6.4.1
  (dense, band), Netlib DVODE MF 21/24, Hairer RADAU5 and RODAS (dense, band), internal Radau IIA3
  (reference implementation only, never in a speed table). Output interfaces are those of v1.
- Harness code: `crates/vigilode-bench` at commit `219a140` on `production/e1b-dense`. The measured commit
  (the node branch after it merges the post-T7 integration tip) is recorded in the handoff and in
  `provenance.git_commit` of the results.

## Protocol rule (endpoint-only stepping)

A dense row with `stats.output_clipped_steps != 0`, or with any `Solution::at(t_k) == None` for `t_k` in the
span of a successful run, gets status `protocol-violation`. Such a row is kept, never dropped. The expected
count is 0 (`protocol_violation_rows` in the JSON); a non-zero count is reported as a finding, not repaired.
Each dense row records `stats.route.driver_id` as `native.driver_id`.

## Reference and metrics

Unchanged from v1: reference is the `vigilode` arm (grid-clipped route) at rtol 1e-12 with the attempt
budget 2,000,000; decisive cross-check Diffsol BDF dense (first successful of 1e-12 .. 1e-9); disagreement
above 1e-8 marks a problem `reference-disputed`. Metrics `error_own` and `error_common`
(`error_rtol = 1`), work counters, callback counts and allocation counts are those of v1.

Matched error: at the half-decade levels of v1, median time and RHS count of the comparator and of its
baseline are interpolated log-log on their own successful points; a ratio exists only where both bracket the
level, the reference is undisputed and both arms are deterministic. The report has two bases:

1. `dense` (**headline**): baseline `vigilode-dense` (interior `WFilteredEnforce`, the product default); for
   band-structured comparators `vigilode-banded-dense`.
2. `grid-clipped` (continuity with v1): baseline `vigilode` / `vigilode-banded`.

No ratio is promoted to a speed claim.

### Interior control evidence (A2)

For each problem x rtol and each of the dense and banded-dense routes the report lists `error_own` and
accepted steps of the `WFilteredEnforce` arm against the `Off` arm, the ratio `error_own(off) /
error_own(enforce)`, the ratio `accepted(enforce) / accepted(off)`, and whether `error_own(enforce)` is at
most 2 tolerance units. All entries are computed by the harness into the JSON (`interior_control`) and
copied by the report. No threshold is applied to the verdict: the table is evidence for the interior-control
design, read after the run.

## Host and timing

The measurement runs on the owner's workstation, pinned to one core (`taskset -c 2`) with
`OPENBLAS_NUM_THREADS=1`, in-process repetitions in rotated arm order, no pinned frequency.
`timing_authority = "diagnostic_only"`. The host is recorded in `host.json` (including `timing_authority`)
and the raw text in `host.txt`. **Ratios across hosts are never compared**: v2 ratios (workstation) are not
set against v1 ratios (shared 2-core cloud host), neither in the report nor in the ledger.

## Verdict, fixed in advance

**INCONCLUSIVE.** Timing is diagnostic, so the row exists for provenance and for the interior-error
evidence. Ledger row `L-0142` supersedes **`L-0140`**, the head of the `L-0139` chain, for the
VigilODE-relative ratios only. It is appended on a branch that already contains `L-0141`, after the
integration tip is merged, with `inputs_sha256` filled and a date no earlier than `L-0141`'s.

## Known limitations, fixed before the run

1. The dense route measured is the one the facade ships at the measured commit; the preregistration does not
   assume which driver that is. `native.driver_id` of each row says.
2. The reference is produced by the grid-clipped `vigilode` arm, not by a dense arm. Dense arms therefore
   have a different step sequence from the reference; their `error_common` includes interpolation error
   between steps, as the comparators' do.
3. A smoke run (one problem, Brusselator, one rtol 1e-6, one repetition, debug build, shared cloud host) was
   made on the harness before this file was committed, to show the four arms run end to end with
   `output_clipped_steps = 0` and 0 protocol violations. It is not a measurement of the corpus and is not
   committed. It showed `error_own` of about 9 tolerance units for all four dense arms at rtol 1e-6 on the
   Brusselator, with identical accepted-step counts (78) for `WFilteredEnforce` and `Off`; this is disclosed
   here so that the corresponding full-run rows are not read as a surprise tuned away. No tolerance, level,
   threshold or arm list is changed after this file is committed.
4. Other limitations of the v1 preregistration (Diffsol takes a JVP; CVODE one-step cap; DVODE defaults;
   Hairer H = 0) apply unchanged.

## Outputs

`results_v2.json` (raw, generated), `REPORT.md` (generated by `vigilode-bench report` from the JSON only),
`run.log`, `host.json`, `host.txt`; copied unchanged into this directory after the workstation run, with a
results section appended below.
