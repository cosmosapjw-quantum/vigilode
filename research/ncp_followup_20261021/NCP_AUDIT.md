# NCP intensive 결과의 독립 재감사

대상 게시 commit은 `7a63876a97fc17f640771ec06658269947049994`, solver source는 `de4c10a9f41321beef2313a3d7fb503bab35d714`다. 원본 `external_audits/vigilode_ncp_intensive_120m_v1_20261021/`를 수정하지 않고 hash, 행 집계, 참조해, 비교 방법을 감사했다. 재현 명령은 다음과 같다.

```sh
python3 research/ncp_followup_20261021/audit_ncp_artifacts.py
```

이 명령은 solver를 다시 실행하지 않는다. 산출물 `NCP_AUDIT.json`은 frozen 원자료에서 계산한 교정값과 다음 실험의 조건을 담는다. 기존 게시물의 정확한 bytes를 보존한 채 이 문서와 JSON을 후속 해석으로 사용한다.

## 판정에 영향을 주는 결과

**Broad production은 HOLD를 유지한다. 회전계는 모두 실패한 것이 아니다.** 원 보고서의 “rotation work precision 0/140 pass”는 잘못된 서술이다. 회수된 JSONL 및 이미 게시된 `WORK_PRECISION_COVERAGE.json`은 **7 pass / 133 fail**로 일치한다. VigilODE의 `rtol=1e-10`, `atol=(1e-10,1e-10)` 7회가 통과했다. 반면 `rtol=1e-6`에서의 약 5156.6 scaled error는 실제로 보존된 수치 실패다.

| VigilODE 내부 rtol | 기록된 sampled-grid 최대 scaled error | 7회 통과 수 | 내부 timed solve 중앙값 |
|---|---:|---:|---:|
| 1e-4 | 903534.612 | 0 | 0.146849 ms |
| 1e-6 | 5156.614 | 0 | 0.402479 ms |
| 1e-8 | 24.3704 | 0 | 0.830360 ms |
| 1e-10 | 0.387683 | 7 | 1.515231 ms |

이는 한 해석해·한 시간구간·기록된 41개 표본의 경험적 근거다. native/VODE 원행은 전체 trajectory 대신 마지막 상태와 집계 오차만 보존했다. 마지막 상태를 독립 계산하여 7개 통과를 확인했지만 native 전체 trajectory max는 저장된 집계와 실행 코드에 의존한다. Julia/diffsol은 전체 상태가 있어 각각 49개/56개 exact-case 행의 grid 판정을 독립 재계산했고 판정 불일치가 0개였다.

성공한 native `rtol=1e-10` 실행의 endpoint absolute error는 `3.876828073259881e-11`, 기록된 최대 phase error는 `3.8770764376749867e-11 rad`, amplitude error는 `2.980060642698845e-12`다. `atol`을 유지하고 평가 rtol을 `1e-10`에서 원래 application budget `1e-6`으로 바꾸면 분모가 성분별로 증가하므로 같은 trajectory의 scaled error는 증가하지 않는다. 따라서 기록된 tighter-budget pass는 원래 budget도 충족하는 설정 후보다. 이것은 tolerance를 완화하여 기존 실패를 성공으로 바꾸는 조작이 아니라 **고정된 application budget을 만족하도록 internal tolerance를 더 엄격하게 선택하는 경로**다. 다음 실행은 두 tolerance를 명시적으로 분리해야 한다.

## Identity와 실행 coverage

