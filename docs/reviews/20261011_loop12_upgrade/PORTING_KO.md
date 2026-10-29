# Loop12 통합: 실제 소스에 결박한 Rust 이식 설계

기준은 `cosmosapjw-quantum/vigilode`의 commit `2e944afc36a20fd20ce33438b2a1d4f4e0a6a5c4`, tree `d19fe946006130beaf139c0b2c22c0821af3e19f`다. 이 문서는 해당 소스를 읽어 작성한 **구현 설계**이며 native 구현·컴파일·시험 결과가 아니다. 첨부 Loop12의 독립 연구에서는 저장소를 읽지 않았지만, 이번 요청은 현재 소스와의 통합 연구를 승인한다. 기존 미실행 노드를 완료로 바꾸지 않고 새 분기와 필요한 의존성만 추가한다.

## 1. 현재 코드에서 확인한 사실

| 읽은 소스와 행 | 실제 계약 | 이번 통합의 결과 |
|---|---|---|
| `crates/rodas5p-integrators/src/problem.rs:20–31,127–144,207–250` | `OdeProblem`은 trait가 아니라 callback을 저장하는 struct이며, `new()`는 Jacobian과 JVP가 모두 없으면 거절한다(224–227행). | 순수 packet API는 별도 opt-in 생성 모델로 만든다. 가짜 JVP를 넣거나 기존 constructor 계약을 약화하지 않는다. |
| 같은 파일 40–60,253–301행 | 기존 구조 선언은 `Unstructured`, `Dense`, `Banded`뿐이다. | `LowRankFeedback` 같은 새 이름은 현재 API가 아니다. 일반 band 선언을 collective feedback 증명으로 해석하지 않는다. |
| 같은 파일 146–171,186–203행 | pointer identity와 epoch는 모델 불변성의 증명이 아니며, epoch는 호출자 약속이다. | packet certificate는 고정된 모델 bytes·모드 순서·초기 enclosure·time·history identity에 결박하고, opaque callback의 동일성을 hash만으로 인증하지 않는다. |
| `crates/rodas5p-integrators/src/routing.rs:241–277,294–350` | 현재 router는 dense/banded/matrix-free 경로를 선택한다. band 검증은 `(t0,y0)`에서 JVP 또는 dense Jacobian을 비교한다. | 이 점검은 whole-domain projectability나 저랭크 증명이 아니다. packet은 별도 명시적 진입점부터 시작한다. |
| `crates/rodas5p-core/src/polynomial_action.rs:477–483,558–572,1355–1378` | `joint_phi_action`은 정확히 대칭인 비양의 실수 연산자를 받는다. | skew-oscillatory carrier 및 일반 nilpotent packet을 이 계약에 넣을 수 없다. Laguerre 이름의 공통성만으로 certificate를 재사용하지 않는다. |
| 같은 파일 616–642,1434–1437행 | 현재 basis는 Chebyshev/Laguerre, cache는 연산자·step·degree·scale에 결박한다. 공개 scalar enclosure는 `phi_0..phi_4`, 실수 `[-600,0]`이다. | 임의 kernel order의 damped moment는 별도의 검증된 scalar API가 필요하다. 기존 5개 phi만으로 모든 order 또는 임의 큰 damping을 지원한다고 하지 않는다. |
| `crates/rodas5p-core/src/shared_shift_jet.rs:3–18,105–127,363–375` | `W_{k+1}=R_0W_k-W_k`가 이미 구현되어 있고 LU solve·현재 residual 비용을 별도로 센다. | Loop12 §11 항등식은 이 native 모듈에서 이미 실현된 기존 기능이다. 새 JVP 제거 개선으로 중복 집계하지 않는다. |
| `crates/rodas5p-integrators/src/transactional_q1_q2.rs:154–166,218–238` | 생성형 `QuadraticModel`이 RHS/JVP/증명식을 같이 제공한다. | 모델 한 곳에서 RHS와 witness를 생성하는 소유권 패턴은 재사용 가능하지만 quadratic 모델의 증명은 cubic collective 모델로 전이되지 않는다. |
| 같은 파일 1717–1745,1815–1839,1847–1916행 | 기존 transaction은 matrix-free context, 8-stage RODAS5P, 보호된 sequential fallback, 실패 비용 누적을 사용한다. | Loop12는 다른 적분맵이므로 이 함수의 q1/q2 accepted로 위장하지 않는다. transaction 원칙만 별도 packet controller에 적용한다. |
| `crates/rodas5p-integrators/src/chart_transport.rs:1–14,25–59` | 기존 certificate는 특정 `y=x²(w+1/kappa)` chart의 조건부 복원이다. | packet의 모든 mode 복원 및 전역 chaining 인증을 기존 chart 인증으로 대체할 수 없다. |
| `crates/rodas5p-core/src/work.rs:4–110,396–409` | JVP와 explicit Jacobian matvec를 구분하며 checked ledger subtraction이 있다. | packet 전용 scalar/kernel/출력/phase/실패 비용을 별도 구조로 추가하고 기존 계수에 숨기지 않는다. |

