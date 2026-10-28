# Adapter cold-seed 교정과 선택적 JVP의 실제 비용 대조

## 후속 exact 정책

이 문서는 seed 교정과 당시 floating parity control의 역사적 결과를
보존한다. 사용자의 추가 지시에 따라 현재 구현은
`strict_entry_work_improvement`의 checked 정수 비교로 확장했다.
현재 판정식·증명·18계약·동일12arm 재실행은
`EXACT_ENTRY_WORK_POLICY.md` 및 `EXACT_WORK_*` 파일을 참조한다.
현재 strict 경로는 `nextdown(1)`을 사용하지 않는다.

## 결과

실제 같은-checkpoint entry pilot이 매번 전역 `Options.h0`로 재시작하는
문제를 교정했다. 새 `seed_entry_pilot_from_explicit_checkpoint`는 **기본 false**이며,
이미 관측한 해당 explicit checkpoint의 시작 proposal만 재사용한다.
수치 허용오차, ODE, RODAS/DOPRI 식, native error checks 및 전체 budget은
유지했다.

기존 entry ratio=.8을 유지하면 cold-ramp 비용은 감소하지만, 완료된
implicit pilot의 work160 / explicit work192 = 5/6은 기존20% margin을
만족하지 못한다. 이를 실패로 숨기거나 임계값을 몰래 바꾸지 않았다.

이 진단 뒤 하나의 명시적 policy control을 등록했다. 기존 validator의
`0<rho<1`을 유지하면서 `rho=nextdown(1)=0.9999999999999999`를 사용했다.
이 실험의 작은 정수 작업량에서는 strict cost parity인 `C_JVP<C_DF`를
시험한다. 이 control은 같은 checkpoint에서 실제 완료·검증된 JVP pilot이
더 저렴한 때에만 implicit 경로에 들어가며, 이후 전체 궤적의 비용 감소도
별도로 측정했다. production default rho=.8은 그대로다.

| rate | RHS-only 총 work | 원래 selective | 시드 교정, rho=.8 | 시드+strict parity |
|---:|---:|---:|---:|---:|
| 1 | 79 | 79 | 79 | 79 |
| 1000 | 2377 | 2958 | 2890 | **1691** |
| 10000 | 19261 | 20190 | 20094 | **1629** |

마지막 arm의 RHS/JVP는 rate1에서79/0, rate1000에서1105/426,
rate10000에서992/445였다. RHS-only 대비 총 deterministic work proxy는
rate1000에서 **28.86%**, rate10000에서 **91.54%** 감소했다. 첫 번째 losing
pilot과 return probe까지 모두 포함한 값이다. wall-clock timing은 수행하지
않았으며 이 proxy 비율을 시간 가속률로 사용하지 않는다.

## 진단과 최소 교정

원래 six-arm 결과 `RESULTS.json`에서 rate1000, t=.125의 explicit
checkpoint는 이미 h=.003662256577412597을 제안하고 있었다. 그러나 같은
시작점에서 비교하는 MF entry pilot은 h0=.0001로 재시작했고, target
.22500838873374865까지 가지 못한 채 .18883370103055072에서 비용 한도에
도달했다. h0에 checkpoint proposal을 쓰면 같은 target을 work160으로
완료한다. rate10000의 이후 checkpoint들에서도 같은 기작을 확인했다.

기존 `reuse_entry_pilot_step`은 **winning pilot 이후** productive implicit
run의 시드만 바꾸므로 이 cold-start 문제를 해결하지 못했다. 새 옵션은
실제로 pilot이 실행되는 entry callsite에만 적용한다. 첫 transient pilot의
시드는 그대로이며 원래 비싼 초기 비교도 보존한다.

## 수학적·코드 계약

현재 checkpoint를 `(t,y)`, 이미 받은 explicit 시작 proposal을 `h_c`,
기본 proposal을 `h_0>0`, replay 끝까지의 거리를 `H>0`이라 하자.
유효한 `h_c`에 대해 먼저

\[
\widetilde h_0=\min\{H,\max(h_0,h_c)\}
\]

를 사용한다. 구현은 기존 native 최소-step proposal floor도 유지하므로
실제 설정값은 `max(tilde_h0,min_step)`이며, H가 floor보다 짧은 tail에서는
native driver가 실제 trial을 endpoint에 맞춘다. 비유한·비양수의 기억값은
사용하지 않는다.

**인과성.** `run_persistent_explicit`은 `advance_to`를 호출하기 전에
`session.checkpoint().proposed_h`를 읽어 `record.initial_step`에 기록한다.
새 helper는 이 필드만 읽고, 완료한 prefix 뒤에서 측정한
`proposed_next_step`은 읽지 않는다. 따라서 재생 대상 시작점보다 미래의
proposal을 시드 근거로 사용하지 않는다.

**수치 계약 보존.** 바뀌는 값은 private pilot Options clone의 h0뿐이다.
RHS, 원래 적분식/tableau, rtol/atol, error norm, acceptance 및 rejection
함수는 같은 코드다. step sequence는 달라질 수 있으며 이는 더 나은
proposal을 시험하는 목적이다. 같은 시작 state와 replay endpoint에 대한
완료 후보만 원래 qualification/cost gate를 통과할 수 있다. 작은 residual의
전역 물리 오차 인증이나 모든 모델의 비용 감소를 추가로 가정하지 않는다.

**비용 계약 보존.** pilot의 모든 시도·RHS·JVP·linear solve는 기존
`charge`에 들어간다. global remaining attempts와 pilot attempt cap을
그대로 사용한다. 옵션은 derivative permission을 추가하지 않는다.
strict-parity control은 기존 정책 parameter 하나를 명시적으로 설정하며,
그에 따라 callback ceiling도 커진 효과를 결과에 포함한다.

