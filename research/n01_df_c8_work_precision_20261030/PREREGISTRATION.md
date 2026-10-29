# C8 source-locked preregistration: native work–precision, stability and conditional original raw-U stage bounds

Date: 2026-10-30 KST; source parent PR123 verified HEAD `2d3d1ef3dce11fbc3c9168bfc69ab16240e40dc6`. This file is created BEFORE any GitHub-hosted native C8 experiment. Independent external SciPy Radau/BDF reference computations and Wolfram symbolic l1-lognorm check have already been performed in the current session and are *not retrospectively preregistered*. Source and evidence remain separately typed.

## Selected stage C8 ONLY
Preserve causal parallelized **derivative-free-first** Rosenbrock/Runge–Kutta augmented Newton-free homotopy (PR91/117/119). Compare **same physical Robertson ODE** for (i) original eight-stage native RODAS5P sequential 3x3 direct reference comparator, and (ii) existing `N01ValueStage` value-only RHS-homotopy at successive implicit-Euler stages. Use matched achieved physical endpoint error in natural l1 and retain counts (RHS, J builds, factorizations, linear solves, map attempted/completed, rejected/failed callbacks, certificate applications). No broad wall speed claim; small direct reference is not the production algorithm. Test initial y=(3/4,1/8,1/8) with rates & T:
- mild k=(0.5,1,0.5), T=0.1, RODAS n=[8,16,32,64,128], RHS-only BE n=[8,16,32,64,128,256,512].
- moderate k=(0.5,10,200), T=0.02, same grids.
- extremely stiff negative control k=(0.04,10000,30000000), T=0.001, n=[8,16,32,64], count all positive/source/step failures. Stop bounded attempts, do not alter tolerances.
Native RODAS fixed-step comparator sets force_accept=true, **not physical adaptive acceptance**; explicitly label. A failed method/domain is a real negative row, not silently dropped.
Independent SciPy 1.?? Radau/BDF each rtol=5e-13 atol=1e-14 references: first diagnostics observed cross differences mild 2.458e-13, moderate 1.725e-12, stiff 4.626e-13. Their agreement is NOT a certified global IVP bound.

## Critical scientific risk and positive C8 construction
Robertson Jacobian is NOT a Metzler generator: negative offdiagonal J[1,2]=-k2*b for b>0. Exact `mu_1(J)=2*k2*b`, though 1^T J=0. Wolfram exact resolvent counterexample k=(1/2,1,1/2), b=c=1/4, tau=2: ||(I-tau J)^-1||1=19/15>1. Do not claim unconditional l1 contractivity.

The one source-owned conditional, derivative-free *certificate path* is:
- on a defined positive tube with all source rates fixed, `||J(z)||_1 <= B=2 max(k1,(k2+2k3)M_tube)` for nonnegative b,c and sum z <=M_tube;
- `tau B<1` yields Neumann resolvent bound `kappa <=1/(1-tau B)` for `A=I-tau J(y)`, not a necessity condition;
- independently outward-enclose all original 8-stage raw-U residuals from represented coefficients/stages/source evaluation, where exact represented float coefficient values define the target;
- if source-domain tube contains each true stage argument, set `d_i=kappa*rho_i`, `L_ij=kappa(|gamma C_ij|+tau B |A_ij|)` for j<i and `e=(I+L+...+L^7)d`, finite triangular recursion; refuse if a tube cannot be established;
- output algebraic error <=sum_i |b_code_i| e_i in natural l1 plus outward endpoint accumulation roundoff. This is **discrete algebraic stage error**, not global time truncation/whole-slab event/observational guarantee.

The source actual Jacobian and time ownership for *full physical BASS33* is not supplied by this Robertson test. The interval raw residual alone is scoped; no unmatched physical tolerance promotion.

## Execution/negative controls and limits
Build a fresh additive test-only native Rust C8 module, not solver modification. Execute on actual GitHub checkout / Rust1.94.1, strict Clippy and focused fmt, original C7 tests, independent external reference report. Do not require impossible success for stiff control; record rejection reason. Record explicit 1-norm counterexample and invalid rate/domain/stale epoch where relevant. Use 1 bounded numerical campaign and one targeted repair when failure diagnosed; keep all RED logs. No automatic start C9/next loops.
DAG inherited: VigilODE 7/23, REI 28/88, successor 0/12 admitted, N01 C6 1/1 scoped, C7 1/1 scoped, C8 0/1 before results, production HOLD. N02 deferred, main untouched, no force push.
