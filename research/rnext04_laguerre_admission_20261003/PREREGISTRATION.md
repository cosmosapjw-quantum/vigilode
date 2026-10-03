# Preregistration: end-to-end Laguerre total admission in the verified domain (remaining-only DAG node R-NEXT-04)

## Question

`joint_phi_action` in the Laguerre basis reports `total_error = EstimateOnly`, because its recurrence rounding had
no propagation bound. L-0039 added a proved signed output-adjoint bound for that component (`recurrence_adjoint`),
and the report carries a candidate `laguerre_adjoint_total`. Does that total cover every error component against
the same exact target, with nothing missing or counted twice? If so, can it be admitted through a guarded,
opt-in API that refuses everything outside the verified domain? The existing `total_error` field is not changed.

## Component-coverage claim (to be written out in full with the results)

Exact target: `F = sum_k s_k phi_k(h A) v` (same-vector input) or `sum_k phi_k(h A) w_k`, for the binary64 `A`, `h`,
`w_k` taken as exact reals. With `beta` the stored binary64 scale, `X = -A / beta` exactly and
`phi_k(hA) = phi_k(-a X)` with `a = h beta` exactly. The coefficient enclosures are computed for that exact `a`, so
the transform contributes no error term (`E_transform = 0`, by construction). The verified Gershgorin enclosure gives
`spec(X) in [0, L']` with `L'` rounded up. Then, per column `k` and with stored coefficients `c~_n`, exact `c_n` and
computed vectors `t^_n`,

`computed_k - F_k = [computed_k - sum c~_n t^_n] + sum c~_n (t^_n - L_n(X) w) + sum (c~_n - c_n) L_n(X) w
- sum_{n>m} c_n L_n(X) w`,

which correspond to `summation`, `recurrence_adjoint` (L-0039, summation by parts with exact local defects),
`coefficient` (radii times `e^{L'/2} >= ||L_n(X)||` times `||w||`) and `truncation`. Add the fused-sum rounding
(`fused_summation`) and, for inputs outside the normalization window, `normalization` plus the subnormal rounding
of the scaled results. `E_total` is their upward-rounded sum, and `laguerre_adjoint_total` is supposed to equal it.

## Implementation (written after this commit)

- `JointPhiReport::admit_laguerre_total(budget) -> TotalErrorAdmission` (opt-in). It returns `Admitted` only for:
  basis Laguerre, execution `certified-enclosures`, Gershgorin-verified enclosure, recurrence branch with degree at
  most `LAGUERRE_ADJOINT_DEGREE_LIMIT` and every column's `recurrence_adjoint` present, a finite
  `laguerre_adjoint_total` at most the budget, and a finite nonnegative budget. The scalar branch defers to
  `total_error` (already `Certified` there when verified). Everything else is `Rejected` with a reason.
  `total_error` stays `EstimateOnly` for the Laguerre recurrence.
- A component registry (`LAGUERRE_TOTAL_COMPONENTS`) naming the six components and their report fields, with a
  function that recomputes the total from the fields with upward rounding.
- Bounded resources: `LAGUERRE_ADJOINT_MAX_DEPTH = 12`; the public envelope functions refuse a larger depth with
  `LAGUERRE_ADJOINT_UNSUPPORTED` before any work. `EnvelopeKey` gains the resource limits (degree limit, maximum
  depth), so a cache entry made under other limits is not reused.

## Tests and command

Native (`crates/rodas5p-core/tests/rnext04_laguerre_admission.rs`) writes `cases.json` here, then an independent
50-digit mpmath check (`tools/rnext04_laguerre_check.py`) writes `RESULTS.json`:

`RNEXT04_CASES=research/rnext04_laguerre_admission_20261003/cases.json cargo test -p rodas5p-core --locked --test rnext04_laguerre_admission -- --nocapture --test-threads=1`
`python3 tools/rnext04_laguerre_check.py --cases research/rnext04_laguerre_admission_20261003/cases.json --output research/rnext04_laguerre_admission_20261003/RESULTS.json`

Cases (fixed now): symmetric `A = Q diag(lambda) Q^T` rounded to binary64 and symmetrized, n in {6, 12, 20}, spectra
in `[-rho, -lambda]` with `rho` in {1, 50, 400} and `h` in {1e-3, 1e-2, 0.1}; distinct and same-vector inputs; a
near-cancellation case (`w_0 = v`, `w_1 = -v / (h * 0.99)` so the fused sum nearly cancels); amplitudes 1e-310
(subnormal) and 1e300 (normalization path); the scalar branch (`A = -2 I`); a declared (unverified) enclosure; a
degree above the adjoint limit; the timing (unbounded) execution; a Chebyshev report. The exact `F` is
`sum_k phi_k(h A) w_k` by a 50-digit symmetric eigendecomposition of the binary64 `A`.

## Gate

**PASS** if all hold:

1. **Coverage.** For every recurrence-branch Laguerre case, `laguerre_adjoint_total` equals the registry's
   upward-rounded recomputation from the component fields (no component missing or duplicated), and the written
   derivation accounts for every term of the error identity above.
2. **Enclosure.** For every admitted case, `||fused - F||_2 <= bound` against the 50-digit `F`. The tightness
   `bound / ||fused - F||` is reported.
3. **Guards.** The declared-enclosure, degree-above-limit, timing-execution and Chebyshev cases, an invalid budget
   and a budget below the bound are `Rejected`. The scalar branch follows `total_error`. Every Laguerre recurrence
   report still has `total_error = EstimateOnly`.
4. **Bounded resources.** A depth above 12 is refused before any work. A cache keyed under different limits does not
   hit. The existing Laguerre adjoint and cache tests pass unchanged.

Otherwise **FAIL**. Stop condition (DAG): an unbounded component or an unresolved physical scaling stops admission
(the case stays `EstimateOnly`/`Rejected`). Claim ceiling: admission on a verified symmetric nonpositive domain at
degree <= 128; no non-normal or general-matrix claim; no speed claim.

## Prior information

L-0039 (adjoint bound exact-checked, 7.35x to 7.6e13x tighter than the majorant), L-0042 (cache). No code of this
node exists before this commit.
