# 고정 native metric의 quartic derivative 및 rounding box 경계 — 검토 수정 반영본

단위: V8_NATIVE_METRIC_DERIVATIVE_ROUNDING_BOUNDS_20261028  
저자: /root/next6_theory  
상태: RECOVERED_THEORY_WITH_EXPLICIT_REVIEW_CORRECTION_NOT_ADMISSION

## 복구와 수정 범위

이 파일은 workspace 손실 후 의미 복구한 THEORY_DRAFT_KO.md의 구성과 증명에 부모가 전달한 독립 검토 지적을 명시적으로 반영했다. 원본 초안과 byte-identical하다고 주장하지 않는다. 정확히 복구한 제안 계약은 역사적 초안으로 보존하고, 이번 수정이 우선하는 이론 범위를 REVIEW_CORRECTIONS.json에 남겼다.

핵심 수정은 exact squared-bound \(U_m\), exact root \(\sqrt{U_m}\), precision p로 outward rounded한 \(B_{m,p}\)의 수렴을 구별하는 것이다. \(U_m\)의 단조성과 exact root의 \(O(h_m)\) 수렴은 유지한다. Fixed p=40 출력의 오차 보장은 \(O(h_m)+O(2^{-p})\)이며 일반적인 exact-limit 수렴을 주장하지 않는다. 미분 API는 실제 unary signature를 사용한다.

부모가 손실 전 전달한 freeze 방향은 depth default 2 / maximum 6, precision default 40 / maximum 128이며, 실행 예산의 복구·잔액·재실행 여부는 root가 결정한다. 이 저자는 budget을 새로 열거나 기존 실행을 되풀이하지 않았다.

## 1. 범위와 고정 근거

본 단위는 기존 native metric의 도함수와 rounding box 상계를 계산하는 증명 객체를 구성한다. Native floating point norm 평가기나 lift evaluator를 새로 만들지 않는다. 기존 source의 linear row compiler, exact scalar polynomial, Bernstein subdivision을 그대로 재사용한다. 이론 저자 작업은 소스 읽기와 기호 유도이며, 실행·native build·norm call·CAS·과학 시험·원격 쓰기를 하지 않았다. 이는 저자 작업의 기록이며 다른 에이전트나 손실 전 root 실행 예산을 초기화하는 뜻이 아니다.

BASS source는 cc602e209e7deeb511ab2a1f3b2268d55206c43c, binding_id는 8cb268e8de741e284cf494013f1e0748d27f03c5e77ba0c64413ae92af082d16이다. 차원은 reduced 30 / physical 33 / query 31이다. Native norm blob은 95efe19477aba8efe378a1f5e083c09b2cdffb69이며, native norm source SHA256은 569cc7094b19d65a432b7e55251642927d1df8346b2b10a558fefdec29d5ea8b다. Binding JSON SHA256은 54c8205f1a13467386f66ea09beb98b51dd3fffd46252691c36f7cb61ffbffc7다.

원본 핵심 경로는 이전 단위 vigilode_next_loop_20261028/deps/capsule 아래이고, 새 단위의 deps/capsule에 unchanged copy가 마련되었다는 부모의 manifest를 읽었다. 현재 복구의 byte identity 재확인은 부모가 맡는다. SOURCE_INVENTORY_KO.md는 원래 읽기 범위와 정확한 API를 기록한다.

중앙 완료수의 마지막 부모 관측은 28/88이며 이 저자 작업 자체의 중앙 증가분은 0이다. 게시 관측은 HOLD_TOOL_CALL_REJECTED_NO_ACK였다. Fresh HEAD, parallel work, AGENTS 및 현재 백업 ACK는 root가 다시 확인한다. Native owner dirty work, reserved/unknown budget은 UNKNOWN_PRESERVED다.

## 2. 재사용할 native metric과 scale 방향

