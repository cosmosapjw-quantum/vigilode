Pinned source BASE.txt, original MathHarness 3.1.0 preserved in preflight.
All cargo commands --locked --offline --release --features polynomial-events.
Run in fresh output directory, do not overwrite original receipts.
    cargo test -p vigilode --test physical_event_contracts -- --nocapture --test-threads=1
    PYTHONPATH=PINNED_HARNESS python3 guide.py generate
    cargo test -p vigilode --test physical_event_contracts training -- --ignored --exact --nocapture --test-threads=1
    python3 audit.py training
    PYTHONPATH=PINNED_HARNESS python3 guide.py select
    export VIGILODE_PHYSICAL_SELECTION=ABSOLUTE_SELECTION_JSON
    cargo test -p vigilode --test physical_event_contracts holdout -- --ignored --exact --nocapture --test-threads=1
    python3 audit.py holdout
    cargo test -p vigilode --test physical_event_contracts precision_campaign -- --ignored --exact --nocapture --test-threads=1
    python3 audit.py precision
    PYTHONPATH=PINNED_HARNESS python3 select_precision.py
    export VIGILODE_PHYSICAL_PRECISION_SELECTION=ABSOLUTE_PRECISION_SELECTION_JSON
    cargo test -p vigilode --test physical_event_contracts precision_holdout -- --ignored --exact --nocapture --test-threads=1
    python3 audit.py precision_holdout
    PYTHONPATH=PINNED_HARNESS python3 exact_checks.py
    python3 audit_disposition.py
    PYTHONPATH=PINNED_HARNESS python3 replay_campaign.py
Store stdout in training.log, holdout.log, precision.log, precision_holdout.log.
Freeze selections before respective holdouts. Replay uses final_facade.log
recorded compiled binary path and verifies source/byte identity. If relocating,
use actual newly built binary path in a new evidence directory. No timing
authority or arbitrary nonlinear physical root guarantee.
