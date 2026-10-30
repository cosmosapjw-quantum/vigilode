# BASS21 source-owned polarized radiation stress and anisotropic work: theory and hostile audit

2026-10-30 KST. **Parent:** `cosmosapjw-quantum/vigilode` PR132, HEAD `f97b0df6e13e09a67db3050a0e8e925e2e009022`, exact parent ZIP SHA256 `ff17ce0c5883de8363898e83ebcb8b30322d35402d0cdbe4c678437493f6103a`. **Original owner:** `cosmosapjw-quantum/bass@cc602e209e7deeb511ab2a1f3b2268d55206c43c`, frozen original full25/50 history archive SHA256 `6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8`. Source/BASS Liouville blob `e15e1875fba7fcc4a42e257e291c54d9b540a3f0`. Full sample=50×193 original 62-state, 17 parent archived sky directions; this work re-evaluates on independent GL48/GL64/GL96 directions.

## Geometry, conventions and energy stress

Signature (-,+,+,+), `c` explicit in `ds²=-c²dt²+a²e^{-2b}(dy₁²+dy₂²)+a²e^{4b}dy₃²`, x=ln(a/a_ref), a_perp=a e^-b, a_parallel=a e^2b. Normal electron rest frame. Original photon energy brightness F(x,μ)=a⁴I/(a_ref⁴ I_ref) and Legendre J_l=(1/2)∫_{-1}^1 F P_l dμ. Parity-even LRS: J_l=∫₀¹ F P_l dμ. No physical Kelvin normalization or spectral CMB assertion.

The already computed PR132 coupled photon correction is `I=I_base+δI`, `Q=I_meridional-I_azimuthal`, U=V=0, and original b(x), g(x)=cσ_T n_e/H and original τ(x) all frozen. Define δJ_l=∫₀¹ δI P_l dμ. Q influences stress **only through the actual resulting δI**, never by adding Q twice to the intensity. Introduce I_phys_scale=I_ref(a_ref/a)^4; the corrected mixed photon stress is

\[
δT^{a}{}_{b}=I_{phys\_scale}\,\operatorname{diag}\left(-δJ_0,\frac{δJ_0-δJ_2}{3},\frac{δJ_0-δJ_2}{3},\frac{δJ_0+2δJ_2}{3}\right).
\]

Therefore δP_parallel−δP_perp=I_phys_scale δJ2, and δT^a{}_a=0 exactly. Physical energy density, not source-normalized photon-number density. No original shear/Einstein equation is modified.

## Exact continuum angular Liouville and moment derivation

For conserved Killing q_i, the local photon energy E and normal-frame μ obey `dμ/dx=-3 b'(x) μ(1−μ²)` and `d ln(a E)/dx=b'(1−3μ²)`. Thus, for source-normalized bolometric intensity perturbation,

\[
∂_x δI=3b'μ(1-μ²)∂_μ δI+4b'(1-3μ²)δI+g(δS_I−δI).
\]

The exact local linear Thomson response sources from PR132 are

\[
δS_I=j_0+(j_2/2-3B_Q/8)P_2(μ),\quad
S_Q=[-3(J_2^{base}+j_2)/4+9B_Q/16](1-μ²),\quad
B_Q=2\int_0^1Q(1-μ²)dμ.
\]

Angular integration gives `∫₀¹(δS_I−δI)dμ=0`: **Thomson scattering has no energy monopole source in the cold comoving frame**. Integrating the Liouville part by parts yields

\[
\frac{dδJ_0}{dx}=-2b'δJ_2,
\quad δJ_0(x)=-2\int_0^x δJ_2(y)\,db(y),\quad δJ_0(0)=0.
\]

The anisotropy equation is

\[
\frac{dδJ_2}{dx}=b'\left(-\frac85δJ_0-\frac{10}{7}δJ_2+\frac{36}{35}δJ_4\right)
-g\left(\frac9{10}δJ_2+\frac3{40}B_Q\right).
\]

