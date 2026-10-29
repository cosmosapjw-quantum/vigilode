# A03 forcing-moment consumer independent review

## Frozen identity

- `crates/rodas5p-core/src/damped_moments.rs`: `3d78f6768af5513d9eeb12ad556b1a72d613b8e3521345561a691db1947a5950`
- `crates/rodas5p-core/tests/damped_forcing_moment_consumer.rs`: `bfc59ee8816f4046c6ff0a0f9b07ce617913e91f003f5832375339c72270ac98`
- Receipt `a03_forcing_moment_receipt.json`: `64246367adb1d13cba2335cd042fdc5d3e29c1051107f74f19e8fbe40dcfa21c`

## Review and decision

The frozen substitution is implemented in the correct direction:

\[
Q_{r,d}(a,h)=\int_0^h(h-s)^r s^d e^{-as}\,ds=I_{d,r}(a,h),
\]

so the wrapper calls `compute_damped_moment(a, h, d, r)`. The exact binary64 `a` and `h` are passed to the existing kernel. It validates finite `a>=0`, finite `h>0`, supported orders, and a finite nonnegative absolute width limit. The returned interval width is bounded with `sub_up`; typed refusal and coarse attempted validation/kernel/width work are retained on failure.

The final eight consumer tests cover unequal indices, zero damping, a subnormal rate-time product, the `0.1*10.0` branch-boundary enclosure with frozen endpoint bits, large-rate width refusal, invalid/nonfinite inputs, and overflow. The corrected receipt binds current source, test, and all command/log hashes, including the Clippy log.

Decision: **SCOPED PASS for the scalar forcing-moment consumer only.** No packet route, `exp(-2κt)` nonlinear solver, carrier/output convention, or authority changed. Full A03 remains HOLD.
