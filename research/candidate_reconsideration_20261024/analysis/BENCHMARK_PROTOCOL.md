# Candidate-reconsideration experiment (registered before execution)

The experiment asks whether bounded candidate exploration removes repeated wasted
projection work while preserving the accepted trajectory and error checks. It
also tests whether the resulting path is competitive with the applicable native
protected solvers. A gain over an unconditional candidate is reported separately
from a gain over the fastest protected comparator.

All primary runs use one execution thread. Seven analytic workload cases cover a
three-rate diagonal system, the same system with deliberately injected RHS work,
a broad diagonal spectrum, componentwise Riccati nonlinearity, a nonnormal
triangular system, a rapidly rotating damped system, and a smooth changing-rate
system with an explicit clock state. The injected work is a synthetic callback
cost experiment, not a claim about an application; the seven cases contain six
distinct ODE families. The clock augmentation makes
the changing-rate model autonomous; its JVP includes the clock derivative.

Every integration lands on 17 common observation times. We use rtol=1e-6,
atol=1e-8, initial h=1e-3, at most 5000 attempts, and a **predeclared common
maximum sampled absolute-error limit of 1e-5**. Nominal tolerance alone does not
admit a run. A failed, incomplete, nonfinite or inaccurate run remains in the
output and receives no speedup claim. These observations do not certify the
continuous trajectory between samples.

Methods are fixed causal forward, adaptive causal forward, direct protected
raw-U fallback, protected matrix-free fast RODAS5P, protected dense fast RODAS5P,
and compact-band fast RODAS5P where its narrow-band structure applies. These are
in-repository controls. New external VODE/DVODE, diffsol and Julia runs are outside
this small experiment. No NCP resources are used. The dense and banded controls
receive exact analytic Jacobians; matrix-free controls receive the same exact
JVP. Callback permissions are therefore explicit and different resource contracts
are reported rather than hidden.

Each case has one untimed warmup per method, then five timed repetitions with a
cyclic method order. Timing includes workspace allocation, candidate probes,
rejections, fallback and receipt creation; model creation, reference evaluation,
serialization and warmup are excluded. Each raw result is emitted immediately as
a JSON line, preserving previous rows if a later method fails. Median timing is
descriptive within-process evidence, not an independent-process confidence
interval. We retain absolute error, scaled error, attempts, all available work
counters, candidate probes/skips and fallback counts.

The causal configuration is uniform across cases (initial basis 3, maximum basis
8, maximum expansions 2, maximum batches 64). The switching policy never sees the
analytic reference or case name. Configuration changes after measurements require
a recorded diagnosis and count toward the user's maximum of three correction
rounds. Performance claims require all repeated runs to pass the common error
limit. A ratio above 1 means the named baseline took longer. Both the adaptive
versus fixed-candidate ratio and the fastest-protected versus adaptive ratio are
reported; they answer different questions.

Model dimensions, horizons, exact solutions and parameter values are specified
in `crates/vigilode-coupled/examples/reconsideration_bench.rs`. The raw JSON header
records the actual parameters and software/source identity supplied at execution.

## Root-algorithm and parallel-cost extension (registered before measurements)

The separate `--root-variants` campaign runs the few-mode, synthetic expensive
few-mode and nonlinear Riccati cases with `--threads 1` and `--threads 2`.
It compares fixed/adaptive causal forward, fixed/adaptive causal Picard,
fixed/adaptive residual-first continuation/correction, protected raw-U and
protected dense. All other settings, references and error limits remain fixed.
The caller-owned two-worker pool is built outside each timed solve, and its
construction is excluded explicitly. Both thread settings retain the full RHS
and JVP work counts. Any parallel benefit is assessed at equal achieved-error
eligibility against the same algorithm at one thread, and separately against
the faster serial forward/protected control. The primary seven-case benchmark
remains single-threaded and is not pooled with this extension.

## Diagnosed final policy revision and a restricted-basis negative control

The first complete campaign found that default exponential cooldown suppressed
cheap failed projections but also missed expensive successful candidates. Its
205 primary timed rows and 240 root-variant rows remain unchanged; reports and
figures are preserved under `analysis/baseline_policy/` and the benchmark source
under `evidence/benchmarks/round1_reconsideration_bench.rs`. The proposed final
revision prices the cooldown by measured failed-probe work relative to the
protected work. It is evaluated by repeating the original seven primary cases
once, with identical inputs, tolerances and repetition counts.

A further registered negative control uses the same few-mode ODE and
`--basis-limit 1`, setting initial and maximum projection dimension to one and
allowing zero basis expansions. This is a deliberately inadequate computational
budget, not a new case-specific switching branch. The selector receives its
ordinary receipts and policy only; it cannot see the exact solution or case
name. All six controls use the same RHS, analytic reference and observations.
The protected solvers are unaffected by the candidate-only projection budget.
The restricted-basis case is identified as `few-mode-basis1` in the raw output.
The final revision is the last correction campaign under the user's three-round
limit. Gains and losses on the original cases are retained alongside this
negative control.

The final primary processes additionally interleave `legacy-switching-causal`,
using `exploration_overhead_fraction=None`, with the revised policy in the same
binary. Thus the old/new policy comparison has a same-process control as well as
the preserved first campaign. This seventh method does not change any earlier
measurement. The six root-variant processes are repeated with the final policy;
their baseline-policy measurements remain separately identified.
