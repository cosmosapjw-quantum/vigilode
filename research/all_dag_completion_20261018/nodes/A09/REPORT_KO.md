# A09: 실제 represented-K H5와 비선형 window client

이번 변경은 원래의 RODAS 단계 방정식과 새 window 병렬화 방법을 각각 구현했다. 원래 A09는 완전한 represented-K 방정식에 대한 공통 homotopy tube 인증까지 도달했다. 새 window client는 원래 비선형 ODE의 요청 시점 오차를 독립 검증한다. 두 결과 모두 정확히 λ=1인 원래 source를 대상으로 한다. 다만 stage 방정식의 해 오차가 곧 ODE 이산화 오차는 아니며, 이번 단일 실행으로 속도 승격이나 범용 production solver 승격을 판정하지 않는다.

## 원래 H5 의무를 실제 코드로 닫은 부분

`actual_k_homotopy.rs`는 로드된 α, β, γ, b와 모델에서 생성한 represented frozen Jacobian J를 소유한다. 표의 대각/상삼각 누출을 0으로 바꾸지 않는다. 양의 majorant A를 엄격한 하삼각 B와 나머지 E로 나누고, S=(I−B)⁻¹ 및 q=‖SE‖∞를 검증한다. q<1만 확인하는 것으로 끝내지 않고 공통 λ∈[0,1] residual d에 대해 실제 반환 반경 r가 d+Ar≤r 및 r≤R를 만족하는지 outward arithmetic으로 다시 확인한다. 따라서 λ=1의 실제 represented-K 목표를 둘러싸는 유일한 해 tube를 반환한다. 역행렬은 근삿값 자체를 신뢰하지 않고 ‖I−VW‖∞<1의 구간 검증을 통해 사용한다. 자세한 유도는 `ACTUAL_K_H5.md`에 있다.

비선형 2-mode fixture에서 상삼각/대각 α 누출 최대값은 약 5.58×10⁻¹⁶이고, β−γI의 해당 누출은 약 3.77×10⁻¹⁶이다. 이를 그대로 포함해 q≤4.28×10⁻¹⁹, 역행렬 residual≤4.45×10⁻¹⁶, 최종 stage-output component 반경≤5.64×10⁻¹¹을 얻었다. 요청 budget 10⁻⁹은 변경하지 않았다. 이 수치는 작은 h와 특정 모델의 검증 결과이며 모든 stiffness나 h에 대한 허용 선언이 아니다.

독립 검증은 동일 인증 알고리즘끼리의 overlap에 의존하지 않는다. χ=0이고 모델/J가 dyadic인 별도 fixture에서 전체 16×16 represented-β 선형계를 Fraction으로 정확히 풀었다. 비인과 β 원소 27개를 유지한 채 stage/output containment 18개가 모두 통과했다. 비선형 fixture는 λ=0,¼,½,¾,1에서 90자리 독립 계산으로 164개 containment를 확인했고, 여덟 self-map 부등식은 별도로 정확한 Fraction으로 재검사했다. 고정밀 근 계산은 수치 증거이며, 존재·유일성의 근거는 구간 부등식과 증명이다.

기존 `homotopy_step`은 여전히 선형화 기반 diagnostic이다. 새 API의 검증 권한을 이전 API로 소급하지 않는다. 원래 HT01/HT02의 ABSTAIN/NOT_RUN 및 expensive-client timing 의무도 소급 변경하지 않는다. 현재 H5 구현은 dense inverse 및 검증 O(n³)을 사용하므로 큰 mode 수에서의 계산 감소는 아직 입증하지 않았다.

## Window 병렬화와 실제 ODE 인증

`nonlinear_homotopy.rs`는 실제 weight 합 S를 사용하는 exact-subflow Strang candidate를 생성한다. 각 correction round의 모든 fine window는 이전 round의 입력으로 병렬 계산한다. 이어서 복소 diagonal affine map들을 고정된 tree로 합성하여 **벡터 상태**의 prefix를 계산한다. 모든 recurrence 계산이 끝난 뒤 product weighted ball에 projection하고, 표현된 중심이 실제로 허용 energy ball 안에 있는지 outward 계산으로 검사한다. 반경만 합성하는 scalar prefix와 이 벡터 계산은 별개이다.

반복 수렴의 핵심은 causal finite horizon이다. exact arithmetic에서 최초 k개 경계는 k round 후 정확해지고, error majorant가 엄격한 하삼각 행렬이므로 전역 q<1 없이도 factorial/binomial bound와 최대 M round 종료가 성립한다. 이는 parareal의 알려진 구조이며 여기서는 collective cubic 모델의 명시적 상수와 projection 이후 벡터 prefix 실현에 연결했다. 주파수 폭과 독립인 defect bound, κ=0의 약한 비선형 block 길이 조건, 전체 setup·round·최종 검증 비용을 포함한 조건부 speedup 정리는 `PARALLEL_THEORY.md`에 있다. 조건부 복잡도 감소와 실제 측정 speedup은 구분한다.

