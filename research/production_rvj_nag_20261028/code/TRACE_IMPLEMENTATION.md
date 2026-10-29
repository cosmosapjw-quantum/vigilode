# Fixed quotient trace implementation

Task: `RVJ-TRACE-20261028`; correction budget remains the parent task's
existing maximum of three. This file records implementation source, not a
new numerical result. Parent theory admission preceded these runtime edits.
The worker has not run Cargo, native tests, benchmarks or scientific oracles;
publication and execution remain with the parent.

## Implemented source

- `crates/vigilode-coupled/src/trace_quotient.rs`
- `crates/vigilode-coupled/examples/rvj_trace_campaign.rs`

The parent owns the module export in `lib.rs`. No default dispatcher is
changed by this additive provider. Its target identifier is
`rvj-quartic-all-complex-multiplicity-v1`.

### Explicit target and bounded API

`TraceSemantics::AllComplexRootsAlgebraicMultiplicity` is required by every
public evaluation function. `SelectedReal` and `RealOnly` fail before
arithmetic and retain their rejection counters. This is a semantic refusal,
not a claim that such other problems cannot be solved by another method.

`QuarticParameters` validates finite ordered intervals even when an incoming
`Interval` was constructed through its public fields. Private parameter
fields then preserve the invariant. Point parameters mean exact represented
binary64 values. General supplied coefficient intervals include their own
conversion allowances; exact `1/3` must not be silently represented as a
point at rounded binary64 `1/3`.

The public functions are:

| Function | Exact target | Bounded scope |
|---|---|---|
| `quartic_moments` | `Tr(q1^k)`, `k=0,...,6` | Seven fixed moments, rank four with multiplicity |
| `quartic_invariant` | Fixed original-variable polynomials from T6 | Includes constant `Tr(q1*q2)=3` and `Tr(q1^2*q2^2)=4ab+3` |
| `quartic_polynomial_trace` | `Tr(sum c[k]*q1^k)` | At most seven enclosed coefficients; empty polynomial is zero |
| `quartic_field` | F6 aggregate vector field T12 | Finite represented candidate only |
| `quartic_field_enclosure` | The same exact vector field over a parameter box | Existing directed interval arithmetic; no time-flow claim |

Constant and linear polynomial requests use `Tr(c0+c1*q1)=4*c0` immediately
after validating every supplied coefficient. This omits unnecessary high
moments even for very large finite parameters. Unsupported polynomial degree,
malformed coefficients, non-finite state and
overflow return errors. No generic compiler, chart search, JVP fallback,
root enumeration, root filtering, rational observable or hidden Newton solve
is introduced. Resource limits here are structural (fixed degree and output
size), not an adjustable convergence constant.

### Same-target arithmetic reduction

`DirectMoments` computes only `S2,S4,S6` needed by the original aggregate
formula; it does not pay for unused `S5`. `SharedPolynomial` evaluates

```
N = (a*a*(a+4) + b*(3*a+4))/128 + 3/512
G1 = -7*a/8 + N
G2 = -b + a/16 + 2*N
```

This is the exact T12 target. The original identity
`G2 - 2*G1 = 29*a/16 - b` is retained as a control. The source straight-line
count is 13 multiplications / 9 additions for the direct field and
8 multiplications / 7 additions for the shared field. Unary sign changes
are exact and excluded from these two counters. These are source operation
counts, not measured hardware FLOPs, elapsed times or a claim of speedup.

A private static trait evaluates each circuit either with binary64 candidate
operations or with `Interval`. Each attempted operation is charged before
execution, including the operation which detects overflow. Caller-owned
`TraceWork` persists across errors. It distinguishes completed/rejected
requests, scalar and interval operations, moment/shared batches and invariant
queries. Root/Jacobian/JVP/AD/Newton counters are zero because none of these
operations is invoked. Parameter construction, RK4 updates, allocations,
serialization and independent oracle work have separate caller timing scope.

## Native campaign and exact oracle boundary

The example uses the actual controlled F6 problem

\[
 F(t,y)=G(y)+p'(t)(1,1)-G(p(t),p(t)),\qquad
 p(t)=3/4+t/8+t^5/64,
\]

with initial value `(39/64,39/64)` on `[-1,1]`. Classical RK4 runs the
preregistered grids `N=16,32,64,128,256` for both routes. `N=256` was added
to the initial grid with parent approval because the attachment's prior
whole-path certificate first passed its `1e-8` cap there; it is not a repair
or a newly tuned acceptance threshold.

Both state routes use the **same direct-moment forcing calculation** on
every RHS call, charged in full. No forcing setup is hidden outside the
trajectory cost. The state field is the only route-dependent part.
The dyadic nodes/midpoints and this polynomial reference fit exactly in
binary64 on the registered finite grids. Reported node errors are useful
reference observations, not whole-slab physical certification.

