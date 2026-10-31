# BASS21 LRS original Q24 -> nonaxial observer E/B (2026-10-31)

**Claim type:** derived exact passive local Lorentz screen transformation + implementation-verified source-frozen scientific experiment. No owner IVPs, no general non-FLRW PSTF hierarchy, no observed CMB prediction or physical velocity measurement. Official DAG VigilODE **7/23**, REI central **28/88**, REI working **33/88**, BASS CE1 **9/15**, admitted successor **0/12**, official delta **0**.

## Source-locked physical system and scope

Original 50 BASS21 LRS cases × 193 saved `x=ln(a/a_ref)` epochs from nested, byte-verified owner source in unchanged VigilODE PR153 full archive (SHA256 `e8faa022fb6e567a97f17f73cca4571f071ffa37de5ea8e9597b0c4d98b088bc`). The original extracted E-mode input `results/HIGH_E_HIERARCHY_50x193.npz` has SHA256 `b087a54bcd3bdbfaef5d91165634bf5c7590e6c43417112e4de0168327b3366b`. Original E_l0+ (l=2,4,...,24) has never been refitted or recomputed. The original sky is **a band-limited reconstruction**, not all l of the original photon kinetic solution. Bolometric brightness convention inherited, not Kelvin.

Fix signature (-,+,+,+), c retained in physical observer velocity v=beta c; in formulas only beta=v/c is dimensionless. Choose original LRS +z axis, outgoing photon **propagation** n and local source screen dyad (e_theta,e_phi). Polarization tensor on source screen has Stokes Q24 and U=0; source Q24= sum_l E_l0 / k_l P_l^2(cos theta), k_l²=4 pi (l+2)!/((2l+1)(l-2)!). Exact sky transport changes field at the **same event** between co-located observers, not along cosmic time.

## General nonaxial Lorentz and screen map

Let observer velocity vector b (|b|=beta<1), bhat=b/beta, gamma=(1-beta²)^(-1/2). For a source photon null four-vector k=(1,n), the moving observer sees k'=(D,D n') with

    D = gamma(1-b·n)
    n' = [n+((gamma-1)(bhat·n)-gamma beta)bhat]/D .

At a chosen observer direction n', use the exact inverse map

    Dinv=gamma(1+b·n'),
    n=[n'+((gamma-1)(bhat·n')+gamma beta)bhat]/Dinv,
    D=1/Dinv.

Transport a source screen basis vector e=(0,e_theta) via the Lorentz matrix. Its time component becomes -gamma beta bhat·e. Remove this time component by adding the null gauge multiple of k' to get unit transverse *observer*-screen vector

    t=e+(gamma-1)(bhat·e)bhat+gamma beta (bhat·e)n'.

With observer canonical spherical basis (e'_theta,e'_phi), set z=(t·e'_theta)+i(t·e'_phi), |z|=1. Phase orientation is fixed by the exact Doppler-weight-one null E->B control:

    (Q'+iU')_d = D^d (conj z)^2 Q24,  (Q'-iU')_d = D^d z^2 Q24.

For actual source-bolometric polarization d=4. The weight-one d=1 is an **artificial transformation-control** for the same angular field and is not the same physical bolometric observable. For axial b || z, z=1 and U'=0. For nonaxial b, U' is generally nonzero.

Spin-weighted Y use _sY_lm=sqrt((2l+1)/(4pi)) d^l_(m,-s)(theta) e^{im phi}, with Wigner small-d evaluated by finite factorial sum. Source normalization is matched exactly: _sY_l0=P_l²/k_l for s=+/-2. Define a_minus=∫(Q'+iU') conj(_{-2}Y_lm) dOmega', a_plus=∫(Q'-iU') conj(_{+2}Y_lm) dOmega', E_lm=(a_minus+a_plus)/2, B_lm=(a_minus-a_plus)/(2i). The sign of B depends on explicitly fixed screen/Stokes convention; |B| and d=1 null result do not. Do not confuse local U with globally geometric B.

### Exact small-velocity pure-quadrupole identity (derived; Wolfram separately checked)

