# BASS21: 원본 125 저장 endpoint에서의 chemistry–opacity와 전단 민감도

연구 단위 `BASS21_NATIVE125_SHEAR_RATE_ATLAS_20261030`. **원본 모델 conditional physics, 2026-10-30 KST.** BASS `cc602e209e7deeb511ab2a1f3b2268d55206c43c`의 바이트 고정 `endpoints.csv`에서 125개 5×5×5 endpoint를 직접 사용하고, 기존 PR122의 5개 full-history 쌍만 그대로 상속한다. 125개는 모두 z=2 원본 BASS solver 저장값이다. 새 ODE IVP, parameter fit, likelihood, CMB transfer 없음. Convention (-,+,+,+), Einstein–Boltzmann angular transfer 없음. BASS original predicted temperature/UV가 아니라 prescribed bath, test-fluid baryon closure이다.

## 1. source chemistry and proper-time equation

원본 21-state chemical abundance는 `Y_i=n_i/n_b` (baryon당 ion 숫자), mass vector `A`, charge vector `q`, 단위 `n_b,n_e [m^-3]`이다. 원본 source의 proper-time RHS와 analytic Jacobian은

`F(Y,t)=[G0(T,UV)+D+n_b(q^T Y) Gne(T)]Y`, `n_e=n_b(q^T Y)`,

`J=G0+D+n_e Gne+n_b (Gne Y) q^T`.

`[F]=s^-1`, `[J]=s^-1`, `[Gne]=m^3 s^-1`. Original photo, RR, DR, weak contributions satisfy `n_b q^T F=E_photo+E_RR+E_DR+E_weak` with signed sources in `m^-3 s^-1`. `A^T F=0` for these saved source rates, but does not mean source energy is thermally closed. Evaluate 125 original 21-species states directly through original BASS `AtomicNetwork.evaluate`. No new solve.

## 2. Forward Thomson optical depth and endpoint differential opacity

`x=ln(a/a_start)` with original start `z_start=20`, final `z=2`, and source-defined mean proper Hubble `H=dot a/a>0`. With `c` and `sigma_T` explicit,

`g=d tau_T/dx = c sigma_T n_e/H`, `tau_T(x_start)=0`.

`tau_T(z=2)` is *forward accumulated* from z20 to z2, not the observed backward optical depth to last scattering. Anisotropic line-of-sight radiative transfer (e.g. sky direction, polarization transport) is not performed; this scalar diagnostic alone cannot certify observed CMB tau.

At a common endpoint x, two shear histories A(s=0), B(s=.6) are paired at identical matter/radiation anisotropy targets. Define `R_s=g_s/n_e,s=c sigma_T/H_s`, and `bar u=(u_A+u_B)/2`, `Delta u=u_B-u_A`. Exact polarization gives

`Delta g = bar R Delta n_e + bar n_e Delta R`,

and because `n_e=n_b x_e`, `Delta n_e = bar n_b Delta x_e + bar x_e Delta n_b`. For the saved 25 matched pairs `Delta n_b=0` *at the endpoint*, therefore `Delta g_chem=bar R bar n_b Delta x_e`, `Delta g_geom=bar n_e Delta R`. This finite two-factor identity is exact for the represented inputs. It is not an independent causal decomposition since chemistry and geometry are dynamically coupled along the trajectory.

Original BASS endpoint CSV stores `g=gamma_T_per_H`, `n_e` but not H directly. Consequently `R=g/n_e`, `H=(c sigma_T)/R` reconstruct H from existing source, not a second independent geometry measurement. The PR122 previous ten full-history files provide direct H values at endpoints as cross-check. Dividing by near-identical n_e makes endpoint chemical contribution tiny but cannot estimate the integrated chemical contribution without full histories.

## 3. Nonuniform shear sampling and discrete response

Initial source shear is `[0, 0.003, 0.03, 0.15, 0.6]` for s00..s04; labels `S_F` in the original *endpoint* are not identical to initial shear. For each m,r in {0,..4}, use original stored `tau_{i,mr}=tau_T(x_end; Sigma_i,m,r)`. Define the secant slope

