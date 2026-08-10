# Next Cycle — Matrix-Free Endpoint-Correction Controller v0.18

## Goal

`direct-decoupled-q1-c2`와 `direct-decoupled-q2-c2`를 strict matrix-free common-W backend에 연결하고, q1 failure에서 q2 correction만 추가하는 transactional endpoint controller를 adaptive trajectory에서 검증한다.

## Fixed comparators

- protected sequential JF RODAS5P
- SABR5P
- q7 correctness oracle
- frozen adaptive BDF2
- frozen adaptive Radau IIA3

BDF/Radau source optimization은 금지한다.

## Tasks

1. `EndpointCorrectionController`를 추가한다.
   - q1/c2 first attempt
   - residual/output gate failure 시 same endpoint에서 q2 correction 1회 추가
   - still failing/nonfinite이면 protected sequential JF fallback
2. v0.14 strict matrix-free common-W backends를 연결한다.
   - independent Rayon GMRES protected control
   - telemetry-admitted block/seeded speculative lane
   - true residual certificate
   - failed speculative work preserved
3. fast path에서 다음을 강제한다.
   - explicit Jacobian builds = 0
   - direct factorizations = 0
   - Newton iterations = 0
4. trajectory gates를 실행한다.
   - complex Dahlquist frequency/stiffness grid
   - oscillatory nonlinear PR
   - moderate/hostile nonnormal
   - noncommuting mass
   - step-size ladder and adaptive run
5. same external global error에서 fixed comparators와 randomized repeated timing을 수행한다.

## Promotion gates

- false convergence = 0
- non-hostile fast fraction ≥ 0.90
- protected fallback fraction ≤ 0.10
- median common-W batch depth < 8 preferred; ≤ 8 required
- global observed order ≥ 4.8
- stiff decay and phase/amplitude regression pass
- same-error median speedup over sequential JF RODAS5P ≥ 1.15
- T1/T4 scientific identity

## Kill rule

q1→q2 escalation이 PR/oscillatory holdout을 살리지 못하거나, matrix-free operator work가 sequential JF baseline을 초과한 채 wall speedup이 없으면 endpoint controller를 HOLD하고 parallel SDC/iterated-RK comparator로 이동한다.
