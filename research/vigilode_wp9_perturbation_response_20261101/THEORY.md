# WP9: original-source perturbation IVPs and exact stationary-weight derivatives

**Contract:** This file and registered parameters are based on a prospective source lock, committed before the new WP9 simulations (prereg `75920e0c3685cb7c86a04620c64b5d3bd1b1baa9`). Original frozen coupled-cubic source Git blob `d78c5c5f6f476f2a5b1ea88628ed25daa6cfc482`, original WP1 SWEEP SHA256 `9dae90079893170edddcf5cf8af193e53b43ab5bcf0c436cb81ce7aa0f8790b0`, WP1 SUMMARY SHA256 `56746487ac5c5c8702b53ef0ffd47694c040ab4e0adcfea1ec25aa3b8e9ba909`. Parent WP8 PR #171 frozen at `4f4ebf9fd6ecf52289ab82e2cc670a7ab72782ff`.

## R1–R3: exact source-conditioned mathematics
Let y_i'=-k_i y_i-g_i y_i^3+sum_{j!=i}c_ij(y_j-y_i), k_i>0, g_i,c_ij>=0. Use Q_ij=c_ij (i!=j) and Q_ii=-sum_j c_ij. For the positive normalized stationary left weight pi(theta)^T Q(theta)=0, 1^T pi(theta)=1, the observable W_theta(t)=pi(theta)^T y_theta(t) obeys
dW_theta/dt = -sum_i pi_i(theta)[k_i(theta)y_i+g_i(theta)y_i^3]+pi(theta)^T q(theta).
This is not a conserved quantity when sinks/sources are present.

For a dimensionless parameter theta with frozen initial y(0), define z=partial_theta y and u=partial_theta^2 y. For our parameter perturbations linear in theta,
z'=J(y)z+f_theta(y), z(0)=0
u'=J(y)u-6diag(g_i y_i)(z_i)^2+2 f_{y theta}(y)z, u(0)=0.
Exact-rational sensitivity of stationary weights:
pi'^T Q=-pi^T Q', 1^T pi'=0;
pi''^T Q=-2pi'^T Q', 1^T pi''=0.
Thus
partial_theta W_theta=pi'^T y+pi^T z,
partial_theta^2 W_theta=pi''^T y+2pi'^T z+pi^T u.

For original asymmetric_2 c01=12, c10=8, pi1(theta)=8/(20+12theta), pi2(theta)=12(1+theta)/(20+12theta). Exact pi'(0)=(-6/25,+6/25), pi''(0)=(36/125,-36/125). **The original initial-state values are binary64-represented dyadic rationals, not exact decimal 1/5 and 3/25.** The asymptotic decimal symbolic illustration is not substituted for the frozen source bits.

## R4–R7: actual new perturbation data and alternating search
Prospectively fixed physical experiment:
- four original source declarations: asymmetric_2, its resolution-equivalent asymmetric_2_refined, coupled_3_cubic, and linear_exchange_control. The refined pair is the same physical source, not an independent fourth model.
- parameter axes EXCHANGE_C01_REL, EXCHANGE_ALL_REL, CUBIC_ALL_REL, DECAY_ALL_REL, FORCING_CONST. The last is an explicit external source perturbation +theta*k_i*y_i(0), not the frozen original unperturbed RHS.
- signed perturbations theta=+/-1/50,+/-1/100,+/-1/200. Native source unchanged, zero-parameter source is validated against saved WP2 micro64 states. Scipy 1.17 Radau and DOP853 at rtol 2e-12, atol 2e-14, common 129-point output grid [0,0.1].
- sixteen concrete combinations actually evaluated and archived; 8 OpenEvolve ProgramDatabase proposals and 8 MCTS/UCT expansions in alternating order. **After every proposed node, real perturbed ODE IVPs are solved and the measured physical response, finite-difference agreement, and cost feed back into the next selection.** External LLM calls=0 and no trained neural network.
- 16 * six signed theta * two independent solvers = 192 actual nonzero perturbed IVP integrations, plus baseline and first/second augmented variational ODEs. Original WP1 BE and native WP3 RODAS5P code, data, and defaults unchanged.

