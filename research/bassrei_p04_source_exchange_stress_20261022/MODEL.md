# P04 model specification: LRS photon transport with source-owned H/He photoionization, Case-A recombination, signed stresses and dynamic shear

This spec is the single source of truth for two independent implementations:

- **R**, the Rust client adapter, `crates/rodas5p-integrators/examples/lrs_photon_fv_support/chem.rs`, a
  child module of the P02 adapter, so it can call P02's private transport.
- **P**, the Python reference, `research/bassrei_p04_source_exchange_stress_20261022/reference/p04_reference.py`.

Neither implementation may copy the other's code. Each reproduces every formula below with the stated
operation order wherever an order is given.

## 0. Units and constants (verbatim from the source owner unless marked "ours")

Units are as follows:

- Time `t` is proper seconds.
- Energies are eV.
- Densities are **comoving** number densities, `N = a^3 n` (cm^-3, with `a(0) = 1`).
- Thermal and stress energies are comoving eV cm^-3.

Physical constants:

| Symbol | Value | Owner |
|---|---|---|
| `C` | `29979245800.0` cm/s | source C9 `C_CGS` |
| `KB` | `8.617333262145e-5` eV/K | source C10 `KB_EV_K` |
| `EV_ERG` | `1.602176634e-12` | source C11 `rate_table.EV_ERG` |
| `BIND` | `[13.598434599702, 24.587389011, 54.41776]` eV for HI, HeI and HeII | source C9 `BIND` |
| `SIGMA_T` | `6.6524587321e-25` cm² | ours, CODATA |
| `G` | `6.67430e-8` cgs | ours, CODATA |

The shear-coupling constant is ours: `K_D = 8*pi*G/(3*C*C) * EV_ERG` in cm^3 eV^-1 s^-2. It is
multiplied by the scenario factor `amp` (default 1).

### Verner fit

The fit parameters come verbatim from source C9 `experiment.py`, as
`FIT[s] = (chi, E0, sigma0, ya, p, yw, y0, y1)` for s = HI, HeI, HeII:

```
(13.60, .4298, 5.475e4, 32.88, 2.963, 0., 0., 0.)
(24.59, 13.61, 949.2, 1.469, 3.188, 2.039, .4434, 2.136)
(54.42, 1.720, 1.369e4, 32.88, 2.963, 0., 0., 0.)
```

For `E < chi`, `sigma_s(E) = 0`. Otherwise, in exactly this order:

```
x = E/E0 - y0
y = hypot(x, y1)
sigma = sigma0*((x-1)**2 + yw*yw) * y**(p/2 - 5.5) * (1 + sqrt(y/ya))**(-p) * 1e-18   (cm^2)
```

### Case-A table

The table is `owner_grackle_rates_129.csv`, sha256
`33396b071c840b40ad79d68ee626de59f12bb2cf0b1df2159627efd7fde42aec`. A byte-identical copy is in the
node at `source/owner_grackle_rates_129.csv`.

- **Header:** `T_K,rr_HII_cm3_s,rr_HeII_cm3_s,rr_HeIII_cm3_s,dr_HeII_cm3_s,rr_cool_HII_erg_cm3_s,rr_cool_HeII_erg_cm3_s,rr_cool_HeIII_erg_cm3_s,dr_cool_HeII_erg_cm3_s`.
- **Rows:** 129 data rows of float literals.
- **Interpolant:** the source's `PchipInterpolator(log T, log values, axis=0, extrapolate=False)`. A
  coefficient is `exp(interp(log T))`.
- **Domain:** `T_min = T[0] <= T <= T_max = T[128]`, inclusive. Outside it, the typed refusal
  `CASE_A_T_DOMAIN` of §6 applies.

### 0.1 PCHIP, reproducing scipy 1.17 per column

The nodes are `x_k = ln T_k` and the values `y_k = ln v_k`, for `k = 0..m-1` with `m = 129`.

