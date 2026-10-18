# A01 continuation slice status

Base checkout identity: branch `audit/production-readiness-20261018`, HEAD `de990927d0f9197f0e37fcaeccee33bea6c7b8af` (shared tree; other DAG work is concurrently dirty). This slice did not commit or push.

## Implemented in this slice

- Existing diagnostic certificate now computes its tail scale from the stored `S = sum(weights)` and actual `max(abs(xi))`, and scales its local Lipschitz factor by stored S. This is still ordinary f64 arithmetic, so it is only a diagnostic estimate; it is not outward.
- Work counters include identity snapshot/comparison items, certificate visits, and reconstruction polynomial terms.
- Regression checks actual stored S/support and work counters.
- 100-digit `mpmath` exact-linear two-mode reference values are stored in `multimode_mpmath_oracle.txt`; regression compares reconstructed centers at tau 0.25, 0.5, and 1.0 to that oracle and confirms errors are within the legacy diagnostic per-mode estimate. The test explicitly asserts `roundoff_enclosed == false`; this is numerical evidence only, not certified inclusion.
- NaN and infinity coefficient inputs produce typed `PacketError::NonFinite`; a finite overflowed candidate norm is refused likewise. A subnormal coefficient remains explicitly diagnostic (`roundoff_enclosed == false`) and is not promoted.

## Commands and outcomes

- `cargo test --locked --offline -p rodas5p-integrators --test production_audit_packet -- --nocapture`: exit 0, 14/14 tests.
- `cargo check --locked --offline -p rodas5p-integrators --all-targets`: exit 0.
- `rustfmt --edition 2021` on the owned Rust source and test files: exit 0.
- `git diff --check`: exit 0.

Raw outputs are adjacent `.log` files. The continued test run experienced a Cargo artifact-lock wait because other DAG work was running; it completed successfully.

## A01 remains HOLD: exact missing obligations

1. `compile_packet` still creates `mu`, `nu`, `free`, and `kernel` with ordinary scalar/complex arithmetic. The available `directed::Interval` scalar operations can enclose real scalar arithmetic, but this packet path has no interval-complex moment/coefficient pipeline and no candidate generated from those coefficient enclosures.
2. `generate_candidate`, `force`, `convolve`, and the full residual use rounded complex arithmetic and rounded causal beta weights. No outward coefficient residual enclosure exists, so cancellation, additive underflow, and product cross terms are absent from the bound.
3. The analytic tail and nonlinear defect formula has not been derived as an outward bound for this concrete generated candidate and actual stored spectral kernel. Merely swapping `width` for `width*max|xi|` and multiplying by S does not prove all formula assumptions or its floating-point evaluation.
4. `reconstruction_polynomials`, `reconstruct_modes`, and `evaluate_mode_at` use rounded Horner/products. The returned center is not coupled to an outward radius for its own reconstruction arithmetic.
5. The certificate bounds the candidate mean P. T1 requires the actual reconstructed trajectory curve Y, its continuous residual, and its reconstructed mean `M_Y`; no interval polynomial identity/bound currently connects P to `M_Y` over the full interval. Thus T1 cannot safely consume the present candidate bound.
6. No verified `(center, outward_radius, source_identity, work)` API is present. `roundoff_enclosed` remains false on every successful certificate.

No A02 work was started. A01 must stay HOLD until the interval pipeline and the missing P-to-Y/residual/tail derivation are implemented and independently reviewed.

## Independent-review finding and bounded repair

A fresh reviewer supplied a counterexample for S slightly below one: with one mode, `xi=1`, `w=1-5e-13`, `z=0.5`, `width=1`, `chi=kappa=0`, `h=1e-8`, and order/degree zero, the pre-repair result was about `5e-9` while the reported mode bound was `4.9999999999974995e-9`. The unweighted microscopic free-mode truncation error had incorrectly been scaled by S. The failing first-run output is preserved in `review_witness_initial_failure.log` (exit 101).

The repair separates the diagnostic terms:

- bare support tail `B = (width * max|xi| * h)^(p+1)/(p+1)!`;
- weighted kernel tail `S*B`;
- mean free tail `A0*B`, where `A0 = sum(w_j |z_j|)`;
- per-mode free/nonlinear tail `B*(|z_j| + h*|chi|/(p+2))`;
- mean Lipschitz factor `S*L_F`, while microscopic transport uses `L_F = 3|chi|`.

The certificate now exposes `bare_tail`, `kernel_tail`, `free_tail`, and `microscopic_lipschitz_l`; legacy `tail` remains an alias for weighted kernel tail. The trajectory consumer uses `microscopic_lipschitz_l` for inherited mean-radius-to-mode transport while retaining `lipschitz_l=S*L_F` for the mean denominator. An additional S<1 trajectory regression confirms this split. The original witness passes after repair. These formulas still use ordinary f64 and do not change `roundoff_enclosed=false`.

Cost-counter review: the former `5N+7(p+1)` identity count was indeed only a proxy. It was replaced by payload accounting for the exact binary64 arrays/scalars stored in a model/identity (`4N+5` model payload and `1+7(p+1)` identity scalar payload), including temporary and final identity snapshots. Packet model clone, output candidate/identity clones, polynomial restriction multiplications, returned mode values, and f64 identity payload comparisons now have named counters. Integer metadata/length checks and candidate free-vector clone/setup, `CausalWeights::compile`, allocator costs, and a complete arithmetic/runtime cost model remain unaccounted. Counter totals are documented as heterogeneous aggregates, not FLOPs or complete cost.

Final repair checks:

- `cargo test --locked --offline -p rodas5p-integrators --test production_audit_packet -- --nocapture`: exit 0, 16/16.
- `cargo check --locked --offline -p rodas5p-integrators --all-targets`: exit 0.
- `rustfmt --edition 2021` on touched Rust files and `git diff --check`: exit 0.

Raw logs: `review_witness_initial_failure.log`, `review_witness_repaired.log`, `production_audit_packet_lipschitz_final.log`, `cargo_check_lipschitz_final.log`, and `diff_check_lipschitz_final.log`.
