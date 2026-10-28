# Independent review of conditional fixed-P continuation

Reviewer: conditioned-provider agent, independent of the continuation author.
Read-only review; no kernel edits or redundant test runs. The final example
source SHA256 is
`d31c7d20296d33906ca0098cb92d67a9af1aafebb1026b4ce5708689eeeb0f06`,
which matches `evidence/conditional-continuation/round3-sweep.receipt.json`.

**Verdict:** no blocking numerical or reporting defect found for the stated
fixture scope. This is conditional nonlinear root continuation, not a verified
whole-ODE production result.

The cubic fixtures have nonnegative x in the declared balls and positive
coefficient, so the endpoint maximum of the affine-in-x-squared derivative
expression bounds the full contraction interval. The dyadic P choices, centers,
radii and polynomial coefficient make the evaluated contraction extrema exact
here. The declared center-correction bounds have adequate slack over the full
lambda intervals, including the offset of every vector component. The two
split patches separately satisfy their stated self-map inequalities.

The proof of finite-precision iteration error correctly distinguishes an exact
self-map from the stronger perturbed-invariance condition. A failure of its
sufficient floor/budget inequality is not used as impossibility evidence.
Only the fixture-wide lower bound on reported callback uncertainty enables the
zero-callback floor refusal. Callback roundoff allowances dominate the biased
polynomial evaluation in the declared domain. The finite iteration and total
callback caps remain authoritative.

The root-distance certificate and residual tolerance are separate. Conditional
root refinement reserves a smaller residual target but still checks the actual
returned root bound; the wrapper keeps its candidate private if that stronger
requirement fails. The initial-center fallback is restricted to admission to
the first unique-root patch. It cannot bypass the cross-patch branch inclusion
check. The quadratic opposite-root test uses individually valid patches with
identical branch labels and still refuses their disjoint connection. Final
inverse identity includes the exact binary P value; the added swapped-provider
test refuses before residual/inverse callbacks.

The main residual and tight bisection reference are binary64 diagnostics rather
than independent interval root enclosures, as explicitly documented. Admission
continues to depend on provider bounds and the kernel's a posteriori root/
residual check. The scalar monotone bisection comparator shares the reported
residual uncertainty and does not update a bracket using an ambiguous sign.
It solves only the final target; continuation solves initial/final stations,
plus a midpoint for the split case. This unequal scope is openly identified
and does not support a general continuation-versus-bisection speed claim.

All solve allocations and wrapper decisions are timed; reference solves and
diagnostic/JSON construction are excluded symmetrically from the final-round
measurements. Failure timings remain visible and cannot be promoted as
speedups. `speed_comparison_eligible` describes the same-contract raw/conditional
comparison; all eligible rows in the final sweep also have successful bisection.
A future expanded catalog must continue checking comparator success before
using its timing ratio. A root-error refinement changes the error requirement
and is excluded from that same-contract flag.

Reused evidence: round3 contains five passing targeted tests and 16 cases times
31 repetitions times three routes (1,488 timed solves), plus a successful
expected branch-refusal guard. Eleven raw cases and thirteen conditional cases
return scoped success; retained cap/domain/root-budget/floor refusals are
expected outcomes, not hidden failures. The receipt exits zero, and final
source/binary identities are preserved. These short, single-process timings
need the separate local campaign before performance promotion.
