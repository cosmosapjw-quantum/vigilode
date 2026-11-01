# Original REI photon group fusion and fixed-stage JVP reuse (2026-11-01)

## Scientific question and ownership

Can the existing VigilODE matrix-free/Rosenbrock JVP workload be reduced for the **actual original REI IGM 46,080-cell, four-group, three-neutral-moment photon partial operator** without altering owner physics, tolerances or accepting tick161? Original owner: `cosmosapjw-quantum/rei_bianchi`. Method/optimization owner: `cosmosapjw-quantum/vigilode`.

Source anchors: VigilODE PR178 `ab52c102cb7d30e1e0f030d05a9a04950001a494`, PR175 `d99c3f06d46e233a7848e5b4562b4a59e2b5d900`; owner REI PR348 `fcc90939b6b7cc842fae65d5a0ffbb69a3ec3120`; PR354 is newer original scientific shadow input and is NOT substituted silently. Parent original accepted tick 160/131072; immutable original archive, state and kernel SHA256 are recorded in the generated receipt. The original source is imported read-only and fail-closed by SHA check.

## Direct derivation (R5)

At a fixed source time and state, the group-shared source is `P[i,s]=sum_g J[g]*a[g,s]*n[i,s]/D[g]`, where `D[g]=e[g]+sum_t a[g,t]*N[t]`, `N[t]=sum_i n[i,t]`. Define
`kappa[s]=sum_g J[g]*a[g,s]/D[g]`,
`q[s]=EV_ERG*sum_g excess[s,g]*J[g]*a[g,s]/D[g]` and
`B[s,t]=sum_g J[g]*a[g,s]*a[g,t]/D[g]^2`.
Then `P[i,s]=n[i,s]*kappa[s]`, `heat[i]=sum_s n[i,s]*q[s]`; differentiating at fixed source gives
`(dP.v)[i,s]=kappa[s]*v[i,s]-n[i,s]*sum_t B[s,t]*sum_j v[j,t]`.
This factorization has a nonlocal three-moment rank upper bound 3, independent of 46,080. Under nonnegative original source weights, B is positive semidefinite (in exact real arithmetic). This is a structural claim about the **stage-0 photon source partial only**. It is not the original full all-X Jacobian, a thermal/OTS closure, or a 26-event map defect certificate.

Reusing `kappa,B` across **Krylov directions at exactly the same (state,time,source)** is valid: W is linearized at fixed state. The new `FrozenPhotonJacobian` makes its own immutable state snapshot. Construct a new instance at each changed stage/time/source, and reject invalid directions. This implementation is exploratory Python, NOT a native RODAS hook or an accepted REI solver replacement.

## Observed computation (R7 / C5-C8)

Original owner REAL 46,080-cell primary shape lane evaluated with original source code (not a 46k surrogate), original 26-event SHA locks, plus source-only counterexample retention. Whole-owner interval inclusion: H I/He I/He II each 46,080/46,080, heat 46,080/46,080. Photon budget and JVP checks passed unchanged. He II is strictly positive at extended precision and underflows to f64; this remains a physical-numerical blocker for claiming full source equivalence.

First hypothesis **fuse entire RHS** was experimentally slower: baseline 1.85320 ms median, fused 2.41098 ms, speed ratio 0.769. The original regression remains in `GROUP_FUSION_FIRST_RUN.json` and new `ORIGINAL26_GROUP_FUSION_CACHED_RESULT.json`. No positive RHS optimization admission.

Second hypothesis **same-stage coefficient hoisting**: repeated JVP baseline 3.18365 ms, fresh fused JVP 2.78604 ms, cached JVP apply 1.64173 ms, cache setup 1.26461 ms; measured ~1.939x ratio per repeated JVP (excluding setup), estimated one-call break-even by median timings for this frozen 46k input. Cached relative JVP mismatch 2.19345e-16 vs previous source-preserving hybrid. Measured timings are one-host exploratory, not standalone native solver time. Cache ownership and call pattern must be tested in Rust and in the original full solver before admission.

Python unittest: normal 11/11; -O 11/11. Reused exact PR180 Rust *test literal repair only* (no solver code changes); native five-clock example 4/4 and native four-route example 5/5. This tests the existing source-authentic 15-state **photo-only** two-arm and four route examples, not 26-event full coupled physics. Original PR178/PR180 GitHub production/long/GMRES/Audit2 workflow failures remain. No new production claim.

## Blind-referee, same-author (C9)

1. **Scope obstruction:** full source includes H/He thermal roots, OTS, 26 events, temporal forcing, global energy and photon ledgers. Rank-3 result does not close full-X step Jacobian or guarantee the original full method stability. Severity BLOCK to production promotion, not source-only research.
2. **Precision obstruction:** group summation changes f64 rounding. Strict original owner interval inclusion on 46,080 cells was measured, but this is a single physical source stage. He II physical extended-precision trace remains below binary64. Retain all the exact raw-source warnings.
3. **Stale Jacobian:** cached B/kappa must not be reused when stage state or source clock changes. Snapshotting avoids external in-place buffer mutation, but integration lifecycle still requires independent review and invalidation tests.
4. **Performance obstruction:** source RHS is slower and repeated JVP speedup is not an IVP speedup. Need matched achieved *global physical error* with full 26-event source to claim method-level acceleration, including factor setup, Krylov counts, fallback and wall time.
5. **Independence limitation:** testing and this self-adversarial review are same-author. Wolfram exact finite-dimensional Woodbury identity supplies auxiliary algebra only; no distinct-author physical referee.

## Verdict and drift watchdog

This unit produces a source-scoped exact algebraic simplification, a measured repeat-JVP optimization, an explicit negative source-RHS performance result, and conserved original interval/source properties. It does **not** increase canonical scientific DAG progress. An additional source-only optimization atlas would be process drift. Positive next step: adapt existing VigilODE fixed-Jacobian matrix-free GMRES or low_rank_bordered preconditioner to **one original full 26-event shadow step**, with frozen inputs and tested fallback, then compare original full/half1/half2 states, 14 ledgers, and unchanged 2e-4/2e-3 gates. Only source-exact/physical-data progress can justify DAG promotion.

Official counts unchanged: VigilODE 7/23, successor 0/12; REI central 28/88, working 33/88, physical fast 4/7; accepted tick160/131072; canonical delta 0. NO_MERGE; NO_FORCE_PUSH; NO_TOLERANCE_RELAXATION; PRODUCTION HOLD.
