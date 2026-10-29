# A. 실대수적 인증 셀과 구조 컴파일: 반복 인증 비용을 줄이는 정리

근거 상태: **derived**. Bernstein convex-hull property와 Neumann 급수는 표준 수학이다. 기존 `outward_certificate.rs::InverseWitness::approximate`도 quadratic-stage contract에서 full-space Neumann inverse-defect witness를 제공한다. 이번 확장은 이 identity를 처음 발견하는 것이 아니라 **매개변수 셀 전체의 current-operator binding과 amortized 재사용, AS05 일반 residual-gain 연결**이다. 아래 current-operator contract·amortization·구조 compiler의 결합은 이 보고서의 적용 설계이며 학술적 신규성은 확정하지 않는다. 실행된 native 결과가 아니다.

## A1. 매개변수 셀 전체에 유효한 역연산자와 correction 인증

고정된 유한차원 subordinate norm을 택한다. 예컨대 물리적 scale s_i>0에 대해 D=diag(1/s_i)를 고정하고 B(theta)=D W(theta) D^-1, 가중 infinity norm을 사용한다. WRMS는 가중 infinity norm 이하이므로 이후 전역 전달에 쓸 수 있다. 다른 metric으로 이동할 때는 별도 transport factor를 부과한다.

실수 매개변수 theta가 box Q에 속하고 B(theta)의 성분이 theta의 polynomial이라 하자. P는 이 셀에서 고정된 square preconditioner다. affine rescaling으로 Q를 [0,1]^d에 대응시킨 뒤

E(theta)=I-P B(theta)=sum_alpha beta_alpha(theta) E_alpha

를 tensor Bernstein 형식으로 정확히 표현한다. beta_alpha>=0, sum beta_alpha=1이다. 계수의 각 성분이 검증된 interval에 들어 있다면 그 interval이 허용하는 모든 E_alpha에 대해 norm 상계 q_alpha를 구한다. q=max_alpha q_alpha<1을 outward로 검증했다면, 모든 theta in Q에 대해 다음이 성립한다.

1. B(theta)는 가역이고 ||B(theta)^-1|| <= ||P||/(1-q).
2. candidate x와 **현재 실제 목표 B(theta), b**의 residual r=b-B(theta)x에 대해
   ||x*-x|| <= ||P r||/(1-q).
3. q를 얻은 뒤 theta만 바뀌는 호출에서 같은 q를 재사용할 수 있다. polynomial 모델·cell·P·metric·assembly uncertainty의 계약은 같아야 한다.
4. inexact correction x_{j+1}=x_j+P(b-B x_j)+delta_j, ||delta_j||<=u_j이면
   ||x*-x_m|| <= q^m ||x*-x_0|| + sum_{j=0}^{m-1} q^(m-1-j) u_j.

**증명.** norm의 볼록성과 Bernstein partition of unity로 ||E(theta)||<=sum beta_alpha q_alpha<=q이다. 따라서 sum_{k>=0} E^k가 수렴해 (I-E)^-1가 되고 norm은 1/(1-q) 이하이다. PB=I-E가 가역이므로 같은 square 차원의 P와 B도 가역이다. B^-1=(I-E)^-1 P이므로 1·2가 성립한다. 불변 계약 아래 이 부등식은 Q 전체의 명제이므로 3이 따른다. 오차 recurrence e_{j+1}=E e_j-delta_j를 반복해 삼각부등식을 취하면 4다. □

finite precision 적용에서는 P r을 계산한 interval의 norm을 사용해야 한다. 계산된 residual r_hat에 ||r-r_hat||<=epsilon_r만 주어졌다면 numerator는 ||P r_hat||+||P||epsilon_r의 검증된 상계다. represented B가 아니라 B_true=B(theta)+Delta_B를 대상으로 하면 E 계수 및 추가 불확실성으로 q_total=q+||P||delta_B<1을 사용한다. arbitrary finite-difference JVP는 delta_B를 제공하지 않는 한 이 정리의 대상이 아니다. 허용된 평가 graph에서 생기는 rounding은 delta_B, residual enclosure, u_j로 각각 정확히 한 번 계상한다. finite positive denominator와 모든 finite bounds를 확인한다.

