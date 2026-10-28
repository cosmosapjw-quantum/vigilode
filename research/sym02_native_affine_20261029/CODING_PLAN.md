# C1–C5 native implementation plan

C1 baseline: PR101 bc5ddbe42b24b1a6364bf653c3e2d6d938ca4087. Relevant native blobs match PR100; cumulative PR101 allocation and authority work is preserved. The new opt-in API does not alter solver selection or tolerances. Initial error is the explicit global caller premise.

C2 equation-to-code: AffineScalarModel validates constant interval lambda,b; RepresentedAffineSegment holds endpoint/H/stage bits; AffineEventCertificate reconstructs five power coefficients, physical-clock residual and Bernstein hull; query returns a separate actual-center rounding enclosure. FastDenseSegments native adapter owns accepted provenance and rechecks packet bits. AffineEventCarry enforces adjacency and binding consistency.

C3 numerical design: existing directed::Interval arithmetic, positive physical width, all-time degree4 Bernstein residual, two-envelope scalar Dini radius, strict derivative/event interval tests. Private immutable certificate data. Finite overflow returns typed refusal. Safe per-call environment probes preserve forbid(unsafe_code). General DOPRI piecewise curves and full nonlinear BASS30 providers are separate targets.

C4 execution allocation: local Rust1.94.1 from supplied archive, locked Cargo dependencies acquired once and then --offline; one Cargo job with two compile workers and a shared file lock. Compilation wall cap240s; focused numerical tests wall90s. New bounded native fixture: only x'=-x over[0,0.125], <=128 attempts; a dimension2 refusal fixture is separately labeled. No legacy campaign replay or performance timing.

C5 files: crates/rodas5p-integrators/src/affine_event_certificate.rs; src/lib.rs module registration; src/rodas5p_fast_dense.rs opt-in adapter and quartic comment correction; tests/affine_event_certificate_contracts.rs negative/scientific/native controls. Test-only unsafe FP manipulation is isolated in integration tests and restored by RAII; production remains safe-only.

Verification commands use cargo test --locked --offline -p rodas5p-integrators --test affine_event_certificate_contracts -- --test-threads=1. Additional exact Fraction checks read native output bits independently. Relevant existing fast-dense contracts are reused as targeted regression, not a claim of all release suites.

