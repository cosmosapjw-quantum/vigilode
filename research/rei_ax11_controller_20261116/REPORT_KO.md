# 원 AXBI controller 오차 성분 진단 및 병목 선택

작성일 2026-11-16. 중앙 검토 pin `56dc609ea9b801c88d3364aed3eee140b3a20d46`, 원 목표 `REI-AXBI-GOAL-20261014`, named consumer는 **REI owner PR377 / AX11 paired-response evolution-error and cost ledger**다. 중앙 여섯 정본 파일과 live HEAD를 확인했고, 기존 census는 재탐색하지 않았다.

원 시나리오 `PLANCKLIKE_MD14_BB50K_CONDITIONAL_HHE_V1`, homogeneous non-tilted axisymmetric Bianchi I, zbar20→4, primary-only Case-A escape를 유지한다. CR/HH/RCT/secondary/molecules는 OFF. F05 OTS, F08 constant-H, PR183의 material-node IVP, PR187의 별도 graph-Schur source, PR188의 unified 후보를 이 물리에 대입하지 않았다. 새 controller나 오차 norm, tolerance 변경은 없다.

## 역할 중복을 피한 실행

작업 중 중앙 REI PR377 `834c8fec...`가 원본/계측/cache/cache+lazy 비교를 사전등록했고, 이후 `946bd77487c9ab27ecd580b26505f8cafb2937bd`로 코드·환경 실패 보존을 게시했다. BASS도 실제 원본 geometry 재사용 비교를 독립 진행 중이었다. 따라서 VigilODE는 같은 대규모 profiling suite를 중복 실행하지 않고, **원 controller의 여섯 성분과 실제 결정에 대한 source-qualified 독립 진단**을 맡았다. PR377의 allocation을 소비하지 않고 별도의 작은 allocation을 native 시작 전에 게시했다.

정본 source identity는 `d7676b9309ac9b084499bbf599e803baecd661c80765489be45839ef5a7cf185`, baseline source corpus는 17파일이다. BASS가 복원한 그 corpus를 모두 SHA256 대조한 뒤 private copy에만 진단 메서드를 덧붙였다. 원 `axbi_driver.rs` 60,056 bytes / SHA256 `a614725c153ee21d88554573c15801087d8fb412954fa51d67b781f671475dbf`는 prefix 전체가 byte-identical이며, 원 tentative/error_metric/adaptive_step 함수 본문은 그대로다. 새 compiled identity는 원본과 다르므로 두 identity를 혼동하지 않는다. 원 checkpoint·config·accepted history 및 사용자 작업 checkout은 변경하지 않았다.

## 원본 error metric의 정식화

원본 full-step 상태 F와 두 half-step 상태 H에 대해

- 세 이온분율: e_i = |x_i(F)-x_i(H)| / max(|x_i(H)|, 1e-5), i=HII,HeII,HeIII.
- 기체에너지: e_w = |w_eV(F)-w_eV(H)| / max(|w_eV(H)|,1e-6), w_eV=w_erg/(1.602176634e-12), eV/H.
- 광자수: e_N = Σ|P_F-P_H| / max(ΣP_H,1e-8), photon/H.
- 광자에너지: e_U = Σ E_j|P_F-P_H| / max(ΣE_j P_H,1e-8), eV/H.

E=max(e_HII,e_HeII,e_HeIII,e_w,e_N,e_U)다. E<1e-5 및 E<2e-4일 때 H를 승인하고, E<2.5e-6이면 다음 dln(a)를 두 배, 그 외 승인에서는 유지한다. 거절 시 절반으로 줄인다. 위 식은 새 norm이 아니라 원 구현을 독립적으로 분해한 것이다. 광자 L1은 총량의 차이 절댓값이 아니라 각 spectral/angular stock의 차이 절댓값 합이다.

## 실제 실행한 12개 원본 비교

ZERO와 SHEAR_PLUS 각각 원 IC 및 정확한 accepted=199980 checkpoint를 사용했다. 각 parent마다 세 h를 검사하고, 진단용 full/half1/half2와 **변경 없는 native adaptive_step**을 별도로 한 번씩 계산했다. 총 12 fixed-parent probes, 24 comparisons, 72 tentative trials, **456 RHS calls**, native process 한 번이다. 어느 경우도 전체 이력의 재시작이나 baseline 재완주는 아니다.

