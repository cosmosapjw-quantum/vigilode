# D5 코드·검증 독립 검토

## 현재 판정

**PASS_SCOPED_PREEXECUTION_SOURCE; EXECUTION_EVIDENCE_PENDING.** `/root/d5_code_review`가
새 구현의 저자와 분리된 C9 검토자로 source를 읽었다. 이 검토는 하나의 보완적
review pass에 속한다. 실제 compiler·test·corpus 실행은 하지 않았으며, root의
실행 로그와 exact oracle 결과가 합류한 뒤 같은 검토를 닫는다.

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

## 남은 합류

1. 정확한 S1 source와 compiler/version/build flags의 실행 identity.
2. 새30개 gateway+5개 Session 계약시험의 원래 로그와 종료 상태.
3. 원본794 JSONL rows 및 독립 exact Fraction oracle 결과.
4. 저장된 결과만 이용한 figure와 실제 image 확인.

이 검토자의 신규 compile/test/solver/benchmark/NCP/CAS/Desktop Commander 실행은0이다.
독립 personnel은 NOT_RUN이며 production HOLD, global completion=null을 유지한다.
