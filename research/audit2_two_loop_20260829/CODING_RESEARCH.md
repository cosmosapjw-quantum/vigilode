# Coding loop — actual source, tests, and negative controls

## Contract and localization

Base: `codex/vigilode-scientific-validity-v2@93fe348ce36859dd5f78b31267d771ea9c054677`.
This is not the unstarted K0 preparation lane. The imported source tree was reconstructed from the GitHub Actions source archive and matched `a0a045226175e261664533a56188352ded1da75a`. The archive is a transport object; the exact source tree, not archive compression bytes, identifies the input.

The external second audit is evaluated against `global_error.rs`, `scientific_validity_v2_campaign.rs`, the official coefficient snapshot, `dense_output_v2.rs`, `StructuredBlockSystem`, and all54 compact campaign cases. Six full trajectories allow independent metric recomputation; the other48 full trajectories are not in the submitted Git bundle. They are not inferred or regenerated.

## Bounded implementation

`output_accuracy.rs` adds a read-only opt-in assessment. It retains the existing policy classification and independently reports a conditional error interval `[max(E-u,0), E+u]` against an explicitly supplied budget. No supplied budget means `BudgetNotSpecified`. This does not convert the historical 54 nonpassing rows to PASS, modify the freeze protocol, or establish that the empirical reference uncertainty is a rigorous bound.

`tests/support/common_w_target_correction.rs` is test-only. It exploits the exact block-lower-triangular target Jacobian with diagonal W. It uses one common-W LU, eight vector solves, and fourteen counted analytic JVPs. It never assembles varying stage Jacobians or the full8n target. The production homotopy/certification path is unchanged. The exact frozen Jacobian and common-W setup still cost work; this is semi-Jacobian-free, not derivative-free.

`tools/audit2_output_policy_research.py` reads existing evidence and official decimal coefficients without modifying them. It creates a new output directory, not an in-place rewrite. It recomputes54 diagnostics, six full-trajectory metrics, local/global scalar refinements, and an exact-flow error decomposition with an intentionally low-order interpolation mutant. No arbitrary accuracy budget is selected from the data.

## Actual validation after runtime recovery

Final exit code:0. Rust1.94.1 with the original Cargo.lock and optional external offline vendor.

| Suite | Passed |
|---|---:|
| Existing global_error_contracts |8|
| New output_accuracy_assessment_contracts |9|
| New structured correction contracts |2 (including12 cases)|
| Existing dense_output_v2_contracts |15|
| Existing homotopy_numerical_contracts |6|
| **Rust total** |**40**|
| Independent Python contracts |8|

The integration test checks that assessment leaves the serialized paired evidence unchanged. The correction test compares independent full-target LU and block-forward solutions using normalized backward error plus a conditioning-aware state tolerance. It does not require equality of tiny secondary error estimates.

Actual Rust mutation runs (not name/grep checks) rejected all three mutations with nonzero Cargo status: missing budget falsely passes; all stage Jacobians replaced by the frozen J; correction-coupling sign reversed. The source was restored, formatted, and the final40 tests passed. All mutation outputs are retained.

The earlier full-workspace attempt was interrupted by runtime replacement. It is **not verified** and is not used as evidence. The completed scoped suite above was rerun after recovery. No external fresh-context code review has happened here: the pass below is a self adversarial pass, supplemented by externally supplied audits and actual Wolfram algebra checks.

## Self adversarial pass and remaining risks

The strongest failure classes checked are false PASS without a budget, false FAIL for accurate but policy-sensitive trajectories, uncertain-reference boundaries, NaN/Inf/negative/overflow inputs, signed zero, scalar-error cancellation, wrong-JVP state, wrong residual sign, and malformed correction shapes.

Unclosed issues are nonidentity/singular mass-domain handling, severe nonnormal conditioning, zero RHS, missing-JVP access, counter overflow, failed LU accounting, and nonlinear residual validity after a proposed update. The full-target correction itself is a local linearized diagnostic, not an existence/uniqueness certificate for a general nonlinear model. Frozen-W triangularity is essential. These are the local candidate-validation tasks, not a reason to replay an unrelated historical campaign.

## Plot-driven review

Local manufactured errors show endpoint order near6 and dense order near5, while the linear-interpolation mutant is order2. Global endpoint and dense errors both approach order5 in the nonstiff sequence. The stiff sequence and smallest-step coefficient-rounding effects are retained rather than fitted away. All54 actual policy points lie above the historical D=0.1E_dense line; this is sensitivity evidence, not an absolute accuracy decision. The12 common-W tests have backward errors below4.4e-17 while the condition-number upper bound ranges32–2687. None of these plots measures wall-time speedup.

## Decision

PROMOTE the mathematics and diagnostics to a research memo. HOLD the correction candidate at test-only status until one fresh review and bounded extension tests pass. Keep the original historical gate and full-target oracle default. Do not create a new package/authority loop, change the held-out family, tune a threshold to these54 rows, or activate production.
