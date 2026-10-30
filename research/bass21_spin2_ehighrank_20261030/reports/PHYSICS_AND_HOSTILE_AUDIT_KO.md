# BASS21 original 50×193 source: fixed-screen spin-2 E+ higher-rank physics and closure leakage

## Authority, physical domain and new scope

New prospective prereg Git commit `c7fc51f2a8007659db095073b83c39a46fea064c`, before the new 50-case experiment. Immutable parent VigilODE PR #143 HEAD `15d68af701fc9267e2c4b04cc81e2550a9bbbd50`, source complete ZIP SHA256 `a2fa5e7cbae98cb7a28df19314b1c6da8b90ad5277c08831384290171f4231ee`. Original owner [BASS commit](https://github.com/cosmosapjw-quantum/bass/commit/cc602e209e7deeb511ab2a1f3b2268d55206c43c), source ZIP SHA256 `6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8`. The BASS SHA belongs to repository cosmosapjw-quantum/bass, not VigilODE. The original BASS 62-state unpolarized histories, geometry, electron/temperature, optical clock and all 50 × 193 physical source cases are unchanged. 0 new IVPs; reuse existing source-frozen two-field I/Q Volterra response. Metric (-,+,+,+). x=ln(a/a_ref), g=c sigma_T n_e/H, b' = db/dx. Physical Q=I_meridional-I_azimuthal, U=V=0, transported LRS spherical screen. All results depend on this screen and on the source's bolometric brightness normalization; no measured kelvins or spectral CMB are asserted.

## New exact derivation (R5 / mathematica independently checked low ranks)

Define the *axisymmetric fixed-screen spin-2 modes* (l even, l>=2):

```
P_l^2(mu)=(1-mu^2)*d^2P_l/dmu^2
N_l=(l+2)!/((2*l+1)*(l-2)!)
c_l = integral_0^1 Q(mu)*P_l^2(mu) dmu / N_l
k_l = sqrt(4*pi/(2*l+1)*(l+2)!/(l-2)!)
E_l0^(+) = k_l*c_l
```

The plus sign is explicitly defined; a common E convention changes sign. The previously verified LRS spin-2 Liouville shear operator from PR #138 has an exact moment adjoint:

```
A_c[l,m] = 1/N_l * integral_0^1 P_m^2 * ((1-3mu^2)*P_l^2 - 3mu(1-mu^2)*d(P_l^2)/dmu) dmu
A_E[l,m] = (k_l/k_m)*A_c[l,m]
d E_l0^(+)/dx |_shear = b' * sum_m A_E[l,m] E_m0^(+)
```

Polynomial integration from l=2..24 yields exact *tridiagonal within even l*, not closure at a finite l. Sample exact coefficients: A_c[2,2]=10/7, A_c[2,4]=10/7, A_c[4,2]=-36/35, A_c[4,6]=504/143, A_c[16,18]=3420/259; A_E[16,18]=360 sqrt(7315)/2849=10.807300237309965. There are no azimuthal m!=0 modes in the **fixed** LRS screen.

Thomson collision (pure cold electron original source):

```
C_E2=-g*(k_2 J2_total/4 + 2 E_20^(+)/5)
C_E_l=-g*E_l0^(+) for l>=4 even.
```

The source J2 is **total** original plus delta intensity J2, not only the perturbation. The exact rank-nine tensor/Stokes source and U_fast,V_slow eigenmodes from PR141/143 are not independently evolved here.

For finite cutoff L, the actual omitted first-neighbour term is exactly

```
BoundaryFlux_L(x)=b'(x) A_E[L,L+2] E_(L+2),0^(+)(x)
```

It is **not identically zero**, and a closure setting E_(L+2)=0 would remove it by assumption, not by dynamics. This identity is not a new no-go theorem about finite truncations: it simply exhibits one explicit nonzero missing term of the chosen local shear transport.

## Actual original-source physics (not symbolic surrogate)

All 50 cases ×193 epochs project Q at independent GL64 and GL96 on l=2,4,...,24, with an additional numerical polynomial-derivative basis independent of scipy.special.lpmv. Existing PR141 E_l0+ (l2..16) is reused after actual reprojection, not overwritten. New source projected high-mode entries l18,20,22,24: **38,600**; newly evaluated shear+collision local generator entries l2..22: **106,150**; actual l16,18,20,22 omitted-neighbour flux entries: **38,600**. Source-input code and actual array files in results/HIGH_E_HIERARCHY_50x193.npz.

| even l | max abs E_l0+ (all 50) | GL64/96 gap | s00 max abs | s04 max abs |
|---|---:|---:|---:|---:|
| 2 | 3.501228383e-02 | 4.857e-17 | 1.034e-02 | 3.501e-02 |
| 4 | 2.450110576e-03 | 6.939e-17 | 4.028e-06 | 2.450e-03 |
| 6 | 2.105371556e-04 | 6.147e-17 | 1.713e-09 | 2.105e-04 |
| 8 | 1.873038548e-05 | 6.944e-17 | 7.114e-13 | 1.873e-05 |
| 10 | 1.695197803e-06 | 7.148e-17 | 2.780e-16 | 1.695e-06 |
| 12 | 1.575740280e-07 | 7.355e-17 | 1.027e-17 | 1.576e-07 |
| 14 | 1.553895897e-08 | 8.129e-17 | 1.409e-17 | 1.554e-08 |
| 16 | 1.734157930e-09 | 7.930e-17 | 1.136e-17 | 1.734e-09 |
| 18 | 2.375651213e-10 | 7.586e-17 | 1.445e-17 | 2.376e-10 |
| 20 | 4.112730528e-11 | 8.008e-17 | 1.391e-17 | 4.113e-11 |
| 22 | 8.490335579e-12 | 8.694e-17 | 1.323e-17 | 8.490e-12 |
| 24 | 1.925881753e-12 | 8.085e-17 | 1.394e-17 | 1.926e-12 |

