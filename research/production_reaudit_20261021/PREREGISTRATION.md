# Final integration verification protocol — 2026-10-21

This document is written before the final combined-source matrix in `run_targeted.py`. It is **not** a retrospective preregistration of the already completed adversarial discoveries, proofs, node-level tests, B1d diagnostic, or historical benchmark runs. Their original chronology and failed logs remain in this package.

The object is the functional union of production wave3 `f2f20c1d8cd606e70ddbd8409c0f3b4529e928a1`, plan `cc2e1a009ac6802ef01a289392e9715074a882ba`, and all-DAG research `ee65d4dbecf3b3cd5f4587c27f87ef297d2bb406`, with the explicitly documented correctness repairs.

Before running, freeze a SHA-256 manifest of the combined source. Run the named native contracts, compatibility tests, doctests, imported component example check, formatting and scoped clippy sequentially with Rust 1.94.1 and the existing locks. Preserve each command, exit code and raw log, including failures. Compare the source manifest before and after. Any changed source or failed command makes that execution FAIL; a subsequent correction requires a new preserved execution directory.

PASS means the specified integration matrix passed at its exact source. It does not mean the whole workspace/native external benchmark crate passed, that arbitrary callbacks have a rigorous global error certificate, or that wall-clock speed improved. No scientific performance threshold, tolerance or historical verdict is changed. Missing external dependencies remain a separate blocker. Unchanged published scientific campaigns are reused by original source and evidence identity, not rerun.