초기 IC의 h는 [7.62939453125e-10,3.814697265625e-10,1.9073486328125e-10]이다. 실제 첫 accepted h의 2배/1배/절반이다. 후기 parent는 a=0.08873100115616298, accepted=199980, h=[3.125e-6,1.5625e-6,7.8125e-7]이다. 후기 original accumulated counters와 wall은 그대로 보존했다.

| 실제 parent / case | h 또는 dln(a) | 지배 성분 | E | 원 결정 / 다음 h |
|---|---:|---|---:|---|
| initial ZERO | 7.62939453125e-10 | photon-energy L1 | 1.185645609726e-5 | 거절 / 절반 |
| initial ZERO | 3.814697265625e-10 | photon-energy L1 | 2.967504106061e-6 | 승인 / 유지 |
| initial ZERO | 1.9073486328125e-10 | photon-energy L1 | 7.422995210987e-7 | 승인 / 두 배 |
| plateau ZERO | 3.125e-6 | photon-number L1 | 2.644638422182e-6 | 승인 / 유지 |
| plateau ZERO | 1.5625e-6 | photon-number L1 | 1.013084697089e-6 | 승인 / 두 배 |
| plateau ZERO | 7.8125e-7 | photon-number L1 | 3.728435468908e-7 | 승인 / 두 배 |

PLUS의 대응 E는 각각 [1.185527192539e-5,2.967207555107e-6,7.422253198351e-7] 및 [2.644631294946e-6,1.013081335361e-6,3.728422009970e-7]로 동일한 지배 성분과 결정을 보였다.

초기 h=3.814697265625e-10에서 ZERO의 two-half photon inventory는 1.3394063126810565e-11 photon/H, 에너지는 2.7463178660328536e-10 eV/H다. 두 denominator floor가 모두 켜진다. 후기 원 h에서는 1.6493062852810022e-6 photon/H, 4.111166887760667e-5 eV/H로 floor가 꺼져 있다. 초기/후기 지배 항의 차이를 gas thermal stiffness나 Newton failure로 잘못 해석하지 않는다.

특히 후기 E≈2.6446e-6은 step 증가 경계 2.5e-6보다 조금 크지만 승인 경계 1e-5보다 작다. **해당 parent에서 step이 유지되는 이유**를 원 controller 그대로 재현했다. 이것이 전체 70.03% plateau의 모든 시점을 설명한다고 주장하지 않는다.

h halving 시 E 비율은 초기 약3.995/3.998, 후기 약2.610/2.717이다. 후기 fixed-parent effective exponent는 약1.384/1.442여서 단순한 E∝h² 비용 추정을 실측 결과로 대체할 수 없다. 이는 세 fixed-parent 국소 비교이며 full-IC T1/T2 수렴성이 아니다. `1e-5 / 2.5e-6 / 6.25e-7`에 대한 결과의 추가 predicate는 동일 pair에 대한 산술 판정일 뿐, 새 tolerance의 이력을 실행한 증거가 아니다.

## 독립 검증과 실행비용

저장된 full/two-half/actual original checkpoint를 별도 Python 구현으로 읽어 여섯 성분을 다시 계산했다. **12/12 성분 최대값과 원 metric이 bit-identical**, **12/12 실제 승인·거절·다음 step·counter 일치**, **12/12 actual gas/photon/ledger 상태가 승인된 H 또는 거절된 parent와 정확히 일치**했다. 고정밀 합산(math.fsum) 대비 성분의 최대 상대차는1.28593e-15였다.

Native wall **0.415899711 s**, CPU **0.381591 s**, process peak RSS 10,044 KiB. 핵심 tentative3회와 unchanged adaptive_step, 단순 snapshot 기록 시간을 각각 CSV에 분리했다. 이 시간은 12진단의 비용이며 최적화 speedup을 뜻하지 않는다. 10개의 승인 결정과2개의 거절 결정은 독립 복제 parent에 대한 이벤트로 원 accepted history에 추가하지 않는다.

