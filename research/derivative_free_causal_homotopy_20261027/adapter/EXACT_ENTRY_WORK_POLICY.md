# 정수 작업량으로 유도한 strict entry 정책

## 구현과 실험 결론

새 `SwitchingConfig::strict_entry_work_improvement`는 실제 완료된 같은
checkpoint/prefix 재생의 정수 작업량을 직접 비교한다. 기본값은 false이며
기존 `entry_cost_ratio=0.8`, 그 validator, 기존 factory 동작을 유지한다.
활성화한 competitive entry pilot에서는 부동소수 ratio나 `nextdown(1)`을
사용하지 않는다. 현재 원자료와 재현 정보는 `EXACT_WORK_RESULTS.json`,
`EXACT_WORK_SUMMARY.json`, `EXACT_WORK_EXECUTION_RECEIPT.json`에 있다.

18개 focused native 계약과 12개 원래 물리 문제 arm을 모두 통과했다.
앞선 parity 실험의 모든 수치 결과, segment counter, 시각, state,
미완료 pilot 결과가 동일하다. 변경된 필드는 policy 이름/metadata 및
strict comparison을 설명하는 decision 문구뿐이다. 이전 floating parity
실험은 역사적 진단으로 그대로 보존했다. 정확히 실행한 이전 source는
`selective_jvp_parity_executed.rs`, 새 source는
`selective_jvp_exact_executed.rs`이다.

| rate | RHS-only 총 작업량 | 원래 selective | checkpoint seed + 기존 .8 | seed + 정수 strict |
|---:|---:|---:|---:|---:|
| 1 | 79 | 79 | 79 | 79 |
| 1000 | 2377 | 2958 | 2890 | **1691** |
| 10000 | 19261 | 20190 | 20094 | **1629** |

정수 strict arm의 총 RHS/JVP는 각각79/0, 1105/426, 992/445이고
시도 횟수는13,178,158이다. RHS-only 대비 작업량 감소는28.8599%,
91.5425%다. 모든 failing/discarded pilot을 포함한다. 이 계산은 기존
결정론적 작업량 목적함수에 관한 것이며 wall-clock 측정은 수행하지 않았다.

## 1. 목적함수와 정확한 경계

기존 adapter가 보고하는 작업량의 정수 원형을

\[
 C(s;n)=N_f(s)+N_{Jv}(s)+N_{f_t}(s)+nN_J(s)+N_{\mathrm{solve}}(s)
 \in\mathbb N_0
\]

로 정의한다. 모든 항은 해당 segment에서 실제 누적한 native counter다.
`work_units`의 binary64 표시값을 역변환하지 않고 위 식을 직접 계산한다.
이 식은 기존 코드의 proxy 단위를 보존한다. 예를 들어 비싼 JVP 하나와
싼 RHS 하나의 실제 시간 비용이 같다는 수학적 주장은 포함하지 않는다.

**정리 E1 — 관측된 동일 구간의 strict 최소 작업량 선택.** 같은 시작
checkpoint에서 같은 끝 시각까지 native 정확도 계약을 만족하며 완료한
두 재생의 작업량이 E와 I라 하자. 동률이면 기존 explicit 경로를 유지하는
규칙 아래에서 관측 목적함수를 최소화하는 후보 선호 조건은 정확히

\[
 I<E,
 \qquad\text{또는 }E>0\text{일 때 }I/E<1
\]

이다. 따라서 이 목적에 대한 손익분기 상수는1이며 별도의20% margin을
수학적으로 요구하지 않는다.

*증명.* 후보로 바꿨을 때 같은 구간의 목적함수 차이는 I−E다. 이것이
음수일 때만 strict 감소한다. 0이면 tie rule에 따라 incumbent를 유지하고,
양수이면 incumbent가 더 저렴하다. E>0인 경우 양수 E로 나누어 동치식을
얻는다. E=0이면 I≥0이므로 strict 개선이 불가능하다. ∎

E1은 **이미 관측한 동일 구간**의 비교 정리다. replay는 별도로 비용을
소비하고, 실제 productive implicit 경로는 accepted explicit prefix의
끝에서 시작한다. 따라서 E1만으로 이후 구간의 비용이나 전체 궤적의
최적성이 증명되지는 않는다. 위 전체 비용 감소는 별도로 실행하여 관측한
결과이며 초기 losing pilot work241도 총계에 들어간다.

**따름정리 E1a — 추가 비용과 오차 구간을 포함한 보수적 조건.** 실제
후보 비용이 `[L_I,U_I]`, incumbent 비용이 `[L_E,U_E]`에 있고 후보에만
발생하는 추가 비용이 `[0,S_U]`에 있다면

\[
 U_I+S_U<L_E
\]

