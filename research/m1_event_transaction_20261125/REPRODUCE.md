# M1-EVENT-TRANSACTION-01 reproduction
Pinned parent: BASE.txt. Source additions in this branch only. Run in a clean
checkout and a separate output directory; preserve recorded raw logs.
Use pinned Rust toolchain, --locked --offline. The MathHarness 3.1.0 archive
from the preflight is immutable (see its HARNESS_SHA256.txt).
- cargo test --locked --offline --release -p vigilode --features retro-dictionary --test event_transaction_contracts -- --test-threads=1
- PYTHONPATH=PINNED_HARNESS python3 research/m1_event_transaction_20261125/guide.py generate
- export VIGILODE_M1_CANDIDATES=ABSOLUTE_PATH_TO_CANDIDATES.json
- cargo test --locked --offline --release -p vigilode --features retro-dictionary --test event_transaction_portfolio training -- --ignored --exact --nocapture --test-threads=1
  Redirect stdout/stderr to training.log (fresh directory).
- PYTHONPATH=PINNED_HARNESS python3 research/m1_event_transaction_20261125/guide.py select
  Freeze SELECTION.json hash before holdout.
- export VIGILODE_M1_SELECTION=ABSOLUTE_PATH_TO_SELECTION.json
- cargo test --locked --offline --release -p vigilode --features retro-dictionary --test event_transaction_portfolio holdout -- --ignored --exact --nocapture --test-threads=1
  Redirect stdout/stderr to holdout.log.
- PYTHONPATH=PINNED_HARNESS python3 research/m1_event_transaction_20261125/audit_candidates.py
- python3 research/m1_event_transaction_20261125/external_event_check.py
Timing is diagnostic only. No automatic selection for unknown event functions.
