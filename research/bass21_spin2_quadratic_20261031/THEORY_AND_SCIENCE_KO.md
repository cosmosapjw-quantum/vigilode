# VigilODE × original BASS21: LRS spin-2 finite-band quadratic polarization budget

Date: 2026-10-31 (Asia/Seoul). Status: EXPLORATORY_SOURCE_FROZEN_PASS, no owner IVP, official DAG delta 0.

## Source and conventions

Immutable stacked source: `cosmosapjw-quantum/vigilode` Draft PR #148 at `2e29f98379d3a7d56cf3cbcda0c71355fbc0117c`, PR #145 at `6bb55eed42e01db9c3715d17c858d8e5632d6a38`, owner `cosmosapjw-quantum/bass@cc602e209e7deeb511ab2a1f3b2268d55206c43c`. The exact 61,789,058-byte PR148 input is SHA256 `626ea52c9fae38db23e0dd9e5f628ed0cba35ee05d54391b02b91ca34a0b3b1c`, containing corrected 60,191,082-byte PR145 SHA256 `c7626d6f7e01e578e4e9825e60ea8e1631e6ec79cc535867d95fc690b0e0c564`. 50 original 62-state owner histories, 193 original saved x epochs, diagonal LRS Bianchi I metric (-,+,+,+), `x=ln(a/a_ref)`, fixed source background, photon+Thomson causal response, original `g=c sigma_T n_e/H >=0`, `b'(x)`, original source `J2`, originally transported meridional screen with U=0. No physical c=1 assumption has been silently introduced: the original source `g` already carries c and the source `x` is dimensionless.

This is *not* full dynamical E or J evolution. It is a new quadratic diagnostic evaluated on original fixed 50×193 solutions, not a future CMB prediction, a photon-energy density, nor an Einstein stress-energy conservation theorem.

## Exact all-rank finite-band identity

For even l=2,4,..., define `P_l^2(mu)`, `N_l=(l+2)!/[(2l+1)(l-2)!]` on mu in [0,1], `k_l=sqrt(4pi N_l)`, `E_l=k_l c_l`, and finite sky `Q_L(mu)=sum_{l<=L} c_l P_l^2(mu)`. The spin-2 LRS projection is orthogonal on the half sphere (even l). Therefore, exactly (not approximately):

`int_0^1 Q_L^2 dmu = sum_{l<=L} E_l^2/(4 pi)`, `P_L = 1/2 int_0^1 Q_L^2 = sum E_l^2/(8 pi)`.

In the exact original projection convention the shear adjoint is `Ddag f = B f - D f'`, with `B(mu)=1-3mu^2` and `D(mu)=3mu(1-mu^2)`. Since `D(0)=D(1)=0`, integration by parts gives the primal `L Q=B Q+(D Q)'`. Also `D'=3B`. Hence the pointwise identity

`Q L Q - (5/2)B Q^2 = (1/2)(D Q^2)'`

and consequently, for every sufficiently regular Q,

`int_0^1 Q L Q dmu = (5/2) int_0^1 (1-3mu^2) Q^2 dmu`.

The equality holds for all finite-band `Q_L` and is exact, not an almost-FRW linearization; the all-rank projector of the *specific LRS source* is being used. It does not establish a general non-FLRW 1+3 PSTF transport operator. The physical `Q^2+U^2` polarization norm is invariant under local screen SO(2) rotation; in the special native transported LRS basis U=0.

## Boundary and Thomson decomposition

For L in {16,18,20,22}, the original coefficient `A_E[l,m]=(k_l/k_m) A_c[l,m]` couples only l±2. The local source generator available for l<=22 is `S_l=b'(x) sum_{m<=24} A_E[l,m] E_m`. Let `F_L=b'(x) A_E[L,L+2] E_{L+2}` from PR145/PR148 (source-checked exactly).

The source-conditioned quadratic model budget is

`P'_L|local = W_internal + W_boundary + C_quad_source + C_damping`, where

`W_internal=(5/2) b' int_0^1 (1-3mu^2) Q_L^2 dmu`,

`W_boundary=E_L F_L/(4 pi)`,

`C_quad_source= -g k_2 J2 E2/(16 pi)`,

`C_damping= -g[(2/5) E2^2 + sum_{4<=l<=L} E_l^2]/(4 pi) <=0`.

The source's l=2 term is `C_E2=-g(k2 J2/4+2E2/5)` and each even l>=4 is `C_El=-g El`. The quadratic loss is sign-definite only for the pure damping term, not for `C_quad_source`. The boundary work is a bilinear *quadratic-norm source term*, not physical radiative stress-energy flux, and its smallness is not a global truncation-error certificate.

