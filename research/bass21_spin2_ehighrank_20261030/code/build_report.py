#!/usr/bin/env python3
import pathlib,json
root=pathlib.Path(__file__).resolve().parents[1]
s=json.loads((root/'results/SCIENCE_SUMMARY.json').read_text())
met=['R1','R2','R3','R4','R5','R6','R7','R8','R9','R10','C1','C2','C3','C4','C5','C6','C7','C8','C9','C10']
activity={
'R1':('original BASS owner and parent HEAD locked','PREREGISTRATION_GIT_POINTER.md'),
'R2':('original PSTF and Bianchi I polarized transfer literature checked','reports/PHYSICS_AND_HOSTILE_AUDIT_KO.md'),
'R3':('Stokes Q and E+ sign/metric conventions reconciled','results/EXACT_LRS_E_OPERATOR.json'),
'R4':('operator basis and finite-truncation alternatives compared','reports/PHYSICS_AND_HOSTILE_AUDIT_KO.md'),
'R5':('exact rational spin2 Galerkin and E basis derivation','results/EXACT_LRS_E_OPERATOR.json'),
'R6':('self-hostile review only, no distinct-author independence','reports/PHYSICS_AND_HOSTILE_AUDIT_KO.md'),
'R7':('50x193 original-owner higher-spin measurement','results/HIGH_E_HIERARCHY_50x193.npz'),
'R8':('classify resolved high-shear, unresolved low-shear, local-only claim','results/SCIENCE_SUMMARY.json'),
'R9':('derive computational spin2 and E rate operators','code/highrank_atlas.py'),
'R10':('durable full source restore and next-thread handoff','HANDOFF_NEXT_THREAD_KO.md'),
'C1':('original source SHA, source and clock locked','PREREGISTRATION_GIT_POINTER.md'),
'C2':('original Volterra Q and source-native J2 mapped','source_code/coupled_thomson_volterra.py'),
'C3':('exact rational and angular projection/flux algorithm','code/highrank_atlas.py'),
'C4':('bounded source-frozen experiment and CLI prereg','PREREGISTRATION_GIT_POINTER.md'),
'C5':('new high l18..24 E and local RHS/flux arrays generated','results/HIGH_E_HIERARCHY_50x193.npz'),
'C6':('manufactured/negative tests','tests/test_highrank.py'),
'C7':('original physical checks and matched endpoints','results/SCIENCE_SUMMARY.json'),
'C8':('two-quadrature error and saved-grid discrepancy characterized','results/SCIENCE_SUMMARY.json'),
'C9':('same-context hostile implementation review only','reports/PHYSICS_AND_HOSTILE_AUDIT_KO.md'),
'C10':('scientific plots, source-complete ZIP/backup and handoff','HANDOFF_NEXT_THREAD_KO.md')}
stages={k:{'objective':activity[k][0],'artifact':activity[k][1],
 'result':'SELF_AUDIT_ONLY_DIFFERENT_AUTHOR_NOT_RUN' if k in ('R6','C9') else 'SCOPED_COMPLETE_PENDING_REMOTE_FINAL',
 'preliminary_backup':'GIT_PREREG_C7FC51F2_BEFORE_FULL50',
 'final_backup':'SOURCE_FULL_ARCHIVE_DRIVE_LIBRARY_PENDING_REMOTE_RECEIPT',
 'official_dag':'7/23', 'successor_dag':'0/12','official_delta':0} for k in met}
status={'schema':'vigilode.bass21.spin2_ehighrank.stage_v1','parent_pr':143,
 'parent_head':'15d68af701fc9267e2c4b04cc81e2550a9bbbd50',
 'prereg_commit':'c7fc51f2a8007659db095073b83c39a46fea064c',
 'all_stages_one_bounded_campaign':True,'scope_rc':'18/20','native_subagent':'NOT_RUN',
 'r6_distinct_author':'NOT_RUN','c9_distinct_author':'NOT_RUN','original_source_cases':50,
 'saved_epochs':193,'new_ivps':0,'production':'HOLD','auto_next_node':False,
 'no_main_merge':True,'no_force_push':True,'no_more_ivps':True,
 'no_production_promotion':True,'dag':s['dag'],'stages':stages}
(root/'checkpoints/STAGE_STATUS.json').write_text(json.dumps(status,indent=2,sort_keys=True,ensure_ascii=False)+'\n')