Define the spacings and secant slopes `h_k = x_{k+1} - x_k` and `m_k = (y_{k+1} - y_k) / h_k`.

**Interior derivatives**, for `1 <= k <= m-2`:

- If `sign(m_k) != sign(m_{k-1})`, or `m_k == 0`, or `m_{k-1} == 0`, then `d_k = 0`.
- Otherwise:

  ```
  w1 = 2*h_k + h_{k-1}
  w2 = h_k + 2*h_{k-1}
  whmean = (w1/m_{k-1} + w2/m_k) / (w1 + w2)
  d_k = 1.0/whmean
  ```

**End derivatives** use `edge(h0, h1, m0, m1)`:

```
d = ((2*h0 + h1)*m0 - h0*m1) / (h0 + h1)
if sign(d) != sign(m0):                              d = 0
elif sign(m0) != sign(m1) and abs(d) > 3*abs(m0):    d = 3*m0
```

Apply it as `d_0 = edge(h_0, h_1, m_0, m_1)` and `d_{m-1} = edge(h_{m-2}, h_{m-3}, m_{m-2}, m_{m-3})`.

**Hermite coefficients** for interval `k`:

```
slope = (y_{k+1}-y_k)/h_k
t     = (d_k + d_{k+1} - 2*slope)/h_k
c0    = t/h_k
c1    = (slope - d_k)/h_k - t
c2    = d_k
c3    = y_k
```

**Evaluation** at `X`:

- Choose `k` with `x_k <= X < x_{k+1}`. `X == x_{m-1}` uses `k = m-2`.
- With `s = X - x_k`, `P = c3 + c2*s + c1*(s*s) + c0*((s*s)*s)`. This is the increasing-power order,
  verified against scipy 1.17 bit for bit.
- The derivative is `P' = c2 + 2*c1*s + 3*c0*(s*s)`.
- The coefficient is `v = exp(P)`, with `dv/dT = v*P'/T`.

## 1. Geometry (prescribed H, dynamic shear)

The background is matter dominated, with `H(t) = (2/3)/(t0 + t)` and `t0 = 2/(3*H0)`. The derived
quantities are:

- `a(t) = (1 + t/t0)^(2/3)`;
- `ln a = (2/3)*ln1p(t/t0)`;
- `a^-3 = (1 + t/t0)^-2`;
- `dH/dt = -1.5*H*H`;
- `d(a^-3)/dt = -3*H*a^-3`.

The shear `s` is a **state** component, with `sigma = diag(-s, -s, 2s)`:

```
ds/dt = -3*H*s + K_D*amp*(D_gamma + D_m)*a^-3
```

`H` is prescribed. This is a test-field background with no Friedmann back-reaction, and the
limitation is disclosed in the preregistration.

## 2. Photon grid and transport (the P02 operator)

**Energy coordinate.** The physical coordinate is `q = ln(E / 1 eV)`.

**Energy edges.** There are three uniform segments with faces aligned to the Verner thresholds:

- `[ln 13.60, ln 24.59]` with `n1` cells;
- `[ln 24.59, ln 54.42]` with `n2` cells;
- `[ln 54.42, ln 100]` with `n3` cells.

Within a segment, edge `k` is `lo + (hi - lo)*k/n`. Each shared face appears once, with exactly the
value `ln(chi)`. `nq = n1 + n2 + n3`.

**Angle edges.** `nm` uniform cells with `mu_k = -1 + 2*k/nm`, endpoints exactly -1 and 1.

**State layout:**

```
[L_lo, (N_i0 .. N_i(nm-1), A_i) for i < nq, L_hi,
 W_gamma, N_HI, N_HII, N_HeI, N_HeII, N_HeIII, U, D_m, W_m, E_esc, R_e, s]
```

