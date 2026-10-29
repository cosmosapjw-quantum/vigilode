# 빠른 진동: 대수적 carrier와 시간 의존 chart의 재도입

기준 소스는 이번 작업의 `SOURCE_INVENTORY.json`과 native diff에 결속된다.
이 문서는 원 연구의 재사용, 새 유도, native pilot, 후속 통합을 구분한다.
수치는 무차원 benchmark 변수이며 물리 단위 일반화에서는 γ, β, ω는 시간의 역수다.

## 1. 과거 연구가 남아 있는 위치

| 원 아이디어 | 실제 보존 위치 | 현재 코드와 연결 |
|---|---|---|
| Homotopy–involution, 합쳐지는 근과 local algebra | `research/rvj_integration_20261004/inputs/loop11_RESEARCH_NOTE_KO.md` §§3–6 | 근의 support를 합쳐도 jet와 독립 amplitude를 버리지 않는다. ±sector는 공통 coefficient를 공유한다. |
| Fourier–Volterra nonlinear coupling | `research/pp05_fourier_client_20261004/PREREGISTRATION.md`, `crates/rodas5p-integrators/src/fourier_path_certificate.rs` | 원방정식의 noncyclic convolution과 phase 오차를 다시 계산하는 native certificate가 있다. |
| FFT predictor | `research/pp06_fourier_fft_candidate_20261004/PREREGISTRATION.md`, `fourier_path_candidate.rs` | harmonic와 formal-degree 두 축의 full-product padding이 구현되어 있다. |
| Scalar causal moment packet | `research/promotion_loop12_20261011/input/RESEARCH_NOTE_KO.md` §§7–8 | 동일 projected waveform 맵에서 가중합을 먼저 하여 kernel contraction의 n배 중복을 없애는 정리다. 일반 독립 상태를 scalar 하나로 압축하는 정리가 아니다. |
| Constant damped rotation + nonlinear residual | `crates/vigilode/src/lawson.rs`, `crates/rodas5p-integrators/src/quotient_flow.rs` | 선언된 constant block을 쓰는 derivative-free Lawson–DOPRI가 있다. 현재 시간 의존 chirp chart와 자동 선택은 별도다. |

원본 보존을 공표한 PR은 [#78](https://github.com/cosmosapjw-quantum/vigilode/pull/78)이다.
이번 PR 검색에서 Fourier/involution/Loop11 이름의 유일한 원 PR은 확인되지 않았다.
따라서 정확한 native 파일·원 archive와 보존 PR을 인용하며 추정 PR 번호를 붙이지 않는다.
원 archive의 기록된 SHA256은 `a7d66500012f2c982470afcbd96b489c2dc7da4391f2e0f0f8aa7eaefd326fe6`이다.

PP05/PP06의 역사적 FAIL은 그때의 모든 과학 계산이 틀렸다는 뜻이 아니다.
PP05는 exact certificate 대비 1.01 비율 제한을 1.0112로 넘었고,
PP05/06의 약한 alias control은 실제로 정확한 후보를 만들었다.
원 기록에서 모든 수락 경계는 실제 오차를 포함했다. 이 결과는 **과거 실행 근거**로 재사용한다.
이번에는 큰 harmonic가 낮은 harmonic에 실제로 접히는 새 control을 추가했다.

## 2. 공통 carrier와 involution의 정확한 연결

실수 대수 A=R[ξ]/(ξ²+1)에서 ξ를 회전 생성자

    J = [[0,-1],[1,0]],   J²=-I

로 표현한다. 임의의 analytic 함수는 이 표현에서 두 실수 coefficient로 환원된다.
특히 `exp(θJ)=cos θ I+sin θ J`다. ξ↦−ξ는 involution이고
두 sector를 보존한 채 `exp(±θJ)`의 cos/sin을 공유한다.
`paired_carrier_rotation`은 독립된 두 amplitude를 입력받아 **한 번의 sin_cos**로
두 회전을 만든다. 이는 scalar 초월함수 호출 수를 2에서 1로 줄이는 직접적인
연산 수 감소이며 전체 ODE wall time 2배 개선을 뜻하지 않는다.

시간 의존 생성자가 A(t)=−γI−ω(t)J이면 모든 시각의 생성자가 commute한다.
따라서 시간순서 연산자 없이

    U(t,s)=exp[-γ(t−s)] exp[-J ∫_s^t ω(u)du]

가 정확하다. ω(t)가 커져도 coefficient 계산은 일정한 수의 scalar 연산으로 남는다.
일반 비가환 회전축에는 이 식을 그대로 적용하지 않는다. 그때에는 commutator를
포함한 Magnus/commutator-free 구성이 별도 대상이다.

**정리 1 (조건부 주파수 독립 적분).** 복소 변수 z가

    z'=−γz−iω(t)z+N(t,z),
    N(t,e^{iθ}z)=e^{iθ}N(t,z)

를 만족한다고 하자. φ'=ω, w=e^{iφ}z이면

    w'=−γw+N(t,w).

증명은 곱미분과 equivariance 대입으로 끝난다. 같은 초기 w, 같은 mesh와
controller, 같은 tolerance 아래 정확 산술의 transformed stage·embedded error·
step 선택은 ω와 무관하다. N의 필요한 미분들이 해의 근방에서 유계이면,
기존 p차 RK의 상수 C에 대하여 `||w_num−w||≤C h^p`이고 C는 ω에 의존하지 않는다.
이는 RK 수렴 정리를 carrier가 제거된 방정식에 적용한 결과다.
새로운 범용 stiff-order 정리라고 주장하지 않는다.

phase 근사 δφ가 있을 때, 회전이 등거리이므로

    ||z_num−z||₂ ≤ ||w_num−w||₂ + ||w||₂ |δφ|

가 성립한다 (`2|sin(δφ/2)|≤|δφ|`). rounding과 phase oracle 정확도는 별도 항이다.
Native pilot의 binary64 `exp/expm1/sin_cos`는 directed phase enclosure가 아니다.

**Fourier 비선형 항의 판별식.** `N(z,z̄)=Σ c_pq z^p z̄^q`이면 회전 후 항마다

    e^{iφ} N(e^{-iφ}w,e^{iφ}w̄)
       = Σ c_pq e^{i(1−p+q)φ} w^p w̄^q

다. p−q=1인 항은 carrier를 제거해도 느리다. 나머지 항은 남은 고속 phase를
명시적으로 보유한다. 따라서 “원 RHS가 빠르게 진동한다”만으로 Lawson을 고르면
안 된다. 예를 들어 상수 physical forcing도 rotating 좌표에서는 진동할 수 있다.
이 식은 algebraic charge 분해를 selector의 사전 정보로 쓰는 구체적 경로다.
많은 모드에서는 harmonic charge의 덧셈이 convolution으로 이어진다.

## 3. 실제 benchmark 15의 chart: 속도 변수 오차를 보존해야 한다

게시된 `external_models.py`와 `CATALOG.json`의 식은

    x'=v,
    v'=−(2γ−β)v−[ω(t)²+γ²−γβ]x,
    ω(t)=ω0 exp(βt),
    (ω0,β,γ,T)=(1,1.2,0.8,8), (x0,v0)=(1,−γ).

`q=x`, `p=(v+γx)/ω(t)`로 두면 직접 곱미분으로

    q'=−γq+ωp,
    p'=(v'+γv)/ω−βp=−ωq−γp.

