# Independent review of nonlinear homotopy theory

Review baseline: `ac89a9ac5600e631120da480eee7f67bb16f9ea0` (read-only repository). This review targets the new `flow.md` and `parallel.md` derivations, not unchanged historical campaigns. No production source edits or scientific runtime results are claimed by this note.

## Reviewed mathematical results

1. Actual positive weight sum S and weighted Hilbert norm: correct; no exact-normalization assumption is imported from the constructor tolerance.
2. Exact collective cubic subflow: correct. The mean rotates at angular frequency lambda*chi*S*|m|^2; the weighted mean-zero component is fixed. Full-flow and split-flow energy decay follows exactly.
3. Sharp nonlinear shear Lipschitz bound and repeated Strang upper bound: correct. The damping interaction-clock sum is midpoint quadrature for exp(-2*kappa*t), hence is bounded by d_kappa(h).
4. Fine-minus-linear Lipschitz defect: correct. The factor 3 belongs to the full derivative norm while the factor 1 belongs to the logarithmic norm; the derivation uses each in its correct role.
5. Full continuous Strang defect: independently differentiated. Both integral terms have the correct signs; first-order cancellation is exact. The nested commutator bounds 16*a^2*beta*R and 24*a*beta^2*R follow by termwise norm bounds. The refined damping/rotation decomposition yields K_A=min(16*(kappa^2+Omega^2),4*(kappa+2*Omega)^2)*beta*R and K_B=24*Omega*beta^2*R. The full residual integration gives an actual finite-step bound, not merely a formal local-order term.
6. Split-to-ODE scalar propagation: correct when both states are in the stated decaying balls and each q bounds the same split map. The bound includes the difference between the fine target and the continuous ODE.
7. Parameter tangent and second derivative bounds: correct for lambda in [0,1] and a lambda-independent initial state. The lambda=0 continuous limits are finite. Differentiability of the physical target should be established from unprojected causal shooting equations, not from the nonsmooth radial projector on the invariant sphere.
8. Projection after the entire affine prefix: correct. The resulting exact map has a unique fixed point by causal induction. Inserting the projector inside the scan would define a different, non-affine recurrence.
9. Exact finite-front and factorial convergence: correct for the strictly lower-triangular nonnegative error majorant. This is an exact-arithmetic theorem and does not imply nilpotence of a represented floating iteration matrix.
10. Final endpoint residual certificate: correct independently of iteration contraction, provided both candidate centers and exact states lie in the declared input balls and the full physical-flow evaluation discrepancy is included.

## Findings sent to authors

- F01 (minor mathematical statement): the original R=0 sentence incorrectly included D^3 B among vanishing derivatives. D^3 B(0) is generally nonzero, with norm bound 6*|lambda*chi|*S^2. Author corrected this before closure.
- F02 (performance proof): a certified upper bound on one parallel fine-map cost is not a lower bound on the serial comparator. The speed theorem must compare parallel upper cost with serial lower cost, or explicitly use an exact abstract matched-kernel cost model.
- F03 (finite arithmetic domain): an eta-close approximation to projection can lie outside the admitted ball. The implemented map must independently certify center membership, or use fixed enlarged balls and inward projection slack.
- F04 (scan arithmetic domain): raw affine prefix values can lie outside invariant balls. Floating scan bounds must cover these actual raw values, not assume the input-ball radius bounds every intermediate prefix.
- F05 (fine-target startup): the coarse-start pointwise error bound should explicitly cover repeated Strang maps by integrating the energy-preserving interaction kicks, not only exact ODE flow.

F02-F05 were corrected in the reviewed final notes: separate parallel upper/serial lower costs; certified membership prerequisite; raw prefix-state rounding coverage; and a split-interaction pointwise startup proof. These corrections close the stated findings.

Decision: **PASS_SCOPED_THEORY**. All reviewed theorems hold with their stated domain, exact-map, membership, and finite-arithmetic error hypotheses. This is a conditional theory decision. No native implementation, current machine speedup, or broad production promotion follows from it. The speed example is conditional on an admissible matched serial lower cost and parallel upper cost, not an observed speed measurement.

The native follow-up must preserve the distinction between the new window-shooting client and the repository’s older stage-homotopy client. A07–A12 deployment claims require separately bound source, runtime, and acceptance evidence. This review alone does not close the original generic A05 controller or legacy stage-H5 obligation.


Reviewed proof identities (SHA-256):

- `flow.md`: `ab3807989b4aa3b35bbed37c70f5b25a69588998a08f9ecd454844ac343ba7dc`
- `parallel.md`: `fbafa78417bdc31266004188b7151a1c1f0792995821afff00796b689726952e`
