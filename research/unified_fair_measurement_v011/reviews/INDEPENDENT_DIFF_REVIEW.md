# Independent diff review — unified fair measurement v0.11

Base: `f2e9b1e8883bc49c124ce5ab421c92fb58516b74`
Scope: requested-output observer, storage-bias removal, measurement profile

## Verdict

**PASS WITH EXPLICIT SCOPE LIMITS**

## Verified strengths

- One shared `OutputCollector` enforces finite, ordered, span-complete requested times.
- Fixed RODAS5P, BDF and Radau use the same output landing rule.
- Adaptive RODAS/SABR only commit observer state after accepted trials.
- Storage bytes count requested outputs only; internal and clipped steps are separate work axes.
- Global-error schema records the output policy explicitly.
- T1/T4 checksum includes the output policy and remains identical.
- CLI contract was upgraded from schema v1 to v2.
- Tests exercise nondividing fixed steps, adaptive rejection and cross-family common grids.

## Issues found and resolved

1. The CLI integration test still expected schema v1. The full workspace test caught this; it was updated to assert schema v2 and the complete output-policy metadata.
2. Strict Clippy rejected a negated partial-order comparison. It was replaced by an explicit nonpositive comparison and the output-observer tests were rerun.
3. Fat-LTO linking could not finish within repeated bounded command windows. A separately named optimized measurement profile was introduced. It does not replace the release profile and must not be compared directly with old fat-LTO absolute times.

## Residual limitations

- Canonical fixed-step grids happen to align with tested step sizes, so `output_clipped_steps=0` there. The clipping implementation is verified in a separate nondividing-step contract test.
- BDF2 falls back to BDF1 after a clipped step because this checkpoint only supports equal-step BDF2 history. Production variable-step BDF remains out of scope.
- Step clipping perturbs adaptive controllers. Dense output requires an independent validation lane before adaptive solver ranking.
- The current corpus is small and dense. No sparse, distributed, GPU or singular-mass claim is justified.

## Review conclusion

The change removes a real comparison bias without changing the protected numerical work paths. It is suitable for closing the fixed-step comparison-authority phase, but not for declaring a general stiff-solver performance winner.
