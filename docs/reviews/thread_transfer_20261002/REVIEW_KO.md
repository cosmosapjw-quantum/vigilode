# Vigilode 연구 통합 리뷰: method 교체보다 현재 blocker의 분리·해소

작성일: 2026-10-02. 성격: 소스에 근거한 탐색적 수학·코딩 리뷰와 독립 재현 패키지.

## 0. 결론과 게시 상태

현재 가장 직접적인 개발 경로는 **기존 RODAS5P의 raw-stage 좌표변환을 JVP-only workspace driver로 옮기는 것**이다. 이미 explicit-J/LU fast driver가 사용하는 대수를 재사용하므로 새로운 시간적분법을 발명하거나 RVJ5를 production에 이식할 필요가 없다. 다음 순서는 (i) native coefficient/target 차이와 residual 예산의 대응을 닫고, (ii) JVP-only 경로의 RHS JVP와 allocation 비용을 줄이고, (iii) homotopy 반경 시도 정책과 Laguerre recurrence certificate를 각각 독립된 연구 단위로 진행하는 것이다.

이번에 실제로 산출한 결과는 네 가지다.

1. R4 homotopy 입력의 독립 enclosure 재현에서 n=8,16 실패는 기존 여섯 번의 반경 탐색에서 발생한다. 각각 7번째 D=4.096, 8번째 D=16.384에서 닫힌다. 반경이 전혀 존재하지 않는다는 증거가 아니었다. 기존 R4 FAIL을 다시 PASS로 쓰는 것이 아니라, 실패 원인을 더 좁힌 결과다.
2. 같은 nilpotent path sum을 전체 합행렬 대신 벡터에 직접 적용하는 항등식을 검증했다. s=8이면 형식적 dense matrix-matrix product가 5회에서 2회로 줄고 matrix-vector product가 3회 필요하다. 실제 zero-skip/block 비용과 병렬 speedup은 별도다.
3. Laguerre의 signed output-adjoint polynomial과 정확한 Bernstein 범위경계를 결합해, 시험한 degree 16/32/64 finite-polynomial recurrence error의 경계를 기존 절댓값 majorant보다 약 7.37배에서 8.24e13배 줄였다. exp/phi 절단·계수 생성·총 부동소수점 인증을 완료했다는 뜻은 아니다.
4. JVP-only 연구 reference 12개에서 raw-stage 좌표변환으로 RHS 조립용 JVP를 step당 7회에서 0회로 줄였다. 현재 연산자에 대한 Krylov JVP와 true residual 검사는 남겼다. 최대 endpoint 차이는 약 2.32e-12이며 native faer 계수 비트와의 일치는 아직 미검증이다.

**원격 push는 이 실행에서 완료하지 못했다.** 연결된 GitHub에서 조회 기능은 작동했으나 노출된 action에는 write/commit/push가 없었다. 직접 Git 네트워크 접근도 `Could not resolve host: github.com`으로 실패했다. Repository 자체의 사용자 write 권한이 없다고 판정한 것은 아니다. GitHub에 실제로 게시한 commit SHA나 push acknowledgement를 만들지 않는다. `publication/PUBLICATION_STATUS.json`과 기존 브랜치 전용 receiver를 함께 제공한다.

이 리뷰는 native benchmark를 새로 승인하거나 timing authority를 변경하지 않는다. 원 solver, 계수 snapshot, 기존 ledger/FAIL, protected 경로, holdout, clock/output 계약을 수정하지 않는다. 새 production 연구는 저장소의 사전등록 절차를 다시 따라야 한다.

## 1. 검토한 버전, 최신성, 자료 범위

대상 저장소는 `cosmosapjw-quantum/vigilode`다. Default `main`의 8월 상태를 최신 연구로 오인하지 않고 refs와 최근 연구 head/PR를 대조해 다음 기존 작업 브랜치를 선택했다.

```
branch = claude/jolly-wozniak-7wl15h-wu25-stiff-benchmark
scientific_source_commit = d1e9ba3b0125ee478c28d0b2c280ca0869289c15
scientific_source_tree   = 25e00400a0b8a76ee41171e717094344069a1b28
```

110개 branch 이름을 열거하고 관련된 최근 head들을 비교했으나, 모든 과거 branch의 모든 commit을 일일이 재심사한 것은 아니다. 연구기록과 실제 코드의 기준은 위 immutable commit이다.

작업 중 다시 조회한 동일 브랜치 HEAD는 다음과 같았다.

```
publication_base_commit = f2797d8891eb7201da31576080f2118bb23b7b1b
publication_base_tree   = da648cf9ea3263849dee0df6b923bff76483ba1c
parent                  = d1e9ba3b0125ee478c28d0b2c280ca0869289c15
```

위 첫 추가 commit은 `docs/reviews/20261002_thread_transfer/REVIEW_PROTOCOL.md` 30줄만 추가했다. 전체 patch를 읽었고 solver와 probe 입력에 변경이 없음을 확인했다. 그 원격 protocol을 이 실행의 push receipt 또는 사전등록으로 소급 계승하지 않는다. 본 패키지는 별도 create-only 경로 `docs/reviews/thread_transfer_20261002/`를 사용한다. 해당 문서를 덮어쓰지 않는다. 게시 receiver는 이 최신 검토 base와 local/remote HEAD가 정확히 일치해야만 작동한다.

