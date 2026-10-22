# H/D/T–He–Li 원자 네트워크: 유한 연구 모형과 근거

이 문서는 `code/atomic_network.py`의 과학적 계약이다. 구현은 원자종 이름만 등록하는 방식이 아니라 21개 핵종·전하 상태의 실제 수밀도 방정식을 제공한다. 모든 활성 광전이와 재결합 계수, 전자밀도 피드백, 중성자·삼중수소 붕괴 지도가 코드에 들어간다. 조건부 원자 연구 모형의 완주 가능성과 완전한 재이온화·핵반응 모형의 물리 검증은 서로 다른 판정이다. 원래 REC U5 HOLD, physical HOLD, global convergence NOT_RUN, production NOT_APPROVED는 바뀌지 않는다.

## 1. 상태와 초기 inventory

상태는 charge fraction이 아니라 baryon당 핵종·이온 수

\[
Y_i=n_i/n_b,\qquad \sum_i A_iY_i=1,\qquad
n_e=n_b\sum_i q_iY_i
\]

이다. 자유전자는 별도의 독립 음수 전하 상태가 아니라 neutrality로 계산한다. `A`는 질량수가 나타내는 baryon 수다. `Z`와 이온전하 `q`는 구분된다. 모형의 baryon 수밀도 환산은 배경 코드의 고정 원자질량상수 기준을 따른다. 핵결합에너지·동위원소별 정확한 rest mass를 Einstein 방정식에 새로 반영하는 작업은 아니다.

| 핵종 | 실제 상태 | H1 대비 초기 핵 수 비율 | 초기 전하 분배 |
|---|---|---:|---|
| n | n | 0 | 0 population |
| H1 | H1I, H1II | 1 | II 비율 0.0002 |
| D | DI, DII | 0.000025 | II 비율 0.0002 |
| T | TI, TII | 0 | 0 population |
| He3 | He3I, He3II, He3III | 0.00001 | 중성 |
| He4 | He4I, He4II, He4III | 0.245/(4×0.755) | 중성 |
| Li6 | Li6I, Li6II, Li6III, Li6IV | 1e-14 | singly ionized |
| Li7 | Li7I, Li7II, Li7III, Li7IV | 5e-10 | singly ionized |

위 숫자는 선택된 pilot 입력이며 관측 posterior나 정밀 BBN 예측이라고 해석하지 않는다. 모든 비율을 함께 `A·Y=1`로 정규화한다. 따라서 0.245는 순수 H–He 비율을 만드는 명목 매개변수이며 D/He3/Li까지 넣은 뒤 He4의 질량수 가중 분율이 정확히 0.245라는 뜻은 아니다.

동위원소별 전하 상태와 수 보존은 따로 추적한다. 전자 반응 계수에는 H1/D/T, He3/He4, Li6/Li7 각각의 **elemental electronic equality**를 명시적으로 적용한다. 이는 동위원소 분해 원자계수를 확보했다는 주장이 아니다. 원소별 전자계수에 동위원소 이름을 붙인 사실 자체를 독립 물리 데이터로 세지 않는다.

0 inventory인 n/T의 스케일은 1e-20이며 실제 초기 population은 0이다. 다른 핵종은 모든 전하 행에 해당 핵종 총량을 스케일로 사용한다. 적분기는 `Y/scales`를 사용해 trace Li의 절대 허용오차가 H의 스케일에 묻히지 않게 할 수 있다. 스케일은 물질을 추가하는 source가 아니다.

## 2. 실제 재사용한 근거

### 광흡수

