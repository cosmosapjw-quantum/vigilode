# Reproduce
Use pinned source BASE.txt and original MathHarness 3.1.0 from preflight.
All cargo commands --locked --offline --release --features polynomial-events.
Use fresh evidence directory, do not overwrite recorded original receipts.
    cargo test -p vigilode --test hybrid_reset_contracts -- --nocapture --test-threads=1
    PYTHONPATH=PINNED_HARNESS python3 guide.py generate
    cargo test -p vigilode --test hybrid_reset_contracts training -- --ignored --exact --nocapture --test-threads=1
    python3 audit.py training
    PYTHONPATH=PINNED_HARNESS python3 guide.py select
    sha256sum SELECTION.json > SELECTION_PREHOLDOUT.sha256
    export VIGILODE_HYBRID_SELECTION=ABSOLUTE_SELECTION_JSON
    cargo test -p vigilode --test hybrid_reset_contracts holdout -- --ignored --exact --nocapture --test-threads=1
    python3 audit.py holdout
    python3 audit_sync_gap.py
    PYTHONPATH=PINNED_HARNESS python3 exact_checks.py
    python3 external_check.py
    PYTHONPATH=PINNED_HARNESS python3 replay_campaign.py
Record stdout as training.log and holdout.log. Final replay uses actual
binary path recorded in final_facade.log; if relocating, use corresponding
freshly built binary path in the fresh evidence directory. Preserve original
selection before holdout; no retune. The manifest validator checks artifact
bytes; the replay script actually executes the binary before validation.
