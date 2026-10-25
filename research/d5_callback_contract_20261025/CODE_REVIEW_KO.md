# D5 코드·검증 독립 검토

## 현재 판정

**PASS_SCOPED_D5_STRICT_CALLBACK_CONTRACT. 열린 material finding은0개다.**
`/root/d5_code_review`가 구현 저자와 분리된 C9 검토자로 source, 실제 실행 로그,
saved corpus, exact oracle와 추가 schema receipt, 최종 PNG를 읽었다. 실행 전
source 검토부터 원자료 합류까지 하나의 보완적 review pass다. 이 검토자에게
별도의 compiler·test·callback·원 numerical oracle 실행은 없었다.

최종 판정은 **REVIEW.json과 SCHEMA_REVIEW.json의 결합**을 요구한다. 전자는
정확한 수치값, 후자는 vector/matrix shape와 frozen-key coverage를 확인한다.

읽은 파일은 `CONTRACT.md`, `PREREGISTRATION.md`, `THEORY.md`, 새 Rust 네 파일
`gateway.rs`, `tests.rs`, `probes.rs`, `session_tests.rs`, 재사용 `fixture.rs`와
`reused/admission.rs`다. `physmath-research-loop`1.0.0의 C9와 근거/상태 분리를
적용했다. 상세 identity는 `evidence/C9_REVIEW.json`에 기록한다.

## 코드에서 확인한 경계

| 대상 | 실제 source에서 확인한 결과 |
|---|---|
| 기존 sparse API | 새 gate만 full-write NaN scratch를 사용한다. 기존 previous-matrix/fixed-sparsity callback을 수정하거나 재해석하지 않는다. 시험의 legacy buffer와 strict buffer는 별도다. |
| Query | N>0와 finite bounds를 검사하며, domain order는 signed-zero 동등성을 갖춘 integer bit key로 판정한다. callback 진입 앞에 query domain 및 witness 검사가 있다. |
| RHS/J output | owned fixed-shape scratch만 callback에 빌려주고, post-witness·status·finite 검사를 통과한 값만 반환한다. Recoverable 오류의 미완성 scratch는 폐기한다. |
| Pair | RHS 임시값을 J 성공 전까지 내부에 보유한다. J 오류나 마지막 witness 오류에서는 `Prepared`가 반환되지 않고 pair count도 증가하지 않는다. |
| Counters | RHS/J는 실제 entry 전에 checked increment한다. preflight refusal은 callback work가 아니다. pairs overflow는 결과 반환을 막는다. cfg(test)의 counter seed는 방어 분기 시험이며 실제 도달성 증거가 아니다. |
| Fatal/handled panic | post-witness를 수행하고 sticky terminal fault를 남긴다. panic payload의 destructor가 다시 panic하면 해당 반환 보장은 적용되지 않는다. 현 계약은 destructor/payload drop의 정상 반환을 요구한다. |
| Epoch | expected epoch의 관측 전후 일치만 검사한다. 정직한 witness/no ABA/no unobserved mutation은 공급자 가정으로 남긴다. |
| Time FD | 앞서 발견된 FP finding은 현재 source에서 수정됐다. stencil 산술 전에 witness를 호출하고, 양쪽 represented point를 첫 RHS 전에 검사하며 실제 차 `t_plus-t_minus`로 나눈다. |
| Session | 새 시험은 같은 Session의 begin/abort/snapshot/cap을 호출한다. Session 재생성으로 attempt budget을 복원하지 않는다. synthetic accepted-prefix fixture는 ODE 해를 계산했다는 증거로 쓰지 않는다. |
| Unsafe boundary | 새 네 Rust source가 forbid(unsafe_code)를 선언한다. MXCSR reader는 바이트가 고정된 별도 fixture dependency에 있다. 이는 dependency tree 전체의 unsafe 부재 주장이 아니다. |

## Finding과 범위

### C9-FP-CONTRACT — P2, CLOSED_BEFORE_EXECUTION