Every exported state has its floating value and raw `u64` IEEE bits, and
every node has its value and bits. The independent oracle can recover
identical dyadic inputs, construct its exact rational Hermite path, and
verify original-field Bernstein residuals/tubes without calling this
native circuit as an oracle. Native field enclosures at retained nodes are
timed separately and explicitly limited to `G` at those inputs. They do
not certify the controlled field's whole slab.

### JSON schema shared with the independent audit lane

Top-level `schema_version=1`, `task_id`, `target_id`, `semantics`, `grid`,
`interval_cases`, `semantics_checks`, `runs`, and `timing`.

- `interval_cases`: `name`, `a_bits`, `b_bits`, `moments` (seven `[lo,hi]`
  pairs), `field` (shared two pairs), `field_direct` (direct two pairs),
  `invariant_q1_squared_plus_q2`, `invariant_q1_q2`, and `work`. Cases include
  ordinary, triple, complex-pair and regular branch counterexample inputs.
- `semantics_checks`: `query`, `expected`, `actual`, result/error and work.
  Negative query keys are exactly `selected_branch` and `real_only`.
- `runs`: one trajectory per route/grid, with `states:[{t,t_bits,state,
  state_bits}]`, node errors, candidate timing, work and separately timed
  `node_field_enclosure`. Rejections retain their error and attempted work;
  later registered cohorts still run.
- `timing`: raw samples and warmups, actual resource choices and exclusions.
  It does not duplicate all trajectories on every timing repeat.

### Bounded timing and profiling scope

Seven interleaved repeats alternate direct/shared and shared/direct order.
There is one warmup block per phase/route. Each block has 16 workload units:
this is a measurement resource choice, not a theorem or solver parameter.
Four phases separately measure candidate field, interval field, combined
candidate-plus-enclosure field, and the complete candidate RK4 trajectory.

Field phases share the identical represented state dataset from the retained
direct `N=128` run. Trajectory samples include forcing, state/bit storage,
allocation and destruction; interval phases include parameter validation.
Counters and errors are retained for all warmups and samples. JSON,
diagnostic reference comparisons, dataset construction and the independent
exact whole-path oracle are excluded from timed blocks and stated as such.
The external oracle's wall time must be reported separately in any full
certified-workflow comparison. There is **no native root enumeration
baseline**, so the example does not claim a root-to-trace speedup.

## Meaningful prospective tests

The module includes nine tests, unexecuted by this worker:

1. Integer companion powers independently compute exact moments at ordinary,
   triple, complex and regular-branch parameters, then check native enclosures.
2. Triple-root local length remains four and original-variable invariants
   retain multiplicity.
3. All five public evaluation surfaces reject selected-real/real-only
   semantics before arithmetic, including the regular `(1,1)` branch control.
4. Both field routes are checked against exact small dyadic targets obtained
   from the independent integer companion moments and the original aggregate.
5. Parameter-box extremal moments, explicitly enclosed `1/3`, and a fixed
   polynomial query exercise representation and coefficient handling.
6. Non-finite, malformed, degree and overflow controls retain attempted work.
7. A smallest-positive-subnormal input retains its nonzero moment enclosure
   under the inherited directed arithmetic assumptions.
8. The shared circuit removes actual charged arithmetic relative to an
   already pruned direct-moment comparator.
9. Constant/linear traces remain exactly four for constant coefficient one
   with finite `a=f64::MAX`, even with a finite `f64::MAX` linear coefficient;
   an invalid linear coefficient is still rejected before the shortcut.

The only matrix construction is the small **test oracle**; it is not called
by the runtime provider. No scientific execution or test pass count is
reported before parent publication and execution. The intended central
commands are the package's `trace_quotient::tests` filter and its
`rvj_trace_campaign` example under the common measurement profile.

## C1–C10 state and remaining gates

- C1/C2: original target, exact quotient identities, finite representation,
  admitted native scope and fixed controls inherited from `TRACE.md`.
- C3/C4: additive implementation and independent integer/original-field tests
  written; default dispatcher and other agents' modules untouched.
- C5: complete generated native state bits available for the registered
  independent exact-rational full-path oracle after execution.
- C6/C7: matched target, counters, interleaved timing and candidate/certificate
  phase separation encoded; measurements still pending parent execution.
- C8: the degree-at-most-one shortcut is the same initial prepublication
  C9 episode, recorded as episode **1/3 of N05** with parent approval. It
  receives no separate repair allowance. The prior behavior was a safe but
  unnecessary refusal on unused-moment overflow, not a false certificate.
  No failed result may be discarded by renaming a route or campaign.
- C9: one independent arithmetic/query/code review found no scoped correctness
  blocker and identified the preceding omission opportunity. A targeted
  readback of that small repair is requested; compile/test and central
  integration checks are parent-owned.
- C10: parent must record source commit, executed environment, outputs,
  review disposition and residual production gates in the final handoff.

Fixed polynomial observables are implementation-ready. General quotient
compilation, complete multivariate rank/stratum proofs, selected singular
branch continuation, actual application model providers and full production
solver admission remain separate tasks. The conditional F5 `120/60` count
is not used by any API, loop bound or admission decision here.
