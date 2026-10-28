# Nonstiff 외부 비교 완료: diffsol/Julia 로컬 어댑터와 감사 자료

사용자가 승인한 호환 어댑터 설치·개발과 빠진 외부 비교만 수행했다. Native solver,
기본 dispatcher, 공개 API 지원 범위 및 root Cargo.toml/Cargo.lock은 변경하지 않았다.
기존25개 계약 시험,576개 native/VODE process record,16개 ensemble record는 재실행 없이
원래 해시와 source identity로 재사용했다. 과거 보고서의 NOT_RUN은 당시 사실로 보존되며,
이번 별도 외부 cohort가 그 누락을 보완한다.

## 실제 실행 identity

- Native/problem source: `45037fc24e8f8101605f309e1fdb3a9a4e3df56d`
- Native tree: `b43b69b1ec5b86f3577d7cd6efdefe2d3acb9bad`
- Adapter/runner 실행 source: `1cf87238ab79ee698621d80bc0a3afd903d2f6ff`
- Adapter/runner tree: `13396d7d47e3ecc06af4cd5b7a7fd01ee1be3cee`
- Diffsol measurement binary SHA256: `798afa7b02c8db49312333a122cc90f9cf2b72c2f48baad8ec674a4e5cffb21a`
- Native binary SHA256: `50534f6dc5da3c7c123e74c83c2d6d437474a94282decd274711534292586ac1`
- Julia executable SHA256: `582718c20c563d824b88a08e2a9d0afcf5015b3053f425697cecc79ec4e9ea36`

후속 보고서 HEAD는 실행 소스가 아니다. 실제 명령·exit code·input/output hash는
EXTERNAL_ADAPTERS.json, evidence/frozen-execution/COMMANDS.json, EXTERNAL_EXECUTION.json,
각 PLAN/RECEIPTS/RAW_MANIFEST와 AUDIT_MANIFEST.json에 있다. 바이너리·depot cache는
Git에 넣지 않았으며, 고정 commit·lock·runtime hash로 재빌드한다.

## 구현과 시험

Diffsol0.16.1/faer standalone workspace는 비강성 Tsit45, 두12차원 강성 대조군은
BDF+analytic JVP/FaerLU를 사용한다. Julia1.12.7의 Tsit5/2.1.5와 Rodas5P/2.7.5는
exact Project/Manifest로 고정했다. Rodas5P는 analytic state Jacobian과 package 기본
linear solver/AD 설정을 사용한다. 두 explicit 경로는 대규모 dense Jacobian을 만들지 않는다.
외부 관측은 한 adaptive history의 보간/saveat이며 native의 관측점 step clipping과 다르다.

Diffsol 최종19개 프로토콜 검사와 strict preflight32개(24 positive case/mode쌍+
8 부정 대조)가 통과했다. Julia 수정 검사는 정상12개·예상 max-step 실패 대조3개·잘못된 요청9개와
모든 RHS/초기값/기준해 및4096/16384 shape를 다뤘다. 정상12개는 수정 전후 상태
비트·시간·작업량이 같았다. 이것은 전체 저장소·모든 production 문제군의 검증이 아니다.
외부 heap allocation은 미계측(null)이며 native allocation 시험을 외부로 확장하지 않는다.

독립 리뷰에서 엄격 검증의 비용 자격 누락, 빈 실패 출력의 통과, Julia rejected trial의
accepted-state 오표기, mode/dimension 거절 누락을 발견하여 한 제한 수정 회차로 고쳤다.
최초 Julia MaxIters 증거는 t=0의 trial 벡터가 초기값과1.0859190353467283만큼 달랐으며
DEVELOPMENT_EVIDENCE.tar.gz에 보존했다. 이제 accepted step 관찰 callback이 preallocated
snapshot을 복사한다. 상태 변경·추가 관측점은 없으며 그 setup/copy 비용은 timed interval에
포함한다. strict-invalid 데이터는 원본 그대로 보존하고 파생 분석에서 비용 자격을 제외한다.

## 측정 결과와 정확도

새 캠페인:96개 설정·384회 계획, 실제 receipt 384개,
완료 process 384개, 기타 process 결과0개.
하네스 관측 시간2083.633962s;7200s 누적 예산과45s/process 한도를 유지했다.
사전 설치·빌드·preflight는 이 solver 캠페인 예산 밖이며 전체 작업이2시간 이내라는 주장은 하지 않는다.
기존144설정과 새96설정의 병합은240설정이며 replay 결과는 `{"parsed_records": 960, "successful_records": 960, "valid_replays": 960, "invalid_replays": 0, "max_reference_disagreement": 1.1102230246251565e-16}`다.
수치적 성공은 exit0과 분리해 저장 상태·기준해·오차 요약·source/method/시간격자/초기값을 재검증했다.

