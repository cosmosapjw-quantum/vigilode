# U07 명시적 method adapter (PPSW1) — 2026-11-16

판정: **U07 PARTIAL**. Peer의 facade end-to-end 경로·거부·접두 보존·작업 회계는 구현·검증했다. 계약의 "independent review"는 구현자 자신이 할 수 없어 **NOT_RUN**이다. BDF/Radau facade adapter와 method 전환(전이 인증서)도 범위 밖이다. 기본 라우팅은 바꾸지 않았다.

## 구현 (`crates/vigilode/src/selection.rs`, glob export, `lib.rs`·`solve.rs` 무변경)
- `PeerOde::solve(problem, span, y0, options, allowed)`: U06 registry로 먼저 admission한다. mass·events·권한(per-step/startup)·미자격 거절은 모든 callback 전에 typed `PeerSolveError::Refused`로 반환한다.
- facade `Problem`을 Peer 부분집합(`Jacobian::Dense` 또는 `Jacobian::Jvp`, band 없음, identity mass)만 직접 하강한다. `solve.rs::build`는 private이고 allowlist 밖이라, FD·band 문제는 하강하지 않고 `UnsupportedJacobian`으로 거부한다.
- callback 오류: `Recoverable` → NonFinite(시도 거절, 보폭 반감), `Fatal` → typed 실패, 그때까지 접수된 접두(t, y) 보존.
- 끝점: `h0 = min(규칙 또는 사용자 h0, (t_end − t0)/2)`라 starter가 t_end를 넘지 않는다. 마지막 step은 t_end에 정확히 맞춘다.
- 작업: startup(h0 규칙 + starter)과 step(접수·거절 시도 전체)을 별도 counter로 반환한다.
- method 소유 상태: Peer 6-stage 이력은 `PeerOde` 안에만 있다. `switch_history_to`는 `MethodSwitchUnsupported`를 반환한다(전이 인증서 없음).

## 검사 (`unified_method_sessions.rs` 6/6)
- e1: 비정규 stiff PR(2×2)에서 dense/JVP × 1/4 thread가 모두 끝점에 정확히 도달한다(오차 2.9e-11). startup에만 Newton이 있고 step은 Newton-free다. JVP 경로는 step마다 n개 JVP로 dense J를 만든다(step_jvp 286 = 143×2).
- e2: mass, events, Newton 미허용, dense 미허용, FD 전용, band 선언은 모두 callback 0회로 거부된다.
- e3: 구간 1e-3에 h0=0.5를 요청하면 h0 ≤ 5e-4로 줄고 끝점 = t_end다.
- e4: t>0.5에서 Fatal → success=false, 접두는 t≤0.4985, 실패한 시도의 RHS까지 계수된다.
- e5: Recoverable 1회 → 거절 1회 후 성공.
- e6: 전환 거부, `GeneralOde::select`는 여전히 RODAS5P를 고른다.
- 호환성: vigilode 전체 331 통과/0 실패, strict clippy, fmt 통과.

## 남은 의무
독립 review, BDF/Radau adapter(기존 observed driver 경로 사용), 이벤트·dense output, method 전환 인증서(U10), FD/band 하강(`solve.rs::build` 재사용에 allowlist 확대 필요).
