# VigilODE 독립 개발 계열과 응용 계열의 통합 감사

기준 코드는 `555e71e03fe31e6c9b21961c8e2d263b1794eb6f`이며, 2026-10-22에 원격 추적 ref의 실제 Git 객체를 비교했다. 이 문서는 개발 통합 기록이다. 외부 연구보고서에서는 아래 결과의 과학적 범위와 API 사양을 사용하고 내부 작업 식별자는 생략한다.

## 1. 독립 개발 브랜치가 이미 포함된 범위

`production/wave3-integration`의 `f2f20c1d8cd606e70ddbd8409c0f3b4529e928a1`과 `production/vigilode-plan-20261021`의 `cc2e1a009ac6802ef01a289392e9715074a882ba`는 모두 현재 기준 코드의 **정확한 조상**이다. 조사한 `origin/production/*` 25개 ref 모두 기준 코드에 없는 incoming commit 수가 0이었다. 따라서 독립 개발 내용을 다시 cherry-pick하거나 이전 구현으로 덮어쓸 필요가 없다. 이 판정은 이름의 유사성이나 문서상의 완료 선언이 아니라 `git rev-list`, `git merge-base`의 객체 관계에 근거한다.

그 사이 10월 21일 통합 재감사는 독립 개발 facade와 별도 인증 연구 모듈을 실제로 합쳤다. 이후 BASS/rei 계열은 그 위에 `StageLinearSystem`, allocation 없는 `partial_t_into`, accepted-step 중단 및 재시작 인터페이스, 구조화된 광자 수송 커널을 추가했다. 이 계보를 빠뜨리면 현재 코드를 단순한 BASS 전용 예제로 오해하거나 이미 고친 공개 API 결함을 다시 열린 문제로 보고하게 된다.

| 독립 개발에서 구현된 기능 | 현재 의미 | 남은 범위 |
|---|---|---|
| `vigilode` 공개 facade | `Problem`, `Options`, `Rodas5p::solve`, typed status와 capabilities 조회 | 모든 조합을 지원하는 것은 아니며 미지원 조합은 명시적으로 거부한다. |
| dense/banded fast RODAS5P | 8-stage 적분, 연속 출력, scalar/vector absolute tolerance 일부 경로 | 전역 오차를 국소 tolerance 하나로 보증하지 않는다. |
| matrix-free JVP 경로 | GMRES를 사용하는 별도 구현 | vector atol, mass matrix, dense output 조합은 현재 공개 경로에서 거부된다. |
| 사건 검출 | 연속 출력 위 Illinois root finding | 사후 처리이다. terminal event 이후 계산을 즉시 멈추지 않으며 접선근·다중근을 완전 검출하지 않는다. |
| facade 소유 통계 record | 내부 record 변경과 공개 JSON을 분리한 exhaustive 변환 | downstream Rust type migration과 안정 API release 검토는 별도이다. |
| 지연된 전처리자 재사용 | bounded/resolvent/diagonal 후보와 비용 계상 | `Tier::Performance`가 자동으로 이 후보를 선택하지 않는다. |
| Laguerre·Leja·Chebyshev 및 homotopy | 명시적 모델·오차예산에 결합된 연구 API | 일반 callback의 정확도 또는 보편적 속도 개선으로 전이할 수 없다. |

현재 `docs/plan/OPEN_ITEMS.md`는 wave-3 시점의 역사 기록이다. 그중 O-01의 sequential dense enforcement 실패 수용은 `dense_output_v2.rs`에서 이미 수정되어 recoverable 오류면 거절하고 다른 오류면 종료한다. O-26의 facade-owned 통계 record도 `records.rs`와 기존 통합 검사로 구현되어 있다. 이 두 항목을 현재 blocker로 재등록해서는 안 된다. 전역 오차, 미지원 기능 조합, 사후 event 처리 등의 실제 남은 제한은 유지된다.

## 2. 별도 tip에만 존재하던 자료의 실제 통합

| 원본 브랜치 | 고유 내용 | 이번 처리 |
|---|---|---|
| `research/orchestration-sandbox-20261022-vigilode` (`f9da4aae…`) | 공통 조상 `2064366e…` 이후 2개 commit, 16개 파일, 40,456 bytes | affine 30→33 오차 norm C99 소스·증명·최초 실패·수정 검증을 그대로 수용했다. |
| `results/ncp-followup-fvm-5fd6e45` (`fbbc832f…`) | 고유 결과 commit 1개, 2,516개 파일, 158,551,005 bytes | 원본 NCP followup 측정과 실패·분석·재현 자료를 그대로 수용했다. |

