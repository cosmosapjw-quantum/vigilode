# 조건부 구조 인증: 정확도·강건성 개선을 위한 원래 잔차의 선형 비용 계산

2026-10-24. 새 연구 단위의 구현은 `crates/rodas5p-core/examples/structured_residual_sweep.rs`이다. 이미 세 차례의 검토를 마친 `invariant_module.rs`는 변경하지 않았다. 본 prototype은 sparse 원래 방정식의 outward residual 인증을 실제로 실행한다. production dispatcher 통합은 다음 단계이며, local conditional campaign에 이 예제를 포함한다.

## 1. 관측된 병목과 이번 결과

앞선 N6에서는 작은 축약 solve보다 원래 조밀 잔차의 O(n²) interval 연산이 오래 걸렸다. 이번에는 **희소 구조를 입력 데이터의 정의로 소유**하여 0인 항을 수치적 추측 없이 생략한다. 같은 방정식·후보·오차 기준에 대한 인증 비용을 O(n+nnz)로 줄였다. 불변 모듈의 알고리즘을 바꾸거나 잔차 검사를 생략하지 않았다.

n=32,128,512, 대각·비정규 upper-bidiagonal·큰 상쇄 fixture에서 2개 Rust 시험과 30개 내부 negative control이 통과했다. 81개 측정 행과 27개 warmup에서 희소·조밀 interval이 각각 정확한 유리수 잔차를 포함했다. 잘못된 후보 9개는 모두 거절되고 원래 방정식의 direct fallback이 성공했다. 원시 결과와 소스 identity는 `../evidence/STRUCTURED_CERTIFICATION.*`, 요약은 `STRUCTURED_CERTIFICATION_SUMMARY.json`에 있다.

## 2. 데이터가 보장하는 구조

고정 epoch의 CSR 데이터 `(n,row_offsets,column_indices,values)`가 **행렬 B 자체를 정의**한다. 각 행의 column index는 엄격히 증가하고 범위 안에 있으며, 중복 항·비유한 값·일치하지 않는 offset을 거절한다. 저장되지 않은 원소는 정의상 정확히 0이다. 데이터는 private field로 소유되고 증명 bound와 함께 재사용된다. 작은 dense entry를 임계값으로 잘라 만드는 구조는 이 계약에 포함하지 않는다.

원래 모델이 이미 dense binary B를 사용한다면 CSR 변환 시 두 표현의 entry별 동일성을 한 번 확인해야 한다. 모델이 `B=D+UVᵀ`를 직접 정의한다면 그 real expression과 반올림해 조립한 dense B는 다를 수 있다. 같은 연산자라는 주장은 representation 계약 또는 차이 상계로 입증해야 한다. 이 prototype의 sparse-to-dense materialization은 각 저장값을 그대로 복사하므로 동일성에 추가 반올림이 없다.

## 3. 선형 비용의 안정성 인증

`S=(B+Bᵀ)/2`라 하고 각 행에

\[
 R_i=\tfrac12\sum_{j\ne i}(|b_{ij}|+|b_{ji}|)
\]
를 둔다. 저장된 offdiagonal `b_ij`마다 `|b_ij|/2`를 두 row radius `R_i,R_j`에 outward 방식으로 더하면 O(n+nnz)로 모든 radius를 얻는다. 실제 `Σ_{j≠i}|S_ij|≤R_i`다.

**정리 1.** 모든 i에서 outward upper bound가 `b_ii+R_i≤0`을 증명하면 `B+Bᵀ≤0`이다.

**증명.** S는 대칭이고 Gershgorin에 의해 `λmax(S)≤max_i(S_ii+Σ_{j≠i}|S_ij|)≤max_i(b_ii+R_i)≤0`이다. 각 radius 합을 위쪽으로 반올림하므로 부동소수점 계산은 충분조건을 약화할 수 있으나 잘못 만족시킬 수 없다. ∎

**정리 2.** 위 조건과 `α≥0`에서 `A=I−αB`는 가역이며 `||A⁻¹||₂≤1`이다. 따라서 원래 residual `r=b−Ax`의 l1 상계가 해 오차의 l2 상계이다.

**증명.** `⟨v,Av⟩=||v||²−α⟨v,Bv⟩≥||v||²`와 Cauchy–Schwarz에서 `||Av||≥||v||`가 따른다. `x−A⁻¹b=−A⁻¹r`에 적용한다. ∎

