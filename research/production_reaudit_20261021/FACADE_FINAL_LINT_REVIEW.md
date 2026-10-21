# Facade final lint review

**PASS — source/control-flow equivalence review.** `/root/runtime_restore` froze the rewrite at `crates/vigilode/src/solve.rs:686`; `/root/facade_audit` independently inspected the exact diff and surrounding validation without editing source or running tests.

The one-arm `match &problem.jacobian` became `if let Jacobian::Banded { lower, upper, .. } = &problem.jacobian`. Both evaluate the same shared-reference scrutinee once, bind the same `&usize` values, enter only for Banded, and do nothing for every other variant. The callback payload remains ignored and unmoved. Band range checking, optional declared-band equality, their early returns/error strings, and all surrounding checks retain their order. The later typed JVP-band refusal remains later. No callback, arithmetic, allocation path or owned-destructor behavior changes.

N01 source-identity addendum: its historical `solve.rs` hash is `d05233df298e206bb096b7e763c280a9248e1ecb8da8cd0c8748b66af2d9f53d`. The reviewed pre-lint index source matches that hash: **True**. The post-lint hash is `e1ad9ea19c0cd948b33fc67e67876b111262a3252ad807cc62af119e2e2331f8`. Prior N01 receipt hashes are intentionally retained; they are not rewritten to impersonate this final source. The separate root `final_04` matrix owns final-source native/clippy validation; this review does not predeclare its outcome.

Scope remains the facade validation lint fix. Broad production/global-accuracy/speed promotion stays **HOLD**. Exact source/diff hashes and review assertions are in `FACADE_FINAL_LINT_REVIEW.json`.
