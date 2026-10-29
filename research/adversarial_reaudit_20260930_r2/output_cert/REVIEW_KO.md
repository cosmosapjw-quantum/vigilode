# 출력·수락 계수·정책·M08 재감사

대상 commit은 `7708ef90554fc3986478d4602de6a01c7266b14f`다. 수리·수치 계약을 읽은 별도 검토자 `/root/r2_output_cert`가 변경 `9259641533fee552d39c6bbc47877588e9b6f554`, `546c229630f15c57e155f74d444db8d2d81560bd`와 현재 소비 경로를 검토했다. production source는 수정하지 않았다. 아래 실험은 현재 Rust 1.94.1 및 제공된 vendor로 실제 실행했다. 하네스 이름을 모델 성능 검증으로 해석하지 않는다.

결론은 **기존 네 finding의 구체적 재현은 수정되었지만, AD-01의 의미론적 계약은 아직 충족되지 않는다**이다. 기존 회귀 7개가 모두 통과하는 상태에서, 표현 가능한 짧은 구간의 허위 완료와 다음 ULP 출력의 조기 소비를 재현했다. 잘못된 enum 필드를 막는 CERT-01 수정도 유효하지만, 유효한 필드의 중간 연산 overflow가 `Mixed` budget을 크게 만드는 별도 결함이 남아 있다.

## 기존 finding의 판정

| 기존 ID | 현재 판정 | 실제 근거와 한계 |
|---|---|---|
| AD-01 | **PARTIALLY_FIXED** | 기존 세 원점/단위 케이스의 dense·clipped 및 작은 span 회귀 3개 PASS. 아래 R2-OUT-01에서 별도 표현 가능한 시간들이 여전히 합쳐짐. |
| AD-02 | **CLOSED_REPRODUCER** | Prothero–Robinson dense Enforce의 최종 계수가 accepted/internal/diagnostics와 일치하는 원 회귀 1개 PASS. 모든 integrator 전체의 계수 계약을 증명하지는 않음. |
| AD-03 | **CLOSED_REPRODUCER** | finite coefficient overflow, NaN operator, NaN preconditioner를 현재 공개 M08 API가 거절하는 회귀 1개 PASS. 일반 roundoff 인증 문제는 아래 별도 한계. |
| CERT-01 | **CLOSED_REPRODUCER** | 직접 enum 및 deserialize를 통한 malformed fields 7개 사례와 정상 policy가 포함된 회귀 PASS. 아래 R2-POL-01은 유효한 입력의 계산 안정성 문제. |
| CERT-02 | **CLOSED_REPRODUCER** | 양쪽 NaN 상태·음수/비유한 atol의 거절 및 정상 scale 계산 회귀 PASS. |

회귀 파일은 원본 네 파일의 byte-identical 복사본을 별도 crate에서 현재 integrator에 링크한 것이다. 실제 7 test PASS, 0 FAIL, 0 ignored. 동일 시험을 전체 테스트 수에 중복 가산해서는 안 된다. 실제 명령·해시·exit은 `EXECUTION_RECEIPT.json`, 원 로그는 `existing_regressions.log`다.

## R2-OUT-01 — P1: 표현 가능한 별도 시각을 여전히 같은 시각으로 처리

**근거 상태: implementation-verified + derived. 실패 층: NUMERICAL / IMPLEMENTATION.**

현재 `output.rs:12–20`은 `4 ε max(|t₁|,|t₂|)`와 `MIN_POSITIVE` floor를 사용하고, driver slack은 `10 ε |tf|`이다. `integrate.rs:92`의 fixed loop가 후자를 종료 조건으로 사용한다. 이 식에서 절대 floor 1만 제거해도 시간 원점에 대한 의존성은 없어지지 않는다.

