# Structural solver: 실패 비용 보존과 원래 stage 잔차의 선형 비용 인증

PR91의 immutable affine mean-field 소비자에서 **실패한 작업을 빠뜨리던 반환 계약을 보완하고, 실제 반환된 전체 stage의 원래 목표 잔차를 outward bound로 검증하는 경로**를 구현했다. 고정 모델은

\[
f(y)=b-dy+\kappa\operatorname{mean}(y)\mathbf1,\qquad d\ge\kappa\ge0
\]

이며 초기 상태와 forcing의 모든 transverse 성분을 유지한다. 기존 raw-U 8-stage 식, endpoint 합산, 사용자 잔차 cap을 보존했다. 새 경로의 시간 개선은 더 약한 잔차 검사나 바뀐 정확도에서 나온 결과가 아니다.

## 결과와 코드

- `MeanFieldPlan::step_charged`, `MeanFieldAffine::sequential_step_charged`는 `StructuralFailure { error, work }`로 모든 반환 실패의 partial work를 돌려준다. stage worker가 실패해도 예약된 전체 batch를 회수한 뒤 과금하며, stage 순서의 첫 오류를 반환한다.
- 기존 `step`, `sequential_step`의 `CoreResult` 인터페이스는 유지했다. 재시도나 fallback을 하는 호출자는 새 charged API를 사용한다.
- 기존 직접 검사는 stage마다 만들던 `state`, `mix`, `rhs`, `J U` 네 배열 중 `mix`, `J U` 배열을 제거했다. component별 계산 순서를 보존하여 PR91과 잔차 및 출력 bits가 같다.
- `MeanFieldAffine::enclosure_plan(h)` 또는 `MeanFieldPlan::enclosed_screen(n)`은 `MeanFieldEnclosurePlan`을 만든다. `step`은 `EnclosedStructuralCandidate`와 `full_residual_upper_bound`, stage별 상한, 실제 work를 반환한다.
- 새 검사는 `O(s^2 n)`의 원래 목표 재구성을 `O(sn+s^2)`의 enclosure로 대체한다. 스칼라 `n=1`에서는 정확한 `P=I` 퇴화를 이용하여 step마다 필요한 bound 계산도 `O(s)`로 줄였다.

새 소비자는 사용자 RHS/JVP/AD/Jacobian callback을 요구하지 않고 상태 행렬을 조립하지 않는다. 주어진 모델의 RHS를 직접 한 번 계산한다. 기존 process-lifetime tableau 초기화의 고정 `8×8` 계수 역행렬과 새 고정 크기 triangular norm-weight table은 상태 차원과 무관한 scalar metadata다. step work에는 state factorization, JVP, Newton update가 모두 0이다.

## 수학적 기여

`THEORY.md`와 `section.tex`에 정리 S1–S4 및 증명을 수록했다. 핵심은 실제 represented 단계에 대한 정확한 항등식이다. `x`는 실제 계산한 초기 RHS, `mu`는 그 실제 평균 계산값이고

\[
U_i=t_i(x-\mu\mathbf1)+v_i\mu\mathbf1+\Delta_i
\]

로 쓴다. `Delta_i`는 부동소수점 lift가 만든 전체 상태 차원의 오차다. 원래 raw-U 잔차는

\[
\begin{aligned}
\mathcal R_i(U)={}&\rho_{ti}(x-\mu\mathbf1)+\rho_{ci}\mu\mathbf1
+h\gamma\kappa\ell_i(\mu-\operatorname{mean}(x))\mathbf1
+h\gamma(x-f(y))\\
&+(I-h\gamma J)\Delta_i
-\sum_{j<i}(\gamma c_{ij}I+h\gamma a_{ij}J)\Delta_j
\end{aligned}
\]

와 정확히 같다. 스칼라 recurrence 잔차만 검사하면 빠지는 평균, RHS, full-state lift 항을 모두 유지했다. 각 block의 정확한 infinity norm

\[
\nu_n(t,c)=\left|t+\frac{c-t}{n}\right|+(n-1)\left|\frac{c-t}{n}\right|
\]

으로 실제 전체 잔차를 상계한다. `h*gamma`와 `d-kappa`도 represented 입력의 **실수 곱과 실수 차**로 outward enclosure에 포함한다.

Neumaier 평균의 FastTwoSum correction을 실제 연산에 대응시켰고, 각 연산의 half-ULP 반올림 상수를 exponent bits에서 계산했다. subnormal 영역에서는 표현할 수 없는 양의 half-spacing을 최소 subnormal로 위쪽 감싼다. 임의 safety multiplier를 사용하지 않았다. 독립 검토자는 Ogita–Rump–Oishi의 원저자 PDF에서 FastTwoSum 조건을 확인했다.

스칼라에서는 실제 평균이 입력과 정확히 같고 transverse 항이 0이다. `u=2^-53`, `eta=2^-1074`에서 곱셈 오차 `u |z|+eta/2`를 사용하여 plan에 error amplification을 미리 모았다. `eta/2`가 표현 불가능하므로 `eta`가 그 upward 표현이다. `n=1`이라는 조건은 대수적 projector 퇴화이며 측정으로 정한 크기 cutoff가 아니다.

