# Preregistration: structural diagonal certificate pipeline and prepared admission context (integrated DAG node INT-02)

## Question

External review finding TF-04 (task N3): for a diagonal model, the q=2 certificate still stores and scans a dense
`n x n` Jacobian and a dense `n x n` inverse witness, so its storage and counted work have an O(n^2) floor that the
blocked `H` does not remove. TF-05: one native q=2 attempt also builds the stage problem and the witness twice
(capability, then certificate). Can a typed diagonal representation run through the whole residual, recurrence,
witness and projection path, and can one immutable prepared context per attempt serve both uses, with every bound
and decision unchanged and every stale or edited input still rejected?

## Changes (opt-in; defaults unchanged)

`rodas5p-integrators`, `outward_certificate.rs`:

- `DiagonalStageProblem { diagonal, y, h, q }` (`J = diag(diagonal)`), validated like `QuadraticStageProblem`, with
  an explicit `to_dense()` for comparisons outside the hot path.
- The inverse witness stores either a dense bound or a diagonal one. `InverseWitness::diagonal_structured` builds the
  diagonal bound from a `DiagonalStageProblem` with the same operations as `InverseWitness::diagonal`. Its identity
  structure is `"diagonal-structured"`, and its Jacobian digest covers the diagonal bits under a separate domain tag.
  `upper()` materializes the dense bound only when asked. `UnverifiedWitness::verify_diagonal` rebuilds and compares
  it as `verify` does.
- `certify_stage_target_diagonal(target, problem, candidate, y_hat, e_hat, witness, atol, rtol)`: the serial
  certificate on the diagonal representation. It touches only the nonzero entries, in the same order as the dense
  path. Its counted operations use the dense formula with the row nonzeros in place of `n`. Its binding digest names
  the diagonal (`J_diagonal`) instead of dense rows. The dense path and its binding are unchanged.

`transactional_q1_q2.rs`:

- `Q2CertificateSource::diagonal_stage_problem` (default `None`) and `DiagonalQuadraticModel`, which wraps a
  `QuadraticModel` whose `A` is diagonal, keeps that diagonal, and returns O(n) stage problems.
- `Q2Admission::PreparedStructuredCertificate(source)`: before the fast path, one `PreparedQ2Certificate` is built for
  the attempt. It holds the stage problem (diagonal when the source offers one, otherwise dense) and its witness. The
  capability decision and the certificate both use it. All binding and consistency checks of the native path stay:
  the model must agree with `f(y)`, with one JVP and with every candidate stage state, and the problem must be this
  step's `(y, h)`.

## Study

The R4 diagonal quadratic family of L-0050 (`A = diag(-1 - i)`, `q_i = -0.05 (1 + i mod 3)`, `y_i = 1 + 0.1 i`),
n in {1, 2, 4, 8, 16, 32, 64}, the same adaptive transactional runs to `t = 0.5` (atol = rtol = 1e-6, initial step
0.05, at most 200 attempts), run twice:

- **dense arm:** `Q2Admission::NativeTargetCertificate(&model)` (current default certificate path);
- **structured arm:** `Q2Admission::PreparedStructuredCertificate(&DiagonalQuadraticModel::new(model))`.

Both arms run through a counting wrapper that records the source calls per attempt. On every q=2 candidate the
dense and structured certificates are also computed directly from the same stages.

## Commands

`INT02_OUTPUT=research/int02_structural_certificate_20261003/RESULTS.json cargo test --release -p rodas5p-integrators --locked --test int02_structural_certificate -- --ignored --nocapture --test-threads=1`

`cargo test -p rodas5p-integrators --locked --test int02_structural_certificate_contracts`

## Gate

**PASS** if all hold:

1. **Same bounds and decisions.** For every q=2 candidate, the structured certificate's `stage_bound`,
   `output_bound`, `embedded_difference_bound` and the four WRMS values are bitwise those of the dense certificate.
   For every attempt, the two arms give the same lane, acceptance, `y_new` bits, error norm, next step and admission
   decision. The trajectories are identical.
2. **Storage and work.** Per certified attempt, the structured arm stores `2n` f64 slots for `J` and the witness
   (dense: `2 n^2`). The least-squares slope of log(counted certificate operations per candidate) against log n over
   n = 4..64 is at most 1.1 for the structured arm, while the dense arm's is at least 1.8.
3. **One construction.** The structured arm builds the stage problem once and the witness once per attempt that
   reaches certification. The dense arm's counts (two each) are reported.
4. **Rejections.** The contract tests reject: a non-diagonal model for `DiagonalQuadraticModel`; a witness for
   another `h`, `gamma` or diagonal; an edited wire witness under `verify_diagonal`; mismatched dimensions;
   non-finite data; a dense witness passed to the diagonal certificate and the converse; a stale certificate under
   `is_bound_to`. A prepared context for another `(y, h)` is not used.
5. **Defaults unchanged.** The existing outward-certificate, R4 homotopy and q2 contract tests pass unchanged, and
   the dense arm is bitwise the pre-change default path. The pre-change path is replayed from the committed
   L-0050 run at n = 1..16: same lanes, acceptance and `serial_certificate_operations`.

Otherwise **FAIL**. Reported, not gated: slots, operations and source calls per n for both arms. No timing; this node
does not activate q=2 admission by default (that is INT-06).

## Stop condition

Any changed bound or decision, or a stale or edited input that is accepted, stops the structured path.

## Prior information

L-0050 (actual q=2 candidates, both certificate arms accept with equal bounds; margins negative for n <= 8). The
external review's TF-04 and N3. No code of this node exists before this commit.

---

## Results (appended after the run at `ab89ce8`)

Output: `RESULTS.json`. Ledger row L-0053. Contract tests `int02_structural_certificate_contracts` 7/7. The existing
outward-certificate, R4 homotopy, q2 diagnostic, transactional, path-action and radius-policy contract tests pass.

**Gate: FAIL** (items 1, 3, 4 and 5 hold; item 2 fails on its threshold for the dense comparison arm).

| Gate item | Outcome |
|---|---|
| 1. Same bounds and decisions | **holds**: all 87 q=2 candidates over n = 1..64 have bitwise equal stage, output and embedded bounds and WRMS values. Every attempt has the same lane, acceptance, `y_new` bits, error norm, next step and admission decision. The contract test also matches the dense certificate bit for bit on every diagonal R3 fixture row |
| 2. Storage and work | **fails as written**. The structured arm stores `2n` slots against the dense `2n^2` (n = 64: 128 against 8,192), and its slope of certificate operations is 1.00. The gate also required the dense arm's slope to be at least 1.8, and it is 1.76 over n = 4..64. Its O(n) terms still weigh at n = 4 and 8 (2,660 and 7,880 operations, against 349,760 at n = 64). The threshold was a wrong prediction about the comparison arm, not a property of the structured path |
| 3. One construction | **holds**: the structured arm builds the stage problem and the witness once per certified attempt, the dense arm twice each |
| 4. Rejections | **holds** (contract tests) |
| 5. Defaults unchanged | **holds**: the dense arm replays L-0050 at n = 1..16 (lanes, acceptance and certificate operations of every attempt) |

Mean certificate operations per candidate (dense / structured): n = 1: 425 / 425, 2: 1,010 / 850, 4: 2,660 / 1,700,
8: 7,880 / 3,400, 16: 26,000 / 6,800, 32: 92,960 / 13,600, 64: 349,760 / 27,200.

For comparison with L-0050's margins: at n = 16 the serial certificate costs 6,800 instead of 26,000 operations per
candidate. This node does not recompute the net-cost margins; INT-06 uses these numbers. No timing.