실제 candidate의 sin/cos와 vector scan은 ordinary binary64다. 그 자체에 인증 권한을 부여하지 않는다. 최종 λ=1 입력을 불변 원본 모델에 다시 결합한 독립 verifier가 각 true ODE window를 검증한다. κ=0은 source-bound packet curve, κ>0은 A03의 common-damped packet verifier를 사용한다. 해당 출력 중심의 변위까지 포함하는 local radius와 candidate successor의 차이를 residual로 만들고, 별도의 outward scalar prefix로 전역 endpoint 오차를 운반한다. 모든 요청 시점의 원래 weighted budget을 만족해야만 성공 객체가 생성된다. 실패하면 설정된 자원 한도 내에서 substep과 round를 늘리며, tolerance·χ·최종 λ는 바꾸지 않는다.

8-mode, 8-window, H=0.125, χ=0.7, width=1.2의 실행은 요청 10⁻⁵에 대해 최대 weighted radius 약 1.90×10⁻⁹을 반환했다. 7개 integration test와 2개 unit test가 통과했다. 테스트는 비선형 equal-frequency closed orbit, common damping, unequal-frequency 1/4-thread 재현성, λ/time 거부, 실패 작업량 보존, direct/continuation 비교, 원래 10⁻⁸ budget까지의 refinement를 포함한다. 벡터 prefix 순서와 표현된 projection membership도 검사했다.

## 다음 개발 단계와 판정 기준

1. **H5 인증의 구조적 역행렬화.** collective frozen J의 diagonal-plus-low-rank 구조에 대해 Woodbury/Schur 표현의 interval residual을 검증하고, 같은 W와 동일 d+Ar≤r 게이트를 유지한다. 기존 dense 인증과 exact-Fraction 선형 fixture가 겹치는지 확인하고 inverse setup까지 포함한 mode 수별 시간을 측정한다. 성공 기준은 동일 원본 target/budget과 실제 전체 비용 감소다.
2. **H5를 실제 accepted step 경로에 연결.** 새 결과는 represented stage-output 오차를 제공한다. production step으로 채택하려면 기존 accepted state의 source binding과 함께 실제 ODE 이산화/전역오차 verifier를 연결해야 한다. stage tube만으로 글로벌 정확도를 표시하지 않는다. 거부·재시도·fallback 비용을 전부 기록한다.
3. **Window verifier 비용 감소.** 현재 κ>0의 unequal-frequency verifier는 계수 변화 enclosure 때문에 subdivision이 많아질 수 있다. A03의 exact damping moments를 polynomial reconstruction과 공유하고 검증 차수를 높여 subdivision을 줄이되, 기존 original-ODE 최종 radius와 unchanged budget 게이트를 그대로 유지한다. 모든 final verification과 setup을 speed 비교에 포함한다.
4. **메모리 재사용과 mode 병렬화.** 현재 vector tree는 O(MN) 저장을 재구성하며 node 내부 mode 합성은 serial이다. 불변 mesh/coarse diagonal setup과 scratch를 재사용하고 큰 N에서 node 내부 병렬화를 검토한다. 동일 tree/order 및 1/4-thread 재현성을 유지하고 allocation·copy·prefix·verification 비용을 별도 계수화한다.
5. **Direct target와 homotopy를 비용으로 선택.** 작은 비선형 block에서는 λ=1 직접 반복이 이미 충분하다. continuation predictor가 줄인 fine round 비용이 추가 λ-stage·민감도·최종 verification 비용을 넘을 때만 선택한다. 동일 source와 budget의 양쪽 실행을 실패까지 포함해 비교하고 손익분기 조건을 router 계약으로 남긴다.
6. **Block 길이와 엄밀한 최종 경로.** 이론의 cH 제한과 factorial bound로 block/round 후보를 정하되, 실제 승인은 독립 endpoint verifier가 결정한다. 긴 구간에서는 검증된 boundary radius를 다음 block으로 전달하고, 최종 시점만이 아니라 모든 요청 시점의 budget을 확인한다. 현재 API에 없는 dense-output 또는 relative-error 보증을 표시하지 않는다.

전체 원본 proof·native source·실행 receipt는 `RESULT.json`에 묶여 있다. 이번 결과는 원래 H5의 source-bound 구현과 nonlinear endpoint certificate의 실현을 지지한다. 범용 production, 전 시간 연속 global accuracy, 기존 expensive-client campaign의 measured speed는 추가 scope별 게이트가 필요하다.
