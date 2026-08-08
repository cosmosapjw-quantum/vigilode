# VALIDATION_MATRIX.md

| Requirement | Test/check | Level | Expected | Status | Evidence |
|---|---|---|---|---|---|
| compile | release build | software | pass | PASS | final validation log |
| lint | strict Clippy | software | zero warnings | PASS | final validation log |
| error metrics | known exact/perturbed trajectory | scientific | exact/1e-14 | PASS | `global_error_contracts` |
| common grid | missing time rejection | scientific | explicit failure | PASS | `global_error_contracts` |
| Pareto dominance | synthetic points | scientific | expected ids | PASS | `global_error_contracts` |
| candidate coverage | five fixed anchors | acceptance | all present | PASS | smoke/canonical JSON |
| reference provenance | analytic exact hashes | scientific | 4 canonical refs | PASS | canonical T1 JSON |
| failure preservation | failed point excluded from winner | software | pass | PASS | Pareto unit test |
| repeated timing | calibrated samples | operational | 5 canonical samples | PASS | canonical T1 JSON |
| timing semantics | suite vs campaign clocks | operational | distinct fields | PASS | RED/GREEN regression |
| thread determinism | T1/T4 checksum | reproducibility | exact equality | PASS | checksum `72cb...706e` |
| rerun determinism | T1 scientific checksum | reproducibility | exact equality | PASS | T1 rerun |
| timing sanity | T1 rerun median ratio | operational | central IQR near 1 | PASS | Q25=0.977, median=1.006, Q75=1.022 |
| numerical trend | fixed anchors | numerical | expected order before floor | PASS | observed-order CSV |
| code regression | fair-ab suite | software | pass | PASS | validation log |
| CLI contract | `global-error-pareto` | software | JSON + checksum identity | PASS | CLI test |
| research harness | workspace validator | operational | pass | PENDING_FINAL | final validation |
| coding harness | harness validator | operational | pass | PENDING_FINAL | final validation |
| clean source rebuild | clean archive | reproducibility | pass | PENDING_FINAL | final validation |
| independent review | base-to-head review | review | no critical issue | PASS | `reviews/INDEPENDENT_DIFF_REVIEW.md` |
| production solver ranking | larger/adaptive corpus | scientific | completed | NOT_RUN | transferred to v0.11 |

Status: `PASS / CONCERN / FAIL / NOT_RUN / NOT_APPLICABLE / PENDING_FINAL`
