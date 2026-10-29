# Copyable N01-R -> native VigilODE continuation
$physmath-research-loop $scispace $exa $wolfram
PROJECT=cosmosapjw-quantum/vigilode
PR106=c6cba2b62629e819dd163325c7c9fb9bb5e3080c
PR104_NATIVE=10854bc0bff81b6d3cf5cb0a44520ed9165c205f
NATIVE_GMRES_BLOB=90bfaaa62afeda9dec27b3dfd594e67cd3034da1
PR107=1e604c7d9235e773f95a95efadff3258bbc3d0d3
PR108=611e697ac4c4beb10b87573e985a95a9bc70ce09
PREREG=9f16b8a579239f4cfa3646482058c303fea9e34b
DAG_VIGILODE=7/23; REI=28/88; PREFIX01=0/1; PREFIX02=0/1; PROPOSED_N00-N11=0/12 admitted; R/C=18/20 scoped; PRODUCTION=HOLD
Restore original 408419-byte SOURCE/evidence ZIP from Drive 18hnbR8qrV9jS_92WTxP_bfANismOJwyW or Dropbox id:BSpOijBcT10AAAAAAEHVOA, SHA256=910dd087148722799063c9f701f4187ce30fce0b4a5a7c9891f70fb3c3733578. Read THEORY_KO and local robust_lab/src/robust.rs. Native integration has NOT been done.
THEORY: A*=I-sigma J*, A0=I-sigma J0, deltaJ,delta_b,eta_apply in w-inf norm. rho_bound=||b0-y||+delta_b+eta_apply+sigma*deltaJ*||xhat||; K<=1/[1-sigma*(mu_w(J0)+deltaJ)] if positive margin; physical conversion C=max(w/p). Admit only outward rho<=tau_original and outward C*K*rho<=stage_error. Old WRMS with residual_scale is not this infinity norm.
CODE: fresh isolated native worktree PR106/PR104; preserve default routes, public API, work counters, callback costs/failures, zero-output-mutation, warm starts/restarts. First implement N01 with owner-bound certified J/RHS/callback errors and physical norm. Then N02 opt-in Givens trigger BEFORE prefix backsolve (do not wrap existing monitored-right which builds every column); N03 opaque provenance-verified MuUpper, N04 whole-slab confinement, N05 adaptive transaction.
TEST: cargo fmt/test/clippy Rust1.94.1, independent Fraction original-residual oracle, false left preconditioned convergence, rho/certificate, 0/NaN weights, wrong clock/epoch/source, nonnormal, failed callback, stale cell, partial output; matched-achieved-error full-IVP ABBA/BAAB and negative control, 8 suites/10 interactions, Q-LEDGER/Q-API/Q-GIVENS.
R6/C9 distinct author NOT_RUN. PR105 historic prereg missing preserved. No main merge, force push, production promotion. Return actual native SHA, commands/exit, source/epoch/rate capsule and measured error budget, public GitHub PR and Drive/Dropbox ACK.
