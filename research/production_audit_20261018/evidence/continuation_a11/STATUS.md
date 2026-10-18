# A11 Gray-Scott logarithmic norm continuation

The implementation adds `GrayScottLogNorm`, returned together with the generated problem by `gray_scott_2d_with_log_norm`. The existing `gray_scott_2d` API is preserved and delegates to the same builder. The descriptor's grid size is private; its method accepts one u and v interval per cell and revalidates every public interval value.

For local reaction block

`J_R(u,v) = [[-v^2-F, -2uv], [v^2, 2uv-(F+k)]]`,

the symmetric-part maximum eigenvalue is

`(a+d)/2 + sqrt(((a-d)/2)^2 + ((b+c)/2)^2)`.

The implementation encloses `a`, `d`, `b+c` using directed interval arithmetic and outward-bounds the square-root expression using the interval magnitudes. It returns the maximum block upper bound. The generated periodic diffusion blocks are symmetric negative semidefinite, so their Euclidean logarithmic norm is nonpositive; by subadditivity of the matrix measure, adding them cannot increase this reaction-block upper bound. Work is linear in the number of grid cells (O(n_grid^2), also O(state dimension)).

Scope: this bounds the analytic semidiscrete Gray-Scott Jacobian on the supplied componentwise boxes. It does not construct or certify those trajectory boxes, enclose callback floating-point roundoff, or by itself certify a trajectory/global defect integral. No broader solver or production claim is made.

## Verification

- `cargo test --locked --offline -p rodas5p-integrators --test a11_gray_scott_log_norm -- --nocapture`: exit 0, 3 passed. Raw output: `a11_focused_test_final.log`.
- `cargo check --locked --offline -p rodas5p-integrators`: exit 0. Raw output: `a11_check.log`.
- `rustfmt --edition 2024 --check crates/rodas5p-integrators/src/gray_scott.rs crates/rodas5p-integrators/tests/a11_gray_scott_log_norm.rs`: exit 0.
- `git diff --check`: exit 0.
- Tests include the generic `J=[[-1,100],[0,-1]]` max-diagonal counterexample, interval tube enclosure samples, malformed/reversed/nonfinite bounds, shape mismatch, and arithmetic overflow refusal.

Base HEAD and current source hashes are recorded in `base_head.txt` and `source_sha256.txt`. Shared checkout contains other workers' edits; they were preserved and not included in this node's scope.
