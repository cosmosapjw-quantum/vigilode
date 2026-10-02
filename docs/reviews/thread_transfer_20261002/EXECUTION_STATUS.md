# Thread-transfer DAG: execution status

This file records how `NEXT_DEVELOPMENT_DAG.json` was executed on this branch. Each native node got its own research
node under `research/`, preregistered and pushed before its code and runs, with its ledger row in
`research/LEDGER.jsonl`. Old ledger rows and verdicts are unchanged: L-0017/L-0024 (R4 homotopy cost) and L-0033
(fast v2) stay FAIL, and timing authority stays on HOLD.

| DAG node | Research node | Ledger | Verdict | Finding |
|---|---|---|---|---|
| P0-PREREG | every node below | - | done | each node preregistered and pushed before its code and runs |
| P0-RADIUS-REPLAY | `thread_transfer_radius_replay_20261002` | L-0034 | PASS | natively, the six-attempt schedule still fails at n = 8 and 16; 7 and 8 attempts close them; full = blocked bit for bit; within 6e-15 of the review's exact reference |
| P1-PATH-ACTION | `thread_transfer_path_action_20261002` | L-0035 | PASS | action-first path sum is exact on integer blocks, encloses the exact stage roots, and takes 0.565x the counted operations of the matrix order |
| P1-RADIUS-PROPOSAL | `thread_transfer_radius_proposal_20261002` | L-0036 | PASS | the residual-seeded radius `2 max B E(0)` closes every R4 fixture in one preflight and one check; the causal box closes the scalar no-go system |
| P0-MF-TARGET | `thread_transfer_mf_target_20261002` | L-0037 | PASS | `r_U = gamma r_K` holds symbolically; the raw U and native K targets differ by 1e-16 to 1e-14 coefficient terms, enclosed exactly |
| P1-MF-WORKSPACE | `thread_transfer_mf_workspace_20261002` | L-0038 | **FAIL** | RHS-assembly JVPs drop 7 -> 0 and semantics hold, but the preregistered one-step tolerances, GCRO-DR on the Brusselator (it fails in every driver) and the allocation target (0.32-1.03x, not <= 0.5x) fail |
| P1-LAGUERRE-CERT | `thread_transfer_laguerre_adjoint_20261002` | L-0039 | PASS | the signed adjoint bound is exact-checked and 7.35x to 7.6e13x tighter than the R4 majorant; one component, total stays `EstimateOnly` |
| P1-NEGATIVE-CONTROLS | `thread_transfer_negative_controls_20261002` | L-0040 | PASS | the RVJ5 counterexamples reproduce; RODAS5P, EXPRB43 and PEXPRB54S4 share neither on these problems |
| P1-SMALLN-COST | `thread_transfer_smalln_cost_20261002` | L-0041 | PASS | compile-time dimension and static dispatch, same results as v2, 0.39-0.60x of its instructions per attempt |
| P2-LAGUERRE-CACHE | `thread_transfer_laguerre_cache_20261002` | L-0042 | PASS | the keyed envelope cache is safe; setup costs tens to hundreds of small actions; the same-vector combination gains nothing on same-sign coefficients |
| P2-CHART-CONTRACT | `thread_transfer_chart_contract_20261002` | L-0043 | PASS | the ratio-chart identity is proved and the semilinear chart's domain, branch, fast-mode and finite-eps conditions are checked (model-specific) |
| P2-TIMING-GATE | - | - | **not run** | stop condition holds: timing authority is still on HOLD, so no timing campaign was started |

Found and fixed on the way (not a DAG node): faer 0.24.4's `Mat::generalized_eigen` under-sizes its scratch for
small pencils with a complex eigenvalue pair and panics, which aborts a release binary. GCRO-DR's harmonic Ritz step
hit it on the Brusselator. `crates/rodas5p-krylov/src/small.rs` now calls `gevd_real` with the missing scratch, and
a regression test covers the 2x2 pencil that triggered it.

## Blockers of `BLOCKERS.json`, after execution

- **B-HOM-RADIUS**: diagnosed natively (L-0034). A residual-seeded radius and a causal box close the fixtures
  (L-0036); the old gate stays FAIL.
- **B-HOM-COST**: still open. Action-first halves the counted operations (L-0035), but the serial certificate
  remains cheapest by count, and the cost margin `1 + p1 - 8 pf` was not re-estimated.
- **B-MF-WORKSPACE**: partly addressed. The JVP removal works (L-0038), but the node FAILs as preregistered, and
  Krylov-side allocations dominate.
- **B-LAGUERRE**: one proved component (L-0039, L-0042). The total is still `EstimateOnly`.
- **B-LATEST-SMALLN**: L-0033 stays FAIL. A separate specialization meets its own threshold (L-0041).
- **B-PHYSICAL-ERROR**, **B-CHART**: negative controls and the chart contract are model- and method-specific
  (L-0040, L-0043); no stage certificate or embedded proxy is called an ODE error bound.
- **B-TIMING**: HOLD unchanged; P2-TIMING-GATE not run.
- **B-NATIVE-RUNTIME**: closed for this branch. Every node above ran natively (Rust 1.94.1), and the repository's
  validation matrix ran on the final head.
- **B-PUBLISH**: closed. Everything is on `claude/jolly-wozniak-7wl15h-wu25-stiff-benchmark`, pushed without force.

## Validation of the final head

The repository's validation matrix ran on `c6dfd3e`. All 14 steps exited 0: fmt; clippy (default and three feature
sets); workspace tests; three feature-gated test runs; ignored tests in the measurement profile; readiness;
research-node and authority checks; and ignored-tests reachability. Across all runs, 2,291 tests passed and none
failed. The ignored fixture writers regenerated every fixture byte for byte.