`d_i(m,r)=(tau_{i+1,mr}-tau_{i,mr})/(Sigma_{i+1}-Sigma_i)`.

The sign of `d_{i+1}-d_i` checks sampled discrete concavity in the **nonuniform** shear grid; raw second differences `tau_{i+2}-2tau_{i+1}+tau_i` do **not** test concavity on this grid and must never be used for such a conclusion. The divided-second derivative proxy is

`C_i=2(d_{i+1}-d_i)/(Sigma_{i+2}-Sigma_i)`.

This is a finite-grid numerical pattern, not a bound on the continuum derivative `d^2tau/dSigma^2`. To promote this pattern to an actual model-level local extremum or a generalized monotonicity theorem would require solver error enclosures or more controlled original-source runs. Especially 7 positive s00->s01 increments are only 2.3e-11..3.05e-9 in absolute tau and remain scientifically unresolved.

## 4. Physics-first source results and interpretation

Stored 25/25 shear-0 to shear-.6 Delta tau negative, range `[-1.2517786846776013e-4, -1.2293483168299146e-4]`; mean `-1.2407565482797477e-4`, relative mean `-0.0017893609445001236`. For the **endpoint** local opacity-rate contrast 25/25 geometry fractions `0.999999983225...0.999999983444`; electron charge-density differences `-1.093e-11...-9.013e-12 m^-3`, while `n_b` exactly identical. **The 5 previously derived complete 193-sample histories**, in contrast, had integrated geometry share ~92.02–92.12%, so endpoint-only analysis would erase the 7.9–8.0% cumulative chemical contribution in those five cases. Do not extrapolate it to remaining 20 until their 193-sample histories are replayed.

All 25 tested secant sequences decrease; 18 have negative first secant and 7 have positive first secant. The absolute mixed matter/radiation interaction residual in the 25-pair Delta tau matrix is at most `1.238678e-10` against ~`1.24e-4` signal, but since source-error controls at 1e-10 are not certified this is a measured near-additivity, not proven separability.

Native 125 endpoint F/J re-evaluation confirms mass/charge identities and photo–RR near cancellation. Source-native mean photo electrons `+1.626856488315394e-17 m^-3s^-1`, RR `-1.6265232203066342e-17 m^-3s^-1`, net chemical electron production `+3.3326800875950363e-21 m^-3s^-1`. These are model-generated native values at z2, not new observed plasma rates.

## 5. Literature context, original source authority and claim boundaries

Original BASS chemistry lock: `cosmosapjw-quantum/bass@cc602e...` Git blob `a49aa527...`, original `atomic_network.py` SHA256 `baecce07...`, source manifest plus 129-value coefficient numeric projection. PR122 raw parent SHA256 `93a305a9...` verified. Pontzen & Challinor (2007), [MNRAS DOI](https://doi.org/10.1111/j.1365-2966.2007.12221.x), derive Bianchi-model CMB radiative transfer including scattering and polarization; that *additional transport* has not been performed here. Fleury, Pitrou & Uzan, [arXiv:1410.8473](https://arxiv.org/abs/1410.8473), study light propagation in homogeneous anisotropic cosmologies. Neither paper validates the BASS finite21 chemical source, ionization fit or these numerical outputs.

Source production/observational claim gates: `NO_MORE_IVPS=true`, `NO_MAIN_MERGE=true`, `NO_FORCE_PUSH=true`, `NO_PRODUCTION_PROMOTION=true`, `PR117_DERIVATIVE_FREE_PRIMARY=true`. REI full17 source, common H/K/nb/nHref, P19 bath, physical CMB transfer and independent-author R6/C9 remain NOT_RUN/HOLD. Original DAG VigilODE 7/23, central REI 28/88, working 33/88, successor 0/12 unchanged. Auxiliary new scoped node 1/1 generated, independent review HOLD.
