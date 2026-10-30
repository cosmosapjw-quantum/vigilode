# VigilODE × BASS21 | next-thread copyable handoff (2026-10-30)

$physmath-research-loop $scispace $exa $wolfram

PROJECT=cosmosapjw-quantum/vigilode
CURRENT_UNIT=BASS21_MATCHED25_FULL_HISTORY_SOURCE_REPLAY_20261030
IMMUTABLE_PARENT_PR=124
IMMUTABLE_PARENT_HEAD=e48a049e143bb9d57a1d9e2fb2b570c32d0db2dc
ORIGINAL_BASS_OWNER=cosmosapjw-quantum/bass@cc602e209e7deeb511ab2a1f3b2268d55206c43c
ORIGINAL_ATOMIC_NETWORK_SHA256=baecce07aded483fa60006cc576feff67d068c0d70394f7095abacc6d47eaa80
NEW_BRANCH=research/bass21-full25-original-saved193-physics-20261030
NEW_PR_AND_HEAD=see_separate_actual_PUBLICATION_RECEIPT.json
AUTO_NEXT_NODE=false
NO_MORE_IVPS=true
NO_MAIN_MERGE=true
NO_FORCE_PUSH=true
NO_PRODUCTION_PROMOTION=true
NO_TOLERANCE_WEAKENING=true

## 실제 확인된 단위

- 원본 BASS native source owner에서 50/50 NPZ saved histories, 각각 193점, SHA256/Git blob exact+NPZ CRC. 40개 재확보+기존 PR122 10개 (총 12,754,469 source bytes). 오염된 중간 transport 1건 폐기 후 정상 데이터 복원.
- 9650회 원본 21-component `AtomicNetwork.evaluate`: 21×21 J, source F, 7 H/He/Li 광이온화 계수, photoelectron, RR, DR, photon absorption/primary kinetic work. 모든 계산 source-owned saved state이며 새 IVP 0개.
- 25/25 matched pair의 동기화한 193시점 원본 H, xe, nb,T,UV와 4825 time rows에서 정확한 유한 Δgamma=geometry+ionization+density 분해. Density difference 0; mean geometry share 0.9207015546065044, ionization share 0.07929844539349523; 25/25 original Δτ negative. Ionization contribution의 평균 99.0996344%가 source z 11.91055→6.93725에 집중됨.
- Native photon-rate 7종 × 25쌍 × 193시점 비교 33,775개 exact 동일 (원래 prescribed bath). 이전 PR122 다섯 pair Δτ/Simpson/chem/geometry 모두 0차 회귀.
- 100 rational exact-identity checks, Wolfram `FullSimplify=0`, 새 Python 20/20 PASS, 7 deterministic science outputs repeat SHA exact. Direct one-point F/J calling context sub-ulp drift ~1e-32 exists, byte identity라고 주장하지 않음. BDF-saved vs sampled Simpson max ~6.8131e-8 per original trajectory, paired contrast max 5.2269e-10. Convergence-certified solver error가 아니라 sample quadrature comparison.
- 학술적 범위: Pontzen–Challinor 2007 polarized Bianchi transfer, Fleury–Pitrou–Uzan 2015 Bianchi I geodesic 참고. 이번 `c sigma_T ne/H` homogeneous forward-clock surrogate를 관측 CMB 7 optical depth / 편광 맵과 동일시하지 않음.

## DAG/claim gate

- VigilODE official **7/23** / N03 **17/20 scoped historic**; REI central **28/88**, working **33/88**, successor admitted **0/12**. Original DAG physical node delta 0. Auxiliary BASS21 full25 saved history **1/1 SCOPED_DATA**, distinct-author independent R6/C9 **NOT_RUN**, review+production HOLD.
- R/C retrospective status: R1–5,R7–10,C1–7,C10 = 17/20 SCOPED_COMPLETE, C8 = 1/20 PARTIAL (original solver global convergence not validated), R6/C9 = 2/20 NOT_RUN. 별도 독립 subagent 노출이 없다면 serial만 수행했다고 기록.
- PR126 C9 source-binding fake-owner vulnerability는 서로 다른 N01 derivative-free research track. 이번 BASS raw original `AtomicNetwork` input/owner와 혼합하지 말고 old failure receipts와 보호 원칙 유지.

## 재현명령 및 산출물

원본 source-inclusive ZIP을 풀고 해당 root에서 `bash VERIFY.sh`; 최초 `code/verify_hashes.py static`는 원본 SHA를 검사한다. 20 tests, 9650 evaluation replay, quadrature/plots, 7 regenerated result SHA를 모두 확인한다. 이후 `results/SCIENCE_SUMMARY.json`, `results/MATCHED_25_PAIR_SUMMARY.csv`, `results/ALL_50_NATIVE_FJ.npz`, `results/MATCHED_25_X_193_OPTICAL_DEPTH.csv`, `THEORY_KO.md`, `REPORT_KO.md`, `STAGE_LEDGER.json`을 읽는다. Original `inputs/`를 수정하거나 IVP를 다시 풀지 않는다.

## 다음 물리 연구의 최소 입장 계약

1. BASS 최신 CE1 angular Liouville sky와 원본 source owner/scale-factor/clock, observer tetrad, photon comoving momentum, density/electron frame을 source-by-source 인증한다. 지금의 25×193 chemistry는 오직 BASS original source model 저장 이력이다.
2. 실제 Thomson collision term의 directional photon streaming, angular optical depth and E/B multipole propagation을 유도하고 동일 owner의 선형화 범위를 판별한다. 193-point reionization source surrogate만 가지고 CMB observational posterior 생성 금지.
3. 별도 다른-author R6/C9 가능 시 수행하고, native RODAS stage transaction을 독립 admission으로 보낸다. REI17/P19/Li/Grackle 별도 source physics와 동일시하지 않는다.
4. 기존 PR117 causal derivative-free RHS/value-only 우선순위, Q-LEDGER/Q-API/Q-GIVENS, eight suites/ten interactions, old PR118 prereg/ledger CI failure, resource budget/tolerances/branch protection을 유지한다. 선행 PR merge나 force push 금지.

**Publication receipt:** actual final GitHub PR/head and Google Drive/Library uploaded ID, SHA and download link are kept outside this self-contained archive as `PUBLICATION_RECEIPT.json`; remote upload ACK and raw readback are different evidence states.