**실대수기하와 연결.** 더 일반적인 semialgebraic domain {g_j(theta)>=0}에서 rational sum-of-squares identity 또는 Bernstein subdivision으로 위 norm inequalities를 인증할 수 있다. 단순히 SDP에서 positive라고 출력됐다는 것만으로 충분하지 않다. rational identity 또는 outward PSD witness를 replay해야 한다. box theorem은 이처럼 복잡한 optimizer 없이 coefficient enclosure만으로 증명된다. coefficient 수 prod_l(p_l+1)는 d에 지수적으로 클 수 있으므로 이 경로는 낮은 매개변수 차원·낮은 degree·분리/희소 구조에서 사용한다. generic quantifier elimination의 다항시간 성능은 주장하지 않는다.

## A2. uniform certificate의 amortized 비용 감소

동일한 모델 셀을 N회 사용하는 경우를 비교한다. 비교 대상은 매 호출 current-operator gain을 새로 검증하는 경로다. 한 호출에서 그 검증 비용을 C_full, 새 경로에서 셀 조회·엄밀한 membership·계약/epoch 확인 비용을 C_member라 한다. 양쪽의 공통 residual, solve, norm 비용은 C_common이다. 새 방법의 offline compilation과 coefficient proof replay에 C_compile이 든다. calibration이나 실패 fallback 비용이 있다면 C_extra>=0으로 모두 추가한다.

W_old=N(C_common+C_full),
W_new=C_compile+C_extra+N(C_common+C_member).

따라서 C_full>C_member이고

N > (C_compile+C_extra)/(C_full-C_member)

이면 **총 arithmetic work가 엄밀히 감소한다.** 차이는 N(C_full-C_member)-C_compile-C_extra이다. 고정 차원의 box 한 개 membership은 O(d), K개의 axis-aligned 계층형 index는 실제 탐색 깊이를 포함해 O(d log K)일 수 있다. 임의 겹친 box 집합에 자동 O(log K)를 가정하지 않는다. 데이터 layout·접근과 전체 residual 확인은 남아 있다. 이는 elapsed time 정리가 아니다.

예시 cost model: C_compile=10000, C_full=1000, C_member=10, C_extra=0이면 N>=11에서 strict saving이며 N=100이면 gain verification 항은100000에서11000으로 줄어든다. 이는 명시한 arithmetic model의 수치 예시이지 현 repo의 instruction 측정값이 아니다. memory, warm-up, failed cell lookup이 실측 속도에 미치는 영향은 따로 측정한다. 현 AS05는 이 일반 q witness를 구현하지 않았으므로 production label은 올리지 않는다.

## A3. 비어 있지 않은 정확한 witness family

theta=s in [0,2]에서

B(s)=[[5/4,-s/4],[0,3/2]], P=diag(4/5,2/3)

라 두면 E(s)=I-PB(s)=[[0,s/5],[0,0]]. [0,2]의 Bernstein 계수는 E(0), E(2) 두 개이며 infinity norm은 각각0,2/5다. 따라서 q=2/5, ||P||inf=4/5, gain<=4/3이다. 더 강하게 E(s)^2=0이므로

B(s)^-1=(I+E(s))P

이며 exact arithmetic의 두 correction sweep 뒤 해가 정확해진다. 이것은 작은 q를 가정한 점근적 추정이 아니라 검증 가능한 비정규 triangular family의 finite termination이다. block diagonal direct sum으로 확장하면 block max norm에서 같은 q가 유지된다. exact full inverse norm은 4/5+2s/15이고 최대16/15이므로 4/3 상계는 보수적이지만 유효하다. 한편 직접 triangular solve도 O(n)이므로 그 비교 대상보다 빠르다는 주장은 하지 않는다. 이 예시는 soundness와 nonempty domain을 확인한다.

고정 s 한 점의 계산값을 다른 s에 재사용하는 것과, [0,2] 전체를 증명한 certificate를 재사용하는 것은 다른 행위다. 후자가 재인증 비용을 줄이는 근거다. s가 셀 밖에 나가거나 metric/P/model이 바뀌면 새 셀을 선택하거나 기존 protected solver로 되돌린다.

## A4. local expression graph가 보장하는 band와 setup 복잡도

이 정리는 source identity의 hash가 mathematical truth를 증명한다는 뜻이 아니다. **동일한 의미를 가진 식 graph로 f와 J를 생성하고, graph 연산 규칙을 검증하는 작은 kernel**을 전제한다.

모델 f_i(t,y)는 정의역 전체에서 미분 가능한 scalar straight-line circuit G_i로 표현된다. 각 G_i는 시간·상수·변수 y_j를 leaf로 가지며 검증된 primitive의 합성으로 구성된다. division 등은 정의역 조건을 별도로 인증한다. 각 node에 입력 variable support의 합집합을 전파한다. 만약 output support S_i가 [i-l,i+u]에 포함된다면, 모든 정의역에서 j가 이 band 밖일 때 partial f_i/partial y_j=0이다.

