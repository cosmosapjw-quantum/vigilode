# One bounded complementary mathematical review

## Scope and independence

Applied physmath-research-loop 1.3.0, one R6 supporting pass for the parent's
R3/R5/R9 objective. The reviewer read only these five new construction files:

- `error_contract/THEORY.md`
- `error_contract/CHECK.py`
- `algebra_contract/THEORY.md`
- `algebra_contract/CHECK.py`
- `EXECUTION_CONTRACT.md`

This is a complementary worker's source-level derivation review, separate from
the two construction workers. It is not an independent model-family proof,
formal verification, native solver validation or a re-audit of the full historical
corpus. Neither CHECK.py was run by this reviewer. There was no recursive review,
new literature search, native execution or source mutation outside this review.
Findings below refer to the statements as read before the parent's corrections.

## Findings and smallest corrections

### R-01: zero-contraction boundary of the refinement floor

**Object:** algebra Theorem C, after C4.

**Observed statement:** if epsilon<=F and R0>epsilon, the uniform recurrence cannot
certify finite success. The paragraph immediately after it treats theta=0
separately, but the preceding impossibility sentence does not restrict theta.

**Concrete counterexample:** theta=0, kappa=epsilon=1/10 and R0=1. Then
F=kappa/(1-theta)=1/10 and R1=kappa=epsilon, so one correction is certified.
An actual scalar realization has A=P=1, f=0, x0=-1, exact residual/correction,
and update error u=1/10; the new residual has magnitude exactly1/10.

**Impact:** the unrestricted sentence is false at a boundary already allowed
by the theorem. It could cause unnecessary provider/precision refusal in a
future planner. The main recurrence C1--C3 and the 0<theta<1 minimum formula
are unaffected.

**Smallest correction:** explicitly restrict the epsilon<=F refusal statement
to 0<theta<1. State the theta=0 branch as: if R0<=epsilon use zero corrections;
otherwise one correction is certified when kappa<=epsilon. If kappa>epsilon,
this uniform bound cannot establish success by repeating the same bound.
Add this exact boundary to the prospective checker if still before source freeze.

**Disposition:** current statement repair before publishing this theorem;
not a defect in the earlier PR93 exact-real correction identity.

### R-02: equality handling in finite-menu pruning

**Object:** error Theorem E4.

**Observed statement:** remove a label if another has no greater error in every
coordinate and no greater cost. The proof relies on a *retained* replacement.
The actual checker is stronger: it requires at least one strict inequality.

**Concrete issue:** if a batch implementation removes every label weakly dominated
by any other label, two identical labels remove each other and the frontier can
become empty. Sequential removal retaining its comparator, or strict dominance,
does not have this problem.

**Smallest correction:** match the checker by requiring at least one strict
coordinate, or explicitly deduplicate equal labels while retaining one
representative of each continuation-equivalence class. The extension/pruning
proof then works as written. Identical numerical error/cost does not identify
different cache/session continuation states.

**Disposition:** small algorithm-contract clarification; the current CHECK.py
already implements the safe strict-dominance variant. No new general theorem
or broader DP search is needed.

### R-03: feasibility of the clipped logarithmic allocation

**Object:** error E5, bounded KKT paragraph after E14.

**Observed omission:** the clipped solution is described with an active budget
equation but without an explicit infeasible lower-bound case or positive
logarithmic-domain bounds.

**Concrete case:** a=w=1, l=u=1, B=1/2 has no feasible eta. No multiplier can
make the displayed clipped expression satisfy the budget. At B=sum(w*l), the
lower bounds are the feasible solution; if all upper bounds fit, the upper
solution is already stated correctly.

**Smallest correction:** require 0<l_i<=u_i in the declared model's valid range
and positive B_i. First reject the relaxation as infeasible when
B<sum_i w_i l_i; then handle lower-bound equality, all-upper-feasible, and the
remaining multiplier solution. This is model feasibility, not a claim that
every original solver is unable to satisfy the physical accuracy request.

**Disposition:** local planning-boundary clarification. The unbounded interior
formula E14, strict-convexity argument and concrete rational example are correct.

### R-04: cubic remainder example should state the sign of g

**Object:** algebra B12.

The displayed upper bound uses g rather than |g|. It is correct for the intended
sealed cubic g>=0, but that sign is not stated in that example paragraph. For a
general negative g the displayed right side is negative while a magnitude is
nonnegative.

**Smallest correction:** say g>=0 explicitly, preserving the intended model,
or use |g| if presenting a general cubic remainder example. The abstract B6--B10
theorem already asks for a truthful nonnegative remainder bound and is unaffected.

**Disposition:** nonblocking assumption-explication, easily corrected together
with R-01--03; no new scientific experiment required.

## Derivations checked without finding a present defect

### Physical-error composition

- E1 adds/subtracts the same exact flow at the actual numerical input on the
  realized accepted grid. Selection may be discontinuous because it is never
  compared at two perturbed states. Tube and local-defect assumptions are explicit.
- E6 and E7 use different intermediate anchors correctly. The Phi(x)=x/2,
  Psi(x)=x example genuinely falsifies the mixed formula. PR93's exact-decay
  truncation belongs with discrete-map nonexpansiveness, not an unmodified stronger
  exact-flow contraction. The algebra leaf B9 is attached to the discrete target,
  so it does not bypass temporal or clock error.