비정규 B도 이 조건을 통과할 수 있다. 고유벡터의 직교성이나 diagonalizability를 가정하지 않는다. 충분조건을 통과하지 못한 입력은 현재 구조 인증 경로에서 거절되며, 다른 inverse bound 공급자가 있으면 같은 잔차 평가를 다시 사용할 수 있다.

## 4. 반올림을 포함한 원래 잔차

각 행에서

\[
 r_i=b_i-x_i+\alpha\sum_{j\in J_i}b_{ij}x_j
\]
를 interval point input으로 계산한다. 모든 곱과 합은 기존 `directed::Interval`의 outward operations를 사용한다. 0인 항을 제거한 이유는 exact CSR representation이며, cancellation을 무시하거나 절대값이 작은 항을 버린 결과가 아니다.

**정리 3.** binary input을 정확한 실수로 해석하고 각 기본 interval operation이 포함성을 만족하면, 계산한 `I_i`는 정확한 `r_i`를 포함한다. outward 합 `Σ_i mag(I_i)`는 `||r||₁`의 상계이다.

**증명.** 초기 point interval은 binary input을 포함한다. 각 곱과 합의 포함성을 저장된 항의 수에 대해 귀납 적용한다. 생략된 원소의 기여는 정확히 0이다. 마지막 b와 x의 차 및 α 곱도 같은 포함성 아래 있다. `|r_i|≤mag(I_i)`를 합하면 결론이다. ∎

산술 underflow·overflow 처리에는 기존 directed arithmetic의 계약을 사용한다. 비유한 연산 결과는 오류로 반환하고 부정확한 작은 bound를 만들지 않는다. 본 예제는 그 library 자체의 기존 시험을 반복하지 않고 새로운 표현·상쇄·negative control을 검사한다.

입장 검사와 원래 residual 모두 O(n+nnz), 저장량도 O(n+nnz)이다. bound를 고정 epoch의 데이터에 묶으므로 각 후보에서 dissipativity를 다시 검사하지 않는다. 새로운 RHS/후보는 차원·finite 여부·epoch를 다시 검사한다. 사전에 budget을 정한 뒤 `nnz`만큼의 정확한 업무량을 허용하는 방식으로 최악 연산 수를 제한할 수 있다.

## 5. 대각+저랭크 확장 정리

다음은 proof-complete 설계이며 이번 Rust prototype은 CSR 부분만 구현했다. `B=D+UVᵀ`, `D=diag(d_i)`, `U,V∈R^{n×r}`가 불변 데이터의 정확한 연산자를 정의하면

\[
 z=V^Tx,\quad w=Dx+Uz,\quad r=b-x+\alpha w
\]
를 각각 interval dot product로 평가하여 O(nr+n) 시간, O(nr+n) 데이터, O(r+n) 임시공간으로 원래 잔차를 인증할 수 있다. 위 포함성 귀납을 두 번 적용하면 된다.

일반 저랭크 형태에 대한 충분한 inverse 조건은

\[
 \max_i d_i+\|U\|_F\|V\|_F\le0
\]
이다. 실제로 `λmax((B+Bᵀ)/2)≤max d_i+||UVᵀ||₂≤max d_i+||U||F||V||F`이다. 이 bound도 O(nr)에 outward 계산할 수 있다. 더욱 구조적인 `B=−D−UUᵀ+K`, `D≥0`, `Kᵀ=−K` 표현은 대칭부가 `−D−UUᵀ≤0`임을 직접 보여 준다. skew structure는 상삼각 자료와 부호 반전 생성 규칙으로 소유하면 된다. 일반 모델의 B를 이런 구조로 근사한 경우에는 `B−B_struct`의 residual 기여를 별도로 포함해야 한다.

## 6. 실제 시험과 비용

alpha=0.5, 대각 −2, upper diagonal 0 또는 0.25인 고정 B를 사용했다. 이들은 dissipativity를 통과하며 upper-bidiagonal 경우 비정규이다. 정답은 작은 dyadic 성분 또는 교대하는 ±2⁴⁰이다. RHS도 dyadic 식으로 구성했다. exact oracle은 `2²⁰` 배율의 i128 정수 산술로 같은 원래 잔차를 계산한다. 잘못된 후보에는 한 성분에 `2⁻¹⁰`을 더해 strict tolerance `10⁻¹⁰` 위반을 검사했다. 모든 정수와 잔차 변환은 사용 범위에서 정확 표현 여부를 assert한다.

