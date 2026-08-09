# PHYS–MATH–CODE AUDIT — actual common-W RHS telemetry v0.15

## Equation-to-code map

| Mathematical object | Code path |
|---|---|
| SABR nonlinear RHS | `StructuredBlockSystem::nonlinear_rhs` |
| common-\(W\) reference transform | `LuFactorization::solve_rows` in telemetry-only path |
| homotopy path-start/tangent/correction batch | `HomotopyBatchEvent` emitted by `truncated_partial_inverse_rows` |
| raw/directional SVD | `analyze_rhs_batch`, `analyze_rhs_directions` |
| subspace drift | `compare_rhs_subspaces` |
| advisory selection | `recommend_common_w_backend` |
| read-only CLI | `homotopy-rhs-telemetry` |

## Reality checks

- Standard and observed homotopy paths share the same internal implementation.
- Concrete telemetry types no longer leak into the production homotopy routine; a small observer trait is used.
- The dispatcher is explicitly disabled in the report schema.
- `explicit_jacobian_builds_in_dispatch=0` does not mean the telemetry run is JF: the reference path records 171 Jacobian builds and 189 factorizations.
- Failed paths preserve already-recorded rows.
- Full telemetry SVD cost is not attributed to solver work because this is an offline experiment. An active policy must count its diagnostic work.

## Independent review findings

### Fixed during the cycle

1. Repeated mutable observer borrowing caused a Rust lifetime error. The root cause was passing `Option<&mut dyn Observer>` by value across repeated calls. It was corrected by short reborrows through `&mut Option<&mut dyn Observer>`.
2. Drift was initially at risk of comparing unrelated consecutive batches. State is now keyed by `(phase, propagation_level)`.
3. Magnitude-dominated near-rank-one batches could be misclassified. Direction-normalized analysis and a median-cosine guard were added.
4. Hostile paths previously could abort an entire screen. Failure rows and partial telemetry are now retained.

### Remaining blockers

- Online rank estimate must be constant-size and work-counted.
- Dispatch must be transactional and true-residual certified.
- Risk labels must be replaced by online proxies.
- Block GMRES restart/deflation is not justified until actual dispatched traces show the corresponding failure.

## Verdict

PASS WITH ACTIVATION HOLD. The telemetry layer is suitable to design a dispatcher; it is not itself a dispatcher or speedup result.
