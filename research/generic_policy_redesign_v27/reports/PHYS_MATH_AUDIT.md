# PHYS–MATH AUDIT — v2.7 actual pexprb54s4 level-1 prefix

## Scope

This audit covers only the read-only R-JF committed trajectory plus an actual
`pexprb54s4` dependency-level-1 prefix probe. Active switching and full E-K
continuation are excluded. No N=2048 execution is allowed in this node.

## 1. Definitions and dimensions

The actual prefix computes the existing fused-exponential level-1 objects

\[
U_2=y_n+h c_2\varphi_1(c_2 hJ_n)F_n,\qquad c_2=1/4,
\]
\[
D_2=F(U_2)-F_n-J_n(U_2-y_n).
\]

The tolerance-scaled diagnostic is

\[
\rho_{c_2}=\left[\frac1m\sum_{i=1}^m
\left(\frac{|h|D_{2,i}}
{\mathrm{atol}+\mathrm{rtol}\max(|y_{n,i}|,|U_{2,i}|)}\right)^2\right]^{1/2}.
\]

For compatible state units, numerator and denominator both have units of the
state component, so every term and \(\rho_{c_2}\) are dimensionless. For
heterogeneous physical units, scalar `atol` remains a scope limitation; this
node does not promote \(\rho\) to a universal coordinate-invariant score.

## 2. Causality correction

An event emitted after accepted R-JF step \(j\) can only act on the *first next
controller proposal*, not on the next eventually accepted row. v2.6 established
that the frozen k=1 policy targets a rejected next proposal in the same
nonautonomous location for N=128 and N=512. v2.7 therefore probes the actual
first proposal.

At that counterexample:

- N=128: first proposed \(h=0.010883336599682315\), R error 1.33636588,
  actual \(\rho=203.884895\).
- N=512: first proposed \(h=0.010870768157056005\), R error 1.36699250,
  actual \(\rho=207.155538\).

Thus accepted-grid replay was not merely a bookkeeping approximation; it could
move the prefix to a materially different state/step size.

## 3. k=3 development survivor

The development-only k=3 persistence filter generated 56 actual prefix events
(26 at N=128, 30 at N=512). Every first R-JF target was accepted, and every
level-1 prefix succeeded. This removes the rejected-next-target failure seen for
k=1 and k=2 on the existing N=128/512 corpus.

This is *not* a proof that k=3 is generically safe. It was identified after
seeing N=512 and therefore cannot be called a sealed selector or holdout result.

## 4. Level-1 rho is not a full-step safety certificate

Historical full-E labels on exact-parity target rows show one E-inadmissible
HIRES event in each dimension:

\[
\rho\simeq2.2712,\qquad q_E\simeq1.135>1.
\]

The same HIRES family also contains accepted/admissible events with
\(\rho\simeq1.86\) and \(\rho\simeq39.45\). Therefore no monotone threshold in
this level-1 scalar can separate full-E admissibility even inside this tiny
local family sample.

This agrees with the earlier mathematical remainder argument: a small or
moderate \(D_2\) does not control later-stage excursions or the Lipschitz
variation of the Jacobian over the full step.

## 5. Work and limiting cases

Every prefix has exactly two RHS evaluations and one phi action. Median prefix
JVP-vector work is 5; P95 is 7. The prefix is therefore nonzero work, but it is
strictly earlier than full completion and retains resumability.

The implementation test proves begin-prefix + resume has exact one-shot parity
for final states, embedded state/error, phi reports, telemetry, and total
`WorkCounters`. Hence no level-1 recomputation is required if a later node ever
continues the same prefix.

## Verdict

- **Actual level-1 mechanism:** PASS.
- **Causal placement on first next proposal:** PASS on the executed corpus.
- **k=3 rejected-next avoidance:** SURVIVING DEVELOPMENT CLAIM ONLY.
- **Level-1 rho as a full-E safety certificate:** REJECTED.
- **Active switching or N=2048 promotion:** NOT AUTHORIZED.

The mathematically admissible next question is whether a *later resumable
prefix level* supplies the missing nonlinear-remainder information before full
E-K completion.
