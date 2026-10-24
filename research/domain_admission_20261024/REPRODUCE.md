# Reproduce in a fresh copy

Verify the S3 archive SHA256 in FINAL_DELIVERY.json before extraction. It contains57payloads plus ARCHIVE_MANIFEST.json, including fullrawtrialbits/states, PythonFraction/MP80 reviewer, exact stability recurrence, RED/GREENlogs and closed budgets. Compiler archives/executables are omitted as regenerable.

Saved-only review: in a COPY of the archive run `python review_saved.py` with mpmath installed. It invokes no solver/compiler, checks the saved budget log hashes and writes REVIEW.json in the copy. Do not overwrite historical artifacts.

Fresh separately authorized experiment (Rust1.94.1,x86_64,ordinaryIEEE/no-fast-math) in a NEW copy:
```
mkdir -p bin evidence
rustc --edition=2021 --test source/tests.rs -o bin/tests
bin/tests --test-threads=1
rustc --edition=2021 -C opt-level=3 source/experiment.rs -o bin/experiment
bin/experiment
```
The Git subtree includes the pinned old consumer required for same-input parity. The full Python reviewer is in the companion. Fresh experiments need a new budget and evidence directory, not the old closed ledger. `python stability.py` uses Fraction interpretations of the pinned binary64 tableau and is distinct from compiled rounding.

Endpointpositivity is not internal-stage/dense positivity; no clipping or speed claim. Research MXCSR guard uses unsafe assembly and must not be copied directly into the production forbid(unsafe_code) crate. Preserve that production policy and validate a safe boundary for integration.
