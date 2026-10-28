# A02-CONTRACT-SUPPLEMENT-02: overflow 입력과 구현 선택의 제한 보완

상태: **READY_FOR_FRESH_DELTA_REVIEW**, `implementation_approved=false`.
이 문서는 supplement_01을 대체하여 과거 기록을 고치는 문서가 아니다.
기존 계약과 입력을 해시로 보존한 채, 아래 세 항목에만 적용하는 새 계약
overlay다. 독립 검토 이후의 구현·수치 실행·과학적 채택은 아직 수행하지
않았다. 다른 입력·모델·물리 tolerance·reference 차수·자원 제한은 그대로다.

## 1. 입력 버전과 기존 증명의 재사용

기준 계약은 `../../../A02_VERIFIED_TRAJECTORY_CONTRACT.md`, 기준 입력은
`../supplement_01/FROZEN_ACCEPTANCE_INPUTS.json`이다. `INPUT_HASHES.json`은
그 기존 바이트와 새 `FROZEN_ACCEPTANCE_INPUTS_V2.json`의 별도 해시를
기록한다. 기존 supplement_01의 어떤 파일도 수정하지 않는다.

기존 계약 §2–§6(D1–D5)의 바이트 SHA-256은
`ecc8841db6ad2b2f40c7245102372e31e31120e38ecdb68b91ed0cfa4ddcdcb5`다.
이번 작업은 그 증명 부분을 수정하지 않는다. T1, rotating-frame
`omega=exact(width)*exact(xi_j)`, metadata-only carrier, 실제 저장된 양의
가중치 합 S, 원래 초기 energy bound와 outward `S*E_global <= 1` admission을
재사용한다. `T_FIXED=1e-5/1e-5`, `T_INHERITED=1e-4/1e-4`,
`T_REJECTION=2e-4/2e-4`의 비트를 그대로 유지한다. AT02의 새 trajectory
tolerance 출처 구분도 바꾸지 않는다.

직전 검토의 HOLD 원인은 parent가 전달한 `OVERFLOW_OMEGA` fixture의
admission-before-multiplication 문제다. 직접 저장된 직전 reviewer artifact는
이 단위 시작 시 제공되지 않았다. 이 전달은 이전 검토의 승인으로 쓰지
않는다. 이번 overlay와 v2 입력에 대한 fresh read-only delta review가 다음
단계다.

## 2. AT08: admission을 통과하는 overflow witness

v1의 `OVERFLOW_OMEGA`는 width가 `0x7fefffffffffffff`, 세 xi가 모두
`0x4000000000000000`(2)였다. 현재 `CollectiveModel::new`의 detuning
admission `-1-1e-14 <= xi <= 1+1e-14`에서 먼저 거절된다. 그러므로 이
입력은 typed **domain** refusal의 증거이지 multiplication overflow의
실행 증거가 될 수 없다. 원래 입력과 기대했던 옛 문구는 v1 파일 그대로
보존한다. v2에는 같은 입력을 `OVERFLOW_OMEGA_V1_DOMAIN_REFUSAL` arm으로
명시하여 `DetuningOutOfRange`를 기대한다. 이 arm의 거절은 numerical
inclusion이나 arithmetic-overflow coverage로 집계하지 않는다.

v2의 `OVERFLOW_OMEGA`는 width 비트를 그대로 두고 xi 세 값만
`0x3ff0000000000008`로 바꾼다. 각 값의 정확한 실수는

`x = 1 + 8*2^-52 = 1 + 2^-49`.

현재 admission upper endpoint `fl(1 + 1e-14)`는
`0x3ff000000000002d`(1+45*2^-52)이므로 이 x는 그 안에 있다. 모델의 다른
값은 F_FIXED 그대로다. admission upper endpoint에 nominal support 1을
대입해 xi를 clamp하지 않는다.

최대 유한 양의 binary64 width M의 정확한 값은

`M = (2^53 - 1)*2^971 = (1 - 2^-53)*2^1024`.

따라서 정확한 곱은

`M*x = (2^53 - 1)*(2^49 + 1)*2^922`

`    = 2^1024 + (2^53 - 2^49 - 1)*2^922`

`    = 2^1024 + (15*2^49 - 1)*2^922 > 2^1024`.

마지막 항은 양의 정수와 양의 2의 거듭제곱의 곱이다. 이 증명에는
rounded `width*xi`를 전제로 사용하지 않는다. nearest multiplication을
그대로 쓰면 +Inf가 생기는 범위이며, outward multiplication 또는 이를
소비하는 spectral/speed certificate 경로는 typed arithmetic refusal를
반환해야 한다. 입력 admission을 통과했다는 사실과 실제 산술 경로를
관측했다는 사실은 별도로 기록한다. 어떤 앞선 유효 검사가 typed
arithmetic refusal를 반환하면 해당 실제 지점을 보고하며 matvec 등
도달하지 않은 연산의 coverage를 주장하지 않는다. 유한 Ok certificate는
허용하지 않는다. 이 문서는 그 미래 실행 결과를 주장하지 않는다.

## 3. DC1: 반환 중심 displacement의 immutable accessor

