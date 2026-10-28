# Independent review: fair-ab dense output error conversion

## Scope and identity

Reviewed the bounded exhaustive-match repair in `crates/rodas5p-fair-ab/src/adaptive_global_error.rs` at SHA-256 `84b812999916b9014f686f71cd20bbb1c87cc70e8e254799a2c02cf45bf284fd`, against receipt `receipt.json` (SHA-256 `a73d9ca7905a2b5535fba1d126b2afc915fe5fd6276e5d4ca550788924f53dc8`). The receipt binds the earlier workspace compile failure, focused test, clippy, formatting, and diff-check logs.

## Review

The three newly introduced dense-output error variants are mapped explicitly to `CoreError::InvalidInput`, retaining each original display message. The existing `CoreError` passthrough is preserved, and the match has no catch-all arm, so future variants remain compiler-visible. The focused regression checks all three variants, error class, and exact message. Receipt hashes and exit codes are internally consistent.

## Decision

**SCOPED PASS** for compile integration and error preservation only. This is a mechanical adapter repair; it establishes no numerical enclosure, trajectory accuracy, or scientific promotion. Independent review is based on the recorded source identity and receipt; no runtime model identity is asserted.
