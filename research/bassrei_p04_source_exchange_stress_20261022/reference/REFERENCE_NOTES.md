# P04 Python reference: resolved ambiguities and test results

Files:

- `p04_reference.py`: the independent reference **P** of `../MODEL.md`.
- `test_p04_reference.py`: unittest suite. Each test prints its numbers as `[P04REF]` lines on stderr.

Run the suite with:

```bash
python3 -m unittest discover -s research/bassrei_p04_source_exchange_stress_20261022/reference -p 'test_*.py'
```

Environment: Python 3.12.3, numpy 2.4.2, scipy 1.17.0, glibc libm, x86-64 without AVX-512. On this host,
numpy `log`, `exp`, `power` and `hypot` are bitwise equal to libm.

Provenance:

- Nothing was read from or copied out of the Rust P04 work.
- The Rust P02 adapter `lrs_photon_fv_support/mod.rs` was read only to confirm the transport operation order.
  That order matches the oracle `BASS_REI_FVM_REFERENCE.py`: `select = -h - s*c_j`, `velocity = -vh - vs*c_j`,
  `flux = (velocity*(y/(dq*dm)))*dm` for q faces and `((-3*vs)*mu)*(1-mu*mu)` with the upwind density times
  `dq_i` for mu faces. The Python transport here is written from the oracle.

## 1. Ambiguities in MODEL.md and the choice made

Notation: `a*b*c` means `(a*b)*c`, and every sum is a plain left-to-right binary64 sum starting at `0.0`.
Python floats are used throughout: there is no FMA, and every transcendental is libm.

### Constants and the Verner fit

**A1. `K_D`.** It is computed as `((8*pi)*G/((3*C)*C))*EV_ERG` = `9.967662314789024e-40`.

**A2. Verner `(x-1)**2`.** This is the correctly rounded product `xm1*xm1` with `xm1 = x - 1.0`, which equals
numpy `**2` and Rust `powi(2)`. It is **not** CPython scalar `(x-1)**2`, which is libm `pow(x-1, 2.0)`. Over
200000 random `x`, the two differ by 1 ULP in 179 cases. On the GL nodes of the 4,4,4 and 16,16,16 grids, the
pow variant changes no `sbar`.

**A3. Verner `hypot(x, y1)`.** This is libm `hypot`, called through `numpy.hypot`. A test confirms it is
bitwise equal to libm `hypot` via ctypes on all 768 HeI GL nodes of the 16,16,16 grid. It is **not**
`math.hypot`: CPython has its own algorithm, which differs in 107 of 200000 cases. `math.hypot` changes `sbar` by
at most 1 ULP (16,16,16 grid). For HI and HeII, `y1 = 0`, so `hypot = |x|` exactly.

**A4. The two Verner powers.**

- `y**(p/2 - 5.5)` uses libm `pow`, with the exponent computed as `(p/2) - 5.5`.
- `(1 + sqrt(y/ya))**(-p)` uses libm `pow`.
- The full product is `sigma0*(xm1*xm1 + yw*yw)*y**e*(1+sqrt(y/ya))**(-p)*1e-18`, evaluated left to right.

For `E < chi`, `sigma` is 0, and the test `E < chi` uses the FIT `chi` (13.60, 24.59 or 54.42), not `BIND`.

### Cell-average cross section `sbar` (16-point Gauss–Legendre)

**A5. Nodes and weights.** They come from `numpy.polynomial.legendre.leggauss(16)`, in ascending order.

**A6. Mapping and summation.**

- Each node maps to `q_k = 0.5*(lo+hi) + (0.5*dq)*x_k`.
- The sum is `S = sum_k w_k*sigma(exp(q_k))`, with `k` ascending.
- Then `integral = (0.5*dq)*S` and `sbar = (1.0/dq)*integral`.

Measured alternatives:

- `sbar = 0.5*S` changes the result by at most 1 ULP.
- The mapping `lo + (x_k+1)*(0.5*dq)` changes it by at most 13 ULP.

A 1-ULP change in every `sbar` moves the RHS by at most `4.9e-16`, relative to the per-slot sum of absolute terms.

**A7. Cells below a threshold.** A cell with `q_hi <= ln(chi_s)` has `sbar = 0.0`, set by an explicit face
test. The GL sum would also be exactly 0, because every node of such a cell has `E < chi`.

### Grids and cell averages

**A8. Energy faces.**