여기서 `−βp`가 chart 시간미분이다. 고정 affine lift 정리만 가져오면 이 항이 빠진다.
`z=q+ip`, `w=e^{iφ}z`, `φ=ω0 expm1(βt)/β`에서 `w'=−γw`다.
β=0에서는 φ=ω0t라는 removable limit를 사용한다. 구현은 `βΔt`가 underflow해도
`ωΔt`가 사라지지 않도록 작은 인자에서 `ωΔt·exprel(βΔt)`로 평가한다.
큰 양의 인자에서는 끝점 주파수로 적분식을 다시 써서 중간 `exp(βΔt)` overflow를 피한다.

물리 residual `(r_x,r_v)`가 더해지면 rotating 방정식은 정확히

    w'=−γw+R(φ)[r_x,(r_v+γr_x)/ω].

큰 `ω²x`를 full RHS에서 다시 빼서 residual을 얻지 않고 caller가 추가 항을
명시한다. 이 설계는 cancellation을 줄이고 원방정식과 split의 계약을 분명히 한다.
새 native `ExponentialChirpChart`는 이 mapping을 구현한다.

**정리 2 (physical error transport).** 같은 t와 정확한 phase에서 `||δw||₂≤η`이면

    |δx|≤η,   |δv|≤(γ+ω(t))η,
    ||δy||∞≤max(1,γ+ω(t))η.

증명: 회전이 Euclidean norm을 보존하므로 각 δq,δp의 절대값이 η 이하이다.
`δv=ωδp−γδq`에 삼각부등식을 적용한다. 좀 더 날카로운 velocity gain은
`sqrt(γ²+ω²)`이나, pilot은 단순하고 보수적인 위 경계를 사용한다.

두 성분의 scalar absolute tolerance a, rtol=0에서 WRMS≤1이면 `||δw||₂≤sqrt(2)a`다.
따라서 H=max_[t0,T] max(1,γ+ω(t))에 대해 `a=physical_atol/(sqrt(2)H)`가 충분하다.
이 보장은 **embedded local estimate의 좌표 변환**에 관한 것이며 actual global
error가 local tolerance 이하라는 주장이 아니다. 실제 global endpoint error는 별도로 계산한다.
ω(T)≈14764.8이므로 chart 좌표의 tolerance를 그대로 재사용하면 velocity 오차를 크게 놓친다.

## 4. 구현과 pilot의 범위