독립 oracle은 `y′=v`, `y(t₀)=0`, `y(t)=v(t−t₀)`다. 모든 exact 값은 **입력으로 실제 표현된 binary64 시각**을 기준으로 계산했다. 원점에서 8 ULP의 span은 양의 표현 가능한 구간이며 4 ULP씩 두 번 전진할 수 있다.

| 공개 API 재현 | 실제 관측 | 올바른 계약 |
|---|---|---|
| fixed, `t₀=10¹²`, `tf=t₀+8 ulp(t₀)`, `v=1/(tf−t₀)` | `success=true`, attempts 0, `last_t=t₀`, `last_y=0` | `tf`까지 전진해 최종 1을 반환하거나 명시적 실패 |
| 같은 조건, `t₀=−10¹²` | 같은 허위 완료 | 같은 계약 |
| fixed, `t₀=1`, `tf=next_up(1)` | attempts 0, success true | 진행 불가능한 trial을 포함하더라도 미완료를 성공으로 보고하면 안 됨 |
| dense matrix-free, `[10¹²,10¹²+1]`, `h=0.25`, `v=8192` | `t₀+0.25`와 `next_up(t₀+0.25)`에 같은 2048.000000000006을 기록; 후자는 정확히 2049여서 오차 `−0.9999999999940883`; success true | 미래 request는 다음 accepted interval에서 평가 |
| clipped schedule `[t₀,next_up(t₀),t₀+1]` | `duplicate requested time` InvalidInput | 엄격 증가·표현 가능한 schedule을 duplicate라 하지 않아야 함 |

첫 fixed 예에서 span은 `0.0009765625`, driver slack은 약 `0.0022204460492503152`다. 반환값이 `tf`라는 timestamp를 달고 0을 반환한 것은 아니다. **`last_t=t₀`로 종점이 누락되었는데 완료를 true로 보고했다**는 정확한 finding이다. dense 예는 실제 미래 timestamp에 과거 endpoint 값을 붙이는 별도 관측이다.

원인은 `output.rs:354–383`의 membership 검사다. `next≤t_new+tolerance`이면 아직 미래인 `next>t_new`도 소비하고, `theta`를 `[0,1]`에 clamp하여 이전 endpoint를 반환한다. 원점에 따른 넓은 tolerance를 줄인 수정은 이전 fixture를 통과하지만 순서 관계를 바꾸는 구조는 유지한다. clipped `output.rs:291–295`는 서로 다른 표현 가능 시각을 duplicate로 거절한다.

**수정 설계:** timestamp의 동일성, interval membership, 계산된 landing의 반올림, 시간 전진 가능성은 서로 다른 계약으로 둔다. 저장된 두 유한 timestamp의 엄격 순서를 tolerance로 뒤집지 않는다. `next>t_new`는 이번 interval에서 소비하지 않고, `t_old≤next≤t_new`에서만 interval-local fraction을 평가한다. hard-stop 및 output index는 timestamp 근접성이 아닌 명시적으로 선택한 목표의 identity로 갱신한다. 마지막 목표까지 도달했음을 확인한 경우에만 success를 반환한다. `t+h==t`인 positive h는 typed `TimeResolutionExceeded` 또는 명시적으로 선택·기록한 representable landing 정책을 필요로 한다. 작은 양의 span을 자동 완료로 해석해서는 안 된다.

**개발 gate:** 원점 `0, ±1, ±10¹²`, ULP gap `1,2,4,8,16,32`, 요청 `next_down(endpoint), endpoint, next_up(endpoint)`의 실제 timestamp oracle; dense/clipped/fixed/adaptive 성공·실패 경로; adjacent hard stop의 정확한 방문 identity; partial output prefix를 별도로 검사한다. 잘못된 출력에 대해 tolerance를 크게 해 fixture를 통과시키면 안 된다. `h`가 실제 represented step과 다른 경우 state와 time의 일관성도 확인해야 한다.