### 최종 관찰된 문서 추가와 이 리뷰의 순증분

최종 재확인에서 동일 branch는 `687def63d3f6d24297d37bc1cb1a99558be2d71f` (tree `83777d81ea4e0bb1976df365412df27a63251b28`)까지 진행되었다. `docs/reviews/20261002_thread_transfer/REVIEW_KO.md`의 새 보고서 diff에서 결론·blocker 지도·초기 수학 결과를 읽었다. 전체 새 sibling evidence를 모두 재실행한 것은 아니다. 최신 root의 crates=`7372765fffddcc76ecf49aa590ec78c7c74e7521`, fixtures=`38538ba873c31352f5f0737851b7f2aff607e43c`, research=`9efbb7729142a972db83a737b7de322ca7a9669c`가 기존 source와 같아 본 probe의 실제 solver·입력·native 연구기록은 변하지 않았다.

새 원격 리뷰도 common-radius cap과 action-first path sum을 다룬다. 따라서 이 두 내용은 최신 저장소에 전혀 없던 제안이라고 포장하지 않고 **독립 확인과 후속 native 실행 계획**으로 취급한다. 본 보완 패키지의 추가 중심은 **matrix-free raw-stage residual/precision 계약**과 **Laguerre signed output-adjoint Bernstein certificate**다. 새 원격 리뷰가 지적한 운영 경계도 중요하다. 현재 q2 admission의 순차 certificate와 R4의 병렬 radius 연구를 구분하며, 연구 fixture의 Euler-like candidate가 enclosure를 가진다는 것만으로 실제 q2 승인률 또는 outer-step 수락이 좋아졌다고 말하지 않는다.

게시 receiver의 최종 base는 위 `687def63...`다. 원격에 관찰된 별도 리뷰의 게시를 이 패키지의 push receipt로 계승하지 않고, 기존 폴더는 모두 보존한다. 추가 업데이트가 있으면 receiver가 중단한다.

정독 범위는 README, 연구 ledger 정책, R4 closure와 비용 연구, 최신 fast-v2 사전등록/결과, stage-target 및 outward certificate 핵심, polynomial action의 domain·recurrence·합산·normalization, sequential/fast driver의 실제 stage 루프와 mass/JVP 인터페이스다. 원시 benchmark 전체 6MB trace와 모든 Rust source, 모든 과거 PR을 읽었다고 주장하지 않는다. 관련 함수의 읽은 line ranges와 blob identity는 `SOURCE_REGISTRY.json`, `READ_COVERAGE.json`에 기록했다.

로컬에 full git checkout과 Rust toolchain이 없으므로 `cargo test`, native CVODE/Radau 실행, 실제 Rayon/MPI speedup, repo 전체 lint는 실행하지 않았다. 이를 Python 연구용 reference의 성공으로 대체하지 않는다.

## 2. 프로젝트의 현재 상태를 먼저 정확하게 읽기

### 2.1 최신 RODAS5P fast-v2는 이미 상당한 순차 비용을 제거했다

`rodas5p_fast.rs`는 raw stage U를 사용해 RHS의 Jacobian product를 제거했고, 전체 적분 workspace, in-place Jacobian callback, rejection 시 현재 상태에서의 재사용, zero-skipping LU 및 row extent 추적을 구현했다. L-0033은 35개 문제·허용오차 조합에서 v1의 endpoint, accepted/rejected, RHS/Jacobian/factorization count가 동일하다고 기록한다. 이는 **저장소가 보고한 native 결과**이며 여기서 native run을 다시 수행한 것은 아니다. [R-fast-v2, R-fast]

최신 gate FAIL의 의미는 특정한 성능 예측의 실패다.

| 대상 | v2/v1 instructions per attempted step | 사전 기준 | 기록된 결과 |
|---|---:|---:|---|
| Brusselator n=400 | 0.397 | <=0.6 | 통과 |
| HIRES | 1.011 | <=0.95 | 실패 |
| van der Pol | 0.973 | <=0.95 | 실패 |

작은 dense 시스템에는 banded zero-skip의 이득이 거의 없다. 저장소는 이 FAIL을 보존하면서 large-n 비용 절감 코드를 유지한다. 따라서 “RODAS5P는 step은 적지만 언제나 BDF/Radau보다 느리다”라는 초기 동기를 현재의 모든 regime에 적용하면 안 된다. Native CVODE/Hairer 비교도 이미 추가되어 있다. README의 초기 comparator 부재 설명은 active benchmark 상태와 구분해야 한다. Matched-error walltime 표는 진단일 뿐 timing authority의 HOLD는 유지된다. [R-fast-v2]

### 2.2 아직 비싼 matrix-free 경로는 다른 코드 경로다

`sequential_stages_refined`는 K stage마다 delta, stage state, RHS, gamma mixture, 보고서 벡터들을 만든다. `gmix != 0`이면 별도의 JVP를 RHS에 추가한다. 반면 lean fast driver는 explicit Jacobian/LU를 전제로 하므로 JVP-only 경로의 작업공간 개선까지 완료한 것이 아니다. 여기서 fast driver의 대수를 matrix-free 경로로 옮길 여지가 생긴다. [R-sequential, R-fast]

Cached tableau, sealed witness, caller-owned pool, generic stage count의 doubling levels, component-blocked certificate는 이미 구현되어 있다. 이들을 새로 발견한 미구현 기능으로 제안하지 않는다. Remaining cost의 정확한 함수와 조건을 대상으로 해야 한다.

