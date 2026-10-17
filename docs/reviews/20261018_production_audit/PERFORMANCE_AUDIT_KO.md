# 최신 성능·승격 근거 독립 감사

대상 commit: `2fc61b944fe431d40ba455c373579bce00b0c39a`. 감사 범위는 LP09/LP10, 최신 Diffsol 비교 하네스와 보고서, ME01 결과의 현재 사용 범위다. 기존 캠페인은 재실행하지 않았다. 정적 소스 추적, 기존 JSON의 재집계, Diffsol 0.16.1 공식 API/소스 대조를 수행했다. 이 문서는 일반 RODAS5P 적분기의 모든 정확도 판정을 대체하지 않는다.

**판정:** 현재 자료는 LP10의 `certified specialized high-performance subsystem` 승격을 뒷받침하지 못한다. 특히 LP09의 fairness/accuracy/무할당 판정에는 실행 결과와 연결되지 않은 상수가 있고, 실제 packet 반복에서 할당이 발생한다. 전문 packet solver의 production 승격은 이 지적과 별도 수치 인증 감사를 닫을 때까지 HOLD다. 기존 일반 fast driver의 과거 PASS는 그대로 보존한다. 최신 Diffsol 수치는 구성된 실행의 진단 자료로 보존하되, 공정한 동등 정확도 속도비 및 production 권고의 권위로 사용하지 않는다.

## 확정 지적

### PA-01 — LP09가 정확도 실패와 무관하게 PASS를 반환한다 (높음)

`crates/rodas5p-fair-ab/src/packet_benchmark.rs:200–202`는 두 최종 평균의 거리를 계산하지만, 이를 tolerance와 비교하는 분기가 없다. `245–246`의 `fairness_verified: true`, `verdict: "PASS"`는 성공적으로 반환한 모든 실행에 동일하다. `158`의 packet `error_vs_reference`는 자체 반경이고 `215`의 direct 같은 이름 필드는 두 방법의 차이이다. 두 값은 서로 다른 통계량이며 독립 참조 오차가 아니다. `packet_benchmark_contracts.rs:59–76`는 tolerance=1e−6 실행에서 차이<1e−4만 요구한다. 이 검사는 설정된 물리 tolerance를 100배 완화한다.

교정: `reported_certificate_radius`, `cross_method_discrepancy`, `reference_error_upper`를 별도 필드로 만든다. 동일 물리 출력과 norm에 대해 독립 기준의 불확도까지 합한 실제 오차 상계가 요구 ε 이하일 때만 accuracy PASS를 생성한다. 모든 모드 출력 계약이면 평균뿐 아니라 모든 모드를 검사한다. 부정 테스트는 의도적으로 ε를 초과하는 출력/참조를 주입해 PASS가 차단되는지 확인한다.

### PA-02 — 무할당 수치가 계측되지 않으며 packet 반복 소스와 충돌한다 (높음)

LP09 `packet_benchmark.rs:163,220`는 두 arm의 `hot_loop_allocations`를 모두 0으로 대입한다. candidate solver counters도 `159`에서 `WorkCounters::default()`이며 이를 읽는 VG1 테스트는 counter 배선 자체를 검증하지 않는다. 현재 packet 경로에 Jacobian/LU 호출이 없는 구조적 사실은 보존하되 계측 근거와 구분해야 한다.

`packet_window.rs:314–318`의 매 Picard sweep은 `force`, `convolve`, `poly_add`를 호출한다. `poly_mul`의 `111`, `convolve`의 `201`, `poly_add`의 `85`가 새 `Vec`를 만든다. `packet_trajectory.rs:276,303–305,325`는 각 시도·승인에서 radii, model arrays를 다시 할당/복사한다. 이는 LP10의 전체 pipeline zero-allocation 주장에 대한 직접적인 소스 반증이다. 루프가 유한한 nonzero 크기의 버퍼를 만들므로 빈 입력 최적화로 해소되지 않는다.

교정: setup, attempt, reconstruction, retained-output를 각각 계측한다. `PacketWorkspace`에 최대 차수의 복소 계수와 radii를 예약하고 `*_into` 커널로 작성한다. 기록 출력 할당은 실제 사용 계약에 따라 별도 비용으로 남긴다. 전역 0 상수를 실제 카운터 또는 `NOT_MEASURED`로 바꾼다.

### PA-03 — LP10 checker가 판단의 입력 근거를 검증하지 않는다 (높음)

`tools/lp10_stage0_check.py:40–51`는 JSON에 필요한 문자열과 node 이름이 있는지 검사한다. `62–73`에서는 수학/무할당/공정성 게이트를 True, unresolved blockers를0으로 새로 쓴다. 판정 JSON 해시만 `75–76`에 묶으며 실제 구현·수치 증거의 해시/오차/할당 결과를 읽지 않는다. 따라서 이 checker의 PASS는 선언 구조 일관성이다. PA-01/02와 충돌하는 `INDEPENDENT_DECISION.json`의 승격을 과학적으로 확인하지 못한다.

