# Independent technical review — 2026-10-18

검토자: 별도 `independent_review` agent. 기준 commit `2fc61b944fe431d40ba455c373579bce00b0c39a`. 작성자의 판정을 전제로 채택하지 않고, 변경 source와 증명을 읽어 식을 독립적으로 다시 전개했다. 이 검토는 기존 캠페인을 재실행하지 않았다. 실행 결과는 root가 보존하는 명령·로그에 귀속되며 아래 판단은 별도 source/algebra 검토이다.

## 판정

교정된 T1–T3는 명시한 전제 아래 **PROMOTE**한다. native 코드 수정은 아래의 좁은 결함 교정으로 채택한다. packet의 전역 production enclosure 및 Leja total-error certification은 **HOLD**다. 정확도 계약을 맞춘 wall-time 우위도 이 검토로 승격하지 않는다. 이론 결과의 유효성과 아직 구현하지 않은 전제 검증을 구분한다.

## 독립 유도와 확인

- **T1:** weighted real Hilbert 공간에서 collective derivative의 symmetric quadratic form은 `δmᵀ H(m) δm`이다. `H`의 trace는 0, determinant는 `−χ²|m|⁴`이고 mean map의 제곱 norm은 `S=Σw`이다. 따라서 최대 Rayleigh quotient는 정확히 `S|χ||m|²`; 상수 방향 perturbation으로 등호에 도달한다. 각 주파수의 skew 항과 damping을 더하면 `μ_w(Df)=−κ+S|χ||m|²`이다. 에너지 항등식·segment 평균 bound·Dini derivative를 결합한 integrating-factor bound, 출력 `√S e` 및 `e/√w_j`, 두 exact flow의 지수 3→1 개선이 성립한다. 수정 후 `Y(0)=â(0)` 조건이 명시되었다. 적용할 residual은 실제 출력 곡선의 연속 residual이어야 한다.
- **T2:** Bernstein convex hull과 이항계수 비의 0..1 범위에서 `sup||p||≤U_B≤U_C`를 독립 확인했다. 제한된 derivative control points의 norm은 원래 `M1` 이하이며 cell의 control point 차이를 telescoping하면 `δM1`가 나온다. `d=0`을 별도 정의하고, 실제 outward 계산에서는 두 유효 상한의 minimum으로 dominance를 보존한다. `(1−τ)^12`의 4096→1은 exact bound 개선이다.
- **T3:** exact binary64 입력 product를 interval로 옮긴 뒤 beta, Taylor remainder, backward exponential-moment recurrence, binomial 합의 각 연산이 inclusion을 유지한다. 성공 반환만을 대상으로 하며 overflow는 거절된다. 초기 beta 이후 `B_(n+1)=B_n(j+n+1)/(j+k+n+2)`의 induction이 `O(k+N)` 비용을 준다. N28,k64의 기존 division1950에는 ordinary seed30과 interval1920이 섞여 있었으며 수정 표가 이를 구분한다. 큰 z에서 폭이 작다는 보장은 없다.
- **Leja:** step-i residual은 `prod_(r=i+1)^(l−1)(X−ξ_r)`로 전파되므로 prefix polynomial을 재사용할 수 없다. 현재 `suffix[i+1][j]` index가 맞고, 매 degree에서 한 항씩 누적하면 T3 coefficient-update 비용이 O(m³)에서 O(m²)로 줄어든다. bound setup O(128m²), memory O(m²)는 별도 비용이다. 실제 `X`의 Hermitian spectral interval 전제에만 이 operator bound를 적용한다.
- **Packet:** normalized-time restriction은 `c_k→τ^k c_k`; 수정은 이를 force/convolution 전에 적용한다. 기존 Volterra mean bound `B=(D+K+E0)/(1−hL)`와 component bound `local+hL E0/(1−hL)+δ_j`는 정확한 support/weight/domain 전제 아래 맞다. 출력 mean이 재구성 Q일 때에도 `|m−Q|≤E0+K+hLB=B−D≤B`이므로 추가 D inflation은 필요 없다. 새로운 initial guard는 outward `S_upper E_upper≤1`을 검사하여 초기 불확실성의 true mean domain을 확보한다. 기존 softened detuning/weight와 일반 f64 certificate의 별도 의무는 남는다.
- **Sparse/split:** canonical CSC 검사 후에만 checked sparse view를 만들고 retained symbolic pattern과 exact 배열 비교를 수행한다. refactor 시도 전 numeric_ready를 해제하므로 실패 뒤 stale factor를 쓸 수 없다. split 검사는 NaN mismatch를 통과시키지 않는다. Gray–Scott N2의 중복 periodic 이웃을 합산한 canonical column은 원래 5-point periodic stencil과 일치한다.
- **Evidence API:** LP09 v2의 independent error·allocation `null`, 별도 discrepancy, `DIAGNOSTIC_ONLY`, `fairness_verified=false`는 실제 실행 범위와 맞는다. Leja는 자체 outward norm을 쓰지만 additive underflow 의무를 `EstimateOnly`에 남긴다. public forged cache insertion을 막고 내부 evaluator를 통한 precompute만 허용한다.

## 검토에서 발생한 지적과 종료

