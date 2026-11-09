# PW07 mock experiment summary

**Classification:** Python/SymPy mock only; no Rust solver or production validation. Full trace, detailed report and full driver are in the downloadable research handoff ZIP. The compact script in this repository reproduces exact order conditions and scalar refinement.

- Exact SymPy checks: PTSW2C C(2), Gamma(2), B(3) residuals all zero; PTSW2B C(2), Gamma(2), B(2) residuals all zero. PTSW2B stiff-limit characteristic polynomial is lambda^3, with M_inf^2 nonzero and M_inf^3 zero.
- Scalar test y'=-y, y(0)=1, with exact oracle startup (startup cost/error excluded). Fixed-step observed order approaches 3 for PTSW2C (2.9842 at N=640) and 2 for PTSW2B (1.9912). Variable step uses reciprocal ratios 0.8/1.25; final observed slopes are 2.9826 and 1.9903.
- An earlier 0.75/1.25 variable-ratio sequence was discarded because its product 0.9375 shrank h geometrically and was not a proper refinement family.
- Sampled scalar stiff-limit radii at z=-10^6: PTSW2C 0.67958, PTSW2B 9.54e-4. This is a negative-real-axis sample, not a complex-sector proof.
- For J=[[-1,K],[0,-10]] and six step ratios [1,.5,2,.5,2,1], each individual step radius remains below one, but the six-step product 2-norm reaches 769.6 (PTSW2C) and 795.2 (PTSW2B) at K=10^4. Finite transient amplification only.
- Inexact W residual/operator perturbation mock: identity residual 5.27e-16; solution error 4.69e-5 is below bound 1.19e-4.
- A synthetic 200-step affine bound recurrence and ordered product agree within 4.24e-22. This verifies the mock composition only; Blelloch scan is known prior art.

Limitations: exact oracle startup; scalar smooth IVP; no nonlinear MOL, no complex-sector interval proof, no real Rust/Rayon run, no fair-ab matched-error/G0 test. See PEERW_MOCK_EXPERIMENT_20261109.md in the research ZIP for detailed values.
