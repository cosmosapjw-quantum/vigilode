## Task 3: P0-facade — protected-tier completeness of the facade (node id `p0-facade`)

Plan §3 P0 tier; wave-1 rulings on mass, dense output and partial solutions.

- Route `.mass(M)` and `dense_output = true` in `crates/vigilode` to the protected sequential dense driver
  `integrate_adaptive_dense_observed_with_dense_error_control` (`dense_output_v2.rs`) with
  `DenseErrorControl::WFilteredEnforce` by default for dense output and an `Options::interior_error`
  enum {WFilteredEnforce (default), Enforce, Off} exposed; a mass matrix requires an explicit or FD
  Jacobian (refuse JVP-only with `Unsupported`).
- `Solution::at(t)` evaluates the dense extension when dense output was requested; otherwise returns the
  stored sample or `None`.
- Partial solution after failure: extend the fast dense, banded and matrix-free drivers (`rodas5p_fast.rs`,
  `rodas5p_matrix_free_fast.rs` result types) to report the last accepted `(t, y)` and accepted samples on
  failure without changing any success-path arithmetic; the facade returns them. Test with a fatal RHS error
  after several accepted steps and a fatal JVP error inside GMRES (the wave-1 untested path).
- `SolveError`/`Status` reasons: replace the `StepSizeTooSmall` catch-all with the driver's actual failure
  kind (linear solve, non-finite, step size, attempts budget).
- Bit-identity tests against the direct driver calls for each new route.