H/He 및 Li II/III는 [Verner et al. 1996](https://doi.org/10.1086/177435)의 ground-state analytic fit과 [저자 표](https://www.pa.uky.edu/~verner/photo.html)를 사용한다. 단위 Mb를 SI로 옮길 때 `1 Mb = 1e-22 m²`를 적용한다. 선택한 수치 행은 `vendor/atomic_primary/selected_photo_rows.txt`에 보존했다. 이는 웹에서 읽은 **숫자 행의 추출**이며 전체 원격 파일과 byte 단위로 동일하다고 주장하지 않는다.

중성 Li는 64.39 eV를 넘는 광자에 valence-only 적합을 외삽하거나 흡수를 0으로 놓지 않는다. REI PR151, commit `8a4f5df46f6e3058f5e685b85cc1cf3c20926368`의 T6 contract/reconstruction을 재사용해 다음 분기를 구현한다.

\[
\sigma_{\rm LiI}(E)=\sigma_{2s}(E)+\sigma_{1s}(E),
\]

- `2s`: 5.392≤E<64.39 eV에서는 PH2; E≥64.39 eV에서는 PH1.
- `1s`: E<64.39 eV에서는 문턱 아래 0; E≥64.39 eV에서는 PH1.
- 각 partial에는 이미 해당 subshell의 provider normalization이 들어 있으므로 점유수 2를 다시 곱하지 않는다.

PH1은 [Verner & Yakovlev 1995](https://www.pa.uky.edu/~verner/photo.html)의 partial-shell 적합이다. 저자 페이지는 고에너지 범위까지의 용도를 설명한다. 이번 200 eV 사용은 해당 source 함수의 유한 부분구간이다. 기존 T6 저장 검증의 상한은 **91.83842857919419 eV**였다. 그 검증이 200 eV까지 이미 수행됐다고 확대하지 않는다. 이번 경량 검증은 122.5/200 eV를 포함한 독립 Decimal 함수 대조를 새로 수행하며, 이것은 물리적 전하·가열 yield 검증과 다르다.

중요한 별도 모형 가정은 **흡수 사건마다 전하를 한 단계 올리고 전자를 하나 만든다**는 것이다. Li I의 1s 흡수 역시 `q→q+1`로 coarse-graining한다. 원 provider가 제공하는 것은 직접흡수 단면적이며, 모든 relaxation/fluorescence/Auger/secondary-ionization yield를 제공하는 것이 아니다. 코드와 결과는 이 선택을 조건부 전하 모형으로 명시한다. 최종 전하 분포의 정밀한 원자물리적 정당성이나 직접 이중이온화의 부재를 검증했다고 주장하지 않는다.

### 재결합

H+, He+, He++의 case-A RR에는 [Verner & Ferland 1996](https://doi.org/10.1086/192284)의 [저자 표](https://www.pa.uky.edu/~verner/rec.html)를 사용한다. He I로 내려가는 fit a의 상한 1e6 K를 채택한다.

Li의 RR/DR는 REI PR150 계열의 고정 commit `fd9a21c4d3886a495540b51d780a83b608e72ea2`에서 실제 byte와 Git blob이 확인된 `lithium_coefficients.json`, Badnell RR/DR 원행을 재사용한다. RR은 [Badnell 2006](https://doi.org/10.1086/508465) 및 [RR 저자 데이터](https://amdpp.phys.strath.ac.uk/tamoc/DATA/RR/)를 따른다. DR는 [DR 저자 데이터](https://amdpp.phys.strath.ac.uk/tamoc/DATA/DR/)의 Kelvin 적합이다. He+ DR도 동일 저자 `clist_K`의 Z=2, N=1, M=1, W=2 원행을 직접 읽어 사용한다.

RR 계수는

\[
\alpha_{\rm RR}(T)=\frac{A}{\sqrt{T/T_0}
 (1+\sqrt{T/T_0})^{1-B_{\rm eff}}
 (1+\sqrt{T/T_1})^{1+B_{\rm eff}}},\quad
B_{\rm eff}=B+C\exp(-T_2/T)
\]

이며 DR는

\[
\alpha_{\rm DR}(T)=T^{-3/2}\sum_k c_k\exp(-E_k/T)
\]

이다. 원표의 cm³/s에 `1e-6`을 곱해 m³/s로 변환한다. Li q=1,2,3 RR의 하한은 각각 10,40,90 K이다. 모든 선택 채널을 함께 쓰는 runtime 계약은 **100≤T≤1e6 K**로 고정한다. 이것은 fit 오차의 전역 상한이 아니다.

He+ 및 Li+ DR에는 음수 fit amplitude 하나가 있지만 개별 amplitude를 임의로 0으로 자르지 않는다. 두 경우 모두 음수 항의 `|c3|<c1`, `E3>E1`이므로 모든 T>0에서

\[
c_1e^{-E_1/T}-|c_3|e^{-E_3/T}
\ge(c_1-|c_3|)e^{-E_1/T}>0
\]

이다. 나머지 항은 양수다. Li++의 항은 모두 양수이고 bare ion의 DR는 core electron이 없어 structural zero다. 따라서 선택된 DR 합은 양수 전이율을 구성한다.

원표와 기존 Rust provider의 CI 계수는 cold 범위 밖인 경우가 있으므로 이번 네트워크는 **모든 원소의 collisional ionization을 제외**한다. 일부 H/He CI만 넣으면서 Li cold CI의 누락을 숨기는 방식은 사용하지 않는다. CI/CX의 실제 영향이 작다는 오차 상한은 이번 연구에서 계산하지 않았다.

## 3. UV의 유한 계약과 에너지 분해

UV는 comoving 공통 유체의 local frame에서 주어진 외부 isotropic radiation field다. 전달하는 flux는 `c × photon number density`이며 각 에너지 bin에 적분된 photons m⁻² s⁻¹이다. 기본 지지구간은 5–200 eV, 광자 수 spectrum은 E^-2.5, `uv_scale=1`에서 Γ_HI=1e-12 s⁻¹이다. 이 spectrum과 정규화는 관측으로 추정한 UV background가 아니라 pilot 입력이다.

64개 logarithmic base bin을 모든 원자 threshold 및 Li I 내부껍질 edge에서 분할하면 기본 모형은 71개 bin을 사용한다. 각 bin의 photon number 적분은 power law를 정확히 적분하고, 단면적은 geometric centre에서 평가한다. 이것이 이번에 정의한 유한 양의 quadrature다. 연속 spectrum에 대한 전역 적분 오차를 인증하는 단계는 아니다. H I 정규화는 이 동일한 유한 quadrature에서 계산된다.

Li I 1s event의 primary photoelectron energy는 `E−64.39 eV`이다. ground-charge ledger의 Li I→Li II energy increment는 5.392 eV이므로 unresolved core energy

\[
X_{\rm core}=(64.39-5.392)\,\Gamma_{1s}\ {\rm eV}
\]

를 따로 보존한다. 코드가 반환하는 각 이온당 전력은

\[
W_{\rm abs}=K_{\rm primary}+\chi_{q\to q+1}\Gamma_{\rm total}
 +X_{\rm core}
\]

관계를 만족한다. `E−5.392 eV`를 전부 즉시 가열이라고 부르지 않는다. `absorbed`, `primary_kinetic`, `unresolved_core`, `li1s_absorbed`는 구별된 진단이다. RR의 number-rate fit만으로 recombining electron의 에너지 모멘트나 탈출광자의 평균 에너지를 만들어 내지 않는다.

`AtomicNetwork(uv_cutoff_eV=60)`은 source-supported soft-UV 대조를 구성할 수 있다. 이때 Li II/III 광전이는 threshold 때문에 0이고, 지정 초기조건으로 높은 Li 전하 상태는 점유되지 않는다. 네트워크가 그 상태와 RR 계수를 지원한다는 것과 특정 궤적에서 그 상태가 점유된다는 것은 다르다. 기본 200 eV 모형은 모든 원자 전하 상승 전이의 흡수율이 양수다.

## 4. 유한 양의 generator와 정확한 Jacobian

열벡터 Y에 대해 photo matrix G0, 전자당 재결합 matrix Gne, weak-decay matrix D를 둔다. 전이 j→i의 rate r은 `(i,j)`에 +r, `(j,j)`에 −r을 더한다. 따라서 physical ne≥0에서

\[
Q=G_0+n_eG_{ne}+D,\qquad
Q_{ij}\ge0\ (i\ne j),\quad
\mathbf1^TQ=0,\quad A^TQ=0.
\]

원자 전이는 해당 핵종 안에서만 이동하므로 isotope inventory도 보존한다. n/T decay는 해당 inventory를 다른 핵종으로 옮기지만 baryon 수와 핵 수는 보존한다. 전하 변화로 생성되는 전자는 `q·Y`에 자동 반영된다.

원자 RHS는 physical seconds 기준으로

\[
f(Y)=(G_0+D)Y+n_b(q^TY)G_{ne}Y
\]

이며 analytic Jacobian은

\[
J=G_0+D+n_eG_{ne}+n_b(G_{ne}Y)q^T.
\]

마지막 rank-one 항은 현재 population이 재결합 전자를 공급한다는 효과다. 별도 고정 ne의 선형 Jacobian으로 대체하면 이번 비선형 모형을 미분한 것이 아니다. 고정 x에서 nb,T,UV가 외부 입력일 때의 식이며, full cosmological state의 다른 의존성은 결합 적분기가 추가해야 한다. 독립변수가 x=ln(a/a_ref)이면 `dY/dx=f/H`이다.

implicit solver가 탐색하는 유한 signed trial Y에 대해 RHS/Jacobian은 clipping 없이 매끈한 다항식 확장을 제공한다. 실제 양의 generator 해석과 물리 허용은 비음수 accepted Y, ne에 적용된다. 음수 accepted population을 숨기는 projection은 이 모듈에 없다.

고정된 Q에 한해서 ν≥max_j(−Q_jj)와 P=I+Q/ν를 쓰면

\[
e^{tQ}=e^{-\nu t}\sum_{k\ge0}\frac{(\nu t)^k}{k!}P^k
\]

이므로 positivity와 열합 보존이 따른다. Q=0이면 identity다. 이 증명은 state-dependent ne와 변하는 T/UV/H를 가진 전체 문제를 frozen exponential 한 번으로 정확히 해결했다는 의미가 아니다. 상수 generator에서 event integral을 함께 계산하려면 [Van Loan 1978](https://doi.org/10.1109/TAC.1978.1101743)의 block exponential을 사용할 수 있다. 이번 검증은 SciPy의 작은 상수행렬 지수와 알려진 decay 해만 대조하며, 전체 적분기는 root의 결합 solver가 소유한다.

## 5. n/T, Be7, 음이온과 핵반응의 구분

중성자의 mean lifetime은 [PDG2024](https://pdg.lbl.gov/2024/listings/rpp2024-list-n.pdf)의 878.4 s를 쓰고 `lambda=1/tau`로 계산한다. half-life와 혼동하지 않는다. T half-life는 12.32 Julian years를 사용한다. 이 값과 primordial Be7/T의 조기 붕괴 맥락은 [Khatri & Sunyaev 2011](https://arxiv.org/abs/1009.3932)을 따른다.

맵은 `n→H1II+e`, `TI→He3II+e`, `TII→He3III+e`이다. T의 기존 bound electron 수를 spectator로 유지하는 근사이며 shake-off, bound-state beta branch를 새로 해결하지 않는다. main pilot의 n/T 초기량이 0이므로 실제 궤적의 해당 decay flux도 0이다. 별도 exponential 확인에서 양의 tracer를 넣어 지도와 half-life를 검사한 것은 비영 n/T 우주론 궤적을 계산했다는 뜻이 아니다. decay를 실제로 비영 초기량에 적용하는 후속 연구에는 nuclear rest-mass release와 neutrino energy ledger가 추가로 필요하다.

표준 primordial Be7는 훨씬 높은 redshift에서 Li7로 전환된다. 이번 21상태 모형은 선택한 Li7 초기 inventory에 그 전사(前史)를 포함한 것으로 정의하고 Be7를 별도 동적 상태로 넣지 않는다. late Be 생산이나 주입 시나리오의 해는 아니다.

사용자 도식의 photonuclear 과정은 MeV 규모 문턱을 갖는다. 채택한 외부 UV의 cutoff는 200 eV이므로 그 UV에 의한 photonuclear 사건은 이 모형에서 kinematically inaccessible하다. 핵융합 채널은 해당 유한 원자 generator에 포함하지 않는다. 핵융합률을 새로 검증해 0으로 판정했다는 주장은 하지 않는다.

H−/D−/T−/Li−, 분자, charge exchange는 별도 전자·분자 화학이다. [Galli & Palla 1998](https://arxiv.org/abs/astro-ph/9803315)의 Li 반응표 등 추가 근거를 실제로 검토했지만 이번 활성 집합으로 채택하지 않았다. 일부 계수를 발견한 사실을 모든 온도·source에서의 완전 closure로 확대하지 않는다. 따라서 결과를 “모든 화학종/모든 반응이 완성된 전체 EoR”로 부르지 않는다.

## 6. LRS 결합과 prescribed-temperature 에너지 계약

공통 comoving 속도와 isotropic Maxwellian gas closure를 채택하면

\[
n_g=n_{\rm nuclei}+n_e,\quad
P_b=n_gk_BT,\quad u_{\rm th}=\tfrac32n_gk_BT,\quad \pi_{b,ab}=0.
\]

원자 조성은 ne를 통해 CMB Thomson collision에 실제로 되먹임된다. 별도 collisionless massive component의 signed stress가 matter anisotropic stress를 공급한다. CMB와 massless neutrino의 signed stress는 radiation 쪽에 유지한다. 성분을 더한 뒤 Frobenius/half-contraction 규약에 맞춰 norm을 계산해야 하며, 성분별 norm을 더해 total stress로 삼지 않는다.

이번 모형은 T(x)를 100 K 이상으로 외부에서 지정한다. 비영 n/T decay가 없는 atomic pilot에서 `R_i=n_b f_i`, `R_e=Σq_iR_i`, `I_i`를 ground-charge 기준 누적 이온화 에너지로 두면

\[
u_{\rm ion}=\sum_i n_i I_i,\qquad
Q_{\rm req}=\tfrac32 k_B n_g(\dot T+2HT)
 +\tfrac32k_BT R_e+\sum_i I_iR_i
\]

는 `dot(u_th+u_ion)+3H(u_th+u_ion+P_b)`에 필요한 **net gas power density**다. 알려진 UV absorbed power를 빼서 얻는 것은 `bath input minus escaping radiation`이며 RR heat만을 뜻하지 않는다. 재결합 number coefficient로 탈출복사의 에너지를 결정할 수 없기 때문이다. unresolved core energy 역시 ground-charge 상태만 추적하는 coarse-graining의 별도 항으로 보존해 해석한다.

UV와 baryon thermal/binding energy를 배경 Einstein source에 다시 넣지 않는 test-fluid 근사에서는 관련 energy-density ratio를 실제 궤적으로 진단해야 한다. 이 비율이 작은지를 관찰하는 것과 누락 과정의 전역 오차 상한을 증명하는 것은 다르다. 완전히 닫힌 에너지 보존 우주론으로 승격하려면 UV·탈출광·열 reservoir의 stress와 전달식을 함께 정해야 한다.

## 7. 이번 경량 검증의 범위

`code/verify_atomic.py`는 source hashes, 21-state normalization, 양의 보존 generator, isotope equality, Li PH1/PH2의 source 분기와 독립 Decimal 평가, 흡수/primary/core 에너지 항등식, 21개 Jacobian column의 5점 차분, missing-rank-one 반례, frozen generator 지수 및 n/T half-population 지도를 확인한다. 새 cosmological ODE solve와 benchmark는 이 verifier에서 각각 0이다. 전체 z20→2 결과와 그림의 acceptance는 별도 결합 실행 receipt로 판정한다.

최종 수치와 실행 코드 hash는 `atomic_verification.json`을 따른다. 첫 verifier 실행에서 NumPy boolean의 JSON 직렬화가 실패한 기록은 `atomic_development.json`에 보존했다. 이를 원자 방정식의 통과로 조기 표기하지 않고 보고 형식만 고친 뒤 검증을 다시 실행했다.

문헌 검색의 실제 요청·서지 URL은 `atomic_literature_retrieval.json`, 수치와 원본 파일의 provenance/hash는 `atomic_sources.json`에 보존한다. Exa와 SciSpace를 실제 사용했으며, 검색 반환 자체를 primary rate coefficient의 근거로 대체하지 않았다.