1. **IR-01 / high / CLOSED:** Leja T2는 coefficient radius×computed basis norm, T3는 midpoint coefficient×basis error만 써서 coefficient-radius/basis-error 교차항이 빠졌다. 수정 T3는 `|d_mid|+d_rad`를 사용한다. `d* r*−dmid rhat=(d*−dmid)rhat+d*(r*−rhat)`로 모든 항을 덮는다.
2. **IR-02 / high / CLOSED:** `safe_l2`는 certificate factor의 outward upper norm이 아니다. 새 scaled directed division/multiplication/sum/sqrt helper가 exact stored vector norm의 상계를 만든다. 전체 Leja underflow proof는 별도 HOLD로 표시했다.
3. **IR-03 / medium / CLOSED:** initial uncertainty energy만으로 |m|≤1을 주장하려면 exact Σw1이 필요했다. 새 guard는 실제 positive weights의 outward 합 S까지 곱한다. 이 수정은 domain guard만 닫으며 packet kernel mass/support 계약 전체를 승격하지 않는다.
4. **IR-04 / high / CLOSED:** T1c의 임의 Y에 대해 initial radius e0를 사용하려면 Y(0)=â0가 필요하다. 최종 증명과 machine-readable 전제에 명시되었다.
5. **IR-05 / low / CLOSED:** T2 d0 경계와 outward dominance 문구를 구분했고, T3 division-count의 ordinary/interval 종류를 수정했다.

## 남는 범위

packet은 outward coefficient·force·continuous residual·tail·reconstruction·output rounding을 묶는 native 검증이 필요하다. 정확한 model identity/support/weight 계약도 먼저 닫아야 한다. Leja는 underflow 포함 recurrence/accumulation residual을 완결해야 `EstimateOnly`를 벗어날 수 있다. benchmark는 독립 reference와 같은 출력·정확도·setup/solve/allocation 계약이 필요하다. 수정된 API에는 의도적인 호환성 변화가 있다: cache raw insertion 비공개화, sparse matvec의 Result 반환, packet benchmark schema v2와 optional measurements. 이 변화의 실행 검증은 root의 최종 targeted tests에 연결한다.

이 검토의 정확한 source/proof hash는 `independent_review.json`에 있다. 최종 native 실행과 publication은 root가 별도로 기록한다.

## 새 저랭크 직접해법 및 통합 보고서 추가 검토

앞서 종료한 T1–T3를 재검토하지 않고 새 `WOODBURY_THEORY.md`, exact Fraction reference, source-bound 결과 JSON을 검토했다.

**WB1/WB2: PROMOTE_CONDITIONAL.** 실수 Jacobian은 `diag(D_j)+UCV`이고 `S=I−sHC`, `H=Σπ_jP_j`이다. `q=S⁻¹Σπ_jP_jb_j` 및 `x_j=P_jb_j+sP_jCq`를 대입하여 A·x=b를 직접 확인했다. singular S의 null vector도 A의 nonzero null vector로 옮겨지므로 가역성 동치가 맞다. `||C||=3|χ||m|²`, `||P_j||≤1/a`, weighted U/V norm1에서 WB2의 두 Neumann 상계 및 stage residual-to-error 추정이 나온다. 가정은 positive normalized weights, finite real model parameters, nonnegative damping/shift, 조건에 맞는 가역 Schur matrix다. native floating interval 구현은 아직 없다.

정확 reference의 primitive operation 호출을 세면 각 우변은 N개의 block action6N, 평균4N, Schur와 cubic action12, shift2, block correction8N으로 합계 `18N+14`이다. 현재 small-gain branch의 setup은 `19N+51`이다. R개 완전한 출력을 만들 때 `O(N+RN)` 계산 및 출력 보관 비용을 포함한다. input validation, parsing, unary sign, allocation/loop와 rational bit complexity는 별도다. 이 비용 증명은 dense LU baseline 대비 stage 선형대수 복잡도 개선이며 native runtime 우위를 뜻하지 않는다.

**IR-06 / medium / CLOSED:** 첫 문서는 현재 CollectiveModel의 주파수에 stored carrier Ω를 더했다고 썼지만 source rhs/jvp는 Δξ만 쓴다. 최종 문서는 일반 physical-frame reference와 현재 rotating-frame adapter를 구분하며 native 연결 시 carrier0을 요구한다. 저자는 nonlinear carrier0 family를 추가했다. 현재 원출력의 64개 exact solves는 Gaussian reference 및 exact residual 검사를 통과했고, 추가 sufficient-gain-outside 및 singular cases를 보존한다. 검토자는 실행을 중복하지 않고 source hash와 case별 count/총합/선형 cost law를 재조정해 확인했다. 작성자가 파일을 갱신하던 순간의 첫 identity reconciliation은 불일치로 종료했으며, 변경 완료 후 최종 source hash에 대한 reconciliation이 PASS했다.

LP09 v2의 revised source/test에서 independent reference와 allocation의 `None→null`, diagnostic discrepancy, fairness false를 확인했다. 통합 REPORT_KO의 현재 결론은 이 검토의 conditional theorem 채택, packet/Leja/비교속도 HOLD 범위와 일치한다. 기본 경로의 새 수치 결과는 root 실행 원자료에 귀속되며 본 검토가 별도 실행으로 재확인했다고 주장하지 않는다.

마지막 Leja status-string workspace 재사용 변경도 확인했다. 기존 EstimateOnly 상태·이유·bound를 유지하면서 매 호출의 새 String 생성을 피한다. 새로운 수학식 변경은 없으며 해당 source hash를 갱신했다.