- The telescoping edge convention avoids missing or duplicate charges. In
  particular, represented-stage mean/RHS/lift errors may already be part of a
  residual certificate; output assembly not covered by it remains separate.
- Weighted-infinity norm conversion constants are correct and sharp. Internal
  clock observation displacement is correctly kept outside the next internal
  state recurrence. The nonautonomous time-shift caveat is concrete and necessary.
- E4 dominance remains valid when future options share the same continuation
  contract; its warm/cold counterexample explains why state cannot be dropped.
- E14 and the rational example satisfy the constraint and stationarity. The
  distinction between an upper enclosure floor and a representability lower
  bound is preserved.

### Quotient compiler and original-target residual

- A3 has the correct sign and denominator: multiplication by
  w0= sigma*(zeta-x) gives1-pi(x)/pi(zeta). For nonzero pi(zeta), v has degree d-1.
  Exact annihilation implies W inverse; the near-annihilator inverse bound requires
  theta_pi<1 and complete stored-coefficient defects as stated.
- A6 compiles the stated autonomous/frozen-time raw-U system because its lower
  coupling is exactly a_ij Z+gamma c_ij. It preserves the represented tableau and
  needs no normality hypothesis in the exact quotient case.
- Every stored r_i has degree<d. Consequently B1 has degree<=d, polynomial
  division has a scalar quotient, and B4 correctly extracts its leading term.
- B10 includes base-RHS error and the omitted nonlinear remainder with correct
  signs. The supplied radius bounds only virtual arguments; the separate
  Lipschitz/domain requirement for exact target arguments is explicitly retained.
- Nilpotence of the strictly lower K yields H=omega*sum K^ell without requiring
  a global contraction. Endpoint B9 still requires complete actual evaluation
  error, including coefficient conversion, operator actions and final addition.
- The correlated affine residual B13 gives B14 through the lower block solve.
  B15 needs the stated self-adjoint Hilbert norm and denominator exclusion on
  the whole interval. It is an alternative enclosure, not an extra charge to
  add to an already covering majorant.
- The precision-aware correction identity uses I-AP on the right residual;
  no AP=PA assumption is hidden. C2, C3 and variable-plan C5 follow by direct
  substitution/unrolling. The count6 example is consistent with the exact
  inequalities; this review did not execute its checker.

### Execution and session contract

- S1 assumes the full deterministic state, the callback response law and future
  controls are preserved. It correctly distinguishes retaining the same object
  from copying (t,y,h), and includes effects/provider state where they change
  responses. Pure accounting diagnostics need not be mistaken for physical state.
- The full remaining observation schedule must be registered at probe creation;
  adding a new earlier landing can change the first-window numerical trace.
  Endpoint success is distinguished from remaining global attempt budget.
- Observed fatal failures retain priority. Candidate rollback and work charging
  are distinct. A=E+R and E=C+F(+pending) do not identify attempted operators with
  physical callbacks; this keeps Q-FAIR-AB and the lagged A-P repair separate.
- S2 is valid given a *proved lower bound on final cost* and tie preference for
  the completed candidate. In code, a fixed nonnegative linear work functional
  is a sufficient monotone choice. A generic nonnegative but nonmonotone score
  is not by itself such a lower bound; the proposition's explicit lower-bound
  premise must be preserved. Callback proxy dominance is not a wall-time claim.

## Checker scope and closing boundary

Both checker sources use exact rational arithmetic and include useful distinct
formulations (full-space stage solves, polynomial residual identity, attained
refinement recurrence, exhaustive menu comparison). They do not exercise native
outward arithmetic, a generic domain provider, future cost certificates, or
production callback effects. The polynomial action-count claim is a constructive
algorithm bound, not measured by the checker; its zero-initialized reference
Horner routine is an oracle and need not be the optimized native evaluation.

## Repair disposition and closeout

Before registration, the parent explicitly reported these corrections:

- R-01: the refusal statement is restricted to0<theta<1, the theta0 equality
  branch is stated, and the prospective checker gains an attaining A=P=1,
  theta0, epsilon=F=1/1000 case.
- R-02: strict-coordinate dominance or retention of an equal representative is
  stated explicitly.
- R-03: positive bounded log-domain and B>=sum(w*l) feasibility are explicit.

These are recorded as **parent-reported corrected before source registration**.
The reviewer does not claim an additional execution or a second verification
pass of the repaired bytes. R-04 is a minor assumption-explication sent to the
parent for the same bounded correction episode; it does not invalidate the
abstract residual theorem or require a new experiment.

The parent can freeze source and execute the two declared supporting checks once.
This review is closed and requests no further reviewers, new literature,
whole-corpus repetition or native performance work. Remaining native
domain/rounding/order obligations are already identified in the construction
and independent-PR plan.

## Parent disposition after the closed review

R-04 was resolved before source registration by using `|g|` in the cubic
remainder bound. The published source af7f3e9f55d652554042d2386bf07abb7d857d75
contains this correction. Both registered support checks subsequently passed.
This is a parent execution/disposition record, not a second independent review.