### 2.3 인증하는 대상은 계속 분리해야 한다

프로젝트의 `StageCertificate`는 선언된 stage equation의 정확한 root와 candidate 사이의 거리다. Embedded bound는 그 target의 embedded estimate에 대한 bound이며 ODE 오차의 증명은 아니다. 이 구분은 저장소가 이미 올바르게 표현하고 있다. [R-outward, R-stage]

다음 네 양을 동일한 `PASS`로 합치지 않는다.

- 선형 solve residual과 현재 operator identity.
- Candidate와 정확한 stage target의 거리.
- Embedded truncation proxy.
- 물리 norm에서의 실제 local/global ODE error.

이 스레드의 RVJ 반례는 이 구분을 강화하는 회귀자료다. 그것만으로 기존 RODAS5P 자체에도 동일한 반례가 성립했다고 결론내릴 수는 없다.

## 3. 수학·코딩 연구 A: raw-stage 변환을 matrix-free driver로 이전

### 3.1 정확한 변환과 mass matrix

한 step 동안 M과 현재 J가 고정되어 있고, 필요한 shifted operator가 가역이라고 하자. M은 상수 mass matrix다. 아래는 stage 대수 정리이며 singular-M DAE의 적분 정확성까지 승인하지 않는다.

Raw coefficient를 A,C,gamma라 하고

\[
\Gamma^{-1}=I_s/\gamma-C,\qquad U=\Gamma K,\qquad\alpha=A\Gamma
\]

로 둔다. K와 U는 state increment 단위를 가지며 계수는 무차원이다. 기존 K residual은

\[
r_K=(I_s\otimes M-h\Gamma\otimes J)K-h\mathcal F(y+\alpha K)-h^2\gamma_{\rm rows}\otimes f_t.
\]

그러면 U에 대한 stage equation은

\[
\boxed{(M-h\gamma J)U_i
=h\gamma f(t+c_i h,y+\sum_{j<i}A_{ij}U_j)
+\gamma M\sum_{j<i}C_{ij}U_j
+h^2\gamma\gamma_i f_t.}
\]

W action 안의 JVP는 유지하지만 RHS의 J Gamma-mixture는 없다. M=I이면 C-mixture는 단순 vector combination이다. Nonidentity M에서는 mass action의 비용도 센다.

**증명.** \((I_s-\gamma C)\Gamma=\gamma I_s\)이고 stage/state Kronecker factor를 구분하면

\[
\big[(I_s-\gamma C)\otimes M-h\gamma I_s\otimes J\big]U
=\gamma(I_s\otimes M-h\Gamma\otimes J)K.
\]

Nonlinear stage argument도 AU=alpha K이므로 두 residual은

\[
\boxed{r_U=\gamma r_K}
\]

를 만족한다. M과 J가 서로 commute할 필요는 없다. 임의의 2x2 M,J 및 3-stage raw C를 사용하는 Wolfram symbolic 계산으로 이 항등식을 확인했다. `results/WOLFRAM_RECEIPT.json`에 결과가 있다.

### 3.2 중요한 구현 계약: residual 예산과 coefficient precision

같은 physical component scale을 사용한다면 이상적인 stage 대수에서 WRMS residual도 gamma배다. 따라서 같은 숫자의 tolerance를 양쪽에 그대로 전달하면 안 된다. 기본 대응은

\[
\tau_U=|\gamma|\tau_K
\]

이며 native target allowance와 roundoff floor를 함께 처리해야 한다. RHS norm 기반 상대 tolerance는 RHS 자체가 바뀌므로 별도로 재계산한다. 현재 연산자에 대한 true residual check를 없애는 최적화로 해석하지 않는다.

저장소의 Gamma는 faer inverse에서 만들어진 binary64 행렬이다. Alpha/L의 미세한 upper leakage와 diagonal 차이 때문에 native target과 이상적인 exact transform은 동일한 객체가 아니다. 기존 `StageTarget`은 이 문제를 명시적으로 다룬다. [R-coeff, R-stage]

Native K target에 쓰는 strict-lower Gamma와 stated gamma diagonal을 S, native strict-lower alpha를 alpha0라고 하자. U=SK에서 raw transformed residual과 native K residual의 차이는

\[
\begin{split}
r_U(SK)-\gamma r_K(K)
={}&\big([(I-\gamma C)S-\gamma I]\otimes M\big)K\\
&-h\gamma\{\mathcal F(y+ASK)-\mathcal F(y+\alpha_0K)\}.
\end{split}
\]

따라서 \(D_\Gamma=(I-\gamma C)S-\gamma I\), \(D_\alpha=AS-\alpha_0\)를 outward arithmetic으로 감싸고, stage tube의 Lipschitz bound를 사용해 이 차이를 인증할 수 있다. Output에는 \(b_{\rm code}^TS-b^T\), embedded에는 \(e_s^TS-\tilde b^T\), dense output에는 \(H_{\rm raw}S-D_{\rm dense}\)의 allowance가 추가된다.

이 계약을 닫기 전에 native target의 bit identity를 요구하거나 상이한 residual의 동일성을 주장해서는 안 된다. 기존 `block_sequential_allowance`의 방식은 참고할 수 있지만, 두 대상이 다르므로 그대로 동일 함수라고 할 수도 없다. 공식 coefficient snapshot을 임의의 높은 정밀도 rationals로 교체하지 않는다.