This coefficient and its mixed J4 coupling are **exact for the LRS even-parity continuum angular operator**; they are not an ad hoc closure. The original BASS frozen Liouville source explicitly states
`∂_xF=(s/H)[3μ(1−μ²)∂_μ F−4(3μ²−1)F]+C_T` and unpolarized C2=−(9/10)ΓJ2; the derived equations match with b'=s/H. The source comment mentions an E2 collision correction but leaves polarization off in the original BASS run. A convention-level identification of source-native E2 and our B_Q requires additional source binding before claiming full hierarchy equivalence.

In Wolfram Language exact symbolic integration, the three identities returned `shearJ0 -> -2 j2`, `shearJ2 -> -8j0/5 −10j2/7 +36j4/35`, `collisionMonopole -> 0`, `radiationTrace -> 0`. Independent Python Legendre polynomial quadrature verifies the first three shear matrix columns to approximately 1e-12 (not a different human author).

## Hostile referee: bounded adversarial findings

**Finding S-01, pass:** An attractive but incorrect counting would add Q to T^00 independently. Actual Stokes total intensity is the sum of orthogonal polarizations, so Q changes relative populations but is NOT a separate second photon fluid. Correct use is T[F_base+δI]. Distinct directional tensor integration agrees with J0/J2 stress to <= 7.63e-21.

**Finding S-02, pass with numerical qualification:** The exact energy work identity requires a *differentiable continuum* solution. The source supplies only 193 sampled times and b'; the code evaluates b' by nonuniform-grid finite differences and integrates with cumulative Simpson. This is a diagnostic, not rigorous BDF source/global error certification. All 50 cases max absolute closure residual 4.403e-11 and max case-wise normalized residual 5.766e-5. Do not label these as exact numerical identities.

**Finding S-03, scope hold:** The reconstructed even l=10 response is legitimate source-frozen δJ10 but the *full total J10* was never present in original baseline L8, so no full self-consistent l<=10 photon hierarchy is claimed. Report δJ_l only, preserving the original L8 source status.

**Finding S-04, production blocker retained:** Parent PR132 freezes the unpolarized photon hierarchy, electron density, metric and chemical evolution; despite bidirectional δI↔Q gain, radiation stress does not feed original Einstein/shear or thermal equations. This work cannot close a full coupled cosmic evolution or CMB E/B claim. The original CE1 six-cell and BASS21 193-epoch histories are different events, not mixed.

**Finding S-05, validation limit:** Parent sampled 193/97/49 temporal quadrature differs; no original BDF global error certificate. Max GL64 versus GL96 angular moment gap 1.989e-19; maximum local quadrupole differential budget discrepancy 2.747e-8 due derivative/saved-grid approximation, not presented as zero nor as established divergence. Source owner and all 50 NPZ SHA checks are mandatory.

**Finding S-06, independence:** Wolfram CAS and Cartesian/Legendre independent *methods* actually executed, but no fresh distinct human/model-author blind referee was dispatched. `distinct_author_R6_C9=NOT_RUN`; same-assistant adversarial review is not independent-author certification. No false PASS.

## Source bibliography and input rights

1. Challinor, A. (2000), *Microwave background polarization in cosmological models*, Physical Review D 62, 043004, DOI https://doi.org/10.1103/PhysRevD.62.043004, arXiv:astro-ph/9911481. PSTF covariant polarization source/transport.
2. Pontzen, A. & Challinor, A. (2007), *Bianchi model CMB polarization and its implications for CMB anomalies*, MNRAS 380, 1387–1398, DOI https://doi.org/10.1111/j.1365-2966.2007.12221.x, arXiv:0706.2075. Bianchi polarization transport near FRW, not a license to transfer VII_h dynamics into LRS I.
3. Anile, A. M. (1974), *Anisotropic expansion of the universe and the anisotropy and linear polarisation of the cosmic microwave background*, Astrophysics and Space Science 29, 415–?, DOI https://doi.org/10.1007/BF02639073. Axisymmetric Bianchi I precedent.
4. Kamionkowski, M. (2021), *Cosmological perturbations without the Boltzmann hierarchy*, Physical Review D 104, 063512, DOI https://doi.org/10.1103/PhysRevD.104.063512. Coupled moment integral method; not identical to this finite LRS model.

All formulas here are either re-used from the pinned BASS code or directly derived; references do not independently validate this specific BASS numerical run.