sampled normalized error는 저장된5개 시각·성분에서 `abs(y-reference) / max(1,abs(initial),abs(reference))`의 최댓값이다.
아래 최선 오차는 등록된 네 rtol 중 strict-valid 성공 구성의 최소 sampled normalized error다.
비용비는 공통 오차 상한1e-6/1e-8 아래 각 방법의 가장 빠른 적격 구성을 선택한
`comparator median seconds / DOPRI5 median seconds`다.1보다 크면 그 표본에서 DOPRI5가
빠르다. 동일 상한 충족은 동일 실제 오차를 뜻하지 않는다. N/A를 임의 수치로 채우지 않았다.

|문제·차원|DOPRI5 최선 오차|diffsol 최선 오차|Julia 최선 오차|diffsol/DP 1e-6|diffsol/DP 1e-8|Julia/DP 1e-6|Julia/DP 1e-8|
|---|---:|---:|---:|---:|---:|---:|---:|
|decay · 8|2.227e-11|3.089e-12|2.772e-13|28.24|28.24|3.978|4.911|
|driven · 8|4.672e-10|2.781e-10|6.938e-11|18.42|14.5|3.667|4.327|
|logistic · 8|2.802e-10|3.307e-10|1.361e-10|25.91|19.86|4.097|3.033|
|harmonic · 8|3.025e-09|8.948e-10|2.632e-10|11.12|7.961|2.283|1.939|
|rapid · 8|1.972e-08|5.873e-09|1.754e-09|1.29|N/A|0.7337|N/A|
|damped · 8|1.497e-10|4.178e-11|1.255e-11|7.329|7.232|1.766|1.549|
|mixed-scales · 12|9.442e-11|2.758e-11|7.51e-13|14.25|8.883|2.983|1.86|
|nonnormal · 8|1.69e-10|6.208e-11|1.49e-11|21.94|15.31|4.282|3.123|
|large-vector · 4096|1.9e-11|7.737e-12|3.722e-12|1.261|1.009|0.2341|0.1753|
|large-vector · 16384|1.9e-11|7.737e-12|3.722e-12|2.433|3.128|0.2052|0.1886|
|stiff-control · 12|6.278e-13|2.036e-09|2.706e-11|0.08478|0.1029|0.01725|0.03786|
|moderate-stiff · 12|2.014e-11|1.301e-09|1.696e-11|9.318|10.07|2.098|2.51|

![전체 work–precision 진단](evidence/merged-comparison/WORK_PRECISION.png)

새 외부 비교48개 중 적격 비교46개, N/A2개이며,
적격 비교 중 DOPRI5 비용 손실(비용비<1)은9개다. 손실·미달·timeout·solver failure를
RESULTS.json과 원시 기록에서 삭제하지 않았다. Native dense/matrix-free 및 VODE 비교까지
포함한 전체 수치와 선택 구성 ID는 evidence/merged-comparison/SUMMARY.json에 있다.

새 비교에서 Julia는 rapid의1e-6 상한과 두 대규모 vector의 두 상한에서 DOPRI5보다
빠르게 관측됐다. stiff-control에서는 diffsol BDF와 Julia Rodas5P 모두 DOPRI5보다 빨랐다.
이9개 비용 손실은 같은 상한을 만족한 구성의 진단 비교이며 실제 오차·실행 cohort는 다르다.

기존 baseline의 반례도 그대로 유지한다. VODE 최선 sampled normalized error는
rapid8.231704e-8, harmonic1.284843e-8, stiff-control1.659438e-6로 각각
tight1e-8,1e-8,1e-6/1e-8 상한에 미달했다. DOPRI5 rapid도1.971506e-8로1e-8에 미달하지만,
새 diffsol/Julia는 이 상한에 도달했다. 따라서 rapid의 공통 비교2개는 N/A이며
외부 방법의 실패를 뜻하지 않는다. Native RODAS5P에 비한 stiff DOPRI5 비용 증가와
16384 VODE 비용 cliff도 원래 보고서와 전체 merged SUMMARY에 보존했다.

## 적용 범위와 남은 일

