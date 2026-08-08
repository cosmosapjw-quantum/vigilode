# FAILURE_LOG.md

## F-001 — stale lockfile gate

Symptom: first `--locked` test refused to update Cargo.lock after dependency-edge experimentation.
Root cause: lock metadata had not been refreshed for the current manifest state.
Resolution: one offline unlocked metadata/test pass, then all locked runs succeeded. No dependency was added.

## F-002 — draft API/type mismatch

Symptom: unresolved public exports and wrong manufactured-mass tuple arity.
Root cause: implementation draft and public formalization names diverged.
Resolution: align public contracts and tuple destructuring; targeted tests passed.

## F-003 — incomparable execution clocks

Symptom: dividing T1 full timing campaign by T4 one-pass suite suggested a spurious >300x speedup.
Root cause: one field represented two different durations.
Resolution: split `scientific_suite_wall_seconds` and `timing_campaign_wall_seconds`; add RED/GREEN regression.

## F-004 — clean review worktree compilation timeout

Symptom: clean detached review worktree recompilation of vendored `faer` exceeded tool command windows.
Root cause: full clean dependency rebuild with release/fat-LTO in the constrained runtime.
Disposition: independent static diff review completed; exact-HEAD cached release validation remains mandatory; clean archive build retried during final packaging with explicit receipt.