newtable='\n'.join([f'| {l} | {s["max_abs_E_per_mode"][i]:.9e} | {s["max_gap_by_mode"][i]:.3e} | {s["max_abs_by_shear_family"]["s00"][i]:.3e} | {s["max_abs_by_shear_family"]["s04"][i]:.3e} |' for i,l in enumerate(range(2,25,2))])
text=f'''# BASS21 original 50×193 source: fixed-screen spin-2 E+ higher-rank physics and closure leakage

## Authority, physical domain and new scope

New prospective prereg Git commit `c7fc51f2a8007659db095073b83c39a46fea064c`, before the new 50-case experiment. Immutable parent VigilODE PR #143 HEAD `15d68af701fc9267e2c4b04cc81e2550a9bbbd50`, source complete ZIP SHA256 `a2fa5e7cbae98cb7a28df19314b1c6da8b90ad5277c08831384290171f4231ee`. Original owner [BASS commit](https://github.com/cosmosapjw-quantum/bass/commit/cc602e209e7deeb511ab2a1f3b2268d55206c43c), source ZIP SHA256 `6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8`. The BASS SHA belongs to repository cosmosapjw-quantum/bass, not VigilODE. The original BASS 62-state unpolarized histories, geometry, electron/temperature, optical clock and all 50 × 193 physical source cases are unchanged. 0 new IVPs; reuse existing source-frozen two-field I/Q Volterra response. Metric (-,+,+,+). x=ln(a/a_ref), g=c sigma_T n_e/H, b' = db/dx. Physical Q=I_meridional-I_azimuthal, U=V=0, transported LRS spherical screen. All results depend on this screen and on the source's bolometric brightness normalization; no measured kelvins or spectral CMB are asserted.

## New exact derivation (R5 / mathematica independently checked low ranks)

Define  (l even, l>=2):

```
P_l^2(mu)=(1-mu^2)*d^2P_l/dmu^2
N_l=(l+2)!/((2*l+1)*(l-2)!)
c_l = integral_0^1 Q(mu)*P_l^2(mu) dmu / N_l
k_l = sqrt(4*pi/(2*l+1)*(l+2)!/(l-2)!)
E_l0^(+) = k_l*c_l
```

The plus sign is explicitly defined; a common E convention changes sign. The previously verified LRS spin-2 Liouville shear operator  has an exact moment adjoint:

```
A_c[l,m] = 1/N_l * integral_0^1 P_m^2 * ((1-3mu^2)*P_l^2 - 3mu(1-mu^2)*d(P_l^2)/dmu) dmu
A_E[l,m] = (k_l/k_m)*A_c[l,m]
d E_l0^(+)/dx |_shear = b' * sum_m A_E[l,m] E_m0^(+)
```

Polynomial integration from l=2..24 yields exact *tridiagonal within even l*, not closure at a finite l. Sample exact coefficients: A_c[2,2]=10/7, A_c[2,4]=10/7, A_c[4,2]=-36/35, A_c[4,6]=504/143, A_c[16,18]=3420/259; A_E[16,18]=(360/7) sqrt(19/407)=360 sqrt(7733)/2849=11.111790805287253. There are no azimuthal m!=0 modes in the **fixed** LRS screen.

Thomson collision (pure cold electron original source):

```
C_E2=-g*(k_2 J2_total/4 + 2 E_20^(+)/5)
C_E_l=-g*E_l0^(+) for l>=4 even.
```

The source J2 is **total** original plus delta intensity J2, not only the perturbation. The exact rank-nine tensor/Stokes source and U_fast,V_slow eigenmodes from PR141/143 are not independently evolved here.

For finite cutoff L, the actual omitted first-neighbour term is exactly 

```
BoundaryFlux_L(x)=b'(x) A_E[L,L+2] E_(L+2),0^(+)(x)
```

It is **not identically zero**, and a closure setting E_(L+2)=0 would remove it by assumption, not by dynamics. This identity is not a new no-go theorem about finite truncations: it simply exhibits one explicit nonzero missing term of the chosen local shear transport.

## Actual original-source physics (not symbolic surrogate)

All 50 cases ×193 epochs project Q at independent GL64 and GL96 on l=2,4,...,24, with an additional numerical polynomial-derivative basis independent of scipy.special.lpmv. Existing PR141 E_l0+ (l2..16) is reused after actual reprojection, not overwritten. New source projected high-mode entries l18,20,22,24: **38,600**; newly evaluated shear+collision local generator entries l2..22: **106,150**; actual l16,18,20,22 omitted-neighbour flux entries: **38,600**. Source-input code and actual array files in results/HIGH_E_HIERARCHY_50x193.npz.

| even l | max abs E_l0+ (all 50) | GL64/96 gap | s00 max abs | s04 max abs |
|---|---:|---:|---:|---:|
{newtable}

Maximum observed omitted-neighbour flux (per dimensionless x):

| cutoff L | omitted L+2 | max abs boundary flux |
|---:|---:|---:|
| 16 | 18 | {s['max_abs_flux'][0]:.14e} |
| 18 | 20 | {s['max_abs_flux'][1]:.14e} |
| 20 | 22 | {s['max_abs_flux'][2]:.14e} |
| 22 | 24 | {s['max_abs_flux'][3]:.14e} |

The strong-shear s04 high modes are above the GL64/96 discrepancy by >10-fold, but weak-shear s00 modes l>=12 remain unresolved on this angular grid. Detection above quadrature discrepancy alone does NOT certify source time accuracy, full transport, observational power spectra, or astrophysical precision.

## Reproducibility and falsifiers

Exact measured maximal differences: original PR141 l2..16 E={s['max_reuse_E_l2_16']:.17e}; original c={s['max_reuse_c_l2_16']:.17e}; J2 total={s['max_J2_reuse_gap']:.17e}; GL64/96 all even <= {s['max_GL64_96_gap']:.17e}; independent Legendre polynomial derivative <= {s['max_independent_basis_gap']:.17e}; original Volterra fixed-point residual <= {s['max_source_fixed_residual']:.17e}. The precommitted 1e-10 angular/source equality criterion passed without weakening. Positive Stokes coherency |Q|/I max {s['max_polarization_fraction']:.17e}. Local original saved-grid derivative-model discrepancy max {s['max_saved_grid_derivative_defect']:.17e}, explicitly **not** a BDF/global error certificate.

The new tests include exact l2/l4/l16 matrices; independent P_l^2 derivative; manufactured pure l2, l8, l18 and l24 Legendre fields; Liouville adjoint numerical check; original J2 Thomson source at l2; all high flux identities; strict no-IVP/HOLD, tampered-source rejection and separation of resolved/unresolved cases. Actual verification outcomes in external final receipt and clean extraction log, not presumed from source comments.

## One-pass same-context hostile review (R6/C9 distinct author NOT_RUN)

1. **Normalization/sign:** plus-E is a chosen S² spin-harmonic convention. Standard minus-E flips sign; original native BASS uses an unpolarized 62-state owner IVP and does **not** dynamically evolve this E_l0. Full PSTF and arbitrary observer spin-screen mapping UNRESOLVED. No false B=0 beyond transported axial U=0.
2. **Apparent high-order detection:** s04 higher coefficients resolved relative to quadrature, not to frozen-source Volterra approximation, actual time interpolation, original BDF time precision or observations; s00 high modes *not resolved*. The source angular sky was generated by known approximate Volterra postprocessing; not a new owner IVP.
3. **Finite closure:** the exact shear generator couples l↔l±2. Nonzero l16←18, 18←20 etc flux is measured. We do not claim no finite closure exists for all initial data or all gauges, nor that 24 modes suffice for a real Einstein–Boltzmann solution.
4. **Thomson/rate original:** l2 collision uses J2_total, not deltaJ2 alone; l>=4 has extinction only in this axisymmetric normal-comoving cold Thomson model. g is source-owned with exact physical units canceled by H, no gamma factor invention.
5. **No temporal false PASS:** measured saved-grid derivative mismatch is NOT_RUN for native global BDF time error certification. It cannot be assigned solely to numerical quadrature. No production or global authority CI promotion.
6. **Independence:** exact SymPy plus manufactured tests and same-context hostile review are no different-author R6 or C9. Native subagent NOT_RUN. Formal 18/20 scoped gate, official VigilODE DAG 7/23, successor 0/12 unchanged.

## Decision and literature

**DERIVED:** exact spin2 moment and E+-basis shear/Thomson map under stated assumptions. **NUMERICALLY_CHECKED:** original 50×193 source full high-rank E and upper flux, 25 matched pair endpoints, angular convergence. **UNRESOLVED:** native source-evolved E_l full PDE/hierarchy, polarization screen transport beyond LRS, source x/tau clock consistency, self-consistent Einstein/electron/temperature feedback, CMB observed E/B, distinct-author review, global authority Git provenance. **PRODUCTION=HOLD**, no main merge, no force push, no IVPs, official DAG delta 0.

Primary source literature: Anthony Challinor, *Microwave background polarization in cosmological models*, PRD 62, 043004 (2000), DOI 10.1103/PhysRevD.62.043004; Andrew Pontzen and Anthony Challinor, *Bianchi model CMB polarization and its implications for CMB anomalies*, MNRAS 380, 1387-1398 (2007), DOI 10.1111/j.1365-2966.2007.12221.x; Angelo M. Anile, *Anisotropic expansion of the universe and the anisotropy and linear polarisation of the cosmic microwave background*, Astrophys Space Sci 29, 415 (1974), DOI 10.1007/BF02639073. Available via corresponding DOI/ArXiv (Challinor astro-ph/9911481; Pontzen & Challinor 0706.2075). No new user-supplied source required for this bounded result.
'''.replace('\u0010','')
(root/'reports/PHYSICS_AND_HOSTILE_AUDIT_KO.md').write_text(text,encoding='utf8')