Native AffineErrorNorm::evaluate(old,new,error)는 affine lift의 old/new 상태로 endpoint scales를 만들고, offset 없는 linear error lift를 physical weighted RMS로 집계한다. Row accumulation은 Neumaier compensation을 사용하고, norm accumulation은 scaled sum of squares를 사용한다. 이 floating algorithm의 유효성 조건과 arithmetic behavior를 새 Python evaluator로 복제하지 않는다.

기존 계약에서 선택한 정확한 양의 frozen physical scales \(u_j\), \(j=0,\ldots,32\)에 대한 수학적 fixed metric은

\[
N_u(e)^2=\frac{1}{33}\sum_{j=0}^{32}
              \left(\frac{L_j e}{u_j}\right)^2.
\]

이는 native source가 정의하는 fixed weighted linear metric의 증명용 수학적 대상이다. Native endpoint floating evaluation의 bitwise 값을 계산하거나 rounding equivalence를 주장하는 함수가 아니다. Existing corridor.fixed_norm_id(u,E)가 source blob, scales와 dimensions를 묶는다. B1, B2, r_query, Jop, mu, E0와 residual/barrier record는 같은 fixed_norm_identity를 가져야 한다.

실제 endpoint scales가 \(0<s_j\le u_j\)라면

\[
N_u(e)\le N_s(e).
\]

따라서 \(N_u\)의 상계만으로 더 강한 \(N_s\)의 상계를 주장하지 않는다. 본 단위는 frozen \(N_u\)에서 계산하며 metric 변경은 별도 비교 의무다. Physical scales의 upper bound를 사용하는 방향을 반대로 해석해서는 안 된다.

고정 L의 sparse rows는 다음과 같다. Index는 0부터 시작한다.

| Physical row | Reduced linear row |
|---|---|
| 0–22 | reduced 0–22의 대응 identity |
| 23 | \(-e_{23}\) |
| 24 | \(e_{23}\) |
| 25 | \(-e_{24}-e_{25}\) |
| 26 | \(e_{24}\) |
| 27 | \(e_{25}\) |
| 28 | \(-e_{26}-e_{27}-e_{28}\) |
| 29 | \(e_{26}\) |
| 30 | \(e_{27}\) |
| 31 | \(e_{28}\) |
| 32 | \(e_{29}\) |

각 reduced coordinate의 identity physical row가 있으므로 이 fixed L은 full column rank다. 각 행 안에서 nonzero coefficients는 같은 부호다. 이 두 속성은 일반 affine lift에 자동으로 성립하는 가정이 아니며, 고정 binding에서 확인한 source 속성이다.

## 3. 기존 capsule과 row compiler에서 physical derivatives 얻기

기존 parse_capsule은 represented bits, source binding, clock, arithmetic identity 및 30-state reference data를 검증한다. 물리 clock은 \(x=\ln(a/a_{\rm ref})\), \(a_{\rm ref}=1/21\)이며 physical interval length는 \(\Delta=x_1-x_0>0\)다. Stored Delta triple과 endpoint subtraction의 exact identity를 유지한다. Native h는 별도 필드다. 이 둘을 바꾸어 쓰지 않는다.

\[
\theta=(x-x_0)/\Delta,
\]

\[
q_i(\theta)=y_{0i}
 +(y_{1i}-y_{0i}+d_{0i})\theta
 +(d_{1i}-d_{0i})\theta^2
 +(d_{2i}-d_{1i})\theta^3
 -d_{2i}\theta^4.
\]

이 식은 기존 row.state_power(q)가 제공한다. 새로운 reference polynomial source를 정의하지 않는다.

각 fixed physical row에 query-index terms \(\{i+1:L_{ji}\}\)를 만들고 기존 row_data(0,terms,q,rho)를 호출한다. Derivative용 rho는 31개의 0이다. 기존 row compiler가 반환하는 \(H_j(\theta)=L_jq(\theta)\)를 사용한다. 새 lift evaluator를 만들지 않는다.

