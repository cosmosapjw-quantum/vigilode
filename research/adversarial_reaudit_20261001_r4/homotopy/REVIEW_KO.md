# R4 homotopy 적대적 재감사 및 병렬 인증 연구

검토 production source: `1c54194123ee6abc6daa512e8574922f510b4e2c`.
사전등록·실행 checkout: `b2914f3e3c03eda60a8547619db6284aac8250a2`.
원 실행과 사전등록 파일의 SHA는 `EXECUTION_RECEIPT.json`에 있다.

업데이트는 이전의 선형화 진단과 엄밀한 bound를 실질적으로 분리했다. Native sequential target, quadratic nonlinear remainder, outward 연산, 구조별 inverse witness, q2의 8→7 W-batch 변경까지 구현했다. 새 기본 수학 구조는 유효한 witness와 선언된 모델이라는 전제하에 타당하다. 다만 공개 witness 입력의 검증 경계와 가변 stage 수의 doubling에서 재현 가능한 반례 두 건을 얻었다. production 코드는 수정하지 않았다.

## 1. 직접 실행 결과

native probe 1회는 exit 0, 9개 JSON 행을 냈다. 별도 Python Fraction 검산은 exit 0이다. 종료코드 0은 source 인증이 모두 옳다는 뜻이 아니다. 발견된 반례를 그대로 기록하는 실행기의 정상 종료다.

| 검사 | 실제 결과 | 판정 범위 |
|---|---|---|
| 정상 scalar inverse witness, native 8-stage | 첫 stage bound 0.06168294172747646가 정확한 유리수 오차를 포함 | 정상 생성자 control |
| 동일 identity, upper=[[0]] | 첫 stage bound 0, 실제 오차 >0 | 내용 검증 없이 선언을 신뢰 |
| 동일 identity, upper=[[]] | malformed shape를 거절하지 않고 동일한 0 bound | 공개 입력 shape 검증 실패 |
| chain s=1,2,4,8 | serial/doubling 둘 다 정확 오차 포함 | native s=8도 이 control을 통과 |
| chain s=9,16 | doubling output bound 둘 다 8; 실제 오차 각각 9,16 | 가변 stage API의 일반화 실패 |
| adaptive-depth Fraction candidate | s=1,2,4,8,9,16의 inverse identity 및 결과 모두 정확 | standalone exact arithmetic 후보 |
| diagonal component-factorization | n=1,2,4에서 full=blocked=serial 정확 일치 | 구조 분해 검증; wall time 미측정 |

Runtime 담당자의 같은 turn suite에서도 `coefficient_structural_contracts` 4 PASS/1 ignored, `outward_certificate_contracts` 4 PASS, `r3_certified_budget_contracts` 2 PASS, `q2_diagnostic_replacement_contracts` 4 PASS를 확인했다. 기존 정상 fixture 14개가 통과하는 사실과 새 API 반례가 존재하는 사실은 동시에 성립한다. 원 로그는 root `evidence/runtime/native/`에 있다.

## 2. R4-HOM-01 — 검증된 witness와 전달된 숫자 배열이 같은 타입이다 [P2]

`outward_certificate.rs:135–143`의 `InverseWitness`는 `upper`, `identity`, metadata가 모두 public이고 Deserialize도 가능하다. 그러나 `validate_inputs:435–467`은 identity와 upper의 바깥 길이만 검사한다. 내부 행 길이, 계산된 bound가 실제 inverse를 덮는지, 저장 내용이 올바른 constructor를 거쳤는지는 확인하지 않는다. `upper_dot:318–323`의 zip은 빈 행을 0으로 계산한다.

재현은 native sequential Rodas5P, J=-1, h=1/16, y=1, q=0, 여덟 개 zero candidate stages다. 첫 stage의 정확한 오차는

\[
|K_0^*|=\frac{1/16}{1+\gamma/16}
=\frac{140737488355328}{2281627374017361}>0.
\]