교정: 기존 결정은 시점 기록으로 보존하고 새 superseding decision에 source hash, 실행 입력, 실제 측정, 검사식, 독립 리뷰를 연결한다. 미측정 값은 `null`과 이유를 사용한다. checker가 결과 결정을 미리 강제하지 않게 한다.

### PA-04 — Diffsol 비교의 시간·출력·tolerance 계약이 서로 다르다 (높음)

`tools/benchmarks/diffsol/main.rs:311–324,402–411`에서 Diffsol timed closure는 문제 생성, solver 생성, `solve(final_time)`, 최종 상태 복사까지 포함한다. 공식 0.16.1 `OdeSolverMethod::solve`는 내부 adaptive timepoints 전체의 dense output matrix를 반환한다. VigilODE는 `848–876`에서 문제와 출력 schedule을 timer 전에 만들고 `881–884`에서 초기/최종 두 출력만 적분한다. timer의 작업과 저장 출력 수가 다르다.

Robertson Diffsol의 atol은 `353,445`의 `[rtol×1e−4, rtol×1e−10, rtol×1e−2]`지만 VigilODE는 `851,927`의 scalar `rtol×1e−4`다. JSON은 Diffsol vector atol을 첫 성분 scalar로만 저장한다. 같은 rtol 표가 같은 error-control 계약을 의미하지 않는다.

교정: setup 포함 full-call와 prepared-solver 반복을 분리하고 양쪽에 동일한 출력 grid, norm, component atol을 적용한다. output 요구가 endpoint이면 Diffsol의 endpoint step/stop-time 경로 또는 동일 `solve_dense` grid를 쓴다. 실제 얻은 오차의 work–precision frontier를 보고한다.

### PA-05 — Diffsol 할당·step metrics의 의미가 잘못 표기된다 (높음)

`main.rs:503,647,731,830`는 allocation을 각각 `number_of_steps×7/7/8/10`으로 생성한다. Robertson faer의 경우만 `382–400`에서 첫10개 `step()`의 평균을 정수 나눗셈한 뒤 `433`에서 전체 step에 곱한다. 첫10번 step의 오류도 무시한다. VigilODE 할당은 `908,967,1016,1065`의 0 상수다. 따라서 문서가 제시한 수천 회의 allocation은 완주 시점에 직접 계측한 정확한 횟수가 아니다.

공식 Diffsol 0.16.1 `bdf.rs:1461–1473`은 승인된 step을 반영한 뒤 `number_of_steps += 1`을 실행한다. 하네스 `415` 등은 여기서 error-test failures를 다시 빼 accepted_steps를 만든다. BDF의 승인 수가 과소 집계된다. 예: 저장된 VdP BDF faer tol1e−4는 accepted361, rejected67, total428이다. upstream 의미에 따른 승인 수는428이며 error-test rejects67은 별도이다. nonlinear failures도 별도 계수라 재시도 총합은 정확한 이벤트 정의로 취급해야 한다. 동일 이름의 `linear_solves`도 Diffsol에서 nonlinear iterations를 복사한 값이며 native linear solve 계측과 분리할 필요가 있다.

교정: raw upstream counters를 그대로 저장하고 adapter 정의에 따라 derived fields를 생성한다. allocation은 실제 구간 계측 또는 `estimate`로 표기한다. 기존 JSON은 원본 유지하고 별도 correction view를 만든다.

### PA-06 — 최신 보고서와 연결된 데이터가 일치하지 않는다 (높음)

52개의 성공 row를 `(problem,arm,rtol)`로 연결해 표의 median을 비교한 결과 **49/52**가 JSON과 다르다. 예: Robertson BDF faer tol1e−4는 보고서77μs, JSON146μs; VdP fast tol1e−8은845μs와479μs; Brusselator BDF tol1e−6은5740μs와4375μs. 이 값들로 계산한 서술 속도비도 데이터 출처가 불명확하다.

`main.rs:1182–1197`의 오류는 arm/problem/rtol을 보존한 record가 아니라 `unknown` 문자열로 콘솔에 출력되고 JSON에서 제외된다. JSON에는52건 모두 status=ok이고 문서에서 말하는7개 실패 case가 없다. stderr/실패 시점·solver config를 이번 데이터에서 재구성할 수 없다.

교정: 표를 JSON으로부터 자동 생성하고 campaign ID/hash를 함께 기록한다. 59개 예정 cell의 성공/실패를 모두 typed row로 저장한다. 원자료가 없는 실패 내용은 기존 저자 관찰로 분리해 표시한다.

### PA-07 — 정확도·물리 문제·재현성 표기의 추가 교정 (중간)

