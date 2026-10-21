# Leja: represented dense symmetric action의 전체 오차 증명과 구현

## 결과와 적용 범위

이 작업은 기존 A04 outward replay를 재사용하여

\[
F=\sum_{k=0}^4 s_k\phi_k(hA)w_k
\]

의 **반환된 fused 벡터에 대한 Euclidean 전체 오차 상계**를 닫는다. Distinct 입력이면
각 \(s_k=1\); SameVector 입력이면 모든 \(w_k=v\)이고 API에 전달된 binary64
\(s_k\)를 정확한 실수로 해석한다. \(A\)는 봉인된
`SymmetricNonpositiveOperator`에 저장된 정확히 대칭인 dense binary64 행렬이며,
Gershgorin 계산이 \(\sigma(A)\subset[-\rho,-\lambda]\),
\(0\le\lambda\le\rho\)를 outward 검증한 경우다. \(h\ge0\), 모든 입력은 유한하다.
IEEE-754 round-to-nearest binary64와 gradual underflow, 기존 `directed.rs`의 outward
산술 계약을 사용한다. 연산 overflow/허용 범위 초과는 typed refusal이다.

아래 정리는 입력의 실수 모델 불확실성, 비대칭 행렬, 임의 사용자 matvec callback,
ODE 전역 오차를 포함하지 않는다. 각 열 `columns[k]`의 개별 오차 예산은 이 API의
대상이 아니며, 모든 스케일을 합한 **fused** 출력이 대상이다. 다항식 종료 예산의
충족 여부와 상계의 유효성은 분리한다. 차수 상한에서 종료해도 성공 반환된 유한
상계는 유효하며 `converged=false`일 수 있다.

기존 `_certified` 이름의 API는 호환성을 위해 EstimateOnly 상태를 유지한다.
새 `leja_phi_action_enclosed_with_workspace`는 별도 schema
`vigilode-leja-enclosed-dense-symmetric-v1`로 이 범위의 Certified 상계를 반환한다.
독립 검토자 `/root/independent_review`가 L1–L3, 실제 public wrapper와 전체 replay,
Fraction oracle의 수학을 확인하여 이 범위 승격을 승인했다.

## 정리 L1: 전체 오차 합성

정규화 \(B=(hA-cI)/\gamma\), \(\gamma>0\)를 정확한 실수 연산으로 정의한다.
코드는 outward \([a,b]\supset\sigma(hA)\)로부터 선택한 binary64 \(c,\gamma\)에
대해 \(([a,b]-c)/\gamma\subset[-2,2]\)를 직접 검증한다. 따라서
\(B\)는 실대칭이고 \(\sigma(B)\subset[-2,2]\)다. 실제 저장된 Leja 노드
\(\xi_i\in[-2,2]\)는 정확한 실수로 해석한다. 노드가 최적 Leja라는 가정은
정확성에 필요하지 않는다.

\[
q_0(x)=1,\qquad q_j(x)=\prod_{i=0}^{j-1}(x-\xi_i),
\quad g_k(x)=\phi_k(c+\gamma x).
\]

정확한 Newton 계수를 \(d^*_{j,k}\), 계산한 중심/반지름을
\(\widehat d_{j,k},r^d_{j,k}\)라 쓰며 L2에 의해
\(|d^*_{j,k}-\widehat d_{j,k}|\le r^d_{j,k}\)다. 실제 recurrence 벡터를
\(\widehat v_{j,k}\), 초기 벡터를 \(w_k\)라 쓰고

\[
\eta_{i,k}=\widehat v_{i+1,k}-(B-\xi_iI)\widehat v_{i,k}
\]

를 정의한다. `matvec_interval_into`는 저장 행렬과 **현재 저장 벡터**의 정확한
행렬곱을 outward dot product로 감싼다. `recurrence_interval_next`는 해당
행렬곱, \(h,c,\gamma,\xi_i\)의 모든 연산을 interval replay한다. 그러므로
`interval_distance_upper`와 outward scaled L2 norm으로 얻은
\(R_{i,k}\ge\|\eta_{i,k}\|_2\)에는 cancellation 및 subnormal additive
underflow가 포함된다. relative-error 모델을 사용하지 않는다.

