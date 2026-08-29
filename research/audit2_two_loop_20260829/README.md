# VigilODE audit2: two executed research loops

Read `MATH_RESEARCH.md`, `CODING_RESEARCH.md`, and `RESULTS.json`; then use `CODEX_HANDOFF.md` and `handoff.json` for the genuinely local remainder. Start from the research branch, not old K0 bootstrap packages.

1. The second audit misidentifies the existing direct trajectory discrepancy as a scalar error difference and mislabels the all54 dimension coverage.
2. Policy sensitivity is not independent accuracy. The additive API requires an external observable budget; missing budget does not pass.
3. Official quartic continuous output has exact-start local defect O(h^5), consistent with global order5 in the tested nonstiff setting. Stiff limits and rounded-coefficient floors are separate.
4. The full nonlinear stage-block correction has an exact common-W block-forward implementation. The test-only Rust candidate avoids8 stage Jacobian builds while matching the full-target correction in12 cases. No production or timing claim.

Historical evidence is unchanged. `records.zip`, when present, is a convenience archive of executed data/logs/state, not an execution prerequisite. Reproduction scripts are directly in `tools/`; source tests are directly in the crates. No new dependency, schema, admission gate, or required manifest is introduced.