는 후보의 strict 개선을 보증하는 충분조건이다. 실제 각 비용이 구간 안에
있으므로 I+S≤U_I+S_U<L_E≤E이다. 정확한 counter에 추가 후보 비용이
없는 E1에서는 구간이 점으로 축소되어 I<E가 된다. 따라서 이후 비용
교정은 임의 ratio를 맞추는 대신 실제 비용 항과 검증된 경계를 넣는
방향으로 확장할 수 있다. 미래 구간에 이 조건을 적용하려면 그 미래
구간에 유효한 경계가 별도로 필요하다.

## 2. native callback 중단 경계

**정리 E2 — strict 개선 가능성을 보존하는 정수 callback 상한.** 완료
후보의 총 작업량을 C, 현재까지 청구된 callback 작업량을 L이라 하자.
아직 수행하지 않은 비용과 linear-solve 비용이 음수가 아니면 L≤C이고,
L은 감소하지 않는다. incumbent 작업량 E≥1에 대해 L≥E가 되면
C<E는 불가능하다. native guard가 완료한 callback을 청구한 뒤
`L>cap`에서 멈추므로 정확한 정수 상한은

\[
 \mathrm{cap}=E-1
\]

이다.

*증명.* L≥E이면 C≥L≥E다. 정수 L,E에 대해 L>E−1은 L≥E와 동치다.
따라서 해당 중단은 그 시점에서 strict 승자가 될 수 없는 후보에만
적용한다. 마지막으로 경계를 넘긴 callback까지 청구하는 native guard
규칙은 그대로 유지된다. ∎

E=0에는 strict 승자가 없으며 구현은 guard의 가장 작은 cap0을 사용한다.
E−1이 native u64 상한 sentinel보다 작게 표현될 수 없으면 optional
callback 조기 중단을 사용하지 않는다. 기존 global/pilot attempt budget은
남는다. dense provider의 기존 structure verification allowance처럼
별도 bookkeeping 비용이 guard에 여유로 더해지는 경로는 중단을 늦출 수
있으나 최종 strict 정수 비교를 느슨하게 만들지 않는다. 이번 실험은
provided JVP 경로이므로 이 allowance가 없다.

## 3. 정수 정확성과 qualification

**보조정리 E3 — 표현 손실 없는 판정.** 모든 곱셈과 합이 u128에 들어갈
때 checked arithmetic으로 계산한 C는 정의한 정수 목적함수와 같다.
따라서 binary64의 정확한 정수 범위2^53을 넘어도 동률과 한 단위 차이를
정확히 구별한다. overflow가 발생하면 비교를 false로 끝내고 cheapness
근거로 사용하지 않는다.

*증명.* u64 counter를 u128로 변환하면 정확하다. 차원과 Jacobian 횟수의
checked 곱, 이어지는 각 checked 덧셈이 성공할 때 정수 연산의 귀납으로
결과가 정의한 합과 같다. 실패 시 값 대신 None을 반환하고 preference를
거부하므로 wraparound로 인한 거짓 우세가 없다. ∎

`strict_entry_work_preferred`는 다음 native 증거를 요구한다.

- explicit segment에 accepted step이 있고 실제 양의 prefix가 진행되었다.
- candidate가 성공적으로 완료했다.
- candidate 시작 시각은 explicit의 시작 시각과 같다.
- candidate target 및 reached time은 explicit의 실제 reached time과 같다.
- checked integer candidate cost가 incumbent보다 strict하게 작다.

호출부가 동일한 물리 checkpoint state로 replay를 구성한다. explicit가
원래 더 먼 target 전에 budget으로 멈춘 경우라도 accepted prefix 자체는
유효하므로 incumbent의 전체 `success` flag는 추가로 요구하지 않는다.
새 기능은 **competitive entry pilot이 켜진 경우**의 비용 판정이다.
기존 pilot budget0 direct-entry 모드의 의미, derivative permission,
pressure 조건, retry scheduling, return 정책은 변경하지 않았다.

모든 tolerances, RODAS/DOPRI 식, native acceptance/rejection 검사,
remaining attempt cap과 실제 work charge는 동일하다. strict flag는
미완료 후보나 checkpoint가 다른 후보를 싸다는 이유로 승인하지 않는다.

## 4. 실제 검증과 반례 범위

새 private arithmetic 계약4개는 다음 경계를 실행한다.

1. E=I 동률, ±1 차이, 2^53 부근 및 u64 최대 근처의 정수 차이.
2. 모든 native 비용 종류와 차원 가중치, checked u128 overflow.
3. 실패/다른 start/다른 target/미완료 prefix/NaN 시간/accepted0 거부.
4. E=0,1,192,2^53+1,u64::MAX의 정확한 cap 및 표현 불가 cap 거부.

기존 outward-bound arithmetic2개, entry/checkpoint integration5개,
기존 evidence-switch7개를 함께 실행해 **18 PASS**를 얻었다. integration은
정수 strict 후보가 실제 work160<192로 완료해 entry를 얻으며 그대로 둔
.8 margin은 통과하지 않는다는 분별적 사례를 포함한다. Forbidden에서는
새 두 flag를 모두 켜도 JVP0이다.

