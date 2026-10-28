# Nonlinear collective window maps: constructive proof note

Status: newly derived theory, ready for independent review; no native implementation or floating-point certificate is asserted. Repository read-only baseline: `ac89a9ac`. Sources inspected: `collective_model.rs`, `bound_collective.rs`, and `research/cloud_completion_20261018/PARALLEL_THEORY.md`.

## 1. Mathematical model and norm

Let positive stored weights be interpreted as exact real numbers, let `S = sum_j w_j > 0` be their actual mathematical sum, and use the real Hilbert norm `||a||_w^2 = sum_j w_j |a_j|^2`. Put `m(a)=sum_j w_j a_j`, `U z=(z,...,z)`, and

`A a=(-kappa+i omega_j)a_j`, `B_lambda(a)=i lambda chi |m(a)|^2 m(a) 1`, `0<=lambda<=1`, `kappa>=0`.

Then `||U||=sqrt(S)`, `||m||=sqrt(S)`, `mU=S`, and the orthogonal decomposition is `a=a_perp+(m/S)1`, with `m(a_perp)=0` and `||a||_w^2=||a_perp||_w^2+|m|^2/S`. Do not replace S by 1 because the model constructor accepts a tolerance about 1. The source RHS currently uses `width*xi_j` and does not use the stored carrier; this theorem follows that source field. A future common carrier may be removed exactly by a phase frame, because B is U(1)-equivariant.

On the radius-R ball define `beta=|lambda chi| S^2 R^2`. Direct real differentiation of the cubic gives

`||B|| <= beta R`, `||DB|| <= 3 beta`, `||D²B|| <= 6 beta/R`, `||D³B|| <= 6 beta/R²`.

At R=0 the state flow and its parameter derivatives are identically zero; treat that case separately and avoid divisions by zero. B, DB and D²B vanish at zero, but the generally nonzero D³B bound is the constant 6|lambda chi|S², obtained by cancelling R² before evaluation. These derivative estimates follow by applying the mean and repetition operator norms to the two-dimensional cubic `g(m)=i lambda chi |m|²m`.

The symmetric part of `Dg`, in a frame with m real, is `lambda chi |m|² [[0,1],[1,0]]`. The nonzero weighted collective block therefore has logarithmic norm `S |lambda chi| |m|²`; the mean-zero block is zero. Thus `mu_w(DB)<=beta`, sharper than `||DB||<=3 beta`.

## 2. Exact nonlinear subflow and energy

Along `a'=B_lambda(a)`, `m'=i lambda chi S |m|²m`. Hence

`Psi_B^t(a)=a+(m/S)[exp(i lambda chi S |m|²t)-1]1`.

The mean-zero part is fixed and the mean rotates; therefore the subflow preserves `||a||_w` exactly. The linear subflow is `Psi_A^t(a)_j=exp((-kappa+i omega_j)t)a_j` and scales that norm by `exp(-kappa t)`. The full ODE also satisfies `d||a||_w²/dt=-2 kappa ||a||_w²`, since the nonlinear contribution is `2 Re(i lambda chi |m|⁴)=0`. This proves global existence for finite input and exact energy decay, not merely local admission.

Define the one-step Strang map `S_delta=Psi_A^(delta/2) o Psi_B^delta o Psi_A^(delta/2)` and fine map `F_(lambda,h,m)=S_(h/m)^m`. Their energies decay exactly by `exp(-2 kappa h)`. The coarse map needed for a cheap affine prefix is `G_h=Psi_A^h`; a coarse Strang map is also available, but is not required for the proposed preconditioner.

## 3. Sharp subflow and Strang Lipschitz bounds

The mean-plane differential of `z -> exp(i c |z|²t)z`, after orthogonal input/output rotations, is the shear `[[1,0],[2theta,1]]`, where `theta=c|z|²t`. Its largest singular value is

`psi(|theta|)=sqrt(1+theta²)+|theta|=exp(asinh(|theta|))`.