The photon part has dimension `n_ph = 2 + nq*(nm + 1)`, identical to the P02 layout, so P02 `Layout`
is reused. The total dimension is `n = n_ph + 12`. The matter indices are `IW = n_ph`, then
`IHI, IHII, IHeI, IHeII, IHeIII, IU, IDm, IWm, IEesc, IRe, IS` = `n_ph+1 .. n_ph+11`.

**Transport.** This is exactly P02 `Model::transport(h, s, vh, vs, y, out)` with `h = H(t)` and
`s = y[IS]`, Physical coordinate and Outflow boundaries. It accumulates into `out[0..n_ph]`. The
upwind direction comes from `(h, s)` and the flux velocity from `(vh, vs)`; for the RHS,
`(vh, vs) = (h, s)`.

**Energy weights.**

- Cell-average energy: `Ebar_i = exp(q_lo_i)*expm1(dq_i)/dq_i`.
- Photon energy weights: `Ebar_i` for cell `(i, j)`, `Ebar_0` for `L_lo`, `Ebar_{nq-1}` for `L_hi`,
  and 0 for `A_i`.

**Discrete photon work.** Immediately after transport, and before any other term, set:

```
out[IW] = -( sum_i sum_j Ebar_i*out[cell(i,j)] + Ebar_0*out[L_lo] + Ebar_{nq-1}*out[L_hi] )
```

Sum with `i` ascending, then `j` ascending, then `L_lo`, then `L_hi`. This is the energy the discrete
transport removes, the discrete counterpart of `H*E_gamma + 2*s*D_gamma`.

**Moments.** Sum each with `i` ascending, then `j` ascending:

- `n_gamma = sum N_ij`;
- `E_gamma = sum Ebar_i*N_ij`;
- `D_gamma = sum_i Ebar_i * (sum_j p_j*N_ij)`, with `p_j` the P02 exact cell average of P2.

## 3. Thomson (elastic, number conserving per group)

`N_e = N_HII + N_HeII + 2*N_HeIII` and `Gam = C*SIGMA_T*(N_e*a^-3)`. Per group, use the P02 formula:

```
out[cell(i,j)] += Gam*(dm_j/2*(M0 + 0.5*p_j*M2) - N_ij)
```

## 4. Photoionization (shared event rate, source C10 semantics)

The species and their products are:

| Reactant `s` | Product `s+` | Threshold |
|---|---|---|
| HI (`IHI`) | HII | `BIND[0]` |
| HeI (`IHeI`) | HeII | `BIND[1]` |
| HeII (`IHeII`) | HeIII | `BIND[2]` |

**Cell-average cross section.** `sbar[s][i] = (1/dq_i) * integral over cell i of sigma_s(exp(q)) dq`.
Use a 16-point Gauss–Legendre rule in q, mapped to the cell. Faces align with every `chi`, so no cell
straddles a threshold. Cells below `chi_s` give `sbar = 0`.

**Event rate.** Per comoving volume per second:

```
k = C*sbar[s][i]*(N_s*a^-3)
R = k*N_ij
```

**Updates.** Every update uses the identical float `R`. The loops run `s` outer, then `i`, then `j`
ascending:

```
out[cell(i,j)] -= R
out[A_i]       += R
out[N_s]       -= R
out[N_s+]      += R
out[IU]        += (Ebar_i - BIND_s)*R
```

Binding energy is implicit in the species and is not a state:
`B = BIND[0]*N_HII + BIND[1]*(N_HeII + N_HeIII) + BIND[2]*N_HeIII`.

## 5. Case-A recombination and cooling (source C11 semantics as rates)

**Temperature.** The comoving factors cancel:

```
T = U/(1.5*KB*(N_H + N_He + N_e)),   N_H = N_HI + N_HII,   N_He = N_HeI + N_HeII + N_HeIII
```

**Rate coefficients.** `CaseA(T)` gives `(rrHII, rrHeII, rrHeIII, drHeII, cHII, cHeII, cHeIII, cdrHeII)`.

**Recombination events.** With `ne_p = N_e*a^-3`:

