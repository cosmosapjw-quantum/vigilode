# Phase 9 — Independent base-to-head diff review

Date: 2026-08-07
Frozen base: `rust-pre-homotopy-v0.2.0-alpha1` (`d208e7b0c45a6f67cc1fddecc5e694d675850277`)
Code reviewed after fixes: `adeff4a91c6c21994cf3db08f4a480ce315eb488`

## Review method

The review was performed from a detached linked worktree rather than the implementation worktree. It inspected the complete base-to-head source diff, dependency/fixture changes, public API bypasses, failure semantics, numerical definitions, test evidence, and protected source boundaries.

## Findings

### Critical

None.

### Important — fixed before closeout

#### I-001: validated parameters could be bypassed

`PartialCouplingParameters` initially exposed public `theta` and `lambda` fields. External callers could construct values outside `[0,1]` without calling `new`, weakening the API contract even though non-finite values would usually fail later during matrix construction.

Resolution:

- fields made private;
- read-only accessors added;
- a `compile_fail` doctest was first observed failing while the fields were public, then passing after the fix.

Evidence:

- `receipts/REVIEW_PARAMETER_BYPASS_RED.log`
- `receipts/REVIEW_PARAMETER_BYPASS_GREEN.log`

#### I-002: non-finite certificate could be returned as success

`wrms` deliberately returns infinity when finite error divided by an extremely small positive scale overflows. The first certificate implementation propagated that infinity in an `Ok(AffineOutputCertificate)`, contradicting the scientific contract's explicit-failure rule.

Resolution:

- added a failing regression with the smallest positive subnormal scales;
- certificate fields are now checked collectively for finiteness;
- non-finite certificate values return `CoreError::NonFinite`.

Evidence:

- `receipts/REVIEW_NONFINITE_CERTIFICATE_RED.log`
- `receipts/REVIEW_NONFINITE_CERTIFICATE_GREEN.log`

### Minor / accepted limitations

#### M-001: binary64 structural nilpotency is approximate for the printed tableau

The source coefficient transform leaves roundoff-scale diagonal/upper defects. The constructor validates these against a binary64-scaled tolerance but retains the original values, so the `q=7` result is accurate to roughly machine precision rather than symbolically exact. This is intentional: the research oracle does not silently rewrite the protected coefficient snapshot. The CLI reports `||L^8||_F` and direct-solve disagreement so the defect is visible.

#### M-002: deterministic JSON is guaranteed only under the locked environment

The current reference right-hand side uses standard floating transcendental evaluations. Repeated runs in the locked Rust/Linux environment are byte-identical, but cross-libm bit identity is not claimed. The next experiment branch must store environment/toolchain identity with results and compare numerical values with tolerances across platforms.

#### M-003: explicit dense LU is intentionally not a production path

The oracle forms full block matrices and refactorizes path operators. This is acceptable for reference/kill-screen use and must not be used as evidence for runtime economics of a batched matrix-free homotopy implementation.

## Protected-boundary audit

No changes occurred in:

- official coefficient fixture;
- `sequential.rs`;
- `sabr.rs`;
- `block.rs`;
- Krylov solver sources;
- Fair A/B implementation;
- workspace dependencies or lockfile.

New production behavior is confined to `rodas5p-integrators::homotopy` and the `homotopy-design-check` CLI route.

## Error-swallowing audit

No `unwrap`, `expect`, `panic!`, `todo!`, or `unimplemented!` appears in the new production homotopy module. Invalid dimensions, ranges, finite values, truncation depth, and factorization results propagate typed errors. No automatic fallback is introduced.

## Scientific audit

- sign convention `W=M-h gamma J`: PASS;
- target operator `D-C=I kron M-h beta kron J`: PASS;
- endpoint `lambda=1`: PASS;
- affine FLRH `theta=1`: PASS;
- finite inverse ordering `sum Q^m D^{-1}`: PASS;
- RODAS output contraction with `b`: PASS;
- nonnormality not hidden by determinant-one claim: PASS;
- nonlinear/stiff/DAE/performance claims: correctly absent.

## Final review disposition

**PASS WITH EXPERIMENTAL-SCOPE HOLD.**

The reference API and deterministic screens are suitable to seed the next numerical-experiment branch. No actual homotopy solver, fifth-order preservation claim, stiff-decay claim, or speedup claim is approved by this review.