정확한 recurrence 오차의 telescoping identity는

\[
\widehat v_{j,k}-q_j(B)w_k
=\sum_{i=0}^{j-1}\left[\prod_{\ell=i+1}^{j-1}(B-\xi_\ell I)\right]\eta_{i,k}.
\]

대칭 행렬의 spectral calculus로
\(S_{i+1,j}\ge\max_{x\in[-2,2]}\prod_{\ell=i+1}^{j-1}|x-\xi_\ell|\)이면
각 항의 norm은 \(S_{i+1,j}R_{i,k}\) 이하이다. 기존 suffix table은 정확한 dyadic
128개 구간에서 interval product를 취하므로 이 상계가 성립한다.

차수 \(m\)에서 코드가 반환한 fused 벡터를 \(\widehat F\)라 하면

\[
\|F-\widehat F\|_2\le T_1+T_2+T_3+T_4,
\]

\[
T_1=\sum_k |s_k|\|w_k\|_2
 e^{\max(0,c+2\gamma)}
 \frac{\gamma^{m+1}}{(m+1+k)!}M_{m+1},
\]

\[
T_2=\sum_k |s_k|\sum_{j=0}^m r^d_{j,k}\|\widehat v_{j,k}\|_2,
\]

\[
T_3=\sum_k |s_k|\sum_{i=0}^{m-1}R_{i,k}
 \sum_{j=i+1}^m (|\widehat d_{j,k}|+r^d_{j,k})S_{i+1,j},
\]

그리고 \(T_4\)는
\(\widehat F-\sum_k s_k\sum_{j=0}^m\widehat d_{j,k}\widehat v_{j,k}\)
의 norm을 outward replay로 감싼 값이다. 모든 상계의 연산도 outward이다.

증명: 먼저 정확한 보간다항식과 \(F\)의 차이는 \(T_1\)로 제한된다. \(k\ge1\)에
대해

\[
\phi_k(z)=\frac1{(k-1)!}\int_0^1e^{tz}(1-t)^{k-1}\,dt,
\]

이므로 \(r\)차 도함수를 보간 나머지의 \(r!\)로 나누면
\(e^{\max(0,c+2\gamma)}\gamma^r/(r+k)!\)가 상계다.
\(k=0\)은 exponential의 직접 미분으로 같은 식을 얻는다. Hermite 보간
나머지와 spectral calculus를 적용하면 \(T_1\)이다. 코드의 `support_hi`는
정확한 \(c+2\gamma\) 이상이고 `exp_interval`의 upper를 사용한다.

정확한 보간값과 실제 basis 벡터를 쓰는 **정확한** 계수합 사이 차이에 위
recurrence identity와 \(|d^*|\le|\widehat d|+r^d\)를 적용하면 \(T_3\)다.
그 다음 계수 중심으로 교체하는 차이는 \(T_2\)다. 저장 중심과 저장 basis의
정확한 실수 곱/합은 `accumulate_coefficient_term`의 interval buffers에
포함된다. 이어서 `replay_scaled_fusion`은 스케일 곱과 최종 합을 같은 방식으로
감싸 실제 반환 배열까지의 거리 \(T_4\)를 얻는다. 삼각부등식으로 결론을 얻는다.
이 분해는 coefficient radius × recurrence defect의 cross term도 포함한다.

## 정리 L2: Opitz/Taylor 계수 enclosure

\(s=c-2\gamma\), \(x_j=c+\gamma\xi_j\)를 정확한 실수로 정의한다.
대각이 \((0,0,0,0,x_0,\ldots,x_m)\), subdiagonal이
\((1,1,1,1,\gamma,\ldots,\gamma)\)인 lower bidiagonal 행렬을 \(Z\)라 한다.
Opitz identity는 repeated node에도 divided difference의 연속 연장으로 성립한다.
따라서

\[
[e^Z]_{4+j,4-k}=\gamma^j\phi_k[x_0,\ldots,x_j]=d^*_{j,k}.
\]

