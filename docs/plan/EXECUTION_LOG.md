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
