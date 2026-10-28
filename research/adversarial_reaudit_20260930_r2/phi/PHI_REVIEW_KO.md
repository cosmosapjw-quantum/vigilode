# φ-action 독립 재감사 및 제한된 수학·코딩 연구 결과

검토 대상은 commit `7708ef90554fc3986478d4602de6a01c7266b14f`, tree `23ffcacb8e4afcd72734e162c0e83536d565ba6c`이다. 주요 변경 `1299b62`의 time-normalized augmentation과 unfused happy breakdown 처리를 검토했다. 이전 감사의 PHI-P1/P2/P3를 동일한 문제 정의에서 재실행했으며, 개선을 인정한 뒤 별도의 scale-range 반례를 찾았다. 원본 production 코드·test·lockfile을 수정하지 않았다.

**결론은 이전 세 반례의 제한된 closure를 인정하되, φ API 전체의 scale robustness와 dense oracle의 일반 신뢰성 승격은 보류하는 것이다.** 새 정규화는 수학적으로 타당하고, 이전 반례·negative/zero scale·prefix continuation을 실제로 고친다. 그러나 scaled API의 중간 거듭제곱과 dense oracle의 진폭 conditioning에는 두 수치 문제가 남는다. 이 보고서는 residual estimate를 엄밀한 forward certificate로 간주하지 않는다.

## 1. 실행과 이전 finding의 closure

Rust 1.94.1, 원본 exact source, 제공된 offline vendor를 사용했다. `probe/src/main.rs`는 별도 client로 production API를 호출한다. scalar oracle은 정의

\[
\phi_k(z)=\sum_{j=0}^{\infty}\frac{z^j}{(j+k)!}
\]

의 120항이다. |z|≤0.1의 표에서는 절단오차가 binary64 roundoff보다 훨씬 작고, extreme-range 반례는 A=0의 정확식 φ₄(0)=1/24를 쓴다. production Padé를 oracle로 재사용하지 않았다. 모든 상대오차 판별은 rtol=1e-12, atol=0에서 이루어졌다.

| 대상 | 실제 관측 | 결과 |
|---|---:|---|
| PHI-P1 fused unscaled combination | b₀ 유/무 × 11개 h, 총 22행 | 최대 상대오차 4.44e-16, 모두 converged |
| PHI-P2 dense normalized combination | 같은 22개 조합 | 최대 상대오차 6.66e-16 |
| prefix begin(2) → finish | b₀ 유/무 × 10개 nonzero h | 최대 상대오차 4.44e-16, 모두 converged |
| PHI-P3 unfused A=−I₈, v=e₁, h=.1 | native 1행 | dimension=1, invariant breakdown, 값 .9516258196404042; 정확값과 차이 1.11e-16 |

h 목록은 0, −.1, −1e-12, .1, 1e-4, 1e-8, 1e-12, 1e-14, 1e-20, 1e-40, 1e-70이다. 원래 수치 실패를 고정된 regression window에서 닫았다는 판정이며 모든 연산자·진폭에 대한 정리가 아니다.

기존 targeted contract도 **17개 통과 / 0 실패 / 0 ignored**했다: `fused_phi_krylov_contracts` 4, `fused_phi_scaling_contracts` 4, `phi_action_certificate_contracts` 3, `phi_normalized_augmentation_contracts` 5, `prefix_phi_contracts` 1. 실제 native client와 기존 tests 모두 exit 0이다. 원시 probe의 104행은 관측 레코드 수이며 104개의 독립 test로 부풀리지 않는다. 로그는 `native_probe.jsonl`, `native_probe.stderr`, `existing_contracts.log`, exit 파일에 있다. `summarize.py`의 제한된 assertion도 실제 exit 0이다.

## 2. 수정의 수학적 평가

\(J_p\)를 nilpotent Jordan chain, \(P\)를 물리 성분 projection으로 두고

\[
M=\begin{pmatrix}A&[b_p,\ldots,b_1]\\0&J_p\end{pmatrix},\quad
q=(b_0,e_p)^T,\quad
D_h=\operatorname{diag}(I,h^{p-1},\ldots,h,1)
\]

라 하자. h≠0이면 block multiplication으로

\[
D_h^{-1}(hM)D_h=
\widehat M_h=
\begin{pmatrix}hA&[h^pb_p,\ldots,hb_1]\\0&J_p\end{pmatrix},
\qquad D_h^{-1}q=q,\quad PD_h=P.
\]

