# E-10 Mutation adequacy (tableau coefficient)

Two mutants of `fixtures/rodas5p_coefficients_snapshot.json` entry A[3][1] were built and the full default workspace test suite was run in an isolated worktree at b3e8165 (dev profile).

| mutant | relative perturbation | tests failed | detected |
|---|---:|---:|---|
| A1 | 1e-8 | 10 | yes |
| A2 | 1e-4 | 21 | yes |

Baseline (unmutated) suite: 0 failures. Failing tests are listed in `failed_A1.txt` / `failed_A2.txt` and in `results.json`. Detection at 1e-8 relies mainly on the fixture bit-identity test and the tight scalar-linear exactness test; the fifth-order slope tests also fail because their forced refinement drives the perturbation above round-off.

Mutant B (controller exponent) was not run (session budget). The worktree and its separate build directory were removed after verifying the worktree was clean.