\[
P_{j,1}(\theta)=\Delta^{-1}H'_j(\theta),\qquad
P_{j,2}(\theta)=\Delta^{-2}H''_j(\theta).
\]

도함수 차수는 각각 3 이하, 2 이하다. Physical cell \([\ell,r]\)의 normalized endpoints는
\(a=(\ell-x_0)/\Delta\), \(b=(r-x_0)/\Delta\)이고, 기존 E.substitute(P,a,b)가 \(P(a+(b-a)t)\)를 만든다.

순서는 반드시 전역 \(\theta\)에서 미분하고 \(\Delta^{-k}\)를 적용한 뒤 cell을 제한하는 것이다. Local t로 미분한 뒤 \(\Delta^{-k}\)만 적용하면 cell width의 chain factor가 잘못된다. Native h를 physical \(\Delta\) 대신 넣지 않는다.

실제 E.derivative API는 unary derivative(p)다. 2차 미분은 E.derivative(E.derivative(H_j))로 호출한다. Proposed contract의 derivative(H_j,k)는 수학적 약기이며 실제 함수 signature가 아니다.

## 4. 공통 dyadic leaf의 Bernstein 경계

Requested finite depth m에서 \([0,1]\)을 \(2^m\)개의 공통 dyadic leaves로 나눈다. 모든 33개 physical rows와 두 미분 차수에 같은 leaf partition을 쓴다. Leaf 수와 계산 cap을 작업 전에 확인한다.

각 restricted polynomial은 degree 4의 5 power coefficients로 zero-pad한 뒤 기존 power_to_bernstein과 split_midpoint를 사용한다. Quartic-only exact certificate 엔진을 degree 6으로 확장할 필요가 없다. 이 방법은 cubic derivative를 먼저 제곱해 degree 6 polynomial로 만드는 방법과 다르다.

Leaf I에서 \(P_{j,k}\)의 Bernstein coefficients를 \(\beta_{j,k,I,\ell}\)라 하면

\[
M_{j,k,I}=\max_{\ell=0,\ldots,4}
                       |\beta_{j,k,I,\ell}|
\]

이고 Bernstein basis의 비음수성과 partition of unity로

\[
|P_{j,k}(t)|\le M_{j,k,I}\qquad(t\in I).
\]

따라서

\[
Q_{k,I}=\frac1{33}\sum_{j=0}^{32}
              \left(\frac{M_{j,k,I}}{u_j}\right)^2,
\qquad
U_k^{(m)}=\max_{I\in{\cal P}_m}Q_{k,I}
\]

는

\[
T_k:=\sup_{\rm cell}N_u(q^{(k)})^2\le U_k^{(m)}
\]

를 준다. 스칼라 outward square-root certificate를 적용해 \(B_k^2\ge U_k^{(m)}\)를 만든다. Leaf endpoints, coefficients, M, Q, maximum U, scalar root enclosure를 감사 가능한 evidence로 남긴다.

중요한 계산 순서는 leaf마다 행들의 제곱 상계를 합한 뒤 leaf maximum을 구하는 것이다. 각 행의 전역 최대를 먼저 구해서 제곱합을 만들면 서로 다른 peak 위치를 동시에 달성한다고 취급하므로 refinement로도 사라지지 않는 gap이 남을 수 있다.

## 5. Exact squared-bound 단조성 및 수렴

De Casteljau child coefficients는 parent coefficients의 convex combinations이므로 각 child에서 \(M_{\rm child}\le M_{\rm parent}\)다. 같은 leaf의 모든 행을 집계하므로

\[
U_k^{(m+1)}\le U_k^{(m)}.
\]

이 단조성의 직접 대상은 정확한 유리수 squared-bound U다.

원래 cell-local row polynomial을 \(p_j(t)=\sum_\ell a_{j\ell}t^\ell\)로 쓰고

\[
S_j=\sum_\ell|a_{j\ell}|,\qquad
K_j=\sum_\ell\ell|a_{j\ell}|
\]