따라서 \(Pe^{hM}q=Pe^{\widehat M_h}q\)이다. 부호 제한이 없어 음의 h에도 성립한다. unscaled 조합의 \(w_k\)를 직접 upper-right block에 넣으면 h=0에서도 \(w_0+\sum w_k/k!\)로 연속 확장된다. 이번 native 결과가 이 세 범위를 확인했다. h가 시간 단위를 갖는 경우 A는 역시간 단위이며, 가중치 \(w_k=h^kb_k\)는 모두 출력과 같은 단위를 가져야 한다. 단위가 다른 b를 그대로 같은 norm으로 평가해서는 안 된다.

소스의 `exponential.rs:1438–1447`는 normalized operator를, `:1522–1524`는 substep을 \(e^{\widehat M_h/m}\)로 일관되게 바꿨다. prefix 경로도 `:1774–1777,1851–1858`에서 같은 operator와 unit time을 사용한다. `:1890–1895`는 augmented tail 대신 physical magnitude를 tolerance scale로 유지한다. restart의 정규화 인자 자체에서 새로운 불일치는 발견하지 않았다. 별도의 arbitrary-restart 오차 정리를 증명한 것은 아니며, summed residual estimates가 전역 forward bound라는 주장도 하지 않는다.

unfused `:1000–1008`의 checkpoint 순서 수정은 정확한 invariant breakdown이 minimum dimension 전에 일어나도 projection을 수행하게 한다. 이 변경은 PHI-P3의 원인을 직접 제거한다.

## 3. PHI-R1 — 표현 가능한 가중치가 중간 powi에서 소실

**P2 / NUMERICAL + IMPLEMENTATION / implementation-verified.** 위치:

- `crates/rodas5p-integrators/src/exponential.rs:1346–1364`의 `weighted_phi_vectors`;
- 같은 파일 `:1774–1777`의 prefix 공용 helper 사용;
- `crates/rodas5p-core/src/matrix_functions.rs:252–263`의 중복 weighting.

현재 구현은 먼저 `factor = h.powi(k)`를 계산하고 그 뒤 `factor*b_k`를 수행한다. 최종 곱이 유한하게 표현 가능해도 factor만 underflow/overflow할 수 있다.

\[
A=0,\quad h=10^{-100},\quad b_4=10^{300},\quad b_0=b_1=b_2=b_3=0
\]

이면 정확 결과는

\[
h^4\phi_4(0)b_4=10^{-100}/24
 =4.166666666666667\times10^{-102}.
\]

하지만 \(h^4\)가 먼저 0이 되어 dense API는 `Ok([0])`, fused API는 `value=[0], converged=true, invariant-subspace, error_estimate=0`을 반환했다. 음의 h=−1e-100에서도 같은 결과다. 반대 방향 h=1e100, b₄=1e−300에서는 정확 결과가 4.1666666666666665e98인데 factor가 ∞라 두 API 모두 NonFinite 오류로 거절했다.

이는 일반 비정규 Krylov residual의 약점과 다른 실패다. **Arnoldi에 들어가기 전에 문제 데이터가 바뀐다.** invariant-subspace는 이미 0으로 손상된 weighted problem의 projection에 대한 분류다. 이 분류를 전체 입력 변환 오차까지 보장하는 것으로 해석해서는 안 된다.

범위 제한도 중요하다. 반례는 극단적이지만 유한한 입력이다. 보통 h 창의 실제 integrator에서 같은 잘못된 endpoint를 관찰한 것은 아니다. 바뀐 weighting 라인에 원인이 있지만, 수정 이전 버전이 이 극단 입력에서 성공했다는 실행 비교는 하지 않았으므로 ‘기존 성공 동작의 회귀’로 단정하지 않는다. prefix는 동일 helper를 쓰는 소스 추적까지 확인했으며 이 극단 입력의 prefix 실행은 별도로 하지 않았다.

## 4. PHI-R2 — dense oracle의 입력 진폭 비불변성

**P2 / NUMERICAL / implementation-verified.** 위치: `matrix_functions.rs:310–328`의 weighted block 삽입과 `:85–130`의 power/norm scaling 선택.

문제는 매우 단순하다:

\[
A=-1,\quad h=.1,\quad w_0=0,\quad w_1=c,
\qquad u(c)=c\phi_1(-.1).
\]

