## 해결하는 문제

Causal Picard가 변하지 않은 projected input의 RHS와 이미 확정된 삼각 풀이 prefix를 반복 계산하고, adapter의 implicit entry pilot은 explicit checkpoint가 이미 얻은 step proposal을 버린 채 전역 h0로 다시 시작했다. 완료된 pilot이 실제로 더 저렴해도 기존의 임의 20% 비용 margin 때문에 선택되지 않는 경우도 확인했다. 이 PR은 세 병목을 분리하여 고치고, 구조를 이용한 derivative-free raw-U 소비자와 RHS-only homotopy/RK 소비자에 증명에서 유도한 오차 예산을 이식한다.

기준은 PR90의 `68a28357d960c4c2743c299770f0a33ef63245b2`다. Base는 `research/rigorous-performance-audit-20261027`, head는 `research/derivative-free-causal-homotopy-20261027`이다. PR74/75의 원래 목표식·stage 병렬화·실패 작업량 과금, PR77의 구조적 분해, PR90의 비용 하한과 checkpoint 보존을 실제 구현에서 재사용했다. 기존 적분식과 대조 경로를 유지한 채 네 경로의 이론·코딩 연구 루프를 병렬로 수행했다.

## 구현과 수학적 근거

- **Causal 재사용:** `BatchedPicardReuse`와 adapter의 `PicardReuse` 경로를 추가했다. 고정 epoch에서 전체 projected input이 bit 단위로 같을 때만 RHS를 재사용하고, 같은 RHS prefix의 기존 LU 결과를 보존한다. 모든 worker 결과를 회수하여 실패 작업까지 청구하고 full-space residual 검사를 유지한다. 유한 causal DAG의 오차 전달은 `Σ L^j`로 끝나므로 전역 수축을 요구하지 않지만 비정규 증폭은 남는다.
- **구조적 raw-U:** 선언된 affine mean-field 모델을 collective/transverse 두 스칼라 점화식으로 정확히 분해하고 모든 상태 성분을 병렬 복원한다. 원래 8-stage 목표와 임의 transverse 상태·forcing을 보존한다. 모든 step/stage residual에서 출력으로 가는 sharp 누적 norm `C_N`을 유도하여 `delta=epsilon_alg/C_N`을 사용한다.
- **RHS homotopy/RK:** 실제 후보 잔차가 Picard 잔차보다 크지 않은 `eta=1` 수용 규칙과 accepted-stage 초기값 재사용을 구현했다. Cubic midpoint의 강한 단조성, 비팽창성, 유한-step 절단오차로 step 수와 residual 허용치를 계산한다. 고정 모델에서는 `C=75/64`이며, 임의 절반 분할 대신 절단오차를 제외한 잔여 예산을 사용한다. Outward residual과 실제 상태·시간 반올림을 물리 오차 상한에 포함한다.
- **선택적 JVP adapter:** 미래 endpoint 정보가 아닌 batch 시작 전 checkpoint proposal을 entry pilot에 전달한다. 새 strict 옵션은 같은 구간을 완료한 후보의 기존 작업량 `W=RHS+JVP+ft+n·Jacobian+solve`를 checked `u128`로 비교한다. 목적함수의 strict 감소 조건에서 경계 `I<E`, callback 하한에서 중단 cap `E−1`이 나온다. 동률은 explicit를 유지하며, 실험 경로는 제공된 JVP만 허용한다. 두 새 옵션은 기본 false이고 strict 판정은 양의 pilot budget에 적용된다.

## 실제 개선 결과

| 경로 | 같은 목표·정확도 계약에서 관측한 결과 |
|---|---|
| Causal | 108/108 후보의 stage/output/error bits 동일. 전체 Riccati 궤적에서 RHS **1868→1702**, block solve **1456→1290**; JVP726과 모든 관측 상태·시도 결정은 동일. |
| Structural | 64-step 전체 RHS **1024→576**, shifted resolvent512→0. n2048/worker1에서 매 step 새 plan을 포함한 paired 중앙값 **1.883배**; 같은 cap에서 최대 공통-grid 오차2.398e-10. |
| Homotopy 정확도 계획 | 요구 물리 오차1e-5에서 증명으로 N512를 선택. RHS와 interval 평가를 모두 세면 **2092→1659**, 12/12 arm이 모든 반환점의 outward bound와 오차 요구를 통과. |
| Adapter | rate1000/10000에서 RHS-only 전체 work **2377→1691**, **19261→1629**로 28.86%/91.54% 감소. 초기 실패 pilot과 return probe 포함. 쉬운 rate1은 JVP0 유지. |

Structural 시간은 할당, plan 생성, 전체 residual 검사를 포함하며 동일 worker 수의 강한 derivative-free 순차 기준과 비교했다. 1.883배는 5개 paired block 중앙값이고 관측 범위는 1.472–1.943배다. 보편적인 시간 가속률로 확대하지 않는다.