subnormal span `MIN_POSITIVE/2`의 별도 native probe는 내부 step 0, success false로 반환했다. 이것은 허위 성공의 재현이 아니며 원인·완전한 subnormal 지원을 이 작업에서 해결했다고 주장하지 않는다. 원 로그에 보존했다.

## R2-POL-01 — P2: 유효한 Mixed policy도 중간 overflow에서 과대한 허용 budget을 반환

**근거 상태: implementation-verified + derived. 실패 층: NUMERICAL / IMPLEMENTATION.**

위치: `homotopy_policy.rs:146–166`, 실제 소비 경로 `homotopy.rs:1216` 부근. 새 `validate()`는 malformed enum을 제대로 막는다. 그러나

\[
B=\min\{\eta E,\epsilon_{ref}(h/h_{ref})^p\}
\]

의 두 항을 검증하기 전에 `f64::min`을 취한다. 입력이 모두 유효·유한해도 한 항의 overflow/NaN이 다른 유한 항 뒤에 숨을 수 있다. `h_ref`는 h와 같은 시간 단위를 가지므로 ratio 및 B는 무차원이다.

| 입력 | 정확한 budget | 실제 `decide(output_wrms=5,E=1,h)` |
|---|---:|---|
| `eta=10, epsilon_ref=0, h_ref=1e−308, p=2, h=1e308` | 0 | budget 10, accepted true; `0×∞=NaN`을 min이 제거 |
| `eta=10, epsilon_ref=1e−320, h_ref=1e−160, p=2, h=1` | `0.999988867182683…` | budget 10, accepted true; ratio 제곱이 먼저 overflow |

두 번째 정확값은 십진수 이상값이 아니라 실제 binary64 입력을 `Fraction`으로 바꾸어 계산한

\[
B=\frac{2505590639513609676395827429376}{2505618534107015908629993881161}
\]

이다. 따라서 WRMS 5를 거절해야 한다는 결론은 근사 오차와 무관하다. 이 작업은 공개 policy API의 잘못된 acceptance와 source consumer를 확인했다. 이 파라미터로 전체 ODE integration의 잘못된 fast acceptance까지 실행했다고 주장하지 않는다.

**실제 수행한 coding 후보:** `probe/src/bin/safe_budget.rs`는 각 항을 독립적으로 평가하고 오류를 min 전에 전파하며, 정확 zero budget을 직접 처리한다. 세 native 사례에서 zero case는 `Ok(0)`, finite-exact-budget case는 명시적인 `NonFinite` 오류, 정상 case는 `Ok(0.25)`였다. 세 경우 모두 WRMS 5를 수락하지 않았다. 후보는 원본에 적용하지 않았다. 극단적이지만 정확히 유한한 값을 전부 계산하는 완성품이 아니라, 잘못된 acceptance를 막는 **fail-closed 최소 후보**다.

다음 단계는 mantissa/exponent 분해 또는 bounded scaled multiplication으로 전체 expression을 평가하고, threshold 근처에서 허용 budget의 과대평가를 막는 것이다. 엄밀한 수락 판정에서는 `error_upper ≤ budget_lower`가 충분조건이다. budget을 upward rounding하는 것은 안전한 tolerance 판정과 방향이 반대다. 단순 `log/exp` 치환은 cancellation·roundoff bound 없이 인증으로 승격할 수 없다.

## M08 — overflow 수정은 유효; roundoff 한계는 계속 명시해야 함

현재 `audit2_reusable_transaction_research.rs:437–469`의 중간값 검사는 기존 AD-03의 NaN fail-open을 막는다. 이 성공과 별개로, `:387–393`은 directed rounding이 아님을 인정하면서도 반환 epsilon으로 정확한 역행렬 norm 상계를 서술한다.

기존 문서상의 한계를 구체화한 scalar 확인이다. `u=2⁻⁵²`, `P=1+u`, `W=1−u`이면 입력은 모두 표현 가능하고 정확 산술에서

