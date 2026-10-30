# VigilODE × BASS21 rank-nine tensor / screen-fixed E | executable handoff

$physmath-research-loop $scispace $exa $wolfram

```
PROJECT=cosmosapjw-quantum/vigilode
READ_LIVE_HEAD_FIRST=true
PARENT_PR=138
IMMUTABLE_PARENT_HEAD=c61484b0c29b1cb9835a34b831130982017ddc07
BRANCH=research/bass21-rank9-stokes-e-crosswalk-20261030
PREREG_COMMIT=494bc88a7d284923b668d86703930fcca2891896
UNIT=BASS21_RANK9_COHERENCY_SCREEN_E_20261030
ORIGINAL_OWNER_REPO=cosmosapjw-quantum/bass
ORIGINAL_OWNER_ZIP_SHA256=6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8
PARENT_PR138_ZIP_SHA256=ab7044f604fdd7badb7eb21bae60f7900dd47e1e81b0ed1cce3091283162309f
SOURCE_50=50/50
EPOCHS=193
NEW_IVPS=0
NEW_RANK9_COHERENCY=28950
NEW_E_l0_PLUS=77200
NEW_STOKES_GAIN=96500
NEW_COLLISION_RATES=19300
R6_DISTINCT_AUTHOR=NOT_RUN
C9_DISTINCT_AUTHOR=NOT_RUN
NATIVE_SUBAGENT=NOT_RUN
VIGILODE_DAG=7/23
REI_CENTRAL_DAG=28/88
REI_WORKING_DAG=33/88
BASS_CE1_DAG=9/15
SUCCESSOR_ADMITTED=0/12
OFFICIAL_DAG_DELTA=0
PRODUCTION=HOLD
NO_MAIN_MERGE=true
NO_FORCE_PUSH=true
NO_MORE_IVPS=true
NO_PRODUCTION_PROMOTION=true
AUTO_NEXT_NODE=false
NEXT_CANDIDATE_ONLY=OWNER_NATIVE_POLARIZATION_E_l_TIME_HIERARCHY_AND_CLOSURE
```

## Actual physics

New exact tensor-Stokes equivalence:
- `Mperp=4pi(J0/3+J2/6-BQ/8)` and `Mparallel=4pi((J0-J2)/3+BQ/4)`;
- `S_I=J0+(J2/2-3BQ/8)P2`, `S_Q=(-3J2/4+9BQ/16)(1-mu^2)`;
- exact collision eigenvalues for (J2,BQ): `-1,-3/10` after dividing g;
- fixed screen `E_l0^(+)=sqrt(4pi/(2l+1)*(l+2)!/(l-2)!)*c_l`, `B_l0^(+)=0` in this parity-even U=0 screen; **standard CMB minus-sign convention reverses E**, and this is NOT owner-native full E_l.

All new 50*193 data and 25 matched pairs come from the original previously solved Volterra deltaI/Q and immutable BASS histories (no new IVPs). In particular `max |E20^(+)|=0.035012283834940566`, mean paired final `Delta E20^(+)=0.025577680075470167`, `max |Mparallel-Mperp|=1.8152473513681096`. Actual tensor-vs-Stokes gain max `5.0e-16`; explicit 3D vs analytic tensor max `3.11e-14`; GL64/96 harmonic max `8.13e-17`. These are *angular/source-conditional* checks, not global BDF or observed CMB certificates. Same-author hostile referee in `reports/PHYSICS_AND_AUDIT_KO.md`.

## Restore and next action

Source-complete ZIP contains immutable original PR138 ZIP, exact source helpers, parent NPZ, runnable code, full science NPZ, CSV, maths, tests and plots. Extract it and run `OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 bash VERIFY.sh` for full original owner nested archive SHA reconstruction, 50/50 source NPZ SHA, 9/9 symbolic/science tests, six deterministic outputs replay. The source-full ZIP SHA/bytes, Git Draft PR and CI run identifiers must be taken from the **external final publication receipt**; this file intentionally does not attempt self-referential archive hashing.

Next owner-native dynamic `E_l` stage remains unexecuted: obtain exact owner spin basis conventions and future complete photon+Einstein/electron IC/event, prove coefficient identity with this `E_l0^(+)` or exhibit counterexample, then only with explicit override of NO_MORE_IVPS conduct prospective new evolution. Do not splice KM2G synthetic IC into this source. Historical global authority CI remains distinct from scoped science green; no new official DAG admission. Literature: Challinor PRD62 043004 (2000), Pontzen & Challinor MNRAS380 1387 (2007).
