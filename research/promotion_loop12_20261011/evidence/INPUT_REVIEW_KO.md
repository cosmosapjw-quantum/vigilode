# Loop12 원본에 대한 독립 결정 검토

검토자: `/root/loop12_input_review`. 검토 대상은 첨부 `rvj_independent_loop12_20261011.zip`에서 전개한 `loop12_input`의 원본 주장 A–G다. 이 검토자는 원 후보, 원 실험 설계, 원 증거 생성에 참여하지 않았다. 이번 통합 연구의 새 이론 후보를 작성하지 않았으며, 원본의 수학적 충분조건과 코드 의미를 독립적으로 검토했다. 기존 수치 campaign 및 과거 VigilODE 검증은 재실행하지 않았다. 원본 파일은 수정하지 않았다.

결론은 **A, B, C, E, F, G의 명시된 범위를 PROMOTE**, **D의 수렴·자원 존재 정리를 PROMOTE**, **D에 포함된 임의 출력 횟수에 대한 축약 복잡도 식은 HOLD/NEEDS_CORRECTION**이다. 아래 보정된 비용식을 채택하면 그 비용 정리도 PROMOTE할 수 있다. 이는 조건부 이론 및 읽은 exact-reference 구현의 지위이며 production, 현재 native Rust, 일반 ODE, 실측 속도를 승격하지 않는다.

| 원본 ID | 판정 | 승인 범위 |
|---|---|---|
| A | PROMOTE | 공통 선형계수·공통 평균 피드백의 exact collective/transverse 분리와 독립 초기값 보존 |
| B | PROMOTE | 알려진 주파수·양의 가중치 모델의 kernel tail, mean defect, 모든 개별 모드 오차 상계 |
| C | PROMOTE | 동일 exact waveform map의 dense kernel coefficient-product schedule에 한정한 정확한 n배 감소 |
| D | HOLD, 부분 PROMOTE | analytic-ball 수렴과 p,D,S,b의 로그 자원 존재는 승인. 모든 R_out에 대한 최종 비용식은 보정 필요 |
| E | PROMOTE | 읽은 std-lib exact-reference online 경로의 J/JVP/VJP/유한차분/Krylov/분해 호출 0 |
| F | PROMOTE | 기존 H-dissipative 전제하의 resolvent 항등식과 오차 전달. 새 native 성능 향상이라는 의미는 아님 |
| G | PROMOTE | 명시된 제한 모델군·precision margin에서 사전 선택한 유한 자원의 오차 보장 |

## 1. 판정의 기준과 읽기 범위

Canonical GPT-6 Astra 연구 하네스 v4.0.0의 `PROJECT_INSTRUCTIONS.md`, `MODEL_ROUTER.json`, `docs/MODEL_ROUTING.md`, `state/RESEARCH_STATE.md`, phase 05를 읽었다. 원 state의 `NOT_RUN`은 배포 템플릿이며 이번 검토 상태로 상속하지 않았다. 실행 모델 identity는 UNKNOWN이고 하네스 성능은 NOT_EVALUATED다.

`RESEARCH_NOTE_KO.md`, `CLAIM_LEDGER.json`, `COST_CONTRACT.json`, `RESEARCH_CONTRACT.json`, `rvj_packet/packet.py`, `rvj_packet/convergence.py`, 두 관련 test 파일, 핵심 exact evidence를 읽었다. `MAIN_RESULTS.json`은 전체 bytes를 파싱하되 선택한 실행·오류·판정 필드만 열람했다. F의 “기존 dissipativity 가정”은 원 첨부 내부 `inputs/history/loop11_RESEARCH_NOTE_KO.md`의 §5(155–214행)에서 직접 확인했다. 원 실행 로그 `LAST_TESTS`의 23 passed/exit 0, `CONVERGENCE_TESTS`의 3 passed/exit 0도 읽었다. 이 실행 증거는 첨부에서 재사용하며, 실행 호스트를 새로 입증하거나 테스트를 독립 재실행한 결과가 아니다. 원 문헌의 신규 온라인 재검색이나 과거 원 campaign 재검증을 수행하지 않았다. 따라서 문헌 priority나 외부 원전 전체 내용은 이 리뷰의 새 승인 대상이 아니다. 상세 파일 identity와 읽기 범위는 `INPUT_REVIEW.json`에 있다.