각 파일의 SHA-256과 anchor는 `SOURCE_BINDINGS.json`에 있다. 이 identity는 source readback 근거이며 native test 통과나 수학 증명의 대체물이 아니다.

## 2. 실제 모델 입장 판정

첨부의 알려진 diagonal frequency + 공통 cubic mean-feedback 모델은 **이론 및 독립 reference가 제공한 명시적 모델군**이다. 현재 repository의 실제 nonlinear 고객 모델이 같은 구조라는 증거는 찾지 못했다. `crates/**/*.rs`의 collective/moment/frequency-packet/mean-field 검색과 아래 공개 constructor의 RHS를 확인했으며, 이 제한된 source survey를 외부 모든 사용자 모델의 부재 증명으로 확대하지 않는다.

| 현재 모델 | 판정 및 이유 | 허용되는 사용 |
|---|---|---|
| `problems.rs:435–501`, `complex_dahlquist_problem` | common damping/frequency의 독립 선형 2×2 실수 블록이다. `g=0`, 공통 주파수의 퇴화한 정확 재구성 예로는 연결 가능하지만 nonlinear feedback client가 아니다. | damping·carrier·복소수 실수화 경계의 새 fixture. 직접 exponential baseline보다 빠르다는 주장은 불가. |
| `problems.rs:86–243`, `manufactured_vector_problem` | tridiagonal A에 각 좌표별 cubic defect가 붙는다(157–159행). 일반적으로 nonlinear channel 수가 n이며 공유 cubic mean이 아니다. | packet의 잘못된 입장을 거절하는 negative control. 제조된 exact trajectory의 낮은 apparent rank를 구조 증명으로 쓰지 않는다. |
| `problems.rs:700–752`, `semilinear_advection_diffusion_problem` | transport/diffusion coupling과 n개의 local cubic defect다. | 저랭크 feedback으로 자동 분류하지 않는다. Fourier diagonalization을 나중에 시도하면 변환비용·경계·nonlinear convolution을 새로 부담한다. |
| `problems.rs:571–627`, `stiff_van_der_pol_problem` | 2차원 local polynomial coupling이다. | 별도 finite-rank memory 표현 가능성 자체는 부정하지 않지만 첨부 scalar complex energy theorem의 admitted class가 아니다. n→r 점근 이득의 실증으로 쓰지 않는다. |
| `problems.rs:630–697`, `robertson_problem` | 3종 화학반응이며 실수 dissipative diagonal frequency + imaginary cubic mean 모델과 다르다. | 보존법칙만으로 photon/ensemble식 quotient를 추정하지 않는다. |
| `transactional_q1_q2.rs:163–238`, `QuadraticModel` | `Az+q*z²`의 생성형 모델이다. coordinatewise nonlinearity라 일반적으로 rank-r collective feedback을 보증하지 않는다. | 모델 생성/인증 소유권 패턴을 참고한다. |

따라서 현재 통합의 정확한 상태는 `ADMITTED_GENERATED_RESEARCH_CLASS`, `EXISTING_NONLINEAR_REAL_CLIENT=NOT_ESTABLISHED`다. 이것은 새 모델 클래스 구현을 막는 blocker가 아니다. 그 클래스의 bounded solver를 먼저 구현할 수 있고, 별도로 실제 사용자 모델의 whole-domain 구조 증명을 얻으면 운영 대상이 확장된다. 어떠한 일반 ODE도 `C=I`, `r=n`으로 쓰는 형식적 rank 표현은 압축의 근거가 아니다.

