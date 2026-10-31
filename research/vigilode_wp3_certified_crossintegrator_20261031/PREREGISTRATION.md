# VigilODE WP3: prospective native RODAS5P work–precision experiment

Date: 2026-10-31 KST. This registration precedes the **new GitHub-hosted native RODAS5P full-IVP runs**, not earlier local SciPy runs, inherited WP1/WP2 measurements, or earlier symbolic experiments.

## Frozen authority and nonmutation
- Parent: Draft PR #160, immutable HEAD `cf690ad5ec723052e9eb6f71335f7cb3b43607a6`, tree `193681311d693afd3b1e601f5cf5fb2620c7e46f`.
- Original cooperative cubic source blob `d78c5c5f6f476f2a5b1ea88628ed25daa6cfc482`; original WP1 40 traces `SWEEP.log` SHA256 `9dae90079893170edddcf5cf8af193e53b43ab5bcf0c436cb81ce7aa0f8790b0`.
- WP1 original selected pairs `WP_SUMMARY.json` SHA256 `56746487ac5c5c8702b53ef0ffd47694c040ab4e0adcfea1ec25aa3b8e9ba909`; WP2 exact certificate under source-matched GitHub run 41186122032, certifier Git blob `98cb7cafe36fecb8850d55e9c81638c299484ab7`.
- No edits to original solver/source, production defaults, tolerances, shared CI policy, previous PRs, or original data. No merge/force push.

## Scientific question and protocol
Original `y_i'=-k_i y_i-g_i y_i^3+sum_{j!=i} c_ij(y_j-y_i)` on [0,0.1], original four frozen WP1 configurations: asymmetric_2 (64 output steps), asymmetric_2_refined (128), coupled_3_cubic (128), linear_exchange_control (128), with exact binary64 represented initial values/coefficient bits.

The native comparator is the repository's **original 8-stage fifth-order RODAS5P** via `integrate_sequential_matrix_free_adaptive_dense_observed`, using the same RHS and analytical JVP (derivative permission is explicit; not an RHS-only replacement). Output times exactly match the original WP1 binary64 time-label bits.

Preselected rtol = {1e-4,1e-6,1e-8,1e-10}, atol = 0.01*rtol, initial step = 0.1/N, max step = 0.1, max attempts = 100000. Three complete full-IVP timings per condition on the **same new CI runner**. Report every failure, accepted/rejected step, RHS/JVP/linear work, exact initial source and output bits, and per-grid trajectory bits. No reranking after seeing outcomes.

For each completed run compare exact dyadic state bits to the immutable WP2 micro64 reference. Use WP2 proven reference+clock upper epsilon. Report rigorous achieved peak-global error interval [max(0,eHat-epsilon),eHat+epsilon], separated from requested rtol. Work–precision fronts over predeclared caps {5e-5,1e-4,2e-4,4e-4,8e-4}. Comparison to WP1 wall time across **different CI runs** is diagnostic only unless machines, environment and run order matched, and BE/Rodas derivative information costs are disclosed.

## Negative/acceptance conditions
- Original source hashes and coefficient bits, WP1 log/summary hashes, WP2 certificate/reference hashes; coarse output labels and original initial vector exactly.
- Rust 1.94.1 locked Cargo; native API test and JVP finite-difference checks; no surrogate SciPy or single forced-accept N01 step masquerading as native full IVP.
- Preserve rejected/failed runs and negative linear control. No invented speedups, interpolation outside finite sweep, or production claim. Previous 8/8 same-author tests do not constitute native third-party C9.
- `OFFICIAL_VIGILODE_DAG=7/23`, `SUCCESSOR_ADMITTED=0/12`, `DISTINCT_AUTHOR_C9=0/1_NOT_RUN`, `PRODUCTION=HOLD`, `NO_MERGE=true`, `AUTO_NEXT_NODE=false`.

Earlier standalone exact-Fraction Bernstein tightening, OpenEvolve 0.4.0 offline candidate database, and SciPy 80 condition measurements were performed **before** this registration and remain explicitly retrospective/exploratory evidence; they are not prospective results under this document.