## 2. A: exact quotient와 transverse reconstruction

가중치 합 1, 공통 A와 B, 동일 g(m)를 쓰는 모델에서 가중합을 취하면 m'=Am+Bg(m)이고 d_j=y_j−m은 d_j'=Ad_j다. 같은 초기조건을 사용하는 선형 ODE 유일성으로 y_j=m+e^{tA}(y_j(0)−m(0))를 얻는다. m0만 저장하는 축소와 달리 모든 d_j(0)를 보존해야 한다. 원문 및 코드 설계는 이를 지켰다.

P=1πᵀ에 대해 P²=P, H=diag(π_j)>0이면 HP=P*H다. 따라서 collective/transverse 분리는 H-직교 분해다. 좌표 (m,y1−y3,y2−y3)의 determinant는 π1+π2+π3=1이다. 이것은 regular chart의 구체 예이며, 모든 singular algebraic quotient가 전역 regular chart를 가진다는 진술은 아니다.

다항식 ideal 조건 Dg_a F=ΣB_ab g_b는 실제 궤도에서 g(t)가 닫힌 동차 선형계를 만족하게 하므로 g(0)=0의 불변성을 준다. Dπ F−f_red(π)∈I는 그 제약집합 위에서만 축약 동역학을 동일시한다. 이 충분조건과 명시적 reconstruction을 승인한다. Gröbner 탐색 비용이나 임의 black-box 모델에서 이런 구조의 존재·발견은 승인하지 않는다.

## 3. B: heterogeneous memory, 생략 tail, 모든 mode의 오차

정확한 variation of constants와 가중합으로 scalar Volterra 식을 얻는다. π_j>0, Σπ_j=1, |ξ_j|≤1, κ≥0이면 |K(t)|≤1이다. 에너지 항의 실제 부호는

\[
2\Re\sum_j\pi_j\overline z_j\,i\chi|m|^2m
=2\Re(i\chi|m|^4)=0
\]

이므로 E'=-2κE가 맞다. χ는 실수여야 하며 코드의 Fraction 모델이 이를 만족한다. |m|≤√E≤1에서 |g|≤|χ|, Lipschitz 상수 3|χ|를 사용할 수 있다.

실수 θ에서 e^{iθ} Taylor remainder는 복소 절댓값 기준 |θ|^{p+1}/(p+1)!로 제어된다. 시간적분하면 추가 인자 T/(p+2)가 나온다. 후보를 뺀 식에 exact K를 Lipschitz 항에 남기는 것이 중요하다. 원문은 실제로 이 분해를 사용한다. 따라서

\[
E_m\le e^{LT}\{D+\tau_p(A_0+GT/(p+2))\},\qquad
E_j\le\tau_p(|a_{j0}|+GT/(p+2))+LT E_m
\]

가 성립한다. 각 초기 진폭을 별도로 포함하므로 독립 모드 정보를 지우지 않는다. 작은 reduced defect만으로 원 모델을 수락하는 오류도 kernel tail이 방지한다.

코드는 κ=0, LT<1에서 e^{LT}≤1/(1−LT)를 이용하며, 복소 절댓값을 |Re|+|Im|로 바꾸어 보수적인 유리수 상계를 계산한다. candidate polynomial의 고차 항을 포함한 **unprojected** Volterra residual을 계산한다. candidate 생성 시 projection과 rounding으로 생긴 오차는 이 residual 안에 실제로 들어간다. 이 점은 수렴 정리의 간접 추정과 별도로 사후 인증을 지지한다.

`reconstruction_coefficients`의 계수는

\[
\frac{(i\Delta T)^j}{j!}\,T\sum_k f_k\frac{j!k!}{(j+k+1)!}
=T(i\Delta T)^j\sum_k f_k\frac{k!}{(j+k+1)!}
\]

