# 최신 게시 근거와 별도 물리 연구의 재사용

기준은 `report/local-benchmark-stop-20261025`의 `d79326648a7273c0decd641eaf94d94f5bea7c63`이다. 이 커밋은 실제 최신 벤치마크 코드 `755885522e7e487e5f0d89085cfa15d972df0ea7`의 자식이다. 본 조사는 게시된 작은 결과와 선택된 source를 읽었으며 사용자 host의 대용량 raw나 현재 프로세스를 직접 읽지 않았다. 파일별 commit, Git blob, SHA-256 및 URL은 `LATEST_EVIDENCE.json`에 있다.

## 1. 이전 보고서 이후 실제 추가된 결과

게시된 중지 영수증은 **2026-10-25 21:32 KST에 실행 중지**를 기록한다. 당시 해당 측정 프로세스가 없었고 공유 잠금은 비어 있었다. 따라서 후속 인계의 출발점은 실행 중인 장기 캠페인의 제어가 아니라, 중지된 캠페인의 원자료 조사와 별도 source의 짧은 pilot이다.

- 기존 standard 1,539개 작업/1,540개 시도의 최종 상태는 변경하지 않았다. 전체 집계는 통과 323, 정확도 실패 146, solver 실패 57, 결과 없음 208, 참조 미검증 805다. 이 모수에는 계측 실행과 반복·스레드 조합이 들어간다. 앞선 보고서의 비계측 1,458개 집계와 혼합하면 안 된다.
- 새 Julia 비교는 cases 1/10/15/16 × Tsit5/Rodas5P의 8그룹, 48시도다. 각 그룹의 다섯 측정 구성원이 모두 최대 성분 endpoint 절대오차 + 경험적 참조 불확실도 ≤ 1e-4를 만족했다. Warmup과 별도 process 시간은 구분되어 있다.
- 독립 참조는 cases 1/2/3/5/8/9/20에 범위 한정하여 확보됐다. Case20의 경험적 불확실도는 `3.5435704834308433e-9`다. Case11은 참조 생성 실패이며, case4/12/13/14/17/18/21/23/24의 참조 작업은 미완료다.
- 정확도 V3는 빌드와 driver 계약시험 31개를 통과했으나 계획된 72시도의 **실행은 0회**다. Timeout 진단도 계획 9개, 실행 **0회**다. 후자는 runner의 no-receipt reconciliation lifetime bound가 미해결이었다. 따라서 새 자료가 timeout 원인을 확정했다는 주장은 성립하지 않는다.

| Case | 방법 | 최대 endpoint 절대오차 | 보고된 solve 중앙값 (s) |
|---:|---|---:|---:|
|1|Tsit5|1.676315421e-13|0.086546379|
|1|Rodas5P|2.629063633e-12|0.003063923|
|10|Tsit5|6.837251681e-9|0.008154926|
|10|Rodas5P|4.700907623e-7|0.071958250|
|15|Tsit5|4.919955288e-8|2.362260757|
|15|Rodas5P|1.473144984e-8|3.708116713|
|16|Tsit5|4.770855933e-11|0.001526153|
|16|Rodas5P|5.172116263e-9|0.002742115|

위 시간은 해당 패킷의 `diagnostic_only` 측정값이다. 서로 다른 달성오차·소스·실행 조건의 VigilODE 기록과 합쳐 새로운 속도 배수를 만들지 않는다. 특히 chirp에서 범용 RK와 Rosenbrock 모두 기존 정확도 기준에 도달한 사실은 새로운 구조적 adapter의 정확도 비교 기준으로 유용하다.

읽기 전용 exporter는 전체 RUN_ROOT의 2,565개 receipt 행 중 2,417개 raw 연결을 검증했다. 증거 이슈 124개, 형식 미지원/비종결 16개, 읽기/형식 오류 8개가 남았다. 이슈에는 Julia 등 미지원 출력 형식과 64 MiB 읽기 상한도 포함되므로, 곧바로 raw 소실이나 solver 실패로 해석하지 않는다. 기존 standard 1,539개 receipt는 전부 원래 바이트로 발견됐다. 전체 export는 여러 캠페인의 합이며 위 standard 집계와 다른 모수다.

원자료는 `/tmp/vigilode-local-run-20261024.geLVCp9Q`에 보존되어 있다. Git 패킷은 대용량 raw 백업이 아니다. 로컬 후속 작업은 원본을 보존한 채 사용자가 지정한 지속 저장 위치로 복사하고 크기·hash를 확인하는 것부터 시작하는 편이 좋다.

## 2. BASS/REI 별도 브랜치에서 가져올 계약

`research/program-expansion-20261025`의 `72836e0290b66d33f4ca1771284b1c8070669a74`는 PR85–88의 연구 소비자 결과를 계승한다. PR89 자체는 역할·계획 문서다. 별도 `research/bassrei-port-20261021`의 확인한 tip은 `555e71e03fe31e6c9b21961c8e2d263b1794eb6f`다. 두 계열의 결과를 general-purpose 코드에 통째로 merge하지 않고 다음 수학·인터페이스 조건을 재사용한다.