\(L=Z-sI\)의 대각은 \((-s,-s,-s,-s,\gamma(\xi_0+2),\ldots)\)이고 모든
성분이 비음수다. 기존 코드는 정확한 \(s,L\)를 포함하는 interval을 만든다.
\(e^Z=e^se^L\)이고 \(e^s\)도 `exp_interval`로 포함한다.

각 열 \(v_n=L^ne_j/n!\)은 비음수다. interval recurrence가 이를 귀납적으로
포함하며, lower를 0과 교집합하는 이번 변경은 이 참된 비음수 조건을 사용한다.
\(M\ge\|L\|_\infty\)는 모든 row sum의 outward maximum이다. \(q=M/(n+1)<1\)이면

\[
\left\|\sum_{r=n+1}^{\infty}v_r\right\|_\infty
\le\|v_n\|_\infty\frac{q}{1-q}.
\]

왜냐하면 이후 모든 ratio의 norm이 \(M/(r+1)\le q\)이기 때문이다. 코드가
`term_hi`의 maximum으로 이 기하급수 나머지를 모든 성분 upper에 더한다.
`small`과 `min_terms` 조건은 종료 효율에만 관여하며, soundness는 실제 계산한
\(q<1\) 및 이 tail에 의존한다. `exp_s`와 양수 interval 곱한 뒤 midpoint에서
양 끝점까지의 거리를 outward 계산하면 `d_rad`다. 노드 중복, 매우 작은
\(\gamma\), underflow에서도 같은 포함관계가 유지되고 overflow는 반환되지 않는다.

## Scalar 및 zero-input 분기

