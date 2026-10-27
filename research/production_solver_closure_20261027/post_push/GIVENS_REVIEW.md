# PR92 post-push Givens CI review

Date: 2026-10-28 (Asia/Seoul); source CI logs are dated 2026-10-27 UTC. Review is read-only with respect to the repository. No native build, test, benchmark, numerical retuning, or Rust edit was performed for this review. This temporary note is outside the frozen research node and does not register a new research result.

## Decision

The reported failure demonstrates that an environment-independent assertion of a historical **failure to converge within a fixed budget** is not currently reproducible. It does **not**, by itself, demonstrate false convergence, nor justify promoting the Givens candidate. Keep the failing CI status visible. Do not replace the expected failure with success, remove the assertion, increase the budget, change tolerances, or change numerical constants to obtain a green check.

There is no evidence supporting an immediate numerical solver fix. The justified next change is a narrowly scoped diagnostic/test-provenance task associated with the existing Krylov follow-up, followed by a reviewed decision about which fixture assertions are portable mathematical contracts and which are environment-scoped historical observations. No fourth adapter numerical correction is implicated.

## Observed failure and scope of inspection

The decoded Production CI job `122293695979` and A2/A3 job `122293694635` logs were directly inspected. Both reached `crates/rodas5p-krylov/tests/givens_production_differential_contracts.rs:122`; `candidate_and_production_outcomes_are_pinned_on_the_e05_nonnormal_jacobi_rows` observed `gmres-givens-candidate` at RHS scale `1e14` as `Ok(3640)`, hence `is_ok() == true`, while the fixture expected false. The Production CI log records this at lines 1201–1204 and the A2/A3 log at lines 486–489. Logs were already downloaded by the parent; this continuation directly read their decoded local copies. No executable or solution vector from those jobs was available for independent residual verification.

Inspected current HEAD: `78bb8775bf553ad2b514575a0ef4a025446f5d28`. Comparison baseline: PR91 `206298b4068d923bb47c7160b111b7439098660c`. The current working tree has metadata-only registration edits owned by the parent/another agent; all named code identities below were compared as immutable Git objects. The historical commit named by the test, `b3e8165`, is not available as a local Git object; no source equivalence with that older commit is claimed.

### Byte identity to PR91

Each file below has the same SHA-256 at current HEAD and PR91:

| File | SHA-256 |
| --- | --- |
| `crates/rodas5p-krylov/tests/givens_production_differential_contracts.rs` | `1ea9e9c1e90bb3208bdc914b39a81b8d1474818818ee3c6bb82021ffe05ac007` |
| `crates/rodas5p-krylov/src/gmres_givens.rs` | `b8afd9796f5663951413a1ced085be627f58e2a29ceb377a63759faa6ae6b130` |
| `crates/rodas5p-krylov/src/gmres.rs` | `254f421a3e6325e5492c71597053bb8787a5e3153d5e1563f0b745b73889905e` |
| `crates/rodas5p-krylov/src/common.rs` | `b8a03ade17035487c9e92734d3e4c31dd6b94b43f0d16809fb5178a5e26238a8` |
| `crates/rodas5p-core/src/operator.rs` | `989a2d3953c1f6b83138cb93a51072aa829e311c00bc76ee488ea85d125bff0a` |
| `crates/rodas5p-core/src/matrix.rs` | `40a3f19200d2dda72ddaf6d14c96f726fc978f4c83f687ca2d0abcaa2a49760d` |
| `crates/rodas5p-core/src/norms.rs` | `d9855786d8ab94e1def28e3525fa40e5a44914a3ecf5f1d318e41421f8cfd604` |
| `Cargo.toml` | `716ff1b052b1333e0ee5867f266426f6ecbec11578444f2e2b9c02baa03b037e` |
| `Cargo.lock` | `efb379071fa42c2b6bdbe969163ac2f41cf318c262c5c67d0910450298333351` |
| `rust-toolchain.toml` | `f53198ae4fdecfd87da36fe431c771b54c51e975d01c0e99f653bc14d5d48211` |

These identities rule out a textual PR92 change in the named fixture, kernels, common residual routines, dense operator/matrix/norm implementation, dependency lock, or workspace profiles. They do not establish binary or complete execution-environment identity, and do not independently show that PR91 passes the current CI environment.

## What the historical evidence actually establishes

`research/adversarial_audit_20260927/experiments/E-05/README.md` explicitly says the binary used the **measurement profile** and `RAYON_NUM_THREADS=1`. The two currently failing workflows use Rust 1.94.1 on `ubuntu-latest` and `cargo test` without an explicit profile, hence the default test/dev compilation profile. The workspace measurement profile inherits release optimization, overrides LTO to false, and uses 16 codegen units. Profile/environment variation is therefore a concrete uncontrolled difference, although its causal role has not been isolated.

The archived E-05 raw rows for `n_scale == 1`, Jacobi, and the two relevant solvers are:

| RHS scale | Production GMRES | Givens candidate |
| --- | --- | --- |
| `1e-14` | Success, 3200 iterations; recomputed relative residual `7.613126964732989e-11` | Failure: exhausted 4000 Arnoldi vectors |
| `1` | Failure: exhausted 4000 Arnoldi vectors | Success, 3739 iterations; recomputed relative residual `9.724576419792205e-11` |
| `1e14` | Success, 3440 iterations; recomputed relative residual `9.425953905406227e-11` | Failure: exhausted 4000 Arnoldi vectors |

