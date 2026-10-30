# BASS21 원본 rank-9 coherency ↔ Stokes ↔ screen-fixed E-mode 교차 검증

2026-10-30. 독립 Draft 연구 브랜치. 원본 owner BASS 역사 50개 × 193 saved epochs(62-state)를 **불변** 입력으로 사용했다. 본 문서의 새 50-case 계산은 선행 PR #138 고차 spin-2 연구 뒤에 실시된 별도의 preregistered 계산이다. BASS original source commit은 [cosmosapjw-quantum/bass의 정확한 커밋](https://github.com/cosmosapjw-quantum/bass/commit/cc602e209e7deeb511ab2a1f3b2268d55206c43c)이며, VigilODE 로컬 Git commit이 아니다. 원본 ZIP SHA256 `6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8`; 부모 PR138 source-full ZIP SHA256 `ab7044f604fdd7badb7eb21bae60f7900dd47e1e81b0ed1cce3091283162309f`.

## Definition, conventions, and exact derivation [derived, Wolfram-checked]

Metric `(-,+,+,+)`; normal-comoving cold-electron LRS Bianchi I; `x=ln(a/a_ref)`; `g=c*sigma_T*n_e/H` inherited from source. Original geometry/temperature/electron/optical clock and unpolarized `J_l` are frozen; a pre-existing **two-field Volterra** linear polarization response `deltaI,Q` is re-evaluated on these saved epochs. Not a new IVP or self-consistent Einstein-thermal/radiative solve. Screen basis is the transported local spherical pair `(e_theta,e_phi)`, and `Q=I_theta-I_phi`, `U=V=0`. The source angular brightness is `I=I_original+deltaI`, and `|Q|<=I` is the physical positive-coherency bound. The spherical screen basis itself has coordinate singularities at the poles, while physical Q vanishes there; uniform sky orientation remains a model-gauge convention, not a source-native global CMB E/B gauge guarantee.

Let `P_ab(n)=delta_ab-n_a*n_b`, `T_ab(n)=e_theta,a*e_theta,b-e_phi,a*e_phi,b`. Define the transverse symmetric coherency

```
J_ab(n) = (I(n)/2)*P_ab(n) + (Q(n)/2)*T_ab(n),
M_ab = integral_over_S2 J_ab(n) dOmega.
```

The original BASS owner uses the frame-free rank-nine cold Thomson gain `K[J](n)=(3/(8*pi))*P(n)*M*P(n)` (source `audit/p3_polarized_thomson.py`, Git blob `ea97f1699c3491e607595113faf4e216233a002e`, and `bianchi/q/polarization.py`, blob `7448ea6701f236fd41d22f3c5cadfd80f482d82f`). In an axially symmetric even sky the azimuthal integration yields `M=diag(M_perp,M_perp,M_parallel)`, where

```
J0 = <I>, J2=<I*P2(mu)>, BQ=2<Q*(1-mu**2)>,
M_perp = 4*pi*(J0/3+J2/6-BQ/8),
M_parallel = 4*pi*((J0-J2)/3+BQ/4),
tr M = 4*pi*J0, M_parallel-M_perp = 4*pi*(-J2/2+3*BQ/8).
```

Direct projection of `K[J]` to the same spherical screen gives **exactly**

```
S_I = J0 + (J2/2-3*BQ/8)*P2(mu),
S_Q = (-3*J2/4+9*BQ/16)*(1-mu**2).
```

Consequently the cold Thomson collision on the full sky satisfies

```
C_J0 = 0,
C_J2 = -g*(9*J2/10+3*BQ/40),
C_BQ = -g*(4*J2/5+2*BQ/5),
```

and its homogeneous `(J2,BQ)` collision matrix after factoring `g` is `[[-9/10,-3/40],[-4/5,-2/5]]`; eigenvalues `(-1,-3/10)` exactly. These were checked with independent Wolfram symbolic evaluation as well as local SymPy `test_rank9.py`; **the source has nonzero original J2 forcing**, so do not confuse full-J2 collision with the previous PR132 `deltaJ2` correction. The original rank-9 coherency gain has an integrated symmetric-traceless eigenvalue 7/10, and the corresponding collision mode decays at rate 3g/10; distinct tensor sectors need not all share this rate.

