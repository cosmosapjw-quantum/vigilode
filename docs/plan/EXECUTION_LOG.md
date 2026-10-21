# Execution log

| Node | Branch | Commit | Result |
|---|---|---|---|
| a1-defects | production/a1-defects | a8ec412 | 5 defects fixed with RED/GREEN tests; review clean after 1 fix round |
| a2-wfiltered | production/a2-wfiltered | 7bea78c | W-filtered interior control; interior 0.35 tol units on PR λ=-1e5; kill gate FAIL on step count (2.14x vs Off) |
| b1-lagged | production/b1-lagged | 16f6631 | Lagged-LU Richardson driver; forced refresh = fast driver; kill gate FAIL (HIRES 0.654, Bruss 0.854 fact/step) |
| t-verify | production/t-verify | f2250b7 | 6/6 theory claims verified exactly; ledger L-0137 |
| a1-api | production/a1-api | a605573 | Facade crate `vigilode`; review clean after 1 fix round |
| wave1-integration | production/wave1-integration | a5a48f7 | 5 nodes merged without conflicts; final review fixes; 1484 pass / 4 known allocation-counter failures (pass single-threaded) / 109 ignored |
| c1-transaction | production/c1-transaction | f07c4d7 | Transaction, Budget, AdmissionRule (T8), Telemetry; 20/20 tests; review clean |
| b1b-gmres | production/b1b-gmres | 16edef2 | Lagged LU as right preconditioner for GMRES stage solves; kill gate FAIL (HIRES error 3.75x; Brusselator steps 11.95x, error 11.9x); cause: no h/h_P staleness bound |
| p0-facade | production/p0-facade | a8b15bd | `Rodas5p::solve` facade: FD Jacobian, mass and dense routing, partial solutions, typed failure reasons |
| h-hygiene | production/h-hygiene | d41bf16 | Thread-local allocation counters; vigilode-coupled joins workspace; ledger L-0138 supersedes L-0124 |
| e1-bench | production/e1-bench | 387f87a | 13-arm comparator harness (CVODE, DVODE, RADAU5, RODAS, Diffsol); 102 rows; VigilODE rows grid-constrained (98–99 clipped steps), not a ranking; ledger L-0139 |
| h-checker | production/h-checker | 4f92ec3 | check-research-node checks a re-bound output only against the superseding row (user-approved) |
| w2-final-fix | production/w2-final-fix | 08fd052 | Final-review fixes: Enforce under zero error scale, banded docs, no discarded segments, L-0140 correction row, krylov allocation-test flake |
| wave2-integration | production/wave2-integration | 08fd052 | All wave-2 nodes merged; final review Important 1 + 5 Minors fixed (1 rejected, 1 deferred to wave 3); checker PASS; full-suite run delegated to workstation |
| wave3-base | production/wave3-integration | 75a0c6c | wave-2 integration 08fd052 + plan docs (pre-flight B0) |
| p0-atol | production/p0-atol | 997fefe | Per-component atol on fast dense/banded routes; uniform vector bit-identical to scalar |
| b1c-staleness | production/b1c-staleness | 129aa46 | T2 staleness bound q_pred + a-posteriori stop; kill gate FAIL (HIRES 0.588 fact/acc > 0.4; Brusselator PASS 0.244); ledger L-0141; lagged stays unrouted |
| p0-events | production/p0-events | f65d968 | Event location (Illinois, three samples per step), terminal events truncate the solution |
| ci-docs | production/ci-docs | eabe19d | production.yml pinned 1.94.1 (fmt, clippy --no-deps, tests per package, docs, semver on PRs, bench); four facade examples |
| p0-fastdense | production/p0-fastdense | 35d6604 | Dense output + W-filtered interior control on the fast drivers; facade dense output routes there; stats schema v3 |
| p0-api | production/p0-api | 4945683 | Typed `SolveError::Unsupported`, `#[non_exhaustive]` on facade enums and output structs (not Options/Atol) |
| w3-final-fix | production/w3-final-fix | 5c6978d | Final-review seam tests (uniform vector atol under dense output; terminal events on all dense routes) and doc fixes |
| wave3-integration | production/wave3-integration | 5c6978d | Wave-3 code complete; checker PASS (141 rows); full suites and E1b run delegated to workstation |
| e1b-dense | production/e1b-dense | 546333f | Facade dense-output arms (no clipped steps), preregistration, handoff at 33e60cf; awaiting workstation run and L-0142 |
