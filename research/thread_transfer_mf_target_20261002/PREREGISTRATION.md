# Preregistration: raw-stage (U) and native K target contract (thread-transfer DAG node P0-MF-TARGET)

## Question

The fast driver (L-0032/L-0033) integrates RODAS5P in the raw transformed stages `U = Gamma K` with the snapshot's
own `A`, `C`, `b_code`, so its right-hand side needs no Jacobian product. The review (section 3) proves, for a
constant mass matrix `M` and the step's `J`,

```
(M - h gamma J) U_i = h gamma f(t + c_i h, y + sum_{j<i} A_ij U_j) + gamma M sum_{j<i} C_ij U_j + h^2 gamma gamma_i f_t,
r_U = gamma r_K,
```

for the exact `Gamma = (I/gamma - C)^-1`. The native K target (`StageTarget::sequential`) uses the binary64 faer
inverse instead: strictly lower native `Gamma` and `alpha` with the stated `gamma` on the diagonal. With
`S = strict-lower native Gamma + gamma I` and `alpha0` the strictly lower native `alpha`, for `U = S K`

```
r_U(SK) - gamma r_K(K) = ([(I - gamma C) S - gamma I] (x) M) K - h gamma {F(y + A S K) - F(y + alpha0 K)},
```

so the two targets differ by the coefficient terms `D_Gamma = (I - gamma C) S - gamma I` and `D_alpha = A S - alpha0`,
and the output, embedded and dense projections by `b_code^T S - b^T`, `e_s^T S - btilde^T` and `H_raw S - D_dense`.

Before a matrix-free driver uses the U form (node P1-MF-WORKSPACE), these terms must be enclosed, bound to their
inputs, and the residual tolerances mapped. The claim ceiling is validated stage-target transport, nothing more.

## Implementation (written after this commit)

`crates/rodas5p-integrators/src/raw_stage_target.rs`:

- `raw_stage_allowance(coeffs)`: outward interval enclosures of `D_Gamma`, `D_alpha`, the output, embedded and dense
  discrepancies, with their largest magnitudes; nothing is zeroed and no bit identity is assumed;
- `RawStageReceipt`: binds the target id, a digest of every coefficient bit used (`a`, `c_matrix`, `gamma`, `b_code`,
  `gamma_matrix`, `alpha`, `b`, `btilde`, `dense_h`, `dense_d`), `h`, an operator identity and a residual-scale digest;
  `validate` fails on any difference;
- residual mapping: an absolute or WRMS K-residual budget `tau_K` maps to `|gamma| tau_K` (rounded down); a relative
  criterion is recomputed from the U right-hand side's own norm, never transported;
- `raw_residual_transport_bound`: per stage, `|D_Gamma| |K| + h gamma Lip |D_alpha| |K|` (M = I), upward.

## Tests and commands

1. `crates/rodas5p-integrators/tests/thread_transfer_raw_target.rs` (native contracts):
   `THREAD_TRANSFER_RAW_TARGET_OUTPUT=research/thread_transfer_mf_target_20261002/RESULTS.json cargo test --release -p rodas5p-integrators --locked --test thread_transfer_raw_target -- --nocapture`
   (an ignored test of the same file writes `fixtures/thread_transfer_raw_stage_allowance.json` with the enclosures
   and the coefficient bits);
2. `python3 tools/thread_transfer_raw_target_check.py --allowance fixtures/thread_transfer_raw_stage_allowance.json --output research/thread_transfer_mf_target_20261002/EXACT_CHECK.json`
   (SymPy symbolic identity; exact-rational containment).

## Gate

**PASS** if all hold:

1. **Symbolic identity.** SymPy reduces `r_U - gamma r_K` to zero for generic symbolic 2x2 `M` and `J`, a generic
   3-stage strictly lower raw `C` and `A`, symbolic stages `K` and an undefined function `F`, with
   `Gamma = (I/gamma - C)^-1` and `alpha = A Gamma`.
2. **Outward enclosure.** Every exact rational value of `D_Gamma`, `D_alpha` and the output, embedded and dense
   discrepancies, computed from the coefficient bits with `Fraction`, lies in the corresponding Rust interval.
3. **Native transport.** On a nonautonomous quadratic test problem (n = 3, h in {1e-3, 0.05}, 8 pseudo-random K),
   `|r_U(SK) - gamma r_K(K)|` evaluated in binary64 is at most the transport bound plus an evaluation-rounding term
   `64 eps (|r_U| + |gamma r_K| + 1)` per component.
4. **Receipt invalidation.** A receipt validates on its own inputs and fails after mutating any one of: a coefficient
   bit (each of the ten matrices or vectors), `h`, the operator identity, the scale digest.
5. **Residual mapping.** `|gamma| tau_K` is at most the exact product; a zero, negative or non-finite budget is
   rejected; the relative target is computed from the U right-hand side (a test checks it differs from the
   transported K value on the same stage).

Otherwise **FAIL**. Reported: the largest magnitudes of each discrepancy.

## Prior information

- The review checked the identity symbolically in Wolfram (2x2, 3 stages) and ran 12 Python JVP-only comparisons
  with a NumPy-derived `Gamma` (endpoint differences up to 2.3e-12). `stage_target.rs` documents 28 nonzero diagonal
  or upper entries of native `alpha` (largest 5.6e-16) and 34 of `L` (largest 3.8e-16).
- L-0032 found the fast driver within 2% of the sequential driver on adaptive runs. No code of this node exists
  before this commit.
