# Execution, selection and reuse in the integrated polyalgorithm

## Scope and reuse

This is a source-grounded R3/R5/R9 construction for PR90--93. It reuses the
existing `AdmittedFusedAction`-style guarded polynomial result, charged structural
failures, `ExplicitSession`, integer comparator and original-target residual checks.
It proposes small connections between them; no new universal public certificate
framework or rewritten solver hierarchy is required to implement the design.

The current production facade covers smooth forward identity-mass IVPs with
endpoint observations. Its default is persistent RHS-only DOPRI5. A supplied JVP
is an explicit derivative-assisted capability. DAE, arbitrary geometric splitting,
generic nonlinear causal order and generic physical error certification are not
made supported by renaming an internal result.

## 1. A request has a target before it has a preferred algorithm

A request fixes the physical/model epoch, input state and time, output observations,
norm, domain, arithmetic, callback permissions and one mathematical target:

| Target | Compatible alternatives | Final check |
|---|---|---|
| original shifted linear system | declared quotient/direct, Chebyshev correction, qualified Krylov | original-system residual and inverse/transport bound |
| fused phi combination | enclosed Leja, forward signed-adjoint, backward Clenshaw | complete action-error bound for the same inputs and coefficients |
| original nonlinear stage/root | value-only Picard, rational/scalar candidate, qualified homotopy | original residual, physical branch and domain |
| accepted physical step | explicit RK, qualified causal/Rosenbrock, declared exact flow | temporal plus algebraic/rounding error under a valid anchor |

Polynomial degree reduction does not authorize a change from a phi target to a
resolvent target. The quotient-stage compiler in `algebra_contract/THEORY.md`
returns the same discrete Rosenbrock target; its temporal error is still supplied
by a separate step contract. `error_contract/THEORY.md` gives the two legitimate
anchors that make this composition work.

An implementation can carry these fields in private adapters around existing
types. A returned success is eligible for comparison only after the relevant
domain/target/physical gate. Historical work counters and elapsed seconds remain
different quantities. A short successful solve that misses the required physical
accuracy is not a cost competitor.

## 2. Three kinds of reuse, with different invalidation rules

1. **Storage:** capacity/alignment/scratch can survive many target changes after
   initialization. This saves allocation without asserting mathematical reuse.
2. **Algebra:** coefficients, factors, basis vectors, FSAL derivatives and nonlinear
   histories require the identity each existing provider declares. Step/carrier,
   model, norm, chart, parameter, time and arithmetic changes may invalidate them.
3. **Proof:** a bound may be reused only on its stated domain and norm, with its
   target and anchor unchanged. A numerical input lying in a new certified tube
   can justify semantic reuse even when its bytes change, if that theorem says so.

Do not make all three categories depend on one cache-hit boolean. In particular,
retain homotopy workspace capacity while clearing Anderson history on map changes.
Use exact epoch/bit keys already implemented for cached RHS or conditioned charts.
No opaque callback is granted a whole-space annihilator from sampled vector actions.

## 3. A winning probe is a live computation

The latest `switching.rs::run` creates a probe using `Dopri5::with_session`, extracts
its endpoint and proposed step, and returns from the closure. Its native session
then ceases to exist. `session.rs::ExplicitSession` and
`rodas5p-integrators/src/dopri5.rs::Dopri5Session` already preserve workspace,
controller, FSAL, full output schedule, cursor, cumulative attempt budget and
terminal state while the closure is alive.

### Proposition S1: continuation by retaining state

Let a solver be a deterministic transition system on a full session state S.
The next transition is determined by S, the fixed callback response law, and the
next external control action. Split an otherwise identical execution word at an
accepted checkpoint, inspect S without mutation, then continue the same object.
The subsequent state, accepted/rejected decisions, output sequence and native
callback trace equal those of the unsplit execution. Numerical work is the sum of
the two disjoint trace segments.

Proof: the first states after the pause are identical. Induction on subsequent
control actions gives equal next states and emitted events; concatenating disjoint
event segments gives the accounting statement. This is full-state continuation,
not a claim that copying `(t,y,h)` reconstructs the session.

The comparison permits different host allocation/bookkeeping overhead at the
pause. It does not assert equal elapsed time. It assumes the callback response
law is unchanged. If probe callbacks mutate physical provider state or future
responses depend on a shared call counter, that state must be included/isolated;
pure diagnostic atomics alone can remain globally charged. Observed fatal failures
retain the existing priority and cannot be erased by choosing another candidate.

### Concrete schedule requirement

Current probe construction uses `[start,target,solve_end]` (or the terminal pair).
An `ExplicitSession` may advance only to registered landing observations. Keeping
that object and requesting a new interior observation therefore fails its real
API contract. The direct implementation is to construct the winning-capable probe
with the complete intended remaining observation schedule from the beginning and
keep selection/continued advance inside the same `with_session` closure.

