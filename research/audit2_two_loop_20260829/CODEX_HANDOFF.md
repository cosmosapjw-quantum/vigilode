# Codex continuation: audit2 science/code, not old K0 bootstrap

Start from `research/audit2-output-policy-20260829` in `cosmosapjw-quantum/vigilode`. Its inspected scientific base is `codex/vigilode-scientific-validity-v2@93fe348ce36859dd5f78b31267d771ea9c054677`. The final publication receipt/PR supplies the actual delivery head. Do not run any old K0 START_CONTINUATION, merge its package, reset the user's existing worktrees, or reopen sealed holdouts.

## Already done here

Read MATH_RESEARCH.md, CODING_RESEARCH.md, data/run_summary.json and evidence/verification.json. Actual re-analysis uses all54 compact records and recomputes six full saved trajectory metrics. The independent scalar probe reconstructs the official decimal tableau, separates local/global errors and includes a linear interpolation mutant. New Rust code separates accuracy from policy sensitivity and tests a common-W block-forward correction against the real full target Jacobian. Its runtime path is test-only; historical gates and production certification are unchanged.

## Local work that genuinely remains

1. Inspect current refs/worktrees and preserve dirty/unpushed changes. Create a new isolated worktree from this research branch, not from old K0. An incidental commit SHA difference is a provenance rebind question; inspect the relevant diff. Do not force-reset or demand identical archive bytes.
2. Perform ONE fresh-context review of this bounded diff, focusing on residual sign, frozen-W/strict-lower assumptions, condition-aware tests, missing/uncertain budgets, and valid failure accounting. The report here is a self adversarial pass plus external-audit adjudication, not that fresh review.
3. Extend the test-only correction to nonidentity nonsingular mass matrices, strongly nonnormal cases, zero RHS, missing-JVP access and failed/overflowed solves. Use explicit backward-error/state/invariant criteria; no equality of secondary errors. Retain every informative failure.
4. Only after these tests and review, wire the common-W correction behind an explicit opt-in research entry. Keep the full-target oracle as the default reference; evaluate both on matching trial stage states. Do not silently activate it in production or replace nonlinear validity with a correction norm. Count common-W setup, JVPs, all solves and diagnostic residual applies.
5. Do not change the 54 historical rows or their checksums. The new accuracy API requires an external B; absence of B is BudgetNotSpecified, not PASS. Before any new accuracy/equal-error campaign, specify an observable budget and reference-uncertainty treatment independently of these54 data. Reusing old data for diagnosis is not a fresh holdout.
6. CVODE setup and a future production-baseline performance campaign are separate, later tasks. No timing, ranking or speedup claim is authorized by these tests. No Oregonator execution or calibration freeze now.
7. After scoped tests and the existing differential audit, publish changes only to an OPEN/DRAFT/UNMERGED research PR. Synchronize GitHub and the existing Atlassian workstream with actual commit/tree/tests and truthful pending states; do not mark PM-7/K0 complete. Use ATLAS_SYNC_PENDING if the connector actually fails.

## Reproduction, not mandatory duplicate work

From the repo root:

```bash
python3 tools/audit2_output_policy_research.py --output /tmp/vigilode-audit2-fresh
cargo test --locked -p rodas5p-fair-ab --test output_accuracy_assessment_contracts
cargo test --locked -p rodas5p-integrators --test audit2_structured_correction_contracts -- --nocapture --test-threads=1
cargo test --locked -p rodas5p-integrators --test dense_output_v2_contracts
cargo test --locked -p rodas5p-integrators --test homotopy_numerical_contracts
```

Normal Cargo access suffices; the optional external offline vendor is not a prerequisite. Use the actual tests already recorded here instead of blindly rerunning a large campaign. A full workspace attempt was interrupted by runtime replacement; it is NOT a PASS. Re-run only the affected omitted tests or a resource-bounded host suite if review identifies a distinct regression it can detect.

## Closure and handback

Return exact changed source paths, current base/head/tree, actual test logs, review dispositions, unchanged historical-input checks, any condition/domain counterexamples, and the scoped research PR. Separate RESULT_VALIDITY, PROVENANCE_VALIDITY, and PACKAGING_VALIDITY. No new bootstrap, schema, manifest family or review tier. No fabrication of missing historical evidence.