## 3. 제안하는 API와 파일 구조

아래의 타입·파일·함수는 **전부 제안**이며 현재 컴파일된 API로 인용하면 안 된다. 기존 `OdeProblem`, `WorkCounters`, `Interval`만 현재 타입이다. proposal의 첫 구현은 첨부에서 확인된 identity-mass, scalar complex cubic, 고정 diagonal carrier/frequency 클래스에 한정한다.

```rust
// PROPOSED DESIGN ONLY: private fields + checked constructors.
pub struct CollectiveCubicModel { /* ordered weights, detunings, kappa, chi, omega */ }
pub struct PacketInitialEnclosure { /* all complex centers and component radii */ }
pub struct PacketModelBinding { /* model/schema/epoch/ordering/time/history identity */ }
pub struct PacketWorkspace { /* reusable moments and coefficient scratch */ }
pub struct PacketWindowPlan { /* h, p, D, sweeps, precision, budgets, output mode */ }
pub struct PacketWindowCandidate { /* waveform, retained inputs, work */ }
pub struct PacketWindowCertificate { /* private current-bound authority; no Deserialize */ }
pub enum PacketAttempt { Certified(/*...*/), Rejected(/* reason, retained work */) }

pub fn prepare_collective_packet(/* model, initial, max p, workspace */) -> Result</*...*/>;
pub fn try_packet_window(/* prepared, plan, workspace */) -> Result<PacketAttempt>;
pub fn certify_packet_window(/* exact model inputs, candidate, inherited bounds */)
    -> Result<PacketWindowCertificate>;
pub fn reconstruct_packet_outputs(/* candidate, requested times, certificate */)
    -> Result</* stored outputs and certified radii */>;
```

제안 파일은 `crates/rodas5p-integrators/src/collective_model.rs`, `packet_window.rs`, `packet_trajectory.rs`, `packet_work.rs`, `crates/rodas5p-core/src/damped_moments.rs`다. 별도 native 구현 PR에서 `lib.rs` export를 명시적으로 추가한다. 초기에는 `routing.rs::route_problem`에 자동 선택 분기를 추가하지 않는다. 존재하는 JVP fallback이 필요한 caller에게는 같은 생성 모델에서 비교용 `OdeProblem` adapter를 별도 제공할 수 있다. 그 adapter를 호출한 비용은 fallback/benchmark 비용이며 derivative-free packet cost에서 지우면 안 된다.

생성자와 인증자는 다음을 검사한다.

1. finite parameters, `kappa>=0`, real chi, 명시된 carrier/detuning convention, strictly positive weights, 실제 weights 합이 정확히 1이라는 계약 또는 명시적 normalization을 한다. Binary64 weights를 단순히 합하여 `≈1`이라고 통과시키지 않는다. 합계 1인 exact stored-input 모델을 정의하거나 normalization 오차를 original-model defect에 포함한다.
2. 모든 독립 초기 amplitude와 초기 radius를 보존한다. 짝대칭 `mu_odd=0`에서도 `nu_odd`는 생략하지 않는다. mode order와 각 coefficient identity를 결박한다.
3. physical time `h`, `Omega`, `Delta`, `kappa` 단위를 명시한다. 내부 `tau∈[0,1]`에서는 `Delta*h`, `kappa*h`, nonlinear coefficient `chi*h`를 사용하고 복원 시 physical tolerance로 돌아온다.
4. `p,D,S`, 모든 allocation product와 cost accumulator에 checked arithmetic을 사용한다. NaN, infinity, 음의 radius, 누락된 mode, 중복 출력 time, incompatible mass matrix는 typed refusal이다.
5. energy/tube/Lipschitz 영역 전체를 enclosing한다. 중심 initial state만 energy≤1인 것은 불충분하다. 실패할 경우 h를 줄이거나 더 넓은 valid B,L,G를 재구성하고 모든 실패 작업을 남긴다.

## 4. Damping을 정확히 유지하는 kernel 엔진

N1의 의미는 `exp(-kappa*t)`를 높은 차수의 일반 Taylor 다항식으로 다시 근사하여 인위적인 fast layer를 만드는 것이 아니다. 제안된 `damped_moments`는

