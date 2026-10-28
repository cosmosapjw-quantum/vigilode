# Production frontier 후속 검증: 제한된 범위 판정

2026-10-23 관측. 공개 시작점 `cff15d6da06effe494664f1ef50e20d47d3ad619`의 tree는 `2eb2691d0eab29fde84b6c5e88c0422ebf14c38e`이고, 요구된 native/PDE 조상 두 개와 공개 패키지 검증을 확인했다. 원래 checkout은 변경하지 않았다. 구현 소스는 `49696c982446dfe5d274f5361819b218122b7449`, LP01 추가 순환 실행 스크립트는 `dbcaeee2256edbfd45a1c9177c23c567c92c592a`에 고정했다. 전체 19개 노드의 현재 상태와 차단 이유는 `TASK_STATUS.json`에 있다.

## 바뀐 사실

- FA01: 보호된 scalar native 경로에서 fatal callback을 한 번만 호출하고, 이후 물리 trial을 멈추면서 마지막 수락 상태와 시도 비용을 남긴다. 범위별 계약 시험을 통과했다. row-batch와 임의 custom operator의 부분 작업 기록은 아직 검증하지 않았다.
- SW02: DOPRI5 endpoint/등록 landing과 RODAS-fast dense Jacobian·identity mass·NoSink의 지속 세션을 추가했다. 동일 schedule의 상태·카운터·controller 계약 시험 10개와 terminal 단위 시험 1개가 통과했다. facade, switching, event, 다른 mass/matrix-free 경로의 연결은 남았다.
- GE01: 시간·공간·flow/root·remap·roundoff를 분리하는 예산 소비자와 accepted state/time의 단일 transaction을 추가했다. 집중 시험 12개와 doc 시험 14개를 통과했다. native driver의 실제 후보 연결과 물리 bound 근거는 남았다.
- QA01: 두 상태 homogeneous collision의 typed witness와 원래 공간 잔차 검사를 구현했다. 새 시험 8개와 기존 polynomial 시험 8개가 통과했다. 일반 격자 연산자나 RODAS/causal stage 연결 주장은 하지 않는다.
- LP01: 100/1000 cycles, 4097개 관측의 세 homogeneous linear 2D 사례에서 tight tolerance DOPRI/Lawson 추가 순환 측정 36/36행이 상태 `1e-5`, 위상 `1e-3` rad, 상대 진폭 `1e-5` 표본 상한을 통과했다. 느슨한 DOPRI의 첫 실행 warmup 3개는 상태·진폭 상한에 실패했고 후속 15행은 실행되지 않았다. 모두 원시 기록으로 남겼다.

## 시간과 범위

첫 LP01 실행은 method별 연속 순서였으므로 정확도 pilot으로만 사용한다. 추가 실행은 여섯 경로를 라운드마다 순환했다. 추가 실행의 solver-only 표본은 경로에 따라 최대/최소가 약 1.09~3.65배로 흔들렸고, 프로세스 전체 시간은 짧은 solve보다 시작·감시 비용의 영향을 크게 받았다. 첫 실행과 추가 실행의 시간을 합쳐 속도 배수를 계산하지 않는다. `LP01_CYCLIC_ANALYSIS.json`의 중앙값과 분산은 `diagnostic_only`이며 **범위별 속도 개선 판정은 보류**한다. 기준 해의 80/100자리 차이와 scoring allowance를 함께 검사했다. 관측 사이의 추가 회전은 이 표본 시험으로 배제할 수 없다.

기존 switching, causal, affine, PDE 증명과 캠페인은 `REUSED_EVIDENCE.json`에 기록한 범위만 재사용했다. 실제 BASS/rei 물리 client, 온라인 terminal event, 일반 비선형 PDE, VODE/DVODE·diffsol·Julia 비교와 18문제군 PB01 qualification은 실행하지 않았다. CP01 작은 문제군의 2-thread 이득 부족과 RKL2 양성 위반은 기존 근거를 유지한다.

**판정: broad production HOLD.** 안전 계약의 일부와 특화 LP01 표본 정확도는 통과했으나 필수 세션 연결, 사건·재시작, 물리 오차, 외부 비교가 닫히지 않았다. 다음 시작점은 SW02 facade/switching 연결과 GE01 native accepted-candidate 연결이다. 원시 명령·출력·실패·환경·hash는 `REPRODUCE.md`, `receipts/`, `results/`, `HASH_MANIFEST.json`에 있다.