### 3.3 실행한 reference

`probes/matrix_free_transformed_probe.py`는 analytic JVP-only quadratic/time-dependent RHS를 사용한다. n=2,8,16, h=0.001/0.05, M=I 또는 비단위 constant diagonal mass로 12개 조합을 비교했다. GMRES는 현재 shifted action을 호출하고 true residual을 확인한다. Jacobian assembly count는 0이다.

- RHS 조립용 JVP: 모든 경우 7 -> 0.
- Endpoint 최대 차이: 2.318811809232102e-12.
- Embedded 최대 차이: 3.4799339094445555e-13.
- 여전히 8 sequential stages와 8 linear solves가 필요하다.
- Nonidentity mass의 추가 action과 Krylov의 다른 stopping pattern도 기록했다.

이 reference의 Gamma는 NumPy로 재유도했으므로 faer native bit replay가 아니다. 작은 endpoint 차이는 numerical agreement이지 새로운 order proof 또는 같은 adaptive mesh의 보장이 아니다. High-order AD도 구현하지 않았으며 이 최적화에는 필요하지 않다.

**개발 판단:** 일반 RVJ 이식보다 이 이전을 우선한다. 기존 method, coefficient, controller를 유지한 채 이미 검증된 raw-stage 대수를 JVP-only 경로에 적용하는 제한된 변경이기 때문이다. 첫 production 연구는 M=I와 current-J target으로 제한하고, 이후 mass case와 recycled preconditioner를 독립적으로 확장한다.

## 4. 수학·코딩 연구 B: homotopy radius 실패의 원인을 구체화

### 4.1 현재 충분조건의 의미

저장소의 strict-lower target, inverse witness U, residual enclosure r에서 nonnegative seed \(a=U|r|\)를 만든다. Common state radius D가 주어지면

\[
H(D)=H_0+DH_1,\quad H(D)^s=0,\qquad
E(D)=\sum_{k=0}^{s-1}H(D)^k a.
\]

B를 absolute alpha state-increment map이라 하면 반경 허용조건은

\[
\boxed{BE(D)\le D\mathbf 1.}
\]

이는 정의된 majorant가 닫힌다는 충분조건이다. Radius schedule의 실패는 target root의 부재, Newton/Jacobian의 intrinsic singularity 또는 모든 radius의 부재를 뜻하지 않는다. Polynomial quadratic fixture에서는 \(BE(D)-D\mathbf 1\) 자체가 유한차수 다항식이므로 필요하면 Sturm/interval/Bernstein으로 feasible interval을 조사할 수 있다. 하지만 이 경우는 무거운 전역 소거 전에 단순한 탐색 범위 점검으로 원인이 드러났다.

### 4.2 실제 R4 입력으로 재현한 결과

`r4_studies::homotopy_cost_study`는 n=1,2,4,8,16, h=0.05, diagonal a_i=-1-i, q_i=-0.05(1+i mod 3), y_i=1+0.1i를 사용한다. 모든 candidate stage는 h f(y)다. 반경은 D=0.001에서 시작해 네 배씩, 최대 여섯 번 검사한다. [R-hom-study, R-hom-generator]

이번 계산은 fixture의 native coefficient hex bits와 exact coupling interval endpoints를 읽고, 입력 binary64 값을 정확한 rational로 해석했다. 양수 경계는 2^-100 dyadic grid로 위쪽 반올림했다. 유효한 독립 enclosure이지만 Rust의 directed binary64 연산순서와 bit-parity는 아니다.

| n | 기존 6회 schedule | 최초로 닫히는 확장 시도 | D | 필요한 state radius |
|---:|---|---:|---:|---:|
| 1 | 닫힘 | 4 | 0.064 | 0.0194156992848 |
| 2 | 닫힘 | 5 | 0.256 | 0.0890638411236 |
| 4 | 닫힘 | 6 | 1.024 | 0.395299869522 |
| 8 | 실패 | 7 | 4.096 | 2.30684411937 |
| 16 | 실패 | 8 | 16.384 | 15.0064407455 |

**현재 n>=8 blocker의 정확한 해석은, 이 두 source-informed reference case에서는 반경 시도 한계가 너무 작았다는 것**이다. Native Rust 재현용 `native_replay_homotopy.rs`를 제공했지만 이 환경에는 Rust가 없어 실행하지 않았다. 원래 R4 cost study의 bit-parity gate를 통과했다고 보고하지 않는다.

\(H(D)\)가 nonnegative monotone이므로 이상적인 정확 산술에서

\[
D\ge\|B\sum H(0)^k a\|_\infty
\]

는 이 common-majorant criterion의 필요조건이다. 현재 residual로 계산하므로 exact stage oracle가 필요하지 않다. Reference에서는 이 zero-radius quantity의 1.1배를 제안하고 모든 부등식을 다시 확인했을 때 다섯 case 모두 닫혔다. n=8은 proposal 2.5164116, required 2.2994028; n=16은 proposal 16.2532979, required 15.0045740이었다.

1.1은 이번 source-informed probe의 휴리스틱이며 보편적 보장이 아니다. Native 구현에서 preflight를 공식적인 lower bound로 사용할 때에는 rounding 방향과 admission functional의 동일성을 따로 증명해야 한다. 우선은 **proposal만 생성하고 기존 enclosure로 재검증**하는 용도로 사용한다. 실패하면 reject/fallback하고 모든 시도를 남긴다. Preflight의 scalar/serial 작업도 비용에서 빼면 안 된다.