Maximum observed omitted-neighbour flux (per dimensionless x):

| cutoff L | omitted L+2 | max abs boundary flux |
|---:|---:|---:|
| 16 | 18 | 3.47363789275097e-11 |
| 18 | 20 | 7.10075748388474e-12 |
| 20 | 22 | 1.68939691535110e-12 |
| 22 | 24 | 4.24661438282880e-13 |

The strong-shear s04 high modes are above the GL64/96 discrepancy by >10-fold, but weak-shear s00 modes l>=12 remain unresolved on this angular grid. Detection above quadrature discrepancy alone does NOT certify source time accuracy, full transport, observational power spectra, or astrophysical precision.

## Reproducibility and falsifiers

Exact measured maximal differences: original PR141 l2..16 E=6.93889390390722838e-18; original c=8.67361737988403547e-19; J2 total=1.11022302462515654e-16; GL64/96 all even <= 8.69426418004307598e-17; independent Legendre polynomial derivative <= 1.40852020749450541e-17; original Volterra fixed-point residual <= 4.63301272346505755e-15. The precommitted 1e-10 angular/source equality criterion passed without weakening. Positive Stokes coherency |Q|/I max 1.20236906101861534e-02. Local original saved-grid derivative-model discrepancy max 6.98047043709154602e-05, explicitly **not** a BDF/global error certificate.

The new tests include exact l2/l4/l16 matrices; independent P_l^2 derivative; manufactured pure l2, l8, l18 and l24 Legendre fields; Liouville adjoint numerical check; original J2 Thomson source at l2; all high flux identities; strict no-IVP/HOLD, tampered-source rejection and separation of resolved/unresolved cases. Actual verification outcomes in external final receipt and clean extraction log, not presumed from source comments.

## One-pass same-context hostile review (R6/C9 distinct author NOT_RUN)

1. **Normalization/sign:** plus-E is a chosen S² spin-harmonic convention. Standard minus-E flips sign; original native BASS uses an unpolarized 62-state owner IVP and does **not** dynamically evolve this E_l0. Full PSTF and arbitrary observer spin-screen mapping UNRESOLVED. No false B=0 beyond transported axial U=0.
2. **Apparent high-order detection:** s04 higher coefficients resolved relative to quadrature, not to frozen-source Volterra approximation, actual time interpolation, original BDF time precision or observations; s00 high modes *not resolved*. The source angular sky was generated by known approximate Volterra postprocessing; not a new owner IVP.
3. **Finite closure:** the exact shear generator couples l↔l±2. Nonzero l16←18, 18←20 etc flux is measured. We do not claim no finite closure exists for all initial data or all gauges, nor that 24 modes suffice for a real Einstein–Boltzmann solution.
4. **Thomson/rate original:** l2 collision uses J2_total, not deltaJ2 alone; l>=4 has extinction only in this axisymmetric normal-comoving cold Thomson model. g is source-owned with exact physical units canceled by H, no gamma factor invention.
5. **No temporal false PASS:** measured saved-grid derivative mismatch is NOT_RUN for native global BDF time error certification. It cannot be assigned solely to numerical quadrature. No production or global authority CI promotion.
6. **Independence:** exact SymPy plus manufactured tests and same-context hostile review are no different-author R6 or C9. Native subagent NOT_RUN. Formal 18/20 scoped gate, official VigilODE DAG 7/23, successor 0/12 unchanged.

## Decision and literature

**DERIVED:** exact spin2 moment and E+-basis shear/Thomson map under stated assumptions. **NUMERICALLY_CHECKED:** original 50×193 source full high-rank E and upper flux, 25 matched pair endpoints, angular convergence. **UNRESOLVED:** native source-evolved E_l full PDE/hierarchy, polarization screen transport beyond LRS, source x/tau clock consistency, self-consistent Einstein/electron/temperature feedback, CMB observed E/B, distinct-author review, global authority Git provenance. **PRODUCTION=HOLD**, no main merge, no force push, no IVPs, official DAG delta 0.

Primary source literature: Anthony Challinor, *Microwave background polarization in cosmological models*, PRD 62, 043004 (2000), DOI 10.1103/PhysRevD.62.043004; Andrew Pontzen and Anthony Challinor, *Bianchi model CMB polarization and its implications for CMB anomalies*, MNRAS 380, 1387-1398 (2007), DOI 10.1111/j.1365-2966.2007.12221.x; Angelo M. Anile, *Anisotropic expansion of the universe and the anisotropy and linear polarisation of the cosmic microwave background*, Astrophys Space Sci 29, 415 (1974), DOI 10.1007/BF02639073. Available via corresponding DOI/ArXiv (Challinor astro-ph/9911481; Pontzen & Challinor 0706.2075). No new user-supplied source required for this bounded result.
