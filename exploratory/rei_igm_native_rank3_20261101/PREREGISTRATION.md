# Pre-registered native Rust original REI IGM F05 photon rank-three operator

Date 2026-11-01; exact VigilODE base `5593edcceb5b35ee6f55273afecb553eeb2f1008`; PR #183 is the **only** intended GitHub PR lane. Distinct clean worktree protects old PR183 worktree's uncommitted original Python shadow investigation. No new production worker or core default mutation.

Scientific object: the ORIGINAL `rei_bianchi` F05 tick160 accepted 46,080-cell H/He/OTS source dataset, four photon groups, three absorbing moments, frozen kernel `579df3cc99987dcce9205166752570bdaf4c7cb3640c525be6da260d94b8e644`, source TAR `a861278201313c55e08ba6323b5c1d2ad97bf5765f429807b4eba0a1c2465d0b`, accepted numerical array `cc031a04f5b7b13ccb9ffddf95ec0ced4a9274a9ab999a9e7e3508a5161edb01`, owner read-only Git `fcc90939b6b7cc842fae65d5a0ffbb69a3ec3120`.

Exact real-arithmetic photon rate (one fixed time, original source):
`D_g=e_g+sum_s a_gs N_s`, `N_s=sum_i n_is`, `P_is=sum_g J_g n_is a_gs/D_g`.
`dP_is[v]=kappa_s v_is-n_is sum_t B_st (sum_j v_jt)`,
`kappa_s=sum_g J_g a_gs/D_g`, `B_st=sum_g J_g a_gs a_gt/D_g^2`.
Nonlocal rank <=3 is **photon-source partial operator only**; it is not the full 26-event all-X Jacobian, OTS or thermal closure. Preserve original HeII nonzero longdouble trace even when f64 photon rate underflows. No declaration of exact physical zero.

User-corrected implementation: Rust for VigilODE native method, Rust for original REI microphysics (restored REI PR#360), Python may ONLY export immutable original source-owner input/interval fixtures and independently verify, not be used as production solver or be credited with Rust performance. Existing reduced 15-state RODAS5P experiments remain source-specific, not a substitute for original 26-event coupling.

This bounded science unit:
1. Extract original 46,080-cell n, original 4-group J/e/alpha/excess and original owner stage-0 strict interval boxes to a reproducible fixed-identity *input fixture*, without any original owner mutation. Source must be checked by SHA and original owner exact HEAD; store fixture archive SHA.
2. Implement a truly native Rust matrix-free JVP baseline vs rank-3 fixed-state cached coefficients in an **opt-in** VigilODE example; fixed-state cache invalidation and fail-closed shapes. Reuse original RODAS/Krylov infrastructure where possible without changing default solver.
3. Measured repeated native Rust JVP work vs physically identical native group-rule baseline, including setup, fixed source and all 46k cells. Validate source flux and heat against ORIGINAL owner intervals (3 species × 46k, heat 46k), photon budget, analytic JVP vs source-finite difference, negative invalid inputs and source hashes. Do not claim full native 26-event IVP wall speedup. HeII f64 underflow marked HOLD, preserve longdouble source trace/at least log10 positive flux.
4. Only if valid, publish one result on existing Draft PR #183 (no PR factory, no main merge, no force push) with local backup and exact remote SHA readback. If any case fails, retain negative result, no promotion.

Current official DAG: VigilODE 7/23, successor 0/12; REI central 28/88, working 33/88, physical fast 4/7; canonical delta 0, original accepted tick160/131072 only, production HOLD, no threshold/default relaxation. This study is a method-client support, not a new canonical science node.