정상 diagonal witness를 clone하여 identity를 보존한 채 upper만 [[0]] 또는 [[]]로 바꾸면 `Ok(StageCertificate)`가 돌아오고 `stage_bound[0][0]=0`이다. output/embedded bound 역시 이 예에서는 0으로 나온다. 전자는 caller가 수학적 전제를 위반한 경우이고, 후자는 명백한 malformed shape다. 따라서 이 결과를 정상 constructor의 이론 오류로 해석해서는 안 된다. 현재 HOM-06 corpus는 정상 constructor를 사용하므로 이 probe만으로 그 campaign의 admission을 무효화할 수 없다.

구체적 수정은 `UnverifiedInverseWitness`와 private-field `VerifiedInverseWitness`의 분리다. Deserialized 데이터는 shape·비음수·유한값·operator identity를 확인한 뒤 구조 witness를 재생성하거나, approximate inverse V와 residual certificate로 bound를 재검증해야 한다. SHA가 일치한다는 사실은 operator identity를 묶지만 inverse inequality를 증명하지 않는다. `Q2CertificateSource::witness`의 사용자 override에서 이 값이 소비될 수 있으므로 안전한 API 경계를 먼저 닫아야 한다. 이 감사에서는 잘못된 witness를 넣은 full q2 step까지 실행하지 않았다.

## 3. R4-HOM-02 — 가변 s를 허용하면서 유한 경로합은 8단계로 고정 [P2]

`StageTarget`은 public vectors로 임의 s의 엄밀 하삼각 target을 구성할 수 있다. 입력 검사는 그 구조의 nilpotency를 확인하지만 `doubling_certificate`는 항상 level 0..3으로 (I+H⁴)(I+H²)(I+H)만 계산한다. 일반적으로는 H⁸=0이 필요하며, strict-lower s-stage 구조가 보장하는 것은 Hˢ=0이다.

원 반례는 gamma=0,J=1,h=1,y=1,q=0, alpha=0, coupling의 첫 subdiagonal=1이다. 그러면 K_i*=1+K_(i-1)*=i+1. b는 마지막 stage만 선택하고 candidate는 0이다. s=9와 16에서 serial bound는 각각 9와 16이며 정확하다. Doubling은 두 경우 모두 8을 반환한다. alpha=0이므로 radius=0 closure도 통과한다.

이는 기존 native 8-stage target의 오류가 아니다. 공개 generalized API가 지원하지 않는 s를 조용히 받아 sound bound처럼 돌려주는 문제다. Rodas5P 전용 API라면 s≤8을 검사하고, 일반화할 API라면 구조적 nilpotency 상계로 L=ceil(log2 s)를 선택하면 된다. 사전등록한 독립 Fraction candidate는 같은 여섯 s에서 유한 Neumann identity를 정확하게 만족했다. Rust outward production repair는 아직 하지 않았다.

## 4. 실제로 발전시킨 병렬화 방향

상세 유도는 `THEORY_KO.md`에 있다. 일반 dense (8n)×(8n) 경로합의 단계 깊이만 줄이는 전략은 matrix fill과 n³ 비용 때문에 실제로 느릴 수 있다. 이번에 직접 검산한 구체적 대안은 **diagonal J와 U에서 물리 성분별 8×8 인증 문제로 분해**하는 것이다.

Permutation으로 H가 n개 H_u의 direct sum이 되므로 결과는 정확하게 분리된다. n=4 toy에서는 matrix 저장값이 1024→256, dense matrix product 하나의 형식적 곱셈 수가 32768→2048로 바뀐다. 이 수치는 실제 native loop의 연산계수나 시간 측정이 아니며, algebraic dense-loop 비교다. 제공된 exact script는 full-matrix inverse action, component blocks, serial substitution의 일치를 검사한다. 후속 Rust 구현은 diagonal 구조를 검증된 enum으로 보존하고 outward 연산 순서를 유지해야 한다.

