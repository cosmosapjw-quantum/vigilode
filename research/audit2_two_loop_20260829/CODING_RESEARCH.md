# Coding loop — actual source, tests, and negative controls

## Contract and localization

Base: `codex/vigilode-scientific-validity-v2@93fe348ce36859dd5f78b31267d771ea9c054677`.
This is not the unstarted K0 preparation lane. The imported source tree was reconstructed from the GitHub Actions source archive and matched `a0a045226175e261664533a56188352ded1da75a`. The archive is a transport object; the exact source tree, not archive compression bytes, identifies the input.

The external second audit is evaluated against `global_error.rs`, `scientific_validity_v2_campaign.rs`, the official coefficient snapshot, `dense_output_v2.rs`, `StructuredBlockSystem`, and all54 compact campaign cases. Six full trajectories allow independent metric recomputation; the other48 full trajectories are not in the submitted Git bundle. They are not inferred or regenerated.

## Bounded implementation

`output_accuracy.rs` adds a read-only opt-in assessment. It retains the existing policy classification and independently reports a conditional error interval `[max(E-u,0), E+u]` against an explicitly supplied budget. No supplied budget means `BudgetNotSpecified`. The caller must now supply `ReferenceUncertaintyTreatment`: `EstimateOnly` always leaves a budgeted result `ReferenceUnresolved`, while `DeclaredUpperBound` is an explicit caller assertion and is never inferred from a stored uncertainty, including zero. This does not convert the historical54 nonpassing rows to PASS, modify the freeze protocol, or establish that the empirical reference uncertainty is a rigorous bound.

`src/audit2_research.rs` is compiled only with the non-default `audit2-research` Cargo feature. The default research backend is `FullTargetOracle`; `CommonWBlockForward` requires explicit opt-in. The comparison prepares one projected target snapshot so both arms see matching trial stage states. No integration, gate, campaign, or production dispatch enables the feature or calls the entry. The production homotopy/certification path and unprojected coefficients remain unchanged.

The official decimal tableau has roundoff-scale forbidden entries, so the implementation does not claim that the raw coefficients are exactly strict lower. It records maxima for forbidden alpha (`5.577737968635803e-16`), upper Gamma (`4.994632140352628e-16`), and Gamma diagonal error (`8.881784197001252e-16`), then applies the result-independent research projection tolerance `64*f64::EPSILON = 1.4210854715202004e-14`. Values beyond that fixed tolerance fail; accepted entries are projected to the exact research structure. This rule was fixed before observing correction results and does not edit production data.

The common-W arm uses one setup, one LU, eight solves and fourteen correction JVPs. Its diagnostics count eight shifted applies, fourteen off-diagonal JVPs and one nonlinear residual evaluation; two independent full-target residual applies belong to comparison validation. Every category has attempt/completion counts, underlying work counters, a typed failure phase and retained partial correction. A finite correction norm is never a nonlinear-validity certificate.

`tools/audit2_output_policy_research.py` reads existing evidence and official decimal coefficients without modifying them. It creates a new output directory, not an in-place rewrite. It recomputes54 diagnostics, six full-trajectory metrics, local/global scalar refinements, and an exact-flow error decomposition with an intentionally low-order interpolation mutant. The six full trajectories are the fixed metadata slice containing all six families at n=96 and rtol=1e-8, selected independently of outcomes. No numerical accuracy budget B is selected from the54 data.

## Validation state after continuation

The inherited restored-source log remains valid for its exact pre-continuation source:40 Rust tests and8 Python tests passed under Rust1.94.1 with the original Cargo.lock. It is not relabelled as a test of the continuation edits.

| Suite | Passed |
|---|---:|
| Existing global_error_contracts |8|
| Pre-continuation output_accuracy_assessment_contracts |9|
| Pre-continuation structured correction contracts |2 (including12 cases)|
| Existing dense_output_v2_contracts |15|
| Existing homotopy_numerical_contracts |6|
| **Rust total** |**40**|
| Independent Python contracts |8|

