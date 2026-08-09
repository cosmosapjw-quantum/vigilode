# 다음 cycle — Matrix-Free Common-W Block Gate v0.14

## 중심 질문

Dense Jacobian/LU reference에서 관측된 multiple-RHS 이득을 명시적 Jacobian 조립 없이 재현할 수 있는가?

## 고정 범위

BDF/Radau production optimization은 금지한다. 기존 adaptive BDF1/2와 Radau IIA3는 fixed comparator로만 유지한다.

## Task 1 — matrix-free shifted operator rows

구현:

```rust
trait BatchedLinearOperator {
    fn dimension(&self) -> usize;
    fn apply_rows(
        &self,
        inputs: &[Vec<f64>],
        outputs: &mut [Vec<f64>],
        execution: &ParallelExecution,
        counters: &mut WorkCounters,
    ) -> CoreResult<()>;
}
```

RODAS shifted action:

\[
Wv=Mv-h\gamma J_nv.
\]

필수 조건:

- explicit Jacobian builds = 0;
- factorization builds = 0;
- JVP vector work counted;
- sequential/batched output identity;
- failed batch work retained.

## Task 2 — four equal-tolerance common-W solvers

1. 8 independent GMRES sequential
2. 8 independent GMRES Rayon
3. block GMRES / block Arnoldi
4. shared-basis Rosenbrock–Krylov-style stage-RHS augmentation

모든 경로에 동일한 unpreconditioned true-residual certificate를 적용한다.

## Task 3 — crossover screen

```text
N              32, 128, 512, 2048
RHS rank        1, 2, 4, 8
nonnormality    0, 0.2, 0.9
stiffness       1e2, 1e4, 1e6
threads         1, 2, 4
```

측정:

\[
N_{Jv},N_{Mv},N_{inner},N_{orth},T_{setup},T_{solve},T_{total},M_{peak}.
\]

## Task 4 — survivor integration

Survivor만 다음 code path에 연결한다.

- `StructuredBlockSystem::nonlinear_remainder_snapshot`
- `StructuredBlockSystem::coupling_apply`
- partial-coupling homotopy `q=0,1,2`

## Task 5 — Newton-free adaptive screen

필수 corpus:

- complex Dahlquist;
- oscillatory Prothero–Robinson;
- nonnormal complex block;
- nonsingular mass matrix;
- semilinear parabolic trace.

Acceptance:

\[
E_{\rm emb}+E_{\rm alg}+E_{\rm lin}\le1.
\]

승격 gate:

- explicit Jacobian builds = 0;
- fast-path Newton iterations = 0;
- median propagation rounds <= 4;
- fallback rate <= 10%;
- global order >= 4.8;
- stiff/phase/damping regression 없음;
- same-error holdout wall speedup >= 1.15x.