\[
I_{j,k}(h,\kappa)=\int_0^h e^{-\kappa(h-s)}(h-s)^j s^k\,ds
\]

와 그 enclosure를 제공해야 한다. `kappa=0`에서는 \(h^{j+k+1}j!k!/(j+k+1)!\)로 연속적으로 돌아간다. 큰 \(\kappa h\)에 대해 소수 차이의 catastrophic cancellation이나 overflow가 없어야 하며, 사용할 recurrence/series/asymptotic branch마다 rigorously enclosed remainder와 사용 영역이 필요하다. 부모 이론 보고서의 damping 확장 정리가 이 인터페이스의 수학적 계약이다. 기존 `scalar_phi_enclosure`의 제한을 넘는 order/range는 단순 호출 재사용으로 해결되지 않는다.

모멘트 cache에는 max order, kappa*h의 정확한 bits/enclosure, detuning scaling, 모델/epoch, precision과 rounding mode를 넣는다. h가 바뀌면 damping integral table을 잘못 재사용하지 않는다. mu는 같은 정적 mode specification에 대해 재사용할 수 있지만 nu는 window 초기상태와 함께 바뀐다. 고정 초기 state를 보존한 single window reuse와 multiwindow 초기값 변경을 구분한다.

## 5. Native arithmetic, output, transaction 계약

`directed.rs::Interval`을 실수부·허수부 rectangle의 primitive로 재사용하는 것이 초기 구현의 가장 작은 경로다. 복소 norm, energy 및 coefficient `l1_R` bound는 outward conversion을 포함해야 한다. 첨부 Fraction reference의 exact arithmetic과 native binary64는 동일한 모델 identity와 enclosure를 통해 비교하며 bitwise equality를 목표로 삼지 않는다. Small-tail에서 `0`으로 underflow했다고 exact zero로 간주하지 않는다.

별도의 `PacketWorkLedger`에는 최소한 model admission, mode reads, moment products, polynomial products, causal-convolution terms, damped-moment setup/reuse, rejected candidate work, certificate operations, phase reduction, reconstruction Horner terms, output scalars, allocations/peak storage, elapsed timing의 유무를 기록한다. 기존 WorkCounters의 JVP/explicit J matvec/factorization/solve 카운터를 함께 보고해 0 online derivative claim을 검증하되, 0은 나머지 계산이 무료라는 뜻이 아니다. 첨부가 세었던 `n(p+1)(3D+1)` coefficient terms는 native scalar instruction, CPU cycle, wall time으로 자동 환산하지 않는다.

Window controller는 accepted state를 읽어 immutable trial을 만든 뒤 waveform·원모델 tail·초기 불확실성·rounding·phase·복원오차를 인증하고 한 번에 commit한다. 거절 시 accepted time/state/radius/history는 그대로 유지하고 누적 work만 증가한다. model epoch가 trial 중 변하면 trial을 폐기한다. callback pointer가 같다는 이유로 이미 계산한 mu/nu를 승인하지 않는다. 반복 실패에 대한 h/min-step/resource cap은 explicit stop이다.

원래 C1/C2가 exact initial data single window에 대한 정리라면, multiwindow에는 부모 통합 정리의 inherited-error transfer를 사용해야 한다. 각 window에서 새 \(\nu_k\)의 radius를 \(\sum_j\pi_j|\xi_j|^k\rho_j\) 등 정리의 정확한 가중식으로 carry하고, 기존 \(\rho_j\)를 0으로 초기화하지 않는다. 사용한 frequency/carrier enclosure의 불확실성도 phase/parameter defect로 포함한다. h 및 출력 time의 저장오차와 carrier argument reduction 역시 원래 물리 시간과의 차이에 포함한다.

모든 mode를 매 window마다 출력하면 최소 O(n·windows)의 출력 비용이 남는다. endpoint만 필요한가, 여러 관측량만 필요한가, 모든 mode의 dense output이 필요한가를 API에서 명시한다. mean-only output과 full-state output의 cost table을 섞지 않는다. 여러 window 간 실제 solution dependence가 있으므로 이들을 독립 homotopy window라고 부르면 안 된다. 병렬화 후보는 한 window의 moment reduction, mode reconstruction, 독립 candidate 생성이며 sweep 순차 의존성은 보존한다.

## 6. 기존 10개 노드와 첨부 N1–N8의 결합

