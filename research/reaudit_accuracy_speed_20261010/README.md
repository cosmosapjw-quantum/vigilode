# 2026-10-10 accuracy/speed delta audit

Start with [Korean review](../../docs/reviews/20261010_accuracy_speed/REVIEW_KO.md)
and [mathematical porting design](../../docs/reviews/20261010_accuracy_speed/MATHEMATICS_PORTING_KO.md).

- `FINDINGS.json`, `CLAIMS.json`: new findings and evidence/claim boundaries.
- `NEXT_DEVELOPMENT_DAG.json`: 22 proposed, unexecuted development nodes. Commands
  are templates for tests to be implemented; they are not execution receipts.
- `ARTIFACT_SCHEMA.json`: portable JSON Schema; `validate_artifacts.py`: dependency-free
  cross-file semantic checks, no old scientific rerun.
- `NATIVE.json`, `RESULTS.json`, `CHECKER_PROBES.json`: new bounded execution.
- `NATIVE_FIRST.json`, `RESULTS_FIRST.json`, `FIRST_FAILURE.md`: first FAIL preserved.
- `gain_witness.rs`: represented-real-2x2 Rust prototype, included by the new native
  example; no production dispatch change.
- `SOURCE_BINDING.json`, `REUSED_RESULTS.json`, `VALIDATION.json`: provenance and
  exactly which published work was reused instead of rerun.
- `evidence/independent_decision.json`: independent bounded decision; all production,
  global accuracy and speed claims remain HOLD.

The exact-rational oracle is `check.py`. It reads this node's NATIVE.json and writes
RESULTS.json; reproduce in a separate checkout/copy to preserve published evidence.
The independent review additionally checks complete registered input identity and
all interval intermediates (`evidence/independent_checks.py`). A future automated
promotion checker must incorporate that manifest check, not rely on row counts.

The first invalid-scale fixture used identity and was actually valid; only this
fixture was corrected after the first run. Bounds can be very loose, so local
enclosure PASS is not usefulness, generic solver accuracy or speedup.