정확 해는 c에 선형이다. 물리 spectrum과 step은 고정한 채 진폭만 바꾼 native 결과는 다음과 같다.

| c | dense oracle 상대오차 | direct fused combination |
|---:|---:|---|
| 1 | 1.11e-16 | roundoff 수준 |
| 1e10 | 7.77e-15 | roundoff 수준 |
| 1e20 | 6.80e-10 | roundoff 수준 |
| 1e40 | 4.3049753e-2 | roundoff 수준 |
| 1e60, 1e100, 1e200, 1e300 | **1.0: 모두 Ok(0)** | 모두 정확한 유한 값, 최대 상대오차 3.33e-16 |

예컨대 c=1e60의 정확값은 9.516258196404042e59다. small-step normalization은 h 때문에 upper-right block이 커지는 원인을 제거했지만, 큰 \(w_k\) 자체가 만드는 artificial conditioning까지 제거하지 않았다. 1e40에서는 power-based 경로에서도 상당한 정확도 손실이 생기고, 더 큰 진폭에서는 one-norm fallback과 과도한 squaring 경로에서 값이 0으로 소멸한다. 일반 Padé 이론을 반박하는 것이 아니라 현재 oracle 구현과 representation의 문제다.

direct fused 경로는 sigma balancing을 이미 갖고 있어 같은 진폭 sweep을 통과한다. 따라서 이를 현재 모든 fused integrator가 틀린다는 주장으로 확대하지 않는다. 다만 dense oracle을 독립 기준으로 사용하는 검사·gate에서 허위 반증이나 거짓 일치를 만들 수 있으므로 oracle의 허용 범위는 명시되어야 한다. PHI-P2의 원래 작은-h fixture가 수정되었다는 판정과도 모순되지 않는다.

원시 JSON의 `amplitude_dense`, `amplitude_candidate` 행은 공통 출력 helper 때문에 `h` 필드에 sweep parameter **c**를 기록했다. 그 두 ID의 실제 step은 항상 .1이다. `amplitude_fused`는 별도 `amplitude` 필드를 가진다. 이 legacy 필드 의미는 `PHI_STATUS.json`에도 명시해 분석에서 h와 c를 혼동하지 않게 했다. 원시 출력은 변경하지 않았다.

## 5. 이번에 실제 실행한 후속 후보

### 5.1 가중치의 순서 보존 곱셈

`scaled_weights`는 \(b\to hb\to h^2b\to\cdots\to h^kb\) 순으로 계산한다. |h|≤1이면 중간 절댓값이 감소하고 |h|≥1이면 증가한다. 따라서 처음과 끝이 정상 범위에 있고 정확 중간값이 그 사이에 있다면, 먼저 h^k를 만드는 방식의 불필요한 range failure를 피한다. 음의 h는 부호만 바꾸므로 같은 논리가 적용된다.

각 곱셈이 정상 rounding 영역이고 ku<1이면 표준 곱셈 모형을 직접 적용하여

\[
\widehat w=h^kb(1+\theta_k),\qquad
|\theta_k|\le\gamma_k=\frac{ku}{1-ku}
\]

로 roundoff를 분리할 수 있다. underflow/subnormal 영역에는 이 상대오차 식을 그대로 적용할 수 없고 절대오차 항이 필요하다. 여기서는 그 전 범위 인증을 구현하지 않았다. k가 작은 현행 φ₁–φ₄ 경로에는 단순 순차 곱셈이 적합한 bounded 후보이며, 임의로 큰 k API에는 mantissa/exponent 분해 방식과 비용을 비교할 여지가 있다.

### 5.2 공통 진폭 정규화 dense oracle

\[
\mathcal F_A(h;w_0,\ldots,w_p)
 =e^{hA}w_0+\sum_{k=1}^p\phi_k(hA)w_k
\]

는 모든 w에 공동으로 선형이므로, 유한한 s≠0에 대해

\[
\mathcal F_A(h;w)=s\,\mathcal F_A(h;w/s).
\]

`balanced_dense`는 최대 component magnitude 근방의 2의 거듭제곱 s로 모든 w를 나누어 기존 dense routine을 호출한 뒤 다시 곱한다. 변환이 normal range 안에 있으면 2의 거듭제곱 scaling은 binary floating point에서 불필요한 rounding을 만들지 않는다. 기존 Padé를 바꾸지 않고 artificial amplitude conditioning을 제거하는 동치 표현이다.