이므로 원문의 C_p와 일치한다. 평균 reconstruction은 candidate 자체가 아니라 그 candidate에 Volterra map을 한 번 적용한 값이다. 원문과 test는 이 차이를 정확히 보존했다. κ>0 오차 정리는 승인하지만 damped client 구현은 승인하지 않는다. carrier 복원 오차는 별도의 phase enclosure를 더한 `physical_output` 계약이다. `commit`의 반환은 rotating-frame 출력이라는 기존 역할을 유지해야 한다.

## 4. C: 동일 map과 n배 product 감소의 정확한 의미

각 모드의 free profile과 kernel을 같은 degree projection으로 갱신하고, nonlinear forcing은 실제 가중평균에서 한 번 계산한다. weighted sum, Volterra convolution, 선형 degree projection이 commute하므로, 초기 free mean부터 모든 sweep의 평균이 compressed iteration과 exact arithmetic에서 일치한다. 서로 다른 위치에서 rounding하면 이 항등식이 bit identity를 주지는 않는다.

고정한 padded/dense coefficient schedule에서 각 kernel은 p+1개 계수를 가진다. 동일 forcing coefficient 수가 d_f+1이면 individual sweep은 n(p+1)(d_f+1), scalar sweep은 (p+1)(d_f+1) coefficient products를 쓴다. 이 비용의 비가 정확히 n이다. 원 구현은 kernel tuple의 길이를 유지하고 동일 forcing을 사용하므로 기록된 1785/255=7, 7905/255=31, 32385/255=127의 **특정 product-count 비교**와 일치한다. 이 세 campaign을 이번에 재실행한 것은 아니다.

부분 곱을 projection 전에 생략하는 다른 schedule, zero-skipping, 이미 contraction을 hoist한 baseline과의 비교는 이 정확한 비율의 대상이 아니다. setup, nonlinear polynomial product, output, coefficient-weight 생성 비용을 합친 총비용이 n배가 된다는 뜻도 아니다. 다만 nonlinear 평균을 baseline에서 n번 반복하도록 만드는 허수 비교는 원문에 없다.

## 5. D/G: analytic contraction, 실제 candidate tail, constructive accuracy

\(\|u\|_R=\sum_k|u_k|R^k\)를 사용한다. coefficient conjugation u*의 정의가 명시되어 있으므로 실수 시간의 비정칙 cubic을 복소 시간에서 holomorphic scalar function으로 오인하지 않는다. 이 실 Banach algebra에서 \(\|uvw\|_R\le\|u\|_R\|v\|_R\|w\|_R\), \(\|u^*\|_R=\|u\|_R\)다. Volterra beta weight j!k!/(j+k+1)!≤1이므로 convolution norm은 R\|K\|_R\|f\|_R 이하이다.

따라서 E1은 self-map 및 strict contraction의 충분조건이다. degree projection의 norm은 1 이하이고, 같은 ball에서 projected fixed point도 존재하고 유일하다. 무한차수 fixed point u의 tail은 \(B R^{-D-1}\) 이하이며,

\[
\|u_D-u\|_1\le q_1\|u_D-u\|_1+B R^{-D-1}
\]

를 이항하면 원문의 projection estimate가 나온다. S회 iteration 오차의 처음 2B bound와 단계별 rounding ε_b의 geometric 합을 적용하면 E2가 맞다. 더 나아가 T(u)=u이므로

\[
\|\hat u_S-T(\hat u_S)\|_1
\le(1+q_1)\|\hat u_S-u\|_1
\]

가 되어 사전 resource estimate가 실제 unprojected defect를 상계한다. 여기서 각 rounded iterate가 같은 ball에 남는다는 전제가 필요하며, 코드의 `mapping+epsR<=B` 검사와 원문의 strict margin이 이를 담당한다. fixedpoint의 존재만으로 arbitrary generated candidate가 수렴한다고 주장한 것이 아니다.

명시된 군의 정확 상수 q_R=1377/4096, q_1=45/256, mapping=11169/16384는 E1과 일치한다. κ=0, fixed analytic margin, bounded |Δ|T에서 factorial tail과 geometric iteration/tail은 각각 p,D,S=O(log(1/ε))를 허용한다. rounding에는

\[
2^{-b}\sum_{k=0}^{D}R^k\le B-\mathrm{mapping},\qquad
(D+1)2^{-b}\lesssim\epsilon
\]