실험은 n16, rate1/1000/10000, y0=1, [0,1], 8windows,
rtol1e-7, atol1e-10, h0=1e-4, max50000attempts를 유지했다. 정확해
exp(−rate*t)에 대한 모든 반환 시각/성분의 원래 절대오차 cap1e-6을
12/12 arm이 만족했다. strict arm 최대 오차는 각각
3.3718316472253207e-9, 1.0654127077097675e-9,
1.3146334350983002e-9이다. 모든 arm의 callback atomics와 native counter가
일치하고 Jacobian build 및 f_t 호출은0이다.

첫 strict entry가 인정된 같은-prefix 비교는 rate1000에서
[.125,.22500838873374865], rate10000에서
[.005852350232369075,.015426870629347594]이며 둘 다160<192다.
쉬운 rate1에서는 모든 arm이 JVP0을 유지한다. 시드만 교정하고 .8을
유지한 arm의 전 궤적 비용 손해도 원자료에 남았다. 이 cohort와
관측 grid 밖의 전역 최적성을 주장하지 않는다.

## 5. 아직 이론·측정이 필요한 정책 요소

이번 변경으로 임의 margin을 제거한 범위는 **실제 같은-prefix entry 비용
판정**과 그 callback 중단 경계다. 전체 adaptive solver의 모든 상수를
최적 상수로 증명한 결과는 아니다. 다음 요소를 구분한다.

| 요소 | 현재 상태 | 추가 근거 |
|---|---|---|
| entry strict 경계1 | 관측 정수 목적함수 최소화에서 유도 | E1, exact comparison |
| callback cap E−1 | 정수 prefix lower bound에서 유도 | E2, charged crossing call |
| checkpoint seed max/min | 이미 관측한 proposal과 target 구간에 한정 | 기존 인과성 proof와 native acceptance |
| RHS/JVP/solve 단위 가중치 | 기존 deterministic proxy 정의 | 실제 callback 비용과 병렬 span 측정에 따른 비용 모델 |
| pressure/retry/backoff/rearm | 기존 실행 scheduling heuristic | 모델/구간 변화의 검증된 비용 경계, exploration 손익 분석 |
| return margin .8 | 기존 정책 그대로 | 같은 비용 목적의 독립적인 return 연구 및 증거 |
| step controller 계수·minstep | 기존 numerical driver 계약 | 해당 수치법의 안정성·오차 추정과 implementation-specific 검증 |
| 8windows와 실험 tolerance | 사전 고정한 실험/사용자 정확도 설정 | 다른 regime에 대한 별도 실험 계약 |

**후속 과제**는 future work를 관측 work와 연결하는 검증된 regime 경계,
비대칭 callback 비용, probe 비용을 포함한 누적 정책 분석이다. 이들을
현재 실험 결과에서 이미 증명된 보편 상수로 확대하지 않는다.

## 6. 변경된 의존성의 연구 루프

기존 checkpoint-seed 보고서의 R1–R10/C1–C10 완료 근거를 재사용했다.
이번 추가 루프에서는 R1/C1 목적을 exact observed work로 고정하고,
R2/C2 기존 integer counter·guard를 읽었으며, R3/C3 float 근사 경계를
분리했다. R4/C4에는 `EXACT_WORK_PROTOCOL.md`를 실행 전에 작성했다.
R5/C5에서 E1–E3 및 exact helper/flag/callsite를 도출·구현했고,
R6/C6에서 실제 경계 계약과 변경 부분의 이론 review를 수행했다.
R7/C7은 한 번의 bounded12-arm 실행, R8/C8은 모든 기존 결과의 수치
동일성과 실패 비용 보존, R9/C9는 국소 증명/전역 경험 결과/남은 heuristic
구별, R10/C10은 source·hash·원자료 보존과 추가 parameter 탐색 종료다.

## 코드와 재현 파일

- `crates/vigilode/src/switching.rs`: flag, integer_work, strict comparison,
  callback cap, 기존 outward-enclosure 재사용, private arithmetic contracts.
- `crates/vigilode/tests/entry_checkpoint_seed_contracts.rs`: native boundary,
  permission, same-prefix strict winning integration.
- `crates/vigilode/examples/selective_jvp_20261027.rs`: 네 arm씩 세 rate.
- `EXACT_WORK_EXECUTION_RECEIPT.json`: 실제 명령/exit/시간/source·binary hash.
- `exact_arithmetic.log`, `exact_integration.log`, `exact_build.stderr.log`.
- `EXACT_WORK_RESULTS.json`, `EXACT_WORK_SUMMARY.json`.

측정 profile, Rust1.94.1, locked offline dependencies, compiler jobs2 및
RAYON1을 사용했다. 실행은 공유 flock으로 직렬화했다. 기록된 실행 wall
시간은 provenance이며 성능 timing 주장으로 쓰지 않았다.
