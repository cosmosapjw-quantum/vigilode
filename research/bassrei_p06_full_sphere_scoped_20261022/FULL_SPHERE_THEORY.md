# BASS/rei P06 (scoped): full-sphere photon transport in triaxial Bianchi I

Date: 2026-10-22 KST. Node directory: `research/bassrei_p06_full_sphere_scoped_20261022/`, on base commit
`031cd20b`.

This document derives the full-sphere `(q, μ, φ)` photon transport of a non-tilted triaxial Bianchi-I
geometry. It defines the discrete operator, the rank-6 Thomson kernel, the state layout and the operation
order a Rust adapter must follow to reach bit parity with the oracle `FULL_SPHERE_FVM_REFERENCE.py`. It
also states the polarization positivity contract and lists the obstacles held back (HOLD).

The oracle is an independent pure-Python float formula reference. No Rust code is written or changed by
this node. Run the checks with:

```
cd research/bassrei_p06_full_sphere_scoped_20261022
python3 FULL_SPHERE_FVM_REFERENCE.py CHECKS.json      # 38 checks, writes CHECKS.json
```

## 0. Status

| Item | Status | Where |
|---|---|---|
| Drift `(q̇, μ̇, φ̇)` from conserved covariant momenta | proven, exact check | §2, `a1` |
| Liouville identity `3H + 3q̇ + ∂_μ μ̇ + ∂_φ φ̇ = 0` | proven, exact unisolvent certificate | §3, `a2` |
| LRS drift reduction (equals the P02 operator) | proven, exact check | §4, `a3` |
| Exact triaxial characteristics and their Liouville Jacobian | proven, checked against RK45 rays | §5, `g1`–`g3` |
| Discrete first-order upwind transport on `q`, `μ` and periodic `φ` | implemented; conservation proven | §6, `b1`–`b6` |
| Discrete LRS reduction to P01 `lrs_number_rhs` | proven in exact arithmetic; 1.0e-15 in float | §6.7, `c1`, `c2` |
| A θ-only reduction is not closed under triaxial shear | proven; numerical certificate | §6.8, `c3` |
| Rank-6 Thomson: conservation, isotropy, positivity, rank, LRS limit | proven, implemented | §7, `d*`, `e*`, `f1` |
| State layout, Jacobian band, bitwise Jacobian contract, `f_t` | implemented, checked | §8, `b2`, `b3`, `b6` |
| Bit-parity specification and golden fixture | specified | §9 |
| Polarized axisymmetric `(I, Q)` Thomson kernel and positivity contract | proven, implemented (contract only) | §11, `a6`, `h*` |
| Tilt, aberration, recoil, class-B geometry, full-sphere polarized transport | **HOLD** | §12 |
| Rust adapter, stage solvers, time integration, performance | not part of this node | §12.6 |

**Scope.** The scope is:

- prescribed geometry: non-tilted, vorticity-free Bianchi I, with every structure constant zero;
- the proper time of the normal congruence as the clock;
- unpolarized photons for the transport;
- cold elastic Thomson scattering: no recoil and no electron motion;
- a first-order finite-volume (FV) scheme.

Units follow P01 §2 (`BASS_REI_PORT_RESEARCH.md`).

## 1. Setup and notation

Signature `(−,+,+,+)` and metric `ds² = −dt² + Σ_k a_k(t)² (dx^k)²`, with
`H_k = ȧ_k/a_k`, `H = (H_1+H_2+H_3)/3` and `σ_k = H_k − H` (so `Σσ_k = 0`).

The metric does not depend on `x^k`, so `∂_k` are Killing vectors and the covariant momenta `p_k` are
conserved along null geodesics. The orthonormal (physical) components are `P_k = p_k/a_k`. The energy is
`E = |P|` (null), the direction is `n_k = P_k/E`, and

$$n = (\sqrt{1-\mu^2}\cos\varphi,\ \sqrt{1-\mu^2}\sin\varphi,\ \mu),\qquad q = \ln(E/E_{\rm ref}).$$

Further notation:

- `f` is the occupation per polarization state, and `κ = g/((2πħ)³c³)` (P01 §2).
- The Liouville density is `U = a³ κ E³ f`, with `a³ = a_1 a_2 a_3`.
- The cell contents are `N_ijk = ∫_cell U dq dμ dφ`, using `dΩ = dμ dφ`.
- `Σ N / a³` is the physical photon number density.
- A dot denotes `d/dt` along the ray, with `dt/dλ = p^0 = E`.

## 2. Drift (Theorem 1)

For every `(q, μ, φ)` on the sphere minus the poles:

$$\dot q = -\sum_k H_k n_k^2,\qquad \dot\mu = -\mu(1-\mu^2)(H_3-H_\varphi),\qquad
\dot\varphi = (H_1-H_2)\sin\varphi\cos\varphi,$$

with `H_φ = H_1 cos²φ + H_2 sin²φ`. In shear form, `q̇ = −H − Σ σ_k n_k²`.

**Proof.**

1. Since `p_k` is constant, `P_k(t) = p_k/a_k(t)` and `Ṗ_k = −H_k P_k`.
2. Then `Ė = Σ P_k Ṗ_k / E = −E Σ H_k n_k²`, which gives `q̇`.
3. Let `S := Σ H_l n_l² = −q̇`. Then `ṅ_k = Ṗ_k/E − n_k Ė/E = −n_k (H_k − S)`.
4. `μ = n_3`, so `μ̇ = −μ (H_3 − S)`. With `n_1² + n_2² = 1 − μ²` we have `S = (1−μ²) H_φ + H_3 μ²`,
   hence `H_3 − S = (1 − μ²)(H_3 − H_φ)`.
5. `φ = atan2(n_2, n_1)`, so `φ̇ = (n_1 ṅ_2 − n_2 ṅ_1)/(n_1² + n_2²)
   = (H_1 − H_2) n_1 n_2/(n_1² + n_2²) = (H_1 − H_2) sin φ cos φ`. ∎

**Remarks.**

- `ṅ = −(σ n − (n·σ n) n)` is minus one half of the tangential gradient of `n·σ n`: the angular flow is a
  gradient flow and has no periodic orbits.
- `μ̇` vanishes at `μ = ±1`.
- `φ̇` is bounded but `φ` is singular at the poles (measure zero). The discrete operator never evaluates
  `φ̇` at a pole: φ faces are meridian half-planes, and their velocity does not depend on `μ`.
- `φ = 0, π/2, π, 3π/2` (the principal axes) are fixed points of `φ̇`.

**Exact check `a1`.** Dual numbers over `Fraction` differentiate `P_k(t) = p_k/(1 + H_k t)` at `t = 0` for
40 rational directions with rational `|p|` (inverse stereographic points, scaled). They reproduce `q̇`,
every `ṅ_k`, `μ̇` and `φ̇` exactly.

## 3. Liouville identity and conservative form (Theorem 2)

$$3H + 3\dot q + \partial_\mu\dot\mu + \partial_\varphi\dot\varphi = 0\qquad(3H = H_1+H_2+H_3).$$

**Proof.** We have `∂_μ μ̇ = −(H_3 − H_φ)(1 − 3μ²)` and `∂_φ φ̇ = (H_1 − H_2)(cos²φ − sin²φ)`. Write
`c = cos φ` and `s = sin φ`, and collect the coefficient of each `H_k`:

- `H_3`: `1 − 3μ² − (1 − 3μ²) = 0`.
- `H_1`: `1 − 3(1−μ²)c² + c²(1 − 3μ²) + (c² − s²) = 1 − c² − s² = 0`.
- `H_2`: `1 − 3(1−μ²)s² + s²(1 − 3μ²) − (c² − s²) = 1 − s² − c² = 0`. ∎