| 이전 노드 | 새 packet 경로에서의 역할 | 첨부 노드 |
|---|---|---|
| TP01 구조 생성 | 생성형 collective 모델 및 실제 client의 whole-domain admission으로 확장 | N2,N4 |
| TP02 inverse atlas | pure explicit Volterra packet에는 필수 아님. genuine implicit closure를 추가할 때 사용 | N5 |
| TP03 stage/adjoint 오염 | packet은 별도 방법이므로 기존 RODAS stage influence를 그대로 쓰지 않음. hybrid fallback만 해당 theorem 사용 | N3,N5 |
| TP04 전역 ledger | 정확히 같은 물리 error norm의 window transfer ledger와 결합. packet별 truncation/defect식 필요 | N3 |
| TP05 residual allocation | Krylov 반복 배분은 packet에서 강제하지 않음. 향후 p/D/S/precision 선택 최적화는 다른 계약 | N3,N6 |
| TP06 quotient polynomial | packet의 nilpotent shift 및 basis 변환이 affine/quotient 조건을 만족할 때 선택적 재사용. 기존 symmetric phi admission과 구분 | N1,N2,N6 |
| TP07 nilpotent resolvent | linear packet realization 또는 closure solve에 해당할 때만 사용. explicit packet의 필수 dependency가 아님 | N1,N5 |
| TP08 homotopy parallelism | independent windows 정리와 시간 구간 chaining을 구분; packet sweep depth 및 실제 scheduler span을 새로 셈 | N2,N3,N6 |
| TP09 invariant quotient | common-carrier exact mean/transverse는 모든 initial fibre를 보존하는 확장. affine invariant-manifold restriction과 다름 | N2,N4 |
| TP10 통합 acceptance | 새 candidate ID, preregistration, native 구현/trajectory/count/time 독립 판정 | N6,N7 |

N8 원기하 자료 복원은 별도 provenance 의무로 보존한다. 공개된 subset만 이용해 독립적으로 증명된 packet 경로를 N8 전체 archive 재실행에 종속시키지 않는다. 그 원본의 전체 복원을 주장하는 경우만 실제 missing author input이 필요하다.

`PORTING_NODES.json`은 12개 구체 PR 단위와 TP/N crosswalk, artifact·positive/negative test·claim gate를 제공한다. 모든 native 단계는 `PROPOSED_NOT_EXECUTED`다. 이번 연구 검토에서 N7의 **이론 검토 부분**을 닫더라도 아직 쓰지 않은 Rust 코드의 independent implementation review를 완료로 표시하면 안 된다.

## 7. 구현 우선순위와 종료 기준

첫 경로는 LP01 생성 모델/admission → LP02 finite-window native/certificate → LP03 정확 damping extension → LP04 inherited-error chaining → LP09 matched-cost acceptance다. LP02에 필요한 kernel은 kappa=0부터 동작하게 하여 N1/FFT/rank-r가 모두 끝날 때까지 bounded solver를 미루지 않는다. Rank-r LP05와 실제 client LP06는 별도 분기로 진행한다. 현재 packet 이득을 이미 hoist한 strongest baseline과 동일 맵에서 비교하고, 비용이 같으면 이득 없음으로 기록한다.

새 검증은 source/fixture/checker/gate를 사전 고정한 native 변경에 필요한 범위만 수행한다. 이미 게시된 Fraction/Python 결과 및 과거 native campaign을 매 PR마다 다시 실행하지 않는다. Exact fixture의 최소 선택 집합은 새 native arithmetic 경계 검증에 쓰고, 결과를 과거 전체 캠페인의 새 PASS로 바꾸지 않는다.

물리 정확도, 원모델 보존, 구현 roundoff, runtime failure, measured cost를 분리해서 판정한다. 구조가 맞지 않는 live client는 `UNSUPPORTED_MODEL`, sound하지만 loose certificate는 `CERTIFIED_BOUND_ABOVE_BUDGET`, 비용 불이득은 `NO_COST_ADVANTAGE`, 환경 실패는 `RUNTIME_BLOCKED`다. 어느 하나를 다른 원인의 실패로 덮지 않는다. 모든 성공은 admitted class와 source identity에 한정하며 production default는 별도 release 결정까지 유지한다.