Take b=beta xhat, pure E20=1 so Q=C(1-u²), C=3/k2, u=cos(theta'). At first order the source inverse aberration gives mu=u-beta u sqrt(1-u²)cos(phi), D=1-beta sqrt(1-u²)cos(phi), and screen phase psi=arg(z)=-beta u sin(phi)/sqrt(1-u²). Since Q+iU uses conjugate(z)², expansion yields

    delta Q=beta C sqrt(1-u²) cos(phi)[-d+(d+2)u²],
    delta U=+2 beta C u sqrt(1-u²) sin(phi).

For l=2,m=1 the normalized spin harmonics reduce to +/- sqrt(5/(4pi))*sqrt(1-u²)(1 +/- u)/2 e^(i phi). The exact angular integral for B21 gives

    B21/E20 = i*(d-1)*beta/sqrt(6) + O(beta²).

Wolfram integral: ∫_{-1}¹ (1-u²)[-d+(d+4)u²]du = -16(d-1)/15; normalized coefficient (d-1)/sqrt(6), symbolic difference **0**. Thus d=1 has no first-order leakage; for d=4 coefficient is i*sqrt(3/2) beta. The all-orders d=1 E/B nonmixing under a full-sky boost is supported by Dai & Chluba (2014). The finite beta=.0001 numerical derivative differs from exact coefficient by 1.2248e-8, consistent with O(beta²) in derivative.

## Actually generated physical data

Five *controlled observer frames*, not measured velocities: b=(0,0,0), (+.001,0,0), (+.05,0,0), (-.05,0,0), and (+.05/sqrt2,0,+.05/sqrt2). For 50 original histories x 193 epochs, project **all m** for l=2,...,8, exactly 77 complex modes per source point and frame. 3,715,250 complex E_lm and 3,715,250 complex B_lm produced, including exact numerical zeros at rest and source-frozen induced nonzero B in nonaxial frames.

Max |B| across all original states/modes by frame: rest 1.3868575029493984e-15, x001 4.288115794370951e-5, x005 0.0021494293264436044, negx005 0.0021494293264436026, tilt45 0.0015189255354318796. On 25 s04 source-matched cases at the last epoch, mean Im B21: +x005 +0.0015703920032512997, -x005 -0.0015703920032512986, tilt45 +0.0011097403785418239, x001 +3.132935179421644e-5. These are the source-owned original Q24 amplitudes, not a universal normalization or observable CMB spectrum.

## Scoped tests and quantified uncertainty

- Original PR153 archive and nested PR151+PR148+PR145 E input SHA exact; arrays are 50x193x12, no new IVP.
- Baseline E_l0 recovered at rest to max 5.26058e-16; rest B max 1.38686e-15.
- Independent GL(72,96) vs GL(96,128) **source-weighted** differences: max E 3.5070e-16, max B 2.1143e-17. Source-free pure kernel differences ~1.42e-14.
- Artificial d=1 full-sky B mixing from original source max 1.3923e-15; actual bolometric d=4 B max 0.0021494 at beta=.05.
- Spin spherical-harmonic reality E max 1.15361e-16 and B max 2.79417e-15; +/-x observer reflection B max 3.19929e-18.
- Reciprocal photon direction max 3.33e-16, Doppler inverse max 4.44e-16, screen phase inverse max 6.23e-15; original-vs-observer whole-sky norm by dOmega'=D^-2 dOmega tested independently at source endpoint.
- d1 nonmix is a **scientifically essential negative control**. Initial trial used the opposite spin phase and incorrectly produced d1 B~0.02039 for unit pure E20 at beta=.05; corrected sign passed both zero mixing and d4 B. No prior false calculation promoted.

## What remains unresolved

This is exact local *Lorentz* transport of bandlimited saved source sky and independent spherical harmonic re-projection. It is **not** generic 1+3 screen-PSTF Liouville+Thomson hierarchy, arbitrary spacetime free propagation, actual physical electron velocity, photon stress feedback, observational B-mode, or independent-author R6/C9. No finite observer-output l<=8 Parseval norm/roundtrip asserted. Large original source s00 high multipoles may lie under original quadrature noise. Native user-source full J_l/E_l and Einstein/thermal/electron/clock coupling are still outstanding, and no formal DAG node or production gate is advanced.

## Primary literature

- A. Challinor, *Microwave background polarization in cosmological models*, Phys. Rev. D 62 043004 (2000), DOI 10.1103/PhysRevD.62.043004: exact general polarization tensor, explicit hierarchy only almost-FRW.
- A. Challinor and F. van Leeuwen, *Peculiar velocity effects in high-resolution microwave background experiments*, Phys. Rev. D 65 103001 (2002), DOI 10.1103/PhysRevD.65.103001: Stokes and bolometric boost transformations.
- L. Dai and J. Chluba, *New operator approach to the CMB aberration kernels in harmonic space*, Phys. Rev. D 89 123504 (2014), DOI 10.1103/PhysRevD.89.123504: d=1 no E/B mixing, d≠1 possible.
- S. Yasini and E. Pierpaoli, *Generalized Doppler and aberration kernel for frequency-dependent cosmological observables*, Phys. Rev. D 96 103502 (2017), DOI 10.1103/PhysRevD.96.103502: s and d kernels, d=4 bolometric mixing.