- **decay · 8**: forward identity-mass 선형 감쇠 표본. 일반 비선형 문제군의 보증으로 확장하지 않는다. 최선 오차/비용 이득·손실은 위 표와 machine-readable family 행을 따른다. 다음: 동일 paired 환경에서 setup/callback 비용과 수락 step 수를 분리 측정한다.

- **driven · 8**: 시간 의존 forcing 표본. Native 관측점은 실제 step clipping, 외부는 보간이다. 최선 오차/비용 이득·손실은 위 표와 machine-readable family 행을 따른다. 다음: 비자율 forcing 평가와 출력 보간의 비용·오차 기여를 분리한다.

- **logistic · 8**: 제한된 비선형 logistic 표본. 임의 비선형성 또는 양성 보존 보증이 아니다. 최선 오차/비용 이득·손실은 위 표와 machine-readable family 행을 따른다. 다음: 비선형 상태 범위를 넓히는 별도 등록 검증을 준비한다.

- **harmonic · 8**: 유한 주파수·시간의 조화진동 표본. 관측점 오차가 위상오차의 모든 시간 상계는 아니다. 최선 오차/비용 이득·손실은 위 표와 machine-readable family 행을 따른다. 다음: 장시간 위상·진폭 오차를 분리하고 관측점 밖 검증을 별도 등록한다.

- **rapid · 8**: 100–115 주파수 진동. tight cap에 대한 미달을 최선 achieved-error와 함께 그대로 남긴다. 최선 오차/비용 이득·손실은 위 표와 machine-readable family 행을 따른다. 다음: tight cap 미달의 위상오차/안정성 비용을 진단하고 필요 시 새 허용오차 grid를 사전등록한다.

- **damped · 8**: 감쇠 진동 표본. 작은 진폭의 상대오차는 이 normalized metric만으로 보증되지 않는다. 최선 오차/비용 이득·손실은 위 표와 machine-readable family 행을 따른다. 다음: 작은 진폭 성분의 상대오차와 감쇠 envelope를 별도 검증한다.

- **mixed-scales · 12**: 초기 진폭1e-8–1e3, scalar atol 캠페인. componentwise 상대오차 인증은 아니다. 최선 오차/비용 이득·손실은 위 표와 machine-readable family 행을 따른다. 다음: scalar atol 결과와 별도로 vector atol·성분별 오차를 검증한다.

- **nonnormal · 8**: 8차원 K=30 transient 표본. 일반 비정규 행렬·pseudospectrum 검증이 아니다. 최선 오차/비용 이득·손실은 위 표와 machine-readable family 행을 따른다. 다음: transient 증폭량과 성분별 오차를 분리하고 K/시간범위를 별도 등록한다.

- **large-vector · 4096**: 반복되는8개 대각 감쇠율의4096/16384 상태. 일반 대규모 결합 문제의 복잡도 근거로 확대하지 않는다. 최선 오차/비용 이득·손실은 위 표와 machine-readable family 행을 따른다. 다음: accepted snapshot·보간·메모리 복사와 BLAS/thread 비용을 paired 환경에서 분리한다.

- **large-vector · 16384**: 반복되는8개 대각 감쇠율의4096/16384 상태. 일반 대규모 결합 문제의 복잡도 근거로 확대하지 않는다. 최선 오차/비용 이득·손실은 위 표와 machine-readable family 행을 따른다. 다음: accepted snapshot·보간·메모리 복사와 BLAS/thread 비용을 paired 환경에서 분리한다.

- **stiff-control · 12**: 1/1000/100000 감쇠율의12차원 negative control. DOPRI5 비용 증가를 그대로 보고한다. 최선 오차/비용 이득·손실은 위 표와 machine-readable family 행을 따른다. 다음: 명시적 방법의 비용 손실을 유지하고 수락/거절·선형해법 비용을 별도 진단한다.

- **moderate-stiff · 12**: 12차원 대각 negative control. explicit 자동 stiffness 탐지·switching은 없다. 최선 오차/비용 이득·손실은 위 표와 machine-readable family 행을 따른다. 다음: 명시적 안정성 제한으로 인한 RHS/거절 비용을 stiff 경로와 대조한다.

후속 개선은 rapid의 tight sampled-error 미달·강성 negative control의 비용 증가,
대규모 표본의 adapter/BLAS/메모리 비용 차이를 각각 좁게 조사하는 것이다. 현재 측정 결과에
맞춰 rtol grid·방법·오차 임계값을 바꾸거나 캠페인을 재실행하지 않았다. 통계적 성능 주장에는
동일 시간대의 paired 설계와 추가 반복이 필요하다.16384 VODE 비용 급증 원인은
NOT_MEASURED이며 이 cliff를 복잡도나 보편적 가속비로 해석하지 않는다.