**Exact certificate `a2`.** The left side `L(μ, φ; H)` is linear in `H`. For fixed `H` it lies in the
four-dimensional space `span{1, μ²} ⊗ span{1, cos 2φ}`. A 2×2 grid with two distinct values of `μ²` and
two distinct values of `cos 2φ` is unisolvent for that space.

`a2` evaluates `L` in exact `Fraction` arithmetic at `μ ∈ {0, 1/2}` and `φ ∈ {0, π/2}` (rational circle
points `τ = tan(φ/2) ∈ {0, 1}`) for `H = e_1, e_2, e_3`. All values vanish, which proves the identity.

`a2` also checks:

- 42 further rational points under 6 further random rational geometries, for redundancy (414 exact
  evaluations in all);
- `∂_μ μ̇` by dual numbers in `μ`;
- `∂_φ φ̇` by dual numbers in `τ`, divided by `dφ/dτ = 2/(1+τ²)`.

**Conservative form.** In the collisionless case `f` is constant along rays, so

$$\frac{dU}{dt}\Big|_{\rm ray} = (3H + 3\dot q)\,U = -(\partial_\mu\dot\mu + \partial_\varphi\dot\varphi)\,U
= -(\nabla\cdot v)\,U$$

(using `∂_q q̇ = 0`), which is the continuity equation along `v = (q̇, μ̇, φ̇)`:

$$\partial_t U + \partial_q(\dot q U) + \partial_\mu(\dot\mu U) + \partial_\varphi(\dot\varphi U) = a^3\kappa E^3 C_t[f].$$

The total `∫ U dq dμ dφ` changes only through the `q` boundaries and collisions.

**Comoving energy.** Let `x = ln(aE/E_ref)` with `a = (a_1 a_2 a_3)^{1/3}`. Then `ẋ = −Σ σ_k n_k²`, and `U`
is unchanged (the shift `x − q = ln a` has unit Jacobian). The rates `μ̇` and `φ̇` depend only on
differences `H_k − H_l`. The comoving operator is therefore the same operator evaluated with
`(σ_1, σ_2, σ_3)` in place of `(H_1, H_2, H_3)`.

## 4. LRS reduction (Theorem 3)

Let `H_1 = H_2 = H − s` and `H_3 = H + 2s`. Then:

- `φ̇ = 0`;
- `H_φ = (H − s)(cos²φ + sin²φ) = H − s`, so `H_3 − H_φ = 3s` and `μ̇ = −3 s μ (1 − μ²)`;
- `q̇ = −[(H − s)(1 − μ²) + (H + 2s) μ²] = −H − s(3μ² − 1)`.

This is exactly the P02 operator (`PREREGISTRATION.md` of P02; P01 §2). Check `a3` verifies all three
identities in `Fraction` arithmetic at 60 random rational points.

## 5. Exact characteristics (Theorem 4)

Let `A_k(t, t_0) = a_k(t)/a_k(t_0) = exp ∫_{t_0}^{t} H_k dt`. Because `p_k = a_k P_k` is conserved,
`P_{0,k} = A_k P_k`. Then

$$E_0 = E\sqrt{\textstyle\sum_k A_k^2 n_k^2},\qquad n_{0,k} = A_k n_k E/E_0,\qquad
U(t,q,\mu,\varphi) = \Big(\prod_k A_k\Big)\Big(\frac{E}{E_0}\Big)^3 U_0(q_0,\mu_0,\varphi_0),$$

with:

- `q_0 = q + ½ ln Σ A_k² n_k²`;
- `μ_0 = n_{0,3}`;
- `φ_0 = atan2(A_2 sin φ, A_1 cos φ)`.

**Proof.** `f(t, P) = f_0(P_0)` along the ray (collisionless). `U = a³κE³f`, so
`U/U_0 = (a(t)/a(t_0))³ (E/E_0)³ = (Π A_k)(E/E_0)³`. ∎

**Corollary.** `|∂(q_0, μ_0, φ_0)/∂(q, μ, φ)| = (Π A_k)(E/E_0)³`. This is the Liouville theorem for the
map, and it implies that the integral of `U` is conserved. In the LRS case, `A_1 = A_2 = A` and `A_3 = B`
reproduce the P02 closed form.

**Checks.**

| Check | What it compares | Result | Bound |
|---|---|---|---|
| `g1` | Closed form against `scipy` `solve_ivp(RK45, rtol=1e-12, atol=1e-14)`, for the time-dependent `H_k(t) = c_k/(1+t) + d_k cos(ω_k t)` (closed-form `ln A_k`), over 20 rays including near-pole and near-wrap points | max error 2.2e-13 | 1e-9 |
| `g2` | Finite-difference Jacobian determinant of the closed-form map against the factor | relative error 3.5e-10 | 1e-7 |
| `g3` | Gauss–Legendre helper exactness on polynomial data, and the total of the exact cell contents | total conserved to 1.9e-9 | 1e-6 |

The `g1` integration covers `(q, μ, φ, ln U)`, with `d ln U/dt = (H_3 − H_φ)(1 − 3μ²) − (H_1 − H_2) cos 2φ`.
It compares `q_0`, `μ_0` and `φ_0` (wrapped), and `ln[(Π A_k)(E/E_0)³]`.

## 6. Discrete transport operator

### 6.1 Grids (modelling choices)

- **Energy.** `q_edges` (`nq + 1` values) are strictly increasing and finite, and every width is a normal
  float (at least `f64::MIN_POSITIVE`).
- **Polar angle.** `mu_edges` (`nm + 1` values) are strictly increasing, with `mu_edges[0] == −1.0` and
  `mu_edges[nm] == 1.0` exactly.
- **Azimuth.** `phi_edges` (`nphi + 1` values) are strictly increasing, with `phi_edges[0] = φ0`,
  `0 ≤ φ0 < TAU` and `phi_edges[nphi] == φ0 + TAU` as an f64 sum. Here `TAU = 2.0*math.pi`, which is
  `std::f64::consts::TAU` = 6.283185307179586.
- The last φ edge is the periodic image of the first. There are `nphi` φ faces: face `l ∈ 0..nphi` sits
  at `phi_edges[l]` and separates cell `(l − 1) mod nphi` from cell `l`. The wrap face is evaluated at
  `phi_edges[0]`, never at `phi_edges[nphi]`.
- **Uniform helper.** `phi_edges[l] = φ0 + (TAU*l)/n` for `l < n`, and `phi_edges[n] = φ0 + TAU`.
- With `φ0 = 0`, the wrap face lies on a fixed point of `φ̇` (`sin 0 = 0`) and carries no flux. Any
  `φ0 ≠ k π/2` makes the wrap carry flux. The checks use both.

### 6.2 Exact face-averaged normal velocities

The geometry quantities, computed once per call, are:

- `h̄ = (H_1 + H_2)/2` and `h_d = (H_1 − H_2)/2`;
- `H_1 cos²φ + H_2 sin²φ = h̄ + h_d cos 2φ`, whose average over φ cell `k` is
  `hp_k = h̄ + h_d ⟨cos 2φ⟩_k`;
- `g_k = H_3 − hp_k`.

The face-averaged normal velocities are:

| Face | Velocity | Why it is the exact face average |
|---|---|---|
| `q` face of angular column `(j,k)` | `v^q_{jk} = −(H_3 ⟨μ²⟩_j + ⟨1−μ²⟩_j hp_k)` | `q̇` is separable in `(μ, φ)`, so its cell average is the product of the averages |
| interior `μ` face `l` of column `(i,k)` | `v^μ_{lk} = −g_k w_l`, with `w_l = μ_l(1−μ_l)(1+μ_l)` | average over the φ cell of `μ̇(μ_l, φ)` |
| φ face `l` | `v^φ_l = h_d sin(2 φ_l)` | equals `φ̇(φ_l)`; `φ̇` does not depend on `μ` or `q` |

