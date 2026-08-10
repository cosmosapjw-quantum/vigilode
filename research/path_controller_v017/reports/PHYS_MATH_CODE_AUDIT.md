# PHYS–MATH–CODE AUDIT — Path Controller v0.17

## Equation-to-code map

| Mathematical object | Code path |
|---|---|
| round \((\lambda,\theta,q,\omega,c)\) | `HomotopyRoundSpec` |
| validated bounded schedule | `HomotopyScheduleConfig` |
| partial-path residual | existing `evaluate_partial_path` |
| finite q-truncated inverse | existing `truncated_partial_inverse_rows` |
| scheduled execution | `run_scheduled_homotopy_path` |
| original target certificate | `certify_nonlinear_target` |
| sequential oracle | `sequential_step` with direct linear method |
| research screen | `run_path_controller_screen` |
| CLI | `homotopy-path-controller` |

## Code-path reality

- 새 schedule engine은 기존 partial-path kernels를 재사용한다.
- 기존 fixed API를 바꾸지 않았다.
- numerical failure는 `ScheduledHomotopyPathReport`에 partial points/work와 함께 반환된다.
- BDF/Radau implementation files는 base tag 이후 unchanged다.

## Fixed issues

### P1 — algebraic path success와 timestep acceptance 혼동

초기 screen은 `combined_error <= 1`을 path success로 사용했다. Complex Dahlquist에서 q=7 stage root는 sequential oracle과 일치하지만 embedded error가 1보다 커서 path failure처럼 보였다.

수정 후 다음을 분리했다.

```text
algebraic_accepted
embedded_step_accepted
full_step_accepted
```

Path-controller 판정은 algebraic output budget을 사용한다.

### P1 — path rounds와 실제 critical work 혼동

Schedule length만으로 low-depth를 주장하지 않고 다음을 모두 저장한다.

```text
path_rounds
correction_rounds
w_solve_batches
w_solve_vectors
```

`direct-decoupled-q2-c2`는 path round 1이지만 실제 common-W batch depth는 8이다.

## Remaining risks

### P0

없음. 현재 bounded screen에서 false acceptance는 0이다.

### P1

- global-order/stiff-decay 미검증
- direct LU 및 dense certificate 사용
- hostile nonnormal case 미통과
- endpoint-correction survivor가 SABR/SDC/Picard와 실질적으로 구분되는지 미확정

### P2

- schedule set이 calibration corpus에 맞춰진 exploratory set
- output budget 0.1의 production calibration 부재
- q1→q2 escalation 미구현

### P3

- plot label density
- canonical corpus 규모

## Test sufficiency

추가된 tests:

- invalid schedule rejection
- fixed-path equivalence
- partial failure preservation
- deterministic smoke report
- algebraic/timestep acceptance separation
- deterministic CLI artifact

다음 단계에서 필요한 tests:

- matrix-free backend equivalence
- adaptive q escalation rollback
- trajectory fifth order
- complex phase/amplitude fidelity
- stiff PR and hostile fallback

## 최종 code 판정

```text
research reference layer: PROMOTE
runtime/production path controller: HOLD
```
