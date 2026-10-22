# 독립 코드 검토 — Stage B

검토 시각: 2026-10-21 15:06–15:09 UTC. 이 문서는 함께 저장한 JSON의 SHA256으로 지정한 코드에 대한 **읽기 검토 snapshot**이다. 실제 전체 궤적과 그림의 검토는 이 snapshot에서 아직 수행하지 않았다.

## 판정과 독립성

새 `atomic_network.py`, `geometry.py`, `coupled.py`, `run_pilot.py`에 대해 현재 발견한 차단 결함은 없다. 유한한 조건부 photo/recombination 수모형으로 실행을 진행할 수 있다. 이는 물리적 완전성이나 전역 수렴 인증이 아니다.

검토 agent는 위 네 새 파일의 작성자가 아니다. 다만 재사용된 PR118의 `vendor/tangent/tangent_core.py` 이전 작성에는 참여했다. 따라서 이 문서는 **새 결합 코드의 독립 agent 검토**이며, 전체 의존성 그래프 또는 tangent 원본에 대한 새로운 독립 감사는 아니다. 이번 검토자는 ODE, benchmark 또는 NCP 작업을 실행하지 않았다. 저자가 수행한 유한 검증의 receipt는 별도 근거로 읽고 코드 SHA 일치를 확인했다.

## 1. 핵종, 단위, 광전리

21개 상태는 n; H1 I/II, D I/II, T I/II; He3 I/II/III, He4 I/II/III; Li6 I/II/III/IV, Li7 I/II/III/IV이다. `Y_i`는 baryon당 핵 수다. 초기 inventory를 `sum A_i Y_i=1`로 정규화하고 각 isotope별 총량을 scale로 사용하는 처리는 극미량 Li를 H 절대 허용오차에 종속시키지 않는다. n과 T의 초기 총량은 0이며, 본 pilot에서 이들의 실제 붕괴가 발생한다고 주장할 수 없다. 양의 n/T를 사용한 별도 제조 상태 검증은 수 변화 지도만 검증한다.

광단면 Mb→m²의 `1e-22`, RR/DR 계수 cm³/s→m³/s의 `1e-6`, opacity의 `GAMMA_PER_NE/1e6` 변환이 서로 일치한다. 저장된 UV flux는 등방장에 대한 scalar encounter flux `c n_gamma`이므로 `sum F_bin E_bin/c`의 에너지 밀도 변환이 맞다.

Li I는 64.39 eV 아래에서 2s PH2를, 그 이상에서 1s PH1과 2s PH1의 합을 사용한다. occupancy를 중복 곱하지 않는다. 각 흡수를 q→q+1로 연결하는 것은 명시된 coarse-graining이다. 흡수단면 원전만으로 최종 전하·Auger·fluorescence·열화 수율을 검증했다고 표현하면 안 된다. 200 eV는 이번 새 유한 평가 범위이며 기존 Unit7의 마지막 실제 검증점 91.83842857919419 eV와 구분한다.

`primary_kinetic`, `unresolved_core`, `absorbed`가 구분되어 있다. Li I 1s 사건의 primary는 `E-64.39 eV`, unresolved core는 `64.39-5.392 eV`이다. valence ionization cost와 합하면 흡수 에너지가 된다. 이 분해는 빠진 수율을 임의로 열에 배정하지 않는다.

## 2. 원자 수방정식과 Jacobian

모든 generator는 열벡터 Y에 작용한다. 광전리/재결합은 인접 전하만 이동하고, 약붕괴는 n→H1II, TI→He3II, TII→He3III다. 열합 0, `A^T G=0`, 물리 상태에서 off-diagonal 비음수의 구조를 읽기 검토했다. T의 전하 지도는 spectator bound-electron approximation이며 정밀 beta branching 검증이 아니다.

`ne = nb q^T Y`이므로 정확한 유한모형 Jacobian은

`J = G0 + D + ne Gne + nb (Gne Y) q^T`

이다. 마지막 rank-one 항이 실제 코드에 포함되어 있다. 네트워크는 implicit solver의 유한 signed Newton trial을 클리핑하지 않는다. 물리적 positivity는 저장된 accepted trajectory에서 별도 진단된다. 외부 public energy/Y 입력은 complex cast 전에 거부하도록 검토 권고가 반영되었다.

