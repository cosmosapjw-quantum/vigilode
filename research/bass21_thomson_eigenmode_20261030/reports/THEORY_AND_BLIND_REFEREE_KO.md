# BASS21 원본 50×193: 정확 Thomson 충돌 고유모드, 전단 유입 및 인과 memory

2026-10-30 | PR #141 child | preregistration `ea8f5f6946614f90b414d187cbbdb5ce809179be` (GitHub, **새 50-case 실험 전에 게시**)

**결과 상태:** exact collision algebra `DERIVED + WOLFRAM/SYMPY CHECKED`; 실제 source-owned mode/shear/history `NUMERICALLY_CHECKED`; code `IMPLEMENTATION_VERIFIED`; full owner-native dynamic E_l, Einstein/electron/thermal feedback and observed CMB E/B `NOT_RUN`. Same-author hostile audit is not distinct-author R6/C9.

## Authority / conventions

- Owner repository: `cosmosapjw-quantum/bass`, immutable commit [cc602e209e7deeb511ab2a1f3b2268d55206c43c](https://github.com/cosmosapjw-quantum/bass/commit/cc602e209e7deeb511ab2a1f3b2268d55206c43c). Original source archive SHA256 `6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8`, 50 original 62-state non-polarized sources, 193 immutable saved epochs. Do not treat owner SHA as a VigilODE local commit.
- Parent PR #141 exact HEAD `f14f29d4356f1723ce3efaf8ed15ad06bb955b1a` (unchanged), canonical source-inclusive ZIP SHA256 `ce2b3507c30f5429fb93244bd7f3de7426f1f158bf52962e24734b6d1237dfe2`, 47,114,443 bytes; PR141 original normalized photon moments and coherency M are used only as comparators. No BASS KM2F/KM2G/KM2H/CE1 new-event substitution.
- Signature `(-,+,+,+)`; normal-comoving cold Thomson electrons; x=ln(a/a_ref); dimensionless optical collision rate g=c sigma_T n_e/H≥0; tau original saved optical depth. E remains model-frame screen-fixed E_l0(+), **not** source-native fully evolved E_l. U=V_Stokes=0 under parity-even axisymmetry; Stokes Q is meridional minus azimuthal.

## Exact local collision matrix and physical tensor relation

For full total (not delta-only) intensity quadrupole J2 and B_Q=2 int_0^1(1-mu²) Q dmu,

\[\partial_x \binom{J_2}{B_Q}\big|_{\mathrm{Th}}=-g\begin{pmatrix}9/10 & 3/40\\4/5 & 2/5\end{pmatrix}\binom{J_2}{B_Q}.\]

Define U=8 J2+B_Q (fast eigenvalue 1), V=4 J2−3 B_Q (slow eigenvalue 3/10), with inverse J2=(3U+V)/28, B_Q=(4U−8V)/28. Exactly C_U=−gU and C_V=−3gV/10. Because source owner's frame-free rank-nine coherency has M_parallel−M_perp=4pi(−J2/2+3B_Q/8), there is the exact *physical* identity

\[ M_{\parallel}-M_{\perp}=-(\pi/2)V. \]

The normal-frame photon *tensor anisotropy* is therefore carried by the **slow Thomson collision eigenmode**, but its streaming equation remains coupled to higher moments.

## Source-actual geometric shear and causal memory

For source's bolometric axisymmetric Liouville transfer with b'=db/dx (fixed source), exact projection by parts gives

\[
S^{\rm sh}_{J_2}=b'(-8J_0/5-10J_2/7+36J_4/35),\qquad
S^{\rm sh}_{B_Q}=b'(10B_Q/7+32c_4/7).
\]

Hence S_U=8 S_J2+S_BQ, S_V=4 S_J2−3 S_BQ. The genuinely new J4_total and deltaJ4 are reprojected from actual saved fixed-source coupled full I field at independent GL64 and GL96; c4 reprojected from Q. **No IVPs were solved**. If a frozen collision+shear subblock with given source functions is assumed, the collision Green response is

\[
H_U(x)=e^{-\tau(x)}\int_0^x e^{\tau(s)} S_U(s)\,ds,\quad
H_V(x)=e^{-3\tau(x)/10}\int_0^x e^{3\tau(s)/10} S_V(s)\,ds.
\]

These are causal shear-memory *diagnostics* calculated with the **original saved tau**, not independently evolved U/V solutions. Equalities for an exact continuum collision system require tau'=g and all other transfer terms included in S. On the sampled source tau′−g is not numerically zero; it is reported. To compare with full precomputed source moments, use P_U=e^−tau U(0), P_V=e^−3tau/10 V(0) and define model defects U−(P_U+H_U), V−(P_V+H_V). These are measured diagnostics, **not a global BDF or CMB observational precision bound**.

## Actual new physical results (all original 50×193)

Output NPZ has 27 new/derived full 50×193 2D channel arrays, plus cases and x; selected genuinely newly evaluated channels are total J4 and polarized deltaJ4, shear sources S_U/S_V, causal Green H_U/H_V, and sampled temporal closure defects. U/V are exact linear transformations of existing PR141 J2/B_Q and must not be counted as independent original observations. 25 matched original s04−s00 pairs include separately the nonzero initial differences and free attenuation, not just shear memory.

| Quantity | Observed |
|---|---:|
| max |U|, max |V| | 2.2102629576424873 ; 1.155622355618821 |
| max |H_U|, max |H_V| | 1.8326510975323411 ; 0.9554962441411127 |
| mean endpoint U, V | −0.8507725604645344 ; −0.44383269680577625 |
| mean endpoint H_U, H_V | −0.8508226147249498 ; −0.44385893739883486 |
| mean 25 paired endpoint ΔU, ΔV | −1.70093297233125 ; −0.8873518118857835 |
| mean paired ΔH_U, ΔH_V | −1.7010331324820203 ; −0.8874043201009342 |
| max *casewise* initial ΔU, ΔV across pairs | 0.041046799947968116 ; 0.020523399973984058 |
| original optical-depth fast survival endpoint range | [0.9330086893104205, 0.9331254890157166] |
| original optical-depth slow survival endpoint range | [0.9794126443814115, 0.9794494254203204] |

**Crucial qualification:** mean paired ΔU and ΔH_U are close in this source sample, but individual matched pairs can have a nonzero initial U/V difference. The valid casewise decomposition is ΔU = ΔP_U + ΔH_U + Δ(defect), with an analogous V identity. Do NOT treat paired means as a source-independent shear-only theorem.

### Preregistered falsifier outcomes

- Maximum original J0/J2/BQ reuse difference **0** (same owner and quadrature), c4 difference 4.61e−19; original J4 baseline GL64 projection 1.16e−15; new total J4 GL64/96 difference 1.85e−15; c4 GL64/96 difference 2.79e−18.
- Exact `Mparallel−Mperp+pi V/2` max absolute **3.997e−15**, fast collision `C_U+gU` max 2.776e−17, slow collision `C_V+0.3gV` max 2.429e−17. Mode inversion max ≤5.73e−17. All pre-fixed 1e−10 angular/algebraic gates PASS without tolerance relaxation.
- Actual saved-clock derivative max |d tau/dx−g| = **1.5625719945869254e−4**, reported, NOT a global timing error certificate.
- Independent cumulative Simpson vs trapezoid Green history maximum gaps: fast **4.471998352495454e−5**, slow **2.3508446110098724e−5**. Not substituted or hidden. Max local derivative residuals U=**2.967874676262916e−4**, V=**8.715095106204274e−5**; max integrated Duhamel defects U=**1.0789806127098345e−4**, V=**5.641632957342768e−5**. These do NOT meet the 1e−10 algebraic tolerance; they belong to a different, explicitly diagnostic time-error class.
- Positive photon coherence survived, max observed |Q|/I=0.012023690610186153. No new IVP. No evidence of actual source-native owner E_l state evolution, observed CMB spectra, electron temperature change or arbitrary Bianchi transport.

## Same-context blind referee, finite review pass

1. **Would the eigenmodes diagonalize complete Boltzmann dynamics?** NO. They exactly diagonalize the *local collision submatrix only*. The shear source contains J0,J4 and c4 and the original unpolarized source remains fixed; no infinite-order closure or published production BASS E_l equivalence is asserted. `UNRESOLVED for full dynamics`, not a strong novel no-go.
2. **Does the eigenmode need original baseline J2?** YES. Using deltaJ2 alone would remove original Thomson forcing and give a physically different mode; all calculations explicitly project total fullI and preserve original g/tau.
3. **Is V gauge-independent observed E?** NO. V is a scalar local axisymmetric *coherency tensor anisotropy* in the fixed normal frame. Mapping it to general observer E_l and cosmological CMB EE/BB requires transported spin-screen and full dynamics; owner original 62-state is unpolarized.
4. **Are matched pairs sourced solely by shear?** NO. Casewise ΔU0 up to 0.041 and ΔV0 up to 0.0205, plus survival factors, require retaining both initial and forced portions. Corrected pairing now records both explicitly.
5. **Do tiny exact projection errors certify time dynamics?** NO. The measured trapezoid/Simpson and local derivative defects are many orders above the exact algebraic residuals; source tau′≠g on saved numerical stencil. These are unresolved numerical/source-clock approximation effects, not an implementation hallucinated full time-convergence PASS.
6. **Does this affect PR #141 or main?** No parent source changes, PR untouched, new isolated child. Historical global authority CI remains binding; external BASS owner commit attribution cannot be rewritten to VigilODE. Distinct-author R6/C9 `NOT_RUN`, same-context audit only; native subagent capability not exposed.

## Literature (exact primary)

- Anthony Challinor (2000), *Microwave background polarization in cosmological models*, Phys. Rev. D **62**, 043004, DOI [10.1103/PhysRevD.62.043004](https://doi.org/10.1103/PhysRevD.62.043004), arXiv [astro-ph/9911481](https://arxiv.org/abs/astro-ph/9911481). PSTF polarized radiative-transfer formalism, source metric (+---); no automatic sign identity with (-+++).
- Andrew Pontzen and Anthony Challinor (2007), *Bianchi model CMB polarization and its implications for CMB anomalies*, MNRAS **380**, 1387–1398, DOI [10.1111/j.1365-2966.2007.12221.x](https://doi.org/10.1111/j.1365-2966.2007.12221.x), arXiv [0706.2075](https://arxiv.org/abs/0706.2075); nearly-FRW anisotropy expansion, including rank-two Thomson source and Type I polarization properties.
- Angelo Marcello Anile (1974), *Anisotropic expansion of the universe and the anisotropy and linear polarisation of the cosmic microwave background*, Astrophys. Space Sci. **29**, 415, DOI [10.1007/BF02639073](https://doi.org/10.1007/BF02639073). Historical axisymmetric Bianchi I polarized transfer.
- Native source [BASS frame-free rank-nine operator](https://github.com/cosmosapjw-quantum/bass/blob/cc602e209e7deeb511ab2a1f3b2268d55206c43c/audit/p3_polarized_thomson.py) and [carrier](https://github.com/cosmosapjw-quantum/bass/blob/cc602e209e7deeb511ab2a1f3b2268d55206c43c/bianchi/q/polarization.py), original commit identity immutable.

## Admission and next question

`SCOPED_SOURCE_PHYSICS=COMPLETE_ONLY_WITHIN_FROZEN_PARENT`, `OFFICIAL_VIGILODE=7/23`, `REI_CENTRAL=28/88`, `REI_WORKING=33/88`, `BASS_CE1=9/15`, `SUCCESSOR_ADMITTED=0/12`, `OFFICIAL_DELTA=0`. `NO_MORE_IVPS,NO_MAIN_MERGE,NO_FORCE_PUSH,NO_PRODUCTION_PROMOTION,AUTO_NEXT_NODE=false; PRODUCTION=HOLD`. Next candidate only: owner-native spin-PSTF E_l mapping to gauge-fixed E_l0(+), fully dynamical shear+Thomson J_l/E_l with clock/chemistry response and full IC/event. No automatic next IVP. No additional user-supplied article is necessary for the current scoped question.