- The segment bounds are `math.log(13.60)`, `math.log(24.59)`, `math.log(54.42)` and `math.log(100.0)`.
- Interior faces are `lo + ((hi - lo)*k)/n` for `k = 1..n-1`.
- Each segment's first face is exactly `lo`, every shared face is exactly `ln(chi)`, and the last face is
  exactly `math.log(100.0)`.
- For every `n <= 64`, the formula at `k = n` would give `hi` bitwise anyway.

**A9. Angle faces.** They are `-1.0 + (2.0*k)/nm`, with the endpoints forced to `-1.0` and `1.0`; the formula
gives them anyway. In floats, this grid is not exactly mirror-symmetric: for `nm = 7`, `mu_1 = -0.7142857142857143`
but `mu_6 = 0.7142857142857142`.

**A10. `c_j` and `p_j`.** Both use the oracle float formulas, with libm `pow(., 3.0)` for `**3`:

- `c_j = 3*((b**3 - a**3)/(3*d)) - 1`;
- `p_j = ((b**3 - b) - (a**3 - a))/(2*d)`.

These are not the Fraction-exact `p2_exact` of the P02 Python reference. For `nm = 4` the two are bitwise
equal. For `nm` = 6, 7, 10 or 12 they differ by at most about 1.7e-16 absolute, and a value near 0 can come out
as exactly `0.0` (nm = 7, j = 5).

**A11. `Ebar_i`.** It is `(exp(q_lo)*expm1(dq))/dq`.

### Geometry

**A12. Background formulas.**

- `t0 = 2/(3*H0)`.
- `H = (2/3)/(t0 + t)`.
- `a = (1 + t/t0)**(2/3)`.
- `ln a = (2/3)*log1p(t/t0)`.
- `a^-3 = (1 + t/t0)**-2`, using libm `pow`, not `1/(x*x)`. The two differ by at most 1 ULP on
  `t` in `[0, 1e16]`.
- `dH = (-1.5*H)*H`.
- `d(a^-3)/dt = ((-3)*H)*a^-3`.

`H(0)` is not bitwise `H0`, so `s/H` at `t = 0` is 0.1 only to roundoff.

### RHS term order

**A13. Domain check.** `T` is computed and checked at the start of `rhs`, `jacobian` and `partial_t`, before
any accumulation. A refusal is all-or-nothing, so where the check sits cannot be observed. Each refusal raises
`RateDomainError("CASE_A_T_DOMAIN: ...")` and increments `Model.domain_refusals` under a lock.

**A14. Temperature and electron density.**

- `N_e = (N_HII + N_HeII) + 2*N_HeIII`.
- `N_H = N_HI + N_HII`.
- `N_He = (N_HeI + N_HeII) + N_HeIII`.
- `T = U/((1.5*KB)*((N_H + N_He) + N_e))`.

**A15. `W_gamma` row.**

- One running accumulator collects `Ebar_i*out[cell(i,j)]` over `(i, j)` ascending, then `Ebar_0*out[L_lo]`,
  then `Ebar_{nq-1}*out[L_hi]`.
- The row is set as `out[IW] = -acc`, not accumulated.
- The products are not factored per group. Only `D_gamma` is factored, as the spec writes it.

**A16. `D_gamma`.** `D_gamma = sum_i Ebar_i*inner_i`, with `inner_i = sum_j p_j*N_ij`. It is computed from `y`
after the `W_gamma` row.

**A17. Thomson.**

- `M0` and `M2` are plain sums over `j` ascending. They are not `fsum` as in the oracle.
- `Gam = (C*SIGMA_T)*(N_e*a^-3)`.
- The update is `out += Gam*((dm_j/2)*(M0 + (0.5*p_j)*M2) - N_ij)`, groups in `i` ascending.
- It is applied even though `Gam > 0` is tiny.

**A18. Photoionization.**

- `k = (C*sbar)*(N_s*a^-3)` is computed once per `(s, i)`, and `R = k*N_ij`.
- The five updates follow the spec's order, with the heat term `(Ebar_i - BIND_s)*R`.
- Zero-`sbar` cells are not skipped, which matters only for signed zeros.

**A19. Accumulation order in shared slots.**

- `U`: photoheating in `(s, i, j)` order, then `-Lam`, then one add of `(-a1) - a2`.
- `N_HeII`: `+R` from the HeI channel, then `-R` from the HeII channel, then `-r2`, then `+r3`.
- `N_HII`: `+R` from the HI channel, then `-rH`.

**A20. Recombination and cooling.**

