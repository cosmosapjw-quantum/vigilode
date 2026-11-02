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


## 2026-11-01 successor loop: original full 26-event performance profiling and owner-exact shadow (R5, R6, R7, C2, C5-C9)

### New live sources and interpretation

GitHub VigilODE PR181 `ac09919690ba148cb5e12be37d3926a18ae1aab9` fixes the original 15-state five-clock nonautonomous time derivative, without changing original full 26-event physics. Its opt-in module and PR180 test-literal repair are now additive in this same isolation branch. Existing generic G0 PR182 `8fd2ac31d65df5e70e3bbb266d51b397562edfb5` measured original protected vs transactional full-IVPs at matched achieved error; transactional is slower in its source scope and must not replace protected default. REI PR354 owner V2 gate remains independently shadow-only. REI PR355 photon-moment/Volterra theory is a separate evolving draft and is NOT imported as proven full-X source physics.

### R5 direct mathematical improvement

The ORIGINAL 26-event thermal implicit root uses
`F(x)=ctx.ecoef*exp(x)-U0-dt*(constant+weight*thermal_rhs(ctx,x))`
with interval signs at `lo` and `hi`. In the original exact algorithm, a bracket-expansion loop computes both `F(lo)` and `F(hi)`, and immediately after the loop computes both **again**, even on a branch where the loop has already broken because no expansion is needed. On that branch `lo,hi` are unchanged and interval arithmetic is deterministic; reusing the last two results is algebraically and bitwise equivalent. When all 20 expansion attempts are exhausted, the endpoints may have changed on the last iteration; the candidate **does** recompute then via Python `for...else`. Sign-bracket, interval Newton, strictly positive derivative check, max iterations, all outward arithmetic, and original source methods remain unchanged. This is a narrowly specified shadow candidate; the REI owner module on disk is untouched.

The original `thermal_rhs` is evaluated at 46,080 real H/He source cells with source-original state tick 160. This optimization does **not** replace the original MPRK22(1), Alexander SDIRK2, 26-event table structure, temperatures, OTS, 14 ledgers, or admission policy. The original source/archive/state/kernel SHAs are checked before every run.

### R7/C8 original-owner measured science

Read-only complete `run_step` profile for real full/half1/half2 maps took 5.261658 / 5.147795 / 5.065139 s on the connected Linux host, all source-native PASS. The full-map `thermal_root` cumulative cProfile cost was 3.673 s of 5.262 s; `_up` and `_down` cumulative self times were 1.239 and 1.196 s respectively. These are nested, not additive, timing attributions. The full native run retained 14/14 ledger zero inclusion and exactly reproduced historical independent-output logT 2.1245050576368385e-4 > frozen 2e-4 FAIL. `ORIGINAL26_OWNER_MAP_PROFILE.json` records actual hot paths, not generic solver theory.

Source-exact four-arm ABBA baseline/shadow comparison, each arm **actual full+half1+half2 46k original owner**:
- A0 original: 15.565384 s, 90 `thermal_rhs` evaluations.
- B1 shadow: 14.506656 s, 72 evaluations.
- B2 shadow: 14.722374 s, 72 evaluations.
- A3 original: 15.274416 s, 90 evaluations.

All 4 original source state signatures, concatenating BOTH directed interval endpoints of all five population species plus logT at each of 3 map outputs, are byte-exact identical SHA256 `fe260af02b4ad84ae4c90ed0066e42713ad056a7582d33b0708e5eb6d4b6c6cc`. All 4 have original 14/14 ledgers, 0 table events, historical crossbox failure still > 2e-4, and no accepted history mutation. Median original / median shadow wall ratio 1.0551085595375107 (~5.5% source-host diagnostic speedup). Actual thermal_rhs work reduced 90 -> 72 (20% on original full/half1/half2). `ORIGINAL26_ROOT_ABBA.json` preserves all measurements. No general CPU/HPC speedup or source-owner production certification is inferred.

### Native VigilODE method integration and negative controls

The existing protected native RODAS5P five-clock test-literal repair PR180 and EXACT additive PR181 nonauton opt-in `rei_igm_five_clock_source/nonauton_study.rs` were reused, with no generic solver-core mutation. Rust offline locked original source native example 6/6 PASS, and opt-in original-source nonautonomous 8/8 source-scoped routes PASS plus one posthoc invalid legacy control. The corrected source-only 15-state model does not turn into the original complete 26-event H/He/OTS/thermal IVP. Native replay saved as `NATIVE_PR181_NONAUTON_REPLAY.json`.

### Same-author blind-referee review, one finite pass

1. **BLOCK production:** Original source-owner accepted 160/131072 tick remains unchanged; independent-output logT 2.124505e-4 exceeds 2e-4. Existing V2 correlated-discrete shadow acceptance is a different, not yet official science predicate. No automatic tick161 acceptance.
2. **BLOCK broad 26-event optimization claim:** This is an opt-in shadow replacement of one interval thermal-root call site, not a production worker source edit. One real initial condition, one source-first tick and one Linux host were checked. Pathological 20-expansion states and all broader tables still require independent owner validation.
3. **BLOCK full JVP-admission:** Prior group fusion slower for RHS; stage-cached photon JVP only helps repeated frozen linearizations. Float64 He II underflow remains numerically material. Rank <=3 is photon partial only, not full-X.
4. **HOLD general time-forcing promotion:** PR181 corrects five-clock *reduced* 15-state forcing and keeps explicit segment boundaries. Do not silently transport to full 26-event owner or claim original full source `f_t` is known.
5. **OBSERVED runtime risk:** Optional NumPy-1-compiled `numexpr`/`bottleneck` imports show ABI warnings under NumPy 2.4.2 in this environment. The original source-native processes nevertheless exited 0 and produced exact matching evidence; full clean pinned-environment replay is a separate unrun validation.
6. **Independence:** Same-author experimental construction and hostile review, no native distinct-author subagent; Mathematica algebra checks and physical byte comparison have separate but limited independence. No fake independent referee certification.

### Drift watchdog / official counters

The original cProfile decisively moved this effort from small per-JVP Python optimizations to the original full 26-event runtime thermal bottleneck. The 5.5% real-map shadow saving and exactly preserved physics count as scoped optimization research progress, **not** canonical REI DAG node completion. Stop new photon atlases, process-only gates or alternate surrogate benchmarks. Next real scientific action is owner-controlled, versioned, opt-in thermal bracket optimization with full original 3-lane intervals/event/ledger parity, clean pinned host replay and original science-policy decision. This branch remains independent research, no merge/default/force-push/tolerance changes.

DAG at every stage: VigilODE OFFICIAL 7/23; VigilODE SUCCESSOR 0/12; REI CENTRAL 28/88; REI WORKING 33/88; REI FAST 4/7; original accepted 160/131072; CANONICAL_DELTA=0. GitHub Draft PR #183 is the cumulative publication lane. Full original 26-event owner used ONLY as read-only source. PRODUCTION=HOLD.