**물리 오차의 복원.** V1r의 기준은 full-state에서 `E² = sum((L e)_i / s_i)² / 33`, `s_i = atol_i + rtol_i max(|Yold_i|,|Ytrial_i|)`다. 고정 affine lift에 대해 `R L = I`, `F(Lz+b)=L f(z)`, `J_F L=L J_R` 및 shifted system 가역성이 성립하면 stage별 대응을 증명할 수 있다. 이 정리는 nonlinear 또는 time-dependent chart에는 그대로 적용되지 않는다. 새 chirp/cofactor adapter는 자신의 chain rule, 역변환, 물리 norm 증폭과 반올림 항을 별도로 연결해야 한다.

**정의역과 단계 거절.** PR85는 RHS 전에 모든 stage 입력을 검사하고 실패한 작업량도 계수한다. Restricted RHS에서 endpoint가 허용되어도 내부 stage는 허용되지 않을 수 있다. 기존 제조 모델에서는 guard+반감 1,032 attempts/5,323 RHS가 rate cap을 병용하면 602/4,816으로 줄었다. 이는 재사용하는 과거 작업량 증거이며 새 wall-time 측정이 아니다. 선형 scalar 전체구간의 Bernstein 증명과 비선형 frozen-rate heuristic을 구분한다. 새 adapter의 phase/chart 평가와 선택적 차분 stencil도 정의역 검사를 공유해야 한다.

**연속 출력.** PR86은 실제 저장된 다항식의 원래 벡터장 defect, 이전 구간의 오차 상속, 물리 norm과 query 반올림을 연결했다. Endpoint가 정확한 carrier reconstruction도 dense curve 전체의 정확도를 자동 보증하지 않는다. 회전 phase와 state lifting을 포함한 실제 반환 곡선에 맞는 별도 defect/오차 계산이 필요하다. 기존 Riccati 수축 정리는 임의 진동계의 인증으로 확대하지 않는다.

**실험적 경로의 외부 효과.** PR87의 D4는 rejected trial의 owned journal을 폐기하고 accepted trial만 동일 batch identity와 ACK barrier 아래 전달한다. 따라서 method shadow trial이나 thread calibration은 callback이 재실행 가능한 경우에만 한다. `Fn + Sync`는 외부 효과의 부재를 증명하지 않는다. 일반 capture·파일·메시지는 journal이 되돌리지 않으므로 사용자가 제공한 replay-safe 계약과 명시적인 비용 계수가 필요하다.

**Callback 경계.** PR88은 NaN scratch의 full-write, finite 값, model epoch와 환경 witness를 검사한다. Finite Jacobian이 올바른 도함수임을 보장하지 않으며 실제 `J+I` 음성 대조가 이 차이를 보여준다. 기존 fixed-sparsity incremental-write API를 full-write로 오해해 자동 변경하지 않는다. 동일 source epoch가 바뀌면 method 및 thread calibration의 유효성도 다시 판정해야 한다.

**구조를 가진 격자 문제.** BASS/REI P06은 전구면 삼축 수송의 상태 layout, 격자 기반 bandwidth와 rank-6 Thomson 인자를 제공한다. 이는 general selector가 `dimension` 하나만 보지 말고 grid shape, band/low-rank 구조, callback epoch, 보존량·domain 계약을 받을 이유다. 축대칭이나 rank 퇴화 조건을 만족할 때만 단순화한다. 이 과학적 선언을 검증 없는 자동 물리 모델 추론으로 바꾸지 않는다.

## 3. 새 benchmark가 답해야 할 질문

각 실행은 원래 상태와 허용오차를 유지하면서 선택된 방법·스레드 수, 선택 이유, 사용한 관측량, 초기·제안 스텝, fallback·shadow 비용, 실제 물리 오차, 거절 원인을 함께 기록한다. 대표적인 동일 시작점 구간에서만 대안 경로를 비교하고, 대안이 더 좋은 경우를 다음과 같이 분해한다.

1. **관측 부족:** 필요한 구조/주파수/작업량 정보가 없었는가.
2. **판정 오류:** 정보는 있었지만 분류·손익 threshold가 잘못되었는가.
3. **상태 이월 오류:** 올바른 방법으로 전환해도 h/history/cache를 잃었는가.
4. **실행 비용 오류:** 병렬 준비·동기화·검증·콜백 비용을 빠뜨렸는가.
5. **비교 계약 차이:** 더 빠른 결과가 다른 실제 정확도·출력·물리 norm을 사용했는가.

작은 h만으로 stiff라고 부르거나, 높은 수락률만으로 적응기가 정상이라고 결론짓지 않는다. 새로운 선택 정책의 성공 여부는 대표 조건과 미사용 검증 조건에서 정확도를 유지한 총 비용·실패·지연으로 평가한다. 새 코드의 결과에는 새 source/binary identity를 부여하고 과거 행을 소급 승인하지 않는다.

## 검산 범위

`python3 -B research/local_frontier_native_diffsol_20261024/stop_audit_20261025/verify_packet.py`를 실행해 exit 0, `PASS_PACKET_SELF_CHECK`를 확인했다. 작은 74파일, 191개 비용 이벤트, Julia48시도 집계, case20의 전체 벡터 간 차이를 재계산하는 저장 자료 검사다. 원래 ODE 실행이나 사용자 host raw 전체 검증은 재실행하지 않았다.

PhysMath Research Loop1.3.0의 R2/R3/C2에 해당하는 조사·전제 정렬·source 연결을 수행했다. 고정 source의 과학 결과와 현재 pilot의 검증 상태는 별도로 둔다.