### 4.3 Common radius 자체가 원천적으로 나쁜 경우도 있다

이번 actual fixture의 cap 문제와 별개로 다음 exact triangular system을 생각하자.

\[
k_0=1,\quad k_1=1+k_0^2,\quad k_2=1+k_1^2.
\]

Zero candidate의 root는 [1,2,5]로 유일하다. Common D majorant는 E0=1, E1=1+D이므로 stage 2의 state radius가 1+D가 되어 모든 D>=0에서 닫힐 수 없다. 하지만 stage-dependent radii [0,1,2]는 즉시 닫힌다.

따라서 per-stage/per-component D를 허용하는 것은 단순한 숫자 tuning보다 구조적인 일반화다. Strict causal stage system에서 기존 serial recurrence가 만드는 radius를 proposal로 재사용할 수 있다. 다만 **이 serial construction은 이미 저장소에 있는 알고리즘이며 공짜의 병렬화가 아니다.** 새로운 부분은 anisotropic box의 독립된 재검증·cache·certificate binding 계약이다.

### 4.4 합행렬을 만들 필요가 없는 path-sum action

현재 full doubling은 S=I,Q=H로 두고 S<-S+QS, Q<-Q^2를 사용한다. 최종적으로 E=Sa를 얻는다. E만 필요하면 다음으로 바꿀 수 있다.

```
e = a
Q = H
for level in 0..ceil(log2(s)):
    e = e + Q e       # RHS 전체에 이전 e 사용
    if 다음 level이 있으면 Q = Q Q
```

정확한 arithmetic에서

\[
e=\prod_{\ell=0}^{L-1}(I+H^{2^\ell})a
=\sum_{j=0}^{2^L-1}H^j a
=\sum_{j=0}^{s-1}H^j a.
\]

H가 nonnegative이고 모든 연산을 위쪽으로 감싸면 별도의 유효한 upper bound가 된다. 순서가 달라져 원래 binary64 S 방식과 bit-identical하지 않을 수 있다. Requirement는 새로운 enclosure의 정당성과 target binding이지 무조건 bit identity가 아니다.

s=8,L=3에서 형식적 dense 작업은 기존 5 matrix-matrix products + 1 matrix-vector에서 2 matrix-matrix + 3 matrix-vector로 바뀐다. 이미 component-blocked 경로가 있으므로 그 작은 stage block에 적용해야 한다. 전체 m=sn 행렬을 새로 dense화해서 이득을 없애지 않는다. 실제 zero-skip scalar operation·allocation·critical path를 다시 세어야 한다.

`path_action_probe.py`는 width 1,3,8,9,16에서 full sum, direct action, serial causal result의 exact equality와 4개의 잘못된 입력 거절을 검사했다. Production H의 nilpotency는 state dimension이 아니라 stage blocks s에 의해 결정된다. 해당 블록 구조가 증명된 뒤 log2(s)를 사용한다.

### 4.5 반경을 닫아도 여전히 비용 gate가 남는다

저장소의 idealized budget은 P workers에서

\[
(7-p_1)\lceil8/P\rceil+8p_f
\]

이고 serial은 8 W-solves다. P=8에서 certificate·dispatch·추가 RHS 비용이 들어갈 여유는

\[
1+p_1-8p_f
\]

뿐이다. 기록된 n=1,2,4,8,16의 여유는 각각 0,1.25,0.30,1.0714,0.6190 W-solve 단위다. [R-hom-study]

Radius attempts를 늘리는 것만으로는 이 작은 여유를 더 소모한다. 그러므로 native closure regression을 닫은 뒤 **candidate quality, residual preflight, action-first certificate, measured fallback율**을 같은 source-fixed cost model에서 비교해야 한다. Formal depth 3이라는 숫자만으로 speedup을 선언하지 않는다.

## 5. 수학·코딩 연구 C: Laguerre 출력 오차를 부호를 유지하며 전파

### 5.1 기존 majorant가 커지는 이유

`polynomial_action.rs`의 대상은 주어진 다섯 vectors에 대한 exact exponential phi-combination이다. RVJ의 rational phi와 다른 계산이다. Domain은 self-adjoint, nonpositive operator 및 검증된 spectral enclosure다. Laguerre recurrence의 local rounding residual은 이미 interval로 감싸지만 절댓값 majorant는 음의 두 번째 계수의 cancellation을 버린다. 따라서 매우 큰 bound를 줄 수 있다. 현재 `EstimateOnly`는 정직한 상태이고 그대로 보존해야 한다. [R-poly]

Laguerre recurrence 자체는 DLMF 18.9의 고전적 3항식이다. Bernstein coefficient를 이용한 rigorous polynomial range enclosure는 certified roundoff 연구에서도 사용된다. 여기서는 이 둘을 아래 output-adjoint identity로 직접 연결했다. [W-DLMF, W-Bernstein]

### 5.2 Signed discrete adjoint 정리

정확한 self-adjoint X의 spectrum이 [0,L]에 있고, t_n=L_n(X)w라 하자. 저장된 real coefficients c_n를 정확한 입력값으로 고정한 finite polynomial output은

\[
p_m(X)w=\sum_{n=0}^{m}c_n t_n.
\]