No callback-owning self-referential heap object is needed for this scoped design.
Any first-window trace-parity claim must additionally check that the initial
schedule has the same landing times up to that window; adding an earlier landing
changes the numerical path and needs a scientific comparison. An exact checkpoint
at the requested endpoint can coexist with exhaustion of the global attempt
budget, so endpoint success and permission to continue must be tested separately.

Required integration controls: interrupted versus uninterrupted schedule; a
winning probe followed by at least two future observations; invalid/new schedule
target; model/provider/norm/tolerance epoch change before a callback; global quota
exhaustion at and before an endpoint; fatal and recoverable provider errors; and
same selected path with reuse on/off. Existing native session tests are reused;
the winning-probe interaction is an additional test obligation, not already PASS.

## 4. Work is conserved across rejection and branching

For a completed collection of requests define A=requested invocations,
E=admitted invocations, R=refused before kernel entry, C=completed admitted
invocations, F=failed admitted invocations. Then

    A = E + R,        E = C + F.

While workers are outstanding, include P=pending in the second identity. Every
scheduled worker must be joined or otherwise accounted before P is taken as zero.
These identities do not identify requested invocations with actual callback calls;
each physical callback/action ledger has its own entered/completed/failed counts.
This unifies the two distinct old qualifications: Q-FAIR-AB is A versus E, whereas
the PR92 lagged repair compares entered operators with already charged physical
JVP/matrix entries. Neither can be repaired by adding the other's counter twice.

Work vectors contain nonnegative physical units, for example RHS calls, interval
evaluations, supplied JVP actions, scalar coefficient work, allocations/bytes,
certificate actions and synchronization. Nonselected candidates still add work.
A refused expensive kernel can have zero kernel calls and nonzero scheduling or
validation work. Native elapsed time and Callgrind instructions are recorded in
their own units rather than inferred from callback counts.

### Proposition S2: conservative stopping of a dominated trial

Fix a nonnegative scalar cost functional of the additive work vector, and a
completed admissible competitor with cost C. If an unfinished trial has accumulated
a proved lower cost bound L and L>=C, it cannot finish with strict lower cost.
It may be stopped when the decision rule prefers the completed competitor on a
tie. For integer costs strict improvement over C is equivalent to final cost at
most C-1. These are the source of the exact boundary and budget cap, not fitted
margins. Future unexecuted fatal callbacks are not predicted by this proposition.

The statement is about the named cost functional on the same physical window and
accuracy target. Callback-unit dominance does not imply a wall-time ordering.
Latency-based scheduling needs measured service costs or an explicitly limited
cost model; numerical-analysis constants cannot determine a processor's latency.

The PR93 residence theorem remains conditional on an independently valid future
variation bound. Session reuse makes observed work cheaper; it does not prove
future cost variation or replace that missing assumption.

## 5. Concrete combined execution sequence

1. Retain the accepted physical checkpoint, full error state and all charged work.
2. Inspect only declared model structure and callback permissions. Reuse cached
   algebra whose identity is valid; otherwise prepare it with charged cost.
3. Build the exact request/consumer contract. Use quotient/direct structure when
   eligible; otherwise compare the appropriate polynomial, causal/root or Krylov
   candidates. Endpoint-only consumers may omit stage outputs only under the
   compiler theorem; dense/event consumers require their own output functionals.
4. Derive the remaining physical error budget from a valid anchor/transport, then
   use finite candidate frontiers or proved correction/degree inequalities.
   Do not split the budget into arbitrary fixed fractions.
5. Evaluate candidates with original-target admission and complete work capture.
   Preserve failed attempts. Stop a dominated comparison only under S2.
6. Commit one admissible physical result and its applicable proof state. Continue
   a retained winner under S1; preserve callback-effect delivery and global budgets.
7. Advance the ordinary controller with its declared estimator semantics. A new
   reliable error bound does not by itself prove controller stability.

## 6. Implementation order and positive completion criteria

First qualify the cumulative existing implementation and the seven Q tasks in a
separate PR. Then implement N01 fusion and N02 planning as independently measurable
changes, followed by N04 retained-session continuation. The quotient-stage compiler
is a focused N03/N05 extension, with native outward residual and temporal gates.
N06 residence remains behind its future-variation proof obligation. Existing task
budgets are shared by their alternatives; creating this synthesis does not reset
the already used 3/3 structural or adapter correction loops.

A valid theory connection is complete when its assumptions, proof, computational
form and falsification cases are provided. A native route is promoted only after
its original accuracy, failure, full-cost and required release gates are run on
the stated source/binary. A future independent PR should report both kinds of
completion, without treating this research package as the latter.