- manifest가 열거한 759개 파일의 bytes·SHA256가 모두 일치한다. 실제 파일 760개 중 manifest 자체만 열거하지 않는다. 결과는 identity 검사의 PASS이며 수치적 보증과 별개다.
- phase별 measurement attempt 합은 **1070**이다. 원 rotation adapter 오류 42개를 포함하고, 수정 실행은 별도 attempt로 유지한다. JSON의 phase별 수치를 재사용하며 warmup은 측정 분모에 넣지 않는다.
- 기존 기록의 환경은 reported64CPU/약128GiB다. 실제 throughput block은 28개 독립 single-thread worker였다. 64CPU solver 내부 scaling, AMR, 큰 희소 FVM, SP02/SP03 crossover는 이 campaign에서 시험하지 않았다.
- 전체 workspace test 재실행은 이번 감사의 범위가 아니다. identity가 유지된 기존 검증을 재사용하고 이번 교정의 산술·입력 검사를 수행했다.

## 교정해야 할 구체적 문제

### 1. 회전 throughput 참조 시각이 잘못됐다

`harness/analyze_campaign.py`는 Julia throughput 상태를 `t_i=0.01 i`의 exact rotation과 비교했다. 실제 frozen request는 `t_i=2πi/40`다. 저장된 상태 41개를 실제 시각에 대입하면 최대 scaled error는 **35126828.929 → 7123.0642284**로 교정된다. `rtol=1e-6`의 실패 판정 자체는 유지된다. 원자료와 수정된 숫자를 함께 보존하며 임의로 만든 time grid를 폐기해야 한다.

### 2. Work-precision 그림의 시간 범위가 혼합됐다

집계기는 native 행의 `timing.elapsed_ns`를 무시하고 `process_wall_seconds`를 선택한다. 그 범위는 프로세스 시작·native 내부 warmup 등을 포함한다. Julia/diffsol은 warmed solve 시간을 사용한다. `rtol=1e-10` native 회전계에서 그래프의 median은 **5.559548 ms**, 보존된 내부 timed solve median은 **1.515231 ms**다. 이 둘은 서로 다른 측정이다.

새 비교는 setup, cold process, JIT, warmed solve를 각각 기록한다. native의 timed scope도 setup/integration/output/certificate를 포함하므로 이를 정확히 표시하고 외부 adapter의 scope를 맞춘다. 현재 수치를 교정해도 paired 실행·host 제어가 없으므로 속도 승격은 HOLD다.

### 3. VODE BDF가 Jacobian 없는 functional iteration으로 구성됐다