**증명.** leaf에서는 명제가 자명하다. 합성 primitive에 대한 chain rule에서 입력 subgraph 어디에도 나타나지 않는 변수의 미분은0이다. DAG의 topological induction으로 output에서 성립한다. 합집합은 취소를 발견하지 못할 수 있지만 존재하지 않는 dependency를 만들어내는 방향으로만 보수적이다. □

각 row circuit의 길이를 L_i, primitive의 산술/미분 비용을 상수로 하고, row-local reverse AD로 gradient를 출력한다고 하자. 전체 길이 L=sum L_i이면 값과 모든 band derivative의 생산 비용은 O(L+nb), 저장은 gradient output O(nb) 및 circuit workspace O(L)이다. local stencil에서 L_i=O(b), b=l+u+1이면 **O(nb)**이다. 일반 shared graph를 각 output에서 모두 역전파하면 O(nL)이 될 수 있으므로 그 경우 이 O(nb) 결론을 적용하지 않는다. L과 actual shared graph traversal은 export한다.

이 구조 계약으로 생성된 sparse J는 (t0,y0)의 몇 개 probe에 의존하지 않고 정의역 전체의 band support를 보장한다. 기존 routing.rs::verify_declared_band는 JVP callback이 있으면 이를 reference로 쓰고, 없으면 dense Jacobian을 채워 두 reference products를 계산한다. 여기서 O(n²) 병목은 후자의 dense-reference route에 해당한다. 이 검사를 production에서 단순 삭제해서는 안 된다. 새 sealed model route가 계약을 만족할 때만 별도 route에서 대체한다. arbitrary user callback은 현 validation/fallback 경로를 유지한다.

비교 대상이 dense Jacobian을 실제로 n²개 출력·검사하고, 새 route는 b=O(1)인 local circuits만 O(nb)로 처리한다면, **구조 setup의 복잡도는 quadratic에서 linear로 감소한다**. dense LU 대신 band LU를 선택할 때 일반적인 band storage/factorization도 O(nb), O(nb²) 규모이나 안정성/pivoting·verified solve 조건은 별개로 요구한다. 이미 native banded direct solver가 최강 comparator인 경우 그 solver와 비교해야 하며 O(n³) dense solver만 비교해 과장하지 않는다.

구체적인 비선형 family:

f_i(t,y)=a_i(t)y_{i-1}+b_i(t)y_i+c_i(t)y_{i+1}+g_i(t,y_i)

에서 boundary term은 존재하는 index만 사용한다. 각 g_i의 circuit 크기가 bounded이면 tridiagonal J가 whole-domain identity이며 비용 O(n)이다. b_i나 g_i의 stiffness는 이 sparsity 증명을 바꾸지 않는다. stiffness에 무관한 time-step accuracy는 이 구조 정리만으로 따라오지 않는다.

## A5. 코드에 전달할 계약

제안 타입 (현재 구현된 API 아님):

- `ModelExpressionId`: sealed graph와 primitive semantics, domain assumptions. 외부 callback에서 ID만 복사하여 authority를 만들 수 없음.
- `StructuralJacobianWitness`: row supports, circuit cost, admissible domain, generated f/J binding.
- `ParameterCellWitness`: graph/metric/P/degree/cell, outward coefficient enclosure, q<1, compilation cost.
- `CurrentCellBinding`: current parameters의 interval 전체가 cell 안에 속함; evaluation uncertainty와 model epoch 포함.
- `VerifiedResidualCorrection`: 현재 RHS와 candidate의 recomputed residual, full-space bound, target identity.

직렬화된 JSON은 설명과 proof data이다. constructor는 coefficient proofs와 model semantics를 확인한 뒤 private authority object를 생성한다. counter examples: 다른 graph의 callback, 숨은 off-band coupling, cell boundary 외부 interval, q=1, 잘못된 scale, B formation bound 누락, coefficient bit 수정, NaN/Inf. 각각 fail closed 해야 한다.

우선 적용 순서: SP03 declared structure의 **새 generated-model route** → AS05 general q witness extension → same-step/multi-RHS 및 인접 step에서 cell membership만 갱신 → AS06 전체 stage transfer → 전역오차 및 cost gate. 당장 arbitrary model에 interval inverse를 만들어 generic matrix-free solver보다 싸다고 주장하지 않는다.