Fix one plus-sign spin-harmonic convention for even `l>=2`, `m=0`:

```
{}_2 Y_l0(mu) = sqrt((2*l+1)/(4*pi) * (l-2)!/(l+2)!)*P_l^2(mu),
P_l^2=(1-mu**2)*d_mu^2 P_l,
N_l=(l+2)!/((2*l+1)*(l-2)!),
c_l=<Q*P_l^2>/N_l,
E_l0^(+) = sqrt(4*pi/(2*l+1) * (l+2)!/(l-2)!)*c_l,
B_l0^(+) = 0 for transported-screen U=0, even-axisymmetric full sky.
```

The equally common CMB definition `E_lm=-(a_2,lm+a_-2,lm)/2` yields the **opposite sign** from this explicitly defined `E_l0^(+)`. Therefore this is a fixed-screen mathematical E projection, *not* an identity with original BASS owner dynamically evolved `E_l`. The latter is absent in original unpolarized L<=8 BASS history. Original source `bianchi/q/polarization.py` explicitly limits generic global E/B to diagnostic status without proven continuously transported screen gauge, a restriction respected here. For the previously PR134-defined collision-normalized quadrupole, exact normalization is

```
E20^(+) = sqrt(20*pi)*E2_candidate, E2_candidate=3*BQ/(4*sqrt(6)).
```

This can be used to transport numerical values between explicitly labelled conventions, not to upgrade dynamical or observational claims. Scalar Boltzmann brightness is source-normalized: physical temperature in kelvin, frequency response, line-of-sight observer remapping, stochastic angular power spectra and full Einstein response have not been derived.

## Actual 50x193 original-source measurements [numerically checked]

The archived source is recursively unpacked and byte-validated. For each case, the unchanged parent Volterra `solve_case(h,nangular=48)` evaluates `fullI,Q` on GL64+GL96 independent half-sky output angles. The new code computes `Mperp,Mparallel,Delta`, Stokes gain at five specified directions, collision rates for `J2,BQ`, and `E_l0^(+)` for l=2,4,...,16. Twelve selected source epochs (3 cases × 4 saved times) additionally integrate the **explicit 3D spherical coherency tensor** at 16 equally spaced azimuths, rather than merely using the analytic tensor projection.

Source-derived physical array entries: `Mperp,Mparallel,Delta` 50*193*3=28,950; `E_l0^(+)` 50*193*8=77,200; gain `S_I,S_Q` at 5 outgoing directions 50*193*5*2=96,500; two Thomson collision rates 50*193*2=19,300. Additional saved arrays include J0,J2,BQ, source collision E2, normalized anisotropy, c_l and explicit zero-B diagnostic. At each saved epoch the original cold Thomson extinction/gain and source clock are unchanged. Count physical entries, not independent samples or additional IVP solutions.

Observed extrema from full original 50x193 (source brightness units unless stated):

- max |E20^(+)| = 0.035012283834940566; mean final E20^(+)=+0.012791443004615054; final E20 positive in 37/50, negative 13/50.
- mean 25 matched (s04-s00) final Delta E20^(+) = +0.025577680075470167.
- max |E12,0^(+)| = 1.5757402797312216e-7; in s00 high rank is near quadrature noise, so no blanket detection claim.
- max |M_parallel-M_perp| = 1.8152473513681096; max |(M_parallel-M_perp)/(4pi J0)| = 0.13628477978266815.
- max |C_J2|=0.020765784047076238 and |C_BQ|=0.018381216700318524.

Validation and defect budgets (all **maximum absolute**, not relative, and source-sampled):

- tensor-vs-Stokes moment algebra max `3.552713678800501e-15`;
- trace invariant max `1.7763568394002505e-15`;
- gain `S_I` max `4.440892098500626e-16`, `S_Q` max `4.996003610813204e-16`;
- explicit 3D tensor vs axisymmetric reduction max `3.108624468950438e-14`;
- independent GL64/96 tensor difference max `6.217248937900877e-15`; spin harmonic `E_l0^(+)` difference max `8.12928952343229e-17`;
- original PR134 c2..10 match max `1.3010426069826053e-18`; PR138 c12..16 match **exact double numbers** in this calculation; parent frozen Volterra fixed-point residual max `4.6330127234650575e-15`.

