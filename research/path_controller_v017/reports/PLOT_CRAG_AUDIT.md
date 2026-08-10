# Plot-Based CRAG Audit — Path Controller v0.17

## Generated evidence

1. `ACCEPTANCE_VS_BATCH_DEPTH.png`
2. `RESIDUAL_REDUCTION_VS_VECTOR_WORK.png`
3. `TARGET_RESIDUAL_BY_ROUND.png`
4. `ORACLE_DEFECT_BY_CASE.png`

## Plot reading

### Acceptance versus batch depth

- q=7은 100% non-hostile coverage지만 batch depth 32다.
- `direct-decoupled-q2-c2`는 depth 8에서 같은 100% coverage를 보인다.
- `direct-decoupled-q1-c2`는 depth 6에서 80% coverage다.
- strict depth < 8에서 100% coverage인 schedule은 없다.

### Residual reduction versus vector work

- fixed q=0/1/2 path marching은 work를 늘려도 median target residual을 거의 줄이지 못한다.
- endpoint correction 계열은 같은 q에서 여러 orders of magnitude 더 큰 reduction을 보인다.
- q=7은 가장 정확하지만 vector work 256으로 과도하다.

### Residual by round

- theta ramp는 path가 진행될수록 residual이 증가한다.
- fixed q2는 세 round 후에도 거의 정체한다.
- one-round q1/q2 two-correction schedules는 endpoint에서 급격한 residual 감소를 만든다.

### Oracle defect by case

- q1/c2는 PR에서 algebraic budget을 크게 넘는다.
- q2/c2는 모든 non-hostile case에서 budget 아래다.
- q2/c2와 q7의 output defect는 네 문제에서 거의 oracle floor이며, PR에서도 q2/c2가 0.1 budget을 통과한다.

## CRAG

### C — Correctness

- serial sequential oracle와 output WRMS를 직접 비교했다.
- false acceptance 0.
- hostile case는 통과한 것처럼 숨기지 않았다.

판정: PASS for bounded one-step claims.

### R — Retrieval / prior-art contrast

결과는 classical continuation보다 endpoint preconditioned correction에 가깝다. Parallel-iterated RK와 parallel SDC의 correction-sweep 계보와 비교해야 하며, 단순히 새 homotopy method라고 주장할 수 없다.

판정: claim narrowed.

### A — Augmentation

아직 다음 축이 없다.

- long trajectory
- step-size ladder
- global order
- matrix-free common-W backend
- same-error timing
- broader oscillatory frequency/stiffness grid

판정: further evidence required.

### G — Generation

Plot이 지지하는 다음 예측은:

> q1/c2를 기본 fast attempt로 쓰고, output/residual gate 실패 시 endpoint에서 q2 correction을 추가하면 median depth 6에 가까운 broad-coverage controller가 가능할 수 있다.

이건 예측이지 검증 결과가 아니다.

## Adversarial mutations

| Mutation | Result |
|---|---|
| q: 0→1→2 | accuracy improves but fixed path still stagnates |
| corrections: 0→1→2 | endpoint corrections dominate improvement |
| path length: 3→2→1 | shorter endpoint jump can outperform path marching |
| theta: fixed→ramp | ramp causes severe amplification |
| nonnormality: moderate→hostile | all candidates fail budget; fallback required |

## Claim status

### Surviving

- endpoint-correction schedule is the strongest bounded low-q family.
- q2/c2 attains non-hostile correctness at sequential-depth parity with 1/4 q7 work.

### Narrowed

- homotopy path marching itself is not the source of improvement.
- q1 is a speculative fast lane, not a universal solver.

### Rejected

- nonstationary theta ramp is stabilizing.
- strict sub-eight-batch full-coverage method has already been found.
- current results prove global speedup.