를 동시에 요구하면 되고, D=O(log(1/ε))이므로 b=O(log(1/ε))로 충분하다. 이 결론은 정확한 arithmetic model에 관한 것으로 Fraction bit complexity를 해결하지 않는다.

`convergence.py`의 기본 `bits=112`와 `level<256`은 **유한 구현 한도**다. 이 기본 호출이 임의로 작은 모든 ε를 자동 처리한다는 결론은 나오지 않는다. 원 evidence의 tol=10^-7, p=18, D=36, S=18 및 주어진 certified upper bound는 해당 유한 계약 안에서 승인한다. 독립 oracle 실행이나 native 테스트를 새로 수행했다고 표기하지 않는다.

### 비용식에 필요한 명시적 수정

원 generic 식은

\[
W=O\!\left(np+S(D^2+pD)+R_{\rm out}(np+pD)\right)
\]

인데 최종 로그식에서 R_out pD가 사라졌다. L=log(1/ε)라 쓰면 일반적으로 보장되는 식은

\[
\boxed{W=O\!\left(nL+L^3+R_{\rm out}(nL+L^2)\right).}
\]

R_out=O(L)처럼 출력 비용 일부를 L³에 흡수할 수 있거나 n=Ω(L)처럼 pD를 np에 흡수할 수 있을 때 원 축약식도 성립한다. 아무 제한 없는 R_out에 원 축약식을 적용하는 부분은 승인하지 않는다. 상이한 시각의 모든 output을 현재 endpoint 전용 API가 이미 구현했다는 의미도 부여하지 않는다.

또한 원 코드 `conv`와 reconstruction은 각 coefficient pair에서 `math.factorial`을 반복 호출한다. n배 coefficient-product count에는 문제가 없지만, **총 arithmetic upper bound를 구현에 귀속하려면** beta weights를 재귀적으로 또는 factorial table로 한 번 O(pD)에 구성하여 재사용해야 한다. 기존 cost를 supplied coefficient weights에 대한 operation schedule로 명시하는 것도 수학적으로 가능하다. 반면 현재 literal Python의 모든 factorial 생성 비용까지 O(pD)라고 셀 수는 없다. 이 수정은 원 증명을 무효화하는 반례가 아니라, 정리의 비용 모델과 실제 reference 사이의 명시해야 할 차이다.

phase precision 비용은 원문이 이미 따로 제한했으므로 Ω와 output angle에 독립인 전체 physical-output complexity로 확장하지 않는다.

## 6. E/F 및 적용 경계

E의 승격은 AST test의 금칙어 부재만으로 결정하지 않았다. 읽은 online 경로는 Fraction scalar arithmetic, 직접 polynomial multiplication, causal convolution, moment aggregation, Horner reconstruction이다. 외부 vector field derivative callback이나 factorization oracle가 없다. 원 모델의 구조가 입력으로 주어져야 한다는 조건과 offline symbolic derivative를 허용한다는 구분을 유지한다.

F의 항등식은 γ0≠0에서

\[
R_0hJ=(R_0-I)/\gamma_0
\]

이며 scaled jet는 W_{k+1}=(R0−I)W_k다. 첨부 Loop11 §5의 실제 전제는 H>0, J*H+HJ≤0, h≥0, real γ0>0이다. x=(I−γ0hJ)y이면

\[
\|x\|_H^2\ge\|y\|_H^2+\gamma_0^2h^2\|Jy\|_H^2,
\]

따라서 \(\|(R_0-I)x\|_H\le\|x\|_H\)다. 이 전제에서는 norm claim이 맞다. ε_k에는 center action 오차와 subtraction rounding을 모두 포함해야 한다. inverse oracle 내부가 JVP를 쓰면 전체 JVP는 사라지지 않는다. 같은 항등식이 현재 repo에 이미 구현되어 있는지는 이 첨부 전용 review에서 판정하지 않았으므로, 새 native 속도 기여 여부는 owner의 source-binding 결과를 따른다.

## 7. 유지할 HOLD와 종료 판단