| Event | Rate |
|---|---|
| HII → HI | `rH = rrHII*ne_p*N_HII` |
| HeII → HeI | `r2 = (rrHeII + drHeII)*ne_p*N_HeII` |
| HeIII → HeII | `r3 = rrHeIII*ne_p*N_HeIII` |

**Species updates:**

```
out[IHII]  -= rH;   out[IHI]   += rH
out[IHeII] -= r2;   out[IHeI]  += r2
out[IHeIII]-= r3;   out[IHeII] += r3
```

**Cooling.** `Lam = ne_p*(N_HII*cHII + N_HeII*cHeII + N_HeIII*cHeIII + N_HeII*cdrHeII)/EV_ERG`, in
eV cm^-3 s^-1. Then `out[IU] -= Lam`.

**Escape ledger.** This follows source `rec_step`: capture binding and thermal cooling leave the
local inventory, and no recombination photons are tracked.

```
out[IEesc] += BIND[0]*rH + BIND[1]*r2 + BIND[2]*r3 + Lam
```

**Electron ledger.** `out[IRe] += rH + r2 + r3`.

## 6. Matter work, matter stress and typed refusal

**Matter stress.** It relaxes to the viscous equilibrium `D_eq = -4*tau_m*s*U`:

```
out[IDm] += -2*H*D_m - D_m/tau_m - 4*s*U
```

**Gas energy.** Let `a1 = 2*H*U` and `a2 = 2*s*D_m`. These are adiabatic and shear work, and the same
floats are used in both rows:

```
out[IU]  += -a1 - a2
out[IWm] +=  a1 + a2
```

**Shear.**

```
out[IS] = -3*H*s + K_D*amp*(D_gamma + D_m)*a^-3
```

**Typed refusal.** The RHS, Jacobian and `f_t` each refuse in these cases:

- `T` is non-finite;
- `T < T_min`;
- `T > T_max`.

The refusal returns error kind `NonFinite` with a message starting `CASE_A_T_DOMAIN:` and increments
an atomic counter `domain_refusals`. The driver retries `NonFinite` as a rejection. A solution that
truly leaves the domain therefore ends in a `DriverPartial` with `last_failure == NonFinite`.

## 7. Invariants (exact for the semi-discrete system)

Each of these is linear with constant weights, so RODAS5P with the exact Jacobian preserves it to
roundoff.

| Invariant | Quantity |
|---|---|
| number ledger (source `conserved_photon_electrons`) | `n_gamma + L_lo + L_hi + N_e + R_e` |
| hydrogen | `N_HI + N_HII` |
| helium | `N_HeI + N_HeII + N_HeIII` |
| photon ledger | `n_gamma + L_lo + L_hi + sum_i A_i` |
| energy ledger | `E_gamma + Ebar_0*L_lo + Ebar_{nq-1}*L_hi + W_gamma + U + B + E_esc + W_m` |

## 8. Jacobian and f_t

### Jacobian

The Jacobian is analytic and dense, filled in place without allocation. The matter slots come last,
an arrow-head layout, so the zero-skipping LU fills only the band and the border. It is assembled
from these blocks:

- **Transport:** P02 `jacobian_entries` at `(h = H, s)` with `gamma = 0` and no absorber.
- **`W_gamma` row:** minus the energy-weighted sum of the transport rows.
- **`s` column:** `d out/d s` = `transport(h, s, 0, 1, N)`, with the upwind direction frozen at
  `(h, s)`, plus its `W_gamma` work sum.
- **Thomson:** derivatives with respect to `N`, and through `Gam(N_e)` with respect to `N_HII`,
  `N_HeII` and `N_HeIII`.
- **Photoionization:** derivatives with respect to `N_ij` and `N_s`.
- **Recombination and cooling:**
  - with respect to `N_HII`, `N_HeII`, `N_HeIII` and `U`;
  - through `T` with respect to every species, since `T` depends on `U`, `N_H`, `N_He` and `N_e`;
  - using `dv/dT = v*P'(ln T)/T`.
