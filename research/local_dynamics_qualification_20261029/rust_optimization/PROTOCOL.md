# Rust adapter optimization baseline protocol

This harness is a diagnostic, not production qualification. Build the same
`rust_allocation_probe` example source against the pinned baseline and candidate
trees. Record both source SHAs and SHA256 of each executable before calling
`run_light.py`. The script never builds either binary.

## Frozen inputs and coverage

Timing cases are `dense-fd` (autonomous manufactured coupled dense FD, n=8),
`banded-fd` (autonomous tridiagonal FD, n=64, discrete sine eigenvector exact
solution), `time-jvp` (nonautonomous supplied JVP, n=16, no supplied time
derivative; exact endpoint is `exp(-t)*(1+i/n)`), and `autonomous-jvp`
(rotation control, n=16). Two validation-only controls are excluded from the
timing campaign: `nonautonomous-dense-fd` (n=8, time dependent manufactured
path, dense FD, missing supplied time derivative, span 0.25) and
`span-parameter-control` (n=8 diagonal decay, exact exponential; spans 0.125 and
0.25 crossed with active rate offset 0.0 and 0.4). The active rate parameter
and span are both passed through to the model and analytic oracle; no inert
parameter is used. The two controls run probe and allocation modes only. This
protocol does not include the separate `production_return_bench_20261027`
reentrant sentinel control; that fixture remains a distinct measurement.

Probe and allocation modes emit every output time/state as IEEE-754 bit strings,
endpoint max absolute error against the analytic oracle, status, complete
serialized `Stats` including both work-counter records, and actual RHS/JVP/time
derivative callback counts. Allocation is strictly counted inside
`Rodas5p::solve` by a thread-local flag. Timing mode emits only `solver_ns` and
`solve_plus_drop_ns`; it performs no state/stats snapshot clone, oracle, or
serialization inside the measured interval. It checks success and copies the
typed status before dropping the result. Model/options setup is separately recorded as
`setup_ns`. Endpoint error cap is fixed at `1e-5`; exceeding it or a failed
solver status is retained as a failure. The runner probes/allocates all four primary cases plus both validation-only
controls before any timing. It runs the five-block timing only for the four
timing cases. Do not
describe these numbers as whole-process latency.

## Run protocol

Run one case at a time. The runner checks immutable binary hashes, pins the
process to the requested CPU (default 0), sets solver thread environment to 1,
uses at most 90 seconds per process, and writes every command/stdout/stderr/
exit/timeout to a new output directory. It runs a warmup then five paired ABBA
blocks, alternating `baseline,candidate,candidate,baseline` and
`candidate,baseline,baseline,candidate`, for each timing case. The order is
not an accuracy or parity substitute; exact comparisons come from the separate
probe outputs.
Each timing invocation uses the fixed repeat count 64, frozen before either binary is built or examined. Validation/alloc use one solve and profiler levels are 4/12. A timed process failure, timeout, output mismatch,
or parity failure remains a failure and is never filtered.

The runner-wide child-process and Callgrind-annotation cap is 1200 seconds;
each child is capped at 90 seconds. Cargo and native build locking remain the
controller's responsibility: this script does not acquire the shared Cargo
lock and never builds. Allocation counts are reported as baseline, candidate,
delta, and ratio; equal allocation counts are not required because reductions
are the optimization target. Allocation mode is a separate paired run and is
not mixed into timing. If
Callgrind is installed, profile mode runs each binary at two fixed repeat
levels (default 4 and 12) and retains raw Callgrind files, command logs, and
self/inclusive function summaries plus the fixed-overhead-subtracted `Ir`
per-solve slope. If Valgrind tools are absent, profile status
is `NOT_RUN` with the detection output. Profile is diagnostic and does not
substitute for wall time. There are no tolerance relaxations: state bits,
times, status, complete Stats and callback counts must match exactly between
the two executables. Endpoint error is reported for interpretation, not used
to waive a mismatch.

Example (after the parent has published the execution source):

```bash
python3 research/local_dynamics_qualification_20261029/rust_optimization/run_light.py \
  --baseline /absolute/path/base/rust_allocation_probe \
  --baseline-sha SOURCE_SHA --baseline-binary-sha SHA256 \
  --candidate /absolute/path/candidate/rust_allocation_probe \
  --candidate-sha SOURCE_SHA --candidate-binary-sha SHA256 \
  --output /absolute/path/new-evidence/rust-opt --cpu 0
```

No build or benchmark command is run by this preparation task. The controller
must serialize Cargo/build and benchmark execution under its shared lock.
