# A07 reporting and campaign review

Read-only validation of the retained campaign and post-run reporting repair. No samples were rerun for the report repair.

## Identity

- Manifest SHA-256: `0502ed2f1a0dc02fc5403ebd7eae6a1f6700b88d662666f095fcf7e2f6e95d4f`
- RUN_RECORD SHA-256: `825b1104d3c2e8972a3e94a7705d0fa906dcaa70d18509ce7cdb15b118261556`
- Environment receipt SHA-256: `beb4cee7eee514086521b30ade3fe062f8e8fceb4461f694d90ef8e52821432d`
- Validator source SHA-256: `ca5e6ebbae35619fba20e335cabaecffeef2f4bbca18335d0a291959a95cfd97`
- Validated JSON SHA-256: `45f64070c9a2d4a373f0decc922440848b9c3bdc03bc3ac6b4178311e3eab621`
- Report SHA-256: `a286fb63dc4b884e1f27b41ed7369b794bbd7b5d86416a67a74f421e6b15a582`

## Review outcome

- 20 cold paired timing rows, 160 warm rows, and two allocator rows were retained; all solver rows completed successfully. The source/build/config/run/environment identities and raw rows remain bound in the manifest, RUN_RECORD and environment receipt.
- The report uses true paired medians: cold ratio `0.9195676453507838`, warm ratio `1.4194023547437313`. The validator requires the independent 101×3 reference center to match the final Radau ladder states and rejects hostile truncated or differing fixtures.
- Native max-grid WRMS is `0.33753007149457903`; Diffsol is `3.3026992500504733`, above the empirical interval upper `3.3037774057537246` gate. Validator exit 4 is the expected scientific accuracy failure, not a schema/runtime error.
- Timing authority remains `diagnostic_only`; no same-accuracy speedup is asserted. The empirical reference spread is not a rigorous enclosure.

Decision: campaign/report integrity `PASS_WITH_SCOPE`; matched-accuracy speed and production promotion `HOLD`.
