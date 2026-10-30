# BASS21 frozen-source J/c coupled generator, mathematical construction and adversarial referee

2026-10-30; parent PR134 exact HEAD `21373cf012f348ad6911580461ab466016968433`; prospective registry `ffa82d2885590c097b808bef1c70e210a5a41dbe` (posted **before** the full 50-case new-generator calculation). Original owner `cosmosapjw-quantum/bass@cc602e209e7deeb511ab2a1f3b2268d55206c43c`; nested original 50 NPZ histories SHA-256 `6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8`. Canonical PR134 ZIP SHA-256 `d33ff58f3f9fc8dec0e9cc3b055108f005f309af3209a688a2251ef7e3649a16`.

## Convention and exact generator

Metric signature (-,+,+,+); x=ln(a/a_ref), b'=db/dx, `g=c sigma_T n_e/H`, normal-comoving cold Thomson electrons; model-frame bolometric brightness and Stokes `Q=I_meridional-I_azimuthal` in transported screen. Q and deltaI come from PREEXISTING PR132 same-source coupled Volterra solve with original geometry, tau, electron, unpolarized baseline J_l frozen. This work evaluates the local generator **on these existing fields**. It does not integrate a new original IVP, update the baseline, or identify high screen c_l with BASS-native E_l.

Even parity basis `I_delta(μ)=sum_{m even}(2m+1) deltaJ_m P_m(μ)`, `deltaJ_l=∫_0^1 I_delta P_l dμ`; screen `Q(μ)=sum_{m even>=2} c_m P_m^2(μ)`, `P_m^2=(1-μ²)P''_m`, and `N_l=(l+2)!/[(2l+1)(l-2)!]`, `c_l=∫_0^1 QP_l^2/N_l`.

Liouville shear term for either angular field `f` is `3b'μ(1−μ²)∂_μf+4b'(1−3μ²)f`. Integrating by parts (boundary terms vanish at ±1 and parity zero at μ=0), its exact projection can be written

```
A^J_lm = (2m+1) ∫_0^1 P_m [(1−3μ²)P_l−3μ(1−μ²)P'_l] dμ
A^c_lm = (1/N_l) ∫_0^1 P_m² [(1−3μ²)P_l²−3μ(1−μ²)(P_l²)'] dμ.
```

These are exact rational matrices evaluated by SymPy polynomial integration; the low-rank cells independently rechecked in Wolfram Language. Parity-even angular structure makes both matrices l±2 tridiagonal. Reproduced nontrivial coefficients:

```
J0'|shear = -2 b' deltaJ2
J2'|shear = b'(-8 deltaJ0/5 -10 deltaJ2/7 +36 deltaJ4/35)
J10'|shear includes +(1188/161) b' deltaJ12
c2'|shear = (10/7)b' (c2+c4)
c4'|shear = b'(-36 c2/35 -40 c4/77 +504 c6/143)
c12'|shear includes +(880/87) b' c14.
```

Source-frozen coupled **perturbation** Thomson collision (do not double count baseline `J2_original`): `C_deltaJ0=0`; `C_deltaJ2=−g[9deltaJ2/10+(sqrt(6)/10)E2cand]`; `C_deltaJl=−gdeltaJl` for l≥4; `C_c2=−g[(J2_original+deltaJ2)/4+2c2/5]`; `C_c_l=−g c_l` for l≥4; `E2cand=12c2/(5sqrt(6))`. For homogeneous b'=0, the (deltaJ2,E2) homogeneous collision submatrix has eigenvalues `−1,−3/10` after factoring g, while `J2_original` is a forcing term. `C_deltaJ0=0` is exact at local collision level, not global photon energy conservation in anisotropic expansion.

**Closure:** Rows through l=10 intensity depend on actually projected deltaJ12; spin-2 rows through l=12 depend on actually projected c14. Finite 6×6 truncations with zero incoming l+2 are NOT exact at finite shear. Higher c_l remain source-convention coefficients, not native BASS E/B observables.

## New real physical measurements (source-owned, no IVP)

All original 50×193 histories, 50 exact raw input SHA checks, GL48 for original coupled Volterra source fixed point, GL64/96 for independent angular moments; no alteration of 62-state/electron/tau/geometry. Generated:

