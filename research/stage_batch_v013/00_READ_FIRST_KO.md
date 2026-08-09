# RODAS5P stage-batch feasibility gate v0.13 — 먼저 읽기

## 결론

이번 cycle은 BDF 최적화를 중단하고 원래 연구 DAG의 첫 blocker였던 **실제 한 timestep 내부의 8-stage 병렬/배치 실행 가능성**만 검사했다.

- 실제 Rayon stage concurrency: 관측됨 (`max_parallel_tasks = 4`).
- component-major 8-stage RHS + common-`W` multi-RHS 조합: 18/18 workload에서 sequential scalar round 대비 `>=1.15x`.
- 3개 pinned campaign의 combined provided-batch median speedup: 약 `2.47x`.
- common-`W` dense multi-RHS microkernel median speedup: 약 `3.48x`.
- cheap scalar RHS/JVP를 stage별 Rayon task로 쪼개는 방식은 중앙값에서 오히려 느림.
- 모든 scientific checksum 동일, 모든 수치 경로의 최대 절대차 0.

## 판정 경계

**PROMOTE:** within-step stage batch API, actual-concurrency test, component-major batch reference, multi-RHS crossover evidence.

**HOLD:** integrated homotopy solver speedup, strict Jacobian-free common-`W` solve, explicit SIMD claim, production solver claim.

현재 common-`W` microbenchmark는 JVP로 dense Jacobian을 untimed setup에서 materialize하고 LU를 재사용한다. 따라서 하드웨어/자료배치의 multiple-RHS 가능성을 보이지만 엄격한 Jacobian-free solver는 아니다.

## 다음 작업

`vectorized-jf-common-w-v0.14`에서 dense setup을 제거하고 다음 네 경로를 같은 tolerance와 JVP-only operator로 비교한다.

1. 8개 독립 GMRES sequential
2. 8개 독립 GMRES Rayon
3. matrix-free block GMRES
4. shared-basis/stage-RHS-augmented Rosenbrock–Krylov comparator

그 뒤 survivor를 `StructuredBlockSystem`과 homotopy `q=0,1,2`에 연결한다.