초기 `BoxDomain::new/check`는 floating comparison을 쓰며 domain check가 첫
witness보다 앞섰다. Generic probe가 FP 상태를 고친 뒤 참값을 반환한다면 뒤의
관측으로 앞선 비교를 소급 인증할 수 없다는 경계를 root에 전달했다.

Root는 원 source/test 바이트를 `evidence/preexecution_bit_domain_correction/`에
보존한 뒤 domain order만 integer bit key로 바꿨다. 비교용 key에서 ±0를 동등화하고,
음수는 bits를 반전하며 양수는 sign bit를 토글한다. 유한 binary64의 음수/0/양수
수치 순서를 보존하고 원 query/output bits는 변경하지 않는다. 새 시험은 inverted
subnormal bounds, signed-zero 보존 및 음의 subnormal refusal을 다룬다.

수정 diff와 실제 계약문구를 다시 읽어 이 finding을 닫았다. 계약에는 read-only
probe와 checked arithmetic 중 비관측 FP 변경/FP ABA 부재 가정도 명시됐다.
이는 정상 FP에서의 실제 실행까지 증명한 것이 아니며, 그 근거는 후속 로그와
별도로 연결한다. 이 교정으로 새 과학 재실행은 발생하지 않았다.

### 이미 닫힌 실행 전 교정

stencil 산술 전 FP witness 누락은 저자/root가 원본 source를 보존한 뒤 수정했다.
현재 source에 witness가 있으며 시험에도 environment 우선 refusal이 추가돼 있다.
이는 이 reviewer가 발견한 새 실행 실패나 과학 재실행으로 세지 않는다.

## Frozen corpus 독립 산술 대조

Source loop와 callback 경로를 세어 root 실행에서 대조할 예상값을 얻었다.

| 항목 | source상 예상 |
|---|---:|
| polynomial point rows | 729 |
| represented centered FD rows | 49 |
| 실제 finite-wrong-J pair control | 1 |
| input refusals / FD refusals | 7 / 7 |
| summary 전 rows / 전체 JSONL rows | 793 / 794 |
| 실제 RHS entries / J entries | 828 / 730 |
| witness calls | 3,168 |
| forbidden callback entries | 0 |

Witness 수는 `729*4 + 49*5 + 1*4 + 3`으로 얻는다. Invalid query 7개는
probe/callback 둘 다 진입하지 않고, FD left/right/collapse는 각각 probe1/RHS0,
비유효 eps 네 경우는 probe0/RHS0다. 이 표는 아직 실행 결과가 아니다.

Polynomial grid는 finite rational truth를 직접 계산할 수 있는 dyadic 입력이다.
Exact oracle은 row 종류·고유 grid identity·모든 출력/배열 shape·시계/counter·refusal
종류·summary totals를 함께 확인해야 한다. `J+I`는 실제 callback admission
control1개와 저장 결과의 semantic mutation을 구분해 보고해야 한다. 세 변형은
compiled Rust mutant execution으로 세지 않는다.

## 완료된 합류

1. 정확한 S1 source와 compiler/version/build flags의 실행 identity.
2. 새30개 gateway+5개 Session 계약시험의 원래 로그와 종료 상태.
3. 원본794 JSONL rows 및 독립 exact Fraction oracle 결과.
4. 저장된 결과만 이용한 figure와 실제 image 확인.

위 source identity, 원 로그, schema까지 결합된 saved-output 검토와 실제 figure
확인을 모두 수행했다. 세부 근거와 finding closure는 아래에 있다.
이 검토자의 신규 compile/test/solver/benchmark/NCP/CAS/Desktop Commander 실행은0이다.
독립 personnel은 NOT_RUN이며 production HOLD, global completion=null을 유지한다.

## 실제 실행·원자료·그림 합류 검토