The true saved-grid derivative `gradient_x P_L` is compared with the *model local RHS* and retained as an independent defect; no BDF time certification follows. A model RHS conditioned on a source trajectory need not equal the physical time derivative in a different source-owned system.

## Actual source-data results

The output `results/SOURCE_FROZEN_SPIN2_POWER_50x193.npz` stores six primary source-evaluated fields (quadratic power; internal shear work; boundary work; quadrupole Thomson source; Thomson loss; cumulative boundary work) at 50×193×4 = 38,600 physical source positions, **231,600 primary values**; separately stores 77,200 model-RHS and saved-grid-derivative-defect diagnostics. Also 50 endpoint rows, 25 originally matched s00/s04 shear pairs, 193 epoch means, and three actually rendered scientific figures.

| L | max |upper boundary work| | max |integrated boundary work| |
|---|---:|---:|
| 16 | 4.704802882081204e-21 | 1.7722340263123452e-21 |
| 18 | 1.267632788347763e-22 | 5.554948784706011e-23 |
| 20 | 5.072367524876884e-24 | 2.4488874765198833e-24 |
| 22 | 2.628794186099942e-25 | 1.2904893271949079e-25 |

Max P_L = 4.901605159507037e-05; max internal shear work = 2.673715420347411e-06; max Thomson damping amplitude = 9.532655673664713e-07. `max|source-grid P'_L - model RHS| = 3.423109385053337e-08`, explicitly *not* a global time-error certificate.

Actual parent shear generator array identity difference = 0; parent boundary flux array difference = 0; collision reconstruction difference = 6.94e-18; two independently implemented angular norm/work reconstructions match the source Galerkin results to maximum 8.14e-20 / 9.32e-21, independently differentiated polynomial basis work difference 1.19e-20. No tolerance relaxation or owner source replacement.

## Adversarial status and limits

- S00 high-l has source GL64/96 quadrature floor ~1e-16; do not claim physically resolved high-l s00 boundary work. The strong shear s04 family supports a more meaningful source-resolved signal but remains source-frozen.
- Small `W_boundary` reflects product `E_L F_L`: this is an exact *local* bilinear source contribution, not a rigorous upper bound for omitted all-l couplings or for any full nonlinear coupled solution error.
- This is an optical/photonic polarization quadratic form, not an Einstein–Vlasov energy tensor or BASS CE1 shear-energy theorem; do not conflate them.
- Actual general observer boost mixes Doppler, aberration and screen Lorentz map; local basis SO(2) rotation is not observer transformation. The fixed LRS U=0 bridge does not establish arbitrary observer screen/PSTF hierarchy.
- Parent full source contains original 50 owner histories but no independent owner dynamic E_l IVP in this run; original source BDF time accuracy and J/E/Einstein/electron/temperature joint evolution remain unresolved.
- No native subagent available in the host, no distinct-author R6 or C9. One same-author hostile review and symbolic and numeric crosschecks only.
- PR148 global authority CI previously FAIL, independent current-child CI must be observed separately. No source or main merge, force push, production change or more IVPs.

## Literature and source hierarchy

Primary: Anthony Challinor, *Microwave background polarization in cosmological models*, Physical Review D 62, 043004 (2000), https://doi.org/10.1103/PhysRevD.62.043004 : exact general tensor transfer, but explicit hierarchy almost-FRW. Andrew Pontzen & Anthony Challinor, *Bianchi model CMB polarization and its implications for CMB anomalies*, MNRAS 380, 1387-1398 (2007), https://doi.org/10.1111/j.1365-2966.2007.12221.x : Bianchi hierarchy in nearly-FRW limit, not exact general nonlinear source. A. M. Anile, *Anisotropic expansion of the universe and the anisotropy and linear polarisation of the cosmic microwave background*, Astrophys Space Sci 30 (1974), https://doi.org/10.1007/BF02639073 : axisymmetric Bianchi I polarized transfer. Exact user-owned LRS source equations and numerical data take precedence over porting any linearized external hierarchy.

## Authority / next

Official DAG locked at VigilODE 7/23, REI central 28/88 working 33/88, BASS CE1 9/15, successor 0/12, delta 0. `PRODUCTION=HOLD; NO_MORE_IVPS; NO_MAIN_MERGE; NO_FORCE_PUSH; NO_PRODUCTION_PROMOTION; AUTO_NEXT_NODE=false`.

One bounded new science result has been computed and checked; it is an auxiliary source-scoped diagnostic, not completion of a canonical node. Next candidate only: owner-native general screen-PSTF basis transport and true coupled J_l/E_l with original Einstein/electron/temperature/clock, using RKTE-COVSPIN N12 local collision Lorentz-map as an independent interface reference but NOT substituting its model for BASS owner source.
