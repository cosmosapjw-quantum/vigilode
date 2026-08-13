# G3 — Fused φ-action + adaptive parallel exponential integrator

Branch: `generic-parallel-exp-adaptive-v1.3`

## 1. Fused φ linear-combination engine

Implement one action of the form

\[
\sum_{k=0}^{p}\tau^k\varphi_k(\tau A)b_k
\]

through one augmented Krylov process rather than one Arnoldi process per term.

Required features:

- KIOPS-style augmented operator;
- incomplete orthogonalization as a selectable policy;
- adaptive Krylov dimension;
- adaptive substepping/restart;
- residual/error estimate distinct from nested-dimension heuristic;
- all JVP, projected exponential, restart and rejection work counted;
- dense/high-precision oracle parity.

## 2. Nonautonomous augmentation

Support

\[
Y=(y,t),\qquad \mathcal F(Y)=(F(t,y),1)
\]

without explicit Jacobian formation. Validate against manufactured nonautonomous problems.

## 3. Adaptive EXPRB43 / pexprb54s4

Connect embedded error to the existing common controller with transactional rejection. Separate:

\[
E_{\rm time},\qquad E_{\varphi},\qquad E_{\rm JVP}.
\]

No accepted step may hide φ/JVP failure.

## 4. JVP quality lane

- exact/user JVP lane;
- fresh finite-difference JVP lane;
- compare `J_εv` and `J_{ε/2}v`;
- no persistent per-component perturbation factors;
- fail closed or use an explicitly declared W-method lane when quality is inadequate.

## 5. Generic calibration and sealed holdout

Calibration problems:

- complex Dahlquist;
- oscillatory Prothero–Robinson;
- nonlinear nonnormal block;
- stiff van der Pol;
- Robertson / HIRES;
- diffusion–reaction and advection–diffusion semi-discretizations.

Freeze all tolerance/Krylov policies before the holdout.

Comparators:

- protected sequential JF RODAS5P;
- held G1 q1→q2 plugin;
- byte-frozen adaptive BDF2;
- byte-frozen adaptive Radau IIA3.

## Promotion gates

\[
p_{\rm pexprb}\ge4.8,
\qquad N_J^{\rm explicit}=0,
\qquad N_{\rm Newton}^{\rm primary}=0,
\]

\[
\frac{N_{\rm steps,candidate}}{N_{\rm steps,RODAS}}\le1.2,
\qquad
S_{\rm same-error,median}\ge1.15,
\]

with false convergence zero and T1/T4 scientific identity.

## Kill rule

If the fused adaptive implementation is correct but still fails the `1.15×` same-error speedup gate on expensive-RHS and large-state holdouts, hold the exponential lane. Do not compensate by retuning BDF/Radau or physical clients.
