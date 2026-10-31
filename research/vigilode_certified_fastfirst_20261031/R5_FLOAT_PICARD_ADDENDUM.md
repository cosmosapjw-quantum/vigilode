# R5 amendment: source exactness versus rounded Picard candidate

This file corrects the first mathematical preregistration before any native CI. Keep the original THEORY_PREREG.md immutable.

For represented inputs x, source coefficients, anchor and lambda treated as exact reals, define real H_lambda and residual r(x)=H_lambda(x)-x. The source-derived q_lambda<1 is the exact Lipschitz upper bound. The **float** mapped state is y=fl(H_lambda(x)), not exact H_lambda(x). Put e=y-H_lambda(x). Then

||r(y)||_2
 <= ||r(H_lambda(x))||_2 + ||H_lambda(y)-H_lambda(H_lambda(x))||_2 + ||e||_2
 <= q_lambda ||r(x)||_2 + (1+q_lambda)||e||_2.

The original preregistration's strict geometric check on **rounded** Picard y was overly strong. Exact affine scalar example a=fl(.99), b=fl(.01) has mathematical Picard residual equality ||r(H(x))||=q||r(x)||; any outward enclosure width makes U(y)<=down(q*L(x)) generally impossible for small nonzero rounding. The correct source-certified Picard fallback uses outward upper bounds for q, r(x) and e, and checks that the measured interval lower residual at rounded y does not contradict the valid inexact envelope. It does NOT imply exact geometric contraction of every rounded Picard step, and rounding-floor stagnation can be classified INCONCLUSIVE/BUDGET.

The **speculative Anderson** shortcut still uses strong interval sufficient condition U(z)<=down(q_upper*L(x)). This is an exact-real residual guarantee at the **represented z**, unrelated to how z was formed. If the trial fails, evaluate/charge Picard and use the additive-rounded fallback. A root is accepted only when the final source-owned exact residual U(x) yields div_up(U(x),sub_down(1,q_upper)) <= the unchanged user root-error target. This final bound is valid independently of the success path and includes actual f64 candidate state as an exact-real input. A bad outward computation is a numerical defect, not a general theorem failure.

This is an R5 constructive correction, not a tolerance change, method substitution, or extra admission gate. Every attempted source evaluation and directed residual enclosure remains charged. No general nonlinear/stiff IVP claim.

Verifier priority: independent Fraction oracle for represented inputs; native tests for expected failure and source mutation; finite source lock and exact Git identity.