Dopri5는 사용자가 직접 선택하는 forward identity-mass ODE 경로다. 비자율 RHS와
scalar/vector atol을 지원하지만 이번 캠페인은 scalar atol이다. Jacobian/JVP/dfdt,
factorization/Newton solve를 호출하지 않는다. mass matrix 선언/event/dense output은
RHS 전에 거절한다. 관측시각에는 실제 step을 잘라 도달하며 자동 stiffness 탐지나
method switching은 없다. Jacobian 부재가 nonstiff 판정 근거는 아니다. 한 solve의 RHS
수학적 모델은 FSAL을 위해 동일해야 한다. partial solution은 success=false와 마지막
accepted 상태를 보존하고 호출자는 ensure_success() 또는 명시적 success 검사를 해야 한다.

기존 ensemble 결과는16개 독립4096차원 궤적의1/2worker throughput이며 각 궤적의
시간격자·오차 제어는 독립적이다. worker 사이 상태·작업량 일치를 보존했다. RK stage,
단일 궤적 또는 homotopy 가속 증거가 아니다.

## 시간 해석과 환경

CPU `AMD Ryzen 9 5900X 12-Core Processor`, affinity0–23, 실제 cgroup CPU quota/memory limit는 max다.
Python3.12.3/NumPy2.4.2/Matplotlib3.10.8/SciPy1.17.0.
Rust/Cargo1.94.1 measurement profile은 opt-level3/LTO=false/codegen-units16이다.
공유 host의 load·quota·메모리·runtime/sysimage hash는 CAMPAIGN_ENVIRONMENT.json에 있다.
주기적으로 관측한1분 load 범위는12.504–25.037이며 연속 관측의 엄밀한 최솟값/최댓값은 아니다.
Julia는1thread/OpenBLAS1이며 기존 VODE 환경의 NumPy OpenBLAS24 관측을 바꾸지 않았다.

Julia는 매 process에서 동일 요청의 method/type/shape 준비 solve1회를 먼저 수행하고,
새 상태·문제·작업공간·counter로 측정한다. 이 JIT 준비는 campaign warmup과 별개다.
imports/준비/JSON을 포함한 process wall은45s 한도 안에 있고 solver seconds와 별도로
RESULTS.json에 min/median/max를 기록했다. 준비 시간은 solver timer 밖이다. 측정에는
문제/초기값 구성·callback·거절/실패 작업·저장 상태 수집·accepted snapshot 복사가 포함되고,
기준해 평가·error scoring·JSON serialization은 제외된다. VODE Python callback overhead는
포함하며 Adams/BDF Fortran VODE 비교이지 RK45/Radau/LSODA 대체 비교가 아니다.

세 timed 반복의 min/max는 신뢰구간이 아니다. 모든 wall-time은 diagnostic_only다.
Native/VODE와 새 외부 어댑터는 다른 시간대 cohort이므로 비용비는 탐색적 진단이다.
4개 rtol grid의 최선 후보는 전역 최적 비용을 뜻하지 않는다. rtol은 전역오차가 아니고,
endpoint/5개 관측점 오차는 모든 시간의 엄밀한 보증이 아니다. ConditionalL2나 interval/global
certificate의 승격도 수행하지 않았다. 범용 production·통계적 timing authority는 HOLD다.

## 감사 자료

원래 checkout과45037fc2 detached source의 HEAD/tree/status 최종 확인은 FINAL_CHECKS.json에
있다. NCP/SSH/reset/clean/강제 checkout은 사용하지 않았다. 기존 결과는 덮어쓰지 않았다.
개발 첫 실패, 설치·lock·targeted 시험은 DEVELOPMENT_EVIDENCE.tar.gz/DEVELOPMENT_MANIFEST.json,
외부 및 병합 원시 출력은 각 RAW_OUTPUTS.tar.gz/RAW_MANIFEST.json,
PLAN/RECEIPTS/SUMMARY/REPLAY와 PNG/SVG/PDF 도표는 evidence 아래 보존했다.
BASELINE_MAPPING/SOURCE_CAMPAIGNS는 원래 ID·cohort·path·hash를 추적한다.
최종 독립 판정은 REVIEW.json을 따른다. 외부 감사용 공개는 범용 production 승격이 아니다.
