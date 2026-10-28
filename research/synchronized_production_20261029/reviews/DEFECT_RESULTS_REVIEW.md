# DEFECT results and synthesis cross-review

Reviewer: defect_theory. Date: 2026-10-29. Method: read-only source/algebra
review and existing raw-output/receipt inspection; no new numerical execution.

## Scope and verdict

ACCEPT_SCOPED for the conditional exact-rational consumer and the stage/quotient
composition reviewed below. Physical provider/source/corridor qualification and
native outward-arithmetic transplantation remain open. Whole production HOLD.

Actual final proof execution source:
`ef84aa3d6c028b0922bb5c1e66202b95fd88a2fc`, tree
`2989f07fb04134aa7bab38e38d7272e45d6a0917`.
`lane-checks02/proof-oracles/stderr.txt` records25 tests PASS, including all12
DEFECT methods. Exit0, wall0.11423135599943635s, childCPU0.070223s.
stderr SHA256 `82c82aa5045e684acd6d6ef1a567b0aae5ee35933b32c3fae09bc513beb4eedd`.
The current helper and test files match this execution source by git diff.

The retained first source `7fce5a4cbf5e1de3dd25de3b247f638bc551be24` had23/24
overall PASS and10/11 DEFECT PASS. Its stationary Decimal80 diagnostic returned
`1/16+1e-81` while the exact bound was `1/16`. The exact helper was correct.
The test-only correction uses the exact stationary comparison and performs the
nonstationary amplitude subtraction in Fraction. A new mutation control rejects
an underbound of2^-300 even when Decimal80 displays equality. No epsilon or
physical-acceptance relaxation was introduced. The first stderr SHA256 is
`57e842dce0ba75d3e92533a3db86f492382d960c8c76df52d3fc7dfab6e302f4`.

## Theory-to-code review

The radius polynomial, compact-domain premise, exact zero-radius case,
non-strict invariant boundary, nonnegative decay, positive rational exponential
enclosure, whole-slab maximum, norm/reset transport, strict event sign and
transversality conditions match the implemented conditional contracts.

The minimum with the invariant radius is redundant in exact arithmetic because
both convex-combination terms lie below the radius. It can safely cap an
outward-rounded upper in a later native implementation. It is not itself a new
exact-arithmetic performance gain. Failed sufficient radius admission does not
establish physical instability or an impossibility result.

Independently read `stage_transfer.py` and its tests: the strictly lower
triangular recurrence divides actual residual plus inherited stage contributions
by each stage's inverse margin. The reconstruction increment
`(c1+Lf*c0)*max(stage_error)` is correct. The joined example gives exact residual
1/25 and a feasible radius. A directly certified actual-path residual already
contains its stage errors and must not receive this increment again.

Independently derived the identity behind `directional_quotient.py`:
`(1-sigma*z)Q(z)=D-sigma^d*p(z)`; the original residual is therefore
`sigma^d*p(J)b/D`, including the sigma0 identity case. Its dense symmetric-part
Gershgorin upper yields valid positive Euclidean coercivity for the represented
shifted system. This is an exact research oracle; its dense matrix work is not
a native speed measurement and source bounds do not authenticate themselves.

The adapter clock/fiber theory can feed the conservative constant envelope
`delta_j+G*delta0+L*Phi(B)*(C+h*delta0)` into the Gamma0 scalar consumer. Physical
coordinate error still includes `G*(C+h*delta0)`. A requested common time does
not prove zero internal-clock discrepancy. The adapter's explicit convolution
is sharper than that constant-envelope corollary.

## Native evidence boundary

Read the three final RUN records: checks, benchmark and deployment all identify
ef84 and completed execution. The checks record includes Krylov contracts exit101;
other recorded check commands exit0. This failure is not closed by exact oracle
PASS. Benchmark process exit0 and the two manufactured production-rate invocations
do not supply actual physical D1 inputs or general performance qualification.
Detailed native benchmark aggregation belongs to the independent parent/audit
record, not this mathematical subunit.

No further changes or new scientific runs are requested by this review.
