# VigilODE audit2: two executed research loops

Read `MATH_RESEARCH.md`, `CODING_RESEARCH.md`, and `RESULTS.json`; then use `CODEX_HANDOFF.md` and `handoff.json` for the genuinely local remainder. Start from the research branch, not old K0 bootstrap packages.

1. The second audit misidentifies the existing direct trajectory discrepancy as a scalar error difference and mislabels the all54 dimension coverage.
2. Policy sensitivity is not independent accuracy. The additive API requires an external observable budget and an explicit reference-uncertainty treatment; missing budget does not pass, and an estimate-only uncertainty cannot yield a categorical within/outside verdict.
3. Official quartic continuous output has exact-start local defect O(h^5), consistent with global order5 in the tested nonstiff setting. Stiff limits and rounded-coefficient floors are separate.
4. The projected research stage-block target has an exact common-W block-forward implementation behind the non-default `audit2-research` Cargo feature. `FullTargetOracle` remains the default backend; `CommonWBlockForward` requires explicit opt-in and is evaluated on the same trial stages. The unprojected production coefficients and residual are unchanged. No production or timing claim.

Historical evidence is unchanged. `records.zip`, when present, is a convenience archive of executed data/logs/state, not an execution prerequisite. Reproduction scripts are directly in `tools/`; source tests are directly in the crates. No new dependency, schema, admission gate, or required manifest is introduced.

## Continuation status and claim ceiling

One fresh-context review of the pre-repair bounded diff returned `REQUEST_CHANGES`. It confirmed the residual sign and absence of production activation, while finding that decimal coefficient leakage invalidated a literal exact strict-lower claim, uncertainty authority was implicit, failure work was erased, zero RHS could produce a 0/0 secondary ratio, and formatting needed repair. The repair addresses those findings and the final aggregate 11-test structured suite passed. No second fresh-context review of the repaired head was performed or claimed. Actual final HEAD/tree, CI, and workstream synchronization are supplied by the live Draft PR and Atlassian receipts.

The maximum authorized claim is `EXPLORATORY_NONAUTHORITATIVE`: a research diagnostic for a result-independent projected target. It does not establish unprojected exactness, a nonlinear certificate, an accuracy PASS for the historical54, timing/ranking/speedup, BDF/CVODE comparison, production readiness, holdout/freeze validity, or scientific-publication admission.

The six saved trajectories were selected by the fixed metadata rule “all six families at n=96 and rtol=1e-8,” independent of observed outcomes. This was a disclosed post-campaign extraction rule, not a preregistered holdout. The correction case matrix, numerical acceptance thresholds, and coefficient-projection tolerance were fixed before their results were observed. No numerical budget B was selected from the historical54 results.

## Executable-use and CI follow-through

The current maintenance adds a real default-solver client example and fixes
A1 experiment applicability without removing its frozen guards. It explicitly
tests the opt-in correction feature in CI. `evidence/readiness_verification.log`
is an actual 54-Rust/20-Python scoped run plus complete/partial example output
checks. Numerical source, the c894 correction, and historical evidence remain
unchanged. See `CODEX_HANDOFF.md` for direct use and the next original-target
bridge. Neither a green CI nor this narrow example is production readiness.
