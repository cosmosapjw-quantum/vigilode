# Final preflight handoff
Owner request: remove redundant worktrees, attach mathharness-general-3.1.0,
execute three bounded programming audit/fix cycles, prepare M0 only.

## Read first
BASE.txt and loop1/HANDOFF.md, loop2/HANDOFF.md, loop3/HANDOFF.md.
m0/M0_INPUT_LOCK.json and m0/PROTOCOL.md are the next work package.
Do not interpret old or new smoke results as final scientific qualification.

## Reuse
- Existing active checkout: /home/cosmosapjw/worktrees/vigilode-csas-20261125
- Local preserved root: /home/cosmosapjw/vigilode-preserved/20261125-final-preflight
- Original harness archive pinned in HARNESS_SHA256.txt; protected kernel
  untouched. Actual exact ledger 90 entries, fresh verifier and negative
  controls in EXACT_CHECKS.json, exact_ledger.json.
- run_experiments.py defaults to planning only. Use --execute-smoke,
  --harness-zip /home/cosmosapjw/Dropbox/mathharness-general-3.1.0.zip,
  --harness-root /home/cosmosapjw/vigilode-preserved/20261125-final-preflight/harness/mathharness-general-3.1.0,
  --output a new repo-relative directory. Optional repeated --topic M1..M5.
  The allowlisted smoke tests are in crates/vigilode/tests/retro_preflight_combinations.rs.
- 33 obsolete checkouts removed after preserving commit identities and
  cache member checks. 12 remaining include dirty or active work and a
  separate current production track. Do not delete them by assumption.
  Archive refs, bundle prerequisites, cache archive hashes and actual removal
  log are retained. Bundle verification is not a claim that every old
  worktree was reconstructed and rerun.

## Scope / open items
No full workspace regression, external solver performance rerun, general
nonlinear DAE/event/aperiodic qualification, complex outward transcendental
certificate, or M0 new reference trajectory. Terminal event is still
postprocessing and tangent detection is not general. M0 historical disputes
remain 12/14. Native subagent / Superpowers execution not exposed; direct
audit used. Formal external theorem prover availability not assumed (stale
Isabelle wrapper, missing Lean/Rocq/Z3); exact in-house proof kernel verified.
Separate shared and standalone benchmark Cargo targets are retained to avoid
rebuilding before M0; no other project caches touched.
