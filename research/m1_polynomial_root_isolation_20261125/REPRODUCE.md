# Reproduce
Use clean checkout of published research branch, exact original MathHarness
3.1.0 extracted from preflight archive, unchanged protected kernel (53).
Use fresh output directory; do not overwrite historical outputs. Source
files and Cargo.lock pinned; default feature remains off.
    cargo test --locked --offline --release -p vigilode --features polynomial-events --test polynomial_event_contracts -- --nocapture --test-threads=1
    PYTHONPATH=PINNED_HARNESS python3 guide.py generate
    export VIGILODE_POLY_CANDIDATES=ABSOLUTE_PATH_TO_CANDIDATES.json
    cargo test --locked --offline --release -p vigilode --features polynomial-events --test polynomial_event_portfolio training -- --ignored --exact --nocapture --test-threads=1
    PYTHONPATH=PINNED_HARNESS python3 audit.py training
    PYTHONPATH=PINNED_HARNESS python3 guide.py select
    sha256sum SELECTION.json > SELECTION_PREHOLDOUT.sha256
    export VIGILODE_POLY_SELECTION=ABSOLUTE_PATH_TO_SELECTION.json
    cargo test --locked --offline --release -p vigilode --features polynomial-events --test polynomial_event_portfolio holdout -- --ignored --exact --nocapture --test-threads=1
    PYTHONPATH=PINNED_HARNESS python3 audit.py holdout
    PYTHONPATH=PINNED_HARNESS python3 replay_campaign.py
Scripts locate receipts relative to their directory. Preserve stdout as
contracts.log, training.log, holdout.log respectively; audit_packet.py
reads PACKET records from final_facade.log. For fresh replay adjust absolute
binary path in recorded cargo receipt to newly built same source executable.
Original selection is held fixed in evidence, do not reselect after holdout.
Standalone benchmark locked metadata checked; no new benchmark trajectories.
