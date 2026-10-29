# N00F native-source audit: flexible right GMRES soundness and trigger economics

**Date:** 2026-10-29. **Status:** PASS_SCOPED_AUDIT; native integration NOT_RUN; production HOLD. This is a post-execution report. True preregistration preceded the first Rust experiment at commit `4939cf8460171c7a2e07a7c1e58147e471488174`.

## Authority/source and strongest finding
Parent PR109 head `2d430a7aa81070da17a7a5dba60a2d56639b82db`. Native PR104 source commit `10854bc0bff81b6d3cf5cb0a44520ed9165c205f`, `crates/rodas5p-krylov/src/gmres_into.rs` Git blob `90bfaaa62afeda9dec27b3dfd594e67cd3034da1` (identical in PR106).
At lines 699–720 the existing right-monitored method backsolves a least-squares prefix and reapplies the right preconditioner to combined `V_m y` after every Arnoldi column; at lines 733–735 only monitor Accept commits output. Preconditioner linearity/stationarity is not enforced by the generic trait. Consequently `P^-1(Vy)=Zy` is not justified without a fixed linear P. A wrapper cannot save the expensive candidate work; triggering must occur inside native Arnoldi before backsolve.

## Theory and independent kill example
Known FGMRES relation: `z_j=P_j^-1 v_j`, `A Z_m=V_(m+1) Hbar_m`, candidate `x=x0+Z_m y`, store Z columns O(nm); projected residual only signals when to build a candidate, original residual is the authority. Literature: Y. Saad (1993), *A Flexible Inner-Outer Preconditioned GMRES Algorithm*, SIAM J Sci Comput 14, 461–469, DOI [10.1137/0914028](https://doi.org/10.1137/0914028).

Exact rationals: A=I, b=2e1, P(z)=(1+||z||²)z/2. First Arnoldi v=e1, z=e1, Hbar=(1,0)^T, projected residual=0 and y=2. Naive P(2e1)=5e1 gives original residual 3; flexible Z*y=2e1 gives 0. An independently checking monitor rejects the naive candidate, so this does NOT demonstrate a false native acceptance. It demonstrates missing preconditioner contract and possible work/convergence failure.

Point-matrix weighted nonnormal check: J=[[-1,10],[0,-1]], w=(100,1), sigma=1/4, mu_w=-9/10, resolvent bound 40/49, exact weighted inverse norm 102/125. Physical p=(1,1) conversion costs factor 100. Native WRMS/L2 `residual_scale` is NOT the weighted infinity theorem norm. Whole-slab source, JVP and eta enclosures remain unverified.

## Actual Rust coding/benchmark evidence
No-dependency Rust1.94.1 lab, not a native repo patch. Six tests PASS, strict Clippy PASS after one preserved style failure, fmt PASS. Independent Python exact `Fraction.from_float` audit of 208 represented binary64 (A,b,x) triples: 208/208 original residual gates PASS. Original scientific test is scoped to diagonal linear systems and is not an outward-certified physical-JVP evaluation.

One of three permitted performance loops used. Seven interleaved paired ABBA/BAAB blocks per size, 84 timed solves total, same diagonal systems, RHS, tol=1e-9, same output bits/Arnoldi columns/achieved error. Both arms use *the same* MGS2 and incremental Givens, so this comparison isolates only per-prefix candidate/backsolve frequency. It is not native GMRES, full-IVP, or native-faer baseline.

| n | columns | always→trigger candidate/backsolve checks | median always/trigger | observed min/max | maximum achieved component error |
|---:|---:|---:|---:|---:|---:|
|16|16|16→1|1.4359|1.4118/1.4439|4.44e-16|
|64|35|35→1|1.3434|1.0310/1.7860|8.69e-10|
|128|39|39→1|1.4491|0.9002/1.9972|2.13e-9|

At n=128 one paired block is slower, retain as negative evidence. Error bars in plot (inside ZIP) show block extrema, not confidence intervals. CPU is a virtualized Intel Xeon Platinum 8573C with five online logical CPUs. Results cannot support generalized speedup.

## Audit and DAG
PHYS-MATH: exact nonlinear counterexample PASS; FGMRES relation literature-supported; weighted point resolvent PASS under premises; actual physical stage/source owner OPEN.
PHYS-MATH-CODE: native lines directly inspected; isolated Rust6/6 PASS; Fraction208/208 PASS; no native patch/test/performance. Native N01 source-owner point-J/callback/clock/norm/cert bound, native N02 opt-in stored-Z or fixed-linear trait, N03 opaque certificate, N04 whole-slab symbolic cell, N05 adaptive IVP remain OPEN.
Current inherited counts: original VigilODE **7/23**, central REI **28/88**, PREFIX01 **0/1**, PREFIX02 **0/1**, proposed successor **0/12 admitted**. Only separate audit N00F **1/1 scoped**. R6/C9 distinct-reviewer NOT_RUN. No main merge/force push/tolerance change, production HOLD.

## Immutable evidence
Original archive: **77,957 bytes**, SHA-256 `c05bf14dc5ca3916710663caa77608eb37da2946d24c1b9bd453277a58437d2b`, 22 members, ZIP CRC PASS. Full Rust src/tests/probe, original CSV, independent oracle, Python script, logs, source hashes, plot, DAG and handoff:
- [Google Drive source ZIP, byte hash readback verified](https://drive.google.com/file/d/1PpiLA4it9973yLNWdlyGBCcGnixoTQpi/view?usp=drivesdk)
- Dropbox immutable path: `/BASS_DERIVATION_DOSSIERS_20260912/VIGILODE_N00F_20261029_EVIDENCE.zip`, completed ACK id `id:BSpOijBcT10AAAAAAEHVYQ` size77957; raw independent Dropbox redownload NOT_RUN.
Additional executable source is in the ZIP, not separately claimed as GitHub-native code.

**Next permissible action:** native N01 source-owner residual/stage-error physical certificate, before any N02 integration or performance claim.