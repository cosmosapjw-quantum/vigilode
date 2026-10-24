# Reproduce in a fresh copy

Verify the S3 ZIP SHA256 from FINAL_DELIVERY.json, extract to a NEW directory and verify ARCHIVE_MANIFEST.json. It contains95payloads: all exact stage/trial bits, polynomial certificate, verifier/source, RED/GREEN logs and closed budget. Compiler and executable binaries are excluded as regenerable.

Saved-only review: in a COPY run `python review_saved.py` with mpmath installed. It recomputes Fraction domains/norms, callback permission, rollback, MP80 references and polynomial identities without compiler/solver calls; REVIEW.json is written in that COPY. Never overwrite historical data.

A separately authorized fresh experiment uses Rust1.94.1, x86_64 ordinary IEEE/no-fast-math, a new directory and budget:
```
mkdir -p bin evidence
rustc --edition=2021 --test source/tests_d2.rs -o bin/tests_d2
bin/tests_d2 --test-threads=1
rustc --edition=2021 --test source/tests_d1_regression.rs -o bin/tests_regression
bin/tests_regression --test-threads=1
rustc --edition=2021 -C opt-level=3 source/campaign_d2.rs -o bin/campaign_d2
bin/campaign_d2
python theory.py
python review_saved.py
```
Python scripts and pinned input are in the complete companion. Campaign overwrites its named raw outputs, so use a fresh copy. Do not reset the old closed run.py budget.

Callbacks must match the manufactured analytic diagonal Jacobian. Arm0 is an unsafe-domain polynomial-extension diagnostic; arms1/2 enforce input checks. Nonlinear rate cap is heuristic, not authorization. Internal-stage and endpoint checks do not certify a dense interpolant. The inherited MXCSR asm remains outside the production forbid-unsafe boundary; no direct production copy is authorized by this evidence.