각 정리는 알려진 low-rank collective feedback 모델을 대상으로 한다. 일반적인 local nonlinearity, arbitrary nonnormal spectral decomposition, 입력 frequency 불확실성, κ>0 actual client, full adaptive restart, native outward f64, bit-complexity, 실측 승격은 새 증거가 없다. 이들은 수학적 반례나 실패로 재분류할 대상이 아니라 이번 승인 범위 밖이다.

Packet/moment descriptor는 원 `compile_packet`으로부터 정확히 생성됐다는 trusted-reference 계약을 따른다. 다른 compiler, approximate FFT, 외부 serialized packet으로 확장할 때 identity/roundoff를 검증해야 한다. 원본을 보안용 arbitrary-input checker로 취급하지 않았다.

이 리뷰는 원 증거의 모든 과거 숫자를 다시 생산하지 않았다. proof 단계, 잔차 정의, all-mode reconstruction, 단위·부호, 비용 계수 및 표현을 읽고 판단했다. 이번 과제의 중요한 불확실성은 위 비용식 수정과 적용 범위로 해소됐으므로 추가 메타 감사를 요구하지 않는다. 원본 bytes와 현재 판정·수정을 함께 보존하고 통합 연구로 진행하는 것을 승인한다.


## 8. 동일 review 안에서의 제한된 보정 판정: LC1–LC3

Owner가 위 두 비용 문제를 반영한 `COST_CORRECTIONS_KO.md`와 새로운 `causal_weights.py`를 제출했다. 이 후속 판정은 새 광범위 후보 연구나 리뷰의 재귀 검토가 아니라 동일한 원 주장에 대한 한 번의 수정 확인이다. 최종 bytes의 hash는 JSON에 결박했다.

- **LC1 PROMOTE:** 일반식에 p,D,S=O(ℓ)를 대입한 \(O(n\ell+\ell^3+R_o(n\ell+\ell^2))\)가 정확하다. 출력 횟수나 n의 추가 관계가 없는 원 축약 최종식은 **REJECT**한다. 원 generic 식과 contraction 정리는 보존한다.
- **LC2 PROMOTE:** \(w_{j,k+1}=w_{jk}(k+1)/(j+k+2)\), \(w_{j0}=1/(j+1)\)의 표 생성은 O((p+1)(d_f+1)) scalar 연산이다. 새 구현은 factorial을 호출하지 않으며, 모든 mode/sweep에서 동일 표를 재사용할 수 있다. 전체 cache의 추가 O(pD) 공간을 명시하여 원 streaming-space claim을 상속하지 않았다. 이는 exact scalar prototype의 승인이지 native solver 통합 승인이 아니다.
- **LC3 PROMOTE:** 모든 sweep의 kernel과 forcing을 공통 최대 길이로 zero-padding하고 영 계수도 세는 dense schedule에서는 정확한 절감량이 \(S(n-1)(p+1)(d_f+1)c_v\)다. 실제 forcing 길이를 trim하면 \((n-1)(p+1)c_v\sum_s(d_{f,s}+1)\)를 사용해야 한다. 초기 수정본은 forcing의 최대 차수만으로 모든 sweep에 같은 길이를 대입했지만, 최종본은 이 두 schedule을 구분했다. 비공통 추가 비용의 상계 C_Δ가 정확한 절감보다 작으면 총 작업량이 엄격히 감소한다. 상계 C_Δ≤10000인 예는 수치 측정 결과가 아니라 명시된 전제이며, 72390−10000=62390이라는 조건부 개선량이 맞다.

제공된 새 code check의 5개 group/104개 beta 비교 기록을 읽었다. 별도로 이 reviewer가 원 reference의 exact complex-rational C와 새 convolution의 인터페이스 일치, zero kernel, constant forcing endpoint를 작은 exact probe로 실행했으며 3/3 통과했다. 기록은 `INPUT_REVIEW_NEW_WEIGHT_CHECK.json`에 있다. 원 수치 campaign은 재실행하지 않았다.

따라서 원본 A–G의 범위별 판정과 보정 LC1–LC3의 조건부 승격을 함께 게시할 수 있다. 이 결정으로 historical HOLD를 소급 변경하거나 기존 K-form/global/native/실측 gate를 해제하지 않는다.
