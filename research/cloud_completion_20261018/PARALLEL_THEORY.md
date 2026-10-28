# Constructive parallel transport and homotopy contract

## PT1: outward prefix transport

Assume independently established nonnegative scalar bounds
`e[j+1] <= g[j] e[j] + d[j]`, with finite nonnegative binary64 g,d and e[0].
An exact map is `(g,d): x -> gx+d`. Composition in time order is
`(g2,d2) after (g1,d1) = (g2*g1, g2*d1+d2)`.
For every x>=0, rounding both products and the sum upward produces a majorant.
Replacing any subtree by a larger nonnegative affine majorant preserves that
property. Induction over the up-sweep establishes valid summaries for every
contiguous block. The down-sweep gives a valid prefix before each block, since
the left child inherits its parent's prefix and the right child follows the
complete left summary. Composing each leaf and applying e[0] upward therefore
encloses the exact error recurrence at every output.

This proof does not assume floating-point associativity. Different bracketings
may give different valid upper bounds. The binary tree shape is independent of
Rayon thread count, making successful outputs reproducible across thread counts.
Overflow returns an error before a result escapes; negative/NaN inputs refuse.
Subnormal products use the existing directed primitive's additive outward step.

For N>0 the recursive binary tree has N-1 internal nodes. Its up-sweep
performs N-1 compositions, the down-sweep N-1 right-prefix compositions, then
N inclusive leaf compositions and N bound evaluations. Thus exactly 3N-2
compositions and O(N) storage/work. Recursive fork/join recurrences give
S(N)=S(ceil(N/2))+O(1) for each sweep, hence O(log N) span in the work/span
model and ideal O(N/p+log N) time on p processors. This is implemented by
`AffinePrefixWorkspace::evaluate`; every node/leaf is overwritten recursively,
so no serial O(N) reset remains in that operation. Workspace allocation and
initialization are separate O(N) serial setup, also paid by the allocating
convenience wrapper. No measured speed claim ignores this setup. Small scalar
scans can cost more in scheduling than their serial baseline.

Implementation: `affine_transport::parallel_error_prefix`. This API certifies
only composition arithmetic; it does not manufacture ODE authority for caller
supplied gains. In particular, a gain depending on the unknown incoming radius
must first be bounded on a fixed tube before its leaf may be scheduled in
parallel. The dynamic packet driver uses sequential weighted-ball transport;
the scan is available when fixed certified local maps are already known.

Validation: `affine_oracle.py` derives 220 exact Fraction prefix values from
binary64 inputs, independently of the tree algorithm. Rust checks inclusion,
1/4-thread equality, workspace reuse, operation counts, ordering, zero/empty, subnormal and
overflow cases. These checks supplement the induction proof above.

## PT2: homotopy parallel shooting with a guaranteed iteration bound

Let X be a closed product of window-boundary balls in a complete normed space,
with the maximum component norm. Initial z0 is fixed. Let F_j and G_j be fine
and coarse window maps. Assume each homotopy map
`T_lambda(z)_j = (1-lambda) G_j(z[j-1]) + lambda F_j(z[j-1])`
maps X into X, and the uniform Lipschitz constant is
`q = max_j max(Lip(F_j),Lip(G_j)) < 1` for 0<=lambda<=1.
All components of one Jacobi iteration may then be computed independently.

For z,w in X, convexity gives `||T_lambda(z)-T_lambda(w)|| <= q||z-w||`.
Banach's theorem supplies a unique fixed point for every lambda. Its k-th
parallel iteration satisfies `||z^k-z*|| <= q^k D` if the initial error is at
most D. A completely computable termination condition is
`||z^k-T_lambda(z^k)||/(1-q) <= epsilon`.
If `sup_X ||F-G|| <= Delta`, subtraction of the two fixed-point equations gives
`||z*(lambda+s)-z*(lambda)|| <= |s| Delta/(1-q)`.
Therefore a continuation predictor equal to the previous fixed point starts
the next lambda with the stated bound; lambda=1 is the exact fine shooting
target, not a perturbed equation. Numerical map errors eta enter additively and
produce the explicit floor eta/(1-q); they must be counted in the stopping rule.
For an approximate map T_hat with uniform error eta, the executable stopping
bound is `(||z-T_hat(z)||+eta)/(1-q)`. After k iterations its error is at most
`q^k D + eta*(1-q^k)/(1-q)`.

With M windows, p workers and one-map cost C_F, the ideal fine-target span is
`k ceil(M/p) C_F + setup + communication`, where
`k = ceil(log(D/epsilon)/log(1/q))` for 0<q<1 (zero if D<=epsilon).
For q=0 and D>epsilon, one exact map application suffices.
This is a guaranteed span reduction over M*C_F when the displayed full cost is
smaller. It is a constructive sufficient regime, not a universal speed claim.
It trades total work k*M*C_F for parallel span; no O(M) total-work reduction is
asserted. Homotopy warm starts decrease D using the explicit Delta bound.

For the common-damped collective model, reuse the published energy/logarithmic
norm theorem: on an energy ball E and weight sum S,
`mu_w(Df) <= -kappa + S^2 |chi| E`.
If alpha=kappa-S^2|chi|E>0, an invariant ball gives
`Lip(F_j) <= exp(-alpha h_j)<1`. A coarse map with the same certified contraction
and invariant-ball property meets PT2. This connects the user's homotopy idea
to a concrete physical parameter regime and an explicit iteration/latency bound.
The undamped packet route does not silently inherit these premises.

Native implementation completed in this unit is PT1. A real nonlinear PT2
client still needs F/G enclosures, invariant-ball checks, measured map costs and
endpoint/branch tests. Existing HT01/HT02 results remain historical evidence.
The theorem and precise contract above remove a mathematical planning burden
from that later implementation without describing an unrun client as complete.

## Context and symbolic check

Neumaier, *Global, rigorous and realistic bounds ... Part I* (1994),
https://arnold-neumaier.at/ms/ode.pdf, sections 1–2, supplies context for
logarithmic norm/differential inequality enclosures; this project derives its
own model-specific bounds. Ranocha–Giesselmann, arXiv:2307.12677,
https://arxiv.org/abs/2307.12677, distinguishes residual-based step control from
classical estimators; this work retains explicit continuous verification.
SciSpace discovery metadata and Exa primary-source retrieval are recorded in
`evidence/literature.json`. Wolfram verified the 2x2 symmetric characteristic
polynomial, exact affine associativity and phi1 series. None of these tool
checks substitutes for the monotone interval induction or PT2 premises.

## Independent review correction

The first implementation used level-wide Rayon iterators and serial temporary
initialization. Its PRAM arithmetic depth was logarithmic, but that did not
establish logarithmic native fork/join span. Reviewer dense_transport_review
identified the distinction. It was replaced before publication by the recursive
preallocated implementation above. Initial successful arithmetic tests are
retained; the repaired implementation is retested and separately reviewed.
