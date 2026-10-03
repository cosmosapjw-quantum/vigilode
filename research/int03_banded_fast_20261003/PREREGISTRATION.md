# Preregistration: a banded pipeline for the fast RODAS5P driver (integrated DAG node INT-03)

## Question

External review finding TF-03 (task P1, banded part): `rodas5p-fast-transformed-v2` skips zero multipliers and stops
row updates at tracked extents, so a banded `W` factors in O(n b^2). But it still stores `J` and `W` as dense
`n x n` arrays, copies and scans all `n^2` entries per factorization (`W = I/(h gamma) - J`, the per-row extent
scan, the one-time density scan), and searches pivots over all rows below. Its cost therefore keeps an O(n^2) floor.
Does an explicit band provider remove that floor, with the same step decisions and states as v2?

## Changes (opt-in; v2 unchanged)

`rodas5p-integrators`, `rodas5p_fast.rs`:

- `BandedJacobian { lower, upper, fill }`. `fill(t, y, band)` writes the Jacobian's band row by row: row `i`, column
  `j` with `i - lower <= j <= i + upper`, at `i (lower + upper + 1) + (j + lower - i)`. Entries outside the matrix
  are written as zero. The provider is responsible for agreeing with the problem's Jacobian. A `fill` that writes a
  non-finite value fails the attempt like any non-finite Jacobian.
- `integrate_rodas5p_fast_banded_observed(problem, band, t_span, y0, adaptive, output)`, driver id
  `rodas5p-fast-banded-v1`. It has v2's controller, clock and output rules, rejection reuse, and transformed stages.
  Only the Jacobian and the linear algebra differ:
  - `W` is assembled on the band (`n (l + u + 1)` entries).
  - The LU uses partial pivoting with the pivot search limited to rows `k..=k+l`. `U` is stored with bandwidth
    `u + l` (fill under row swaps), and the multipliers in an `n x l` array (the LAPACK `gbtrf`/`gbtrs`
    organization). Zero multipliers skip their row update, as in v2.
  - Solves apply the interchanges as they go.
  - Nothing is `n x n`: stored slots are `n (l + u + 1)` for `J` plus `n (2l + u + 1)` for the factors.
  - The result reports counted factor and solve operations (one multiply-subtract counts as 2, one division as 1)
    and the stored slots.
- v2 (`integrate_rodas5p_fast_observed`) is not changed.

## Study

1. **Brusselator 1-D**, interleaved `(u_i, v_i)` (`l = u = 2`), cells in `{32, 128, 512, 2048}` (n = 64 .. 4096),
   `t in [0, 10]`, atol = rtol = 1e-6, initial step 1e-6 (the existing `brusselator` helper and a banded Jacobian
   written from the same formulas).
2. **Viscous Burgers, central differences** (`l = u = 1`), `u_t = -u u_x + nu u_xx`, nu = 1e-3, `u(x,0) =
   sin(pi x) + 0.5`, Dirichlet ends at the initial values, n in `{64, 256, 1024, 4096}`, `t in [0, 1]`, atol = rtol =
   1e-6. Large steps make the advection terms dominate `W`'s diagonal, so pivoting occurs.

Each case runs v2 (dense Jacobian through `with_jacobian_into` from the same formulas) and the banded driver. v2 is
not run above n = 1024 (its dense storage is 134 MB at n = 4096). Above that size only the banded driver runs, and
its counts are reported.

## Commands

`INT03_OUTPUT=research/int03_banded_fast_20261003/RESULTS.json cargo test --release -p rodas5p-integrators --locked --test int03_banded_fast -- --ignored --nocapture --test-threads=1`

`cargo test -p rodas5p-integrators --locked --test int03_banded_fast_contracts`

## Gate

**PASS** if all hold:

1. **Parity with v2.** Wherever both run: the same number of attempts, the same accept/reject sequence and the same
   Jacobian reuses. Every accepted state agrees with v2 to a relative max-norm difference of at most 1e-13 (bitwise
   identity is reported separately). The same final success.
2. **No O(n^2) floor.** At fixed band, the least-squares slope of log(counted factor plus solve operations per
   attempt) against log n is in [0.95, 1.05] for each problem over its sizes. Stored slots equal
   `n (3l + 2u + 2)`.
3. **Pivoting correct.** The contract tests factor and solve random banded matrices that need row swaps (n in
   {1, 2, 5, 17, 64}, several `(l, u)` including `l = 0` and `u = 0`). The solutions agree with a dense LU to 1e-12
   relative, and the pivot rows chosen are the dense partial-pivoting ones. A singular banded matrix gives a typed
   linear-solve error.
4. **Contracts.** A band provider that writes NaN fails the attempt (and the step is rejected, as in v2). A state of
   the wrong length, `lower` or `upper` at least n where n > 1, or a mass matrix is rejected before integration.

Otherwise **FAIL**. Reported, not gated: attempts, operations, slots and bitwise identity per case; v2's counted
dense floor (its `n^2` assembly and scans) for comparison. No timing.

## Stop condition

Lost parity, a pivot outside the band region, or an O(n^2) term left in the banded path stops it.

## Amendment (before any recorded run, while writing the study test)

Neither driver exposes its accepted states step by step; they record states at output times. Gate item 1 therefore
compares the following, instead of "every accepted state":

- the states at 41 equally spaced output times (each an accepted step endpoint, since steps are clipped to output
  times);
- the full `WorkCounters` of both runs, which must be equal. These cover Jacobian builds, RHS evaluations,
  factorizations, solves, accepted and rejected steps, and failures by kind.

The other parts of the comparison (attempts, accepted, rejected, reuses, final success) are as before. No study code
had run when this was written.

## Prior information

L-0033 (fast v2 FAIL on its own performance gate) and L-0041 (small-n specialization) are unchanged. The external
review's TF-03 and P1. No code of this node exists before this commit.
