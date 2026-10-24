# D3 independent source/math review

Reviewer: /root/literature_next_contract.
Scope: fixed manufactured Riccati D3 continuation of S0
d702012e06eeafda5ce30b7c017e51b2c9bcbe4c.
Review method: independent integrating-factor derivation, primary literature
comparison, line-by-line source/contract review. No scientific, test,
compiler, benchmark, NCP, or Desktop Commander execution by this reviewer.

Status: PASS_SCOPED_SOURCE_AND_MATH_REVIEW.
The actual driver source has been reviewed and all material source findings
are closed. The root's campaign and saved-output audit receipts remain
separate execution evidence to be attached after they are available.
This is not native production qualification.

## Skill context used

The governing request supplied physmath-research-loop 1.0.0 (approved
design v0.2): C9 inspects important diffs, scientific/numerical claims and
independence, provides original material to another reviewer, and closes
concrete findings through relevant evidence.
The current installed c38/physmath-research-loop SKILL.md was also read
and reports 1.3.0. Its C9 excerpt says to review changed surfaces and
dangerous assumptions, fix the highest-risk findings, and selectively
recheck affected portions. Its subagent rule bounds work to one production/
verification pass with at most one risk-justified fresh-context review.
These compatible review limits were followed; the current package did not
replace the user's explicit 1.0.0 whole-loop request or change scientific
scope. This reviewer did one complementary math/source pass and re-read
only material finding corrections and the newly completed driver.

## Reviewed identities (initial completed core snapshot)

| File | SHA-256 |
|---|---|
| source/dense.rs | 0335f692d75b4b0a2254050471884518a5357e01db7ad0cb22bb4b6baf15b8b9 |
| source/directed.rs | 2adadb6a5d77073e0a7d3d60aedfbf37e124d15a586a880d7d5a8c8d5307a814 |
| source/domain.rs | ec07aaa6ce1046d659b83c35d21fd7d02fcfe46497cefaae81b8c4081099067e |
| source/test_dense.rs | 487bb0e0c9f6582b716bd7601a5ceb99a986e7bcf4f2fd03dcc763ae4aaf6f5f |
| source/safe_core.rs | 9c19abd69f2a71b83b93e3a0de256c5f85c941cd7c28fee7c63c04cf04e81144 |
| THEORY.md | 3a8736fcc0e656e11d21746c0d096f9735e55b1d6ff6d4b337704a36675d9ccc |

Inherited admission.rs, stage_safe.rs, trajectory.rs and the saved-output
review_dense.py were also read for dependency/context; their previous
scientific validation is reused rather than re-executed.

## Independent mathematical checks

For e=y-p, exact subtraction gives
e'=-[lambda+alpha(y+p)]e-r, r=p'+lambda*p+alpha*p^2.
Exact IVP positivity follows from nonnegative initial data and uniqueness;
polynomial positivity is independently certified from Bernstein controls.
Therefore a=lambda+alpha(y+p)>=lambda and variation of constants proves
the endpoint comparison bound. No positive-tube assumption is needed.
Inherited E0 is mandatory. The exact comparison majorant is monotone
toward rho/lambda, so max(E0,E1) uniformly encloses it.

The scalar model uses lambda in {1,20,300}, alpha in {0,4}.
The full33 affine lift has fixed represented offsets 1,0.5,0.2. Compact
component bounds lift by absolute row sums, including the dependent
rows. The physical reference is the same represented initial IVP.

The Bernstein derivative, product and degree-elevation coefficients in
dense.rs agree with the independent formulas. In particular, derivative
elevation starts from degree n-1 and product has degree 2n. Binomial
ratios are interval divisions of exact small integers, not rounded values
misclassified as exact coefficients.

THEORY.md explicitly handles represented time differences, inherited
radii, original-field defect, query rounding, positivity preconditions,
and external/native gates. No mathematical correction was needed.

## Core source review

- Curve and Certified fields are private; Model parameters and rates are
  fixed and validation rejects unsupported alpha/degree/nonfinite inputs.
- Controls and times are immutable. Public diagnostics/accessors return
  copies, so modifying them cannot mutate a certificate.
- Stored endpoint times define Delta. Interval subtraction encloses their
  exact difference; derivative and query coordinate use that same interval.
- Whole-curve domain is a sufficient Bernstein test. Negative control
  lower bounds return DomainNotCertified rather than a false negativity
  theorem.
- Defect intervals enclose the degree-2n polynomial of the actual stored
  controls. The same alpha stays bound to Curve and Certified.
- The native exp_interval implementation uses range reduction plus an
  interval degree-20 Taylor sum and explicit remainder; it is a reused
  enclosure provider. RN/gradual-underflow/FMA/compiler assumptions remain.
- Negative exponent interval extrema are used in the correct direction:
  gain upper at exponent upper, response from exponential lower, capped
  by Delta.hi as an independently valid upper bound.
