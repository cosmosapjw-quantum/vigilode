Use pinned parent BASE.txt and published research branch, original
MathHarness 3.1.0 from preflight. Fresh output directory, do not overwrite
the recorded frozen selection. Cargo --locked --offline --release.
  cargo test -p vigilode --features polynomial-events --test dae_evolution_contracts -- --nocapture --test-threads=1
  PYTHONPATH=PINNED_HARNESS python3 guide.py generate
  cargo test -p vigilode --features polynomial-events --test dae_evolution_contracts training -- --ignored --exact --nocapture --test-threads=1
  OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 python3 audit.py training
  PYTHONPATH=PINNED_HARNESS python3 guide.py select
  sha256sum SELECTION.json > SELECTION_PREHOLDOUT.sha256
  export VIGILODE_DAE_EVOLUTION_SELECTION=ABSOLUTE_SELECTION_JSON
  cargo test -p vigilode --features polynomial-events --test dae_evolution_contracts holdout -- --ignored --exact --nocapture --test-threads=1
  python3 audit.py holdout
  cargo test -p vigilode --features polynomial-events --test dae_evolution_contracts recovery -- --ignored --exact --nocapture --test-threads=1
  python3 audit.py recovery
  PYTHONPATH=PINNED_HARNESS python3 exact_checks.py
  PYTHONPATH=PINNED_HARNESS python3 replay_campaign.py
Record actual stdout as training.log,holdout.log,recovery.log. Recovery
is separate post-holdout diagnostic, not a new independent holdout.
Replay uses final_facade.log recorded binary path; on relocation use
actual freshly built binary path and new evidence directory. Manifest
checks recorded byte identity; script actually executes the binary.