- 참조는 `1095–1098`에서 같은 `rodas5p_fast`의 tol1e−10 단일 실행으로 생성된다. 저장 참조 벡터·불확도·독립 참조가 없어 HIRES의5.5e−12 등은 이 참조와의 차이이다. 새 production 전역 정확도 근거에는 독립 참조 또는 검증된 enclosure가 필요하다.
- 보고서의 참조 atol=1e−12 고정 주장은 소스와 다르다. 실제 source는 문제별 rtol 배율을 유지하여 Robertson/HIRES1e−14, VdP/Brusselator1e−10이다.
- Brusselator 문서는 −4.4u,+3.4u,Neumann 경계를 쓴다. 양쪽 구현(`176–185`, `743–762`)은 −4u,+3u와 경계 (u,v)=(1,3)의 Dirichlet 값이다. **두 arm의 구현 방정식은 서로 일치한다.** 잘못된 대상은 보고서의 물리 모델 설명이다.
- 비교 manifest는 별도 Cargo.lock이 없고 version `0.16.1`은 Cargo의 exact pin이 아니다. README대로 `/tmp/diffsol_bench`에 복사하면 `../../../crates` path가 대상 repo를 가리키지 않는다. JSON output path도 `1176`의 특정 작성자 절대 경로다. 플랫폼 정보는 Linux x86_64뿐이다. 현재 자료만으로 보고서가 말한 locked/offline 실행을 재현할 수 없다.

## 유지할 기존 근거와 적용 한계

ME01은 instructions, allocations, bytes를 나누고 모든 candidate의 `wall_time_speedup:null`, timing HOLD를 보존한다. 이 원칙과 기존 동등 계산 identity 근거를 재사용한다. E4 banded, E7 workspace least squares, E8 into-GMRES는 기록된 Ir/할당 비용에서 유망한 실제 구현 경로다. E2 controller 변경은 현 결과 LOSS/DOMINATED이므로 기본값 승격에 쓰지 않는다.

추가 source-level gap: `tools/me01_full_cost_check.py:325–340`의 strongest rival 판정은 같은 rtol의 warm Ir만 비교한다. preregistration의 error ratio band와 E2 matched-error frontier 규칙을 실행하지 않는다. 이번 기록의 E2 48쌍은 저장 error ratio가[0.5,2] 범위 안에 있으므로 이 사실 자체를 현재 accuracy 실패로 부풀리지 않는다. 다만 전체 frontier 계산과 향후 mismatch 거절은 별도 구현해야 한다. PROFILE의 `tree_clean:false`와 source_commit만으로 dirty source를 재현할 수 없어 다음 campaign에서는 source diff/hash snapshot을 함께 저장한다. 기존 campaign 반복은 요구하지 않는다.

## 계산을 줄이는 구체적 다음 단계

1. **계측과 승격 계약을 먼저 고친다.** 실제 오류 상계와 모든 출력 비교가 통과하지 않으면 성능 후보에서 제외한다. allocator를 별도 binary로 두어 timing을 오염시키지 않는다. 단일 process median은 진단 범위로 유지하고 실제 반복 샘플·순서·환경을 저장한다.
2. **packet workspace를 고정 크기로 예약한다.** degree d 후보에 대해 cubic forcing의 degree는3d, kernel order p의 convolution degree는p+3d+1이다. 이 차수로 p/free/force/convolution/scratch/new-radii를 한 번 예약하고 각 sweep에 fill/overwrite한다. convolution·truncation·projection의 식을 그대로 유지하면 floating operation order를 보존하는 `*_into` 구현의 bitwise 비교가 가능하다. 생성자/출력 저장 비용은 별도이며, 지정한 반복 내부 allocation은0으로 검증할 수 있다.
3. **강한 O(N) 직접 baseline을 추가한다.** 이 collective 모델의 실수2N Jacobian은 2×2 대각 block과 rank≤2 평균 feedback으로 표현된다. 각 Rosenbrock stage matrix A=A0−UCV에서 A0의 block inverses와2×2 Schur matrix를 사용하면 setup/solve를 O(N)으로 구성할 수 있다. Schur inverse의 잔차/역행렬 상계를 인증한 뒤 packet을 이 baseline 및 기존 matrix-free와 비교한다. O(N) RHS만 hoist한 dense LU baseline의 O(N³) 비용을 packet의 유일한 상대자로 삼지 않는다.
4. **검증된 참조로 work–precision frontier를 계산한다.** 참조 중심 r과 인증 반경ρ가 있고 d=||y−r||이면 실제 오차 e는 max(0,d−ρ)≤e≤d+ρ다(삼각부등식 두 방향). 따라서 d+ρ≤ε인 row만 수용하면 참조 오차를 후보 오차로 잘못 해석하지 않는다. 기준회색 구간은 참조를 더 정밀하게 한 뒤 판정한다. 각 ε에서 수용된 row의 최소 full cost를 비교하며 setup·출력·검증·거절을 포함한다.

공식 외부 source(2026-10-18 확인):
- https://docs.rs/diffsol/0.16.1/diffsol/ode_solver/method/trait.OdeSolverMethod.html : solve가 내부 adaptive timepoints를 저장한다.
- https://docs.rs/diffsol/0.16.1/src/diffsol/ode_solver/bdf.rs.html : source1461–1473, accepted step 뒤 step counter 증가.
- https://docs.rs/diffsol/0.16.1/diffsol/ode_solver/struct.OdeSolverStatistics.html : upstream raw counter 의미.

이 감사는 새 no-go 정리를 주장하지 않는다. 새로 발견한 근거 배선 오류와 비교 계약의 교정을 제공하며, 생산 적합성 결정은 수학·수치·코드 감사 결과와 함께 최종 통합해야 한다.