Source-specific observed response (at t=0.1):
- asymmetric_2 one-edge exchange: dW/dtheta=-0.0169628408664; pi-change term -0.004159932493; state term -0.01280290837; d2W/dtheta2=+0.02113701999.
- coupled_3_cubic one-edge exchange: dW/dtheta=+0.0000557061858; pi-change term -0.000286520610; state term +0.000342226796. A frozen-pi-only analysis overestimates the signed total magnitude by ~6.14x in this specific model.
- coupled_3_cubic decay strength: dW/dtheta=-0.014955924658; d2W/dtheta2=+0.0117262327063.
- linear_exchange_control CUBIC_ALL_REL and EXCHANGE_ALL_REL: exact zero derivative controls.

Actual Radau vs DOP853 maximum y difference <3.46e-13; augmented z cross-check <2.94e-13 and second-variation crosscheck <4.4e-14. Outward proof of global interval error NOT_RUN. The analytic second-order approximation predicts the independently integrated +/-2% W at t=.1 with a smaller max absolute discrepancy than linear for all 13 source cases whose first-order truncation error exceeds 1e-11; median linear/second-order error ratio ~191. This is source-scoped evidence over the preregistered small theta range, not a rigorous Taylor remainder enclosure.

## R6 hostile referee
- Uniform pi need not annihilate nonsymmetric Q; pi(theta) must change along one-edge perturbations. Frozen pi gives the wrong derivative.
- 2nd-order curvature from actual augmented ODE is independently checked versus centered finite differences, not inferred from the same +/-2% outputs.
- In symmetric linear null control, uniform exchange-rate perturbation does not change W(t); do not claim all exchange perturbations produce a signal.
- Externally added source q != 0 is a genuinely different perturbed model, not the exact original-source IVP.
- Mathematical decimal values are NOT interchangeable with the original binary64 source identity.
- Cross-integrator numerical agreement is not a formal HOL kernel proof, rigorous residual bound or native RODAS5P speed comparison.
- No REI Bianchi IGM owner-IVP, cosmological likelihood or production admission was performed.

## R8–R10 status
23/23 local Python same-author tests PASS; original input source hash and archived reference endpoint checks PASS. Full source-contained scripts, input binaries, new physical trajectories, real ProgramDatabase, results, 23 tests, reports and figures are in the independent WP9 evidence ZIP; Git compact result has the full ZIP and content digests.

Historical CI recovery is separate: three authentic historical GitHub merge commits can be retrieved and were anchored under `archive/vigilode-historical-9ebeb123315e-20261101`, `archive/vigilode-historical-fb946b32938f-20261101`, `archive/vigilode-historical-b89508a60c69-20261101`. The original 2026-10-18 historical commits with 040b..., 4a5..., 40a... are still absent from GitHub direct reads; remote Desktop Commander host was offline. No historical object was fabricated or allowlisted. Global authority CI must be checked on this new WP9 PR before claim of improvement.

**Gates:** Official VigilODE DAG 7/23, admitted successor 0/12, C9 distinct author NOT_RUN, production HOLD, NO_MERGE, NO_DEFAULT_CHANGE, NO_TOLERANCE_RELAXATION, AUTO_NEXT_NODE=false. Novelty relative to general sensitivity ODE literature unresolved.

Primary sources for comparison: Cangelosi & Heinkenschloss (arXiv:2411.09655, sensitivity of quantities of interest to component-function perturbations); Blanes, Iserles & MacNamara (2022, ESAIM M2AN, DOI 10.1051/m2an/2022042); Corless & Corliss (1991), Rationale for Guaranteed ODE Defect Control.