\(h=0\), \(\rho=0\), 또는 검증된 \(\lambda=\rho\)이면 target spectrum은 한 점이다.
센터 `b`와 정확한 점의 차이를 outward `b-a`로 감싼다.
\(|\phi'_k(z)|\le e^{\max(0,b,c+2\gamma)}/(k+1)!\)를 사용한 variation 항과
scalar 계수 radius, 완전한 출력 replay를 더하면 scalar 전체 상계다.
Scalar 계수는 \(|c|\le1\)에서 64항 Taylor와 기하 tail을, 나머지에서
\(\phi_{k+1}(c)=(\phi_k(c)-1/k!)/c\)의 interval recurrence를 사용한다.
\(c=0\)에서 \(1/6,1/24\)는 outward rational interval이다.

모든 scaled input이 정확히 0이면 target과 반환값은 0이고 zero-input 분기가
유효하다. 이 분기의 출력 replay도 최종 경로에서 수행된다.

## 정리 L3: 보장되는 연산 수 감소

고정 cap \(q-1\)에서 prefix node bounds를 과거처럼 각각 재평가하면
\(128q(q+1)/2\)번 interval product가 필요하다. 이번 prefix sweep은 같은
128개 dyadic cell에서 product를 순차 연장하여 **동일한 각 prefix bound**를
\(128q\)번에 계산한다. \(q=129\)이면 1,073,280 → 16,512, 정확히 65배 감소다.
별도 suffix table은 여전히 \(O(128q^2)\)이며 이 비용을 숨기지 않는다.

차수 \(m\)까지 매 단계 factorial을 재계산하던 \(T_1\)의 다섯 coefficient를

\[
C_{1,k}=\gamma/(k+1)!,\qquad
C_{r+1,k}=C_{r,k}\gamma/(r+1+k)
\]

로 갱신한다. 양수 outward 연산의 귀납으로 원래 정확한 계수를 계속 상계한다.
이 부분의 곱/나눗셈은 \(5m^2+35m\)에서 \(10m+5\)로 감소한다.
\(T_2\)의 prefix sum도 기존
\(\sum_{j=1}^m(j+1)\)개 항 대신 초기항과 새 항만 누적하므로
\(O(m^2)\to O(m)\), storage \(O(1)\)다. \(T_3\)는 기존의 \(O(m^2)\) suffix
transport를 유지한다. Dense interval replay 전체 비용 \(O(mn^2)\)와 새 실제
wall-clock speedup은 별도 실행으로 판정한다. 정확도 예산을 완화하지 않았다.

## 구현 및 재현

- `crates/rodas5p-core/src/leja_action.rs`: prefix sweep, incremental bound state,
  positive Taylor lower intersection, scoped enclosed API.
- `crates/rodas5p-core/tests/leja_enclosed_contract.rs`: 실제 binary64 입력/output export.
- `leja_fraction_oracle.py`: 표준 Python Fraction으로 \(Q^TQ=I\),
  \(A=Q\Lambda Q^T\)를 **정확히** 검사하고 rational exponential/phi interval로
  반환 오차의 제곱 upper가 bound² 이하인지 검사한다. libm reference를 사용하지 않는다.
- node prefix bitwise equivalence와 Taylor underflow 단위검사를 추가했다.

```sh
source ../runtime/env.sh
LEJA_ENCLOSURE_RUNS=research/cloud_completion_20261018/evidence/leja_enclosure_runs.json \
  cargo test --locked --offline -p rodas5p-core --test leja_enclosed_contract
python3 research/cloud_completion_20261018/leja_fraction_oracle.py \
  research/cloud_completion_20261018/evidence/leja_enclosure_runs.json \
  --output research/cloud_completion_20261018/evidence/leja_fraction_check.json
cargo test --locked --offline -p rodas5p-core --lib cloud_
cargo test --locked --offline -p rodas5p-core --test leja_total_contract
cargo test --locked --offline -p rodas5p-core --test leja_setup_work_contract
```

검증 기록의 source/evidence identity는 통합 완료 시 작성한다. 이 문서는 원래
연구에서 닫힌 A04 replay/immutable cache/report 검토를 재사용하며 새 외부
일반 불가능성 정리나 문헌 우선권을 주장하지 않는다.

## 실제 검증 결과와 남은 개발

- 새로운 native contract 2/2 PASS: 18개 represented input fixture의 legacy/enclosed
  output 및 bound bitwise 동일성, schema/status, 미충족 예산, declared-domain refusal,
  마지막 성공 report transaction 보존.
- Exact Fraction oracle 18/18 PASS. 1500-bit 미만의 rational tail을 사용하고 실제
  행렬/고유분해 identity를 exact 검사했다. 이것은 판별 fixture 근거이고 일반 정리는
  위 L1/L2의 논증이다.
- 신규 prefix bitwise/positive-underflow 단위검사 2/2 PASS.
- 기존 total contract 19/19 PASS (새 scoped warm zero-allocation 1개 포함),
  setup receipt 5/5 PASS, holdout smoke 1/1 PASS (대규모 exporter 1개는 미실행).
- Leja compile-fail doctest 4/4 PASS, core all-target clippy `-D warnings` PASS.
- 첫 native 실행은 상대 export 경로가 crate root에서 해석되어 저장 단계만 실패했다.
  경로를 명시적으로 repo root 기준으로 해석하도록 고쳤고 같은 계산을 재실행했다.
  initial log를 유지했다. 첫 doctest 실행의 dynamic libstd 경로 오류는
  `LD_LIBRARY_PATH`에 설치된 target lib 경로를 추가한 뒤 재실행했다.
  Rust sysroot metadata 복구는 통합 런타임 기록을 참조한다.
- 독립 검토 뒤 legacy reason을 완료된 검토에 맞게 교정하면서 과거 이유 문구를
  요구하던 test assertion 하나가 실패했다. 새 의미(legacy EstimateOnly와 명시적
  enclosed API 안내)를 검사하도록 수정했고 관련 target을 다시 통과시켰다.
  수치 구현과 threshold는 이 수정에서 변경되지 않았다.

기존 holdout exporter의 `leja-cert` arm 이름은 역사적 식별자로 유지하되 실제
`total_error()`가 EstimateOnly이면 status/admitted도 이를 반영하도록 고쳤다.
과거 공개된 RUNS 자료는 변경하지 않았다.

다음 개발의 의무는 이 scoped proof의 재검증이 아니라, 필요한 경우 sparse 또는
사용자 callback의 검증 가능한 operator action 계약을 추가하고 실제 replay 비용을
동등 정확도 benchmark로 측정하는 것이다. 이 scope의 native 총오차 계약은 완료되었다.
일반 ODE production/global accuracy/speed의 상태는 통합 보고서에서 따로 관리한다.