### 6.3 Flux, upwind rule and boundaries

- **Flux.** `flux = v · N_up / h_up`, where `h_up` is the width of the upwind cell normal to the face
  (`dq`, `dm` or `dphi`). This equals the face-averaged velocity times the upwind cell average of `U`
  times the face measure.
- **Tie rule.** If `v >= 0.0`, the upwind cell is the lower-index cell: `i−1` for `q`, `j−1` for `μ`,
  `(l−1) mod nphi` for φ. This includes `v = −0.0`, because `−0.0 >= 0.0` is true in IEEE arithmetic.
  Otherwise the upwind cell is the upper one. This is the P01/P02 rule.
- **Energy boundaries.** With `outflow`, the inflow through faces `0` and `nq` is zero and the outflow
  goes to the ledgers `L_lo` / `L_hi`. With `closed`, faces `0` and `nq` are skipped.
- **Pole faces.** The faces `μ = ±1` are never evaluated (`w = 0` there).
- **Azimuth.** The φ direction is periodic.

### 6.4 Conservation (Proposition 1)

Every interior face flux enters two cells with opposite signs. A boundary `q`-face flux enters one cell and
one ledger. Hence `Σ rhs + out_lo + out_hi = 0` identically in exact arithmetic, for all data and geometry.

On the state layout of §8 this gives `Σ_r f_r(y) = 0`: every Jacobian column sums to zero. Check `b1`
(120 random cases, both boundary kinds, 1 ≤ nphi ≤ 6, offsets `φ0 ∈ {0, 0.3, 2.9, 5.5}`) gives a
maximal relative residual of 8.8e-17 against the bound 1e-14.

### 6.5 Positivity of forward Euler

Each cell's right-hand side is `−λ_c N_c` plus nonnegative multiples of upwind neighbours. With
`dt · max_c λ_c ≤ 1`, forward Euler is positive; `grid_transport_cfl` returns `1/max λ`. With Thomson
rate `Γ`, the condition `dt (max λ + Γ) ≤ 1` suffices, because the gain kernel is at least 3/4 (§7.4).
Check `b4` covers both.

No claim is made for Rosenbrock steps (P01 §3).

### 6.6 Consistency and convergence

The face velocities are exact face averages and the reconstruction is piecewise-constant upwind, so the
scheme is first-order consistent.

On a `μ` face where `H_3 − H_φ` changes sign inside a φ cell (`H_3` between `H_1` and `H_2`, as in the
convergence case), upwinding by the sign of the face average stays first-order consistent and keeps Euler
positivity.

**Check `g4` (reported, no order gate).** The setup is:

- `H = (0.3, −0.1, 0.05)` and `T = 1`;
- `U_0 = exp(−q²/0.5)(1 + 0.4n_1 + 0.3n_1n_2 − 0.2n_3 + 0.25n_3²)` (smooth on the sphere, no symmetry);
- `q ∈ [−3, 3]`, `φ0 = 0.25`, outflow boundaries;
- the exact sparse operator is assembled from the oracle by 27-colour probing and checked to 1e-14 against
  the oracle on a random vector;
- `expm_multiply` gives the time integration;
- exact cell contents come from 3-point Gauss–Legendre in `(q, θ, φ)`.

| `nq × nm × nphi` | cells | relative L1 error | observed order |
|---|---|---|---|
| 12 × 6 × 6 | 432 | 0.0670 | – |
| 24 × 12 × 12 | 3456 | 0.0447 | 0.58 |
| 48 × 24 × 24 | 27648 | 0.0270 | 0.73 |

The errors decrease monotonically and the order rises. This is the pre-asymptotic approach of a first-order
scheme: on the coarsest grid the numerical diffusion `Δq |v| T ≈ 0.1` is comparable to `σ² = 0.25`. The
P02 pilot showed the same behaviour (orders 0.52 → 0.875).

### 6.7 Discrete LRS reduction (Proposition 2)

If `H_1 == H_2` (f64), then:

- `h_d = 0.5*(H_1 − H_2) = +0.0` exactly;
- `hp_k = h̄ + 0·c2_k = H_1` exactly, so `v^q_{jk}` does not depend on `k`;
- `v^φ_l = ±0`, so the φ fluxes are `±0` and leave every value unchanged;
- `v^μ_{lk} = −(H_3 − H_1) w_l`.

Take axisymmetric data `N_ijk = N^LRS_ij g_k`, with `g_k = Δφ_k/TAU`. In exact arithmetic,
`Σ_k rhs_ijk = rhs^LRS_ij` and the outflows agree. Here the LRS operator is P01's `lrs_number_rhs` with
`H = (2H_1 + H_3)/3` and `s = (H_3 − H_1)/3`.

**Proof.** `⟨μ²⟩ + ⟨1−μ²⟩ = 1` exactly, so `−(H_3⟨μ²⟩ + H_1⟨1−μ²⟩) = −H − s(3⟨μ²⟩ − 1)`. Also
`Σ_k g_k = 1`, and `(H_3 − H_1) w = 3s μ(1−μ²)`. ∎

The image of axisymmetric data stays axisymmetric (`c2`: defect 3.5e-16).

**Floating point.** The 3D sum is not bit-identical to the 2D oracle, because the expressions for `⟨μ²⟩`
and the φ summation differ. Check `c1` (6 φ grids including nphi = 1, 2, 3, 5, 7, 16 with offsets,
6 geometries, both boundaries, 60 cases) gives a maximal relative error of 1.0e-15 against the bound
1e-14.

**Bit parity is defined between the Rust 3D adapter and this 3D oracle, not between 3D and 2D.** One more
caveat: a column whose `q` velocity is within rounding of zero can pick a different upwind cell in 2D and
in 3D. The flux difference is then bounded by `|v|·N/dq`, which is at the rounding level.

### 6.8 Why a θ-only reduction is not closed (Proposition 3)

Average the conservation law over φ (the φ-flux divergence integrates to zero over a period). With
`Ū = ⟨U⟩_φ`, the averaged fluxes are

$$\langle\dot q U\rangle = -[H_3\mu^2 + (1-\mu^2)\bar h]\,\bar U - (1-\mu^2)h_d\langle\cos 2\varphi\,U\rangle,\qquad
\langle\dot\mu U\rangle = -\mu(1-\mu^2)\big[(H_3-\bar h)\,\bar U - h_d\langle\cos 2\varphi\,U\rangle\big].$$

The φ-averaged system depends on the azimuthal moment `m = 2`. Its own evolution couples `m` to `m ± 2`
(through `h_d cos 2φ` in `q̇`, `μ̇` and `φ̇ = h_d sin 2φ`), which gives an infinite hierarchy. The hierarchy
closes only for `h_d = 0`. Axisymmetric data do not stay axisymmetric.

**Numerical certificate `c3`** (fixed upwind pattern, `h_d = 0.02` and `0.01`):

- At first order in time, the φ-summed derivative of axisymmetric data equals the θ-only operator with
  `H_⊥ = h̄` (relative gap 2.1e-16).
- The non-axisymmetric part of `dN/dt` is linear in `h_d` (ratio 2.0000000000000018 on halving).
- The φ-summed **second** derivative differs from the θ-only one by exactly `h_d² C`, because
  `A(h_d) = A_0 + h_d A_1` at a fixed upwind pattern, and the `O(h_d)` terms vanish after the φ sum by
  telescoping and period integrals. The ratio is 3.99999999999995 and the gap is 4.1e-3, against a
  rounding threshold of 1e-6 relative.

