# VigilODE NCP intensive benchmark — 결과 보고

## 범위와 실행

프로토콜 `vigilode_ncp_intensive_120m_v1`; source commit `de4c10a9f41321beef2313a3d7fb503bab35d714`, tree `42c3917d5303568ca9169c0465728b783af8f45c`. NCP `s1a0e5abed09` (Xeon Gold 5220, KVM): 1 socket, 64 reported cores/logical CPUs, 1 thread/core, 1 NUMA node, cpuset 0–63, cgroup CPU/memory quota 없음, RAM 135.05 GB, 시작 시 available 93.67 GB, swap 없음. exclusivity/clock lock는 미확인. 128 host telemetry samples에서 load1 median 0.82/max 6.46, available RAM 77.6–82.9 GiB, reported frequency 2194.848 MHz로 관측됐다(주파수 고정 증거 아님; host-wide snapshot). 시작 `2026-10-21T06:21:09Z`, 종료 `2026-10-21T07:15:14Z`, 경과 3244.59 s (54m04.6s), 120분 제한 미도달.

Rust 1.94.1와 pinned Python/Julia/diffsol toolchain에서 native benchmark adapter와 외부 adapters를 빌드했다. Julia의 최초 smoke API 오류 및 diffsol version/API 빌드 실패, 결과 framing/aggregator 오류는 모두 원본 로그로 보존하고 수정된 isolated adapter/recovery 파생물을 별도 보관했다. Cargo.lock과 solver source는 바꾸지 않았다. full workspace test suite는 이번 campaign에서 실행하지 않았다. Package verifier PASS는 source/evidence identity·coverage 검사이며 solver 정확성 증거가 아니다.

## 측정 구성과 제한

Screening은 24 cases, 216 measurements/72 warmups, 7개 문제군을 포함한다. Confirmation·work-precision·throughput·nonlinear 단계를 포함해 `COVERAGE.json`의 phase별 계획/완료 수를 확인한다. 28-worker throughput block은 28개의 독립 1-thread worker를 CPUs 12–39에 pin하고 18.41 s 동안 실행했다(28 worker exit 성공, Python rotation 3-arm의 adapter 출력 오류를 제외한 기록 175/196 measurement rows 유효; 21 Python rotation adapter failures를 별도 유지, 단순 유효행 처리량 약 9.51 rows/s). Corrected rotation throughput은 별도 3 worker/CPUs 12–14였다. 확인/precision blocks는 주로 CPU 8 단일 affinity였다. 초기 screening은 unpinned였다. 따라서 latency는 diagnostic-only이며 screening/throughput/serial을 한 순위로 합치지 않는다. 실제 solver 내부 1/2/4/8/16/32 scaling은 NOT_RUN; 테스트된 경로에서 내부 병렬 backend를 확인하지 못했다.

비교군: SciPy 1.11.4 VODE Adams/BDF (float64, process-isolated), diffsol 0.17.1 Tsit45/BDF + dense NalgebraLU, Julia 1.12.7 OrdinaryDiffEq 7.8.1 (Tsit5/Rodas5P, dense default linear path), VigilODE native `rodas5p-fast`. CVODE/RADAU5는 실행하지 않았다. Julia warmed solve는 위 solve-time이며, worker process wall에는 Julia 시작/패키지 부하 비용(약 15–17초)이 별도로 포함된다.

## 정확도와 성능 관측

정확한 해석적/검증된 제조해가 있는 scoped confirmation은 `CONFIRMATION_RESULTS_RECOVERED.jsonl`, `JULIA_RESULTS.jsonl`, `DIFFSOL_RESULTS_RECOVERED.jsonl` 및 tolerance 결과를 근거로 한다. 7회 confirmation의 정확도 통과는 해당 case/tolerance에만 해당한다. 반복 arm은 paired/interleaved가 아니어서 speed ratio confidence interval을 산출하지 않았고, speed 기반 promotion은 하지 않는다.