These measurements pass the registered **absolute 1e-10** source-projection consistency targets. They do NOT bound the original BDF time-grid global error, source optical-depth uncertainty, spectral polarization or calibration. In particular `small residual on a saved response` does not make a frozen-source solution a self-consistent Einstein–Boltzmann evolution.

## Blind-style adversarial review (same author, CAS-assisted)

1. **Potentially fatal sign/normalization trap**: Several CMB references define `E=-E_plus`. The source owner additionally lacks an evolved native `E_l`. We explicitly label the plus convention and preserve an exact sign reversal, avoiding a fabricated owner equivalence. **Resolved for scoped claims, open for native E_l**.
2. **Tensor convention trap**: rank-nine gain coefficient is `3/(8*pi)` for the specific coherency normalization `J=I*P/2+Q*T/2`; no extra 2 may be inserted. A pure isotropic I manufactured sky leaves gain I unchanged and Q zero; a pure I quadrupole generates the source Q with negative sign. Tests pass.
3. **Double-counted original J2 trap**: the full tensor calculation uses `J2_total=J2_original+deltaJ2`, not a standalone deltaJ2 collision expression. Comparing to PR132 requires subtracting the frozen unpolarized baseline only for the perturbation sector. Corrected by explicit definition and tests.
4. **Screen gauge and polarity**: Global B=0 follows only for this explicit continuous parity-even screen and U=0. It is not asserted for arbitrary tilted/non-LRS events or observed CMB, nor for owner-native frame-independent spin-2 mode mapping without a gauge witness. **OPEN for full native target**.
5. **Closure**: The exact rank-nine collision operator is low rank, but the Liouville shear transport still couples l to l+2; the preceding PR138 measured nonzero upper flux. Rank-nine collision does NOT close the polarized evolution of all l. **OPEN for full native target**.
6. **Numerical validation is bounded**: small GL difference establishes angular consistency only, not source temporal error, rigorous interval enclosure or full multigroup frequency convergence. No relaxation of preregistered tolerance; tested 50/50 source physical cases.
7. **Reproducibility and authority**: Exact owner BASS Git commit is a different repository, not a VigilODE local commit. Typed repository binding and nested immutable ZIP readback are required; historical global CI remains unresolved. Independent distinct-author R6/C9 review remains NOT_RUN, native subagent NOT_RUN.

**Decision:** The exact frame-free rank-nine ↔ Stokes reduced collision proof is DERIVED and CAS-checkable; all 50x193 source frozen rank-nine/E_l0^(+) data are NUMERICALLY_CHECKED and implementation verified after targeted/clean replay. Full BASS-native dynamic J_l/E_l closure is UNRESOLVED, thus no official DAG promotion, production HOLD, merge HOLD.

## Source bibliography (actual primary documents)

- A. Challinor, *Microwave background polarization in cosmological models*, Phys. Rev. D 62, 043004 (2000), DOI `10.1103/PhysRevD.62.043004`, `https://arxiv.org/abs/astro-ph/9911481`. Original metric (+---) must be converted when comparing with our (-+++).
- A. Pontzen and A. Challinor, *Bianchi model CMB polarization and its implications for CMB anomalies*, MNRAS 380, 1387–1398 (2007), DOI `10.1111/j.1365-2966.2007.12221.x`, `https://arxiv.org/abs/0706.2075`. Near-FRW transport paper; no assumed equality with finite LRS source e.g. observed spectrum.
- BASS owner source: [audited cold Thomson rank-nine tensor](https://github.com/cosmosapjw-quantum/bass/blob/cc602e209e7deeb511ab2a1f3b2268d55206c43c/audit/p3_polarized_thomson.py) and [basis-independent carrier](https://github.com/cosmosapjw-quantum/bass/blob/cc602e209e7deeb511ab2a1f3b2268d55206c43c/bianchi/q/polarization.py).
