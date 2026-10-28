# p0-stats — N01 implementation receipt

[D] Implemented under the 2026-10-21 production re-audit. See `research/production_reaudit_20261021/NEXT_N01_IMPLEMENTATION.md` and its JSON sibling for exact scope, migration notes and validation receipts.

- Facade-owned recursive Stats/route/work records preserve `vigilode.stats.v4` fields, order and serde representation; all 61 current WorkCounters fields are covered (the wave4 plan's 62 is stale).
- Public capability query and solve reuse typed unsupported predicates. The 384-case matrix exercises six route families and seven refusal variants without query callback execution.
- O-09's typed interpolation failure and transactional event handling were already implemented and verified in the preceding facade audit. Reused as `FailureReason::EventInterpolationFailed`; no duplicate variant or unrelated rerun.
- Native validation: 85 tests, 2 doc tests, warning-denied rustdoc and four actual examples PASS. Successful receipts have unchanged source manifests. The pre-mirror golden remains unchanged. A real two-test JSON regression exposed omitted zero-counter serde attributes; fixed and retained as RED evidence. Root owns final integrated clippy/bench verification.

[D] Final independent source re-review PASS after the native tests exposed the initial mirror zero-suppression omission; initial review and failure logs remain available.
