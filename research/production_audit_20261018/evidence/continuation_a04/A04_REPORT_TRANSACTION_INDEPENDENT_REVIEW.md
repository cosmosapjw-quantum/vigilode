# A04 report transaction independent review

## Frozen identity

- `crates/rodas5p-core/src/leja_action.rs`: `e9e01e246acc25d81170b95761a8b525c30f6753a4bc9f9624cf57d5e9a043b1`
- `crates/rodas5p-core/tests/leja_total_contract.rs`: `46fa750e1b4bfa2ca7550c3548c0856899e0d85fff07b42053e24a884f43c01f`
- Receipt `a04_report_certification_receipt.json`: `93395f67e79475bbf5247f78890aa07928db8edc66a62d604dedbef6b538a74d`

## Review and decision

The report resize and metadata writes are now staged through the successful report commit. Before fallible calculation exits, attempted certification work is stored in `workspace.last_certification_ops`; primitive work remains in `WorkCounters`. The sole existing-report `certification_ops` write is in the final success commit.

Regressions cover an earlier successful report followed by invalid dimension/time and by post-work arithmetic/output refusal. They check complete report equality while confirming attempted work is retained outside the previous report. Receipt source/log hashes were verified; focused refusal cases and the 16-test Leja contract pass. No arithmetic formula, threshold, or total-error authority changed.

Decision: **SCOPED PASS for report transaction integrity.** Leja total arithmetic remains `EstimateOnly`/HOLD.