부동소수점 recurrence의 정확한 local defect를 delta_j라 쓰면

\[
\hat t_{n+1}=a_n(X)\hat t_n+b_n\hat t_{n-1}+\delta_{n+1},\quad
 a_n(x)=\frac{2n+1-x}{n+1},\quad b_n=-\frac n{n+1}.
\]

초기 \(\hat t_0=w\)는 주어진 stored vector와 정확히 같다. 입력 normalization 오차는 별도의 기존 budget에 둔다.

Backward polynomials를

\[
z_{m+1}=z_{m+2}=0,\qquad
z_j(x)=c_j+a_j(x)z_{j+1}(x)-\frac{j+1}{j+2}z_{j+2}(x)
\]

로 정의하면

\[
\boxed{\sum_{n=0}^{m}c_n(\hat t_n-t_n)
=\sum_{j=1}^{m}z_j(X)\delta_j.}
\]

증명은 recurrence에 z를 곱하고 합하여 중간 error coefficient를 telescope시키면 된다. Wolfram에서는 degree 5의 임의 residual symbol을 사용해 두 식의 차이가 정확히 0임을 확인했다.

따라서 \(\|\delta_j\|_2\le\epsilon_j\)와

\[
\beta_j\ge\sup_{x\in[0,L]}|z_j(x)|
\]

를 인증하면

\[
\boxed{B_{\rm recurrence}=\sum_{j=1}^m\beta_j\epsilon_j}
\]

는 최종 finite-polynomial output의 recurrence error를 감싼다. Self-adjoint spectral theorem을 쓰므로 eigenvalues만 알려진 nonnormal matrix에는 적용하지 않는다.

### 5.3 Bernstein certificate와 exact arithmetic probe

각 z_j를 [0,L]에서 Bernstein basis로 바꾸면 range는 coefficient의 convex hull 안에 있다. Dyadic de Casteljau subdivision을 반복하고 subinterval별 max absolute coefficient를 취하면 전 구간의 bound다. Grid의 maximum을 bound로 착각하지 않는다. 이번 prototype은 exact Fraction, exact dyadic subdivision, integer square-root 기반 outward norm을 사용한다. Arbitrary real signed c_n에 대한 정리는 유지되지만 실행 시험은 exp-Laguerre형 geometric stored coefficients를 사용했다.

| m | L | 시험 | 기존 majorant / 새 recurrence bound |
|---:|---:|---|---:|
| 16 | 1 | diagonal 3D | 7.37 |
| 32 | 1 | diagonal 3D | 6.63e5 |
| 64 | 1 | diagonal 3D | 8.24e13 |
| 32 | 4 | diagonal 3D | 5.07e5 |
| 64 | 4 | diagonal 3D | 6.30e13 |
| 32 | 16 | diagonal 3D | 7.58e5 |
| 32 | 4 | non-diagonal symmetric 4D | 6.43e5 |

각 시험에서 signed adjoint identity, 실제 finite-polynomial error의 enclosure, 합산 roundoff를 추가한 총 finite-stored-polynomial error enclosure를 확인했다. Non-diagonal example은 정확한 dyadic orthogonal rotation으로 만들어 symmetric spectrum의 정당성도 확보했다.

**이 결과는 전체 exp/phi certificate 또는 speedup이 아니다.** Truncation tail, coefficients enclosure radius, source error, summation, normalization을 기존처럼 추가해야 한다. 새 beta 계산은 degree 64에서도 Python exact arithmetic에서 대략 초 단위가 될 수 있다. 이것은 native 비교 benchmark가 아니라 setup이 action보다 비쌀 수 있다는 경고다. Degree 4096까지의 scalability는 전혀 검사하지 않았다.

### 5.4 실제 적용할 때의 경계

새 envelope cache key에는 polynomial degree, chosen coefficient bits, transformed spectral interval, relevant transform/scaling semantics와 proof version을 포함한다. Current X가 그 verified domain에 속하는지의 확인은 재사용할 때에도 남는다. 다른 phi column의 coefficients를 섞거나 declared-only spectrum을 verified로 취급하지 않는다. SameVector case는 먼저 signed coefficient들을 결합할 수 있으나 cancellation condition과 coefficient error를 함께 전파해야 한다.

제한된 degree에서 이 recurrence term이 검증되고 기존 다른 error component와 합쳐진 뒤에만 새 `Certified` 경로를 추가한다. 실패하면 EstimateOnly 또는 기존 Chebyshev/fallback 상태를 유지한다. Big rational prototype을 production loop에 그대로 넣는 것은 권하지 않는다. Cached interval-Bernstein, mesh reuse, exact offline certificates와 directed-rounding online residual dot의 분업을 먼저 시험한다.

## 6. 이 스레드에서 가져올 것과 가져오지 않을 것

