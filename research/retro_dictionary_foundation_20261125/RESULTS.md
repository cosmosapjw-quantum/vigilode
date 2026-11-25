# Verified results (C9)
Core: 26 passed, 0 failed, 2 ignored. Integrators: 55 passed, 0 failed,
8 ignored. Facade: 20 passed, 0 failed, 0 ignored. Total 101 passed,
0 failed, 10 ignored; one new ignored timing case was separately executed
and passed. Other ignored cases and whole workspace are not asserted.
Feature on/off checks, rustfmt check, library Clippy -D warnings passed.
See raw final logs, earlier compile/lint failures retained, independent
audit JSONs and timing receipts. The final style-only edits were made
after the timing binary was measured, and final regression rerun.
No new full external solver measurements or production promotion.

Packed table (8192 points): 9 lanes 3.141/0.908/0.306 ms;
36 lanes 16.617/3.596/1.096 ms (individual scalar / packed /
4-thread packed). Release assembly has SSE2 mulpd/addpd.
Prepared RHS n128, 64 RHS: fresh factor per RHS 21.903 ms,
serial reused 17.141 ms, faer batch 16.815 ms, local Rayon 4.596 ms.
Prepared owner setup 116077 ns separate. Full original interval residual
included. Rayon wins on larger batches, loses small single RHS.
Aperiodic 256 transitions: blocked scan slower for n2/n8/n32; no
forced parallel adoption. All timings diagnostic on pinned host CPU0..3.
