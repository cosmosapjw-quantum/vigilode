# RVJ offline 연구 이력 1회 적대적 감사

검토자: `/root/archive_audit` (실제 분리된 source reviewer). 기준일 2026-10-04. 사용자가 선택한 GPT-6 연구/코딩 v4 방법론의 core를 읽었으나, 실제 호스트 모델 식별을 주장하지 않는다.

## 판정

Loop 08–11의 **조건부 수학 구조는 Rust 이식의 기반으로 수용할 수 있다**. 이번 읽기에서 stated assumptions 안의 핵심 resolvent/FFT 식을 반박하는 오류를 발견하지 않았다. 과거 520 exact inequalities, 33 tests, 16 paths 등은 **역사적 실행 기록**이다. 이번에 다시 실행하거나 현재 PASS로 상속하지 않았다. Native method promotion, performance, arbitrary nonlinear ODE coverage는 계속 HOLD다.

첫 이식은 `shared current shifted-resolvent jet`의 제한된 dense real API를 권한다. FFT predictor는 이미 연구 reference에 존재한다. FFT를 처음 만드는 일을 새 연구 진전으로 표시하면 안 된다. 두 연구 축의 진짜 다음 결합은 **독립 RHS를 보존하는 shared action → coupled Fourier client → 동일 원방정식 certificate**다. 현재 Loop11의 두 연구 코드는 이 결합을 아직 수행하지 않았다.

## 읽은 범위와 재실행 경계

- Top-level HANDOFF, LATEST_ORIGINAL_HANDOFF.
- Stages 08–11 HANDOFF, Stage08/09 RESEARCH_NOTE의 핵심 정리/실패, Stage09 CANDIDATE_AUDIT, Stage10 CLAIM_LEDGER.
- Stage11 RESEARCH_NOTE, CLAIM_LEDGER, RESEARCH_CONTRACT, FINAL_VERIFICATION, NEXT_DEVELOPMENT_DAG, FAILURE_REGISTRY.
- Stage11 `code/shift_jet.py`, `code/fft_bridge.py`, `tests/test_fusion.py`, `tests/test_fft.py` 및 실제 사용된 parent `inputs/loop10/code/fourier_volterra.py`.
- Loop01–07의 모든 계산을 재감사한 것이 아니라, Loop09의 통합 source inventory와 counterexample ledger 및 필요한 handoff를 읽어 inherited failure ceilings를 계승했다.
- 기존 numerical suite, algebra suite, oracle campaign, timing을 **실행하지 않았다**. 아래 scalar 예는 입력 계약의 필요성을 보이는 직접 대수 계산이다.
- Archive identity/integrity 검사는 owner 소관이며 본 검토가 byte verification을 대신하지 않는다.

## 우선 이식 경계

### A-PORT-01 — dissipativity가 검증되지 않은 bound helper를 certificate로 노출하면 안 된다 (P1, 이식 전제 누락 위험)

`code/shift_jet.py:44–60`은 h, γ0, δ 범위를 확인하지만 `J*H+HJ≤0`을 확인하지 않고 `series_bound`라는 값을 반환한다. 모듈 문서와 역사 fixture는 Euclidean dissipativity를 별도로 확인한다고 정직하게 제한한다. 그러므로 **게시된 fixture의 실패라는 뜻은 아니다**. 그대로 public Rust certificate로 옮기면 가정이 사라진다.

직접 반례: scalar J=1, h=1, γ0=1/2, δ=1/10, b=1, p=0. 중심값 U0=2, 실제 target 값 5/2, 오차 1/2. helper의 tail은 ρ/(1−ρ)=1/4, target residual은 1/5다. 두 값 모두 실제 오차보다 작다. 이는 dissipativity를 위반한 입력이며, 바로 그 이유로 checked constructor가 필요하다.

권고: `CurrentDissipativeOperator`가 matrix/metric/dimension/operator epoch를 immutable하게 소유하고 Euclidean dissipativity의 충분조건을 검증한다. 최소 구현은 실제 사용 범위를 제한한 symmetric-part diagonal dominance 또는 구조적 `S−Sᵀ−D` constructor일 수 있다. 일반 H를 지원한다고 표시하려면 H-norm inverse proof와 변환/rounding을 함께 제공해야 한다. Eigenvalue sign만으로 대체하지 않는다.

### A-PORT-02 — f64 residual diagnostic과 outward error certificate를 분리 (P1, 이식 전제 누락 위험)