The remaining mean-zero subspace is the identity. Taking the maximum derivative over a convex radius-R ball yields the sharp global bound `Lip(Psi_B^t;B_R)=psi(beta t)`. Sharpness follows by taking a constant-mode boundary state and the maximizing shear direction, approached from inside the ball.

The first half A-step reduces R to `R exp(-kappa delta/2)`; therefore

`q_S(delta,R)=exp(-kappa delta+asinh(beta delta exp(-kappa delta)))`.

Both linear halves are scaled unitary maps, so this bound is also sharp on the ball. For the fine map let `delta=h/m`, `R_l=R exp(-kappa l delta)`, `beta_l=|lambda chi| S² R_l²`. Then

`q_F <= product_(l=0..m-1) q_S(delta,R_l)`

`=exp[-kappa h+sum_l asinh(beta delta exp(-kappa(2l+1)delta))]`.

The product is a certified bound once its scalar arithmetic is outward; it need not be globally sharp. In particular `q_F<=exp(-kappa h+beta tau_m)` with

`tau_m=delta exp(-kappa delta) sum_l exp(-2 kappa l delta)`

`=d_kappa(h) kappa delta/sinh(kappa delta) <= d_kappa(h)`,

where `d_kappa(h)=(1-exp(-2 kappa h))/(2kappa)` and `d_0(h)=h`. The closed expression is interpreted by continuity at kappa=0. For the continuous ODE, integrating the decaying logarithmic norm gives `Lip(Phi_h;B_R)<=exp(-kappa h+beta d_kappa(h))`.

## 4. Fine-minus-linear derivative defect

Let `U_t=diag(exp(i omega_j t))`. Extracting total damping and rotation from each Strang step represents

`F_(lambda,h,m)=exp(-kappa h) U_h H_(lambda,m)`,

where H is the composition, in increasing l, of `U_(-(l+1/2)delta) o Psi_(B_lambda)^(tau_l) o U_((l+1/2)delta)`, and `tau_l=delta exp(-kappa(2l+1)delta)`.

Each conjugated flow preserves the R-ball, has logarithmic norm at most beta and Jacobian norm at most 3 beta, and the total artificial time is tau_m. The tangent matrix Y consequently obeys `||Y(s)||<=exp(beta s)`. Integrating its differential equation yields

`||Y(tau_m)-I|| <= integral_0^tau_m 3 beta exp(beta s) ds = 3(exp(beta tau_m)-1)`.

Therefore, on the convex R-ball,

`Lip(F_(lambda,h,m)-G_h) <= 3 exp(-kappa h)(exp(beta tau_m)-1)`.

This is the constructive nonlinear defect constant needed for a coarse-linear prefix preconditioner. It is independent of mode count and common carrier. It must not be confused with the weaker sum `Lip(F)+Lip(G)` or with a pointwise F-G error estimate.

## 5. Continuous-ODE error of the fine map

This section closes the split-map versus ODE gap. The fine map alone is not the exact nonlinear ODE flow.

Remove any common frequency omega_star by the equivariance above and put `L=diag(-kappa+i(omega_j-omega_star))`, `Omega=max_j |omega_j-omega_star|`, `a=sqrt(kappa²+Omega²)`. The minimum enclosing frequency interval gives the best such Omega. Use the actual stored frequencies, not `width` alone if xi admission tolerances allow values slightly larger than 1.

For fixed lambda write B for B_lambda, and define vector-field commutators

`C=LB-DB Lx`, `C_A=LC-DC Lx`, `C_B=DB C-DC B`.

Differentiation gives

`C_A=L²B-2L DB Lx+D²B[Lx,Lx]+DB L²x`,

`C_B=2 DB L B-DB DB Lx-L DB B+D²B[B,Lx]`.

The bounds in Section 1 establish, on the R-ball,

`||C_A|| <= K_A=16 a² beta R`, `||C_B|| <= K_B=24 a beta² R`.

These constants vanish with the nonlinearity. A useful improvement exploits homogeneous degree three. Write L=-kappa I+K, where K=i diag(omega_j-omega_star), and C_K=KB-DB Kx. Euler's identity gives C=2kappa B+C_K, C_A=4kappa²B+4kappa C_K+(K C_K-D C_K Kx), and C_B=DB C_K-D C_K B. Consequently one may use the strictly no-worse computable constants