현재 q2 admission은 serial `certify_stage_target`를 호출한다. 연구용 `doubling_certificate`는 이 hot path에 통합되지 않았다. q1 fast path도 operational criterion이다. 따라서 arm 이름의 certified는 전체 적분이나 모든 fast acceptance를 인증한다는 뜻이 아니다.

동일 W-vector 비용 c_W라는 이상적 가정에서 P≥8일 때 전체 비용이 baseline보다 작아지려면

\[
(T_{cert}+T_{rhs}+T_{pool}+T_{other})/c_W<1+p_1-8p_f
\]

가 필요하다. q1=0,pf=0이라도 certificate와 추가 RHS, scheduling이 W solve 한 번보다 싸야 한다. persistent pool은 필요하지만 vector work 증가를 없애지는 않는다. coupled-linear-6처럼 기본 witness가 처음부터 unavailable인 문제는 speculative 7 batches를 수행한 뒤 fallback하는 대신, 구조 판정과 전략 선택을 앞당겨야 한다. 이 변경이 q1 기회를 포기하는 효과까지 별도 arm에서 평가해야 한다.

## 5. 게시된 HOM-06 수치가 말하는 것

다음은 이번에 새로 timing한 값이 아니라 저장소에 게시된 원 campaign을 읽은 결과다. 파일 identity와 요약을 `INHERITED_CAMPAIGN_SUMMARY.json`에 보존했다.

| workers | speedup point | 95% interval | verified decision |
|---:|---:|---:|---|
| 1 | 0.17350 | [0.13492, 0.18705] | Block |
| 2 | 0.07176 | [0.03783, 0.12939] | Block |
| 4 | 0.04815 | [0.02646, 0.10423] | Block |
| 8 | 0.03689 | [0.01974, 0.09574] | Block |

Accuracy와 per-step admission mismatch 조건은 그 corpus에서 충족했다. 그러나 wall-time acceleration은 실패했다. source는 step마다 thread pool을 만들며, 연구용 doubling은 matrix product마다 pool을 만든다. RHS/JVP 연산도 baseline의 여러 배다. 이 negative 결과가 loop를 중단할 이유는 아니지만, 단순히 worker 수를 늘리는 다음 실험은 근거가 약하다. 먼저 persistent pool, structured witness/majorant, unsupported-structure 사전 routing, deterministic vector batching을 구현해야 한다.

Campaign은 4-vCPU shared host에서 수행되었고 통계 authority hold가 따로 있다. 이 보고서는 그 값을 authoritatively 재측정했다고 주장하지 않는다. 최종 승격은 정확도와 전체 시도 비용을 함께 고정한 새로운 matched campaign으로 판단해야 한다.

## 6. 남는 명시적 한계와 다음 실행 순서

점wise RHS 및 한 방향 JVP의 일치는 generic ODE와 quadratic certificate model이 neighborhood에서 동일하다는 증명이 아니다. 코드도 이 제한을 공개했다. shared typed quadratic model은 구조상 binding을 제공하고, 일반화하려면 model-defect 및 remainder의 tube enclosure가 필요하다. Native target certificate 자체도 ODE local/global error bound가 아니다.

`certified_q2_budget`는 embedded lower E를 1024 eps로 아래에서 clamp한다. 실제 식은 min(absolute, fraction·max(E,1024eps)^(6/5))이며 문서의 E^(6/5)와 구별해 명시해야 한다. 이번에는 그 floor에 대한 full-step 반례를 탐색하지 않았다.

구체적 변경·dependency·acceptance·risk를 `NEXT_STEPS.json`의 7개 node로 제공했다. 실행 순서는 witness/shape 경계를 닫고, stage-depth 계약을 고정하고, persistent pool과 diagonal majorant를 구현한 뒤 unsupported-structure routing을 넣어 전체 비용을 다시 측정하는 것이다. 새로운 수학·코딩 후보의 owner 제안은 **조건부 채택 대기**이며, 최종 독립 결정은 root의 decision artifact에 따른다.