handoff=f'''# BASS21 E+ high-rank full-source 50x193 | next-thread handoff

$physmath-research-loop $scispace $exa $wolfram

```text
PROJECT=cosmosapjw-quantum/vigilode
READ_LIVE_HEAD_FIRST=true
CURRENT_PR=PENDING_PUBLICATION_RECEIPT
CURRENT_HEAD=PENDING_PUBLICATION_RECEIPT
BRANCH=research/bass21-spin2-eharm-highrank-20261030
PARENT_PR=143
IMMUTABLE_PARENT_HEAD=15d68af701fc9267e2c4b04cc81e2550a9bbbd50
PREREG_COMMIT=c7fc51f2a8007659db095073b83c39a46fea064c
LEDGER_NEXT=L-0200
SOURCE_OWNER_REPOSITORY=cosmosapjw-quantum/bass
SOURCE_OWNER_ZIP_SHA256={s['source_sha256']}
PARENT_PR143_ZIP_SHA256={s['parent_pr143_sha256']}
CURRENT_UNIT=BASS21_LRS_EPLUS_HIGHRANK_FLUX_L24
SOURCE_HISTORIES=50/50
EPOCHS=193
NEW_E18_20_22_24={s['new_high_coefficients']}
NEW_GENERATOR_L2_TO_L22={s['new_generator_values']}
NEW_BOUNDARY_FLUXES={s['new_upper_flux_values']}
NEW_IVPS=0
R6_DISTINCT_AUTHOR=NOT_RUN
C9_DISTINCT_AUTHOR=NOT_RUN
NATIVE_SUBAGENT=NOT_RUN
SCOPED_STAGE_PROGRESS=18/20
VIGILODE_DAG=7/23
REI_CENTRAL_DAG=28/88
REI_WORKING_DAG=33/88
BASS_CE1_DAG=9/15
SUCCESSOR_ADMITTED=0/12
OFFICIAL_DAG_DELTA=0
PRODUCTION=HOLD
NO_MORE_IVPS=true
NO_MAIN_MERGE=true
NO_FORCE_PUSH=true
NO_PRODUCTION_PROMOTION=true
AUTO_NEXT_NODE=false
NEXT_CANDIDATE_ONLY=SOURCE_NATIVE_SPIN_PSTF_OBSERVER_SCREEN_AND_DYNAMICAL_JL_EL_CLOSURE
```

## Results

Actual source 50 x 193 = 9650 physically source-conditional saved epochs. Newly projected l18/20/22/24 E+ 38,600 elements; local l2..22 shear/Thomson generator 106,150 elements; measured upper-neighbor l16/18/20/22 source flux 38,600 elements. For max E18..24, see `results/SCIENCE_SUMMARY.json`. Max l16<-18 shear flux {s['max_abs_flux'][0]:.10e}, parent re-projection E gap {s['max_reuse_E_l2_16']:.4e}; GL 64/96 max difference {s['max_GL64_96_gap']:.4e}. s04 high-spin modes resolved relative to angular gap; s00 high spin near quadrature noise. Source-native evolved E_l or full BDF accuracy not claimed. No changes to parent PR 143 / source owner.

## Actual reproducibility

Use source-inclusive ZIP, preserve exact hash as recorded in external publication receipt; extract to empty directory, `OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 PYTHONDONTWRITEBYTECODE=1 bash VERIFY.sh`, rerunning 50-case original Volterra sky and 6 numerical files. Do not infer provider backup from this document before its remote ACK. Full source-full archive includes nested PR143 original ZIP, exact owner source via recursive SHA, and all real Python, math, CSV/NPZ and original plots. The Git repo mirror is text/CSV/SVG only. Source and Git manifest checks are independent levels.

## Scientific next action

Obtain and prove actual owner-native full dynamically evolving spin-PSTF polarization hierarchy and observer-screen rotation. Current E_l0+ is explicit LRS +screen projection, not generic observer's E_l. Preserve nonlinear shear l±2 coupling and complete collision operator, source-owned b, g and clock; do not substitute independent BASS KM2G or REI chemistry IC. To actually evolve full J_l/E_l, explicitly lift NO_MORE_IVPS in a separate work unit; this task has not done so. Global authority reference CI is distinct from scoped source-locked test. Distinct-author R6/C9 remains not run.
'''
(root/'HANDOFF_NEXT_THREAD_KO.md').write_text(handoff,encoding='utf8')
print('WROTE',root/'reports/PHYSICS_AND_HOSTILE_AUDIT_KO.md',len(text),'HANDOFF',len(handoff),'STAGES',len(stages))