- Nonstiff decay n=8: VigilODE 7/7 정확도 통과, median 0.161 ms; Julia Tsit5 7/7, 0.0311 ms; VODE Adams 7/7, 0.431 ms; BDF 0/7.
- Stiff diagonal n=32, L=1000: VigilODE 7/7, 1.024 ms; Julia Rodas5P 7/7, 1.059 ms; VODE Adams/BDF 0/7. 이 비교군의 차이는 practical 10% 이내이며 paired uncertainty가 없어 승격 근거가 아니다.
- Rotation omega=10: 수정 adapter로 모든 arm이 실행됐지만 tested componentwise rtol 1e-6 / atol 1e-10에서 모두 실패. VigilODE max scaled error 약 5156.6. Work precision rtol 1e-4…1e-10 네 점 모두 통과 0/140. 원래 adapter 회전 branch 누락은 환경/adapter 실패로 보존하며 수치 실패와 합치지 않았다.
- Damping zeta=1: VigilODE/Julia 7/7 pass; VODE Adams/BDF fail.
- Nonnormal transient: VigilODE/Julia 7/7 pass; VODE Adams/BDF fail.
- Structured heat n=128: VigilODE, Julia, VODE BDF, diffsol BDF 모두 7/7 pass. VigilODE median 14.56 ms, ~13.5 MiB peak RSS; Julia Rodas5P 9.20 ms, ~987 MiB; VODE BDF 252.4 ms, ~61.9 MiB; diffsol BDF 약 4.09 ms. RSS 경계는 각 프로세스/worker가 보고한 peak이며 process-tree 또는 OS 상한으로 해석하지 않는다. VigilODE 측정은 이 케이스에서 Julia보다 느리고 메모리 footprint는 작다.
- Mixed near-zero: VigilODE/Julia 7/7 pass, VODE Adams/BDF fail.
- Robertson / Van der Pol mu=20: solver runs completed, positivity/mass or state descriptors recorded, 그러나 independent numerical reference 없음. 정확도 UNKNOWN/HOLD. Robertson sampled solutions nonnegative; max mass residual 8.9e-16. Van der Pol sampled max |state|=2.0. 이는 trajectory accuracy 증명이 아니다.
- diffsol 0.17.1 exact-case confirmation: decay n=8, stiff diagonal n=32, nonnormal transient, heat n=128에서 각각 7/7 accuracy pass; rotation, damping zeta=1, mixed near-zero는 각각 0/7. Robertson/Van der Pol은 independent reference가 없어 UNKNOWN. measured per-call median은 raw에 있으며 RSS가 제공되지 않은 행은 memory Pareto에서 제외했다.
- Screening-only families/parameters는 coverage와 accuracy pass/fail를 raw에 보존했다. Screening time은 unpinned이므로 성능 승격 근거로 사용하지 않는다.

Decay n=128 work precision은 4 tolerance level, 5 arms, 140 measurements/40 warmups. aggregate 77 pass/63 fail. 회전 work-precision aggregator는 Python adapter 108행을 원본 summary에 빠뜨렸으며, 복구 집계는 140 measurements/40 warmups이고 0 pass/140이다. 원 summary/raw 모두 보존했다. diffsol 원본 JSONL은 literal `\n` framing 때문에 파서가 실패했고, 원본 SHA와 recovered JSONL을 분리했다. 결과 실행은 70/70 측정행 모두 상태 OK다.

## scoped 판정

`DECISIONS.json`에는 solver/backend × exact case × rtol 1e-6의 판정이 있다. 반복된 exact-budget accuracy fail은 그 사례/설정에 한정해 `prune`로 기록했고, 그 밖의 정확도 통과 범위도 비교 폭·reference·paired uncertainty 부족으로 `hold`다. 어떠한 broad production 승격도 없다. `specialized_for_specific_cases_only`는 이 데이터만으로도 아직 부여하지 않았다. heat n=128은 VigilODE의 낮은 관측 RSS와 느린 runtime이 함께 보여 scoped resource tradeoff 후보이나, 한 구조/크기뿐이다.

`PARETO_FRONTIERS.json`은 RSS와 latency가 모두 관측된 confirmation median만의 기술적 Pareto 요약이다. timeout/unknown accuracy를 제거해 성공률 분모를 감추는 용도가 아니다. Work precision, scaling, memory, failure-coverage 그림은 `figures/` 아래 있다.

## HOLD 및 최적화 근거

1. Heat adapter는 dense Jacobian을 basis JVP 반복으로 만들고, periodic heat JVP가 RHS를 호출한다. n=128에서 별도 미계상 derivative work 5,376 RHS 호출, 계상된 336 RHS calls, 42 step/Jacobian builds, Jacobian reuse 0회가 관측됐다. 이건 benchmark adapter 경로의 측정 사실이며 제품 solver 전반의 병목으로 확대하지 않는다. 이미 지원되는 structured/analytic Jacobian 경로와 adapter accounting을 비교할 수 있다.
2. Rotation의 근영역 componentwise scaled error가 모든 solver에서 budget을 초과했다. phase/amplitude와 componentwise 지표를 함께 추적하되 tolerance를 완화하지 않고 원인을 조사해야 한다.
3. confirmation arms는 paired block이 아니므로 다음 속도 판정은 deterministic interleaving과 사전등록된 paired interval이 필요하다.

## 상태

Package: PASS (identity/coverage only). Builds/adapters/smokes: 성공 기록됨. 전체 workspace tests: NOT_RUN. Benchmarks: campaign 완료, 여러 exact scopes accuracy fail. Gate: NOT_RUN (해당 frozen promotion reader는 다른 프로토콜용). broad production: HOLD. 실패·unknown·adapter/aggregator 문제는 원본과 recovered 파일을 나누어 보존했다. 상세 counts는 `COVERAGE.json`, decision rows는 `DECISIONS.json`을 본다.
