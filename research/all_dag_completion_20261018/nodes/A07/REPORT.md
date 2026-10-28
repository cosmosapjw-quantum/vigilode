# A07: comparable campaigns and generated reports

The Robertson/Diffsol campaign already published under
`research/production_audit_20261018/evidence/continuation_a07/` is reused. Its
independent review admitted campaign integrity, not a same-accuracy speed claim:
the native maximum-grid WRMS was about 0.338 and Diffsol about 3.303 at the fixed
physical tolerances. The retained 182 rows and the external pinned Cargo.lock
remain unchanged. Repeating that experiment would not repair the accuracy gate.

This node completes the companion collective structured-versus-dense reporting
path required by A06. `tools/all_dag_benchmark.py` provides an executable campaign
runner, a hostile-input validator and a deterministic Markdown generator. Eight
targeted Python tests pass. A historical 30-row local campaign is reprocessed
without solver execution in `retained_collective_report/`; all three tolerance
groups pass the empirical achieved-accuracy closeness rule. That historical
import explicitly has no new source/build binding and cannot promote speed.

## Fixed comparison contract

Both arms use the same collective equation, zero carrier, stored coefficients,
initial state, `OutputSchedule::uniform(0,1,0.1)`, and physical tolerances
`rtol in [1e-6,1e-8,1e-10]`, `atol=rtol*1e-3`. The precise floating grid and
initial-state formulas are defined by the captured native source bytes. The
runner does not tune physical tolerances to create a favorable comparison.

Per-arm measured time includes solver/adapter/workspace setup, integration and
requested output. The coefficient cache is prewarmed equally. The independent
two-resolution RK4 reference is excluded from both timed arms. This is an
approximate empirical comparison, not a trajectory enclosure comparison;
the present arms do not perform continuous-trajectory certification.

For each fixed problem/tolerance group let `e_a` be its maximum measured absolute
error relative to the fine reference and `u` the maximum reference refinement
discrepancy. Admission requires every arm to complete every requested output and
pass its normalized empirical accuracy gate, and additionally

`(max_a e_a + u)/(min_a e_a - u) <= 1.05`, with `min_a e_a > u`.

The 1.05 threshold is frozen before new measurements. This is a conservative
closeness rule relative to the observed reference spread; `u` is empirical and
does not turn the interval into a mathematical enclosure. Below reference
resolution the rule refuses admission. Equal requested tolerances alone are
insufficient: a hostile fixture doubling one arm's achieved absolute error is
rejected even when both normalized tolerance gates pass.

## Execution and identity

After native source is frozen, from repository root:

```sh
python3 tools/all_dag_benchmark.py run --modes 8 32 --repeats 5 --processes 3 --output artifacts/generated/a07_collective_campaign
python3 tools/tests/test_all_dag_benchmark.py
```

The output directory must be empty. Offline locked Cargo is the default.
`--online` enables dependency retrieval, without changing Cargo.lock. Compilation
uses the same executable for both arms. The tool captures protocol and source
hashes before build; it records Git HEAD/status, Cargo.lock, toolchain versions,
build command, binary hash and process commands. A source change during build
or execution invalidates the source binding. Build failures preserve their
command and output and receive a typed status. The run path is part of this
implementation; its actual execution status must be read from its evidence,
not inferred from successful Python contract tests.

Independent fresh processes each run both arms with alternating repeat order.
Timing ratios are paired by process/repeat, then reduced to one median per
process. A seeded bootstrap resamples those process-level medians. Fewer than
three independent processes is explicitly insufficient. This avoids treating
all in-process repeats as independent observations. Complete process wall time
and compilation time are recorded separately from per-arm medians. Process wall
time includes startup and reference calculation; startup alone is not isolated.
All timings remain diagnostics because the inherited native example
contains allocator instrumentation in both arms.

Raw JSONL is preserved, including failed/incomplete arms. Missing/duplicate
rows, altered tolerance contracts, false error sums, NaN, incomplete output,
and uncalibrated zero-allocation claims are rejected. Allocation values are
admitted only when the cross-crate model-clone positive control sees at least
three allocations; otherwise only null measured counts are allowed. The
generated Markdown is regenerated from parsed validated JSON and compared byte
for byte, so there is no separately maintained numerical report table.

## Status and remaining promotion gate

The campaign/reporting implementation is complete for the collective pair and
reuses the already published generic Robertson comparison. A07 does not require
either solver to win. Source-bound successful execution plus the matched-error
rule admits a comparable diagnostic dataset. Uninstrumented timing replications,
a rigorous reference where that claim is desired, and independent review of a
predeclared speed criterion are separate requirements for speed promotion.
Broad production and speed promotion remain HOLD.
