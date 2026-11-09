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


## Fresh replay and reproducibility update (2026-11-09)

- Full mock driver SHA-256: `007543ec21224597ffdccebf5b9053b420e0e9c71add68971bb75a2b3b31ec5b`.
- Fresh full result SHA-256: `f4b31cab861ac1366b3027ac248243a762e57d66b76e3c02ab72fc4cc3f8e121`.
- Compact GitHub-source driver SHA-256: `6080a5bb3f28e29200d18885fa4006848e317d53bc7841744c7609a4a78556e4`; Git blob `69151b577e18ed77bc7fce8422d9d028335c0b07`.
- Compact regenerated JSON SHA-256: `0f4ecc3e8fcda1ab2c2a9d4b6f62b178502f2be72edd0d209a487b5f91dd2372`; repository JSON blob `d8da8a91b461bfe944912cd2a65d618a1eef4828`. The parsed numeric values agree; pretty-print float formatting differs by two bytes.
- Fresh CPU overlap replay (9 repeats, BLAS pinned to one thread): serial median `0.032002318 s`, two-thread median `0.021496683 s`, apparent microbenchmark ratio `1.48871×`, median task overlap `0.017554987 s`, output parity exact. An earlier run measured `1.78117×`; this variation is retained and is not an end-to-end solver speed claim.
- Both Python drivers passed `py_compile` and executed. Rust/Cargo/rustfmt/Clippy/CI/native Peer solver/full-IVP tests remain NOT RUN.