비정규 upper-bidiagonal fixture의 중앙값, µs:

| n | 1회 sparse setup | direct solve | sparse certificate | dense certificate | solve+sparse certificate |
|---:|---:|---:|---:|---:|---:|
| 32 | 3.234 | 0.190 | 6.039 | 83.566 | 6.329 |
| 128 | 9.614 | 0.721 | 24.166 | 1449.974 | 25.058 |
| 512 | 28.784 | 2.714 | 97.026 | 22342.824 | 99.830 |

전체 fixture의 certificate-only 중앙값 비율은 dense/sparse 약 13.8–330.5였다. dense 비교군은 원래 N6에서 사용한 조밀 outward loop이며 0인 entry도 방문한다. 이번 측정은 구조를 이미 소유한 경우 이 불필요한 작업을 제거할 수 있음을 보여 준다. 같은 direct solve를 모든 mode에서 실행하고 독립 oracle 검사는 timing 밖에 두었다. setup은 별도 기록되며 반복되는 mode마다 새로 실행하지 않았다. raw rows에는 `solve_seconds`, `certification_seconds`, `total_seconds`, `fallback_seconds/count`가 구분되어 있다.

비용 해석에서 중요한 점은 sparse 인증도 이 매우 싼 direct solve보다 약 32–36배 오래 걸린다는 것이다. 따라서 **기존 조밀 인증을 대체할 때의 이득**과 **인증을 하지 않던 빠른 direct solve에 추가할 때의 비용**을 분리해야 한다. 이 실험만으로 모든 ODE step에 interval certification을 켜는 결정을 내리지 않는다. production의 conditional policy는 실제 오차 요구와 예상 재사용 횟수에 따라 선택한다.

## 7. 낮은 추가 비용을 위한 실제 다음 통합 계약

1. 원래 residual 인증이 이미 필요한 경로에서는 CSR representation으로 즉시 같은 검사를 대체한다. sparse 조건이 없으면 원래 dense/backend를 유지한다. 구조나 tolerance를 바꾸지 않는다.
2. 사용자가 강화된 선형 해 인증을 요청한 경우, `setup_budget`, `max_nnz`, `max_certificate_work`, 현재 epoch와 허용오차를 함께 설정한다. 조건이 맞지 않으면 승인된 일반 경로로 돌아가고 그 비용을 기록한다.
3. 일반 direct solver가 충분히 빠른 구간에서는 현재 prototype을 기본 경로로 승격하지 않는다. 기존 원래 residual 계산을 재사용하면서 **roundoff error만 작은 보정으로 더하는 2단계 인증**을 다음 후보로 삼는다. 첫 단계의 포함성이 증명되면 대부분의 넓은 margin 입력은 저렴하게 끝내고, tolerance 경계·큰 cancellation에서만 full interval을 호출할 수 있다. 현재 코드는 이 fast-bound 단계를 구현했다고 주장하지 않는다.
4. 구조 identity, drift·rank 변화와 pattern 변경을 negative control에 추가한 뒤 immutable CSR provider를 core module로 이동한다. N6와의 연결은 `certify_candidates` 내부의 원래 연산자 interface를 대체하는 별도 변경으로 수행한다. 현재 prototype은 이러한 통합 없이 독립 실행 가능하다.
5. conditional local campaign에서는 정확도/거절/재시도/전체 비용을 먼저 비교하고, 빠른 mode가 실제 original-error 기준을 만족한 경우에만 해당 구조 구간에 적응 선택한다. sparse certification의 속도 이득을 whole-ODE speedup으로 전환하려면 같은 초기조건·output contract의 전체 적분을 추가 측정해야 한다.

수학적 proof에 대한 독립 검토는 fixed-P 담당 agent가 Gershgorin radius, resolvent norm 및 원래 residual 전달을 별도로 유도하여 확인했다. source implementation은 두 계약 시험과 30개 negative control을 통과했다. 테스트 명령은 `cargo test --locked --offline -p rodas5p-core --example structured_residual_sweep`, 측정 명령은 `cargo run --locked --offline --profile measurement -p rodas5p-core --example structured_residual_sweep`이며 추가 CLI 인자는 없다.
