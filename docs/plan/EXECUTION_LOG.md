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