새 proof-budget homotopy 실험에서 같은 seed의 Picard와 가속 정책은 평가 횟수가 같았다. 따라서 **2092→1659 절감은 accepted-stage 시드 재사용에 귀속**하며 Anderson만의 효과로 설명하지 않는다. 앞선 고정-grid 결과424→296과 warm-only/acceleration-only 대조도 별도로 보존했다. RHS, interval 평가, JVP의 실제 시간 비용이 같다고 가정하지 않는다.

## 검증, 음성 대조군, 남은 범위

**서로 다른 native 계약59개 PASS**, 변경한 두 crate의 **locked/offline all-target compile PASS**다. Exact rational 검산, 독립 scalar/dense oracle, 전체 상태의 정확해 비교, directed interval 검증을 함께 사용했다. Adapter의 12개 arm은 원래1e-6 cap을 유지했고 callback atomics와 native counter가 일치했다. 정수 판정에는 동률, 2^53 이상 한 단위 차이, overflow, 잘못된 replay endpoint 계약을 포함했다.

불리한 결과도 유지했다. Causal forward는 일부 목표에서 여전히 가장 저렴하고 작은 병렬 셀은 느려졌다. 별도 causal root-attempt timing은 외부 적분기가 거절한 step이므로 accepted-trajectory 가속으로 쓰지 않는다. 직접 RK4는4회인데 Picard20회, screened36회였고 continuation이12회를28회로 늘린 경우도 있다. 시드만 고치고 기존 .8 margin을 유지한 adapter는 전체 비용에서 RHS-only보다 비쌌다. 진단 후 추가한 seed/parity/exact 정책 및 정확도 계획은 별도 후속 프로토콜로 표시했으며 사전 고정 결과나 holdout으로 바꾸어 설명하지 않았다.

Causal 경로에는 **frozen JVP와 작은 projected LU**가 남는다. Structural의 derivative-free 주장은 선언된 모델에서 상태 행렬과 매 step factorization이 없다는 범위다. 공유 tableau loader의 고정 **8×8 metadata 역행렬**은 프로세스당 한 번 수행되고 timing 전 diagnostics가 초기화했다. Structural `CoreResult`는 실패 시 partial-work를 반환하지 않으므로 adaptive fallback 연결 전 과금 계약 보완이 필요하다. 일반 비선형 closure, carrier 변경의 고차 order 조건, 비수축 homotopy의 branch 제어, 미래 구간 비용 예측은 추가 연구 과제다. 기존 pressure/backoff/return 규칙 전체나 임의 RHS의 물리 오차를 인증한 결과는 아니다.

최종 원자료 감사에서 debug 취소 receipt의 `tests_executed:false`가 완성 로그와 모순됨을 교정했다. 짧은 로그는 완성 로그의 byte prefix였고, 긴 로그에는6+6 PASS가 있었다. 원 receipt와 두 로그를 보존한 correction receipt를 추가했으며 measurement-profile 실행만 대표 근거로 선택하여 중복 집계하지 않았다.

## 검토 자료

[개요와 결과](https://github.com/cosmosapjw-quantum/vigilode/blob/research/derivative-free-causal-homotopy-20261027/research/derivative_free_causal_homotopy_20261027/README_KO.md), [재현 명령](https://github.com/cosmosapjw-quantum/vigilode/blob/research/derivative-free-causal-homotopy-20261027/research/derivative_free_causal_homotopy_20261027/REPRODUCE.md), [R1–R10/C1–C10 실행 기록](https://github.com/cosmosapjw-quantum/vigilode/blob/research/derivative-free-causal-homotopy-20261027/research/derivative_free_causal_homotopy_20261027/RESEARCH_LOOPS.md), [정리와 증명](https://github.com/cosmosapjw-quantum/vigilode/blob/research/derivative-free-causal-homotopy-20261027/research/derivative_free_causal_homotopy_20261027/theory/THEORY.md), [REVTeX 원문](https://github.com/cosmosapjw-quantum/vigilode/blob/research/derivative-free-causal-homotopy-20261027/research/derivative_free_causal_homotopy_20261027/report/derivative_free_causal_homotopy.tex)을 제공한다. 세부 node는 [causal](https://github.com/cosmosapjw-quantum/vigilode/blob/research/derivative-free-causal-homotopy-20261027/research/derivative_free_causal_homotopy_20261027/causal/REPORT_KO.md), [structural 상수](https://github.com/cosmosapjw-quantum/vigilode/blob/research/derivative-free-causal-homotopy-20261027/research/derivative_free_causal_homotopy_20261027/evidence/STRUCTURAL_CONSTANTS_DERIVATION.md), [homotopy](https://github.com/cosmosapjw-quantum/vigilode/blob/research/derivative-free-causal-homotopy-20261027/research/derivative_free_causal_homotopy_20261027/homotopy/EXACT_CONSTANTS.md), [adapter](https://github.com/cosmosapjw-quantum/vigilode/blob/research/derivative-free-causal-homotopy-20261027/research/derivative_free_causal_homotopy_20261027/adapter/EXACT_ENTRY_WORK_POLICY.md)에 연결했다. 명령·소스·바이너리 hash, 실패 결과와 원자료 archive를 함께 보존했다.