S1 executed source는 `0c73202734a52f30e4214d455be0fa9c47d37e0f`다. 원격 publication
receipt의 gateway/tests/probes/session-tests/oracle/fixture/admission 7개 blob/size와
현재 바이트가 일치한다. Rust1.94.1/LLVM21.1.8의 실제 version log와 `-F unsafe-code`
컴파일 argv를 읽었다. 새 gateway30/Session5 시험은 각각 원 로그에서 PASS이며,
그 결과를 재실행하지 않았다. Fixture와 새 gateway의 분리 경계는 실제 build에서도
유지된다. 일부 독립 binary에 사용되지 않는 API의 dead-code warning은 보존돼 있다.

Toolchain 초기 restore의 duplicated manifest.in 실패(exit1), 부분 runtime의
loader 실패(exit127), 이후73 runtime payload의 재사용 검증과11 payload 추가를
읽었다. 이는 과학적 반례가 아닌 인프라 복구 실패·교정이다. 기존 실패 기록은
남아 있고, 실행 budget의 현재16개 output/log hash가 실제 바이트와 일치한다.

원 corpus는170,379bytes/SHA256
`cb64d9455289f183363749289038d6f250bf63e0ab95ea94cd51defa88d2ba21`, 실제794행이다.
전체 원본의 hash와 행수를 읽고 경계/내부/FD/잘못된J/refusal/summary 행을 직접
대조했다. 원 callback corpus와 exact numerical oracle은 재실행하지 않았다.
기존 oracle 실행은8,613 comparison assertions, point값6,561 comparisons,
exact mismatch0과 FD최대오차0을 기록한다. Source에서 독립적으로 예측한
RHS828/J730/witness3,168/forbidden0과 실제 summary가 일치한다.

### C9-SCHEMA-COVERAGE — P2, CLOSED_BY_ADDITIVE_SAVED_DATA_REVIEW

Exact oracle의 `zip()`은 RHS/Jv/FD estimate 배열이 짧을 때 일부 항을 조용히
생략할 수 있다. 또한 unique point729/FD49라는 개수만으로 사전등록된 key set을
정확히 강제하지 않는다. 이는 현재 데이터가 누락됐다는 주장과 구분한다.
Root가 원 oracle/source/result/log와 corpus를 그대로 보존하고
validate_saved_schema.py를 추가했다. Script 전체와 actual receipt/log를 읽어
정확한 vector/matrix shape,9^3/7×7 key sets, refusal ID/name sets, duplicate JSON
key 거부를 확인했다. 실제 field 수는 RHS1,458/J2,916/Jv1,458/mass729=6,561,
FD98이다. Short RHS, short FD estimate, wrong grid index의 세 malformed-schema
control은 모두 거부됐다. Model/callback/원 numerical oracle 실행은0이다.
**Numerical+schema receipt를 결합해** 이 finding과 전체 C9 판정을 닫았다.

원 raw input은 계속170,379bytes/SHA256
cb64d9455289f183363749289038d6f250bf63e0ab95ea94cd51defa88d2ba21이다.
추가 schema 실행 후 현재 budget의17개 output/log hash가 실제 바이트와 모두
일치한다. 이는 과거 budget을 초기화하거나 완료 callback 실행을 반복한 근거가 아니다.

### 최종 figure 실제 확인

`D5_DERIVATIVE_WITNESSES.png`와 `D5_BOUNDARY_WITNESSES.png`를 실제로 열었다.
90mm 설계(3.55inch,8–10pt)에서 legend·zero markers·축·패널 간격이 읽히고,
180mm 확대에서도 가독성 문제가 없다. 현재 PNG의 clipping/overlap은 발견하지
않았다. Initial annotation 접촉 및 좁은 Outside label은 제거/교정됐으며 추가
plot 실행은 필요없다. 초기 PNG/PDF/source와 그 receipt가 보존돼 있다.

Caption은 derivative A의 x=1/4,y=3/4 slice, B의 각 p당7개 t 오차 최대와49개
stencil 전체, dyadic 제조식에서의 관측오차0을 명시해야 한다. Boundary A의 세
변형은 저장된J의 semantic interpretation이며 B의 finite wrong J1/1만 실제
callback control이다. 본 이미지와 이 의미는 일치한다.
