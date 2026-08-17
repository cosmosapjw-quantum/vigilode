# PHYS–MATH–CODE AUDIT — G4-S5B3 Attempt Geometry

## Executable path

```text
frozen G3 problem/config builders
  -> JVP-only or time-augmented OdeProblem
  -> disabled fused adaptive pexprb54s4 trajectory
  -> read-only fused adaptive pexprb54s4 trajectory
  -> bitwise trajectory/controller/work comparison
  -> complete per-attempt telemetry and work rows
  -> paired disabled/read-only overhead protocol
  -> threshold-free Python analysis and plots
```

## Production-path changes

No integrator tableau, controller equation, Krylov kernel, RHS/JVP path, BDF,
or Radau implementation is modified. The only pre-existing source made visible
is the frozen G3 problem/config builder, using crate-private (`pub(crate)`)
exposure so the new research runner cannot drift from the canonical corpus.

The added code consists of:

- a read-only G4-S5B3 research runner;
- a CLI output command;
- smoke and CLI contract tests;
- a threshold-free analysis script and tests.

## Numerical identity gates

All 12 disabled/read-only trajectory pairs pass:

- requested times and states bitwise identical;
- success/message/internal-step/output-clipping fields identical;
- attempt counts and accepted/rejected sequences identical;
- accepted/rejected step-size and error histories bitwise identical;
- maximum Krylov dimensions and phi substeps identical;
- all existing `WorkCounters` identical;
- complete per-trial work rows whose sum equals aggregate numerical work;
- zero unscorable and zero legacy-unclassified attempts;
- zero added RHS, JVP, JVP-vector, phi-action, partial-time, Jacobian-build, or
  Newton work.

The proposed-interval authority is

\[
\Gamma=\frac{T_{\rm suite}}
              {\sum_{\rm attempts}|h_{\rm trial}|},
\]

including rejected attempts. Paired arms have bitwise-equal proposed intervals.

## Analysis correctness gates

- all distinct finite scores plus lower/upper boundary sentinels are swept;
- no threshold field is generated;
- Pareto false-count rows are exported separately;
- exact cross-outcome score collisions are emitted;
- per-problem, per-tolerance, leave-one-group-out, concentration, and overhead
  views are generated;
- synthetic tests enforce both sentinels, no selection, collision detection,
  rank separation, and Pareto generation.

## Residual risks

1. **P0 scientific blocker:** the raw score loses tolerance information and is
   not suitable for blanket admission.
2. **P1 relevance mismatch:** the current sweep includes residence attempts,
   while a future event-triggered rule would query only entry/re-entry events.
3. **P1 sample imbalance:** only seven rejected attempts exist, with six from
   one problem family.
4. **P1 timing uncertainty:** seven pairs measure practical noise but do not
   establish a confidence bound; CPU affinity/frequency are not controlled as
   a publication benchmark.
5. **P2 diagnostic allocation:** read-only row construction is included in
   solver timing, but output serialization and offline plotting are excluded.
6. **P2 interface debt:** making G3 builders crate-private is minimal and
   source-sharing preserving, but a later survivor-only cleanup may move common
   corpus construction into a dedicated research fixture module.

## Regression protection

- dedicated Rust smoke runner test;
- CLI machine-readable schema test;
- existing four early-flow-defect identity tests;
- two Python analysis tests;
- full workspace all-target regression;
- frozen source hashes for BDF, Radau, Cargo lock/config, and toolchain.

## Verdict

The code is a valid, complete, read-only falsification harness. It preserves
all numerical physics and work accounting. Its scientific output blocks, rather
than supports, activation of a raw-score global threshold.
