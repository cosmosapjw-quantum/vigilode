# Preregistration: signed output-adjoint Laguerre recurrence bound (thread-transfer DAG node P1-LAGUERRE-CERT)

## Question

The Laguerre phi action (`crates/rodas5p-core/src/polynomial_action.rs`) encloses the exact local residual
`eps_j` of every recurrence step but has no valid propagation of them to the output, so its total error stays
`EstimateOnly`; the R4 scalar majorant (`recurrence_majorant`) ignores the three-term cancellation and grows like
`(1 + sqrt 2)^n`. The review (section 5) proves, for symmetric `X` with spectrum in `[0, L']` and the stored
coefficients `c_n` taken as exact,

```
sum_n c_n (t_hat_n - t_n) = sum_{j=1}^m z_j(X) delta_j,
z_(m+1) = z_(m+2) = 0,  z_j = c_j + ((2j+1 - x)/(j+1)) z_(j+1) - ((j+1)/(j+2)) z_(j+2),
```

so `B = sum_j beta_j eps_j` with `beta_j >= sup_[0, L'] |z_j|` bounds the recurrence error of the finite polynomial
output. Does a native interval-Bernstein implementation produce valid and much tighter bounds?

## Implementation (written after this commit)

`crates/rodas5p-core/src/laguerre_adjoint.rs`:

- interval Bernstein polynomials on `[0, L']` (degree elevation, multiplication by `alpha - gamma x`, sums, de
  Casteljau halving), all in directed interval arithmetic; the backward recurrence for `z_j` is run in the
  Bernstein basis (convex combinations, no monomial cancellation);
- `laguerre_adjoint_envelopes(stored, extent, depth)`: `beta_j` as the largest coefficient magnitude over
  `2^depth` subintervals (convex-hull property), for degree at most `LAGUERRE_ADJOINT_DEGREE_LIMIT = 128`
  (a policy; larger degrees are refused);
- `laguerre_adjoint_recurrence_bound(stored, extent, local, depth) = sum_{j>=1} beta_j eps_(j-1)`, upward;
- a public research wrapper exposing the existing Laguerre recurrence with its local residual bounds.

In `joint_phi_action` (Laguerre basis, bounds on, degree <= 128), each column gets a new component
`recurrence_adjoint` (stored-coefficient adjoint bound times `|scale_k|`) and the report a candidate total
`laguerre_adjoint_total` (the bounded components plus every `recurrence_adjoint`) when the enclosure is verified.
**`TotalErrorStatus` is unchanged** (Laguerre stays `EstimateOnly`): the claim ceiling is one new proved
component. A declared-only spectrum gets no candidate total.

## Tests and commands

1. `cargo test --release -p rodas5p-core --locked --test thread_transfer_laguerre_adjoint -- --nocapture` with
   `THREAD_TRANSFER_LAGUERRE_OUTPUT=research/thread_transfer_laguerre_adjoint_20261002/RESULTS.json`; an ignored
   test of the same file writes `fixtures/thread_transfer_laguerre_cases.json` (operators, sources, stored
   coefficient bits, computed recurrence vectors, local residual bounds, `beta_j`).
2. `python3 tools/thread_transfer_laguerre_check.py --cases fixtures/thread_transfer_laguerre_cases.json --output research/thread_transfer_laguerre_adjoint_20261002/EXACT_CHECK.json`
   (exact rationals).

Cases (as in the review): diagonal 3-D `X` with eigenvalues `L'/8, L'/2, L'` and a dyadic-rotated symmetric 4-D `X`;
`(m, L')` in {(16, 1), (32, 1), (64, 1), (32, 4), (64, 4), (32, 16)} and the rotated case (32, 4); stored coefficients
`(1 - q) q^n` in binary64 (q = 1/2 for m = 16, else 3/4), plus random signed coefficients for the identity.

## Gate

**PASS** if all hold:

1. **Backward identity.** In exact rationals, `sum_n c_n (t_hat_n - t_n) = sum_j z_j(X) delta_j` holds exactly for
   arbitrary signed stored coefficients and the actual binary64 recurrence vectors (all cases).
2. **Bernstein bounds valid.** Every native `beta_j` is at least the exact-rational Bernstein-subdivision bound at
   the same depth (which bounds `sup |z_j|`), and at least `|z_j(x)|` at 257 exact grid points; a mutated stored
   coefficient changes the envelopes, and a deliberately lowered `beta` is rejected by the checker.
3. **Enclosure.** In every case the exact recurrence error `||sum_n c_n (t_hat_n - t_n)||_2` is at most the native
   bound `sum_j beta_j eps_j`.
4. **Integration.** On a verified (Gershgorin) Laguerre case of `joint_phi_action` with degree <= 128, all five
   columns carry `recurrence_adjoint` and the report a `laguerre_adjoint_total` at least the bounded components, while
   `total_error` stays `EstimateOnly`; with a declared enclosure there is no candidate total; above degree 128 the
   adjoint is absent and nothing else changes.
5. **Tighter.** The ratio majorant / adjoint bound is at least 5 at m = 16 and at least 1e5 at m = 32 and 64 (the review
   found 7.37, 6.6e5 and 8.2e13).

Otherwise **FAIL**. Reported: setup cost (interval operations) of the envelopes per degree, and native/exact `beta`
ratios.

## Prior information

- The review's exact Python prototype (`probes/laguerre_adjoint_probe.py`) and its ratios above.
- No native code of this node exists before this commit.
