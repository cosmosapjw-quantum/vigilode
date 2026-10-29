# Discriminating mathematical checks

Run `python3 discriminating_checks.py`. This script writes `RESULTS.json` in
its own directory and preserves `FIRST_FAILURE.txt` if a first failure occurs.
The completed run had no failures. No Rust/native solver was run, and no file
in the read-only repository was changed.

These checks supplement the new proofs; they do not establish floating-point
enclosures. Python complex arithmetic and transcendental functions are ordinary
floating computations. In particular, the exact energy identity is tested up to
the displayed rounding error, not claimed for the returned floating points.

* Four collective cases use unequal frequencies and nonuniform weights. Three
  have an actual sum approximately `1+5e-13`, admitted by the current constructor;
  one uses `S=1.2` as an explicitly out-of-constructor generalized-formula stress.
* Both zero and positive damping are exercised, and both the nonlinear scale
  beta and the preconditioned contraction bound rho lie on either side of one.
  Every iteration computes the entire affine recurrence before any projection.
* 251 ball projections activate. Every finite front agrees with the independently
  sequentially composed fine target within the declared floating allowance;
  final discrepancies are at most `7.043e-15`. The target is the fine split-map
  solution, not automatically the exact ODE solution.
* The error recurrence, binomial and factorial majorants are checked at all
  boundaries and rounds. Independently, 1,632 Fraction boundary-round values
  verify the exact rational weighted combinatorial formula, finite front,
  binomial upper bound and factorial upper bound, for both small and large
  derivative defects.
* Twelve scalar cases have an independent closed-form exact nonlinear ODE and
  an independent closed-form repeated Strang map. They test second-order split
  error with damping, exact commuting limits, the general commutator bound and
  first/second continuation sensitivity bounds.
* An altered initial boundary is rejected. Its separately integrated fine
  trajectory is demonstrably a different root; a low shooting residual alone
  cannot certify the prescribed initial-value problem without this constraint.
* Two explicit conditional cost examples have `M=32/64`, `p=16/32`, `m=512`
  fine substeps and six nonlinear corrections. Setup, vector/scalar scans,
  projection, dispatch, final fine evaluation and potential rejection/fallback
  are charged. Conditional span ratios are approximately `2.068/4.091`; these
  are numerical substitutions into an assumed work/span cost model, not timing
  measurements or a hardware speed guarantee. Full fallback is explicitly more
  expensive than the serial comparator.

The zero-damping noncontractive case is particularly discriminating: rho is
about 36.43, yet the finite-front property still reaches the fine target in M
iterations. This establishes neither fast convergence nor speed in that regime;
it prevents an incorrect claim that rho<1 is necessary for finite termination.