A θ-only shortcut therefore has an `O(h_d² t²)` error in the angle-integrated contents. It is wrong, not
merely less accurate.

## 7. Rank-6 Thomson

### 7.1 Continuum kernel and the real quadrupole basis

The cold unpolarized Thomson phase function is `(3/16π)(1 + cos²Θ) = (1/4π)[1 + ½ P_2(cos Θ)]`. The
addition theorem gives `P_2(n·n') = (4π/5) Σ_m Y_2m(n) Y_2m(n')`. Hence

$$C[f] = \Gamma\Big[\frac1{4\pi}\int f\,d\Omega' + \frac1{10}\sum_m Y_{2m}(n)\int Y_{2m}f\,d\Omega' - f\Big].$$

**Basis and normalization (modelling choice).** Unnormalized polynomial harmonics with rational weights:

| m | `b_m` | in `(μ, φ)` | `w_m` | orthonormal `Y = sqrt(10 w_m/4π) b_m` |
|---|---|---|---|---|
| 0 | `(3z² − 1)/2` | `P_2(μ)` | 1/2 | `sqrt(5/4π) P_2` |
| 1 | `xz` | `μ√(1−μ²) cos φ` | 3/2 | `sqrt(15/4π) xz` |
| 2 | `yz` | `μ√(1−μ²) sin φ` | 3/2 | `sqrt(15/4π) yz` |
| 3 | `x² − y²` | `(1−μ²) cos 2φ` | 3/8 | `sqrt(15/16π)(x² − y²)` |
| 4 | `xy` | `(1−μ²) sin φ cos φ` | 3/2 | `sqrt(15/4π) xy` |

With these, `½ P_2(n·n') = Σ_m w_m b_m(n) b_m(n')` and

$$C[f] = \Gamma\Big[\frac1{4\pi}\int f\,d\Omega' + \frac1{4\pi}\sum_m w_m b_m(n)\int b_m f\,d\Omega' - f\Big].$$

**Proof of the identity.** Expand `(n·n')² = x²x'² + y²y'² + z²z'² + 2(xy x'y' + xz x'z' + yz y'z')`. Use
`x² + y² = 1 − z²` to write `x²y'² + y²x'² = (1−z²)(1−z'²) − x²x'² − y²y'²`. Substituting gives
`(1/8)(3z²−1)(3z'²−1) + (3/8)(x²−y²)(x'²−y'²) + (3/2)(xz x'z' + yz y'z' + xy x'y') = (3/4)(n·n')² − 1/4`. ∎

Check `a4` verifies the identity exactly at 60 pairs of rational points of S².

Elastic scattering keeps `E`, so the same operator acts on `U` group by group: the factor `a³κE³` commutes
with it.

### 7.2 Discrete operator (piecewise-constant Galerkin)

For angular cell `c = (j, k)`, define:

- `Ω_c = d_j Δφ_k`;
- `B_{m,c} = Ω_c^{-1} ∫_c b_m dΩ` (exact);
- `ω_c = (d_j/2)(Δφ_k/TAU) = Ω_c/4π`.

For one energy group,

$$\dot N_c = \Gamma\Big[\omega_c\Big(M + \sum_{m=0}^4 w_m B_{m,c} Q_m\Big) - N_c\Big],\qquad
M = \sum_c N_c,\quad Q_m = \sum_c B_{m,c}N_c.$$

Here `M` is the monopole (`m0` in the code) and `Q_m` are the five quadrupole moments (`mom[m]`).

This follows from inserting `U = N_c/(Δq Ω_c)` on each cell and integrating `C` over the cell. It is
diagonal loss plus rank 6 (monopole plus 5 quadrupoles) per energy group.

### 7.3 Exact cell averages in closed form

The averages factor as `B_{m,c} = F_{m,j} G_{m,k}`:

| m | `F_{m,j}` | `G_{m,k}` |
|---|---|---|
| 0 | `p2_j` | 1 |
| 1 | `r_j` | `⟨cos φ⟩_k` |
| 2 | `r_j` | `⟨sin φ⟩_k` |
| 3 | `t2_j` | `⟨cos 2φ⟩_k` |
| 4 | `t2_j` | `⟨sin φ cos φ⟩_k` |

**μ cell `[a, b]`, `d = b − a`.** Write `e_a = (1 − a)(1 + a)`, `e_b = (1 − b)(1 + b)`, `s = √e`.

- `m2 = (a² + ab + b²)/3 = ⟨μ²⟩`.
- `t2 = (e_a + e_b)/2 + d²/6 = ⟨1 − μ²⟩`. Every term is nonnegative, so there is no `1 − x` cancellation
  near the poles.
- `p2 = m2 − t2/2 = ⟨P_2⟩`.
- `r = (a + b)(e_a + s_a s_b + e_b) / (3(s_a + s_b)) = ⟨μ√(1−μ²)⟩`. This follows from
  `∫ = (s_a³ − s_b³)/3`, a difference of cubes and `s_a − s_b = d(a+b)/(s_a + s_b)`. The width `d` cancels.
- Guard: `s_a + s_b = 0` only for the single cell `[−1, 1]`, where `r = 0` by symmetry (rule 2.2: no
  division by a possibly zero value).

**φ cell `[α, β]`, `Δ = β − α`.**

- `⟨cos φ⟩ = cos((α+β)/2) · sin(Δ/2)/(Δ/2)`.
- `⟨sin φ⟩ = sin((α+β)/2) · sin(Δ/2)/(Δ/2)`.
- `⟨cos 2φ⟩ = cos(α+β) · sin Δ/Δ`.
- `⟨sin φ cos φ⟩ = ½ sin(α+β) · sin Δ/Δ`.

These are the product (half-angle) forms of the difference quotients `(sin β − sin α)/Δ` and its
relatives, which cancel catastrophically on narrow cells.

**Checks.**

- `d1` compares against `mpmath` at 40 digits, including width 1e-9 and pole cells of width 1e-6. The
  maximal absolute error is 1.03e-16. The naive difference quotient errs by 1.8e-8 at width 1e-9.
- `a5` verifies `d·m2 = (b³ − a³)/3`, `d·t2 = d − (b³ − a³)/3`, `d·p2 = ((b³−b) − (a³−a))/2` and `m2 + t2 = 1`
  exactly in `Fraction` arithmetic.

### 7.4 Properties

1. **Group number.**

   $$\sum_c \dot N_c = \Gamma\Big[(\textstyle\sum_c \omega_c) M + \sum_m w_m Q_m \sum_c \omega_c B_{m,c} - M\Big],$$

   with `Σ ω_c = 1` and `Σ ω_c B_{m,c} = (4π)^{-1} ∫ b_m dΩ = 0`. For `m = 0` the zero comes from
   `∫_{−1}^{1} P_2 = 0`, exact in rationals (`a5`). For `m ≥ 1` it comes from the period integrals of
   `cos φ`, `sin φ`, `cos 2φ` and `sin φ cos φ`, by telescoping periodic primitives (`d6`: below 1e-50 at
   50 digits). Hence `Σ_c Ṅ_c = 0` identically for exact edges. In f64 the edge `φ0 + TAU` differs from
   `φ0 + 2π` by at most an ulp. Check `d2` gives a relative residual of 2.9e-16 for `Γ` up to 1e6.
2. **Isotropic stationarity.** For `N_c = c Ω_c`: `M = 4πc` and `Q_m = c ∫ b_m = 0`, so the gain is
   `ω_c · 4πc = N_c`. Check `d3` gives 5.1e-16.
