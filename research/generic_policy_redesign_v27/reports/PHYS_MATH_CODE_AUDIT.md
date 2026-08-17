# PHYS–MATH–CODE AUDIT — v2.7 actual level-1 prefix

## Equation-to-code map

- `pexprb54s4_level1_prefix_with_tolerance_scaled_telemetry`: computes only
  F0/J operator, c2 phi1 action, U2, D2, and raw/scaled early telemetry.
- `Pexprb54s4Level1Prefix`: retains the exact numerical prefix state required
  for continuation.
- `pexprb54s4_fused_step_resume_level1`: consumes that retained prefix rather
  than reacquiring level 1.
- research runner: commits only R-JF; probes level 1 before the first next R
  trial when the causal persistence policy fires; records R outcome afterward.

## TDD and mechanism evidence

The RED→GREEN contract test checks one-shot versus begin+resume and requires:

- bitwise-equal `y_new`, embedded state, and error vector;
- equal phi reports and early telemetry;
- equal total `WorkCounters`;
- prefix work is a strict subset of full work.

Existing early-defect, tolerance-scaled, fused-order, and G3 smoke contracts
also pass after the refactor. The research runner separately checks that
committed R-JF numerical/work fields equal the independent v2.6 attempt trace.

## Campaign evidence

Fourteen new solver shards were executed:

- 12 k=3 family×dimension shards (6 families × N=128/512);
- 2 frozen-k1 nonautonomous counterexample shards.

No full E-K continuation was executed and no N=2048 job was opened.

For all 12 k=3 shards:

- every prefix returned successfully;
- the first target R trial was accepted;
- R-JF numerical fields match the independent attempt-trace authority exactly;
- explicit Jacobian builds, direct factorizations, and Newton iterations in the
  prefix are zero.

## Work-ledger interpretation

The deterministic vector ledger is the current authority, not debug wall time.
Across k=3 events:

- median prefix / target-R JVP ratio: 4.237% in both dimensions;
- P95: 7.56% (N=128), 8.14% (N=512);
- median prefix / historical full-E JVP ratio: 17.24% and 20%;
- aggregate prefix / full-trajectory R-JVP ratio: 0.221% and 0.188%;
- aggregate prefix / full-trajectory R-RHS ratio: 1.51% and 1.69%.

Debug same-build wall ratios are retained only as descriptive evidence because
this node could not establish a publication-grade optimized timing authority.

## Adversarial findings

1. **Causal-indexing bug class confirmed.** Frozen k=1 fires on a rejected first
   proposal in both dimensions. Accepted-grid `j+1` is therefore invalid as a
   runtime authority.
2. **Persistence alone is not safety.** k=3 removes rejected-next events but
   still contains one historical full-E-inadmissible HIRES event per dimension.
3. **Level-1 scalar threshold is insufficient.** The unsafe HIRES point is not
   an extreme rho outlier and is bracketed by safe HIRES points.
4. **Historical labels are audit-only.** Full-E labels come from exact-parity
   historical runs; v2.7 itself never completed E-K.
5. **Development leakage is explicit.** k=3 was chosen after examining N=512,
   so it cannot be treated as a sealed holdout policy.

## Verdict

The code correctly implements a resumable, actual dependency-level-1 probe and
preserves R-JF authority. The policy layer is **not** ready for switching. The
next code change should extend the retained prefix to the next dependency level
and measure additional remainder/cost information without computing the final
E-K endpoint.
