# A03-SCALAR-RATE-01 구현 인계

실행 커밋 고정 전 구현만 작성했다. 빌드·테스트·oracle는 NOT_RUN이다.
기존 damped moment API 398행은 byte 변경하지 않았으며 새 adapter만 추가했다.

`NormalizedDampingRate`는 private fields에 κ/h bits, q와 outward
2(q+1)κh interval을 저장한다. `query`는 원본 bits를 대조한다.
`checked_interval_forcing_moment`는 포함성 책임이 caller에 있는 lower-level
arithmetic API이며 arbitrary interval provenance를 인증하지 않는다.

Endpoint kernel은 기존 Taylor/recurrence 식을 counted 연산과 caller scratch로
실행한다. recurrence의 정확한 양성 적분 항등식 E_(m-1)≤1/m을 매 단계
교차시켜 무의미한 중간 과대성장을 차단한다. 이 새 교차에 대한 독립 diff
검토가 필요하다. 결과의 lower/upper endpoint는 monotonicity에 따라 조합한다.
기존 recurrence가 large-rate에서 width gate를 실패한 경우에만 양의 beta-cell
식 A3.8을 사용한다. 두 horizon endpoint의 모든 cell 시도를 합산하며
8191 cap 안에서 uniform depth를 올린다. depth20/leaves4096보다 먼저 total
cell cap이 도달할 수 있다. 좌표 dyadic scaling이 exact하게 증명되지 않는
경로는 partition arithmetic refusal이다.

`interval_primitive_attempts`는 누적 field이고 cap은 10^7이다. 새 wrapper의
Interval add/sub/mul/div 호출과 endpoint exp, directed width 호출을 각각 먼저
1 charge한다. 각 Interval 구현 내부의 directed corner 연산은 중복 합산하지
않는다. min/max/loop/validation은 arithmetic field에 섞지 않는다. 내부 libm
instruction/FLOP 수를 뜻하지 않는다. `allocation_requests`는 adapter가 직접
요청하는 heap 횟수이며 0이다. 하위 directed 오류의 String 할당을 포함한 실제
allocator 횟수는 test의 setup/query_allocations에 별도 실측한다.

S01–S10은 고정 binary64 fixture이며 성공/typed refusal과 소유자·scratch 보존,
누적 work를 검사한다. 양성 대조 3 clone가 3으로 관측되지 않으면 모든 zero
주장은 ALLOCATION_UNAVAILABLE이다. warmed query 전에 동일 fixture를 별도
work로 호출하고 actual 측정은 새 work에 누적한다. 성공과 거절 결과 모두 JSON
으로 내보낸다. oracle는 Fraction으로 exact original rate와 horizon endpoints를
검사한다. degree128,+64,cap2048을 유지한다. 큰 rate의 Q00은 inverse-positive
exponential-series bound를 사용한다.

Controller가 source와 입력을 push하여 EXEC_SHA를 고정한 다음 실행할 명령:

```sh
VIGILODE_SCALAR_RATE_OUTPUT=<absolute-native.json> cargo test --locked --offline -p rodas5p-core --test production_audit_scalar_rate -- --nocapture
python3 research/production_audit_20261018/evidence/continuation_a03_research/scalar_rate_01/fraction_oracle.py <native.json> <oracle.json>
cargo test --locked --offline -p rodas5p-core --test damped_moment_contracts --test damped_moment_enclosure_regressions --test damped_forcing_moment_consumer
```

위 명령은 아직 실행하지 않았다. 새 scalar 결과는 packet/damped trajectory,
high-order, lab phase, physical tolerance, production 또는 speed의 승격이 아니다.