로 둔다. Leaf \([a,a+w]\subset[0,1]\)에 제한한 power coefficients는

\[
c_r=w^r\sum_{\ell\ge r}{\ell\choose r}
                       a_{j\ell}a^{\ell-r}.
\]

그러므로

\[
\sum_{r\ge1}|c_r|
 \le\sum_{\ell\ge1}|a_{j\ell}|((a+w)^\ell-a^\ell)
 \le wK_j.
\]

Bernstein coefficient와 \(p_j(a)\) 사이의 거리는 \(wK_j\) 이하이고, leaf 위의 함수값과 \(p_j(a)\) 사이의 거리도 \(wK_j\) 이하이다. 따라서 leaf의 임의의 t에서

\[
|p_j(t)|\le M_{j,I}\le |p_j(t)|+2wK_j.
\]

\(h_m=2^{-m}\), \(T=\sup N_u(p)^2\)라 쓰면

\[
0\le U^{(m)}-T
 \le E_m:=
 \frac1{33}\sum_j
 \frac{4h_mK_jS_j+4h_m^2K_j^2}{u_j^2}.
\]

따라서 exact squared-bound는 \(O(h_m)\)으로 수렴한다. \(T>0\)인 고정 다항식에 대해 exact root의 오차는

\[
0\le\sqrt{U^{(m)}}-\sqrt T
 =\frac{U^{(m)}-T}{\sqrt{U^{(m)}}+\sqrt T}
 \le\frac{E_m}{2\sqrt T}=O(h_m).
\]

여기서 수렴하는 대상은 exact root \(\sqrt{U^{(m)}}\)다. Precision p로 upward rounding한 출력 \(B_{m,p}\)를 같은 대상으로 취급하지 않는다. 아래의 scalar certificate를 함께 사용하면

\[
0\le B_{m,p}-\sqrt T
 <\frac{E_m}{2\sqrt T}
       +2^{-p}\sqrt{T+E_m}.
\]

따라서 rounded output의 보장된 gap은 \(O(h_m)+O(2^{-p})\)다. p=40을 고정하면 mesh refinement만으로 exact supremum에 수렴한다고 일반적으로 주장할 수 없다. Exact root가 표현 가능하거나 exact rational-square fast path가 있는 특수 경우에도 실제 종료 bound가 그 값을 달성했다는 근거를 별도로 확인한다. 일반적인 정확한 limit 주장은 mesh를 줄이면서 p도 증가시켜 \(2^{-p}\to0\)인 경우에만 따른다.

\(T=0\)이면 양의 weights 아래 모든 physical row polynomial이 identically zero다. 따라서 exact \(U^{(m)}=0\)이고 zero branch는 \(B_{m,p}=0\)을 정확히 반환한다. 위의 \(1/\sqrt T\) 식을 이 경우에 적용하지 않는다.

## 6. Scale-aware scalar square-root certificate

\(Q=0\)이면 exact zero를 반환한다. \(Q>0\)이면 정수 e를

\[
4^e\le Q<4^{e+1}
\]

가 되도록 선택하고 \(z=Q/4^e\in[1,4)\)로 둔다. 정밀도 p에 대해 integer square-root와 exact squared comparison으로

\[
m=\left\lceil2^p\sqrt z\right\rceil,\qquad
B=2^{e-p}m
\]

을 구성한다. Rational numerator/denominator를 사용하므로 floating log나 floating sqrt에 의존할 필요가 없다. 원래 rational이 perfect square인지 정확하게 검사하여 그 exact root를 반환하는 fast path는 가능하다.

\[
\sqrt Q\le B<\sqrt Q+2^{e-p}
                 \le(1+2^{-p})\sqrt Q.
\]

Root certificate는 lower² ≤ Q ≤ upper²를 exact rational arithmetic으로 검사 가능하게 출력한다. 예를 들어 \(10^{-616}\) 같은 작은 Q에 고정 absolute \(2^{-40}\) 격자를 적용해서 유용성을 잃지 않도록 exponent를 분리한다. 이것은 양의 스칼라에 대한 enclosure 함수이며 vector norm/lift API가 아니다. 고정 p에서 남는 root rounding error와 leaf refinement error를 구분한다.