`confirmation_worker.py` 및 관련 worker는 Jacobian callback·band·`with_jacobian=True` 없이 SciPy `ode`를 구성한다. [SciPy v1.11.4 원본 `_ode.py`](https://github.com/scipy/scipy/blob/v1.11.4/scipy/integrate/_ode.py)의 기본값과 method-flag 경로를 확인했다. 이 구성은 Adams MF10 / BDF MF20의 functional iteration이다. 저장된 “default finite difference Jacobian” 설명은 틀렸다.

이는 VODE 자체가 잘못됐다는 증거가 아니며, stiff BDF의 대표적인 chord/Newton 구성을 비교한 것도 아니다. 다음에는 기존 MF20을 그대로 보존하면서 analytic Jacobian 또는 내부 finite-difference Jacobian BDF를 **별도 arm**으로 추가한다. 구조가 허용되면 band 정보를 명시한다. derivative RHS/Jacobian/LU 비용을 함께 계상하고 같은 achieved-error에서 비교한다. SciPy의 실수 `vode` wrapper는 DVODE를 호출하므로 이름이 다른 독립 알고리즘 두 개로 중복 계상하지 않는다.

### 4. 판정 scope와 counter의 의미가 일부 잘못됐다

`DECISIONS.json`에서 nonnormal과 mixed-nearzero의 rtol을 모두 `1e-6`으로 기록했지만, frozen case와 raw는 각각 **1e-7**, **1e-8**이다. 총10개 solver×case scope의 교정을 JSON에 열거했다. 기존 정확도 판정은 해당 실제 tolerance의 결과로 보존한다.

Julia adapter의 `nlu`는 `nsolve`/`nsolve_linear`를 가져온다. 선형계 풀이 수와 LU 분해 수는 다르다. 후속 schema에서는 `linear_solves`로 기록하고 실제 factorization counter가 없으면 `nlu:null`을 쓴다. RSS도 worker/child/process-tree의 범위를 구분하고, `ru_maxrss`의 누적 high-water 값을 개별 실행의 독립 peak처럼 해석하지 않는다.

### 5. 오차 평가기의 입력 계약이 약하다

기존 metrics 함수들은 `zip`으로 길이가 다른 grid/state/component를 조용히 잘라내며, 빈 출력을 max error0으로 만들 수 있다. 이번에 확인한 정상 행의 성과를 취소할 근거는 아니지만 production qualification 도구의 결함이다. 새 audit evaluator는 비어 있지 않은 입력, 정확한 shape, 유한한 상태를 먼저 검사한다. exact initial state 및 empty/mismatched/nonfinite 4개 rejection 검사를 실행했다. 다음 실행 하네스는 실제 terminal time과 저장된 출력 시각까지 확인해야 한다.

## 사용자에게 전달된 이식 조언과의 관계

이 bundle의 native arm은 `native-rodas5p-fast` 하나다. 따라서 “dense-fast가 SP02보다3–5배 빠르다”, “SP02 zero-allocation 달성”, “Schur로 GMRES3–5회”라는 문장은 이 campaign으로 뒷받침되지 않는다. 코드의 별도 근거와 실제 해당 경로의 실행이 필요하다. 차원1600이라는 하나의 숫자로 dense/sparse 교차점을 결정할 수도 없다. band 폭·preconditioner 품질·RHS 비용·threading·재사용률이 함께 결정한다.

유용한 실제 관측은 heat n128 adapter의 Jacobian 생성 비용이다. basis JVP를 n회 실행하고 periodic heat JVP가 RHS를 다시 호출하여, 42번 Jacobian build에서 **5376 extra RHS calls**가 생겼다. 이미 저장된 solver nfev336과 합치면 이 adapter 경로의 총 RHS 성격 평가 수는5712다. 이 구조는 analytic stencil 작성 또는 band/matrix-free 경로로 줄일 수 있고, 단순 해석적 Jacobian 교정의 실험부터 시작할 근거가 충분하다. 다만 dense 행렬을 유지하는 개선은 dense LU의 차수까지 바꾸지 않는다.

## 다음 실행의 최소 계약

1. application error budget, internal controller tolerance, 실제 achieved-error를 별도 필드로 고정한다. 회전계는 기존 grid를 유지하면서 phase/amplitude와 비동기 표본 grid를 추가해 aliasing을 점검한다. 원 case의 시간 간격은 `ω=10`에서 quarter-period, `ω=100`에서 half-period 배수여서 phase-error 스트레스를 주지만 연속 시간 max를 포괄하지 않는다.
2. complete output times/states와 원 실행 status를 보존한다. malformed·truncated·nonfinite 결과는 정확도 평가 전에 거부한다.
3. native analytic stencil 후보, 원 basis-JVP, 실제 SP02/SP03와 VODE Jacobian arm을 분리한다. 동일 문제·구조 정보·고정 application budget으로 dimension/bandwidth sweep을 수행한다.
4. 모든 후보를 paired/interleaved 순서로 실행하고 cold-start/steady-state를 별도 비교한다. 새 optimizer의 비용 절감과 기존 solver 대비 성능을 서로 다른 근거로 기록한다.
5. nonlinear Robertson/Van der Pol은 독립 refinement reference를 먼저 구성한다. positivity/mass 통과만으로 trajectory accuracy를 승격하지 않는다.

결론은 **artifact integrity PASS, 일부 보고/측정 의미론 교정 필요, tight-tolerance 회전계 scoped 성공 근거 복원, broad production HOLD**다. 다음 개발은 근거가 확인된 analytic Jacobian·구조화 선형계·정확도 budget 분리에 집중한다.