적대적 저장증거 검사: 오염된 component, 틀린 winner, 누락된 probe, 위조된 next-step, NaN, 변경된 checkpoint를 **6/6 normal 및 6/6 optimized Python**에서 거부했다. 원 source용 Rust regression2/2는 실제 red→green을 기록했다. 이후 read-only checkpoint 노출 seam의 release compile과 실제12개 비교가 성공했다. 전체 repo CI 또는 다른 scientific tests의 PASS는 주장하지 않는다.

실패도 보존했다. 첫 test build는 runner가 native8MiB 파일 제한을 compiler incremental 파일에 잘못 적용해 SIGXFSZ로 중단됐고, 당시 `EXPECTED_RED` 라벨은 잘못된 분류였다. `FIRST_FAILURE_CLASSIFICATION.json`에서 이를 명시해 TDD red로 쓰지 않는다. 제한 범주를 바로잡고 incremental/debugmetadata를 끈 뒤 실제2개 assertion red를 관측했다. 첫 example build의 private checkpoint_text 접근 오류는 원 함수 변경 없이 읽기 전용 wrapper로 수정했고, 기존 두 build 한도 안에서 종료했다. 실패 포함 전체 build/test/native wall **12.711816812 s**, CPU **12.349678 s**, historical unknown cost는 여전히 UNKNOWN이다.

## 비용 병목 선택: 기존 BASS 측정의 검증된 재사용

BASS 작업의 36개 timing rows와324개 saved output 항목을 실제 해시/byte수로 대조했다. 비기준 비교는216개이며 저장 native state, gas/photon/ledger, 원 cadence 및 output가 exact하다. BASS source hash와 결과 파일별 해시를 `DONOR_COST_CONSUMPTION.json`에 고정했다. 읽은 시점에는 결과 일부가 아직 미커밋이었으므로 중앙 승인 또는 published result로 자동 간주하지 않는다.

BASS original32accepted early/plateau windows의 계측에서 background quadrature는 전체 window wall의 **96.16–97.45%**다. 이는 adaptive/output에 중첩된 시간으로 다시 더하면 안 된다. 동일 출력 exact-reuse candidate/original median wall비는0.0432–0.0574였으나, 이는 **BASS가 실행한 해당 짧은 window의 결과**다. VigilODE가 새로 측정한 whole-IVP 개선으로 쓰지 않는다. BASS plateau parent는 accepted200000, 본 진단 parent는199980으로 다르다.

따라서 현재 측정으로 선택할 실행시간 병목은 **point_at_scale의 중복4097-node quadrature**, step 수/승인 쪽 지배 항은 검사한 parent에서의 **photon L1 metric**이다. 이 두 병목을 구분한다. chemistry-only, feedback-only, directory-accounting-only의 별도 CPU share는 이 데이터로 확정되지 않았다. lazy-spectrum 실제 변경은 owner PR377 경로에서 별도 원 reserve/cadence/state parity를 검증해야 하며 본 턴에는 구현/측정 완료를 주장하지 않는다.

## Owner 소비와 진전 한계

이번 실물 증거는 AX11의 이전 미기록 dominant error component를 실제 원본 parent에서 특정하고, geometry 재사용의 기존 실측과 결합해 다음 수정의 우선순위를 정했다. 해당 12-probe 결과는 owner PR377의 오류성분/비용 ledger에 소비하도록 전달한다. 전체 AX11의 full-IC 두 refinement와 dt/dE/dmu 분리, fixed-a/equal-elapsed-proper-time response는 여전히 미완료다.

**원 목표10/17, expansion28/88 유지.** 새 과학 노드 승인0, production HOLD. 작은 supplier 단위의 완료는 AX11 전체 완료가 아니다. 원 controller, tolerance, source/IC, energy-angle grid, spectra reserve, 미완료 historical 비용을 바꾸지 않았다. PR183 F05·PR187 negative work-precision·PR188 T02를 별도 보존하고, 범위 밖 solver 교체·새 controller·abstract gate를 추가하지 않는다.