`float_shared:74–90`은 LU/Horner 계산과 work counters만 반환한다. 실제 residual의 rounding bound, solve/action enclosure, accumulated recurrence error는 제공하지 않는다. Loop11 note가 이를 명시하므로 과거 결과는 올바른 범위의 numerical probe다.

권고: 이식 API는 `(untrusted candidate, work record)`와 별도 `certify_current_target()`를 구분한다. `||r||`를 계산했다는 이유로 certified upper로 만들지 않는다. 직렬 coefficient 계산이 부정확해도 최종 target residual enclosure가 유효하면 수용 가능하다. residual gate를 통과하지 못하면 degree/cluster 분할 또는 direct fallback으로 돌아가고 tolerance를 넓히지 않는다. Empty/mismatched dimensions, nonfinite parameters/shifts, resource overflows를 명시적으로 reject한다.

### A-PORT-03 — FFT certificate는 trusted constructor 경계를 전제로 한다 (P2, native API 설계)

Parent `fourier_volterra.py:176–198`은 전달된 `phase`와 `phase_error`를 신뢰하고 `d_phase`에 사용한다. Binding에는 state/model/h/path가 있고 phase enclosure 객체는 없다. `build():205–223`, `build_fft():68–85`의 정상 경로는 이를 직접 생성하므로 이번 source audit에서 역사 기록이 부정확하다고 판정하지 않았다. 그러나 arbitrary caller가 phase error를 축소해 넣을 수 있는 Rust public constructor로 그대로 노출해서는 안 된다.

권고: phase enclosure를 `(ω,t,h,precision,representation,coefficients,error)`에 결속하고 certificate constructor 내부에서 계산하거나 checked typed witness를 받는다. 초기 invariant leaf의 provenance도 state generation 숫자만으로 대체하지 않는다. trusted reference의 hash consistency와 hostile-input capability security를 구별한다.

### A-PORT-04 — 비용 절감과 병렬성 승격은 아직 미완료 (P2, claim ceiling)

65 independent LU 대비 1 factorization / 18 block solves / 36 scalar RHS solves는 기록된 유효한 **work count**다. 하지만 18 단계의 recurrence dependency, target reconstruction, 65 residual action, common RHS span의 비용은 남는다. Common-γ RODAS는 원래 factorization을 공유하므로 이 비교의 수혜 대상이 아닐 수 있다. Schur reuse와 최적 multi-shift baseline을 포함하지 않은 65-LU 비교를 production speedup으로 승격하지 않는다.

권고: prospective gate에 m(target 수), p, rank r, n, cluster width, residual cost, storage를 넣는다. 예시 구조는 `Cjet=F+(p+1)S_r+pM_r+mE_(n,r,p)+mR_(n,r)`이다. Direct baseline도 동일 residual 비용을 포함하며, 이미 가능한 factorization/Schur reuse를 적용해야 한다. 이 cost model은 wall-clock 결과가 아니다. Jet coefficient recurrence와 target evaluation의 병렬 구간을 따로 계상한다.

### A-PORT-05 — full FFT support와 formal polynomial axis를 보존 (P2, regression 방지)

`fft_bridge.py:16–51`의 full product padding `Nk≥2(3K+S)+1`, `Nj≥3P+Q+1`은 보수적이고 올바르다. 두 번째 FFT 축은 physical time이 아니라 polynomial coefficient index다. Conjugation은 `(k,j)→(−k,j)`이며 복소 grid 값 conjugation으로 바꾸면 j도 뒤집힐 수 있다. 높은 polynomial degree는 Volterra primitive 후 낮은 coefficient에 기여하므로 임의 조기 절단이 안전하지 않다.

권고: Rust candidate는 sparse exact reference와 full product를 먼저 비교하고 이후 pruning을 명시한다. Final target residual은 원래 noncyclic convolution과 λ=1을 재계산한다. FFT roundtrip, conserved action, corrector increment만으로 accept하지 않는다. Reference import는 namespace로 고정한다. 과거 동일 이름 `run_studies` import 충돌이 이미 기록되어 있으므로 경로 기반 global import를 복제하지 않는다.

## 수용한 조건부 결과

