# Development Replay Audit

Two fail-closed verifier defects were exposed before the authoritative replay
passed.

1. The first verifier version required the semantic contract spelling
   `protected-sequential-matrix-free-rjf` in the runtime `committed_method`
   field. The durable v3.6 schema spells the same authority lane
   `protected-sequential-matrix-free-rodas5p`. The verifier was corrected to
   require the sealed semantic value in the contract and exact preservation of
   the durable v3.6 runtime label in v3.7. The console log is retained; that
   pre-hardened runner did not durably retain its raw shard directory.
2. The second verifier version incorrectly required v3.7
   `total_speculative_jvp_before_target` to remain equal to v3.6 after a capped
   continuation. This contradicts the contract: `S_prefix` must remain exact,
   while `S_total` records the newly bounded continuation economy. The verifier
   was corrected to enforce exact prefix-policy parity and independent causal
   reconstruction of the v3.7 total ledger. Its exact failed 30-shard directory
   is retained in the external closeout evidence archive.

Neither failure was a solver, budget, endpoint, or R-JF parity failure. The
final replay passed all frozen-policy and bounded-continuation gates. Its 30
successful raw shards are preserved losslessly in the external deterministic
closeout archive, with each member hash listed separately in the committed
verification JSON.