- 115,800 local J/c total generator evaluations (`50*193*(6+6)`), with separate shear and collision contributions and sampled-grid residuals (not new IVPs).
- 48,250 newly measured high coefficients `deltaJ12,deltaJ14,c12,c14,c16` (`50*193*5`), plus source-qualified 25 matched endpoints.
- max|c12|=1.4339270236737016e-9, max|c14|=1.1294699274522188e-10, max|deltaJ12|=3.55137261438686e-11; max|b'*(1188/161)*deltaJ12|=3.3844080592713422e-12; max|b'*(880/87)*c14|=1.4802454091369512e-11.
- final mean c12=-4.831105391847981e-10; matched final mean delta c12=-9.66221078370295e-10. Sign/magnitude are MODEL-FRAME quantities. The high-shear s04 source has max|c12|=1.4339e-9 versus s00 max|c12|=9.35e-20. The s00 c12 is not resolved above angular quadrature accuracy and is NOT a detectable nonzero physical signal.
- max GL64/96 deltaJ gap=2.025e-19 and c gap=6.939e-18; max PR133 deltaJ reuse gap=1.694e-20 and PR134 c2–c10 reuse gap=1.301e-18. Thus new HIGH-SHEAR c12 is well separated from angular quadrature noise, but no global BDF/time convergence guarantee follows.
- max J0 integrated source-frozen geometric work gap=4.402918532821109e-11 (matches inherited PR133); max sampled-grid local deltaJ residual=2.7466652386427335e-8; max sampled-grid local c residual=8.987918826122189e-6. Nonuniform saved-grid differences and numerical b'=gradient(b,x) are diagnostic only.

## Adversarial review: actual findings and boundaries

1. **Resolved R6-style algebra sign challenge (same author, CAS-assisted):** exact IBP and low-rank Wolfram coefficient checks confirm J2 -8/5, c2 +10/7, c4 -36/35 and eigenvalues -1,-3/10. Not distinct-author R6.
2. **Resolved double-counting challenge:** `deltaJ2` collision uses δJ2+E2, while E2/c2 collision uses original J2+δJ2. Treating J2original as δJ2 collision forcing would double count source baseline, so prohibited.
3. **New observed physical result:** s04 high-rank modes above angular error; s00 high-rank values below absolute error. Never generalize 50-case nonzero signal from an unresolved subset.
4. **Model truncation/closure hazard:** actual nonzero deltaJ12 and c14 feed l10 and l12. Zero-upper-multipole closure is not exact; interval convergence across unmeasured l16+ for derivatives l14+ NOT certified. Local generators stop at l10, c12 where measured neighbours suffice.
5. **Finite-grid hazard:** b' is from saved grid, and sampled numerical derivatives/Simpson do NOT certify native original BDF IVP or continuum response. Separate numerical implementation discrepancy from physical model claim.
6. **Source-owner equivalence limit:** Original BASS stores unpolarized L8 photons, while response and c_l are frozen-source model-derived. Not fully self-consistent original J_l/E_l evolution, not nonlinear Einstein/electron backreaction and not actual observed CMB E/B. BASS KM2G new Einstein-coupled photon/e/p source has distinct ICs and must not be spliced into these 50 histories.
7. **Review independence:** native external R6/C9 author NOT_RUN, own hostile review plus Wolfram symbolic and original-source replay performed. Global historical authority CI remains separate and potentially FAIL; scoped validations cannot override it.

Scientific status: exact Galerkin matrices DERIVED; saved-source high-shear spin2 data NUMERICALLY_CHECKED; specific frozen-source generator IMPLEMENTATION_VERIFIED after full replay; all BASS-native polarization equality/Einstein closure UNRESOLVED; production HOLD; official DAG delta 0.

## Primary references

- A. Challinor (2000), *Microwave background polarization in cosmological models*, Phys. Rev. D **62**, 043004, DOI 10.1103/PhysRevD.62.043004, arXiv astro-ph/9911481 (note metric (+−−−) in the original).
- A. Pontzen and A. Challinor (2007), *Bianchi model CMB polarization and its implications for CMB anomalies*, MNRAS **380**, 1387–1398, DOI 10.1111/j.1365-2966.2007.12221.x, arXiv 0706.2075 (near-FRW linearizations are not this finite-anisotropy source calculation).
- M. Kamionkowski (2021), *Cosmological perturbations without the Boltzmann hierarchy*, arXiv 2105.02887 (two coupled integral equations as a distinct approach; no asserted equivalence with this LRS Volterra model).