`K_A=beta R min(16(kappa²+Omega²), 4(kappa+2Omega)²)`,

`K_B=24 Omega beta² R`.

For the improvement, `||C_K||<=4 Omega beta R`, the K-only double commutator has bound `16 Omega² beta R`, and the B/C_K commutator has bound `24 Omega beta² R`, by the same displayed expansions with L replaced by K. Pure damping cancels exactly from C_B. At Omega=0 this recovers `C_A=4kappa²B`, `C_B=0`, matching the scalar damped-cubic phase defect to leading order. Either the original or these improved constants may be used in the following proof; no general sharpness is claimed.

Here is a direct defect proof with signs fixed. For `u=exp(tL/2)x`, `v_s=Psi_B^s(u)`, `v=v_t`, and `y(t)=exp(tL/2)v`, differentiation yields

`r(t)=y'(t)-Ly(t)-B(y(t))`

`=exp(tL/2){ integral_0^(t/2) exp(-sL) C(exp(sL)v) ds - (1/2) integral_0^t D Psi_B^(t-s)(v_s) C(v_s) ds }`.

For the first integral subtract `(t/2)C(v)`. Its integrand difference is bounded by `K_A s exp(kappa s)`, because differentiation with respect to s gives the pulled-back negative C_A. After multiplication by the outside `exp(tL/2)`, its norm is at most `K_A t²/8`.

For the second integral subtract `t C(v)`. Put `Q(s)=D Psi_B^(t-s)(v_s) C(v_s)`. Then `Q'(s)=-D Psi_B^(t-s)(v_s) C_B(v_s)` and Q(t)=C(v). Hence the difference is bounded by `K_B integral_0^t integral_s^t exp(beta(t-r)) dr ds <= K_B t² exp(beta t)/2`. Including its factor 1/2 and the outer contraction gives at most `K_B t² exp(beta t)/4`. The two removed first-order terms cancel exactly.

Every state evaluated in these estimates remains in B_R: A is contractive forward and B preserves energy. Thus

`||r(t)|| <= t²[K_A/8 + exp(beta t)K_B/4]`.

The nonlinear variation-of-constants formula transports the residual with the exact full-flow derivative, whose norm is at most `exp(beta(h-t))`. Integration proves the rigorous local bound

`D(delta,R) = delta³ exp(beta delta)(K_A/24+K_B/12)`

`=delta³ exp(beta delta) R[(2/3)a² beta+2a beta²]` for the original constants; the improved K_A,K_B may replace them directly

for `||S_delta(x)-Phi_delta(x)||_w`, uniformly on B_R. No asymptotic remainder has been omitted.

For the fine map compute `D_l=D(delta,R_l)` and q_l from Section 3, then run the scalar recurrence

`E_0=0`, `E_(l+1)=q_l E_l+D_l`.

Induction by adding/subtracting `S_delta(Phi_(l delta)(x))` proves `||F_(lambda,h,m)(x)-Phi_h(x)||_w<=E_m`. This is an O(m) scalar calculation, with no dependence on N in its constant. A fixed known global radius may replace R_l at extra conservatism. For several windows the same affine recurrence, with their local E_m and q_F, certifies the continuous-flow error of the composed fine target and can use the already implemented affine prefix.

## 6. Parameter continuation and second-order predictor

In the interaction representation of Section 4 take `c=|chi|S²R²` without lambda. The lambda derivative v satisfies `v'=lambda Dg v+g`, and w=second lambda derivative satisfies `w'=lambda Dg w+2 Dg v+lambda D²g[v,v]`. The same equations hold on each conjugated segment; v and w are continuous between them. For 0<=lambda<=1, v(0)=w(0)=0, `mu(lambda Dg)<=c`, `||g||<=cR`, `||Dg||<=3c`, and `||D²g||<=6c/R` imply

`||v||<=R(exp(cs)-1)`,

`||w||<=6R exp(cs)(exp(cs)-1-cs)`.

