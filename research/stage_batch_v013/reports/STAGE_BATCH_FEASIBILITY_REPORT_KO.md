# RODAS5P Stage-Batch Feasibility Gate v0.13

Date: 2026-08-09
Base: `adaptive-all-methods-v0.12.0-alpha1`
Branch: `vectorized-jf-stage-batch-v0.13-reset`
Scope: within-step vectorization/batching feasibility only

## 1. Current DAG state

원래 중심 질문은 RODAS5P의 8-stage lower-triangular nonlinear dependency를 저깊이 homotopy/structure propagation으로 압축하고, stage RHS/JVP/common-`W` 연산을 실제 batch로 실행해 Jacobian-free stiff/oscillatory solver의 wall time을 줄일 수 있는가이다.

이미 완료된 선행 조건:

- pure-Rust same-kernel baseline;
- protected sequential/adaptive RODAS5P;
- SABR and partial-coupling homotopy reference paths;
- fixed-step fair-measurement authority;
- all-current-method adaptive reference API.

이번 cycle의 유일한 선택 node는 `actual stage-batch concurrency`였다. BDF variable-order/Nordsieck/lagging 연구는 comparator archive로 동결했다.

## 2. Contract reconstruction

### Physical/numerical contract

Metric, units, and cosmology conventions are irrelevant to this numerical microkernel. The ODE contract is

\[
M\dot y=f(t,y),\qquad
W=M-h\gamma J_n.
\]

Two dimensionless benchmark families are used:

1. real 2-by-2 representation of complex Dahlquist modes,
   \[
   y'=(-\sigma+i\omega)y;
   \]
2. oscillatory Prothero–Robinson-like nonlinear modes,
   \[
   y'=\lambda(y-\phi(t))+\phi'(t)+\mu(y-\phi(t))^3.
   \]

The signs are chosen so \(\sigma>0\) and \(\lambda<0\) produce damping/stiff attraction.

### Code contract

- 8 stage rows enter in deterministic order.
- Sequential and Rayon paths call the same scalar physics closure.
- A supplied batch callback uses component-major traversal across 8 stages.
- One frozen JVP operator is shared across stage rows, matching Rosenbrock's step-frozen \(J_n\).
- common-`W` single-RHS and multi-RHS solves reuse the same LU factorization.
- Timing excludes problem construction, Jacobian materialization, and factorization setup; it measures the repeated stage kernels only.
- Numerical outputs must agree exactly in binary64 for the chosen deterministic kernels.

### Honesty contract

Machine-readable fields explicitly state:

```text
rhs_and_jvp_paths_matrix_free              true
common_w_dense_reference_setup_used        true
strict_jacobian_free_common_w_demonstrated false
explicit_simd_demonstrated                 false
```

## 3. Implemented execution paths

### RHS

- `sequential-scalar`
- `rayon-stage-2`
- `rayon-stage-4`
- `provided-component-major`

### Frozen JVP

- `sequential-scalar`
- `rayon-stage-2`
- `rayon-stage-4`

### common-W solve

- eight single-RHS LU solves
- one multi-RHS LU solve

### combined round

- sequential scalar RHS + sequential JVP + single-RHS solves
- component-major batch RHS + sequential JVP + multi-RHS solve
- Rayon stage RHS/JVP + multi-RHS solve

The concurrency probe inserts a bounded sleep in eight RHS tasks and records simultaneous active tasks. It observed four concurrent tasks in the four-thread local pool.

## 4. Experiment matrix

```text
families       complex-dahlquist, oscillatory-prothero-robinson
dimensions     32, 128, 512
work repeats   1, 8, 32
stages         8
campaigns      3
inner samples  7 per row
CPU affinity   cores 0-4
library threads 1
```

Each campaign generated 18 cases and 234 timing rows. Scientific fields were identical across all three campaigns.

## 5. Canonical results

Scientific checksum:

```text
be5a3015bb0fc2085659ac87ce91518822b8b1312cd9effd05bd7b9fa8c9523b
```

All rows had maximum absolute numerical difference exactly 0 relative to the sequential reference.

### Kernel-level campaign medians

| Kernel | Candidate set | Median speedup | Maximum | rows >=1.15x |
|---|---|---:|---:|---:|
| RHS | all nonserial paths | 0.396x | 1.162x | 2/54 |
| JVP | Rayon paths | 0.114x | 0.954x | 0/36 |
| common-W | multi-RHS LU | 3.479x | 3.723x | 18/18 |
| combined | all nonserial paths | 1.678x | 3.308x | 30/54 |

### Combined backend medians

| Backend | Median | Min | Max | rows >=1.15x |
|---|---:|---:|---:|---:|
| component-major batch + multi-RHS | 2.473x | 1.601x | 3.308x | 18/18 |
| Rayon-2 stage + multi-RHS | 0.931x | 0.195x | 2.563x | 6/18 |
| Rayon-4 stage + multi-RHS | 0.461x | 0.066x | 2.108x | 6/18 |

Rayon stage execution becomes beneficial only at the largest tested state dimension, where each task has enough work to amortize scheduling and per-row allocation. Four threads are usually worse than two because only eight stage tasks exist and the scalar kernels are still too fine-grained.

## 6. Interpretation

### Surviving claim

There is genuine, measurable within-timestep batching opportunity. The strongest evidence is not thread-per-stage execution but

\[
\boxed{
\text{component-major stage batch}
+
\text{common-}W\text{ multi-RHS solve}
}
\]

which passes the 1.15x microkernel gate in all 18 workloads.

### Narrowed claim

Rayon can expose true stage concurrency, but it is not a universal acceleration mechanism. For cheap RHS/JVP kernels, task overhead dominates. It becomes useful only when dimension or per-stage physics cost is sufficiently large.

### Rejected claim

This cycle does not demonstrate a strict Jacobian-free solver. Dense \(J\) and \(W\) are materialized during untimed setup for the LU multiple-RHS reference. It also does not demonstrate explicit SIMD, end-to-end homotopy speedup, fifth-order preservation of a new path, or reduced total adaptive integration time.

## 7. Relation to prior art

- Iterated RK methods show that implicit stage dependency can be traded for a small number of parallel iterations, with sequential depth determined by iteration count rather than total stage count.
- Stage-parallel IRK implementations use stage subgroups or batch operations and report that attainable speedup is bounded by the number of stages.
- Parallel SDC demonstrates that diagonal/nonstationary preconditioner coefficients determine whether stage/node parallelism is useful or erased by extra sweeps.
- Matrix-free monolithic homotopy provides a residual-only/no-linear-system continuation precedent, but its role is generic nonlinear globalization rather than preservation of RODAS5P order and stiff accuracy.
- Rosenbrock–Krylov is the principal Jacobian-free comparator because it co-designs temporal order and a shared Krylov approximation.

## 8. Verdict

### PROMOTE

- actual within-step stage-concurrency API;
- deterministic ordered Rayon stage execution;
- component-major stage RHS batch reference;
- common-W multiple-RHS crossover evidence;
- numerical identity and repeated timing protocol;
- explicit honesty fields separating matrix-free and dense-reference lanes.

### HOLD

- strict Jacobian-free common-W solve;
- explicit SIMD claim;
- integrated q=0/1/2 homotopy speedup;
- production/default solver change;
- sparse/PDE/GPU/DAE extrapolation.

## 9. Recovery note

An earlier transient workspace was lost to runtime reset. No result from that lost workspace is used as final evidence. The branch was reconstructed from the durable v0.12 source bundle, and all tests, three canonical campaigns, analyses, and plots reported here were rerun on the reconstructed tree.
