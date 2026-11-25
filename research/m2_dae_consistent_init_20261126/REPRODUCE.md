# Reproduce
Pinned source BASE.txt and original MathHarness 3.1.0 extracted in preflight.
Use fresh evidence directory; do not overwrite frozen selection/receipts.
All cargo test commands --locked --offline --release --features polynomial-events.
    cargo test -p vigilode --test dae_init_contracts -- --nocapture --test-threads=1
    PYTHONPATH=PINNED_HARNESS python3 guide.py generate
    cargo test -p vigilode --test dae_init_contracts training -- --ignored --exact --nocapture --test-threads=1
    python3 audit.py training
    PYTHONPATH=PINNED_HARNESS python3 guide.py select
    sha256sum SELECTION.json > SELECTION_PREHOLDOUT.sha256
    export VIGILODE_DAE_SELECTION=ABSOLUTE_SELECTION_JSON
    cargo test -p vigilode --test dae_init_contracts holdout -- --ignored --exact --nocapture --test-threads=1
    python3 audit.py holdout
    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 python3 audit.py mass
    PYTHONPATH=PINNED_HARNESS python3 exact_checks.py
    PYTHONPATH=PINNED_HARNESS python3 replay_campaign.py
Record actual stdout in contracts.log, training.log, holdout.log.
Replay uses final_facade.log recorded binary path; if relocating, use
actual rebuilt binary path in new evidence directory. Manifest validates
bytes; replay script explicitly executes before validation. No retuning
after frozen holdout; diagnostic timing only. See PLAN.md and LITERATURE.md.
