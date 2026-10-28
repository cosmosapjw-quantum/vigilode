> 통합 완료 주석: 이 문서는 source/design 감사 시점의 기록이다. PP01/PP02는 이후 source 699a7adc6dba5fc45d4da01a5456f10f25dcf210의 새 native 실행으로 COMPLETE_RESEARCH_SCOPE가 됐다. 최신 상태와 수치는 NEXT_DEVELOPMENT_DAG.json 및 RESULTS.json을 따른다. 원 역사 verdict는 바꾸지 않았다.

# 다항식·공유 resolvent·homotopy 병렬화: 통합 확장 연구

작성일: 2026-10-04. 구현 기준: `826fa05cc1fdaf7b1a0cf7ac45116596b4ddff65`를 포함하는 현재 작업 트리. 아카이브 기준: `research/rvj_full_history_20261004`, 특히 Loop10/11. 연구 절차: GPT-6 Astra v4.0.0 하네스의 claim/source/실행/승격 분리. 이 명칭은 절차 바인딩이며 실행 모델의 신원 증명이 아니다.

**판정:** Laguerre를 처음 도입할 단계는 지났다. 기존의 제한된 인증을 실제 후보 라우터에 연결하고, 넓은 stiff 영역에서 인증이 너무 느슨한 원인을 좁혀 개선해야 한다. Loop11의 shared shift jet은 별도의 유망한 연산 공유 원리다. 그러나 현재 RODAS의 common-γ stage를 추가로 같은 shift에 모아 새 LU 절감을 얻는다는 주장은 성립하지 않는다. 실제 이득의 대상은 **서로 다른 가까운 shifts + 준비된 RHS + 작은 공통 RHS rank**다. 실제 Fourier coupling의 RHS가 이 구조를 갖는지 먼저 확인해야 한다.

이 문서는 게시된 캠페인을 재실행하지 않았다. 아래 수치는 해당 결과 파일/기록의 보고치다. 새로운 내용은 명시한 대수적 유도와 구체적 이식 계약이다. 새 wall-clock, 수렴차수, 범용 UA 또는 production speedup을 주장하지 않는다. 후보 생성자가 최종 방법 승격을 판정하지 않는다.

## 1. 게시 결과의 한 번 감사와 유지할 실패

