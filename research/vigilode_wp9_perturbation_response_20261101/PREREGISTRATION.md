# WP9 prospective perturbation-response experiment (2026-11-01 KST)

This registration is **PROSPECTIVE for the new WP9 model-perturbation IVP simulations**, not retrospective WP1-WP8 source calculations. No native solver performance or production claim.

Immutable source: parent WP8 PR #171 head `4f4ebf9fd6ecf52289ab82e2cc670a7ab72782ff`; original fixed cooperative cubic source blob `d78c5c5f6f476f2a5b1ea88628ed25daa6cfc482`. Original WP1 `SWEEP.log` SHA256 `9dae90079893170edddcf5cf8af193e53b43ab5bcf0c436cb81ce7aa0f8790b0`, `WP_SUMMARY.json` SHA256 `56746487ac5c5c8702b53ef0ffd47694c040ab4e0adcfea1ec25aa3b8e9ba909`. WP3 native RODAS5P and WP2 reference bits remain immutable.

## Scientific objective
Use actual *newly integrated* perturbation IVPs from frozen RHS, not more certification-bound tuning: resolve the change in the source-specific positive-stationary-weight observable W(theta,t)=pi(theta)^T y(theta,t), first-order response z=d y/dtheta, and d pi/dtheta for a source-dependent exchange generator.

Use three distinct original physical-source configurations: asymmetric_2, coupled_3_cubic, linear_exchange_control; repeat asymmetric_2_refined only as a source-equivalence control. Horizon [0,0.1], fixed positive y0 original source bits, sample 129 evenly spaced output times. Parameter theta dimensionless:
- EXCHANGE_C01_REL: original c_01 multiplied by (1+theta).
- EXCHANGE_ALL_REL: all original off-diagonal c_ij multiplied by (1+theta).
- CUBIC_ALL_REL: all original g_i multiplied by (1+theta).
- DECAY_ALL_REL: all original k_i multiplied by (1+theta).
- FORCING_CONST: add +theta*k_i*y0_i to original RHS, an explicitly source-conditional external source perturbation (not baseline source).
Each theta uses ± 1/50, ±1/100, ±1/200 in separate IVPs, baseline theta=0. All perturbed couplings/nonlinear rates remain nonnegative. Compare SciPy Radau and DOP853 independently at atol=2e-14 and rtol=2e-12, with a coupled variational IVP z'=J(y)z+partial_theta f(y), and exact-rational pi(theta) obtained from perturbed Q; d pi/dtheta obtained from differentiated stationarity conditions.

Admit a perturbation candidate only with actual ODE numerical integration, positivity, accurate source identity, and finite difference response matching its variational expression. Evaluate observable at t=.02,.05,.1 and show +/- response, FD convergence and solver cross-check. Fixed pi0 and changing pi(theta) are different diagnostics; do not conflate. Linear exchange control with all exchange scaled is a physically meaningful zero-sensitivity negative control; cubic perturbation on g=0 also zero. Cross-solver agreement is numerically checked, not a rigorous interval certificate or native RODAS5P comparison.

## Alternating search
Eight OpenEvolve-ProgramDatabase candidate proposals and eight MCTS-UCT expansions on theory-space nodes. After EACH proposal, run the actual source-locked numerical perturbation IVPs, pass observed response/nonlinearity, robustness, costs and failures into the next selection step. No neural network training or external LLM API calls. Preserve all invalid candidates and evaluation traces; generate an actual plot and graph.

## Global authority repair
Inspect original local/archived Git bundle SHA256 `9512aa3d596315ce952482214b4ae26edb47a50b3c857301485d228efa1d2862` and historic CI 110 missing source citations. A missing Git object must NOT be replaced with a guessed SHA or allowlisted; only original verified unreachable commit objects may be anchored by independent archive ref if GitHub object identity/provenance are confirmed. Remote Desktop host access requires online status; no implicit claim of access when offline. No modifications to production running processes.

## Gates
OFFICIAL_VIGILODE_DAG=7/23; SUCCESSOR_ADMITTED=0/12; PRODUCTION=HOLD; NO_MERGE, NO_DEFAULT_CHANGE, NO_TOLERANCE_RELAXATION, AUTO_NEXT_NODE=false. Different-author C9 NOT_RUN, new HOL formal proof NOT_RUN. Only actual new science measurements can be recorded as this loop's new data.