\[
PW=1-u^2,\qquad \epsilon=u^2=2^{-104},\qquad |(PW)^{-1}|=(1-u^2)^{-1}>1.
\]

현재 native API는 product를 1로 반올림하여 epsilon=0, `certified=true`를 반환한다. `1/(1−epsilon)`을 반환값으로 해석한 수치 상계는 1이므로 엄밀한 upper bound가 아니다. **PW는 실제로 가역이다.** 이 예를 새로운 singular false-certificate나 기존 AD-03의 재발로 세지 않는다. 일반 inverse bound를 인증한 근거가 없다는 `LIMITATION_CONFIRMED`다.

직접 유도한 충분조건은 `E=I−PW`, 계산값 `Ê`, 검증된 오차 `||E−Ê||₁≤η_fp`, outward norm `||Ê||₁≤e_upper`에 대해 `q=e_upper+η_fp<1`이면 Neumann 급수로

\[
\|(PW)^{-1}\|_1\le (1-q)^{-1}
\]

이다. cache의 byte/identity equality로 `ΔW=0`을 보이는 것은 **평가 roundoff `η_fp=0`**을 보이는 것과 다르다. 임의 matrix-free callback은 이 오차를 자동 제공하지 않으므로, 우선 기존 bool을 diagnostic 상태로 표기하거나 certified operator application의 오차 enclosure 계약을 추가해야 한다. explicit dense 경로부터 interval/FMA error enclosure로 인증을 만들고 범위를 제한하는 편이 구현 가능하다.

## 수락 계수: 성과와 추가 범위

`adaptive.rs:47–63`의 최종 거절 재분류와 dense 호출은 기존 AD-02 회귀를 닫는다. `integrate.rs:230–232`의 legacy unobserved driver에는 같은 호출이 없지만, 이것만으로 현재 오계수를 입증하지 않았다. SABR fast kernel 자체가 embedded+fixed-point criterion을 적용하며 Sequential finite 검증도 있어, 동일 조건에서 outer-only rejection의 도달성을 따져야 한다. 이 경로는 **source coverage concern**, 신규 finding은 아니다.

지속 가능한 개선은 trial kernel의 work와 최종 disposition을 분리하여 outer transaction 한 곳에서 disposition을 한 번 기록하는 것이다. 필요한 gate는 각각의 lane에서 `attempts=accepted+rejected+명시된 무판정실패`, `accepted=committed advances` 및 failure 이후 history/cache rollback이다. q1/q2 내부 subtrial과 macro trial을 같은 계수에 섞지 않는 단위 정의가 선행되어야 한다.

## 재현과 종료 상태

`native_probe.jsonl`은 exit 0인 반례 실행의 원 출력이다. **probe의 exit 0은 solver correctness PASS가 아니다.** `exact_oracles.json`, `derive_exact_oracles.py`는 exact scalar reference를 보존한다. `safe_budget_candidate.jsonl`은 후보의 세 bounded 사례이며 `safe_budget_candidate.exit=0`이다.

다른 경로에서 재실행할 때 Rust 및 offline vendor 환경을 설정한 후 다음을 실행한다. 제공 helper는 원본 source 해시 75개를 검사하고 별도 디렉터리에 probe를 복사하며 원본 저장소를 수정하지 않는다.

```bash
python3 run_probe.py --repo /path/to/vigilode --out /tmp/vigilode-output-cert-rerun --target-dir /tmp/vigilode-output-cert-target
```

helper의 relocation `--prepare-only`는 실제 실행되어 정상 종료했다. 자동 재배치 helper를 통해 full compile을 다시 반복하지 않았으며, native 실행은 위 원 명령들로 수행했다. test/command receipt와 source identity는 별도로 보존한다. 외부 게시와 전체 보고서 판정은 상위 owner가 담당한다. 이 하위 작업은 추가 재귀 감사를 요청하지 않으며, 확인된 두 결함의 수정 gate를 구체화한 상태로 종료한다.
