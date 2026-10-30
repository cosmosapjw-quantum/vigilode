# BASS21 original-source collision eigenmode and shear-memory physics | preregistration, 2026-10-30

## Source lock

Parent is `cosmosapjw-quantum/vigilode` PR #141 at exact HEAD `f14f29d4356f1723ce3efaf8ed15ad06bb955b1a`, `L-0198`. Parent source-complete ZIP `VIGILODE_BASS21_RANK9_EHARM_20261030_FULL_SOURCE.zip`, exactly 47,114,443 bytes, SHA256 `ce2b3507c30f5429fb93244bd7f3de7426f1f158bf52962e24734b6d1237dfe2`. Original independent source owner **cosmosapjw-quantum/bass**, commit `cc602e209e7deeb511ab2a1f3b2268d55206c43c`, original 50×193 original BASS 62-state source ZIP SHA256 `6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8`. No cross-source `KM2F`, `KM2G`, `KM2H`, `CE1` or `REI F4` physics inputs. No new IVPs or dynamics promotion. All original x,b,g,tau and electron source histories unchanged.

## New bounded scientific question

Can one project the source-owned **full** Thomson quadrupole pair `(J_2, B_Q)` into exact collision eigencoordinates and quantify on **all 50×193 original saved source epochs** the collision damping, geometric-shear injection and finite optical-depth *causal shear response* that are missed by an `E2-only` or collision-only closure?

Metric (-,+,+,+), cold normal-comoving electrons, same Stokes `Q=I_theta-I_phi` convention and spin-2 `c_4`; `x=ln(a/a_ref)`, `g=c sigma_T n_e/H`, `tau` original optical depth. Source temperature/electron/clock/geometry frozen. Total J_l = original unpolarized 62-state radiation plus **preexisting** polarized Volterra deltaI. Full intensity sky and Q are re-evaluated using existing PR132 operator, **not** a newly evolved self-consistent owner E_l hierarchy.

Define `U=8 J_2+B_Q`, `V=4 J_2-3 B_Q`, `J_2=(3 U+V)/28`, `B_Q=(4 U-8 V)/28`, with *full* J2 not merely deltaJ2. Original source collision is

`(C_J2,C_BQ)^T=-g [[9/10,3/40],[4/5,2/5]] (J2,BQ)^T`, hence exact `C_U=-g U`, `C_V=-(3/10)g V`. The coherency anisotropy is `M_parallel-M_perp=-(pi/2)V`. In source's original geometric streaming, define `b'=db/dx` and `J4_total` from same source sky and compute

`S_J2^sh=b'(-8 J0/5 -10 J2/7 +36 J4/35)`, `S_BQ^sh=b'(10 BQ/7 + 32 c4/7)`,

`S_U^sh=8 S_J2^sh+S_BQ^sh`, `S_V^sh=4 S_J2^sh-3 S_BQ^sh`.

For these **prescribed precomputed** sources, independent causal shear-memory integrals are `H_U(x)=int_0^x exp[-(tau(x)-tau(s))] S_U^sh(s) ds`, `H_V(x)=int_0^x exp[-3(tau(x)-tau(s))/10] S_V^sh(s) ds`. These are Green-response diagnostics and must **not** be mislabelled the full total J/E self-consistent field. Do not call the local generator independent source-native E_l hierarchy. Compare input `tau` derivative versus `g` as a source consistency check, and sampled-derivative `dU/dx +gU-S_U`, `dV/dx +(3g/10)V-S_V` as **diagnostics only** (not a BDF certificate). A significant residual is retained, not repaired by fitting.

## Prospective real data

New arrays `U,V`, `S_U,S_V`, collision rates, `H_U,H_V`, source J4_total and deltaJ4, exact tensor anisotropy, unforced optical transmissions, derivative residuals, 25 matched-pair saved-time differences, and real endpoint/full sample CSV + scientific plots from all 50 original histories (193 epochs each). Parent's J0/J2/BQ/E_l0 and M anisotropy are immutable comparators; avoid double counting them as new independent observations. Report sign distributions and physically dimensionless ratios where meaningful, not just 50-case averages.

## Pre-set falsifiers

1. Wolfram/SymPy exact symbolic matrix eigenvalues and inversion; manufactured eigenvectors, G=0 and b'=0 limits.
2. Original BASS owner raw SHA exact; all 50/50 original NPZ content hashes verify on new extraction.
3. Reproject full J0/J2/BQ and `c4` from parent fixed Volterra; agree with PR141 and PR134 at max absolute <=1e-10 for each case/epoch; independent GL64/96 J4/c4 discrepancy <=1e-10.
4. `Mparallel-Mperp=-(pi/2)V` absolute max <=1e-10; `C_U+gU=0` and `C_V+3gV/10=0` max <=1e-10 in source units; `d tau/dx≈g` sampled-grid discrepancy **reported without preassumed zero**.
5. Independent quadrature for integrating H modes (cumulative trapezoid vs Simpson) discrepancy reported; *not* used as arbitrary tolerance or proof of global source temporal accuracy.
6. No new IVPs, no convergence/dynamic CMB/observational claims, no production or main merge; any missing source, nonfinite or negative g fails closed. Same-context adversarial review is not real distinct-author R6/C9; native subagent only if actually callable.

## DAG and stopping boundary

Official `VigilODE=7/23`, `REI central=28/88`, `REI working=33/88`, `BASS CE1=9/15`, `successor admitted=0/12`, new official delta 0 unless actual fixed-authority criteria prove otherwise. `NO_MORE_IVPS=true; NO_MAIN_MERGE=true; NO_FORCE_PUSH=true; NO_PRODUCTION_PROMOTION=true; AUTO_NEXT_NODE=false; PRODUCTION=HOLD`. Preregister and then one source-owned 50-case campaign, targeted tests and one self blind review. Backup with exact SHA and restore; Draft child PR only and no automatic successor.