개별 block norm은 sharp하다. 이를 더한 전체 rounding envelope는 보수적 상계다. 이 둘을 구별한다.

## 최종 전체 궤적 성능

모든 case는 `[0,1]`, 64 steps, `d=20`, `kappa=19`, 원래 physical cap `1e-6`, same-target cap `1e-11`, residual cap `1e-9`를 사용했다. baseline은 PR91 source의 동일 바이트를 같은 binary에 컴파일했다. model allocation, plan 생성, stage, 전체 residual verification, endpoint, 반환 allocation을 포함했다. pool 생성, 진단 JSON, observation 저장은 timing 밖이다.

각 값은 5개 ABBA/BAAB paired block의 중앙값과 관측 범위다. 범위는 confidence interval이 아니다.

| 차원 | worker | fused direct / PR91 | enclosure / PR91 |
|---:|---:|---:|---:|
| 1 | 1 | 1.011× [0.737, 1.231] | **1.190× [1.154, 1.323]** |
| 64 | 1 | 1.168× [0.915, 1.266] | **1.783× [1.649, 1.998]** |
| 2048 | 1 | 0.840× [0.619, 1.220] | **2.063× [1.226, 3.614]** |
| 2048 | 2 | 1.213× [1.032, 1.276] | **2.152× [0.745, 2.328]** |

모든 반환 grid state가 PR91과 bit 단위로 같았다. n2048의 최대 공통-grid 실제 오차는 `2.397597120840089e-10`, n64는 `2.384490382922877e-10`, n1은 `4.440892098500626e-16`이었다.

n2048 전체 64-step 작업량은 다음과 같다.

| 작업 | PR91 | 새 enclosure |
|---|---:|---:|
| RHS evaluation | 576 | **64** |
| physical linear action | 512 | **0** |
| full-state causal accumulation | 7,340,032 | **0** |
| 생성한 lift component | 1,048,576 | 1,048,576 |
| 검증 범위의 원래 residual component | 1,048,576 | 1,048,576 |
| outward 평균 검증 component | 0 | 262,144 |
| outward lift 검증 component | 0 | 1,048,576 |

일반적인 모든 차원과 thread 수에서 시간 이득이 보장된다는 주장은 하지 않는다. fused direct의 마지막 n2048/worker1 중앙값은 0.840×였고 범위도 1을 가로질렀다. two-worker 측정도 변동이 크므로 동일 worker 수의 algorithm 비교와 worker scaling을 구별해야 한다.

### 불리한 결과와 수정 3회

초기 작은 차원 probe에서는 n1의 fused direct가 **0.694×**, 일반 enclosure가 **0.392×**로 느려졌다. 원인은 stage마다 전달하는 전체 counter payload와 상태 차원에 무관한 triangular enclosure의 고정 비용이었다. 최종 수정은 실패 작업량을 정확한 batch count로 재구성하고, 위의 스칼라 퇴화 정리로 계획에 bound 합을 옮겼다. 이 후 enclosure의 scalar 결과가 1.190×가 되었다.

이 결과는 관측 후 수정·확인 실험이다. 초기 불리한 샘플을 삭제하거나 holdout으로 표시하지 않았다. 수정 루프는 다음 **3회**로 종료했다.

1. 부모 검토가 지적한 crate 외부 baseline import를 `examples/support` 내부로 옮겼다. 바이트를 보존했다.
2. 32-bit `usize`에서 dimension bound가 잘리는 정수 cast를 `u128` 비교로 고쳤다. 현재 64-bit 결과는 그대로이며, 새로운 scalar adverse probe를 명시적으로 추가했다.
3. 그 adverse probe의 원인을 겨냥하여 per-worker payload와 scalar enclosure를 수학적으로 최적화했다.

`CORRECTION_LOOPS.json`, 각 실행 receipt 및 이전 `BENCH_*.json`과 최종 `FINAL_BENCH_*.json`이 각각 남아 있다.

## 검증과 검토

**서로 다른 native 계약 11개가 통과했다:** 기존 구조적 계약 5개와 새 production 계약 6개다. 반복 실행을 새 계약 수로 세지 않았다. 새 계약은 초기 거절의 work=0, RHS 실패의 사전 과금, 병렬 실패 lift 전체 회수, residual cap 거절의 전체 work, 이전 연산/출력 bits, serial/parallel 동일성을 다룬다.

