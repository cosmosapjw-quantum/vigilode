# VigilODE × BASS21 | new LRS photon polarization source-owned handoff

$physmath-research-loop $scispace $exa $wolfram

```makefile
PROJECT=cosmosapjw-quantum/vigilode
BASE_PR=127
BASE_HEAD=9f7a5a6be49673265011263b5fc8304de9b9f132
IMMUTABLE_PARENT_PR=124
IMMUTABLE_PARENT_HEAD=e48a049e143bb9d57a1d9e2fb2b570c32d0db2dc
SOURCE_OWNER=cosmosapjw-quantum/bass@cc602e209e7deeb511ab2a1f3b2268d55206c43c
ATOMIC_SOURCE_SHA256=baecce07aded483fa60006cc576feff67d068c0d70394f7095abacc6d47eaa80
ORIGINAL_SOURCE_FULL_ZIP_SHA256=6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8
UNIT=BASS21_SOURCE_OWNED_LRS_SINGLE_SCATTER_POLARIZATION_20261030
GENERATED_Q=50x193x17=164050
GENERATED_PAIRED_DELTA_Q=25x193x17=82025
SHAPLEY_ENDPOINT_ROWS=25x17=425
NEW_IVPS=0
NEW_UNIT_SCOPED=18/20
R6_DISTINCT_AUTHOR=NOT_RUN
C9_DISTINCT_AUTHOR=NOT_RUN
NATIVE_SUBAGENT=NOT_RUN
VIGILODE_DAG=7/23
REI_CENTRAL_DAG=28/88
REI_WORKING_DAG=33/88
BASS_CE1_OTHER_SOURCE_DAG=9/15
SUCCESSOR_ADMITTED_DAG=0/12
OFFICIAL_DAG_DELTA=0
AUXILIARY_SCIENCE_DATA=1/1_SCOPED_HOLD
PRODUCTION=HOLD
NO_MORE_IVPS=true
NO_MAIN_MERGE=true
NO_FORCE_PUSH=true
NO_PRODUCTION_PROMOTION=true
NO_TOLERANCE_WEAKENING=true
AUTO_NEXT_NODE=false
```

## Source identity and real science

- The original BASS full62 NPZ has SAME_EVENT b(t), photon energy Legendre J2(t), proper ne, H, and forward tau, all original 50 files/193 epochs SHA checked. No six-cell CE1 observer sky is imported; CE1 41-epoch event mapping remains unsupported.
- Cold normal-comoving source, signature (-,+,+,+), `x=ln(a/a_ref)`. LRS null conserved q_i. For observer mu, δb=b(x)-b(xo), `D²=(1-mu²)exp(2δb)+mu²exp(-4δb)`, `mu_x=mu exp(-2δb)/D`.
- Original unpolarized energy brightness `F(μ,x)=sum_(ell=0)^8(2ell+1)J_ell(x)P_ell(μ)`. Exact cold Thomson gain `Qgain=-3/4 J2(1-mu_x²)`.
- SOURCE-FROZEN first-gain LOS: `Qtilde_o=-3/4 ∫_0^xo dx [c σT ne/H] J2 exp[-(tau_o-tau_x)] (1-mu_x²)/D^4`. `Q/I=Qtilde/F(mu_o,xo)`. Polarization feedback and multiple gain interactions omitted; not full polarized Einstein–Boltzmann CMB or observed z=0 tau.
- Computed 164050 Q, 82025 pair ΔQ, matched equatorial mean ΔQ=+0.010318432952964909 at z2; max |Q/I|=0.011741191785797517.
- Exact 3-group Shapley mean ΔQ fractions: J2 +95.46312064681434%, geometry b +4.61716367115622%, (g,tau) -0.08028431797057%; algebraic partition, NOT causal.
- 13 focused Python tests PASS, Wolfram exact 0 identities, 193→97 sampled-grid Simpson change max 1.151072350283333e-9; NO BDF global or polarized feedback error certificate.

## What to read/replay

1. Read `THEORY_KO.md`, `REPORT_KO.md`, `provenance/SOURCE_IDENTITY.json`, `DAG_DELTA.json` and original 23,770,654-byte complete archive; do not rely on Github text mirror to replace binary original NPZ.
2. In full source-inclusive new ZIP, `bash VERIFY.sh` reconstructs all Q, matches deterministic numerical/content, validates 13 tests and refinements.
3. Check `FILE_MANIFEST.json` for immutable package hashes. Original archive is nested under `source/` (its original bytes preserved).
4. Examine `stages/R1..R10,C1..C10.json`: retrospective evidence snapshots, not 20 independent preregistrations; independent-author R6,C9 remain NOT_RUN. Per-stage upload/publish receipts must name actual provider ACK, not inferred status.

## Required future direction, NOT automatically executed

Need source-consistent **full polarized Thomson gain with E2 feedback**, screen/geodesic transfer and electron/temperature regime certificate; eventually source same-event observer history z<=0, spectra and real CMB EB observables. Do not mix REI17, P19, Grackle as interchangeable chemistry. Preserve PR127 GitHub ledger FAIL and all historical failures; no merging/force pushes/production/solver rewrites/IVPs.