- **Matter rows:** the matter equations themselves.

### f_t

`f_t` is the time derivative at fixed `y` of every term containing `H(t)` or `a^-3(t)`:

- transport with `(vh, vs) = (dH/dt, 0)`, upwind direction from `(H, s)`, plus its `W_gamma` work sum;
- `d(a^-3)/dt = -3*H*a^-3` in Thomson, photoionization, recombination, cooling, `E_esc` and `R_e`;
- `dH/dt` in the `D_m`, `U`, `W_m` and `s` rows.

## 9. Initial data (scenario parameters)

**Densities.** Proper equals comoving at `t = 0`. `nH0` is a scenario value.

```
nHe     = nH0*0.2454/(4*(1 - 0.2454))
N_HI    = xHI0*nH0
N_HII   = (1 - xHI0)*nH0
N_HeI   = fHeI*nHe
N_HeII  = fHeII*nHe
N_HeIII = nHe - N_HeI - N_HeII
U       = 1.5*KB*T0*(nH0 + nHe + N_e)
D_m     = Dm0
s       = s0_over_H0*H0
```

Every ledger starts at zero: `W_gamma = W_m = E_esc = R_e = L_lo = L_hi = A_i = 0`.

**Photons.** The total is `photons_per_H*nH0`, distributed as
`N_ij ∝ Ebar_i^(-alpha)*dq_i*(dm_j/2)*(1 + a2*p_j)` and normalized to the total.

## 10. Diagnostics (both implementations)

At a given state and time, both implementations report:

- photon moments: `n_gamma`, `E_gamma`, `D_gamma`;
- gas state: `T`, `x_HII = N_HII/N_H`, `x_HeII = N_HeII/N_He`, `x_HeIII = N_HeIII/N_He`;
- shear: `s/H`;
- ledgers: `number_ledger`, `energy_ledger`, `H_ledger`, `He_ledger`, `photon_ledger`.

## Corrections before preregistration (found while implementing; both implementations follow them)

1. **§8, the `f_t` of the shear row.** The governing rule applies: `f_t` is the time derivative at fixed
   `y` of every term that contains `H(t)` or `a^-3(t)`. So
   `f_t[IS] = -3*(dH/dt)*s + K_D*amp*(D_gamma + D_m)*d(a^-3)/dt`, with `d(a^-3)/dt = -3*H*a^-3`. The list
   in §8 named only the `dH/dt` term.
   - The independent Python reference found the gap (its ambiguity A29), and finite differences confirm
     it.
   - Both implementations carry the term. The Rust test checks it bit for bit, and against finite
     differences at `amp = 1e30`, with a negative control.
2. **§6, how a run that leaves the table ends.** The driver approaches the boundary by alternating
   accepted steps and refused attempts, and every rejection is a `CASE_A_T_DOMAIN` refusal
   (`domain_refusals > 0`). The run ends in a `DriverPartial` in one of two ways:
   - `MinimumStep`, with `last_failure = NonFinite`;
   - `AttemptBudget`, where `last_failure` may be `None` if the last attempt was accepted.
3. **Grid and LU.** The dense fast driver uses its allocation-free in-place LU only when `n <= 64` or
   when at most a tenth of the first Jacobian is nonzero. The integration grid is therefore
   `(n1, n2, n3, nm) = (16, 16, 16, 8)`, giving `n = 446`. The runner refuses (`LU_NOT_IN_PLACE`) a run
   that would use the allocating LU.
4. **Operation-order choices.** These are listed in `reference/REFERENCE_NOTES.md` and in the Rust
   adapter's notes:
   - `(x-1)**2` is computed as `xm1*xm1`;
   - `a^-3` is `(1 + t/t0).powf(-2)`;
   - the libm functions are used.

   Parity gates use term-scaled tolerances, so they do not depend on these choices bit for bit.