3. **Positivity.** The discrete kernel `K_{cc'} := 1 + Σ_m w_m B_{m,c} B_{m,c'}` is the double cell average
   of `1 + ½ P_2(n·n')`. That function is at least 3/4, because `P_2 ≥ −1/2`. The gain matrix `ω_c K_{cc'}`
   is therefore entrywise positive, and forward Euler with `Γ dt ≤ 1` is positive.
   - `d4` compares `K` against 10-point Gauss quadrature of the continuum kernel (agreement 2.0e-15;
     `min K = 0.852 ≥ 0.75`).
   - `d5` checks the Euler step.
4. **Rank.** `T + Γ I = Γ diag(ω) V D Vᵀ`, with `V = [1, B_0, …, B_4]` (`M × 6`) and `D = diag(1, w)`.
   Since `ω > 0` and `D ≻ 0`, `rank(T + ΓI) = rank V ≤ 6`.
   - **Sufficient conditions for rank 6:** `p2` is not constant over `j`, `r ≠ 0`, and the φ vectors
     `{1, ⟨cos φ⟩, ⟨sin φ⟩, ⟨cos 2φ⟩, ⟨sin φ cos φ⟩}` are independent in `R^nphi` (so `nphi ≥ 5`).
   - **Proof:** row `j` of `Σ α V = 0` reads
     `(α_0 + α_1 p_j) 1 + α_2 r_j cφ + α_3 r_j sφ + α_4 t_j c2 + α_5 t_j s2h = 0`. φ independence forces
     each coefficient to vanish; then `t_j > 0`, `r ≠ 0` and `p` non-constant give `α = 0`.
   - With `nphi < 5`, rank 6 can still be reached through the different μ factors.
   - **Check `e1`:** rank 6 on (nm, nphi) = (4, 5), (3, 6), (6, 8) and (4, 3). The 6th singular value lies
     in 0.11–0.52 and the 7th is at most 8.7e-16.
   - **Predicted degenerate ranks, verified by `e3`:**
     - uniform `nphi = 4` gives rank 5 (`⟨cos 2φ⟩ ≡ 0`);
     - uniform `nphi = 2` gives rank 3;
     - `nphi = 1` gives rank 2 (the LRS kernel);
     - a symmetric two-cell μ grid gives rank 5 (`p2` constant).
   - Stage solvers must accept rank-deficient factors. A zero column of `U`, `V` contributes an identity
     row to the Woodbury capacitance.
5. **LRS limit.** For axisymmetric `N_c = N_j g_k`:
   - `Q_m = Σ_j F_{m,j} N_j Σ_k G_{m,k} g_k = 0` for `m ≥ 1`;
   - `M = Σ_j N_j` is the 2D `M0`;
   - `Q_0 = Σ_j p_j N_j` is the 2D `M2`;
   - `Σ_k ω_c = d_j/2`.

   Hence `Σ_k Ṅ_c = Γ[(d_j/2)(M0 + ½ p_j M2) − N_j]`, the P01/P02 rank-2 form. Check `f1` gives 2.2e-15,
   and the non-axisymmetric moments `Q_{1..4}` are at most 3.7e-16 relative.
6. **Low-rank factors** (`thomson6_factors`):
   - `U = [ω, ω∘w_0B_0, …, ω∘w_4B_4]` and `V = [1, B_0, …, B_4]`;
   - the per-group Jacobian is `Γ(U Vᵀ − I)` (`e2`: 1.5e-16);
   - the right-hand side uses the S form of §9.5, which equals `U Vᵀ N` only to rounding.

## 8. State layout, Jacobian and `f_t`

**Layout.** This is the P02 layout with `nm` replaced by `M = nm·nphi`:

```
y = [L_lo, (N_{i,0..M-1}, A_i) for i in 0..nq, L_hi],   n = 2 + nq*(M + 1)
cell(i, j, k) = 1 + i*(M + 1) + j*nphi + k     absorbed(i) = 1 + i*(M + 1) + M
LO = 0     HI = n - 1     cells-only vector index: i*M + j*nphi + k
```

For `nphi = 1` this is exactly P02 (`cell(i, j, 0) = 1 + i(nm+1) + j`). The index maps are a bijection
onto `0..n` (`b2`).

**Ledgers.**

- `L_lo` accumulates `−flux` at `q` face 0 (the number leaving downward), and `L_hi` accumulates `+flux`
  at face `nq`.
- `A_i` accumulates the absorbed number `Σ_c k̄_i N_{ic}`. The absorber is P02's, with one cell-averaged
  rate `k̄_i` per energy group, independent of angle.
- The sum of all components is the semi-discrete linear invariant.

**Bandwidth.** `lower = upper = M + 1`. The `q` coupling has offset exactly `M + 1`; the Thomson blocks
(offset at most `M − 1`) and the ledger rows lie inside. `b2` observes 13 for `M = 12`.

**Jacobian contract (§8.3).** Accumulate the dense or band Jacobian from `+0.0` through the entries
template `grid_state_jacobian_entries`, which visits only the upwind contributions in the right-hand side's
own loop order:

- transport: `coef = v·(1.0/h_up)`, with `add(lower, col, −coef)` then `add(upper, col, coef)`;
- Thomson: `J[r,c] = Γ·(ω_r·s − δ_rc)`, where `s = 1.0`, then
  `s += Aw[m][r]*B[m][c]` for `m = 0..4` in order, and `δ_rc` is the f64 `1.0` or `0.0`;
- absorber: `−k̄_i` on the diagonal, then `+k̄_i` into `A_i`.

Every column then equals `state_rhs(e_c)` bit for bit. `b6` checks 3178 entries on four configurations,
including `nphi = 1, 2` and both boundary kinds. Why this holds:

- skipped contributions are `±0.0`, and an accumulator that starts at `+0.0` never becomes `−0.0`;
- a unit vector gives `M_0 = 1.0` and `M_m = B_{m,c}` exactly.

