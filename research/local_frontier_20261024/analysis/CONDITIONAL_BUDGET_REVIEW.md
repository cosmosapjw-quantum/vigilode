# Independent review: causal sensitivity and finite work allocation

Reviewer: the N3 regime/cost implementation agent, independent of this planner's
author. Scope: `CONDITIONAL_POLICY_KO.md`, `conditional_budget.py`, its seven
unit tests, and the nine preserved study rows. No planner or solver source
was edited by this review.

**Verdict:** no blocking mathematical or implementation error found for the
stated finite, exact, caller-supplied-envelope problem. This does not validate
those envelopes for an ODE or establish an online runtime improvement.

## Matrix order and causal propagation

Write M=diag(M_i). From d<=Mr+Kd with nonnegative strictly lower triangular K,
the nonnegative inverse S=(I-K)^-1=I+K+...+K^(s-1) gives d<=SMr. Therefore
|b|^T d<=|b|^T S M r, and the weight vector is **M^T S^T |b|**.
Because M is diagonal this is M(I-K)^-T|b|, exactly the implemented order.
Moving M to the other side of S would be wrong in general; the implementation
does not do so. The reverse loop solves z=|b|+K^Tz and then multiplies by M.
Weights are nonnegative, not necessarily strictly positive when output
sensitivities vanish. The code correctly permits zero weights.

This proof bounds additional stage-solve error under a verified comparison
inequality. It does not prove a general nonlinear inverse estimate, temporal
order, absolute stability or a global ODE error certificate. The document
appropriately requires separately justified local operator/Lipschitz bounds.

## Floors, feasibility and Pareto pruning

The floor test sum(w_i e_i)>E is a sufficient reason that these declared
envelopes cannot certify the target. The converse is not asserted by the code.
At equality, positive q and a positive weighted initial excess can require
infinitely many iterations; the finite DP correctly returns None. With q=0,
one iteration can reach the floor exactly. Initial=constant-floor is valid at
zero work. These boundary cases were checked independently.

After the first i stages, each DP state stores total integer work, the least
declared error achieved at that work, and a witness path. Future choices are
independent additive nonnegative costs and errors. Thus a state with no lower
cost and no lower error is dominated for every identical continuation. Keeping
the smaller error at equal cost and then removing dominated increasing-cost
states preserves a minimum-work feasible solution. Induction over stages gives
the claimed exact optimality over the bounded choice set. This proof requires
the stage envelopes and costs to be fixed for the allocation; state-dependent
cost/contraction updates would require a new state representation.

## Complexity and implementation scope

There are at most W+1 integer cost buckets and k_max+1 choices per stage, so
O(s W k_max) **state transitions** is a correct bound. It is not the full
Python runtime bound: sorting costs, constructing witness tuples, precomputing
rational powers and growing integer bit sizes contribute overhead. The report
already limits its statement to transition counts/rational operations and
disclaims a bit-complexity or wall-time bound. The planner is pseudopolynomial
in the numeric work cap, not polynomial in its bit encoding.

The accepted API requires E>0 even though the mathematical inequality also
makes sense at E=0. This is an input-domain restriction, not an incorrect
feasibility answer. Costs and iteration caps are bounded integers; actual
fixed setup work must be charged separately as documented. An identical setup
cost cannot change the argmin, but route-dependent setup would need inclusion
in a wider optimization. Fraction conversion of a float captures its exact
binary value; callers wanting decimal fractions should pass strings.

## Evidence and reproducibility

The existing seven-test log and matching source hashes were reused rather than
rerunning unchanged tests. New independent checks exhaustively enumerated 64
small seeded allocations, including nonzero floors, zero weights, q=0,
zero iteration caps and finite work limits. All minimum-work answers matched
the exact enumeration. Three explicit floor-boundary checks passed. Another
24 exact-rational causal systems confirmed equality between propagated stage
errors and the weighted residual expression. Results are recorded in
`CONDITIONAL_BUDGET_REVIEW.json`; the deterministic replay source is
`../src/review_conditional_budget.py`. These are mathematical correctness tests,
not ODE performance samples.

The nonblocking reproducibility item is **addressed**. All nine rows in
`CONDITIONAL_BUDGET.json` now explicitly include the lower sensitivity matrix,
output coefficients, inverse bounds, initial residuals, floors, contractions,
unit costs, iteration caps and work limits, along with each target and result.
This was checked by inspecting all nine input records; no unchanged experiment
was rerun. The planner source and recorded numerical outputs remain unchanged.
The independent exhaustive checks above still refer to their own declared
seeded cases, rather than being mislabeled as exhaustive checks of these nine
study inputs.

## Added global-order sufficient condition

The added section 3 error recursion and global bound are correct for positive
accepted step sizes, L>=0, a common norm, and uniform valid map-Lipschitz,
local-truncation and additional-solve-error bounds over the relevant domain.
The comparison is between the inexact update and the baseline map at the
same accepted starting state. With A=C+c, unrolling gives

    e_N <= product_j(1+L h_j) e_0
           + A h_max^p sum_j h_j product_{k>j}(1+L h_k).

For L>0 the weighted sum telescopes exactly to
`(product_j(1+L h_j)-1)/L`; bounding the product by exp(LT) yields the stated
coefficient `(exp(LT)-1)/L`. For L=0 the sum is T. Thus no uniform-step
assumption is needed. The proof also holds for adaptive accepted step sequences
when all these actual-step bounds remain valid. Preserving global order p
requires exact initial data or e_0=O(h_max^p), with constants independent of
refinement. Otherwise the displayed initial-error term remains, as it should.

This is a sufficient conditional convergence statement, not a claim that an
embedded estimate supplies these bounds, that a nonlinear solver residual
alone controls the update, or that a large stiff Lipschitz factor is useful.
The document explicitly preserves those restrictions and does not claim an
expanded absolute-stability region. No planner/solver source or execution
result changed during this additional mathematical review.

Reviewed implementation SHA256:
`6ad63e9e4017db45322f54ce6184c949401aeda71e2515f8924c5dbf9d486049`.
Reviewed test SHA256:
`d686e1e1d42135e99216db361b9bce96d111c2f78c2e6e282a7b00b0b9262808`.