| 기존 결과 | 읽은 근거와 이번 판정 | 후속 작업의 한계 |
|---|---|---|
| RNEXT04 / L-0047 | `research/rnext04_laguerre_admission_20261003/{PREREGISTRATION.md,RESULTS.json}` 및 `polynomial_action.rs`의 admission/component registry. 게시 gate PASS. 52 admitted 결과의 bound/error 7.4–1452, normalization 예외와 최초 실패 공개 | 검증된 symmetric nonpositive, degree ≤128, directed execution 안의 opt-in admission. 전역 `total_error`는 여전히 EstimateOnly. 일반 비정규 행렬의 인증으로 확장하지 않음 |
| RNEXT04의 stiff 한계 | ρ=400,h=0.1의 여섯 경우는 degree111–114, actual error 약6e−14–3e−13인데 bound9.5e6–2.1e8로 예산 거절 | 정확한 후보와 유용한 상계는 다른 성질. `exp(L'/2)`이 들어가는 tail/coefficient 항부터 개선. 회귀 majorant만 다시 개선해도 이 병목은 안 사라짐 |
| INT05 / L-0056 | `certify_exp_action`은 explicit current A, metric, numerical-range enclosure, interval polynomial evaluation을 결합. 작은 residual이나 Ritz value 자체를 인증하지 않음 | 함수는 exp-action 인증이며 일반 fused φ-combination 인증이 아님. 이를 서로 바꾸어 붙이면 target이 달라짐 |
| REV02 / L-0060 | `research/rev02_nonnormal_stepping_20261003`의 gate **FAIL** 유지. bounded18개에서 enclosing; stiff six개 중 relative usefulness gate는3개 실패 | 절대 bound≈9e−14–7e−13은 유용할 수 있지만 decayed output의 상대 기준은 실패. nonnormal random/Jordan의 매우 큰 bound도 유지. Gershgorin의 성장률 과대평가와 metric transport를 다룰 것 |
| RNEXT06 / L-0050 | 같은 q2 후보/target 비교. **FAIL** 유지: action-first finishing work68n 누락. 수정 post-hoc 비용에서 n≤8 abstain; n=16의 margin은 약0.043 solve/attempt | 수정표를 prospective PASS로 재명명하지 않음. serial depth7로 남고 실제 dispatch/더 싼 preconditioned baseline까지 감안한 속도 결론은 없음 |
| INT02 / L-0053 | `PreparedStructuredCertificate`가 witness/problem 한 번 구성, diagonal storage2n, certificate425n; 게시 **FAIL**은 dense comparison slope 예측1.8에 미달1.76 | 비트별 equivalent87후보와 구조적 개선은 보존하되 전체 gate PASS로 바꾸지 않음. n=16 certificate6800 vs26000을 새로운 net margin으로 곧바로 해석하지 않음 |
| INT06 | `docs/reviews/20261003_integrated_plan/Q2_ACTIVATION_DECISION.md`는 이미 게시된 **derived decision**. n16의 structured mean margin+0.63을 이전 기록에서 유도하고 default 비활성화 결정. `CRITICAL_REVIEW.md`C1은 directed/plain 단위 불일치를 지적 | 새 INT06 numerical run은 없음. directed-operation multiplier k=1,2,3,4에서 +0.63,+0.26,−0.11,−0.49라는 게시 민감도도 유지. 추가 measured activation만 미완료이며 기존 결정문 자체를 미구현이라 부르지 않음 |
| Loop11 shared shift jet | `code/shift_jet.py`, note§5, work-accounting. dissipativity 증명, exact reference, f64 probe를 분리한 구조 타당. 65targets→1factorization/18block solves의 조건 명시 | native directed implementation 없음. baseline은65개 개별 LU이며 Schur reuse/multi-shift Krylov보다 빠름은 미측정. recurrence depth18, output/reconstruction/residual은 무료 아님 |
| Loop11 FFT | `fft_bridge.py`가 실제 nonlinear coefficient convolution을 계산하고 parent `certificate`로 원 target을 다시 검증함. harmonic/degree 두 축 및 conjugation의 의미 일치 | 이미 완료된 Python FFT predictor를 다시 미구현 과제로 등록하지 않음. native Rust 이식, general closure, coupled shared-resolvent 결합은 별도 미완료 |

재검증을 여는 사건은 source/hash/API 변경, 새 operator domain, 새로운 RHS compression, 새로운 rounding backend, 구체적 반례다. 위 게시 캠페인은 그런 사건 없는 한 재실행하지 않는다. 과거 단일 runtime, 결과표, archive hash는 native 이식의 성능/정확성 증명이 아니다.

## 2. 'Krylov 대신 polynomial'의 정확한 의미

차수 m인 임의의 다항식에 대해 `p_m(A)v ∈ span{v,Av,…,A^m v}`다. Laguerre/Chebyshev/Leja도 이 **벡터공간** 밖으로 나가는 방식이 아니다. 실용적인 대안은 Arnoldi/Lanczos로 기저를 직교화하고 작은 projected matrix function을 계산하는 알고리즘 대신, 미리 정한 다항식 기저의 짧은 recurrence/Newton form을 쓰는 것이다. 이 구분을 해야 “Krylov가 아닌 방법”의 성과를 바르게 측정할 수 있다.

기대 이득은 Arnoldi의 inner products/global reductions, basis storage, reorthogonalization을 줄이는 것이다. degree개의 matvec dependency 자체가 자동으로 병렬화되지는 않는다. 서로 다른 RHS를 block로 계산하거나 동일 recurrence에서 여러 φ계수/target을 함께 누적하는 것이 가능한 병렬성이다. 동기화가 비싼 환경과 높은 degree가 필요한 환경의 결론은 다를 수 있다.

