# N01 design correction: derivative-free causal parallel homotopy primary
Date 2026-10-29. Supersedes PRIORITY in prior JVP-oriented R3 audit, not its actual source-path findings. This is a prospective corrected research objective, not historical backdating.

## User-authoritative motto
Causal, parallelized, derivative/matrix-free augmentation of Rosenbrock / Runge–Kutta with parallel accelerated homotopy Newton-free root finder.

The preferred main path is RHS-value-only with causal stages and bounded parallel independent stage work, screened accelerated homotopy, original equation residuals and physical error gates. JVP-only / matrix-free Krylov is a secondary verified fallback, not the preferred primary design. Dense Jacobian is only a small independent oracle.

## Primary original sources read
PR91 derivative-free causal-homotopy; PR92 production setup (persistent RHS-only default); PR74 coupled JVP comparator; PR93 polyalgorithm design; PR90 accounting; PR115 current nonlinear owner. Rust parity spec, vectorized-JF path controller spec, unified candidate spec; R-NEXT-07 epoch; Audit2 matrix-free; production closure; derivative_free_causal_homotopy README; synchronized_production; integrated_polyalgorithm. Existing code: crates/vigilode-coupled/src/rhs_homotopy.rs and causal.rs, crates/vigilode/src/production.rs, crates/rodas5p-integrators/src/problem.rs.

## Derived causal theorem
Let s-stage errors satisfy e_i <= d_i + Sum_{j<i} L_ij e_j, L>=0 strict lower triangular. Then e <= (I-L)^-1 d = (I + L + ... + L^(s-1)) d, since L^s=0. This does not require global contractivity; it does require exact causal dependency and nonnegative upper bounds. Nonnormal amplification persists and must be measured/certified. Residual-dominant Anderson updates only protect the specified map residual; they do NOT automatically certify physical root error.

## Constructive source-specific derivative-free RK stage
Robertson polynomial f=(-k1*a+k2*b*c,k1*a-k2*b*c-k3*b*b,k3*b*b). For a lower-diagonal implicit RK stage z=y+tau*f(z), tau>=0, y=(a0,b0,c0)>=0, rates>=0, M=sum y:
  c(b)=c0+tau*k3*b*b
  a(b)=(a0+tau*k2*b*c(b))/(1+tau*k1)
  F(b)=a(b)+b+c(b)-M.
F(0)= -(tau*k1*a0/(1+tau*k1)+b0)<=0; F(M)>=0.
F'(b)=1+[tau*k2/(1+tau*k1)]*(c0+3*tau*k3*b*b)+2*tau*k3*b>=1.
Hence b is uniquely bracketed in [0,M], with positive a,c; verified interval bisection is derivative-free and Newton-free. Multiple independent physical cells/trajectories can evaluate concurrently; do not run causal dependent stage j before predecessor i. This is a scoped BE/diagonal-RK anchor, NOT the original 8-stage raw-U RODAS5P. If an RK stage carrier has negative components the positivity argument does not apply.

## Next selected work
R5 constructive computation: verify this source-specific root and nonnegative causal stage envelope, independently test against exact rational and current Rust source. Link to already implemented RHS-only homotopy rather than re-inventing it. C5 candidate is optional ONLY after proof, with external original 8-stage residual gate required for any RODAS claim. N01 source epoch should distinguish model generation from ephemeral evaluations, but that repair alone is insufficient.

DAG unchanged: Original VigilODE 7/23, central REI 28/88, successor 0/12 admitted, N00F 1/1 scoped, derivative-free N01 bridge 0/1, production HOLD; distinct R6/C9 and full-IVP NOT_RUN. No default routing change, merge, force push, tolerance change or unverified speed claim.