| 스레드 결과 | Vigilode 적용 | 금지할 확대 해석 |
|---|---|---|
| Strict ordinary W-method는 arbitrary bounded W에서 D>=p | 새로운 저깊이 tableau 탐색의 feasibility gate | variable gamma만으로 generic W5 depth2-3 달성 |
| Exact-J action과 근사 W 대체는 다름 | current JVP target 유지, basis/preconditioner만 재사용 | approximate W를 강직 정확도에 안전한 것으로 간주 |
| RVJ semilinear two-step 반례 | 별도 negative-control corpus와 internal stability test | 이것을 기존 RODAS5P의 실패라고 보고 |
| RVJ embedded blindness | physical LTE/graph reference와 proxy를 분리 | vigilode StageTargetBound를 ODE오차 증명이라고 부르기 |
| Darboux/cofactor chart | 별도 method identity의 optional model-specific chart interface | raw RODAS와 bit-identical라고 주장하거나 arbitrary data를 manifold로 projection |
| Hensel closure jet | 실제 algebraic closure client가 있을 때만 branch+regularity 계약으로 사용 | 이미 triangular한 RODAS stage에 불필요한 root loop 추가 |
| Fast geometric inverse | 구조적 preconditioner로 사용하고 full current J residual 확인 | omitted (DP)mu 항을 없애고 같은 operator라고 주장 |
| Rational multi-shift sharing | verified rational action의 특정 도메인에서 사용 | exact exponential phi 모듈을 rational phi로 몰래 교체 |

이 스레드의 세 map인 RJ5, RVJ5, chart-RVJ5와 저장소의 RODAS5P는 구별된 mathematical identity를 갖는다. 스레드에서 구한 최소-depth scalar gamma 약 0.27805도 실제 RODAS5P snapshot의 gamma 약 0.21194를 교체할 근거가 아니다.

### 6.1 Optional chart를 구현한다면 필요한 계약

정확한 cofactor 관계가 아니라

\[
L_F C=(-\kappa+a)C+r_C,\qquad L_FD=aD+r_D
\]

라면 w=C/D는

