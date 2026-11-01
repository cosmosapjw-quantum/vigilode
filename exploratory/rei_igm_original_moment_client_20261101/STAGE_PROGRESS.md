# R1–R10 / C1–C10: 단일 bounded VigilODE → REI IGM 원본 광자 client

모든 단계는 **하나의 연구 단위**다. 별개인 20개 물리 실험이나 20개 DAG 노드로 계산하지 않는다. 최초 실험의 증거는 `frozen_pr332_source.json`, `reference_trajectory.json`, `generated/RESULTS.json`, `SOURCE_46080_CELL_PHOTOELECTRONS.csv.gz`, `tests/test_moments.py`와 full-source backup에 있다. `PYTHON-SCIENCE` 사전 단계의 실제 실행 PASS, native Rust는 원격 실행 receipt를 읽은 뒤만 승격한다.

각 단계 종료의 보고된 DAG: **VigilODE 7/23, REI central 28/88, REI working 33/88, REI Fast 4/7, BASS CE1 9/15, 공식 Δ0** (서로 다른 DAG, 최신 소유자 승인으로 재계산되지 않음).

| 이론 단계 | 실제 증거·범위 | 코딩 단계 | 실제 증거·범위 | 보존·게시 |
|---|---|---|---|---|
| R1 | owner REI, method owner VigilODE, original interval HOLD 계약 | C1 | 46k, 4 photon-group, 3 moments, original one tick 계약 | P0 ZIP + single Draft PR 예정 |
| R2 | 원본 PR332 source, REI PR342 remainder, VigilODE PR169 mission policy 재사용 | C2 | source group J,p,e,S0의 실제 REI 원본 ZIP byte SHA 정렬 | P0 ZIP + Git text |
| R3 | fixed forcing vs evolving original RT/heat/OTS 구분 | C3 | moment→6-component state, analytic JVP | P0 ZIP + Git text |
| R4 | full 46k vs exact global moment reduction & counterfactual 선택 | C4 | independent SciPy, native RODAS5P bounded CI | P0 ZIP + Git text |
| R5 | shared source denominator/photo conserved moment closure 직접 유도 | C5 | Python & Rust executable code 작성 | P0 ZIP + Git text |
| R6 | source-conditional vs owner admission self-referee; real failed attempts preserved | C6 | Python 10/10 normal and optimized, native still CI-dependent | P0 ZIP + Git text |
| R7 | 46,080 source-cell cumulative photoelectron actual calculation | C7 | photon & nuclei conservation and source-replay finite checks | P0 ZIP + Git text |
| R8 | one-tick original-source HI/HeI photon absorption response quantified | C8 | Python Radau/DOP853 concordance 2.22e-16; native test pending | P0 ZIP + Git text |
| R9 | source reduced IVP time/units/JVP mapped to native solver | C9 | same-context hostile check; distinct author NOT_RUN | P0 ZIP + Git text |
| R10 | immutable input/result status and next REI M1 dependency recorded | C10 | full reproducible ZIP, next thread handoff; native deploy PASS pending | P0 ZIP + Git text |

## Stage conversion policy

- SOURCE_PRECI / P0: before remote publication: original source ZIP + Python/Rust source + theory + model-generated full 46k result + tests + first scientific summary, ZIP CRC and SHA.
- SCIENCE_REMOTE / P1: isolate VigilODE branch from exact PR169 owner mission parent, upload source/tests/source-specific workflow and preserve native result/failures. `PR_CREATED` alone ≠ native PASS.
- CLOSEOUT / P2: only after native readback update report/ledger with actual original source frozen RODAS5P result and archive identity; preserve any CI failure, admit central Δ0.

Original REI 26-event full-X `D_h` Jacobian+uniform remainder **NOT_RUN**. Original accepted tick160/131072, failed `[160,161]` logT error `2.1245050576368385e-4>2e-4` unchanged. `NO_MERGE`, `NO_FORCE_PUSH`, `AUTO_NEXT_NODE=false`, production HOLD.