## 3. 후보별 domain·witness·비용·실제 API

| 후보 | 적합한 operator/문제 | 필요한 출력오차 근거 | 주요 비용 / 실패 조건 | 현재 이식 지점 |
|---|---|---|---|---|
| Chebyshev | 검증된 real symmetric nonpositive A, 유계 real interval | interval mapping, coefficient/tail, recurrence, sum, normalization의 total | O(m) matvec, 짧은 recurrence; 같은 벡터는 한 recurrence로5φ 재사용. enclosure 검증/차수비용 포함 | `core::polynomial_action::{SymmetricNonpositiveOperator::gershgorin, joint_phi_action, JointPhiInput, JointPhiReport::admit_total_error}` |
| Laguerre | 같은 symmetric nonpositive domain, x=−A/β≥0; β에 따른 tradeoff | signed output adjoint + coefficient/tail/sum/normalization. degree≤128의 기존 `admit_laguerre_total` | adjoint-envelope/cache/setup 포함. exp(L'/2) 상계가 크면 정확한 값이어도 거절. 이 regime을 숨기지 않음 | `joint_phi_action_laguerre_scales`, `laguerre_scale_for_degree`, `laguerre_adjoint.rs`; 새 backend가 아니라 기존 opt-in 경로 연결 |
| Newton–Leja | real/complex enclosure 위 exp 또는 φ; 일반 비정규는 spectrum-only 금지 | verified numerical range/contour remainder 또는 원래 target residual의 amplification + recurrence rounding | O(m) matvec, 직교화 없음. node/scale/degree 선택 및 divided-difference 안정성 비용. 현 native 구현 없음 | 새 `core/src/leja_action.rs` 후보 backend. `LinearOperator::apply_rows`, `OperatorApplicationWork`, 기존 `fused_phi_linear_combination`을 비교 seam으로 사용 |
| Scaled Taylor | 일반 dense A 및 augmented exp로 fused φ; matrix-free 확장 별도 | 현재 `taylor_phi_action`은 rounded augmented M의 exact-arithmetic tail만 제공, total rounding 미인증. INT05/REV02는 exp target에 directed enclosure 제공 | m×s products, dense augmentation 비용. norm64 cap, transform hA rounding, normalization/underflow를 빠뜨리면 안 됨 | `core/src/taylor_phi.rs::{taylor_phi_action,OperatorEpoch}`, `nonnormal_certificate::{certify_exp_action,certify_exp_action_stepped,certify_exp_action_auto}` |
| Shared resolvent jet | 현재 J가 H-dissipative, h≥0, γ0>0, 실수 shifts | ρ=|δ|/γ0<1 tail; current target의 directed residual과 inverse gain. RHS압축 오차·rounding 포함 | 중심 LU1회, (p+1)r RHS solves, targets별 evaluation/residual. 원형은 pr J-actions이나 normalized recurrence는 이를 제거. common-γ stage에는 추가 LU절감 없음 | 이번 작업 중 `core/src/shared_shift_jet.rs` native slice 개발 중. `DenseMatrix`, `LuFactorization::{new,solve_rows}` 활용. 실행 증거는 주 보고서 참조 |
| Rational interpolation / rational Krylov | polynomial degree가 큰 stiff domain; 실제 poles/resolvent witness가 확보됨 | pole exclusion, rational approximation remainder, 각 inexact solve residual amplification | 여러 pole/setup 비용; multi-shift/Schur reuse가 정당한 comparator. shared jet과 동일알고리즘이라고 부르지 않음 | 현재 후보 연구/DAG. 기존 native Arnoldi `exponential::{krylov_phi_action,fused_phi_action,fused_phi_linear_combination}`와 direct LU 재사용을 comparator로 고정 |

Hermite/Faber류는 수학적 대안이나 지금의 병목에 우선 구현할 이유는 부족하다. skew-Hermitian/oscillatory 영역에는 `iA`의 Hermitian enclosure를 활용하는 complex Chebyshev 또는 verified ellipse 위 Leja/Faber가 자연스럽지만, 현재 real symmetric nonpositive API를 형식만 바꿔 사용해서는 안 된다. 복소수 norm/rounding 및 물리 phase error 계약을 먼저 추가한다.

외부 원문 확인 범위는 abstract/metadata이며 새 성능 결과를 빌려오지 않았다. Al-Mohy–Higham2011은 Taylor action/φ augmentation 비교 기준을 지지한다. Caliari et al.2016은 Leja의 scale/node/degree를 함께 선택하는 필요성을 지지한다. Deka–Tokman–Einkemmer2022는 solver와 action backend의 결합에 따라 승자가 달라진다고 보고한다. Crouzeix–Palencia2017은 numerical-range 기반 상계의 일반 근거다. 모두 VigilODE 구현의 유효성이나 속도 자체를 증명하지 않는다. 출처 URL은 아래와 JSON에 보존한다.

## 4. Shared shift jet의 구체적 확장: 임의 RHS를 하나로 속이지 않기

아래는 이번 통합을 위한 **직접 유도**다. 새 계산은 실행하지 않았다. 고정 positive Hermitian H에 대해 `J*H+HJ≤0`, h≥0, realγ0>0으로 둔다. `P0=I−γ0hJ`, `R0=P0^-1`, `K=hR0J`이면 기존 Loop11의 `||R0||_H≤1`, `||K||_H≤1/γ0`를 사용한다. δ=γ−γ0, ρ=|δ|/γ0<1이다.

서로 다른 target RHS를 열로 모아 `B=[b1,…,bM]=Q C+E`로 나타내자. Q의 r개 열을 공유할 수 있어도 각 column coefficient `c_j`와 compression residual `e_j=b_j−Qc_j`는 반드시 유지한다. exact low rank인 경우만 E=0이다. 계산은

`V0=R0 Q`, `V_(k+1)=R0(hJ V_k)`, `xhat_j≈Σ_(k=0)^p δ_j^k V_k c_j`.

정확 연산의 shared-jet tail과 압축 오차는

`||R_(γj)b_j−Σ_(k=0)^p δ_j^k V_k c_j||_H`
`≤ [ρ_j^(p+1)/(1−ρ_j)] ||R0Q||_(2→H) ||c_j||_2 + ||e_j||_H`.

Q가 H-orthonormal이라고 실제 검증되면 `||R0Q||_(2→H)≤1`이다. QR/SVD가 수치상 반환했다고 직교성을 exact로 가정하면 안 된다. 그렇지 않으면 verified `||Q||_(2→H)`를 대신 쓴다. 평가/solve/J-action roundoff는 별도 bound를 더하거나, 다음 **원 target residual 하나로 전체 후보오차를 인증**한다.

`r_j=b_j−(I−γ_j h J)xhat_j`, `||x_j−xhat_j||_H≤||r_j||_H`.

이 마지막 inequality는 real γ_j>0와 같은 current dissipative J를 쓰므로 성립한다. directed current residual이면 compression, truncation, recurrence, inexact solve, evaluation 오차를 모두 포함한다. analytic tail은 degree 제안용으로 쓰고, full-target residual을 admission용으로 쓰는 것이 최초 native 이식의 단순한 선택이다. 두 상계가 각각 완전한 bound이면 더 작은 것을 쓸 수 있지만, 불완전한 component끼리 작은 값만 골라 합치면 안 된다.

물리 Euclidean/WRMS 출력으로 변환할 때 H-norm을 동일시하지 않는다. 예를 들어 `||e||_2≤||H^-1/2||_2 ||e||_H`; componentwise error scales가 고정된 WRMS라면 이 변환과 `1/√n`을 포함한 operator norm을 써야 한다. 현재 첫 구현은 H=I로 좁혀 이 변환을 생략할 수 있다. 복소 shift의 수학이 빠졌다는 뜻은 아니다. **Loop02 note§10/T2.9는 이미 Reγ>0이면 `||(I−γhJ)^−1||_H≤|γ|/Reγ`를 증명했다.** 새 과제는 그 복소 domain/gain을 native directed residual/출력 합산에 이식하는 것이다. real-positive의 gain1을 복소 shift에 그대로 사용하지 않는다.

### 이번 native slice의 normalized recurrence

현재 개발 중인 `shared_shift_jet.rs`는 `T=γ0K=R0−I`, `z=δ/γ0`, `W0=R0B`, `W_(k+1)=R0W_k−W_k`를 사용한다. 따라서 `Σ z^k W_k`는 기존 jet과 같은 exact polynomial이고 `||T||_H≤1`이다. J-action과 큰 inverse powersγ0^−k를 후보 생성에서 제거한다. 단, `R0W−W`가 작은 경우 cancellation의 상대 정확도 문제가 있으므로 안정성이 자동 보장되지는 않는다. current target의 directed residual gate가 실제 저장 후보오차를 판단한다.

첫 slice의 row-Gershgorin dissipativity witness는 **충분조건**이다. valid dissipative J도 row bound가 양수이면 unsupported가 될 수 있다. 이것은 해당 J가 비-dissipative라는 판정이 아니다. 인증 가능한 domain 확대는 별도 verified eigenupper/metric witness 작업이다. 이 문서는 코드가 작성 중인 것을 읽었고, 성공적 실행/최종 수락을 대신 주장하지 않는다.

### 비용 모델

M개의 각기 다른 shift와 단일 RHS의 naive baseline은 M factorization + M solves다. common shift baseline은 **1 factorization + M solves**다. Loop11의 `65 × rank2 B` 방송형 baseline은65 factorization+130scalar solves로, 임의 RHS65개의 baseline과 구별한다.

원형 shared-jet 비용은 `C_rank + C_factor + r(p+1) C_solve + rp C_J + C_eval + C_residual + C_memory + C_dispatch`다. normalized recurrence는 `rp C_J`를 `rp` vector subtraction으로 바꾼다. current-target residual의 J 비용은 여전히 남는다. 단순한 target별 평가·reconstruction은 O(Mnr(p+1)); jet storage는 nr(p+1), output은 최소nM이다. r≈M이면 압축이 실패한 것이 아니라 이 방식의 이득 조건이 사라진 것이다. `CommonShift`, `HighRank`, `WideCluster`, `ResidualReject`, `BudgetExhausted` 같은 typed abstention을 반환하고 그 비용을 ledger에 남긴다.

`(p+1)r < M`은 일부 solve 수에 대한 충분히 거친 screen일 뿐 전체 성능조건이 아니다. LU가 매우 비싸면 이 부등식 없이도 이득일 수 있고, setup이 싸면 이 부등식만으로도 손해일 수 있다. policy는 measured baseline 비용을 사용해야 한다.

±δ involution은 같은 Q의 even/odd jet을 공유할 수 있지만 서로 다른 RHS의 `c_+`, `c_-`를 같은 amplitude로 대체할 수 없다. `x_+=even(Q,c_+)+odd(Q,c_+)`, `x_-=even(Q,c_-)−odd(Q,c_-)`를 유지한다. 상태 자유도는 줄지 않고 공통 kernel만 공유된다.

## 5. Homotopy가 병렬화하는 것과 그대로 남기는 것

RODAS stage i의 RHS가 j<i의 stage에 의존한다면 서로 독립적인 RHS가 아니다. shared operator를 가져도 미래 RHS가 생기지 않는다. 현재 q1/q2 homotopy는 coupled stage predictor를 만들고 original-stage certificate로 받아들이는 방식이다. 이를 shared-shift family의 target-parallel evaluation과 구분한다.

가능한 축은 (a) 현재 준비된 RHS block columns, (b) 공유 jet 완성 후 여러 shifts/targets의 평가와 residual, (c) 정해진 homotopy iterate에서의 병렬 stage/RHS 평가, (d) Fourier full-product convolution이다. jet degree k의 recurrence와 homotopy correction round 간에는 여전히 dependency가 있다. speculation/fallback은 모두 cost에 남는다.

INT06 후속은 새로운 알고리즘 이름을 먼저 붙이기보다 `Q2Admission::PreparedStructuredCertificate`, `PreparedQ2Certificate::prepare`, `certify_stage_target_diagonal`를 실제 q2 attempt의 동일 state/h/budget에서 평가해야 한다. witness 한번 구성 비용, 준비 실패, 모든 q1/q2 시도, certificate finishing, 실패한 radius, fallback8stages, dispatch를 포함한다. INT02의425n과 RNEXT06의 옛 margin으로 유도한 이미 게시된 +0.63은 결정문의 estimate이며 speedup이 아니다. 특히 directed/plain 비용단위 환산의 k=3에서 margin이 음수라는 기존 감사를 수용한다. linear solve baseline도 같은 preconditioner/실행 방식으로 갱신해야 한다.

## 6. 실제 coupled Fourier client로 연결하는 이식 순서

Loop10/11의 부모 문제는 independent scalar mode가 아니다. reference `fourier_volterra.py::rhs`는

`a'= i q c conjugate(a) b`, `b'= i(q/2)c a²`,
`c=g+λε Re(e^(iωt) a)`이며 λ=1이 physical target이다. 현재 invariant leaf는 `|a|²+2|b|²=9/16`, `q=σ5/4`다. genuine coupling을 보존하되 general varying closure를 이미 풀었다고 주장하지 않는다.

1. **Reference adapter:** Loop11 `fft_rhs/build_fft`와 Loop10 `rhs/certificate/commit`를 정확한 source identity로 연결한 전용 client를 만든다. 입력은 state generation, model epoch, h, harmonic K, degree p, full phase witness, candidate bytes다. output certificate는 exact/noncyclic original λ=1 RHS를 쓴다. 기존 quadratic-only `Q2CertificateSource`가 이 cubic-plus-phase complex model을 인증한다고 가정하지 않는다. 별도 `FourierPathTarget`/`FourierPathCertificate` 타입을 사용한다.
2. **Native coefficient seam:** `(harmonic k, polynomial j)` sparse/dense coefficient packet와 `conjugate(k,j)→(-k,j)`를 이식한다. degree 축은 물리 time FFT가 아니다. 기존 full-product padding `N_k≥2(3K+s)+1`, `N_j≥3p+q+1`와 primitive의 초기상수 계약을 유지한다. 먼저 작은 direct noncyclic native convolution으로 Python exact fixtures와 비교한다. 이후 native FFT를 후보 생성기로 붙인다.
3. **Shared operator insertion:** frozen-current-J Newton/Picard correction 또는 multi-parameter target이 실제 여러 nearby positive-real shifts를 요구할 때만 shift-jet을 연결한다. 현 2-complex-mode reference가 자연스럽게65다른 shifts를 요구한다고 꾸미지 않는다. Jacobian이 conservative coupling 때문에 H-dissipative가 아닐 수 있으므로 witness를 검사한다. 실패하면 일반 inverse-gain witness 또는 기존 direct solve를 쓴다. FFT predictor와 shared jet은 같은 물리 target certificate를 거친다.
4. **Arbitrary RHS:** correction RHS들을 actual array로 수집하고 full/low rank를 측정한다. verified compression residual, per-mode independent amplitude, phase mismatch를 보존한다. 과도한 rank 또는 γ동일이면 공유 jet을 건너뛴다. 지원 가능 영역이 없는 경우 그것도 유효한 결과다.
5. **General closure/damping:** 이후 별도 노드에서 q가 경로를 따라 변하는 explicit model을 정의한다. q0=5/4 shortcut을 제거하고 closure tube/inverse/branch/defect를 whole path에서 검증한다. old parent certificate를 바꾸지 않고 새 model에 무단 적용하는 것은 금지한다.

native entrypoint 후보 경로는 새 `crates/rodas5p-integrators/src/fourier_path_candidate.rs`와 `fourier_path_certificate.rs`다. existing seam은 `stage_chart_candidate.rs`의 **candidate에 acceptance field 없음**이라는 설계 원칙 및 `chart_transport.rs`의 physical reconstruction 개념이다. 실제 코드 공유 가능성은 확인 후 판단하며, 이름이 비슷하다는 이유로 quadratic certificate를 재사용하지 않는다.

## 7. 구체적 다음 개발 DAG

기계 판독 원본은 동명 JSON의 `nodes`다. PP01/02는 이번 native slice에서 진행 중이며 주 보고서의 실행·검증 상태가 최종 권위다. 아래 순서는 범용 전면교체가 아니라 gate를 통과한 좁은 기능의 opt-in 이식 순서다.

| ID | 내용 / 선행 | 구체적 seam | 수락 / 중단 |
|---|---|---|---|
| PP01 | current-operator witness와 결과 provenance 계약 / 없음 | `operator.rs`, `directed.rs`, 현재개발`shared_shift_jet.rs` | current input으로 매번 재인증. future cache는 J/h/metric/γ/RHS identity 바뀌면 재사용 거절; nonfinite/shape/negative/unsupported 모두 typed reject |
| PP02 | real dissipative normalized shared shift jet / PP01 | `LuFactorization::new/solve_rows`, `shared_shift_jet` | current directed residual≤budget일 때만 admit; ρ≥1, cap초과, witness없음은 fallback. 이번native결과를 주보고서에서받아 상태갱신 |
| PP03 | arbitrary-RHS rank adapter / PP02 | 새`shared_shift_jet::RhsCompression` | rank1,rank2,fullrank,near-dependent,unequal±amplitude; E residual 전체계상. fullrank/commonshift에서 경제성 없으면 abstain |
| PP04 | prospective cluster/degree/rank selector / PP03 | 새`shared_shift_jet::SharedActionPolicy` + work.rs 보조ledger | factor/solve/J/eval/QR/residual/alloc/rejection 모두계상; naive LU 외 Schur재사용/가능한multi-shift baseline 포함. 미측정비용으로 PROMOTE 금지 |
| PP05 | 실제 coupled Fourier reference adapter / 없음 | Loop10`rhs/certificate/commit`와 Loop11`build_fft`의 source-bound fixture | 모드/amplitude/λ=1/full support/epoch/초기조건 보존. original certificate변조·phase omit·alias candidate는 reject |
| PP06 | native coefficient/FFT candidate / PP05 | 새`fourier_path_candidate.rs`, direct noncyclic comparator | direct/FFT candidate를 동일 target에 인증; conjugation·padding·primitive RED 보존. FFT roundtrip만 맞으면 통과하는 테스트는 금지 |
| PP07 | 실제 coupled client + shared action / PP04,PP06 | correction operator/RHS adapter | 실제 shifts/rank/dissipativity 충족시 사용. coupling 제거 또는 없는shift65개를 인위적성공근거로 삼으면 중단 |
| PP08 | Laguerre total budget용 opt-in router / 없음 | existing`joint_phi_action`/`admit_laguerre_total` | current gershgorin domain/cap/rounding/backend identity 보존. RNEXT04 전체재실행 없이 새router의boundary/negative cases만검증 |
| PP09 | Laguerre stiff bound 개선 / PP08 | coefficient/tail functions + `laguerre_adjoint` cache | exp(L'/2) 병목항을 단독분해, sharper finite-degree envelope 또는 certified scaling 검토. proof없는 “정확해보임” admission중단 |
| PP10 | Leja candidate와 independent witness / 없음 | 새`leja_action.rs`, `LinearOperator::apply_rows` | node/scale/degree/enclosure/roundoff계상. 비정규spectrum-only이면 EstimateOnly; original full-target validation전 admission금지 |
| PP11 | Taylor φ total-target gap / 없음 | `taylor_phi.rs`, `nonnormal_certificate.rs` | exp와augmentedφ target차이 및 hA transformrounding 포함. tail-only 인증/roundedM 원본A혼동 차단 |
| PP12 | nonnormal decay-aware bound / PP11 | verified H symmetric-part eigenupper / `certify_exp_action_stepped` | directed inertia/other verified eigenupper로 negative decay증명, failed REV02 사례는 별도새결과. 원FAIL수정금지 |
| PP13 | INT06 동일조건 net-cost / 없음 | `PreparedStructuredCertificate`, same step budget/linear baseline | all attempts+68n finishing+witness+dispatch 포함; no positive all-in margin면 해당regime abstain |
| PP14 | general Fourier closure/damping / PP07 | 새model, tube, physical reconstruction | q(t) branch/tube/inverse/phase/residual전부. invariantleaf결과를 general-UA로확장하면중단 |
| PP16 | existing complex-gain theorem의 native 이식 / PP02 | Loop02§10 + 새complex directed residual | Reγ>0, gain|γ|/Reγ, residue/source-jet/physical metric 변환계상. 이미증명된정리의 재발명 금지 |
| PP15 | bounded native comparative campaign / PP07,PP08,PP10,PP11,PP13 | source-bound counter/paired timing harness | same physical error·output·state; failedattempts/allsetup 포함; counter-only는 speedup결론 불가 |

PP10–12는 PP02의 prerequisite가 아니다. shared-resolvent 이식은 exp/φ polynomial 구현을 기다릴 필요가 없다. 반대로 polynomial 인증이 resolvent inverse witness를 대신하지도 않는다. 서로 독립적인 수학/코딩 줄기를 병행하고 실제 client 단계에서 같은 physical error 계약으로 비교한다.

## 8. 문헌 및 증거 범위

- Al-Mohy & Higham (2011), *Computing the Action of the Matrix Exponential, with an Application to Exponential Integrators*, DOI10.1137/100788860. 읽은 범위: 저자 저장소 abstract/metadata. https://eprints.maths.manchester.ac.uk/1591/
- Caliari, Kandolf, Ostermann & Rainer (2016), *The Leja Method Revisited: Backward Error Analysis for the Matrix Exponential*, DOI10.1137/15M1027620, arXiv1506.08665v2. 읽은 범위: 원문 abstract/metadata. https://arxiv.org/abs/1506.08665
- Deka, Tokman & Einkemmer (2022), *A comparison of Leja- and Krylov-based iterative schemes for Exponential Integrators*, arXiv2211.08948. 읽은 범위: 원문 abstract. https://arxiv.org/abs/2211.08948
- Crouzeix & Palencia (2017), *The Numerical Range is a (1+√2)-Spectral Set*, DOI10.1137/17M1116672. 읽은 범위: publisher abstract/metadata. https://epubs.siam.org/doi/10.1137/17M1116672
- SciSpace를 위 논문들의 검색·metadata discovery에 사용했으며 abstract의 알고리즘/비교범위만 인용했다. 주장은 primary source로 연결했다. Wolfram의 새 계산은 이 subtask에서 하지 않았다. Loop11의 게시 CAS 결과는 게시 증거로만 취급했다.

## 9. Closeout

완료: 기존 게시 evidence1회 code/claim audit, originalFAIL 보존, currentnative API mapping, arbitrary-RHS bound 직접유도, 실제 nonlinear Fourier client와 병렬성의 연결조건,16개 개발 task의 acceptance/kill설계. 미실행: 신규 numerical campaign/native Rust/성능측정. 이 문서 자체의 권한은 설계 제안이며 production 활성화나 최종 방법 승격이 아니다.