- `rH = (rrHII*ne_p)*N_HII`.
- `r2 = ((rrHeII + drHeII)*ne_p)*N_HeII`.
- `r3 = (rrHeIII*ne_p)*N_HeIII`.
- `Lam = (ne_p*(((N_HII*cHII + N_HeII*cHeII) + N_HeIII*cHeIII) + N_HeII*cdrHeII))/EV_ERG`.
- `E_esc` add: `((B0*rH + B1*r2) + B2*r3) + Lam`.
- `R_e` add: `(rH + r2) + r3`.

**A21. Matter rows.**

- `D_m` add: `(((-2)*H)*D_m - D_m/tau_m) - (4*s)*U`.
- `a1 = (2*H)*U` and `a2 = (2*s)*D_m`.
- The `s` row is set as `((-3)*H)*s + ((K_D*amp)*(D_gamma + D_m))*a^-3`.

### Case-A interpolation

**A22. Knots.** They are `x = ln T` and `y = ln v` per column. `CaseA` builds scipy's
`PchipInterpolator(np.log(T), np.log(values), axis=0, extrapolate=False)`, which is the source semantics.
`pchip_manual` uses `math.log`. A test confirms the two log functions agree bitwise on all 1161 table entries.

**A23. Evaluation.**

- `X = math.log(T)` and `v = math.exp(P)`.
- `dv/dT = (v*P')/T`.
- The domain check is on `T`, inclusive, with `T_min = 1.00000000000000091e4` and `T_max = 2.40000000000000291e4`.
- The interval search is `bisect_right`. `X == x_{m-1}` uses `k = m-2`, so the last node reproduces `ln v` only to
  roundoff; the other 128 nodes reproduce it exactly.

**A24. `P'`.** It is the spec formula `c2 + (2*c1)*s + (3*c0)*(s*s)`, which is bitwise identical to scipy's
`derivative()` PPoly. scipy's `__call__(X, nu=1)` uses a different association, `(c1*s)*2` and `(c0*(s*s))*3`.
In practice it was also bitwise identical on all 161040 values tested.

**A25. Backends.** `CaseA` exposes both: `'scipy'`, the default used by the model, and `'manual'`. The
`sign()` function follows `numpy.sign`. The `m == 2` special case of scipy is not implemented, since MODEL.md
fixes `m = 129`; the manual PCHIP refuses `m < 3`.

### Initial data and integration

**A26. Initial state.**

- `nHe = (nH0*0.2454)/(4*(1 - 0.2454))`.
- `N_HeIII = (nHe - N_HeI) - N_HeII`.
- `U = ((1.5*KB)*T0)*((nH0 + nHe) + N_e)`, where `N_e` comes from the species.
- The photons are `N_ij = (total*w_ij)/W`, with `w_ij = ((Ebar_i**(-alpha)*dq_i)*(dm_j/2))*(1 + a2*p_j)`, `W` the
  running sum in `(i, j)` order, and `total = photons_per_H*nH0`.
- `T(y0)` equals `T0` only to roundoff.

**A27. Ledgers.** They use the spec's left-to-right order:

- `B = (B0*N_HII + B1*(N_HeII + N_HeIII)) + B2*N_HeIII`.
- photon ledger: `((n_gamma + L_lo) + L_hi) + sum_i A_i`.

**A28. Jacobian.**

- The dense numpy Jacobian builds its transport block from coefficients that equal the RHS flux of a unit
  content, so each column equals `transport(e_k)` bitwise (tested).
- The `W_gamma` row is `-(w_E @ J_transport)`, a numpy matmul, so its summation order is BLAS-defined.
- The `s` column is `transport(H, s, 0, 1, N)`.
- The Jacobian is therefore not bit-comparable with Rust; it is checked against finite differences.

**A29. `f_t` in the `s` row (spec gap, flagged).** MODEL.md section 8 lists `dH/dt` for the `s` row, but its
governing sentence says "every term containing H(t) or a^-3(t)". The `s` row's `K_D` term contains `a^-3`, so I
include `K_D*amp*(D_gamma + D_m)*d(a^-3)/dt`.

- Finite differences confirm this term: with `amp = 1e30` it dominates `f_t[s]`. With the term included, the error
  is `2.3e-12` relative to FD. With the bullet list only, the error is 100%.
- In the test scenario (`amp = 1`) the term is about `1.7e-11` of `f_t[s]`, so an implementation that follows only
  the bullet list would differ there at about that level.

