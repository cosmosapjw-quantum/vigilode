# RUNTIME_INTERRUPTION_RECOVERY — 2026-08-07

## Classification

The interrupted allocation/kernel-audit work is not treated as a normal continuous run.

- Rust parity v0.1.0-alpha1 source and history: **DURABLE_VERIFIED** from the complete Git bundle.
- Uncommitted R10-A changes reported before the runtime reset: **TRANSCRIPT_ONLY** and therefore not inherited as completed work.
- Current R10-A branch: **RECONSTRUCTED** from the verified baseline and will be reimplemented test-first.

## Durable baseline

- baseline commit: `cbb29dfb32ef4a358f5e77f4320190534543fa36`
- branch: `rust-adaptive-v0.2`
- Git bundle SHA-256: `1b275adda5c707540354ddbb6f147b365f7b491d1612a75153c08dfe923616ea`
- Rust toolchain archive SHA-256: `294b3d81fa72e62581276290c60c81eb8b58498d333d422ca1dfc432877d0c40`
- offline crate bundle SHA-256: `dc2b1ba76059b56042852975845578e51df67ebfdd59b29305106ad71b578be3`
- rustc: `rustc 1.94.1 (e408947bf 2026-03-25)`
- cargo: `cargo 1.94.1 (29ea6fb6a 2026-03-24)`

## Recovery rule

No pre-reset test, benchmark, or uncommitted file is used as completion evidence. Every R10-A contract is recreated from a failing test, rerun on the reconstructed branch, and committed before proceeding to R10-B.