두 후보를 함께 사용한 scale-range 3개 및 amplitude 10개, **총 13개 fixture에서 최대 상대오차 7.77e-16**을 얻었다. 같은 원래 tolerance를 유지했다. 구현은 standalone client 안에만 있고 production 변경이 아니다.

이 후보도 일반 해법으로 승인할 수 없다. 서로 다른 component의 진폭 범위가 극단적으로 넓으면 common normalization이 작은 component를 underflow시킬 수 있다. 예컨대 큰 항의 상쇄 후 작은 항이 지배하는 문제에는 absolute condition sum과 output 규모의 차이를 추적해야 한다. 이번 sweep은 공통 진폭 변경을 검증했으며 임의 mixed dynamic range와 cancellation의 안정성은 미평가다. 일반 nonnormal matrix에서의 Padé forward-error 인증 역시 미해결이다.

## 6. 구체적 다음 개발 단계

1. **공용 weighted transformation을 하나로 만든다.** core와 integrators의 powi 곱셈 중복을 제거하고, 순차 곱셈 또는 exponent-aware multiplication을 사용한다. input finite 여부, intermediate underflow, true output overflow를 별도로 구분한다. acceptance: 이 보고서 PHI-R1 세 입력이 finite correct result를 반환하고, negative/zero h 및 기존 normalized/prefix contracts가 유지되어야 한다. 함수 이름이나 status field 변경만으로 closure하지 않는다.
2. **Dense oracle에 동치 balancing과 진단을 추가한다.** direct fused의 sigma 정책과 수학적 관계를 문서화하되 roundoff 의존성은 숨기지 않는다. acceptance: c=1e−200…1e300의 동일 scalar action이 relative1e−12 이내, finite-output zero collapse 없음. 그다음 mixed-amplitude/cancellation fixture를 별도로 추가하고, 정규화 중 비영 component가 0이 되면 관측 가능한 diagnostic 또는 안전한 다른 경로가 필요하다.
3. **현재 bounded closure를 integration 소비 경로에 연결한다.** `require_fused` (`exponential.rs:2430–2438`)는 여전히 converged만 소비한다. residual-estimate 허용 정책과 bound-required 정책을 분리하고, 입력 변환/roundoff 실패를 exact-projection status로 덮지 않는다. 일반 강인성 확보 이전에는 finite ordinary-scale 실험에서 성능을 평가하되 이를 rigorous certificate로 게시하지 않는다.
4. **상계 연구는 physical output을 목표로 진행한다.** normalized augmentation이 physical A의 dissipativity를 그대로 상속하지 않는다는 이전 조건을 유지한다. residual variation-of-constants, 물리 semigroup 상계, Jordan tail의 forcing 전달을 함께 다루어 physical endpoint error budget으로 연결해야 한다. Σ substep estimate를 그대로 certified global budget으로 승격하지 않는다. 이 단계는 수학 증명과 directed-rounding/roundoff 구현을 필요로 하며 이번 작은 scale repair와 혼합하지 않는다.

실행 가능한 다음 노드와 acceptance는 `PHI_STATUS.json`을 통합 DAG에 연결할 수 있다. 본 후보 생성자의 판정은 제한된 `derived + numerically checked + implementation-verified`까지다. 최종 연구 후보 승격은 root가 마련한 별도 decision reviewer의 검토 대상이며, 이 문서 작성자가 스스로 승인하지 않는다.

## 재현

작업 루트에서 준비된 Rust/vendor 환경을 사용한다. probe Cargo.toml은 이 보고서가 작성된 `reaudit/phi/probe`에서 `../../../vigilode`의 exact checkout을 참조한다. 저장소 내 보고서 폴더로 옮겨 게시할 때에는 해당 상대 경로를 실제 checkout 위치로 조정해야 한다. production 코드 수정은 필요 없다.

```bash
source runtime_r2/env.sh
CARGO_BUILD_JOBS=1 CARGO_TARGET_DIR=runtime_r2/target-phi \
  cargo run --manifest-path reaudit/phi/probe/Cargo.toml --offline
```

기존 검사는 `existing_contracts.log`에 compiler/run provenance와 개별 test 이름이 보존되어 있다. 원본 commit/tree, probe·lock·로그 hash, 실행 exit, 보류 범위를 `PHI_STATUS.json`에 함께 기록했다.
