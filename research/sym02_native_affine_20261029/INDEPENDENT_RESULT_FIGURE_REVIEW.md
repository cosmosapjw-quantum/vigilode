# SYM02 complementary C9 result and figure review

Reviewer: external_survey; separate from parent/native implementation and complementary to source_contract's source review in the same single C9 pass. Date: 2026-10-29. **Verdict: PASS_SCOPED_RESULT_AND_FIGURE_REVIEW; blocking findings: 0.**

## Actual review and fixed evidence

I read `THEORY.md`, `CODING_PLAN.md`, `THEORY_DECISION.md`, `survey/INDEPENDENT_MATH.md`, `FINAL_SOURCE_IDENTITY.json`, `NATIVE_BITS.json`, `BIT_ORACLE_RESULTS.json`, `MATH_RESULTS.json`, `PLOT_DATA.json`, `PLOT_INFO.json`, `plot_validation.py`, `STAGES.json`, `INDEPENDENT_CODE_REVIEW.md`, and `evidence/C6_C8_FINAL_TESTS.log` under `vigilode_sym02/research/sym02_native_affine_20261029/`. I directly viewed the actual `VALIDATION.png` through view_image. No scientific script, build, test, IVP, benchmark or CAS was executed by this reviewer. No further reviewers were started.

- Published compile/code identity `567420e6b1cc6b847732c72ab0d47484a516a1b9` agrees between final source identity, native raw data and bit-oracle result. Final identity records immutable Git readback for five executed source/test/example files. The complementary source reviewer owns the static-review-to-final-format source comparison.
- The focused final log actually records **13 new tests + 8 existing fast-dense tests PASS**, zero failures. It does not establish release-wide/production/GMRES/authority-ledger CI success.
- Native bit-oracle evidence is **6 records: 4 declared represented packets and 2 actual accepted native scalar steps; 42 query records; 381 exact-or-diagnostic checks, including 14 Decimal90 native diagnostics**. The verifier reconstructs the exact stored-real quartic, physical clock width, residual/Bernstein hull, query polynomial and actual-center radius using Fraction. The result correctly labels the mixed check count rather than calling all381 exact algebra assertions.
- Mathematical evidence is separately **324 rational cases, 16,524 exact algebra assertions and 11,016 analytic diagnostic samples**. It is not 324 physical IVPs. The whole-slab justification is the Bernstein/Dini argument, including the two-envelope proof in the independent mathematical contract; finite analytic diagnostics remain diagnostics.

## Figure and quantity acceptance

The actual PNG is readable: panel A shows all positive native query-error/rounding bounds and analytic discrepancies on logarithmic axes; panel B shows the 15/16 threshold, actual native centers and the analytic reference crossing. Error bars in panel B are smaller than marker/plot resolution, consistent with the approximately 3.32e-9/6.23e-9 query radii against an event-value range of order1e-1. This visibility limitation does not replace the numerical enclosure evidence.

The plotted 14 native query records come only from the two actual accepted steps; the shared endpoint at t=0.0625 occurs twice, so14 means records, not14 distinct times. All42 raw query records remain in PLOT_DATA, including the declared cancellation/subnormal/parameter-box controls. Exactly six zero-valued series entries are omitted only from the logarithmic panel, as disclosed by PLOT_INFO; no positive outlier is hidden and panel B retains the zero-initial-error point.

The state is dimensionless and time is the chosen fixture `unit-time`, as the raw unit_id says. The axis phrase “Physical time” is read as the equation's unnormalized clock, not an assertion of SI seconds or a cosmological source. An explicit “fixture time units” caption would improve standalone reuse but is nonblocking. The red curve and vertical crossing are labeled diagnostic. Scatter upper bounds are not presented as a sampled proof of a uniform all-query rounding constant.

## Scope, provenance and completion boundary

Source/model/clock/unit/event/epoch identifiers bind caller assertions. They do not independently prove an arbitrary RHS callback equals y'=-lambda*y+b, establish a physical source-rate uncertainty, or supply the incoming exact-IVP error premise. THEORY, raw source labels and the original static review state this correctly. Native provenance means the adapter copied actual accepted RODAS segment data and observed native returned bits. The declared lambda/b/E0 premises still apply.

The accepted output is conditional scalar-affine whole-slab defect/state/event certification plus pointwise native-query discrepancy and guarded accumulated-error carry. BASS30 provider=false and performance_claim=false remain appropriate. No general nonlinear/full30 provider, production release, uniform sampled-rounding theorem, strong no-go or speedup follows.

At the reviewed STAGES snapshot, **17/20** is internally consistent (R1–R10 and C1–C7). C8/C9/C10 updates and their actual publication/backup receipts are parent closeout work; this review does not assert those future writes. Original core **7/23** means the inherited scoped original DAG histogram (7 complete_scoped, 15 pending_local, 1 registration_repaired), delta0. It is not seven fresh broad scientific closures. Current evidence labels differ: Q-GIVENS/Q-LINT can be PASS_SCOPED_CURRENT while broad R-QUALIFY remains HOLD. Parent's forthcoming DAG_BASIS.json records that distinction. Central **28/88** and auxiliary **0/1 until final C10 acceptance** remain separate. Auxiliary may become1/1 after the complete scoped acceptance and actual final preservation record without altering either original DAG count.

Clippy's final outcome was not available to this review and is not claimed. Parent subsequently reported one needless_range_loop in a new test helper and is making equivalent iterator/sum repairs followed by republishing and targeted verification. This review is bound to the reviewed source/results snapshot above; final source identity and post-repair test receipts must replace it for delivery. No new result/figure issue follows from the reported equivalent helper repair. Inherited production/GMRES/ledger CI failures remain separate. No result or figure correction is required for the reviewed scientific scope.