모든 대상은 기준 checkout에 없던 경로였다. 기존 파일과 다른 bytes가 있으면 중단하도록 한 import를 사용했으며 겹치는 변경은 없었다. `BRANCH_IMPORT_MANIFEST.json`은 각 파일의 원본 commit, Git blob SHA-1, SHA-256, 길이를 기록한다. 총 2,532개 파일의 byte identity를 유지한다. 공개 시에는 이미 원격에 있는 blob을 재사용할 수 있으므로 158 MB를 다시 업로드할 필요가 없다. 원래 실행 일자와 source commit을 새 코드에 맞춰 고치지 않았다.

추가된 NCP followup은 numerical source `5fd6e459…`의 결과이다. 예를 들어 periodic heat의 analytic stencil은 두 arm과 1,046회 실행에서 반환 times/states가 동일했고 총 RHS 작업이 5,712에서 336으로 줄었다는 기존 근거를 갖는다. 이는 구조를 아는 Jacobian 조립의 유효한 특수화 근거이다. 현재의 새 low-rank 코드, 다른 PDE 또는 전체 production solver의 성능 결과는 아니다. 기존 결과를 다시 계산하지 않았다.

## 3. Affine 축약 오차 norm의 수학적 의미

원래 상태를 \(v=Lz+c\)로 축약하고 \(z\in\mathbb R^{30}\), \(v\in\mathbb R^{33}\)라 하자. 원래 오차 척도를 보존하려면 국소 오차 \(e\)를 먼저 접공간에 lift해야 한다.

\[
 N(e)^2=\frac1{33}\sum_{i=1}^{33}\frac{(Le)_i^2}{s_i^2}
       =\frac1{33}e^TL^T\operatorname{diag}(s_i^{-2})Le,
 \qquad s_i>0.
\]

이 식은 단순한 30차원 대각 norm과 다르다. 제거한 중성 분율의 오차는 해당 이온 분율 오차 합의 음수이며, 보존 총량의 상수항은 차분에서 사라진다. 원래 33개 scale과 분모 33을 모두 유지해야 한다. 세 Li 이온 오차를 모두 1로 두고 모든 scale을 1로 두면 올바른 norm은 \(\sqrt{12/33}=0.6030226892\), 단순 30차원 norm은 \(\sqrt{3/30}=0.3162277660\)이다. 후자는 이 방향을 약 1.907배 작게 평가한다.

수용한 C99 구현은 binary64 입력을 extended `long double` 보상합과 `hypotl`로 계산한다. `[1e308,-1e308,1e-300]`과 같은 혼합 지수 취소에서 작은 중성 오차를 지우지 않기 위해 합산 전 정규화를 하지 않는다. 내부 exponent range가 부족한 플랫폼은 status 3을 반환한다. 따라서 동일한 API가 모든 플랫폼에서 성공한다는 뜻도, 모든 binary64 입력에 대해 correctly rounded라는 증명도 아니다.

새 호스트에서 컴파일 및 1회의 portability replay를 수행했다. 독립 1200자리 Decimal 대조 10개와 음성 검사 5개가 기존 32 ulp 기준을 통과했다. 명령·컴파일러·각 결과는 `LIFTED_NORM_REPLAY.json`과 `.log`에 있다. 이는 C99 커널의 현재 호스트 실행 근거이며 native ODE trajectory 실행 횟수와 NCP 제출 횟수는 모두 0이다.

## 4. 판정과 구체적 다음 개발

범용 production 승격은 여전히 보류한다. 검증된 dense/banded facade 사용, 구조화된 photon client, analytic stencil 및 affine norm은 각각 서로 다른 범위의 근거를 갖는다. 소스의 통합과 수학적·실험적 보증의 범위 확대는 구별해야 한다.