## 실제 same-window evidence

rho=.8의 시드 교정에서 모든 late pilot이 완료되어도 work ratio5/6이므로
`ImplicitNotCheaperOnPrefix`가 유지된다. 이 enum은 정책 margin 충족 여부를
뜻한다. 숫자상 work160이192보다 작은 사실과20% margin을 못 채우는
정책 판단을 구별한다.

strict-parity control에서는 다음 실제 비교가 entry를 지지했다.

| rate | 같은 시작 checkpoint | 동일 replay target | JVP work / explicit work |
|---:|---:|---:|---:|
| 1000 | .125 | .22500838873374865 | 160 / 192 |
| 10000 | .005852350232369075 | .015426870629347594 | 160 / 192 |

첫 transient losing pilot은 rho=.8에서193work, parity control에서241work를
소비했고 모두 총계에 들어갔다. 현재 window의 우세는 미래 모든 window의
우세 정리가 아니다. 위 전체 궤적 표는 이후의 실제 실행 결과다. rate1의
쉬운 대조군은 네 arm 모두 JVP0을 유지했다.

## 정확도와 검증

제조 문제는 모든 arm에서 `y_i'=-rate*y_i`, n16, y_i(0)=1, [0,1],
rate1/1000/10000이다. 8window, rtol1e-7, atol1e-10, h0=1e-4,
max50000attempts를 고정했다. 모든 반환 시각의 모든 성분을 정확해
exp(-rate*t)와 비교했다. 원래 절대오차 cap1e-6을 12/12 arm이 통과했다.

parity control의 최대 오차는 rate1에서3.37183e-9,
rate1000에서1.06541e-9, rate10000에서1.31463e-9이다. 이는 실제 반환
grid의 검증이다. 모든 arm의 atomics와 native RHS/JVP 총계가 같으며
Jacobian build와 explicit time-derivative 호출은0이다.

새 native 계약4개, 기존 evidence 계약7개, private helper 계약1개,
총 **12 PASS**를 실행했다. default/factory의 독립 opt-in, 각 pilot의
같은 checkpoint/endpoint 시드, 미래 proposal 독립성, 잘못된 proposal,
원래 accuracy/budget/full accounting, Forbidden permission을 확인했다.

기존 six-arm 결과의 모든 기존 row field가 nine-arm 실행에서 그대로
재현됐다. 그 nine-arm의 모든 field가 final twelve-arm에서도 그대로
재현됐다. rho=.8 시드 on/off의 모든 반환 시각/state도 동일하다. winning
route가 생긴 parity control에는 동일한 물리 오차 cap으로 결과를 비교한다.

## 연구 루프와 재현 근거

| 단계 | 완료한 작업 |
|---|---|
| R1/C1 | 기존 adapter 결과와 h0/cap/모델 계약을 유지했다. |
| R2/C2 | prior B3 및 실제 entry callsite, checkpoint.initial_step 캡처 시점을 읽었다. |
| R3/C3 | 시작 proposal과 미래 proposed_next_step을 구별하고 최소 h0 변경을 설계했다. |
| R4/C4 | default-false seed 대조와 compile lock을 정하고 실행 전 protocol을 썼다. |
| R5/C5 | 인과적 시드 식·동일 수치 gate 계약을 유도하고 실제 helper/옵션/callsite를 구현했다. |
| R6/C6 | 별도 theory agent가 clipping·finite/sign·cost accounting을 검토했고,12계약을 실행했다. |
| R7/C7 | 원래6arm, 시드9arm, 단한번의 후속 parity12arm을 실제 비교했다. |
| R8/C8 | cold-ramp와20%margin 기작을 구분하고 모든 실패·추가 probe 비용을 보존했다. |
| R9/C9 | 시작점 인과성·local qualification·미래 성능의 경험적 상태를 보고서와 API에 기록했다. |
| R10/C10 | source/실행/원자료/교정 기록을 보존하고 parameter 탐색을 종료했다. |

`CHECKPOINT_SEED_PROTOCOL.md`는9arm 실행 전,
`PARITY_CONTROL_PROTOCOL.md`는9arm 결과를 본 뒤12arm 실행 전에 작성했다.
후자는 명시적인 post-diagnosis control이며 holdout이나 일반 최적 정책의
근거로 표시하지 않는다. 새로운 strong no-go 결과는 제시하지 않았다.

파일: `REPAIRED_RESULTS.json`, `SEED_SUMMARY.json`,
`SEED_BUILD_RECEIPT.json`, `SEED_EXECUTION_RECEIPT.json`,
`PARITY_RESULTS.json`, `PARITY_SUMMARY.json`,
`PARITY_EXECUTION_RECEIPT.json`, native logs.
원래 `RESULTS.json`과 `EXECUTION_RECEIPT.json`은 보존했다.
실행한9arm example source는 `selective_jvp_seeded_executed.rs`에 있으며
SEED_BUILD_RECEIPT의 source hash와 일치한다. 이 floating parity control의 정확한 실행 source는
`selective_jvp_parity_executed.rs`이며 PARITY receipt의 source hash와
일치한다. 현재 source의 후속 exact 비교 실행은 별도 EXACT_WORK receipt에
example·switching source 및 binary hash가 있다.

이 추가 작업은 기본 adaptive 정책을 자동 변경하지 않았다. 사용자는
new checkpoint-seed 옵션과 기존 entry ratio를 명시적으로 선택할 수 있다.
이 세 rate cohort에서는 정확도를 유지하면서 RHS-only 우선·실측 우세 후
JVP 사용이라는 요청의 실제 작동 예를 확보했다.
