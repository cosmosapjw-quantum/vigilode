# R5: derivative-free nonlinear stage reduction (scoped, 2026-10-29)

## Original research objective restored
The user's authoritative design is *causal + parallelized derivative/matrix-free augmentation of Rosenbrock/Runge–Kutta, with a parallel accelerated homotopy Newton-free root finder*. RHS-only (no derivative callback) is the preferred constructive route, not merely an optimization of JVP. PR91 already implements RHS-only homotopy, causal Picard memoization and a source-structured derivative-free raw-U route. PR92 keeps persistent RHS-only as the production facade default. PR74's JVP-coupled homotopy remains an opt-in comparator, not the primary architecture.

## Conditional mass-action theorem
For exact represented k1,k2,k3>=0, y=(a0,b0,c0)>=0 and tau>=0, consider one diagonal implicit-RK/BE stage z=y+tau*f(z), f=(-k1*a+k2*b*c,k1*a-k2*b*c-k3*b*b,k3*b*b).
Set D=1+tau*k1>0, M=a0+b0+c0,
c(b)=c0+tau*k3*b*b,
a(b)=(a0+tau*k2*b*c(b))/D,
F(b)=a(b)+b+c(b)-M.
F(0)=-b0-tau*k1*a0/D<=0; F(M)=a(M)+c(M)>=0.
F'(b)=1+tau*k2*(c0+3*tau*k3*b*b)/D+2*tau*k3*b>=1 on [0,M].
By the intermediate value theorem and strict monotonicity there is exactly one b* in [0,M]. The physical root z*=(a(b*),b*,c(b*)) is nonnegative, conserves total M and solves all three original stage equations. If M=0 the zero solution is unique.
For any proposed b_hat in [0,M], mean-value theorem yields |b_hat-b*|<=|F(b_hat)|. For p_i>0, define
La=tau*k2*(c0+3*tau*k3*M*M)/D; Lc=2*tau*k3*M;
||z(b_hat)-z*||_{p,infinity} <= max(La/p_a, 1/p_b, Lc/p_c) |F(b_hat)|.
Moreover the **actual original vector stage defect** equals
z(b_hat)-y-tau*f(z(b_hat))=(0,F(b_hat),0).
For computer admission, evaluate original F, La, Lc, reconstructed z, state/rate/clock identities by outward arithmetic. An unqualified binary64 residual cannot certify error.

## Causal amplification theorem (reused)
If e<=d+Le componentwise with d>=0 and L>=0 strictly lower triangular for s stages, then L^s=0 and
e <= (I+L+...+L^(s-1))d.
No global contraction required, but induced-norm amplification can be large. Genuine causal predecessors must not be scheduled out of order; independent cells/trajectories or independent same-depth evaluations may be parallelized.

## Actual verification
Local Python Fraction-from-exact-dyadics oracle (seed 20261029), 384 cases, 60-stage rational brackets with 144-step independent reference, 3456/3456 exact assertions PASS. Exact manufactured root z*=(1/2,1/4,1/4) with y=(35/64,27/128,31/128), k=(1/2,1,1/2), tau=1/4 PASS. Replayed on Python 3.13.5. The initial test incorrectly asserted the *candidate* residual had total zero and sign -F; exit 1 retained, fixed assertion to (0,F,0) and total F without changing tolerance or model. Wolfram exact symbolic check returned 0 residual-identity difference, 0 derivative-formula difference and 0 manufactured-root residual. This check is same-context CAS not independent-author R6.

This is a source-specific *auxiliary implicit RK* exact-real model. It neither implements nor validates the original 8-stage RODAS5P raw-U stage solve; no full-IVP, native Rust patch or parallel benchmark has been run in this corrected R5. Historical PR115 Rust mirror 28/28 is inherited only. External source legitimacy and physical33 application remain open.

## Next one node
Reuse crates/vigilode-coupled/src/rhs_homotopy.rs RhsOnlyMap and causal.rs for value-only source-owned stage inputs. Separate model-generation epoch from transient stage evaluation. Preserve original 8-stage residual and physical error gates, budgeted parallel failure work, fallback JVP without making JVP default, and strong sequential RHS-only comparator. No architecture refactor until the correct source-stage admission is demonstrated.

DAG: original VigilODE 7/23, central REI 28/88, successor 0/12 admitted; auxiliary R5 exact case 1/1 scoped, native N01 0/1 admitted, N00F 1/1 scoped. PRODUCTION HOLD.