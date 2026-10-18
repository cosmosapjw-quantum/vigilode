# 2026-10-18 production readiness continuation — 결과 보고서

## 실행 identity와 환경

- 브랜치: `audit/production-readiness-20261018`
- 실행 커밋(EXEC_SHA): `bbb9bb4aaa9e8748109cee7b439ea2828015de8b`
- 해당 커밋은 실행 전 원격 브랜치에 게시했고 fetch한 origin ref와 SHA가 일치했다. 이어진 검증 전용 커밋은 별도로 게시 예정이다.
- 시작 감사 commit `6c067105f0b9e16cbfd397b2d5316673e698e00e`, 감사 대상 source `2fc61b944fe431d40ba455c373579bce00b0c39a`.
- `rustc 1.94.1`, `cargo 1.94.1`, `x86_64-unknown-linux-gnu`; 모든 cargo 테스트/검사는 `--locked --offline`. `Cargo.toml`/`Cargo.lock`은 수정하지 않았다.
- 순수 preregistration, 고정 계약 및 source 입력 SHA-256은 `evidence/continuation_inputs_sha256.json`에 기록했다. 원시 결과와 첫 실패/수정 로그는 `evidence/continuation_*` 아래 유지했다.

## 구현과 검증

- **A01:** 실제 저장 `S`와 `max|xi|`를 사용하도록 진단 tail 분리, generated identity 검사, 비용 항목, exact-linear one-mode와 100-digit mpmath two-mode 중간/endpoint oracle 및 비유한/overflow refusal regressions를 보강했다. 독립 반례로 `S<1` mode-bound 누락을 고쳐 재검증했다. `roundoff_enclosed=false`를 유지한다. 일반 복소 계수·force·residual·tail·재구성 산술의 외향 포함과 반환 중심-반경 결속이 없어 **A01 HOLD**. 비용 카운터도 clone/setup/allocator 등 전체 비용이 아니다.
- **A04:** warmed scalar 경로에서 30회 발생하던 allocation을 고정 배열·외향 계수 생성으로 제거했다. allocator 카운터를 thread-local로 바꾸고 동시 격리 회귀를 넣었다. 독립 검토가 `c=0`의 `1/6`, `1/24` 점구간 결함을 찾아 `div_down/div_up` 경계와 직접 반경 회귀로 고쳤다. 성공한 동일 capacity/cache prewarmed 경로로 무할당 문구를 좁혔다. 직접 반경 1/1, Leja 통합 14/14, 문서 13/13, compile-fail 캐시 계약 1/1 및 독립 review scoped PASS. 그래도 operator action, recurrence, coefficient-weighted sums, output scaling/fusion에 additive-underflow enclosure가 없어 **전체 Leja는 EstimateOnly/HOLD**.
- **A05:** 입력 shape 검증 다음 실제 callback 시도에 RHS work를 charge하고 Err/비유한 출력/배치 fallback을 보존한다. 잘못된 shape는 callback/work가 실행되지 않도록 고쳤다. 관련 테스트 4/4. 연속 residual과 전 tube one-sided gain은 없어 **global accuracy HOLD**.
- **A06:** 회전 좌표계, `carrier=0`, `omega=width*xi`에 한정된 O(N) rank-two Woodbury `solve_into`, 2×2 Schur refusal, source/state binding과 외향 residual/error witness를 구현했다. 독립 검토 scoped mathematical PASS, 관련 테스트 5/5. setup에는 heap buffers가 있고 비용은 측정값이 아닌 보수적 proxy이며 end-to-end trajectory/speed가 없어 **cost/performance HOLD**.
- **A11:** Gray-Scott reaction-block symmetric-part 최대 고유값과 주기 경계 diffusion의 비양성 대칭 성질로 supplied componentwise boxes 상의 `mu_2` 상계를 구현했다. 독립 scoped PASS, 테스트 3/3. trajectory boxes, callback roundoff, global defect를 만들지 않는다.
- **통합 표적:** A01/A05/A06/A11 및 packet 경계 관련 적분기 테스트 44/44; damped regression과 Leja 계약 18/18 (EXEC `e974801…`); 최신 A04 변경을 포함한 Leja 14/14, 직접 계수 반경 1/1, 문서 13/13, workspace all-target check와 fmt check 모두 exit 0 (EXEC `bbb9bb4…`). 범위는 표적 회귀이며 전체 workspace `cargo test`를 실행한 기록이 아니다.

## DAG 및 과거 finding 판정

`CONTINUATION_DAG.json`과 `CONTINUATION_FINDINGS.json`이 machine-readable 상태다. 요약하면 A06/A11은 제한된 수학/구현 경로의 PASS, A01/A04/A05/A07/A12는 전체 수용 조건 HOLD, A02/A03/A08/A09/A10은 선행 의무 미충족으로 BLOCKED다.

- **FIXED:** PK-01, PK-03–06, SA-01–05, DP-02–03. `PK-02`, `SA-02`는 잘못된 authority를 제거/보정했지만 total certificate는 계속 HOLD다.
- **OPEN 또는 PARTIAL_OPEN:** PK-07–10, SA-06, PA-02, PA-04–07, DP-01, DP-04.
- **SUPERSEDED_SCOPE:** PA-03의 LP10 broad declaration은 독립 증거로 채택하지 않고 후속 범위 decision으로 철회했다. benchmark는 `DIAGNOSTIC_ONLY`다.
- **수학 범위 제한:** DP-01/04와 generic PR witness의 dense-output 정규화 실패는 연속 인증으로 해결되지 않았다. 기존 49/52 timing 불일치는 교정 비교로 포장하지 않았고 캠페인도 반복하지 않았다.

## 다음 실행점

1. A01에서 generated candidate, force, full residual, tail, 모든 output reconstruction을 outward interval로 구성하고 returned center와 radius를 immutable source identity에 결속; 독립 exact/high-precision, subnormal/overflow/NaN/Inf 및 setup/output work 검증. 이 전까지 A02/A03/A10은 시작하지 않는다.
2. A04에서 additive-underflow를 operator multiply/accumulate, Newton recurrence, coefficient sum, final scaling/fusion 전 과정에 enclosure로 전파; EstimateOnly 해제 금지.
3. A05는 실제 연속 residual과 검증된 one-sided tube gain API를 먼저 설계/검토하고 새 101-output PR witness를 전 시각/연속구간에서 검증.
4. A07은 이전 campaign을 반복하지 말고 방정식·atol vector·output grid·independent reference·setup/cold/warm 경계·실제 allocator·실패 arm·build/source identity를 사전 동결한 새 matched campaign을 만든다. A08/A09는 A04+A07이 통과할 때만 진행.
5. 연구 ledger 7개 게시 게이트 결과를 아래에 기록한다. 게시 성공은 과학적 production 승격과 별도다.