- `crates/vigilode/src/oscillatory_chart.rs`: checked time interval, finite input,
  stable β→0 phase, 양방향 chart, residual mapping, physical norm majorant, involution pair.
- `crates/vigilode/tests/oscillatory_chart_contracts.rs`: chart chain rule,
  arbitrary amplitude, nonlinear equivariance, non-equivariant forcing, malformed input.
- `crates/vigilode/examples/oscillatory_chart_pilot.rs`: 기존 native DOPRI를
  physical RHS와 rotating residual 양쪽에 실제 실행한다. 추가 Jacobian/JVP/Newton 호출은 없다.
- `verify_algebra.py`: full noncyclic product와 padded transform, harmful alias,
  involution, polynomial charge, exact binary parameter의 80-digit Decimal phase를 검산한다.

두 pilot 가족은 실제 catalog chirp와 `−ε|z|²z` radial residual을 추가한 nonlinear
chirp다. 후자는 amplitude

    r(t)=exp(−γt)/sqrt[1+(ε/γ)(1−exp(−2γt))]

라는 별도의 해석 해가 있다. 같은 요청 physical atol=a에 대하여 full 경로는
physical WRMS를 쓰므로 embedded estimate의 infinity bound는 sqrt(2)a다.
chart 경로는 위 majorant로 수송하여 infinity bound a라는 더 엄격한 충분조건을 쓴다.
따라서 두 경로의 local infinity budget이나 실제 달성 오차가 같다는 비교가 아니다.
공통 endpoint cap 1e-4의 통과 여부와 실제 달성 오차를 함께 보고한다.

최종 phase 수선 후 native 측정은 각 설정의 warmup 1회를 제외한 5회 median이다.
요청 atol=1e-8에서 실제 catalog chirp는 full 78.17 ms, chart 0.2894 ms였고,
RHS 호출 수는 1,197,457 대 1,603이었다. Nonlinear chirp는 70.86 ms 대 0.2815 ms,
RHS 1,161,373 대 1,687이었다. Chart endpoint 오차는 independent Decimal reference에서
약 1.3e-9이고 full 경로는 약 7.5e-5였다. 이 측정은 공유 실행 host의 좁은 선언 model
진단이며 사용자 local CPU의 결과나 범용 method 순위가 아니다.
`PILOT_RESULTS.json`, `INDEPENDENT_ENDPOINT_CHECK.json`에 실제 반복값과 좌표를 보존한다.

## 5. 현재 switcher에 연결할 선택 규칙

1. 선언된 model identity와 epoch, chart의 invertibility, 시간 구간을 먼저 확인한다.
   Unknown black-box RHS를 몇 점 검사한 결과는 structural proof 대신 쓰지 않는다.
2. `constant rotation`, `commuting chirp`, `Fourier coupled residual`, `unknown`을 분리한다.
   p−q=1 또는 harmonic charge 보존이 컴파일 단계에서 확인되면 residual의 fast phase 제거를 예상할 수 있다.
3. 같은 accepted state에서 한 번의 제한된 paired pilot으로 physical error·work를 비교한다.
   predictor 재사용과 switching overhead를 포함하고, 실패 시 기존 method로 돌아간다.
4. error-budget 증가, model epoch 변화, phase oracle 불확실성 증가, residual bandwidth 증가가
   있으면 chart의 유효성과 경제성을 재평가한다. 성능 hysteresis로 수학 조건을 무시하지 않는다.
5. 시간 의존 physical WRMS를 stage acceptance에 직접 주입하는 후속 API가 필요하다.
   현재 전체 horizon의 H는 안전한 local majorant지만 후반 최대 주파수로 초반을 과해상할 수 있다.
   step tube의 H_n과 accumulated physical defect를 결합하면 이를 줄일 수 있다.

FFT는 늘 이기는 선택이 아니다. sparse noncyclic 곱과 padded FFT의 실제 coefficient 수,
padding, certificate 비용을 비교하여 crossover에서만 선택한다. 높은 polynomial degree를
조기에 자르면 primitive 이후 낮은 coefficient에 기여하는 항을 잃을 수 있으므로 full support를
계산한 뒤 정당화된 remainder를 계상한다. 기존 PP05/06 코드를 재사용할 위치가 바로 여기다.

## 문헌 연결

Hochbruck, Leibold, Ostermann, *On the convergence of Lawson methods for semilinear
stiff problems*, Numerische Mathematik 145 (2020), 553–580,
[doi:10.1007/s00211-020-01120-4](https://doi.org/10.1007/s00211-020-01120-4).
§2의 flow 변환과 정리 4.7–4.8의 regularity 조건이 관련된다.
변환 후 비선형항의 regularity와 경계 조건이 중요하므로 일반 PDE에서의 자동 uniform-order
주장으로 확장하지 않는다. 이번 새 정리 1은 exact equivariance 때문에 transformed equation이
주파수 자체를 포함하지 않는 더 좁고 직접적인 경우다.