- Endpoint and uniform radii use upward products/sums. All 33 full-curve
  component radii are checked against the frozen absolute budget.
- Queries enclose exact represented-time normalization and exact stored
  curve evaluation; separate center arithmetic is compared with that
  interval. The returned radius adds upward full33 solution radius and
  query displacement.
- Query domain checks use the same fixed affine reconstruction ordering.
- All query errors are returned without certificate mutation.
- safe_core.rs forbids unsafe at crate level and includes directed/domain/
  dense only. The inherited MXCSR asm is confined to research admission
  used by the runner/test wrapper; the production FP-provider gate stays open.

No concrete core correctness error was found in the reviewed snapshot.

## Findings and disposition

F1 — fixed query corpus completeness:
The first saved-output reviewer version compared query counts to summary
self-report. Requested 37 + one interior per accepted segment needed an
independent hard assertion and exact expected-time multiset comparison.
Disposition: CLOSED by final source readback. The oracle independently
reconstructs the frozen binary64 tf*k/37 schedule for k=1..37, overrides
the final value with tf as the driver does, requires 37 distinct expected
and actual times, and compares their bitwise multisets. It also checks
exactly one query labelled interior per segment with t0<tq<t1, refuses
unknown query kinds, and retains the total-count checks. This establishes
the intended source contract; actual stored-output conformance is decided
by the root's oracle execution.

F2 — query-rounding-only budget refusal:
The initial 14 focused tests exercised certificate-budget refusal, valid
queries and outside-time refusal, but not the case where U_full passes
and U_full+query_rounding exceeds budget. This is a concrete branch of
the new output contract. Requested one focused test, or reuse an actual
campaign query-refusal record. No unrelated test expansion requested.
Disposition: CLOSED for the core refusal and Session abort contract.
The added d3_query_rounding_only_refusal_preserves_transaction test first
certifies at the exact computed peak U_full budget, observes Budget at
an interior query, aborts the lease and checks the accepted snapshot and
caller-held radius/cursor/history. This isolates the query-rounding-only
refusal from the prior certificate-budget refusal. Actual driver ordering
is separately reviewed in F3.

Root's measured evidence/green_contracts.log records 15/15 PASS, exit0,
with SHA-256 dfbd10b1e29cec901253bd5bc526a273427bd7553b0bd9cfdf404c949f95c6d1.
The revised test_dense.rs SHA-256 is
cf734b7f65c67c6aa3aa2d2cc00420a4ab9506d9bea182a78977a14324f85245.
The current review_dense.py SHA-256 is
d3c9779f3617c3b733a5271f330cb3a1e0b76415be94e18a98f4abae33f7c6fc.
The root BUDGET entries also record safe_core_forbid_compile exit0 and
green_compile exit0. Those actual execution receipts were read; this
reviewer did not rerun them.

F3 — driver commit ordering:
Disposition: CLOSED by source review of source/campaign_dense.rs.
The driver creates one fixed Model per run and verifies alpha bits against
the run specification. Its rates equal the pinned trajectory::rate array.
It checks Curve degree/time/endpoint bits against the accepted snapshot
and trial candidate, then checks certificate alpha/incoming/budget binding.
All requested times in the candidate interval plus one strict interior
point are evaluated and checked in prepare_dense before Session::finish.
The final environment check follows query preparation. Only Accept updates
the inherited endpoint radius, requested cursor, scientific summary extrema
and saved segment. Rejection paths consume the same pending lease and
assert unchanged accepted snapshot, incoming radius and cursor, with the
attempt count increased once. The completion predicate requires final
time, 37 requested outputs, one interior per saved segment, matching
accepted/rejected counts, zero forbidden RHS inputs and both budgets.
No analytic exact solution is called by candidate generation, acceptance,
controller or query evaluation. Requested times do not become hard solver
landing points. File writes occur after accepted commit and an I/O error
terminates the process; external effects are explicitly not rolled back.

The work accounting distinguishes stage_rhs from curve_rhs_vectors and
analytic_jf_vectors. Quintic construction evaluates four equivalent RHS
vectors (including those inside two Jf evaluations), versus two for cubic.
These counts must not be collapsed into a wall-time speedup claim.

## Execution and claim limits

This reviewer performed no compile/test/campaign execution. Root's
measured logs must supply those separate results. A source review PASS
does not replace actual test/compiler/scientific evidence.

Exact Fraction residual and query-displacement checks are independent
of the Rust Bernstein interval implementation. Decimal80 Riccati and
contraction comparisons are high-precision numerical audits, not formal
interval proofs; the source theorem/interval arithmetic supplies the
continuous proof.

The reviewed unit does not close general callback/Jacobian, external
callback side effects, arbitrary initial uncertainty, actual BASS,
native owner/reservation/dirty state, native integration, or the production
FP boundary. Historical D2 native_subagent/independent_personnel status
remains unchanged; this is a new scoped independent source/math review.

