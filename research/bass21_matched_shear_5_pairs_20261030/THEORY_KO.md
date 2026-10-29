# BASS21 source-owned matched-shear histories: exact ionization–geometry optical-depth decomposition

2026-10-30 KST. Native-source *stored-model data*, not newly integrated IVP or an observational/line-of-sight/CMB opacity estimate.

## Definitions and physical assumptions

Original `cosmosapjw-quantum/bass@cc602e209e7deeb511ab2a1f3b2268d55206c43c` prescribed bath and proper-time `AtomicNetwork`. Original source proper mean expansion H=d ln a/dt in s^-1, proper baryon density `n_b` in m^-3, original baryon-normalized 21 component Y, electron charge vector q, `x_e=q^T Y`, `n_e=n_b x_e`. Geometry x=ln(a/a_start), a_start=1/21, source z=21 exp(-x)-1, from 20 to 2. `c sigma_T` constant K=(2.99792458e10 cm/s)(6.6524587051e-25 cm²)/1e6 = 1.994356946945426e-20 m³/s. BASS internally defines **cumulative forward-clock** `dτ_T/dx=K n_e/H`; this is NOT the standard backward observer optical depth from z=0. This is a homogeneous comoving-electron Thomson monopole in BASS's test-fluid approximation; true directional Bianchi geodesic optical depth, electron peculiar velocity and polarization transfer are outside scope. Source uses prescribed T and UV, not self-consistent photon thermal radiation transfer.

## Exact constructed theorem: finite matched-pair split (DERIVED)

For A=(initial Σ_F=0), B=(initial Σ_F=0.6), at *identical source x* let bar(f)=(f_A+f_B)/2 and Δf=f_B-f_A. Put v=H^-1, w=n_b, y=x_e. The native instantaneous opacity g_s=K n_{e,s}v_s=K w_s y_s v_s. Apply the exact two-product polarization Δ(ab)=bar(a)Δb+bar(b)Δa successively:

Δg = K [bar(v) bar(w) Δy + bar(v) bar(y) Δw + bar(n_e) Δv].

The three terms are termed `ionization`, `density`, and `geometry` in this algebraic ordering. The identity has zero exact-real residual for any finite positive H_A/H_B and any densities, no perturbative shear expansion and no time evolution. It is an accounting identity, *not* a causal intervention that varies H independently of chemistry. All terms have units 1, same as dτ/dx; integrate from common x_0=0 to saved x_end=ln(7).

This decomposition depends on the chosen order (split `n_e/H` then `n_b*x_e`). In our actual matched five BASS pairs, `Δn_b=0` *at all 193 stored epochs*, so the density term identically vanishes and the remaining two-factor division into n_e and H is exact and unambiguous within that convention. Wolfram symbolic simplification produced `{0,0}` for both polarizations. An independent Fraction-from-binary64 test checks exact identities on 25 case/epoch samples.

## Dynamical cross-check without evolving any IVP

The original source `dY/dt=F(Y,T,UV,n_b)` implies `dx_e/dx=(q^T F)/H` in the BASS comoving normalized model. At matched x, its exact differential contrast similarly splits:

Δ(d x_e/dx)=bar(H^-1) Δ(q^T F)+bar(q^T F) Δ(H^-1).

The first term changes the local chemical source at fixed clock, the second changes conversion from physical seconds to x. Integrating these separately can produce strong cancellation (~0.00358 with opposite sign for the reference pair). **A 193-point Simpson quadrature of the two large contributions is not precise enough to resolve a ~1e-12 endpoint Δx_e**. Therefore no quantitatively accurate integrated ionization-dynamics attribution from this cancellation is claimed; the optical-depth split is integrated *directly from saved x_e,H*, which is substantially better conditioned.

## Original source-model data, five matched histories (NUMERICALLY_CHECKED)

Five post-hoc selected pairs sample center/corners from the original 25 matter/radiation matched grid; 10 source binaries 193 points each, SHA verified against their own BASS `RESULT.json`, 1,930 original source `AtomicNetwork.evaluate` calls, no IVP. Same initial P_m and P_r from each stored physical state (validated), and s0/s4 Σ_F targets 0/0.6; same x/z, identical prescribed T/UV, identical n_b. Original archive bytes preserved in source package.

| Pair (m,r) | Original saved Δτ_T | Integrated geometry | Integrated ionization | Geometry share | max |Δx_e| (history) |
|---|---:|---:|---:|---:|---:|
| `m00_r00` | -0.000125177868468 | -0.000115309944545 | -9.86793466925e-06 | 92.116870% | 0.000517908553 |
| `m02_r02` | -0.000124104746875 | -0.000114264789349 | -9.83993885379e-06 | 92.071262% | 0.00051641258 |
| `m04_r04` | -0.000122934831683 | -0.000113126680936 | -9.80814558451e-06 | 92.021670% | 0.000514735347 |
| `m00_r04` | -0.000123224581537 | -0.000113406212875 | -9.81836711821e-06 | 92.032136% | 0.000515275784 |
| `m04_r00` | -0.000124888006598 | -0.000115030205297 | -9.85783764903e-06 | 92.106660% | 0.000517368186 |

Across these **five selected pairs only**, geometry share ranges 92.021670%–92.116870%, ionization share complementary; saved Δτ_T ranges [-0.000125177868468,-0.000122934831683]. The reference pair intermediate |Δx_e| peaks 0.000517908553114, whereas final |Δx_e| is only 1.59794399934e-12: endpoint chemistry alone does not capture the ionization contribution to accumulated opacity. The middle third of the stored x interval (z≈9.978→4.739) contributes 83.74%–83.94% of total Δτ across these pairs. This is a *model-history conditional output*, not a universal monotonic shear theorem.

## Error classes and source ownership

Pointwise source Δgamma decomposes with residual ≤5.6216e-17. Simpson **total versus native stored Δτ end** ≤3.6348e-11; maximum over all saved x ≤5.2269e-10. These are observed residuals from sampling/interpolation/native old BDF, NOT a rigorous decomposition of the BDF solver's global error. The source's 21-state rates/heat yields remain BASS original; no REI17/Puchwein19 source substitution, no same-provider bath or inventory admission. Photon absorption W m^-3 ≠ deposited gas heating.

## Primary literature (context, not imported source)

- Pontzen & Challinor, *Linearization of homogeneous, nearly-isotropic cosmological models*, CQG **28** 185007 (2011), DOI 10.1088/0264-9381/28/18/185007, Bianchi vs perturbation distinction. No claim it derives this BASS numerical ratio.
- Pontzen & Challinor, *Bianchi Model CMB Polarization and its Implications for CMB Anomalies*, MNRAS (2007), DOI 10.1111/j.1365-2966.2007.12221.x, clarifies true Bianchi radiative transfer complexity, which this scalar tau cannot replace.
- Sailer, Chen & White, *Optical depth to reionization from perturbative 21cm clustering*, arXiv:2205.11504, optical depth as derived electron reionization-history quantity; not BASS source identity.
- Native atomic original model references Verner et al. 1996 photoionization, Verner & Ferland 1996 RR, Badnell DR/RR are pinned in PR118/PR269 source original, not re-fitted here.

## Scientific admission

A new auxiliary BASS21 matched-shear **physical-model data** unit 1/1 SCOPED_DATA_GENERATED, independent review HOLD. Original VigilODE DAG 7/23, REI central 28/88, REI working 33/88, successor 0/12 admitted, physical fast 4/7, F4 PARTIAL, L4 false, all **unchanged**. Independent native different-author R6/C9 NOT_RUN. Do not convert these scalar τ labels to cosmological observational likelihood or production.