## 7. Rounding box의 fixed-metric 반경

비음수 reduced component radii \(\rho_i\)가 주어지면 query-coordinate radii를 \((0,\rho_0,\ldots,\rho_{29})\)로 만든다. 기존 row_data가 반환하는 row support

\[
\sigma_j=\sum_i|L_{ji}|\rho_i
\]

를 사용한다. 새 L-vector evaluator를 구현하지 않는다.

\[
Q_\rho=\frac1{33}\sum_j\left(\frac{\sigma_j}{u_j}\right)^2
\]

이면 모든 \(|e_i|\le\rho_i\)에 대해 \(N_u(e)^2\le Q_\rho\)다. 특히 이번 fixed L의 각 행은 nonzero coefficient가 같은 부호이므로 \(e=+\rho\)가 모든 \(|L_je|=\sigma_j\)를 동시에 달성한다. 따라서 \(Q_\rho\)는 independent component box의 정확한 최대 squared radius다.

이 성질은 arbitrary mixed-sign L에 대한 일반 정리가 아니므로 실제 binding source와 row-sign 확인에 묶는다. Box maximum의 attaining corner는 실제 floating-query error의 witness가 아니다. Signed point error의 exact norm이라고 반환해서도 안 된다. He 예제에서 \(e_{24}=1,e_{25}=-1\)의 squared value는 \(2/33\)이지만, \(\rho_{24}=\rho_{25}=1\)인 box maximum은 \(6/33=2/11\)이다.

Query-specific 경로는 기존 query_membership(capsule,clock_bits,state_bits)의 PASS를 먼저 요구한다. Source-derived uniform rho box와 그 query의 component_errors box를 분리해 기록한다. Endpoint exact-copy이면 actual component error가 zero이므로 해당 knot의 r_query도 zero다. Uniform bound를 zero로 축소하는 뜻은 아니다.

## 8. 손계산 reference cases와 검증 설계

아래 값은 이론에서 도출한 기대값이다. 이 저자 작업에서 실행했다는 결과가 아니며 원본 또는 복구 과정에서 PASS 수를 선취하지 않는다.

| Case | Expected result / purpose |
|---|---|
| Zero quartic / zero box | \(B_1=B_2=r_q=0\). |
| Linear reference | \(B_1\)의 physical scale은 \(1/\Delta\), \(B_2=0\). Native h만 바꾸어도 reference derivative는 바뀌지 않는다. |
| All-coordinate smoothstep | \(q_i=3\theta^2-2\theta^3\), \(\Delta=2\), \(u_j=|\sum_iL_{ji}|\)이면 \(U_1^{(0)}=1\), \(U_1^{(1)}=9/16\), \(U_2=9/4\). True \(B_1=3/4\), \(B_2=3/2\). |
| Half cell smoothstep | \(\theta\in[0,1/2]\)에서 \(U_1=9/16\), \(U_2=9/4\). Local derivative chain factor 누락을 검출한다. |
| Asynchronous peaks | \(p_0=t,p_1=1-t\), 나머지 0, \(u=1\). True squared supremum \(1/33\). \(n=2^m\) common leaves이면 \(U_n=(1+1/n^2)/33\). |
| Asynchronous numeric rationals | \(n=1,2,4\)에서 \(2/33,5/132,17/528\). Global row maxima를 먼저 합산하면 \(2/33\)에 머무른다. |
| Uniform fixed-source box | 모든 \(\rho_i=r\), \(u_j=|\sum_iL_{ji}|\)이면 \(Q_\rho=r^2\). \(r=1/8\), \(r=10^{-308}\) 모두 exact rational roots. |
| He neutral coupling | \(\rho_{24}=\rho_{25}=1,u=1\)이면 \(Q_\rho=2/11\). Neutral row를 빠뜨린 \(2/33\)은 잘못된 값. |
| Root scaling | 작은/큰 nonsquare Q에서 lower² ≤ Q ≤ upper² 및 relative precision certificate를 유지한다. |
| Refusals | nonpositive scales, source/metric mismatch, negative radii, bad clock/partition, cap/depth 초과, failed membership, generated record collision을 거부한다. |
| Integration | 생성된 B1/B2/r_query를 unchanged residual/piecewise chain으로 전달하되 다른 bound와 provenance 의무를 유지한다. |