최종 정확한 유리수 oracle은 **21 fixtures, 168 stage bounds, 728 full-space residual components**를 통과했다. 원래 모든 raw-U 성분식을 `Fraction`으로 직접 계산하며 새 factorization이나 rounding bound 코드를 사용하지 않는다. 0-rate, `d=kappa`, 비표현 rate 차와 step, 큰 cancellation `1e100`, subnormal `1e-320`가 포함된다. 이 oracle fixture 수집에서는 `1e120` cap으로 실제 stage와 그 bound를 반환받은 뒤 각 bound의 유효성을 정확한 유리수로 따로 검사했다. 물리 정확도와 timing 실험의 `1e-9` residual cap은 앞 표의 모든 궤적에서 유지했다. 기존 **8,184 half-ULP exponent/mantissa 검산**은 바뀌지 않은 radius 함수에 대해 재사용했다.

별도 native agent `/root/production_homotopy`가 독립적으로 잔차 전개, 평균/lift 반올림, 스칼라 퇴화, 실패 과금과 실제 receipt를 검토했다. 부모 문맥은 공유했으며 blinded review로 표현하지 않았다. `../theory/STRUCTURAL_REVIEW.md`와 JSON에 source hash와 원전 확인이 있다. 검토 범위에서 열린 correctness finding은 없다.

### 실제 instruction profile

부모 실행자가 최종 frozen binary에서 Callgrind 3.22.0으로 세 arm을 각각 2회/6회 전체 trajectory로 실행했다. 여섯 실행이 모두 exit 0이며 source와 binary가 변하지 않았다. 초기화 비용을 뺀 `(Ir(6)-Ir(2))/4`는 n2048/worker1에서 다음과 같다.

| 경로 | trajectory당 guest instruction `Ir` | PR91 대비 |
|---|---:|---:|
| PR91 | 347,516,244.75 | 1.0000 |
| Fused direct | 309,511,284.25 | **0.89064 (10.94% 감소)** |
| Enclosure | 130,256,982.00 | **0.37482 (62.52% 감소)** |

이는 native wall time과 다른 측정이다. Fused 경로는 instruction 수가 줄었지만 마지막 wall-time 중앙값이 느렸으므로, instruction 감소를 일정한 시간 가속으로 바꾸어 설명하지 않았다. Enclosure는 같은 입력/출력의 실제 계산량 감소와 위의 paired wall-time 개선을 모두 보였다.

`instruction_profile/PROTOCOL.json`, `RUNS.json`, `SUMMARY.json`, 여섯 원본 `.callgrind` 및 stdout/stderr에 exact command, hash와 실제 결과가 있다.

## 사용 예

```rust
use rodas5p_integrators::ParallelExecution;
use vigilode_coupled::structural_mean_field::MeanFieldAffine;

let model = MeanFieldAffine::new(20.0, 19.0, vec![0.2; 64])?;
let plan = model.enclosure_plan(1.0 / 64.0)?;
let execution = ParallelExecution::sequential();
let state = vec![0.5; 64];
match plan.step(&model, &state, 1e-9, &execution) {
    Ok(candidate) => {
        // Use the candidate only after the caller's temporal-error contract.
        let next_state = candidate.y_new;
        let original_stage_bound = candidate.full_residual_upper_bound;
        let charged_work = candidate.work;
    }
    Err(failure) => {
        // Accumulate this work before any retry/fallback.
        let charged_work = failure.work;
        let reason = failure.error;
    }
}
```

Plan은 `h`, rates와 dimension에 묶인다. 같은 rates/dimension의 다른 forcing에는 재사용할 수 있다. 모델은 immutable이므로 rate/forcing mutation으로 certificate 전제가 바뀌지 않는다. 요청한 residual cap을 outward bound가 만족하지 못하면 charged failure를 반환한다. operational screen으로 조용히 대체하지 않는다.

## 재현과 정확한 범위

```bash
cargo test -p vigilode-coupled --profile measurement --locked --offline \
  --test structural_mean_field_contracts --test structural_production_contracts
cargo build -p vigilode-coupled --profile measurement --locked --offline \
  --example structural_production_bench_20261027
structural_production_bench_20261027 bench 2048 64 5 1
structural_production_bench_20261027 profile 2048 64 enclosed 8 1
structural_production_bench_20261027 oracle
python3 research/production_solver_closure_20261027/structural/analyze_results.py final
```

실제 경로·환경은 receipt에 고정했다. Rust 1.94.1, `measurement` profile, `--locked --offline`, jobs 2였으며 native build/timing/profile에는 공유 lock을 사용했다. Local release의 fat LTO/배포 CPU 측정은 `LOCAL_TASKS.json`의 별도 계약이다.

이번 certificate는 represented 모델/tableau와 실제 stage의 원래 잔차를 다룬다. 전체 physical trajectory를 outward 인증하려면 기존 `AlgebraicResidualBudget`의 ordinary-f64 누적 norm을 outward 값으로 확장하고, RODAS의 time truncation, 실제 endpoint 및 관측 시간 반올림을 합쳐야 한다. `LOCAL_TASKS.json`은 이 후속 이론 PR과 charged adaptive consumer PR의 목적, 첫 유도, 입력·출력·성공 기준 및 DAG를 명시한다. 현재 physical accuracy 실험과 stage certificate는 각각의 근거로 유지한다.
