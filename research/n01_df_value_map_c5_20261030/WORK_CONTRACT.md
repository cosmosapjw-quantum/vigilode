# N01 derivative-free C5: post-execution method and source contract

Date: 2026-10-30 KST. Source parent PR117 HEAD `0032b7bd7a0d5092eac26ad9408b9a261df2916e` (Draft, unmerged). **This note is posted after the first local implementation and tests, NOT preregistration.** Earlier GitHub branch-create attempts failed during tool service issues; do not backdate the Git commit.

Selected research stage: C5 source-owned Robertson diagonal implicit-RK stage bridge, reusing PR91 `rhs_homotopy::RhsOnlyMap` and `solve_rhs_homotopy`. PR117 R5 theorem and its original 10,652-byte SHA256 `3dc8cfa72372f4885a14a6cfd00ca98a82eea2b7e16e6b1cb779e9fafeb9191b` ZIP reverified byte/CRC.

Mathematical target `z=y+tau*f(z)` for mass-action Robertson with nonnegative y,k,tau. It is a **single diagonal RK/BE stage**, not original eight-stage RODAS5P raw-U. Source supplies immutable rates/state/w/p under N01 opaque trial+epoch; callbacks read-only without `advance()`. Derivative-free scalar map `G(b)=clamp(b-F(b)/L,[0,M_upper])`, `L>=sup F'(b)`; source-owned outward interval physical stage original residual and p-error upper must both pass before output mutation. Inner homotopy residual alone has no scientific admission authority.

C5 tests executed in actual current-turn local Rust 1.94.1 **interface mirror** (NOT full native workspace): 10/10 pass, strict clippy, fmt; independent Python Fraction-from-f64 of 256 source/candidate/bound rows: 1280/1280 checks pass. Initial Rust literal syntax, shim clippy and test-style failures preserved. An identified swallowed certificate exception was corrected by an explicit `certificate_error` field and tests rerun. No benchmark loops used.

Publication intends an additive core trial-bound source-input accessor, additive `vigilode-coupled` derivative-free bridge source and one opt-in `lib.rs` export only. No existing GMRES, default routing, warm starts, caps, precision or rate assumptions modified. Real native full-workspace compilation is NOT_RUN, independent different-author R6/C9 NOT_RUN, matched original-8-stage full-IVP NOT_RUN, production HOLD.

DAG (post local C5, before native integration): VigilODE 7/23, REI 28/88, proposed successor 0/12 admitted, N00F 1/1 scoped, N01_DF_C5 1/1 *interface-mirror scoped only*, N01_NATIVE_ADMITTED 0/1.
