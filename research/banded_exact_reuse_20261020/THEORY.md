# Exact factor reuse and scope of numerical equivalence

## Shifted-system identity

In the transformed RODAS5P implementation each stage solves W u_i=b_i with W=I/(gamma*h)-J(t_n,y_n). Here gamma is dimensionless, h has units of time, J and W have inverse-time units, b has state/time units, and u has state units. A finite, nonsingular W is required for the direct solve.

For two attempts,

W_1-W_0=(1/(gamma*h_1)-1/(gamma*h_0))*I-(J_1-J_0).

Holding J fixed while h changes therefore does not in general preserve W. The Wolfram evaluator verified this symbolic matrix identity. With J=[[-2,1],[3,-4]], b=[1,2], oldW=40I-J, newW=20I-J, the old solve is [46/1845,29/615], the new solve [26/525,47/525], and b-newW*(oldsolve)=[184/369,116/123], not zero. The sameW solve has zero residual. These exact rational results give a concrete counterexample to the proposed shortcut 'same Jacobian means reusable factors'; they are not a no-go theorem for all inexact-linear-solve strategies.

## Sufficient implementation-level reuse condition

Cache only after a successful factorization whose stored factors are finite. Retain the binary64 bit pattern of the computed shift inv=1/(gamma*h) and every stored Jacobian entry, together with their fixed dimension and bandwidth (owned by one integration workspace). Reuse only when those patterns all agree. Do not use a hash alone. Failed/missing factor construction invalidates the key before overwriting factors. Stage solves do not modify factors. Signed-zero differences conservatively miss; nonfinite matrices are never recorded.

Under deterministic assembly, partial-pivot tie-breaking and arithmetic, bit-identical finite inputs reconstruct the same factor and pivot arrays. Reusing them changes no stage linear system or operation in forward/back substitution. By induction through the stages, the state update and embedded error, step controller and requested outputs are identical to repeated refactorization. This is a sufficient scoped computational equivalence result, not a theorem about arbitrary compiler fast-math, nondeterministic callbacks, differently rounded coefficients, or cross-platform byte identity. Tests verify factor, pivot and output bits as well as independent residuals.

All Jacobian, RHS and partial-time callbacks still run at every new state. The optimization only skips redundant LU work when both the actual Jacobian and rounded shift match. It does not rely on a user declaration of constant Jacobian. It may help constant-coefficient problems with repeated steps; changing nonlinear Jacobians may yield no hits and incur key comparison/copy overhead. That negative case is part of the experiment.

## Why not approximate Jacobian freezing

Approximate/time-lagged Jacobians are a different integrator modification, requiring appropriate Rosenbrock/W-method order and stability conditions. The present patch does not assert that the existing RODAS5P coefficients satisfy all arbitrary-W conditions. Primary literature discovered via Exa/SciSpace includes Steihaug & Wolfbrandt(1979), An Attempt to Avoid Exact Jacobian and Nonlinear Equations..., DOI10.2307/2006293; Norsett & Wolfbrandt(1979), Order conditions for Rosenbrock type methods, DOI10.1007/BF01397646; Verwer/Scholz and collaborators, time-lagged Jacobian methods, CWI preprint https://ir.cwi.nl/pub/8986 and DOI10.1002/ZAMM.19830630104. Abstract/source metadata were reviewed; no uninspected detailed theorem is invoked here.

## Fixed-width specialization

For lower=upper=2, row storage width is7: two lower entries, diagonal and up to four upper/fill entries. The special functions retain the original Slices assembly loops, pivot comparison '>', row interchange order, division and multiply-subtract order, including boundaries and pivot-induced fill. Only runtime width arithmetic becomes compile-time constants. Other bandwidths explicitly use the original Slices implementation. There is no unpivoted block-Thomas approximation or change to the effective matrix.

## Finite-output contract

A standalone solve returning Ok must not contain NaN/Inf. Reject nonfinite input, check storage-size arithmetic before allocation, and check the output after triangular solve. Do not reject all small pivots: tiny but representable solutions remain valid. A finite-input solution overflow is a numerical representation failure reported as a typed error, not a theorem that the mathematical system has no solution. Scaled triangular solves could extend the representable range but are outside this patch; LAPACK LATRS documents such scaling separately.