\[
\boxed{w'=-\kappa w+\frac{r_C-wr_D}{D}}
\]

를 만족한다. 따라서 작은 polynomial coefficient residual만 보고 chart error가 작다고 판정할 수 없다. |D|의 하한, |w|, inverse chart norm, physical component weights를 포함해야 한다. Moving frame에는 connection \(-S^{-1}L_FS\)가 남고, finite-epsilon invariant graph와 critical equilibrium graph도 구별한다.

Chart는 state뿐 아니라 event function, output reconstruction, dense output, mass transformation, initial condition, branch identity까지 바꾼다. x=0/D=0, ill-conditioned inverse, branch/infinity boundary에서는 명시적으로 invalid/fallback해야 한다. 비영인 transverse mode를 강제로 0으로 만드는 fast projection은 별도 모델 근사이지 solver 최적화가 아니다.

## 7. 구체적인 다음 개발 순서

기계 판독형 SSOT는 `NEXT_DEVELOPMENT_DAG.json`이다. 각 node에는 source dependency, 수정 예정 파일, acceptance test, 금지 claim, stop condition이 들어 있다.

### 첫 번째: 현 코드의 음성 회귀와 MF target 계약

먼저 제공한 native radius replay를 실제 Rust toolchain에서 실행해 R4 6회 실패와 7/8회 확장 closure를 별도 결과로 고정한다. 동시에 MF raw/native target allowance를 정확한 snapshot bytes로 만든다. Gamma roundoff leakage를 무시하지 말고 현재 R4 stage-target 정책과 동일한 방식으로 source/operator identity를 결합한다. Generic no-chart RVJ는 production에 넣지 않는다.

### 두 번째: matrix-free fast workspace

별도 실험 driver에서 M=I, current JVP, 기존 clock/output/controller를 유지하고 reusable vectors와 raw U RHS를 구현한다. `build_step_context`의 tableau clone, per-stage states/RHS/report allocation, gmix JVP를 줄이되 counters를 감추지 않는다. 기존 LGMRES/GCRO-DR의 operator identity/invalidation 규칙과 rejection transaction을 유지한다. Seven RHS JVP 제거는 contract로 검사하고 전체 cost는 실제 GMRES current-J calls와 global reductions까지 포함한다.

Native acceptance는 endpoint agreement 하나로 끝내지 않는다. Same physical error에서의 matched-mesh residual, coefficient allowance, dense output, represented time, rejection rollback, zero RHS, very small h, nonautonomous partial-t, current-J plus recycled preconditioner를 따로 검사한다. Direct factorization을 사용하는 fallback은 strict MF mode에서는 fail-closed여야 한다.

### 세 번째: homotopy radius와 certificate action

수치적으로 닫히는 radius를 찾는 것과 빠른 admission을 분리한다. Current residual H(0) preflight 또는 historical component radii로 proposal을 만들고 같은 enclosure로 검증한다. Full sum을 action-first로 바꿀 때는 bit identity gate 대신 새로운 independent upper-bound gate가 필요할 수 있다. 그 변경은 기존 FAIL row를 수정하지 말고 새 node에 사전등록한다. Old candidate rates, setup, pool, allocation, residual evaluation을 모두 비용에 포함한다.

### 네 번째: Laguerre recurrence budget

먼저 m<=64 또는128의 제한된 verified symmetric domain에서 adjoint-Bernstein term을 기존 ErrorComponents에 붙인다. Degree를 키우기 전에 coefficient/interval cache와 directed-op 비용부터 확인한다. Cold setup을 warm action 뒤에 숨기지 않는다. Total certificate가 닫히지 않으면 EstimateOnly를 유지한다. 이전 continuous scale 연구의 0/22 degree reduction 실패는 그대로 남긴다.

### 다섯 번째: chart/physical error 경로와 성능 승인

Chart-aware path는 model-specific experimental identity로 유지한다. Supplied thread counterexamples와 actual RODAS/exponential paths를 각각 독립 reference로 비교한다. Physical error와 embedded proxy를 분리하며 error budget의 inner solve, source jet, algebraic closure, chart reconstruction, method truncation 항들을 explicit하게 보존한다.

그 뒤에야 native matched-error campaign을 사전등록한다. 기존 CVODE/Hairer native adapters, 동일 output schedule와 uncertainty 조건을 재사용한다. Timing authority HOLD를 보고서만으로 해제하지 않는다. 작은 n의 한 적분에 stage-level Rayon/MPI를 강제하지 말고, 독립 trajectory/parameter ensemble 병렬화도 별도의 throughput 대상으로 비교한다.

## 8. 검증 산출물과 남은 실행 경계

이번 자체 실행은 Python 3.13.5, NumPy 2.3.5, SymPy 1.14.0, mpmath 1.3.0에서 수행했다. Connected Wolfram 15.0.1에서는 generic mass/stage residual identity, signed Laguerre adjoint identity, synthetic common-radius obstruction을 확인했다. Local Wolfram kernel 실행이라고 보고하지 않는다.

검증 묶음은 다음처럼 분리한다.

- Unit/security contracts: 13개. 계산의 작은 계약과 create-only same-branch receiver의 local bare-repository smoke test 포함.
- Homotopy actual-fixture-like enclosure: n=1,2,4,8,16의 5개 case. Exact native coefficient bits 사용, Rust directed arithmetic bit-parity 미실행.
- Path-action exact identity: 5개 width와 잘못된 premise 4개.
- Laguerre finite-polynomial bound: 7개 case. Whole exp/phi bounds 미완료.
- JVP-only raw-stage transform: 12개 case. Native Rust port·AD backend·adaptive integration 미실행.
- 이전 스레드 loop3: 별도 scratch copy에서 exact 40/40 및 numerical 89/89 재실행. 반례를 포함하므로 solver 성공률이라고 해석하지 않는다.

Source coefficient full JSON의 SHA-256와 Git blob ID는 intake에서 모두 검증했다. Native stage-target fixture는 첫 object의 명시적 bit selection이며 전체 원본을 byte 복제한 것으로 가장하지 않는다. `inputs/thread_loop3/proofs`도 원 archive의 script SHA와 연결되어 있다. 전체 repo checkout을 받지 못했으므로 모든 파일의 byte audit을 수행했다는 주장은 없다.

새 probe는 source-informed exploration이다. Existing remote review protocol을 뒤늦게 읽고 여기에 우리의 결과를 소급 “preregistered”라고 붙이지 않는다. 단위 테스트가 실제 통과했다는 사실과 처음부터 RED/GREEN cycle을 기록했다는 사실도 구분한다. 후자는 이 실행 전체에 대해 주장하지 않는다.

## 9. 게시와 복구

`publication/apply_same_branch.py`는 기본 dry-run이며 다음을 확인한다.

1. 승인된 기존 branch가 checkout되어 있고 local/index가 clean이다.
2. Origin이 정확히 승인된 GitHub repo이고 local/remote HEAD가 검토한 publication base다.
3. Manifest SHA-256, 경로, symlink, 중복, 대상 폴더 create-only 조건을 만족한다.
4. `--apply`에서 본 리뷰 폴더만 stage/commit하고, `--push`에서 같은 branch에 non-force push한다.
5. Push 후 remote ref와 commit이 일치할 때만 R1 identity receipt를 발급한다. Restore verification이 아니다.

로컬의 disposable bare origin에서 이러한 절차를 테스트했다. 그 test push는 사용자 GitHub push가 아니다. Runtime DNS 실패 log와 실제 원격 미게시 상태는 publication receipt에 따로 보존한다. 이후 원격이 바뀌면 receiver는 중단한다. 변경분 검토 없이 --force 또는 base override로 진행하지 않는다.

## 10. 근거 연결

Repo 근거 ID는 `SOURCE_REGISTRY.json`의 immutable commit/path와 연결된다. 주요 연결은 다음과 같다.

- R-fast-v2: latest fast-v2 preregistration/results, L-0033.
- R-fast, R-sequential: explicit-LU donor 및 JVP target의 현재 구현.
- R-outward, R-stage, R-hom-study, R-hom-generator: stage-target 의미, 반경 정책과 실제 R4 입력.
- R-poly: Laguerre/Chebyshev의 domain, recurrence 및 ErrorComponents.
- R-coeff, R-bits: 공식 coefficient snapshot과 native target bit selection.
- T-loop3: 스레드의 정확한 semilinear counterexample, estimator blindness, chart repair 및 재현 스크립트.
- W-DLMF: NIST DLMF 18.9, Laguerre 3항 recurrence.
- W-Bernstein: Rocca–Magron–Dang, arXiv:1610.07038. Bernstein 기반 roundoff enclosure 배경만 사용하며 이번 adjoint 결합의 성능이나 전체 인증을 대신 증명하지 않는다.

**최종 권고:** 현재 stable authority를 유지한 채, raw-stage MF workspace와 필요한 native residual allowance를 첫 구현 대상으로 잡는다. Homotopy는 radius-cap 원인부터 닫고 action-first/box cost를 검증한다. Laguerre는 output-aware recurrence certificate를 제한된 도메인에 붙인다. RVJ 계열과 Darboux chart는 negative controls와 optional research methods로 보존하고, 범용 production replacement로 승격하지 않는다.