**`f_t`.** For a fixed upwind pattern the operator is linear in `(H_1, H_2, H_3)`. So `f_t` is the
transport with velocities computed from `Ḣ` and upwind selection taken from `H(t)` (the oracle's `select`
argument, P02's `transport(h, s, vh, vs)`), plus the absorber terms `−dk̄_i/dt · N`. The Thomson rate is
constant. `b3` checks linearity and that `f_t` matches a central difference.

**JVP.** The JVP is the right-hand side applied to the direction, at the same `t`.

## 9. Bit-parity contract (what the Rust adapter must implement)

### 9.1 Arithmetic rules

- **Format.** IEEE-754 binary64, round to nearest.
- **No FMA.** Never write `mul_add`; Rust does not contract `a*b + c` by itself.
- **Association.** Every expression is evaluated exactly as parenthesized below, and `a + b + c` means
  `(a + b) + c`. A single product or a sum of two operands is commutative in IEEE arithmetic; only the
  association matters.
- **Divisions.** `x/2.0` and `0.5*x` are identical. `/3.0`, `/6.0` and `/TAU` are true divisions, not
  multiplications by reciprocals.
- **Powers.** Squares and cubes are written as products. Do not use `powf`, unlike P02, which mirrored
  the 2D oracle's `**3`.
- **Transcendentals.**
  - `sqrt` is correctly rounded.
  - `sin` and `cos` come from the platform libm: Python `math` uses C libm, and Rust `f64::sin`/`cos` use
    the LLVM intrinsic, which is libm on Linux glibc.
  - The oracle run used glibc 2.39 on x86_64.
  - If the libm differs, compare the precomputed constants of the golden fixture first. Ulp differences are
    possible only in the φ constants (`cphi`, `sphi`, `c2`, `s2h`, `sn2`) and everything downstream of them.
- **Tie rule.** Upwind selection uses `v >= 0.0` (true for `−0.0`).

### 9.2 Grid constants (once per grid)

```
dq[i]   = q[i+1] - q[i]          dm[j] = mu[j+1] - mu[j]          dphi[k] = phi[k+1] - phi[k]
# mu cell j, a = mu[j], b = mu[j+1], d = dm[j]
m2[j]   = ((a*a + a*b) + b*b) / 3.0
ea      = (1.0 - a) * (1.0 + a);         eb = (1.0 - b) * (1.0 + b)
t2[j]   = (ea + eb) / 2.0 + (d*d) / 6.0
p2[j]   = m2[j] - t2[j] / 2.0
sa = sqrt(ea); sb = sqrt(eb); den = sa + sb
r[j]    = if den > 0.0 { ((a + b) * ((ea + sa*sb) + eb)) / (3.0*den) } else { 0.0 }
e[j]    = d / 2.0
# phi cell k, al = phi[k], be = phi[k+1], D = dphi[k]
g[k]    = D / TAU
half    = 0.5 * D;      sinc_half = sin(half) / half;      mid = 0.5 * (al + be)
cphi[k] = cos(mid) * sinc_half;          sphi[k] = sin(mid) * sinc_half
tot     = al + be;      sinc = sin(D) / D
c2[k]   = cos(tot) * sinc;               s2h[k] = 0.5 * (sin(tot) * sinc)
# interior mu faces l = 1..nm-1 and periodic phi faces l = 0..nphi-1
w[l]    = mu[l] * ((1.0 - mu[l]) * (1.0 + mu[l]))
sn2[l]  = sin(2.0 * phi[l])
# Thomson, c = j*nphi + k, W = (0.5, 1.5, 1.5, 0.375, 1.5)
B[0][c] = p2[j];  B[1][c] = r[j]*cphi[k];  B[2][c] = r[j]*sphi[k]
B[3][c] = t2[j]*c2[k];  B[4][c] = t2[j]*s2h[k]
Aw[m][c] = W[m] * B[m][c];               om[c] = e[j] * g[k]
```

### 9.3 Geometry (per call, and per `select` geometry)

```
hbar = 0.5 * (H1 + H2);    hd = 0.5 * (H1 - H2)        # refuse if either is non-finite
hp[k] = hbar + hd * c2[k];  gk[k] = H3 - hp[k]
vq[j*nphi + k] = -(H3 * m2[j] + t2[j] * hp[k])
```

The flux velocities use `(H1, H2, H3)`. The upwind selection uses the same formulas applied to `select`,
which defaults to `(H1, H2, H3)`.

### 9.4 Transport: operation order

Accumulators start at `+0.0`. `x` is the cells-only vector. On the state layout, `out[LO]` and `out[HI]`
are `out_lo` and `out_hi`, and a cell's value is the transport value before Thomson adds to it.

```
# Stage Q: columns j-major then k; energy faces l = 0..=nq within each column
for j in 0..nm: for k in 0..nphi:
    jk = j*nphi + k;  v = vq[jk];  sel = vq_sel[jk]
    for l in 0..=nq:
        if closed and (l == 0 or l == nq): continue
        up = if sel >= 0.0 { l-1 } else { l }
        val = if 0 <= up < nq { x[up*M + jk] / dq[up] } else { 0.0 }
        flux = v * val
        if l > 0 { rhs[(l-1)*M + jk] -= flux } else { out_lo -= flux }
        if l < nq { rhs[l*M + jk] += flux } else { out_hi += flux }
# Stage MU: i-major, then k, then interior faces l = 1..nm-1
for i in 0..nq: for k in 0..nphi: for l in 1..nm:
    v = -(gk[k] * w[l]);  sel = -(gk_sel[k] * w[l])
    up = if sel >= 0.0 { l-1 } else { l }
    flux = v * (x[i*M + up*nphi + k] / dm[up])
    rhs[i*M + (l-1)*nphi + k] -= flux;   rhs[i*M + l*nphi + k] += flux
# Stage PHI: i-major, then j, then periodic faces l = 0..nphi-1
for i in 0..nq: for j in 0..nm: for l in 0..nphi:
    v = hd * sn2[l];  sel = hd_sel * sn2[l]
    lo = if l > 0 { l-1 } else { nphi-1 }
    up = if sel >= 0.0 { lo } else { l }
    flux = v * (x[i*M + j*nphi + up] / dphi[up])
    rhs[i*M + j*nphi + lo] -= flux;      rhs[i*M + j*nphi + l] += flux
```

For `nphi = 1`, `lo == l == 0`, and the face subtracts and adds `±0` (`sn2[0]`'s flux) to the same cell.

### 9.5 Thomson and absorber (state right-hand side, after transport)

```
if rate != 0.0:                          # skipped entirely when rate == 0.0 (as in P02)
  for i in 0..nq:
    m0 = 0.0; mom[0..5] = 0.0
    for c in 0..M: n = N[i][c]; m0 += n; for m in 0..5: mom[m] += B[m][c] * n
    for c in 0..M:
      s = m0; for m in 0..5: s += Aw[m][c] * mom[m]
      out[cell(i,c)] += rate * (om[c] * s - N[i][c])
if absorber:
  for i in 0..nq:
    absorbed = 0.0
    for c in 0..M: rr = kbar[i] * N[i][c]; out[cell(i,c)] -= rr; absorbed += rr
    out[absorbed(i)] += absorbed
```

The standalone `thomson6_rhs` returns `rate * (om[c] * s - N[c])` for one group, in the same order.

### 9.6 Golden fixture

`parity_fixture()` defines `q = [−1, −0.25, 0.5, 1.75]`, `mu = [−1, −0.375, 0.25, 1]`,
`phi = uniform_phi_edges(4, 0.25)`, contents `1 + (c%7)·0.125 − (c%3)·0.0625`, `H = (0.3, −0.1, 0.05)`,
`select = (−0.2, 0.4, 0.1)`, `rate = 2.5` and `kbar = [0.2, 0.0, 0.7]`.

`CHECKS.json → diagnostics.parity_fixture` holds the hex (`float.hex`, which Rust can parse bit-exactly) of
every input, every grid constant and the outputs:

- transport `outflow`, `closed` and `select`;
- one Thomson group;
- the full state right-hand side.

It also holds `sha256_outputs = 1514967febca0332cee1775d44d711d57996ca556fce57aa680cdbd437db941a`. A Rust
parity test should compare element-wise hex first and the digest second.

## 10. Typed refusals

Every refusal is a `FullSphereRefusal(ValueError)` subclass with a stable `code`. The suggested Rust
mapping is `CoreError::InvalidInput` for value errors and `CoreError::Dimension` for `*.length`.

| class | codes |
|---|---|
| `GridRefusal` | `{q,mu,phi}.too_few_edges`, `.not_a_sequence`, `.not_real`, `.nonfinite_edge`, `.width` (non-increasing or non-finite width), `.subnormal_width`; `mu.not_full_sphere`; `phi.not_periodic_cover` (`φ0 ∉ [0, TAU)` or last edge `≠ φ0 + TAU`); `grid.cell_count` |
| `StateRefusal` | `state.length`, `state.nonfinite` |
| `GeometryRefusal` | `geometry.not_real`, `geometry.nonfinite`, `geometry.overflow` (`(H1 ± H2)/2` overflows), `geometry.select` |
| `RateRefusal` | `rate.invalid` (Γ non-finite or negative), `absorber.invalid` |
| `BoundaryRefusal` | `boundary.invalid` |
| `CharacteristicRefusal` | `characteristic.amplification` (`A_k` must be finite and > 0), `characteristic.direction`, `characteristic.quadrature` |
| `PolarizationRefusal` | `polarization.length`, `polarization.nonfinite`, `polarization.negative_intensity` |

Contents may be negative (linear JVP use), as in P01. Checks `i1`–`i3` exercise all 27 codes, plus
positive controls for offset φ grids.

## 11. Polarization contract (axisymmetric `(I, Q)` only)

### 11.1 Coherency positivity

In a screen basis, `ρ = ½ [[I + Q, U − iV], [U + iV, I − Q]]`. Its eigenvalues are `(I ± |P|)/2`, with
`|P| = sqrt(Q² + U² + V²)`. Realizability means `ρ ⪰ 0`, which is equivalent to `I ≥ |P|`. The realizable
set is a convex (Lorentz) cone.

`Q`, `U` and `V` are signed, so positivity is a cone condition, not a componentwise one. `h1` checks
`coherency_eigenvalues` against `numpy.linalg.eigvalsh`.

### 11.2 Chandrasekhar kernel and its `(I, Q)` form

For axisymmetric fields, `U = V = 0`, with `Q = I_l − I_r` referred to the meridian plane. Chandrasekhar's
Rayleigh/Thomson phase matrix gives

$$J_l = \tfrac38\!\int\![(2(1-\mu^2)(1-\mu'^2) + \mu^2\mu'^2)I_l' + \mu^2 I_r']d\mu',\qquad
J_r = \tfrac38\!\int\![\mu'^2 I_l' + I_r']d\mu'.$$

In `(I, Q)`, with `X_l = ½ ∫ P_l X dμ`:

$$\dot I = \Gamma\big[I_0 - \tfrac12 P_2(\mu)\Pi - I\big],\qquad
\dot Q = \Gamma\big[\tfrac12(1-P_2(\mu))\Pi - Q\big],\qquad \Pi = Q_0 - I_2 - Q_2.$$

This matches the CMB form, given the `(−i)^l` moment convention. With `Q = 0`:

- `İ` is P01's rank-2 unpolarized kernel (`h1`, against `thomson_rhs`, to 2.6e-16);
- anisotropic unpolarized radiation sources `Q`, because `Π = −I_2`.

**Discrete form** (cell contents, `p_j = ⟨P_2⟩_j`):

- `I0 = ½ Σ I_j`, `I2 = ½ Σ p_j I_j`, and likewise `Q0`, `Q2` (sequential sums);
- `Π = (Q0 − I2) − Q2`;
- `İ_j = Γ((d_j I0 − ½(d_j p_j) Π) − I_j)`;
- `Q̇_j = Γ(½(d_j(1 − p_j)) Π − Q_j)`.

In exact arithmetic this is identical to the cell-averaged `(l, r)` kernel (`a6`, in `Fraction` arithmetic,
using `t2 = (2/3)(1 − p2)` and `m2 = (1 + 2p2)/3`). That kernel has entries `2 t2_j t2_j' + m2_j m2_j'`,
`m2_j`, `m2_j'` and `1`, all nonnegative.

Further checks:

- number `Σ İ = 0` and the isotropic unpolarized state is stationary (`h2`);
- in f64, the two forms agree to 2.6e-16 (`h1`).

### 11.3 Positivity theorem

With `U = V = 0`, `I ≥ |Q|` is equivalent to `(I_l, I_r) ≥ 0`. The gain maps nonnegative `(l, r)` vectors
to nonnegative ones. Forward Euler is `X⁺ = (1 − Γdt) X + Γdt J[X]`, a convex combination for `Γ dt ≤ 1`,
so it preserves realizability.

| Check | Setup | Result |
|---|---|---|
| `h3` | 1200 trials of random admissible states, including fully polarized cells `I = |Q|` and empty cells, at `Γdt ∈ {0.25, 0.5, 0.9, 1}` | minimum scaled `I − |Q|` = 0.0 |
| `h4` | `Γ dt = 1.5` on a fully polarized single-cell state | violates: `I − |Q| = −0.75·d` |

SSP Runge–Kutta methods (convex combinations of Euler steps) inherit the bound. Rosenbrock methods do not.

### 11.4 Componentwise clipping is wrong (`h5`)

Componentwise clipping fails in four ways:

1. `(I, Q) = (0.5, 0.9)` has nonnegative components but is not realizable. Clipping does nothing, and
   `λ_min < 0` remains.
2. `(−0.1, 0.05)`: clipping creates number, breaking the ledger, and the state is still not realizable.
3. `(1, −0.75)` is admissible, yet clipping changes the physical `(I_l, I_r)` from `(0.125, 0.875)` to
   `(0.5, 0.5)`.
4. On an actual over-long Euler step (`Γdt = 1.5`), clipping leaves a non-PSD cell and changes the total
   number by `+0.0489`.

The number-conserving alternative is `coherency_limit`:

- it scales the polarization vector to `|P| ≤ I` and keeps `I`;
- a `nextafter` loop makes the evaluated `|P'| ≤ I` exact in f64, so `λ_min ≥ 0` exactly (`h6`);
- it refuses `I < 0`, since no number-conserving local repair exists, and the step must be rejected.

A limiter is a modification of the physical state. It must be booked as such and is not part of the
right-hand side.

## 12. HOLD: obstacles not resolved in this node

### 12.1 Why a θ-only shortcut is invalid (summary)

1. Under triaxial shear the φ-averaged equations are not closed (§6.8). They need the `m = 2` moment, which
   couples to `m = 4, 6, …`. The error of the shortcut is `O(h_d² t²)` in the integrated contents (`c3`).
2. Full-sphere Thomson has rank 6 per group. A θ-only model keeps only the monopole and `b_0`, and drops the
   `m = ±1, ±2` couplings that non-axisymmetric data excite.
3. A tilt (12.2) singles out a direction that is generically not a principal shear axis, so no symmetry
   axis exists at all.
4. For polarization, the screen-basis rotation (12.5) vanishes only when `φ̇ = 0`.

### 12.2 Tilt and aberration

**Obstacle.** If the electron (baryon) frame moves with velocity `v` relative to the normal frame, Thomson
is isotropic only in the electron rest frame. Several components can be tilted while the total momentum
constraint of Bianchi I still holds. Transforming to the normal frame:

- Doppler-shifts energies, `E' = γE(1 − v·n)`;
- aberrates directions;
- rescales the rate.

**Consequences.**

- The collision operator couples energy groups, through `O(v) E∂_E f` and dipole terms. It is no longer
  "diagonal plus rank 6 per group".
- The `(q, μ, φ)` cell measure must be transformed consistently, or the scattering evaluated by a
  conservative inter-group remap.
- An energy ledger to the matter is needed.

Positivity and number or energy conservation of such an inter-group operator are not established here.

### 12.3 Recoil

Compton recoil, `E' = E/(1 + (E/m_e c²)(1 − cos Θ))`, and thermal electron motion (Kompaneets) both
transfer energy between photon groups and the electrons. With stimulated scattering they are nonlinear in
`f`, through `f(1 + f')`.

The per-group elastic operator here conserves number and photon energy exactly per group, and recoil breaks
that. A replacement needs:

- a conservative and positive inter-group discretization (Chang–Cooper/Larsen type);
- a matter energy ledger;
- the Jacobian structure of the stimulated terms.

### 12.4 Class-B (and class-A non-I) geometry

The derivation of §2–§3 uses vanishing structure constants.

- **Class A non-I (types II, VI₀, VII₀, VIII, IX):** `n_ab ≠ 0` adds curvature or rotation terms to the
  direction drift and a frame-rotation term `Ω`.
- **Class B (`a_b ≠ 0`, e.g. types III, IV, V, VIh, VIIh):** in a homogeneous orthonormal frame the
  spatial divergence of the photon current keeps the term `−2 a_b N^b`. Its number balance is
  `ṅ + 3Hn − 2a_b N^b = 0` (Ellis–MacCallum sign convention for `a_b`; outline, not checked). This is not a phase-space divergence: the total number
  changes, which requires its own ledger. The drift `ṅ^a` gains `a_b`-dependent terms.
- Spatial homogeneity of the tetrad components does not allow deleting these terms (P01 §2).

A class-B port needs the full spacetime-current divergence, a new Liouville identity and a geometric source
ledger. None of these is derived or checked here.

### 12.5 Full-sphere polarized transport

**Derivation sketch, unchecked.** Parallel transport of the polarization covector in Bianchi I, in the gauge
`f_0 = 0`, gives `d(f_i/a_i)/dλ = 0`. The orthonormal polarization vector is therefore constant modulo the
direction. Its transverse part obeys `Ḟ_⊥ = −(F_⊥·ṅ) n`, which is Levi-Civita transport along the drifting
direction on S².

**Consequences.**

- In the `(e_θ, e_φ)` basis the polarization angle rotates at rate `χ̇ = ±μ φ̇`, with the sign set by the
  `(Q, U)` convention.
- `Q ± iU` therefore mixes at rate `2μφ̇ = μ(H_1 − H_2) sin 2φ`. This vanishes in LRS (`φ̇ = 0`), which is
  why the `(I, Q)` reduction is consistent there, and only there.
- Full-sphere polarized Thomson needs spin-2 (E/B) harmonics and the 2×2 coherency cone constraint per cell,
  with a positivity-compatible discretization of the rotation term.

All of this is HOLD.

### 12.6 Not in this node

The following are not in this node:

- the Rust adapter (`lrs_photon_fv_support` extended to `nphi`);
- RODAS5P integration, stage solvers, zero-allocation measurements (VG-1) and timing.

For the follow-on solver node, two implications:

- The per-group angular transport block is no longer tridiagonal. It is a 2D upwind stencil on
  `(μ × periodic φ)` with offsets `±nphi` and `±1`, with wrap.
- Coarse periodic grids can produce directed cycles in the upwind graph. For example, `nphi = 2` with both
  faces at positive `sin 2φ` gives a cycle. An exact sweep must refuse such patterns, as P03's mixed-drift
  refusal does, and the low-rank capacitance is 6×6 per group.

## 13. Checks and results

`python3 FULL_SPHERE_FVM_REFERENCE.py CHECKS.json` reports `pass_scoped` with 38 tests, 0 failures and 0
errors. The run took 24 s wall (diagnostic only) on Python 3.12.3, numpy 2.4.2, scipy 1.17.0, mpmath 1.3.0
and glibc 2.39. The oracle and the read-only P01 oracle SHA-256 values are recorded in
`CHECKS.json → inputs`. The P01 oracle was imported by file path with bytecode writing disabled.

| Requested | Tests | Result (bound) |
|---|---|---|
| (a) exact algebra | `a1`–`a6` | all `Fraction` identities exact |
| (b) number conservation | `b1`, `b2`, `b3`, `b4`, `b5`, `b6` | sum residual 8.8e-17 (1e-14); band = M+1; linearity 4.9e-16; Euler ≥ 0; 3178 bitwise Jacobian entries |
| (c) LRS reduction | `c1`, `c2`, `c3` | 1.0e-15 (1e-14); axisymmetry 3.5e-16; θ-only ratios 2 and 4 |
| (d) Thomson | `d1`–`d6` | averages 1.0e-16; number 2.9e-16; isotropic 5.1e-16 (1e-14); kernel 2.0e-15; `min K` 0.852 |
| (e) rank 6 | `e1`, `e2`, `e3` | rank 6 on 4 grids with `M ≥ 12`; factors 1.5e-16; predicted degenerate ranks 5/3/2/5 |
| (f) Thomson LRS | `f1` | 2.2e-15 (1e-14) |
| (g) characteristics | `g1`–`g4` | rays 2.2e-13 (1e-9); Jacobian 3.5e-10; convergence 0.067 → 0.045 → 0.027 |
| (h) polarization | `h1`–`h6` | `I ≥ |Q|` kept in 1200 trials; violation at Γdt = 1.5; clipping failures shown |
| (i) refusals | `i1`–`i3` | 27 codes |

**Development disclosures.** Before the final run, five checks failed in development runs. None of the
fixes changed a tolerance or an oracle formula.

| Check | Cause | Fix |
|---|---|---|
| `b3` | The finite-difference fixture drove an absorption rate negative; the oracle refused it correctly | changed the fixture's `kbar` |
| `h5` | `(1 + (−0.8))/2` is not exactly 0.1 | used exactly representable values |
| `h6` | The bound `−1e-16·I` was below one ulp | made the limiter rounding-aware and tightened the assertion to exact `≥ 0` |
| `g3`, polynomial | 6-point Gauss was too coarse for the `sin 3θ` component | 10 points |
| `g3`, total | 5-point Gauss on unit cells | 8 points on 16 cells |

## 14. Modelling choices (for the Rust implementer)

1. **Variables and contents.** `U = a³κE³f` with `a³ = a_1a_2a_3`, and cell contents
   `N = ∫ U dq dμ dφ`. Unlike P01 there is no `2π` factor: φ is explicit.
2. **φ grid.** `φ0 ∈ [0, TAU)`, last edge `== φ0 + TAU` (f64), `nphi` periodic faces, the wrap face at
   `phi_edges[0]`, and wrap neighbours `(l − 1) mod nphi`.
3. **μ grid.** `μ` edges are exactly `±1` at the ends; the pole faces are never evaluated.
4. **Face velocities.** Each face uses the exact face average of the normal velocity (§6.2). The upwind
   direction is the sign of that average, even where the pointwise `μ̇` changes sign inside a φ cell.
5. **Tie rule.** `v >= 0.0` takes the lower-index (for φ, the previous periodic) cell.
6. **Flux form.** `flux = v · (N_up / h_up)`. This differs from P01's `v·(N/(dq·dm))·dm`, and it makes
   Jacobian columns bit-identical to unit-vector right-hand sides.
7. **Operation order.** Stage Q (columns `j`-major then `k`, faces `0..=nq`), then stage MU (`i`, `k`,
   faces `1..nm−1`), then stage PHI (`i`, `j`, faces `0..nphi−1`). Each face does `lower −= flux` and then
   `upper += flux`. Then Thomson per group (moments in cell order, then gains), then the absorber.
8. **Real-harmonic normalization.** Unnormalized `b_m` (`P_2`, `xz`, `yz`, `x² − y²`, `xy`) with weights
   `w = (1/2, 3/2, 3/2, 3/8, 3/2)`. These equal the orthonormal real `Y_2m` through
   `Y = sqrt(10 w_m/4π) b_m`. The gain uses `ω_c = (d_j/2)(Δφ_k/TAU)`, not `Ω_c/(4π)` computed from a
   rounded `4π`.
9. **Cell averages.** The closed forms are cancellation-free (§7.3), and squares and cubes are products.
10. **Boundaries and absorber.** The energy boundaries, the ledgers and the per-group absorber are P02's.
    The Thomson block is skipped when `Γ == 0.0`.
11. **Comoving coordinate.** Use the same operator with `σ_k` in place of `H_k`.
12. **Polarization.** Contract only, axisymmetric `(I, Q)` with `Q = I_l − I_r` in the meridian plane.
    Positivity holds for Euler steps with `Γ dt ≤ 1`. The number-conserving limiter refuses `I < 0`.