For the continuation source, the final aggregate structured suite passed all11 tests in one invocation and the updated accuracy suite passed9 tests. Together with8 global-error,15 dense-output, and6 homotopy tests, the bounded affected Rust set passed49 tests. Two adjacent core coefficient checks also passed, for51 scoped Rust tests. The independent Python contract suite passed8 tests and the analysis script re-read all54 compact rows and recomputed all six saved trajectories. The exact commands and exit statuses are in `evidence/continuation_verification.log`.

The expanded structured suite covers the original12 identity-mass cases; nonsingular nonidentity mass determinant2.7; strong nonnormal off-diagonal ratio134.6667; zero RHS; missing, failing, and inconsistent JVP; singular and first/later-stage overflowed solves; NaN; and malformed shape. Acceptance uses independently applied normalized backward error, state difference scaled by the full-target condition estimate, and explicit structural/accounting invariants. It never requires equality of secondary errors. The correction case matrix and numerical bounds were fixed before outcomes were observed.

Actual Rust mutation runs (not name/grep checks) rejected all three mutations with nonzero Cargo status: missing budget falsely passes; all stage Jacobians replaced by the frozen J; correction-coupling sign reversed. The source was restored, formatted, and the final40 tests passed. All mutation outputs are retained.

The earlier full-workspace attempt was interrupted by runtime replacement. It is **not verified** and is not used as evidence. No full workspace rerun is implied by the continuation.

Exactly one fresh-context review of `93fe348ce36859dd5f78b31267d771ea9c054677..25e086f86819577978e0710d2dab9c352555c4cc` returned `REQUEST_CHANGES`. It confirmed the residual sign `R=lhs-rhs`, update `K<-K-z`, and lack of production activation. P1 findings were the invalid literal exact-structure claim caused by decimal leakage, missing uncertainty authority, and erased failure work; P2 was zero-RHS 0/0 handling; P3 was formatting. The continuation repair addresses each item. The repaired final head has not received a second fresh-context review; none is claimed or required by this one-review bounded task. See `evidence/fresh_review_disposition.md`.

## Self adversarial pass and remaining risks

The strongest failure classes checked are false PASS without a budget, estimate-only uncertainty misused as an authority, false FAIL for accurate but policy-sensitive trajectories, uncertain-reference boundaries, NaN/Inf/negative/overflow inputs, signed zero, scalar-error cancellation, wrong-JVP state, wrong residual sign, malformed correction shapes, singular and overflowed common-W solves, and loss of work/partial-progress evidence.

The implemented tests close the originally named nonidentity nonsingular mass, strong nonnormal, zero-RHS, missing/failed/inconsistent JVP, solve-failure/overflow, and explicit accounting gaps within this research path. They deliberately retain singular W and inconsistent JVP as counterexamples, not supported domains. The correction remains a local linearized diagnostic, not an existence/uniqueness certificate for a general nonlinear model. Frozen common W, the projected strict-lower target, well-defined stage evaluations, and a JVP consistent with the oracle are essential.

## Plot-driven review

Local manufactured errors show endpoint order near6 and dense order near5, while the linear-interpolation mutant is order2. Global endpoint and dense errors both approach order5 in the nonstiff sequence. The stiff sequence and smallest-step coefficient-rounding effects are retained rather than fitted away. All54 actual policy points lie above the historical D=0.1E_dense line; this is sensitivity evidence, not an absolute accuracy decision. The original12 projected common-W cases have maximum independent backward error about4.6955771192625894e-17, maximum state-relative difference4.664692226645801e-15, and condition range32.1868941892757–2686.7976507838134. None of these plots or tests measures wall-time speedup.

## Decision

Retain the mathematics and diagnostics as `EXPLORATORY_NONAUTHORITATIVE` research only. The correction is available only through an explicit non-default research feature, with the full-target oracle default. Scoped verification passed; this does not upgrade the scientific claim. Actual delivery HEAD/tree, CI, and workstream synchronization remain live external receipts. Do not create a new package/authority loop, change the held-out family, tune a threshold or budget to these54 rows, activate production, or infer timing/ranking/speedup, BDF/CVODE, holdout/freeze, nonlinear-certificate, or publication-admission claims.