For w, substitute the v bound: the forcing is at most `6cR[(exp(cs)-1)+(exp(cs)-1)²]=6cR(exp(2cs)-exp(cs))`; its integrating-factor integral is the displayed expression. Multiply both bounds by `exp(-kappa h)` and set s=tau_m for F. The exact continuous-flow analogue uses s=d_kappa(h), with time-dependent unitary conjugation. This justifies a tangent predictor and an explicit quadratic lambda-step remainder. If the tangent is only approximately computed, its error times the lambda step must be added separately.

Physical continuation `lambda chi` is distinct from the old PT2 convex interpolation `(1-lambda)G+lambda F`. Only lambda=1 is the requested physical nonlinear model. All stopping and certification must target the fine map at lambda=1.

## 7. Nonvacuous admitted parameter regime

For S=R=1, kappa=1, |chi|=0.1, Omega<=0.1, h=1, m=512 and every lambda in [0,1], one may use `a<=1.01`, `beta<=0.1`, `exp(beta/m)<1.001`, and `q_l<=1`. The local accumulation is bounded by

`E_m <= 1.001[(2/3)(1.01)²(0.1)+2(1.01)(0.1)²]/512² < 3.369e-7`.

Also `q_F<=exp(-0.9)<0.407`, so arbitrary many equal windows, even with the same conservative initial radius R=1, have composed fine-versus-continuous error `<3.369e-7/(1-0.407)<5.682e-7`.

The derivative defect is `d<=3 exp(-1)(exp(0.05)-1)` since tau_m<=1/2. A coarse-linear inverse geometric sum has norm at most `1/(1-exp(-1))`; therefore its nonlinear correction constant is `<0.09`. These explicit inequalities give a genuine nonlinear accuracy/contraction regime rather than an assumption that a useful regime exists. They do not establish a measured speedup; map cost, finite workers, projection, prefix and communication costs remain chargeable. A tighter implementation should use decaying R_l and actual tau_m rather than these deliberately simple bounds.

## 8. Native implementation contract (future work)

1. Bind the client to the immutable CollectiveModel, with owned or borrowed identity and generated coefficients; do not accept an unrelated generic OdeProblem callback as proof authority.
2. Admit positive weights, exact actual S, finite represented coefficients, kappa>=0, positive durations and integer m>=1. Freeze a verified radius R and frequency spread Omega before parallel scheduling.
3. Specify whether `lambda*chi` is an exact real product with interval enclosure or a newly represented parameter. A rounded product alone does not define the exact continuation family assumed above; lambda=1 must reproduce the source chi.
4. Implement exact-form nonlinear mean rotation with rigorously enclosed complex exponentials. Use `exp(i theta)-1=-2sin²(theta/2)+i sin(theta)` or an enclosed expm1 form for small theta. Standard libm point sin/cos/exp calls are not interval certificates; large-argument reduction and underflow need explicit enclosure contracts.
5. Return a point center plus weighted-ball evaluation radius, separate from the truncation E_m and shooting residual. Maintain weighted balls between steps; do not repeatedly convert a collective ball to independent component boxes and back.
6. Certify S, mean, energy, exp/asinh/expm1, q, E_m and derivative-defect bounds with directed arithmetic or interval libraries. Do not infer energy preservation of floating output from exact algebra. Project/round only with a proved nonexpansive map plus evaluation enclosure.
7. Keep global error budget split into continuous-model truncation, nonlinear shooting residual, floating evaluation, and initial-input uncertainty. A fine-map fixed point is not yet the continuous solution.
8. Complexity: each exact-form Strang substep is O(N) arithmetic/storage; F costs O(mN); q/E_m metadata cost O(m). Phase tables and vectors may be reused, but allocations and setup must be measured, not assumed zero.

Suggested minimal proof-facing API result: `{center, evaluation_radius_w, ode_error_radius_w, lipschitz_q, linear_defect_lipschitz_d, invariant_radius_in, invariant_radius_out, actual_weight_sum_enclosure, coefficient_identity, lambda, h, substeps, evidence_status}`. A missing transcendental enclosure backend is a typed certification blocker, not a numerical failure of this theorem.