Smoothstep capsule의 represented coefficients는 \(y_0=0,y_1=1,d_0=-1,d_1=2,d_2=0\)이다. Asynchronous example은 \(\Delta=1\), \(q_0=\theta^2/2,q_1=\theta-\theta^2/2\)로 구성할 수 있다. \(y_0=0,y_1=(1/2,1/2),d_0=(-1/2,1/2),d_1=d_2=0\)이 해당 reference를 준다.

PROPOSED_EXACT_CHECKS.wl은 위 scalar identities의 제한된 CAS 검산 입력이다. 입력 작성은 CAS 실행이 아니며, native model 또는 전체 과학 인증의 대체물이 아니다.

## 9. 구현 계약과 residual 연결

제안 API의 범위는 다음과 같다.

1. cell_derivative_bounds(capsule,physical_left,physical_right,u33,depth,precision)는 fixed-cell B1/B2, exact squared bounds, common leaf evidence, clock/source/metric identity를 출력한다.
2. query_metric_bounds(capsule,clock_bits,state_bits,u33,precision)는 기존 membership 결과와 uniform/query-specific box bounds 및 typed r_query record를 출력한다.
3. box_envelope_certificate(nonnegative30rho,u33,precision)는 source-bound supports, exact Qrho와 scalar root enclosure를 출력한다. 임의의 signed point vector norm 요청을 받지 않는다.
4. generated_residual_transfer는 기존 capsule/partition/cells 계약에 generated bounds를 결합하고 unchanged residual_transfer를 호출한다. 기존 사용자 필드를 조용히 덮어쓰지 않고 collision을 거부하거나 root가 명시한 semantics를 적용한다.

Generated records는 left/right physical cell, exact query bits/knot, source binding, arithmetic identity, fixed metric에 묶는다. 기존 proof_status string만으로 native owner provenance가 생기지는 않는다.

이 단위가 제거할 수 있는 외부 입력은 source-bound reference quartic에 대한 B1/B2와 component rounding box에서 fixed-metric r_query를 얻는 작업이다. Actual residual evaluation error, source error, Jop, Tx, lognorm mu, inherited/initial error, actual native capsule의 owner record 등은 자동으로 제거되지 않는다. 이 경계들을 같은 fixed metric에서 제공해야 unchanged residual와 piecewise comparison으로 이어진다.

## 10. 남는 의무와 완료 상한

Actual application은 유효한 owner-origin capsule과 model-bound records가 확보될 때까지 BLOCKED_INPUT이다. REI의 25-state client는 별도 binding이며 현재 source-derived 결과를 그 client에 이식했다고 주장하지 않는다.

Actual query_rounding_bound와 actual_query_rounding_bound는 값이 발급되기 전까지 null을 유지한다. Actual ODE solution enclosure와 whole_domain_certificate는 false, VerifiedModelProvider는 미발급이다. V7 full restore는 NOT_RUN, production은 HOLD, globalphysicalcompletion은 null, L4는 false다. 연속 각도 positivity, residual/Jacobian/lognorm/inherited/initial/source bound 의무는 별도다.

닫힌 과거 캠페인이나 불명 budget을 재실행·초기화하지 않는다. 이 문서는 source-derived scalar certificates의 구성과 그 증명을 제공한다. Fresh scope-matched independent review와 root의 frozen contract/execution gate를 대신하지 않는다.
