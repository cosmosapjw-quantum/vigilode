# A05 bounded repair authorization (cumulative repair 2)

Read-only independent reviewer approved one targeted repair on the current A05 implementation. The prior repair episode and its findings remain preserved; this is cumulative repair 2, with no accounting reset.

Frozen base: HEAD `d087dab9a8dacff0b195566c4a649273acdb476c`; execution source hashes are in the sibling `EXECUTION.json`.

Allowed edits: work accounting, accounting contract comments and relevant unit tests in `src/bound_pr101.rs`; the newly added scalar quartic accessor and related tests in `src/dense_output_v2.rs`; exact-counter regression in `tests/production_audit_pr101_provider.rs`; new evidence only.

Forbidden edits: model, formula, arithmetic evaluation order, tolerance, public API, Fraction oracle, prior ledger. A successful repair may admit only the sealed PR101 local provider slice; full A05 trajectory/global output/speed remains HOLD.

Acceptance: individually charge each fallible interval/outward arithmetic call immediately before it, excluding internals; define `mag` accounting; exact successful and overflow counter deltas; preserve output/certificate and accumulated attempted work on refusal; run prior eight regressions, relevant unit tests, both allocator positive-control/warmed-zero checks, and Fraction oracle; retain first failure and final source hashes; obtain independent re-review. Any additional defect returns to review and is not an automatic third repair.