**A30. Radau refusal emulation.**

- A refusal in the RHS returns a NaN vector. scipy Radau treats non-finite `F` as a Newton failure and halves
  the step, which emulates the driver's NonFinite-as-rejection.
- A refusal in the Jacobian aborts the solve.
- An unsuccessful run after refusals returns
  `failure = {kind: "NonFinite", message: "CASE_A_T_DOMAIN: ...", last_refusal, t_reached}`.

**A31. `atol` scale.** `atol = 1e-14*scale`, where `scale` is:

- `nH0` for every number-like slot (photon cells, `L_lo`, `L_hi`, `A_i`, species, `R_e`);
- `|U0|` for `W_gamma`, `U`, `D_m`, `W_m` and `E_esc`;
- `|s0|` for `s`, or `H0` if `s0 = 0`.

**Comparing with R.** A component's RHS can be the small remainder of cancelling terms, for example the
`N_HeII` row. Compare R and P per slot against `Model.rhs_with_abs(t, y)[1]`, the sum of the absolute values of
every term accumulated into the slot, not against `|f_k|`.

## 2. Test results

All 24 tests pass, in about 2.2 s.

**(a) PCHIP.**

- Manual and scipy agree bitwise on all `4*128*8 = 4096` Hermite coefficients.
- At the 129 nodes, 1032 of 1032 values are bitwise identical, with max rel 0.
- On the dense sweep (20001 log-uniform `T`, 8 columns), 160008 of 160008 `P` values are bitwise identical, with
  max rel 0.
- Manual `P'` matches scipy `derivative()` bitwise on 160008 of 160008 values, and `__call__(nu=1)` on 160008 of
  160008.
- `exp(P)` coefficients and `dv/dT` via `CaseA`, scipy against manual: 160008 of 160008 bitwise, with max rel 0.

**(b) Domain refusal.** These all raise `CASE_A_T_DOMAIN:` with both backends: `T_min*(1-1e-15)`,
`T_max*(1+1e-15)`, `nextafter` outside each end, NaN, `±inf` and 0. Inclusive endpoints are accepted.

- `rhs`, `jacobian` and `partial_t` each refuse on states whose computed `T` lies just outside either end; the
  counter goes up by 3 per state, and a NaN `U` adds one more.
- An out-of-domain Radau run (`T0 = 2.3e4`, 0.5 photons per H) ends with `status -1` after 308 refusals. Its
  failure record is `NonFinite` / `CASE_A_T_DOMAIN: T=24000.000000000033 K ...`, `t_reached = 1.757e11 s`.

**(c) Invariants.** The test uses 94 random admissible states from the test, stress (`nH0 = 3`, `H0 = 2e-14`,
`amp = 1e30`) and Thomson-dominated families, on grids 4,4,4,4 / 3,5,7,6 / 16,16,16,8. It measures
`|fsum(w*f)| / fsum(|w|*abs_terms)`; the bound is 1e-14:

| Ledger | Max ratio |
|---|---|
| number | 4.81e-16 |
| H | 0 |
| He | 4.24e-16 |
| photon | 5.21e-17 |
| energy | 4.16e-16 |

**(d) Jacobian against central Richardson FD**, on 12 states from the three families.

- **Per-column relative error** (bound 1e-6):

  | Family | Max |
  |---|---|
  | test | 4.16e-8 |
  | stress | 1.95e-8 |
  | thomson | 2.42e-10 |

- **Per-entry relative error**, for entries at least 1e-3 of the column max:

  | Family | Max |
  |---|---|
  | test | 4.0e-7 |
  | stress | 2.4e-7 |
  | thomson | 1.0e-9 |

- Zero columns are exactly zero in FD.
- The photon and `W_gamma` rows are exactly linear in every species. A 25% step check, with a roundoff floor,
  pins them; the Thomson `dGam/dN_HII` and `dGam/dN_HeIII` entries agree to 2.5e-10.
- Each transport column equals `transport(e_k)` bitwise.
- **FD steps.** A step of `|y_k|` (or the slot scale) is used for slots where `f` is exactly linear. Species and
  `U` use `1e-4` relative; species are floored at `1e-6*N_tot`. The step is quartered until the whole stencil lies
  in one PCHIP interval, because the spline is only C1 at its knots. `s` uses `1e-3` relative.

**(e) `f_t` against FD in `t`**, on 15 states. The max-norm relative error is 1.07e-11, and the per-component
relative error is at most 1.17e-9 for components at least 1e-6 of the max. The check also passes at `t = 0`.

