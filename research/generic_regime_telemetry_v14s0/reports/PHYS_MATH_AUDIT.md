# G4-S0 PHYS–MATH AUDIT

## 정의·표기

- `E-K`: explicit-J-free, Newton-free fused exponential Rosenbrock/Krylov lane.
- `R-JF`: protected matrix-free RODAS5P lane.
- `C-JF`: held q1→q2 causal endpoint corrector.
- `R-J`: frozen Jacobian-based Radau IIA3 comparator.
- 모든 speedup은 protected R-JF 기준이며 dimensionless다.

## 단위·차원

External endpoint error는 state의 Euclidean norm이며 current generic problems는 dimensionless test systems다. Wall time은 seconds, step count와 work counts는 dimensionless counts다. 서로 다른 dimensional quantities를 임의 scalar score로 합치지 않았다.

## known-limit consistency

- linearization-exact complex Dahlquist에서 E-K 우위가 관측됨.
- nonlinear oscillatory PR에서 R-JF 우위가 관측됨.
- 이는 G3의 직접 결과와 정성적으로 일치한다.
- same-error target은 protected RODAS error level로 고정하고 candidate tolerance를 별도로 선택했다.

## 잠재적 반례

- Dimension threshold의 100% in-sample fit은 method physics가 아니라 current problem-family/dimension confounding으로 설명 가능하다.
- Frozen Radau의 auto-oracle 우위는 explicit-J/direct small-dense setting에 의존하며 large sparse setting에서 유지되지 않을 수 있다.
- Endpoint-only error target은 trajectory-wide phase/amplitude error 또는 event fidelity를 완전히 대표하지 않는다.
- Switching 자체는 아직 실행되지 않아 switch-induced order loss를 검증하지 않았다.

## 판정

- Oracle complementarity: PASS for current frozen corpus.
- Single-feature regime classifier: FAIL / insufficient generalization.
- Active method switching: NOT YET ADMISSIBLE.
