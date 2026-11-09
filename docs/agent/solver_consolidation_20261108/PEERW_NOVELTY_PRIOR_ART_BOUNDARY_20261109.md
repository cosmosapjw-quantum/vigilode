# Peer-W scientific novelty boundary (PW07)

**Verdict: novelty not established.** The current work is a falsifiable theorem candidate, not a publication claim.

| Idea | Prior art | Claim excluded |
|---|---|---|
| Parallel two-step stiff W/Peer stages | Podhaisky et al. 2002; Schmitt & Weiner 2004 | first parallel stiff Peer |
| Peer-W local/global error control | Kulikov & Weiner 2011 | first Peer-W global control |
| True global error bound for a Peer triple | Kulikov & Weiner 2015 ITSP3 | first Peer with true global control |
| Coupled GL solution/defect propagation | Constantinescu 2017 | first coupled solution-defect estimator |
| Adaptive multistep error models | Arevalo et al. 2021 | first history/step-ratio aware error model |
| Variable-step Peer common-norm stability | Lang & Schmitt 2026 | first variable-step Peer norm estimate |
| Contraction residual bound / affine scan | Banach fixed-point theorem / Blelloch 1990 | first contraction bound or affine scan |

## Remaining candidate

A source-attested **heterogeneous transactional method-switch certificate**: combine the original-W stage residual and operator perturbation, method-specific Peer startup/history transfer, validated propagation gain, RHS-only homotopy root residual where applicable, switching/restart defect, rounding and output interpolation in one original-IVP global-error budget. The recurrence is derived conditionally; source-specific kappa, chi, eta and delta are not yet validated. The ordered affine scan is an implementation route, not the novelty claim.

Constantinescu (2017) already estimates global error through coupled GL solution/defect propagation for same-order formulas with common initial data and proportionally related principal LTE. The 2011/2015 Peer papers preempt single-method Peer global error control. The proposed gap is limited to heterogeneous runtime transfer with method-specific histories and original-equation residual/source attestation, and requires a deeper focused prior-art search before submission.

Stop if propagation/transfer/residual/rounding bounds cannot be source-attested, or if matched achieved-error full-IVP evidence does not show value. References: https://doi.org/10.1137/S0036142902411057 ; https://doi.org/10.1016/j.cam.2011.08.006 ; https://doi.org/10.1137/140979952 ; https://doi.org/10.1016/j.cam.2017.05.012 ; https://doi.org/10.1007/s11075-020-00900-1 ; https://arxiv.org/abs/2608.26913
