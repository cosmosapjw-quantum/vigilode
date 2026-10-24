# Independent review: structured original-residual prototype

Reviewer: integration/root agent, independent of the example author. Read-only source review; existing bounded native receipts are reused. No unresolved blocking mathematical or implementation error was found in the fixed fixtures and stated prototype domain.

The private owned CSR type validates offsets, finite values, strictly increasing unique in-range columns and matrix dimension. Missing entries therefore denote exact zero of the owned operator, rather than an unverified assertion about an opaque callback. It accumulates half the magnitude of each off-diagonal entry into both corresponding symmetric-part row radii. By the triangle inequality these radii enclose the off-diagonal absolute sums of (B+Bᵀ)/2. Diagonal plus radius ≤0 proves dissipativity. This costs O(n+nnz), including initialization, and is valid for the immutable operator until the plan is replaced.

For alpha≥0, the symmetric part of A=I−alpha B is ≥I. Cauchy–Schwarz gives ||Ax||₂≥||x||₂, hence ||A⁻¹||₂≤1. The sparse computation evaluates the original residual b−x+alpha Bx with outward Interval arithmetic. Summing component magnitudes gives an L1 upper bound, also an L2 upper bound, and thus a valid solution-error upper bound. Sparsity changes the work, not the equation or the acceptance tolerance.

The dense comparator materializes exactly the same stored dyadic entries. Its directed residual has a different evaluation order but the same mathematical target. The exact integer oracle is valid only for the hard-coded alpha=1/2 and beta∈{0,1/4} fixtures with representable dyadic components. Its integer cancellation is checked against an explicit 2^53 output limit. It is not a general oracle for arbitrary CSR values. Timings use the same direct triangular candidate and separate setup, solve, certificate and fallback. Dense materialization is excluded and disclosed; a kernel ratio is not an end-to-end ODE ratio.

Malformed/stale/unstable cases and a perturbed candidate are rejected, and fallback is certified on the current original equation. Main timing candidates are manufactured exactly-solvable fixtures. This checks cost reduction without claiming that arbitrary ill-conditioned nonnormal operators admit the same dissipativity bound. The standalone prototype allocates output intervals and is not a zero-allocation production API. Promotion requires integrating a reusable workspace/provider, resource limits and arbitrary sparse cancellation/error tests into the original solver without weakening its acceptance contract.

Reviewed source SHA256: `019bc0fd92d45f7badea0ddb4c4fb10e2cc775286e0d8315e9d0eb9d21e84a9d`.

Native repeated-process receipts: `evidence/conditional-wrapper-native-final-smoke/structured_residual_sweep_*/`. The independent wrapper retains all raw rows and binds the binary/source. The current checkout smoke is exploratory, not a published clean-checkout or user-host performance run.