저자 `atomic_verification.json`의 21/21 유한 검증을 읽었고 receipt의 코드·데이터 5개 SHA가 현재 파일과 일치했다. 21열 five-point FD Jacobian 최대 상대차는 6.7560e-12, rank-one 항 누락의 제조 반례 결함은 0.181985다. 이 값들은 저자 실행의 재사용 결과이며 검토자가 재실행한 값이 아니다.

## 3. 62상태 결합과 signed tensor

결합 상태는 frozen full40, scaled21 원자량, 광학깊이 1개다. `x=ln(a/a_ref)`에 맞춰 원자 dY/dt와 opacity를 H로 나눈다. fixed-x에서 H는 원자 Y에 직접 의존하지 않는 test-fluid 모형이다.

기하 H-gradient의 b 성분은 `rho_b=-6 pi`에 따른 `-3 H0² Pi_m/H²`, v 성분은 `Href² v/H²`, gamma/nu monopole 성분은 각각 `H0² Omega/(2 H² a⁴)`다. opacity의 H 미분과 ne 미분은 gamma energy/number 블록에만 들어간다. nu에는 Thomson damping을 넣지 않는다. scaled chemistry block의 row/column scale, `-outer(chemistry,Hgrad)`, tau 미분도 이에 일치한다.

별도 원자 ne가 실제 RHS와 진단에 사용되며, frozen source의 `ne_source=None`에서 남는 0 진단을 잘못 저장하지 않는다. Σ와 Pi는 LRS 직교 성분의 Frobenius 규약 `sqrt(6)`을 사용하고, 에너지 밀도 단위 stress에 `kappa_t=8 pi G/c²`를 적용한다. matter와 radiation signed 성분을 합한 뒤 절댓값을 취한다. radiation은 gamma+nu이고, baryon과 prescribed UV의 anisotropic stress는 이 모형에서 0이다.

## 4. 에너지 진단의 의미

T와 UV는 외부에 주어진 함수다. `n_gas=nb sum Y+ne`, `p=kB T n_gas`, `u_th=3p/2`이며, thermal expansion과 입자 생성 항, ionization-energy 변화가 모두 `net_required_gas_input`에 포함된다. `bath_minus_escape=Q_required-P_absorbed`는 미해결 외부 공급과 방출의 차이다. 이것을 RR 열화율이나 독립적인 thermostat 공급률이라고 부르면 안 된다.

UV·열·이온화 에너지를 Friedmann 식에 추가하지 않으므로 baryon rest-mass를 중복 계산하지 않는다. 작은 trace-energy fraction은 이 test-fluid 선택의 저장 지점 진단이며, 무시한 모든 효과의 오차 상한은 아니다. CI/CX/negative ions/molecules/nuclear fusion 및 radiative transfer의 생략에 대해 수치적 오차 상한을 주장하지 않는다.

## 5. 실행·검증·표현상의 범위

runner는 완료 payload의 identity/size/SHA를 확인해 재사용하며, 기존 실패·미확인 directory는 자동 재실행하거나 탈취하지 않는다. 모든 선택 case가 완료이면 자원 배분 전에 종료한다. live peer lease와 실제 자원 관측을 사용하며, 입증된 dead lease는 보존하되 CPU를 차감하지 않는다. 저자 runner 검증 13개 scoped 항목의 PASS를 읽었다. 이것은 실제 NCP 실행 기록이 아니다.

`einstein_constraint` gate는 H를 계산한 동일 대수 제약의 재계산 잔차다. 독립적인 Einstein 동역학 residual 또는 전역 보존 정확도 인증으로 확대하면 안 된다. population/trace-energy gate는 저장된 시간 노드, radiation angular positivity는 129개 각도 mesh의 유한 진단이다. 전체 시간·각도 positivity 정리나 global convergence 결과는 아니다.

**아직 남은 검토:** 최종 코드 변경점과 고정 hash, 실제 62상태 Jacobian 검증, 전체 z20→2 실행 receipt, 저장 trajectory bytes, 출력 그림의 수치/라벨 일치. `Physical HOLD`, `global convergence NOT_RUN`, `production NOT_APPROVED`, 원래 REC U5 HOLD는 유지한다.