Source: `experiments/E-05/e05_raw.json` within that audit. Failure rows have null recomputed residuals and do not provide a lower-bound certificate showing that every permissible floating-point execution must exceed the residual tolerance. The E-05 README reports no false-convergence row among its main results; its FAIL status concerns separate stale-reuse criteria. It cannot be read as a theorem that this candidate must fail at scale `1e14`.

The current fixture recreates `A = Q(D + N)Q^T` with `n = 256`, Householder reflectors, an LCG seed, and diagonal entries computed with `powf` and `log10`; it uses restart 40, maximum 4000 Arnoldi vectors, relative tolerance `1e-10`, and zero absolute tolerance. It anchors three entries of `b_unit`, but does not pin all matrix or Jacobi-diagonal bits. The E-05 README lists `e05_matrices.json`; no such file is tracked in the current repository, and none is in that experiment directory. Therefore the historical matrix-bit identity has not been established by the available serialized fixture.

`DenseMatrix::matmul` is a manual triple loop, and its matvec is a manual row-dot reduction. The matrix generator is not a BLAS call. Attributing the failure to BLAS threading would be unsupported. Differences in elementary-function evaluation, compiler/profile behavior, later floating-point arithmetic, or library implementation can be investigated, but none has been isolated here. The historical sensitivity to rescaling, together with restart and the finite budget, makes a rounding-sensitive outcome plausible; it remains a diagnosis hypothesis rather than an established root cause.

## Mathematical meaning of success and of the fixture

For the unweighted solve used here, let `r_hat` be the binary64 residual recomputed by the actual operator path, and let `tau = rtol * safe_l2(b)`. The current `solve_gmres_givens_with_workspace_and_residual_scale` unconditionally recomputes `b - A*x` after its solve loop (`gmres_givens.rs`, final return block), rejects a nonfinite residual norm, and returns an error if the norm exceeds `tau`. Only after that check can it return `Ok(LinearSolveReport { converged: true, ... })`. In-cycle projected residual triggers also lead to an explicit true-residual certification before accepting a candidate.

Consequently, under the inspected implementation and a deterministic correctly functioning operator, every returned `Ok` satisfies the **implemented floating-point residual criterion**. This source-level implication explains why `Ok(3640)` alone is not evidence of false convergence. It is not an interval-certified statement about the exact real residual, and it is not independent verification of the actual CI result.

The test first asserts `result.is_ok() == expected_ok`, and checks the independently recomputed residual only afterward. The directly observed CI panic therefore prevents that later external check for the offending candidate. The log does not expose `x`, the reported residual, the external residual, or the margin to `tau`; this missing evidence should be obtained before making a numerical correctness claim.

The historical assertion is stronger and different: it requires a particular algorithm to fail on one generated matrix in a particular number of permitted steps. A fixed historical trace does not establish this claim for other arithmetic environments. Ideal exact-arithmetic relative-residual GMRES has a scaling relationship under multiplication of the RHS by a positive scalar; floating-point operation order, finite-budget stopping, and near-threshold decisions do not inherit a universal bitwise outcome guarantee from it. The archived success/failure table is valuable evidence about that run, but a portable failure contract needs a deliberately stable failure mechanism or a valid certificate, not only a former budget exhaustion.

No `solve_gmres_givens` reference was found in `crates/rodas5p-integrators/src`, `crates/vigilode/src`, or `crates/vigilode-coupled/src`. References in the Krylov source comprise exports, definitions/wrappers, and its internal test. This supports the existing classification of Givens as an exported research candidate rather than the inspected production dispatch route. It does not establish absence of every possible external caller.

## Precise safe follow-up

Use an existing registered Krylov follow-up; do not create another frozen research node merely to absorb this CI failure. First make a diagnostic-only change that retains both the historical outcome assertion and all residual requirements. Report or verify each returned residual **before** the historical-outcome assertion, so a failure records whether the returned solution met the independent criterion. Emit complete failure information, including actual outcome/error, iterations, threshold, reported residual, independently recomputed residual, and its signed margin. Do not loosen tolerances or turn the current assertion into an ignored test.

Then perform a bounded reproduction matrix with PR91 and PR92, default test/dev and measurement profiles, using the same pinned toolchain and environment. Record the complete generated `A` and `b` bit hashes, Jacobi diagonal bit hash, compiler target/options, dependency lock, CPU, libc, operating-system image, and executable hash. A second comparison must consume one serialized matrix/RHS fixture across profiles, separating generator differences from solve-path differences. Preserve all failures and successes. There is no reason to rerun unrelated adapter or scalar/Fraction oracles for this isolation task.

For every returned solution in the affected row, verify a higher-precision or outward-bounded residual using the serialized binary64 inputs. Distinguish the external binary64 safety check from the stronger exact-input residual claim. If a failure or success change remains, trace restart-boundary residual margins and the specific acceptance/breakdown branch, keeping all numerical settings fixed. Locate a stable reproduction before considering an algorithm correction.

Acceptance for that diagnostic follow-up requires: (1) the historical table remains preserved with its original source/environment scope; (2) every observed `Ok` is checked independently; (3) the dev/measurement and baseline/current comparison has explicit fixture and binary identities; (4) the actual cause is isolated or the remaining uncertainty is stated; and (5) CI policy is reviewed openly. If the outcome pin is shown to be environment-sensitive, a later reviewed test-contract change may separate the portable residual-safety invariant from the environment-qualified historical differential observation. It must neither claim the old failure was a theorem nor claim this new success proves production equivalence or cost superiority.

Until that work is complete, report the post-push CI failure and retain the existing restriction against unconditional Givens promotion. No numerical retuning or immediate production-runtime fix is warranted by the observed failure alone.