1. **공개 facade.** 최종 수정 코드에서 관련 facade regression만 실행한다. 기존 10월 21일 통합 검사의 native 298, doctest 4, Python 142 성공은 원래 소스의 기록으로 재사용한다. 새 kernel 추가가 그 총시험 수를 새 코드의 전체 검증으로 바꾸지는 않는다.
2. **Affine norm의 실제 이식.** 고정 크기 배열과 status 확인을 갖춘 명시적 FFI wrapper 또는 client-owned error norm hook을 추가한다. old/new full33 상태를 복원하여 caller의 원래 scale 규칙을 계산하고, embedded error를 lift한 norm으로 수락 여부를 정한다. norm 실패 시 accepted state를 commit하지 않는다. H/He/Li 상관 오차에서 full33 적분과 수락·거절 결정을 비교하는 검사가 이식의 수용 조건이다. 이 커널을 병합했다는 이유만으로 해당 hook이 연결되었다고 보고하지 않는다.
3. **지연 전처리자.** 기존 resolvent 진단에서 HIRES의 factorization/accepted-step은 약 0.441로 원래 0.4 목표보다 크고, 추가 back-solve가 발생했다. 더 넓은 경로의 기본값으로 켜기보다 새 구조 전처리자와 총비용을 같은 정확도에서 비교한다.
4. **범용 정확도.** 국소 추정 오차, 출력 보간 오차, 흐름의 증폭을 분리한 측정을 수행한다. 일반 ODE에서 관측 전역 오차가 tolerance 한 단위를 넘는다는 사실 자체는 구현 결함의 증명이 아니며, 사용자가 정한 출력 예산의 만족 여부를 별도로 판단한다.
5. **역사 자료의 재사용.** pure solver, heat followup, photon/chemistry 캠페인의 source와 metric을 각각 명시한다. 미지원 비교 arm과 실패를 제거하지 않고, native code 변경에 따른 새 실행은 필요한 의존성에 한정한다.

## 5. 검증 범위

이번 분담에서는 Git ancestry와 파일 내용 identity를 1회 조사하고, 독립 branch의 공개 C99 코드를 현재 호스트에서 1회 실행했다. 기존 proof나 NCP 캠페인을 재실행하지 않았다. native Rust code 변경·git merge·commit·push는 수행하지 않았다. 부모 통합 작업이 최종 소스 검증과 발행을 담당한다.

## 6. 수용한 NCP followup의 해석상 주의점

원래 source `5fd6e459…`에서 실제 측정한 결과만 아래에 요약한다. 이 절의 근거는 `CAMPAIGN_REPORT_v2.json`이며 구조화된 추출은 `HISTORICAL_FOLLOWUP_EXTRACT.json`이다.

| 대상 | 보존된 관측 | 현재 보고서에 허용되는 결론 |
|---|---|---|
| heat analytic stencil | 동일 times/states, RHS 5,712→336; warmed elapsed STEN/BASE 0.586, IQR 0.582–0.590; 489/489 paired blocks에서 STEN이 짧음 | 동일 fixture의 Jacobian 조립 비용 감소. 다른 크기·다른 PDE의 보편적 시간 우위는 미측정이다. |
| VODE MF20 | scaled application error 0.990, 1% 여유로 통과; NST 1,060, NCFN 349 | functional-iteration BDF의 보존된 결과이며 Jacobian BDF 전체의 대표값으로 쓰지 않는다. |
| VODE MF21/MF22 | internal tolerance를 application budget과 같게 둔 실행에서 scaled error 4.72/5.97 | 해당 설정은 application budget을 만족하지 않았다. tightened tolerance에서 수렴했다는 검증 기록이 있으므로 VODE 방법 자체를 제거할 근거가 아니다. |
| Julia Rodas5P | scaled error 약 0.0013, 5 steps; solve 약 5.8–9.1 ms, GC 포함 반복 47–55 ms | 통과한 비교 arm이다. 16–17 s process wall은 package/JIT 비용이므로 solve 시간과 섞지 않는다. |
| diffsol BDF | scaled error 약 0.0055, states-only 검증; solve 약 4.2 ms | 시간 배열을 solver가 반환하지 않아 requested grid를 가정했다. builder/변환을 timer 밖에 두므로 native와 범위가 다르다. |
| Bordered kernel | n=1,600 residual 1.108e-13, error 5.88e-14; n=4,096 residual 8.9e-16, error 4.4e-16 | 제조한 선형계에서의 정확도 결과이다. BASS/rei time history나 일반 ODE 성능 실험은 아니다. |

native heat 경로가 40개 출력 구간에 42 steps를 사용한 데 비해 Julia가 보간으로 5 steps를 사용했다. 따라서 정확한 출력 시각에 매번 착지하는 정책의 비용을 별도 실험으로 분리하는 것은 구체적인 개선 방향이다. 단, 연속 출력의 정확도 계약을 보존해야 하며 이 자료만으로 step 수 감소를 구현 완료라고 할 수 없다.

원래 캠페인에서 미실행 항목은 allocation mode, n=32/512/1600/4096 heat 크기 확장, native banded/matrix-free heat 비교, 각 solver의 internal tolerance ladder, endpoint-only 비교, rotation 전체 trajectory 재실행이다. Julia cache의 metadata 변경과 외부 backup 작업의 동시 실행도 기록되어 있다. 이 사실들을 지운 단일 speedup 수치를 외부 보고서의 일반 결론으로 사용하지 않는다.
