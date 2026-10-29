# Uncertainty register

This register separates asserted upper bounds from estimates and unmeasured
candidate quantities. It does not contain a Bateman candidate result.

| ID | Quantity | Classification | Bound/value | Basis | Admission use |
|---|---|---|---:|---|---|
| U-REF-NOMINAL | L2 error of the stored nominal Bateman reference | `DECLARED_UPPER_BOUND` | `1e-15` | Exact-rational Taylor--Lagrange endpoint enclosure over the exact binary64 inputs | Consumed inside the output budget |
| U-REF-CHANGED-W | L2 error of the stored changed-W Bateman reference | `DECLARED_UPPER_BOUND` | `1e-15` | Same independent method at binary64 `h=0.0005` | Consumed inside the output budget |
| U-REF-MAX-OBSERVED-PROOF | Largest exact verifier L2 upper bound across the two stored references | `PROVED_CONSTRUCTION_VALUE` | `2.075243427511439e-17` | Stdlib `Fraction.from_float`; exact squared-L2 endpoint comparison | Confirms the declaration is not understated |
| U-CANDIDATE | Candidate-to-reference output L2 error | `UNOBSERVED` | none | Local six-case runner not executed | Cannot support a claim |
| U-EMBEDDED | Candidate embedded-error L2 | `UNOBSERVED` | ceiling `2e-4` frozen | Candidate-independent preregistration | Local gate only |
| U-ORIGINAL-TARGET | Original-target residual and contraction | `UNOBSERVED` | ceilings `1e-10` and `1e-8` frozen | Candidate-independent preregistration | Local gate only |
| U-WOLFRAM | Wolfram machine-number cross-check | `SUPPORTING_CROSS_CHECK_ONLY` | no admitted bound | Independent evaluator agreed with the analytic form and values | Never used alone for admission |
| U-LITERATURE | Applicability of cited Bateman/stiff-IVP literature to this exact implementation | `SUPPORTING_NOT_EXECUTABLE` | none | Primary/peer-reviewed literature metadata | Supports model choice; does not prove bits or code coupling |
| U-COMPACT-RECEIPT | Independent recomputation of embedded/residual/contraction scalars from the compact local receipt | `NOT_RECOMPUTABLE_WITHOUT_RAW_VECTORS` | none | The receipt retains scalars, frozen thresholds, booleans, and complete counters but not every source vector | Later review must retain this limitation |
| U-STATE-INTEGRITY | State SHA-256 authenticity | `INTEGRITY_NOT_ATTESTATION` | exact internal digest recomputation only | Unkeyed SHA-256 catches inconsistent bits/hash but cannot attest the executing host against coordinated fabrication | Never treated as host attestation |

## Exact reference method

For each parent/stable-daughter pair,

\[
P(h)=P(0)e^{-\lambda h},\qquad
D(h)=P(0)+D(0)-P(h).
\]

With exact `Fraction.from_float` inputs and

\[
S_n(x)=\sum_{k=0}^{n}\frac{(-x)^k}{k!},
\]

the Taylor--Lagrange remainder sign gives

\[
S_{41}(x)\le e^{-x}\le S_{40}(x),\qquad x\ge0.
\]

The nominal fast exponent is slightly greater than one in binary64, so the
verifier deliberately does not rely on a simplistic monotone alternating-term
argument. A stored rounded binary64 reference also need not lie inside the
very narrow exponential bracket. The proof instead takes the maximum distance
from each stored component to the two exact endpoints and compares the sum of
squared distances with the square of the declared L2 uncertainty.

## Output admission semantics

Define

\[
B=\text{output\_atol\_l2}+
\text{output\_rtol}\lVert y_{\mathrm{reference}}\rVert_2.
\]

For a declared reference-error upper bound `u`, the candidate can pass the
output gate only when every value is finite and nonnegative and

\[
E_{\mathrm{reference}}+u\le B.
\]

The admitted f64 path must not use ordinary nearest-even values as if they were
rigorous bounds. It outward-bounds absolute component differences and their L2
norm, outward-rounds the addition of `u`, and inward-rounds `B`. The receipt
separately records the conservative `output_budget_l2` and
`output_error_upper_l2`. The latter is an asserted true-error upper bound only
when the treatment is `DECLARED_UPPER_BOUND` and the conservative arithmetic
path completed. An `ESTIMATE_ONLY` uncertainty always rejects categorical
admission. The previous expanded-tolerance expression
`E_reference <= B + u` is not valid for a true-error budget and is not used.

The rounding regression includes the case `E=1`, `u=2^-54`, `B=1`: ordinary
binary64 addition rounds `E+u` back to `1` and would falsely accept, while the
conservative path rejects. This repair did not alter the frozen budgets.

## Non-adjustment rule

No budget or declared uncertainty may be changed after the local runner emits
any candidate or fallback observation. A miss, nonfinite value, unexpected
selection, incomplete receipt, or proof mismatch is a result to preserve, not
a reason to widen a threshold.
