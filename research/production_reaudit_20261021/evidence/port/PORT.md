# Published all-DAG source integration

The published A01–A11 certificate implementations are now present beside the current production paths. This closes the missing-code integration gap; it does not switch the public `vigilode` facade to a new solver or establish a generic production/speed claim. On the integrated main worktree, 72 native integrator contracts, 13 native core contracts and two compile-fail token doctests passed. Three additional compatibility targets still require the root's final fresh-target retry after build failures described below.

## Source and retained evidence

- Integration base: `4a5fc656730fe8243f06b6e603cb8aa758b00b2f`.
- Published package imported: `33e45625be0803771156704263c3b1c1f92d2b40`.
- Later research head inspected: `ee65d4dbecf3b3cd5f4587c27f87ef297d2bb406`. Its additional component benchmark suite was not required for this solver port.
- All 168 tracked files under `research/all_dag_completion_20261018/` are byte-identical to the published package. Previous proofs, negative benchmark results, logs and manifest hashes remain intact.
- `PORT.json` records 62 imported/combined source, test, example and tool files, with published Git blob and SHA-256 mappings. Formatting-only changes are retained in the current file identity. Its source inventory is an observation before the root's final source freeze, not a substitute for that freeze.
- No old ledger rows were merged or rewritten. The old `SOURCE_MANIFEST.json` intentionally describes its historical source. Running its old validator against this extended source should not be advertised as a fresh pass.

The initial merge inventory was produced in a separate detached worktree and found four source conflicts plus a ledger conflict. All source conflict resolutions were performed explicitly; the original production `bound_pr101.rs` remains byte-identical to the integration base.

## Compatibility resolutions and proof obligations

### Packet output: preserve both displacement and carryable radius

The production API exposes the outward weighted distance between its exact represented polynomial and the binary64 centers returned to the caller. The published research API also needs a weighted ball that can be carried into the next verified window. Both quantities are retained.

Let `Y_j` be the exact stored polynomial, `c_j` its returned center, and let the existing interval evaluation produce `|Y_j-c_j| <= delta_j`. For the actual nonnegative stored weights `w_j`, compute

\[
\eta=\sqrt{\sum_j w_j\delta_j^2}
\]

with outward products, additions and square root. If the existing weighted owner proves `||y-Y||_w <= R`, the triangle inequality proves

\[
\|y-c\|_w\le R+\eta.
\]

`weighted_center_displacement()` returns the same-query outward `eta`. `centered_weighted_radius()` returns the outward sum only for a weighted-ball owner; component-disk ownership keeps its previous `None` result and per-mode enclosure semantics. The same displacement calculation is used once for both APIs. No sum-of-weights-equals-one assumption is added.

The compatibility bridge adds `3*n+1` charged primitives to component-owner queries compared with the historical research implementation: square/multiply/add per mode, then square root. This was already work performed by the current production displacement API. The imported three-mode A01 test now accounts for `110 + 3*3 + 1`, and checks the outward triangle sum and both public queries. Both A01 tests and all five original packet-curve-output tests passed.

### PR101: retain distinct immutable targets

The current `BoundPr101Problem` is a frozen scalar witness with a local segment certificate. The imported `Pr101Model` accepts immutable finite parameters and supplies an opt-in transactional continuous verified driver. They are different APIs with different scope. Keeping the frozen module unchanged and moving the imported implementation to `pr101_verified.rs` preserves both contracts. Its public error type is named `Pr101VerifiedError`; the original `Pr101Error` remains unchanged.

This is a namespace/ownership separation, not a mathematical generalization of the frozen witness. The imported generic provider references the parameterized model's error type. Nine frozen-provider tests, seven parameterized-driver tests and one independent PR101 test passed together with the generic A05 contracts.

### Dense output: preserve estimate controls and add verified ownership

`WFilteredReport` and `WFilteredEnforce` remain available with their original estimate semantics and explicit-Jacobian restrictions. `VerifiedResidual` is added as an explicit request that refuses the arbitrary-callback entry point; the immutable `VerifiedContinuousProblem` API is the route to continuous residual verification. There is no automatic promotion from an estimate to a certificate.

The imported interval evaluation expands the stored quartic before range evaluation:

\[
q(\theta)=a_0+a_1\theta+a_2\theta^2+a_3\theta^3+a_4\theta^4,
\]
\[
a_0=y_0,\quad a_1=y_1-y_0+d_0,\quad a_2=d_1-d_0,
\quad a_3=d_2-d_1,\quad a_4=-d_2.
\]

This is the same represented polynomial as the endpoint blend. Outward coefficients and Horner evaluation enclose it while avoiding the artificial `O(|y0|)` interval dependency width of separately bounding both endpoint blend terms. The published proof is reused. The current O01 error-propagation fix was combined separately by the dense-output agent; its final tests and independent review remain that workstream's evidence.

## Coverage and execution

| Scope | Integration evidence |
|---|---|
| A01 packet owner | Both radius APIs retained; two A01 and five existing curve-output tests pass |
| A02 trajectory | Verified weighted trajectory/adaptive APIs restored; five adaptive and three fixed-trajectory tests pass |
| A03 damping | Original-target variation bound and verified damped driver restored; eight tests pass |
| A04 Leja | Borrowed admitted enclosure token restored; seven tests and one compile-fail doctest pass |
| A05 generic residual | Immutable provider driver restored; seven tests pass; both PR101 APIs retained |
| A06 collective direct | Structured stage certificates/workspace restored; three A06 and one allocation test pass |
| A07 benchmark | Published full-cost harness and negative result retained; no new timing promotion |
| A08 polynomial | Same-budget guarded Leja/Laguerre/Chebyshev router restored; six tests and one compile-fail doctest pass |
| A09 homotopy | Actual-K and nonlinear window APIs restored; three and seven tests pass |
| A10 Bernstein | Frozen-owner residual refinement restored; four tests pass |
| A11 Gray–Scott | Bound verified provider and fused callback changes restored; seven tests pass |
| A12 review | Root's independent integration review and final source-bound matrix required |

All commands, exit codes, log hashes and exact arguments are in `PORT.json`. These tests are acceptance/correctness checks, not a fresh wall-clock speed campaign. Previous proof or benchmark campaigns were not rerun merely because their documents were imported.

Two extra main-worktree commands failed before test execution with Rust `E0463` (“can't find crate” for `rodas5p_integrators` or `rodas5p_krylov`), followed by inference errors. Their raw logs are `packet_compatibility.log` and `homotopy_unit.log`. Shared artifacts from different worktree paths are a plausible environment cause; this diagnosis is not proven by the failure alone. No workaround changed source semantics or dependencies. Root will perform the final matrix from the main worktree with a fresh target directory; until that succeeds, the affected compatibility obligations remain open. The original 85 native passes and two doctest passes remain separately recorded.

The failed command targets are `production_audit_packet_center_displacement`, `packet_trajectory_contracts` and `nonlinear_homotopy::tests`. They are the exact remaining port-specific retry scope; the root may combine them with its final integration gate. The author of this port does not independently approve its own A12 resolution.