**(f) Photoionization energy.** Per channel, photon energy loss equals `BIND*events + heat` to a max rel of
2.15e-16, against a bound of 1e-15, over 20 states on the 4,4,4 and 16,16,16 grids. Each `out[A_i]` is bitwise the
`(s, j)`-ordered sum of the same `R` floats, which confirms the RHS uses the identical float `R`.

**(g) Radau.** The run uses `nH0 = 2.4e-5`, `H0 = 3e-17`, `tau_m = 1e14`, `amp = 1`, `xHI0 = 0.6`,
`fHeI = 0.2`, `fHeII = 0.5`, `T0 = 12000`, `Dm0 = 0`, `s0/H0 = 0.1`, 0.05 photons per H, `alpha = 1.5`,
`a2 = 0.5`, on the 4,4,4,4 grid (n = 74), over `t` in `[0, 1e12]` with 11 outputs, `rtol 1e-10` and
`atol 1e-14*scale`.

- It succeeds with no refusals: `nfev 1359`, `njev 2`, `nlu 34`.
- Invariant drift, relative to the initial value (bound 1e-10):

  | Ledger | Max drift |
  |---|---|
  | number | 2.6e-16 |
  | H | 0 |
  | He | 2.2e-16 |
  | photon | 3.5e-16 |
  | energy | 1.9e-16 |

- Diagnostics:

  | Quantity | t = 0 | t = 1e12 |
  |---|---|---|
  | `n_gamma` | 1.2e-6 | 5.626837488e-7 |
  | `E_gamma` | 3.264575081e-5 | 2.009506195e-5 |
  | `D_gamma` | 2.295404354e-6 | 1.412850651e-6 |
  | `T` | 12000 | 12526.8556 |
  | `x_HII` | 0.4 | 0.4257498626 |
  | `x_HeII` | 0.5 | 0.5074976967 |
  | `x_HeIII` | 0.3 | 0.3011809212 |
  | `s/H` | 0.1 | 0.0999955002 |

**Other checks.**

- The vectorized and scalar Verner functions agree bitwise on 60015 values.
- `sbar` agrees bitwise between its scalar and vectorized paths.
- The 16-point GL `sbar` agrees with adaptive quadrature to 3.4e-16 on the 4,4,4 grid.
- Grid faces, layout and exact cell-average identities hold: `sum dm/2 = 1`, `sum dm*p = 0` and `c = 2p`.
- The geometry closed forms agree with finite differences.

## 3. Physics and numerics observations (MODEL.md not changed)

1. **Spec gap.** The `f_t` `s` row needs the `K_D`·`d(a^-3)/dt` term (A29).
2. **Weak terms in the test scenario.** Thomson and the shear back-reaction are negligible there:
   - `Gam` is `2.3e-19 s^-1` at `t = 0`, so `Gam*t` is about `2e-7` over 1e12 s.
   - `K_D*(D_gamma + D_m)*a^-3` is `8.5e-12` of `3*H*s` at `t = 0`.

   Test (g) therefore does not exercise them. The stress and Thomson-dominated families (`amp = 1e30`,
   `nH0 = 3`) were added so that FD checks do.
3. **Narrow table domain.** It is `T` in `[1e4, 2.4e4]` K. Adiabatic cooling over about `t0 = 2.2e16 s`, or strong
   photoheating, leaves it. The typed refusal is the intended outcome, as the out-of-domain test shows.
4. **Upwind branch for `s`.** The energy upwind selection `-H - s*c_j >= 0` flips on cell `j` when
   `s*c_j < -H`, since `c_j` lies in `(-1, 2)`. That happens first on near-axial cells, for `s < -H/max(c_j)`
   (about `-H/2`), and for `s > H` on equatorial cells. Across a flip the RHS is only piecewise linear in `s`.
   The analytic Jacobian is the derivative on the current branch, as MODEL.md states. The FD tests keep
   `|s| <= 0.3*H`.
5. **Physics consistency.** The physics is internally consistent:
   - comoving adiabatic `-2HU`;
   - shear work `2sD` on matter and photons;
   - viscous equilibrium `D_eq = -4*tau*s*U`, from `p = 2U/3`;
   - `ds/dt = -3Hs + (8*pi*G/3c^2)*D_proper`;
   - Thomson kernel `1 + P2(mu)*P2(mu')/2`;
   - the ledgers close exactly with the stated weights.