기존 §6의 구현 선택은 **(a)의 좁은 accessor**로 고정한다. 두 immutable
view `PacketCurveOutputView`와 `PacketCertifiedView`에
`weighted_center_displacement(&self) -> f64`를 제공하도록 구현한다.
값은 public input이 아니라 성공한 같은 `enclose_at` 호출이 만든 private
workspace/view scalar다. wrapper는 inner의 같은 값을 읽기만 한다.
외부 생성·수정·재결속 경로는 제공하지 않는다.

현재 query가 같은 exact represented curve의 interval evaluation과 실제
반환 binary64 mode 중심 C_j로부터 계산하는
`delta_j >= |Y_j(u)-C_j|`를 그대로 재사용한다. 같은 mode-loop에서 실제
저장된 `w_j`를 써서

`s_0 = 0`,

`s_{j+1} = add_up(s_j, mul_up(w_j, mul_up(delta_j, delta_j)))`,

`delta_w = sqrt_up(s_N)`

를 계산하고, 완료된 query view에 불변으로 저장한다. directed helper의
finite/underflow/overflow 계약과 typed refusal를 그대로 따른다. exact-zero
shortcut을 새로 가정하지 않는다. `|Y_j-C_j| <= delta_j`와 양의 w_j에서
`||Y-C||_w^2 <= sum(w_j*delta_j^2) <= s_N`이므로 반환 delta_w가 필요한
상계다. 중간 scalar나 불완전 workspace로 성공한 view를 만들 수 없다.

기존 `weighted_radius()`의 의미와 계산은 바꾸지 않는다. 그것은
`B_local`, 즉 exact frozen polynomial 주위의 base T1 weighted radius다.
새 accessor는 그 B_local, inherited seed, temporal lift, flow gain이나
marginal radii를 더한 값이 아니다. 기존 mode/mean 중심과 반경 계산을
바꾸지 않고 새 scalar만 만든다. A02 endpoint는 같은 view의 같은 C_j를
commit하고

`e_next = add_up(B_local, G*e_k, K_time, delta_w)`

를 기존 §6대로 outward 구성한다. marginal radii의 norm으로 delta_w나
e_next를 대체하지 않는다. 기존 §6의 선택 (b)는 이번 구현 선택에서
사용하지 않는다.

새 집계는 N mode마다 outward multiply 두 번과 add 한 번, 마지막 sqrt
한 번이다. 현재 helper-call accounting 정의에서 추가
`curve_output_certificate_ops`는 성공 경로 `3*N+1`이며 각 시도한 helper를
실패 경로에서도 누적한다. getter는 재계산·할당을 하지 않는다. setup과
trajectory seed 복사 등 다른 counter의 최종 정수 tally는 구현 source에서
실행 전에 별도 동결한다. 기존 source-bound exact counter 기대값은 새
source와 연결해 갱신하며, 관측한 실패에 맞춰 사후 조정하지 않는다.
추가 집계는 기존 delta를 재사용하여 중복 polynomial evaluation과 새 Vec
할당을 만들지 않는다. 향후 검증은 새 accessor의 exact bound 포함,
기존 반환 중심/반경 bits 보존, immutable field compile-fail, typed refusal,
정확한 work delta를 다룬다.

## 4. AT06: 첫 window의 두 평가 순서

F_FIXED/C_FIXED의 원래 requested grid와 initial_h는 바꾸지 않는다.
첫 trial window는 global `[0,1/16]`이고, 그 안의 requested point는
`1/32` 하나다. 그 다음 requested `3/32`는 이 window 밖이다. transaction의
평가 순서를 다음과 같이 고정한다.

1. `query_index=0`: requested global `1/32`, bits `0x3fa0000000000000`.
2. `query_index=1`: mandatory endpoint global `1/16`, bits `0x3fb0000000000000`.

두 번째 값은 requested grid에 새로 삽입하는 관측시각이 아니라 commit에
필수인 endpoint evaluation이다. 순서는 requested interior 다음 endpoint다.
`TXN_FAIL_FIRST`는 index0 평가 전에 기존 typed OutwardRefusal를 반환한다.
`TXN_FAIL_SECOND`는 index0의 정상 query를 완료한 뒤, index1 endpoint
평가 전에 같은 typed refusal를 반환한다. 첫 성공한 partial query의 work를
포함해 모든 시도 work가 남고 accepted state/cursor/history/retained windows는
bitwise 보존되어야 한다. 이 seam은 test-private이며 numerical inclusion
arm으로 집계하지 않는다. 실제 F_REJECTION의 h=1/4 tolerance rejection,
두 commits, accepted-size replay, unchanged tolerance는 그대로다.

## 5. 다음 실행점과 범위

새 v2 manifest와 이 overlay의 fresh independent delta review가 READY이면
하위 구현자가 위 accessor 및 A02 trajectory를 구현할 수 있다. 이 파일은
그 권한을 자체 부여하지 않는다. source/preregistration checkpoint 게시,
입력 해시 동결, source-derived counter tally, 기존 AT01–AT12의 표적 실행과
독립 판정이 뒤따른다. 첫 실패·양성 allocation control·기존 L-0124 mismatch
보존 의무를 유지한다. 현재 code/test/build/solver 실행은 모두 NOT_RUN이며
A02 및 전체 production 판정은 HOLD다.
