# G1 PHYS–MATH AUDIT

## Conventions, units, signs

- Metric convention은 이 regular-ODE numerical gate에서 직접 사용되지 않는다.
- Test variables와 independent time은 dimensionless다.
- Constant-mass case의 \(M\)은 nonsingular이며 \(MJ\ne JM\)인 경우를 포함한다.
- Shifted operator sign은 protected RODAS convention과 동일하다:
  \[
  W=M-h\gamma J.
  \]
- \(q=1,2\)는 causal-propagation truncation depth이지 시간차수나 물리차원이 아니다.
- WRMS certificate와 embedded error는 무차원이다.

## PASS

1. fast path의 explicit-J build, direct factorization, Newton iteration은 정확히 0이다.
2. q1 endpoint/state/work가 q2 escalation에서 보존된다.
3. 실패한 speculative work는 fallback 비용에서 삭제되지 않는다.
4. protected fallback은 original matrix-free RODAS endpoint를 복구한다.
5. finite one-step corpus에서 protected output 대비 false accept는 0이다.
6. stiff damping, complex oscillation, nonlinear relaxation, nonnormality, semilinear traces,
   kinetics, noncommuting mass를 모두 포함한다.
7. post-hoc clipping이나 state fitting은 없다.

## Ranked concerns

### P1 — 핵심 가속 목표 실패

Step count는 보존됐지만 adaptive accepted step의 절반 이상이 fallback이고 median/P95 depth가 16이다.
Critical path와 wall time을 줄인다는 중심 목적을 달성하지 못했다.

### P1 — operational certificate는 theorem이 아님

관측된 output-correction contraction이 실제 nonlinear tail을 지배한다는 보장은 없다. Nonnormal
amplification과 Jacobian variation은 이 추론을 깨뜨릴 수 있다.

### P1 — fast-path 독립 5차 결과 없음

Adaptive result는 q2와 fallback의 혼합이다. 따라서 method-level fifth-order claim은 허용되지 않는다.

### P2 — small-state threading economics

현재 n≤32 adaptive corpus에서 thread overhead가 계산량보다 크다. 더 큰 systems에서 crossover가
가능하지만, 5.125× RHS와 8.534× JVP overhead를 뒤집는다는 증거는 없다.

### P2 — common-grid output policy

동일 policy가 양쪽 RODAS lane에 적용되므로 negative relative comparison은 유효하다. 다만 high-order
candidate의 독립 tolerance/order 해석에는 endpoint-only lane이나 dense output가 추가로 필요하다.

## Verdict

- Mathematical consistency and transaction semantics: PASS WITH BOUNDED EVIDENCE.
- Generic low-depth acceleration: FAIL.
- Independent fifth-order fast solver: NOT ESTABLISHED.
