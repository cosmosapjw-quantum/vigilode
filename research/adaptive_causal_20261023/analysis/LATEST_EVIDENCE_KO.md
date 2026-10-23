# 최신 실행 근거와 새 비교 계약

2026-10-23. 이번 시작점은 `14c74978b93339a79d7228a324fa77552dae548e`다. 이 시점에 fetch된 원격 tip과 소스가 일치한다. 사용자가 완료했다고 알린 로컬 실행 중 아직 원격에 게시되지 않은 receipt는 이 호스트에서 확인할 수 없다. 확인 가능한 가장 최신 실행은 아래 두 묶음이다.

- `production_hardening_20261023`: 실행 소스 `5913fe06`, 72개 구성·288회 process record, dense/landing 출력과 physical norm 연구. 288개 재채점과 576개 raw hash 검증, 별도 최종 검토가 이미 있다.
- `nonstiff_external_adapters_20261023`: adapter 소스 `1cf87238`, 384개 diffsol/Julia 실행, 기존 native/VODE와 합한 960개 record의 독립 재채점이 게시되어 있다. native 기준 소스는 `45037fc2`다.

이 근거는 기존 가정·소스 범위에서 재사용한다. 이번 작업을 위해 옛 solver campaign을 다시 실행하지 않는다. 기존 비교는 큰 벡터에서 Julia가 더 빠르고, rapid oscillator의 엄격한 sampled-error cap을 일부 DOPRI 설정이 놓쳤다는 반대 결과도 포함한다. 공유 host와 서로 다른 실행 cohort의 시간을 새 코드의 속도비로 합치지 않는다. 파일 identity는 `evidence/BASELINE.json`에 기록했다.

## 새 native 비교

`adaptive_causal_bench.rs`는 decay, logistic, Prothero–Robinson, Robertson, rapid/damped oscillator, nonnormal, 매끄러운 nonstiff–stiff–nonstiff 전환, 256차원 diagonal vector를 포함한다. 비교군은 DOPRI5, RODAS5P finite difference, RODAS5P analytic JVP, switching FD, switching JVP다. 같은 17개 출력 시각을 사용한다. 처음 다섯 방법은 동일한 16개 구간에서 workspace를 재시작하는 통제 비교다. 추가로 `dopri-whole`은 단일 호출·관측시각 착지, `dopri-whole-dense`는 단일 호출·quartic 표본, `rodas-whole-fd`는 단일 호출·continuous extension을 사용해 workspace를 유지한다. 마지막 RODAS 경로는 기본 `WFilteredEnforce` 내부 오차 제어를 추가로 사용한다. 재시작 비용을 강제 baseline에서도 지워 버리지 않도록 이 전체 적분 비교를 함께 제시한다. 여덟 경로 모두 실제 관측 오차로 비교하며, 추가 switching 중간 시각의 오차도 보수적으로 포함한다.

1. 실행 binary·source commit을 고정하고 CPU affinity 하나, Rayon/BLAS/OMP thread 수 1을 사용한다.
2. rtol 1e-4/1e-6/1e-8, atol=rtol×1e-3, global attempt budget 25000. 한 warmup과 세 timed 반복을 문제/허용오차 그룹 내 무작위 순서로 실행한다.
3. 문제·옵션·상태·workspace 구성과 모든 재시작·실패 candidate 비용을 solver timer에 포함한다. 기준해 계산과 JSON 직렬화는 제외한다.
4. analytic reference를 Python에서 독립 계산한다. Robertson은 SciPy VODE BDF analytic Jacobian의 rtol1e-12/atol1e-15 결과를 사용하고, 10배 느슨한 설정과의 차이를 함께 기록한다. 이 차이는 수치 reference의 refinement 진단이며 엄밀한 enclosure는 아니다.
5. 같은 달성 오차 상한 1e-4/1e-6/1e-8을 만족하는 설정끼리 비용을 비교한다. 모든 실패·시간 제한·미실행을 보존한다. 위상 오차, 진폭 오차, Robertson 질량 drift도 따로 기록한다.

FD와 JVP를 사용한 방법은 명시적으로 구분한다. 이러한 fallback의 성공을 완전 derivative-free 방법의 성공으로 표현하지 않는다. 작은 공유 환경의 세 반복은 성능 진단을 제공하며 통계적으로 보장된 가속률을 제공하지 않는다.