1. `A_s=C[ξ]/(ξ³−s²ξ)`는 모든 s에서 rank 3이다. s=0에서 support 하나라는 말은 dimension 하나라는 말이 아니다. Evaluator의 determinant 2s³와 coefficient inverse conditioning은 서술과 일치한다. ± involution은 μ=0 또는 family parameter μ까지 반전하는 경우에만 정당하다.
2. Dissipative current J, h≥0, γ0>0 아래 `R0=(I−γ0hJ)⁻¹`, `K=hR0J`에 대해 `||R0||H≤1`, `||K||H≤1/γ0`. Neumann expansion과 `ρ=|δ|/γ0<1` tail은 타당하다. Exact residual `b−(I−(γ0+δ)hJ)Up=δ^(p+1)hJ K^p R0b`도 telescoping과 일치한다.
3. Stable confluent exponential의 `c2=e^a b²/2·sinhc(bs/2)²`는 removable singularity를 정확히 해소한다. f64 transcendental rounding certificate는 별도다.
4. Cauchy–FFT coefficient alias bound는 holomorphic disk와 finite sampling 가정하에 올바르다. Conjugate-dependent vector field에 직접 적용할 수 없으며 small contour의 sample-noise amplification은 별도 항이다.
5. FFT-generated finite floats를 exact binary rationals로 고정하고 parent original-target certificate로 검사하는 구조는 predictor arithmetic의 신뢰를 요구하지 않는다. 원 certificate의 model/domain/phase assumptions와 trusted construction이 모두 유지되어야 한다.

## 반드시 보존할 과거 반례/한계

- Strict bounded-W same-step depth<5로 일반 W5를 얻겠다는 주장과 variable γ만으로 장벽을 없애는 주장은 되살리지 않는다.
- Generic RVJ/RJ uniform stiff order, approximate-W stability, embedded blindness 반례는 새 coordinate/map으로 철회되지 않는다.
- Five-node oscillatory forcing alias와 history extrapolation gains 31/769는 보존한다.
- Neutral mode에는 stationary damping box δ/ν를 쓸 수 없다. Physical phase/frequency uncertainty는 amplitude error와 따로 계상한다.
- Homotopy direct 6 대 continuation 18 RHS 사례와 full-horizon timeouts를 비용 개선으로 요약하지 않는다.
- Constant q=5/4는 특정 conservative invariant leaf의 결과다. Damping/general varying closure로 이식하면 q를 다시 증명하거나 enclosure가 필요하다.
- Geometry authentic missing input과 full-suite blocker는 그대로 유지한다. Local algebra의 정확성은 일반 coupled Fourier dynamics가 scalar 하나로 압축된다는 근거가 아니다.

## 구체적 Rust 이식 순서와 완료 기준

1. **Data contract**: checked current operator, positive real shifts, exact RHS identity/rank, budget/resource limits, immutable work/candidate result.
2. **Shared action primitive**: one factorization, coefficient block recurrence, Horner/parity evaluation. Verify zero δ / h=0 / signs / wide-cluster rejection / independent RHS columns / stale operator.
3. **Directed target residual**: outward dot/matrix action plus H/Euclidean inverse gain; verify signed cancellation, subnormal/overflow handling, shifted operator mismatch. Keep historical exact fixtures as oracle data pointers; only new native port tests are needed.
4. **Prospective selector**: reject if predicted reuse is absent; split wide clusters; common-γ case delegates to ordinary shared factorization. Count output construction and certificate cost. Do not call this a measured speedup.
5. **Coupled Fourier client**: first specify which actual action family is needed. Parent Loop10 primitive is already scalar analytic moment integration and does not automatically require a matrix resolvent. A synthetic extra resolvent inserted just to demonstrate reuse would change the method. Use a real semilinear coupled block or actual existing resolvent client and keep all modal amplitudes/common RHS coefficients.
6. **Matched comparison**: same physical target/budget, direct reused-factorization/Schur, polynomial/exponential action where appropriate, multi-shift candidate. Only actual cost advantage across a preregistered regime justifies selector promotion.

Laguerre/Chebyshev/Leja는 original-target action 계약 아래의 경쟁 backend로 유지한다. 이번 archive는 Laguerre가 구현되었다거나 여러 모드의 인과 의존성이 사라졌다는 새 근거를 제공하지 않는다. Existing published polynomial lane의 theorem을 다시 전부 계산할 필요는 없고, current operator contract와 새 client mapping이 바뀌는 곳만 검증하면 된다.

최종 권고: **scoped primitive port 진행**, **generic integrator/performance promotion HOLD**. 수학 문서의 제한을 지키면 연구를 제품 구조로 옮길 준비는 되어 있다. 다음의 핵심은 이미 있는 수학을 다시 증명하는 일이 아니라 현재 source에 assumptions와 failure semantics를 빠짐없이 담는 것이다.
