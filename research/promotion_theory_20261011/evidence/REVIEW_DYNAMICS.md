# 독립 결정 검토: homotopy·차원 축소·전역 정확도·잔차 배분

작성일: 2026-10-11. 독립 reviewer: `/root/review_dynamics`.
대상 source: `8ce9bda0d72d308facd615ac42ec137560fd68c6`.
역할: 후보 생성·검증 설계에 참여하지 않은 분리된 decision reviewer. 후보와 production source를 직접 수정하지 않았다. 정리의 수학적 적용 범위와 실제 source binding을 함께 검토했다. 과거 scientific campaign은 재실행하지 않았다.

**최종 판정의 범위: 아래 양의 정리들은 명시된 가정을 갖춘 이론 정리 및 구현 계약으로 PROMOTE한다. 기존 represented K-form에 대한 무조건 no-fold/8-round 종료 주장은 승인하지 않는다. production, 실제 trajectory의 전역오차, 실측 속도 승격은 각각 HOLD다.**

## 검토한 고정 증거

- `HOMOTOPY.md` 전체 H1–H5 및 대응 `HOMOTOPY.json`.
- `THEORY_GLOBAL.md` 전체 G1–G4 및 §6.3 복잡도 family, `GLOBAL_CLAIMS.json`.
- 신규 유리수 진단의 코드와 결과를 읽었다: `homotopy_exact_check.py`, `HOMOTOPY_EXACT_CHECK.json`, `check_global_exact.py`, `GLOBAL_EXACT_CHECKS.json`, repository `research/promotion_theory_20261011/check_exact.py`와 `EXACT_RESULTS.json`. reviewer가 재실행하지 않았고 유한 예제를 일반 증명으로 간주하지 않았다.
- 실제 source: `block.rs`의 `stage_mix`, `diagonal_apply`, `coupling_apply`, `rhs_base`, `nonlinear_remainder_snapshot`, `target_jacobian_matrix`; `homotopy.rs::evaluate_partial_path`; `coefficients.rs`의 represented K 계수 생성; `rodas5p_matrix_free_fast.rs`의 strict-lower U 계수 검증, stage assembly 및 output.
- 기존 공개 R3의 `homotopy/coefficient_structure.json`, `homotopy/HOMOTOPY_RESEARCH_KO.md`, `decision/INDEPENDENT_DECISION_KO.md`를 재사용했다. 기존 계수 leakage 실험은 다시 실행하지 않았다.
- GPT-6 Astra research v4.0.0의 독립 결정 gate를 적용했다. 모델 성능 비교나 formal proof assistant 검증은 수행하지 않았다.

확정 문서의 SHA256과 기계 판독 판정은 `REVIEW_DYNAMICS.json`에 기록한다. identity hash는 검토 문서를 고정하는 용도다.

## 항목별 결정

| 항목 | 결정 | 승인하는 정확한 범위 |
|---|---|---|
| H1 | PROMOTE | 실제로 strict-causal이고 동일한 가역 선형 W에 결합된 target의 유일한 C^k graph와 no-fold 정리 |
| H2a | PROMOTE | 같은 callback domain/tube에서 검증한 strict-lower B와 closure에 따른 유한 path-sum root enclosure |
| H2b | PROMOTE | fixed-old-iterate Jacobi의 최대 s회 exact 종료 및 inexact-round convolution; 원 target binding 필요 |
| H3a–b | PROMOTE | strict-causal target에서의 독립 window와 R>1 holomorphic ball 계약을 만족하는 jet predictor tail |
| H3c | PROMOTE | 같은 target·출력 예산과 모든 overhead, 실제 comparator 하계가 주어진 counted-cost model의 엄밀한 span 개선 |
| H4 | PROMOTE | 정확한 affine invariant 모델과 동시 J/M/f_t binding 아래 동일 discrete target의 reduced solve 및 명시 baseline 대비 work 감소 |
| H5 | PROMOTE | actual noncausal leakage를 보존한 B+E majorant, ||SE||≤q<1, radius closure 아래 원 target의 root 인증과 positive expansion tail |
| G1 / TG1 | PROMOTE | full-space 또는 증명된 invariant error subspace의 inverse gain과 nonlinear stage tube를 갖춘 전체 stage·output 오차 전달 |
| G2 / TG2 | PROMOTE | 실제 계수의 낮은 차수 defect와 6차 remainder 또는 rigorous defect integral에 의한 local truncation 상계 |
| G3 / TG3 | PROMOTE | 모든 accepted step의 truncation·stage·operator·RHS·output·time 오차, flow tube, metric transport를 포함한 finite-trajectory 전역 부등식 |
| G4 / TG4 | PROMOTE | 검증된 contraction/floor와 고정 영향도 아래의 연속 최적 배분, 정수 ceiling 상계 및 총비용 절감 충분조건 |
| §6.3 / TG5 | PROMOTE | 같은 scheduled backend와 공통 threshold comparator에 대한 정확한 매개변수 work family |
| 현재 represented K target의 unconditional no-fold/finite-round 주장 | HOLD | strict-lower source identity가 자동 성립하지 않으므로 추가 적용 증거 없이 정리 결론을 옮길 수 없음 |
| production·실제 global trajectory·wall-time | HOLD | 새 계약의 native witness·전체 호출 경로·동일 정확도 전체 비용 실행 증거가 각각 필요 |

PROMOTE는 조건을 지운 보편 명제나 실제 배포 승인이 아니다. 이 표의 가정이 수학적으로 닫힌 regime에서는 결론 자체가 보장되며, 무조건적 empirical 성능 향상을 뜻하지 않는다.

## 실제 검토에서 발견한 오류와 수정

1. **G1 gain의 authority 혼동.** 초안은 full induced inverse norm을 요구하면서 관측 residual 방향의 더 작은 gain을 함께 허용했다. 그 gain은 nonlinear coupling으로 생긴 다른 방향의 오차에 적용할 수 없다. 수정본은 모든 defect·coupling 차이의 합을 포함하는 증명된 invariant subspace 또는 full-space bound를 요구한다. 이 수정은 필수이며 확인했다.
2. **G2 truncation 상계 방향.** 초안 defect 식과 JSON의 `tau <= RHS`만으로는 G3에 투입하는 tau의 authority가 분명하지 않았다. 수정본은 `actual local error <= tau`, `tau := outward evaluated upper RHS`로 정했다. 독립적인 Simpson 값은 여전히 상계가 아니다.
3. **K-form strict-lower binding 누락.** 과거 R3는 native represented alpha와 L의 diagonal/upper leakage 및 8제곱 nonzero를 이미 증명했다. 현재 source도 일반 LU inverse로 Gamma를 만들고 모든 matrix entry를 stage mixing에 사용하므로 symbolic strict-lower 구조가 자동 보존된다고 할 수 없다. 과거 계수의 구체 bit pattern을 최신 binary에 재측정했다고 주장하지 않는다. 현재 construction에서 exact structural contract를 별도로 요구한다는 결론이다. strict-lower projection은 target 변경이며 동일 root를 선언할 근거가 아니다. 이 반대 증거를 숨기지 않고 causal reference/실제 target 경계를 정리의 조건에 포함해야 한다.
4. **JVP와 출력 예산.** closure가 frozen되었다는 이유만으로 finite-difference JVP가 정확한 선형 W가 되지 않는다. exact/reference W와 candidate operator mismatch를 분리해야 한다. H2의 stage epsilon은 `|b|^T e + output roundoff`를 거쳐야 output budget이 된다. 수정본에 이 경계가 반영됐다.

## Homotopy와 기하 정리의 상세 판단

H1의 증명은 귀납과 lower-triangular Jacobian으로 충분하다. 모든 재귀 상태가 callback domain에 있어야 존재성이 성립한다. globally polynomial callback이면 root도 다항식이지만 binary64 range, 작은 계수, 낮은 polynomial degree까지 보장하지 않는다. 이 정리는 일반 implicit closure의 branch/fold 문제를 없애지 않는다. 실제 current K coefficients의 exact 구조가 확인되지 않은 곳에서는 branch-label 생략을 허용하지 않는다.

H2a에서 candidate 주변의 product tube와 `d+B r<=r`를 함께 사용하면 이전 exact stage가 domain 안에 있다는 사실을 순서대로 닫는다. 따라서 norm(B)<1 없이도 존재·유일성과 유한 합 상계가 성립한다. H2b의 error power 및 roundoff convolution도 맞다. 다만 서로 다른 lambda, W, norm 또는 tube에서 얻은 B를 섞으면 해당 식이 성립하지 않는다. stage별 다른 norm을 쓰면 출력 metric까지의 transfer를 추가하거나 문서대로 동일 metric을 사용한다. 작은 spectral radius만으로 작은 bound를 보장하지 않는다.

H3b의 holomorphic ball 조건은 uniform contraction과 locally uniform holomorphic limit을 제공한다. Cauchy coefficient estimate에 geometric tail을 합하면 endpoint tail `r R^(-p)/(R-1)`이다. 계수 order 안의 stage solves는 병렬이어도 coefficient orders와 convolution 계산은 sequential 비용이다. 새 predictor가 이전 corrected window에 의존하지 않는다는 조건도 정확하다. 실제 endpoint 하나만 필요할 때 불필요한 windows를 생성하여 speedup으로 세면 안 된다.

H3c는 latency의 upper bound를 comparator의 lower bound와 비교하므로 논리적으로 sound하다. upper bound 두 개를 비교하는 오류를 하지 않는다. `s=8,m=3` 예의 5/8 ratio는 overhead 및 comparator 조건을 만족하는 비용 모델의 결론이다. 24 stage tasks 대 8 tasks이므로 이 예 자체는 work 감소가 아니다. failed preflight/fallback, setup, packing, barrier를 실제로 포함해야 한다. strongest applicable direct/reuse arm이 더 싸면 예제의 하계는 폐기한다.

H4는 `WU=UW_r`와 stage/source identity에서 reduced root를 lift하고, 같은 exact causal target의 uniqueness로 equality를 얻는다. mass가 singular여도 W의 가역성과 intertwining이 있으면 discrete target 정리는 적용할 수 있지만 이것이 DAE 전체 해의 존재·수렴 정리는 아니다. full physical WRMS의 lift factor `sqrt(r/n)||D_s U D_r^-1||`는 올바르다. nonlinear chart로 그대로 일반화하거나 numerical near-invariance를 exact identity로 취급할 수 없다. 실제 입력의 off-manifold residual과 lifting roundoff도 별도 budget이다.

차원 축소의 O(n^3) 대비 O(n^2)/온라인 O(nr+r^3) 결론은 직접 reduced callback, identity 검증비용 및 actual dense-factorization baseline을 전제로 한다. 기존 banded/sparse/이미 reduced backend 전체에 대한 하계 개선으로 확대하지 않는다. 이 가정하에서는 실제 산술 work가 줄어드는 양의 정리이므로 단순 no-go 결과가 아니다.

## H5의 near-causal 복구 정리에 대한 추가 독립 판단

리뷰에서 발견한 source-binding 실패를 삭제하지 않고 실제 계수에 대한 조건부 정리로 보완했다. `S=(I−B)^−1`, `T=SE`이면 `(I−B)(I−T)=I−B−E`이므로 inverse의 순서는 `(I−T)^−1 S`가 맞다. S와 E의 순서를 뒤집으면 일반적으로 틀린다. 0≤||T||≤q<1과 같은 tube의 Lipschitz majorant를 확인하면 이 inverse는 비음수다.

positive slack으로 만든 exact radius는 `d+(B+E)r=r−δ1`을 만족한다. outward radius로 바꾼 후 closure를 다시 검사한다는 조건이 있으므로 수치 enclosure에 대한 숨은 self-map 가정도 없다. 유한차원 compact product ball에서 Brouwer 존재 정리를 쓰고, 두 root 거리의 부등식에 비음수 S를 곱해 `e≤Te`를 얻으면 q<1에 의해 유일하다. actual candidate error는 `Vd`, 그 norm은 `||Sd||/(1−q)`로 감싸진다.

모든 lambda에 대해 한 common tube의 map/domain/closure/q를 uniform하게 증명한 경우에만 같은 local branch가 interval 전체에서 연결된다. 별도 window의 geometric tube overlap만으로는 같은 root를 뜻하지 않는다. 문서가 요구하는 certified shared-root 또는 connecting common tube는 이 간극을 막는다. H4의 near-causal 적용에도 reduced root의 lift가 해당 full tube 안에 있어야 한다.

positive expansion tail은 q^(m+1)||Sd||/(1−q)이며, m 공식은 epsilon>0, 0<q<1, ||Sd||>0인 branch에서 유효하다. q=0 또는 ||Sd||=0은 m=0으로 처리한다. 실제 iterative stage map에는 A=B+E의 powers를 사용해야 하며 s회 exact 종료를 재도입할 수 없다. 추가된 v=V1에 대해서는 Av=v−1≥0, v≥1이고 weighted maximum norm의 정확한 induced norm은 alpha=max_i(1−1/v_i)<1이다. 따라서 같은 tube에서 actual Jacobi의 geometric rate와 roundoff floor가 성립한다. native positive v_bar는 Av_bar≤alpha_bar v_bar를 outward 검사해야 한다. 작은 q가 작은 alpha를 뜻하지 않는다는 경계도 보존됐다.

이 조건부 정리를 PROMOTE하지만 실제 source의 E/q/r가 이번에 계산되지 않았으므로 현재 production 적용은 HOLD다. 이론적 복구와 실행된 복구를 구분한다.

## 전역 정확도와 작업량 정리의 상세 판단

G1은 stage RHS 차이의 Lipschitz bound와 exact target defect를 합친 후 W inverse를 적용하는 귀납이다. `(I-A)^-1`은 finite polynomial이므로 contraction 조건이 필요 없다. `w=G(I-A)^(-T)|b|`의 transpose 순서도 맞다. backward sweep은 dependency edge당 한 번의 scalar 곱/합으로 O(s+e_s)이며, bound가 고정 tube에서 유효할 때만 최적화에 재사용할 수 있다. projected inverse norm이나 observed residual ratio는 이 전제를 대체하지 않는다.

G2의 Taylor 방법은 5차 전개와 6차 remainder의 삼각부등식이다. stored binary64 tableau의 d_1,...,d_5를 0으로 선언하지 않는 점이 중요하다. approximate J/f_t도 reference identity 또는 mismatch로 보존한다. uniform resolvent와 derivative tube가 없으면 derivative sample을 끼워 넣어 PROMOTE할 수 없다. defect reconstruction 경로는 ODE flow와 reconstruction을 잇는 weighted differential inequality로 sound하며, candidate 전체를 직접 감쌌을 때 G1 contamination을 중복 계산하지 않는 조건도 맞다.

G3의 metric ratio 방향은 `max_j s_n,j/s_(n+1),j`가 맞다. exact flow의 lognorm 상계는 true trajectory와 computed-start flow를 잇는 convex tube 전체에서 유효해야 한다. corrected Lambda는 실제 flow amplification을 감싸는 upper bound다. 이 조건 아래 accepted grid가 adaptive하게 선택되었어도 사후 recurrence와 product/sum 식이 성립한다. initial/time uncertainty, callback assembly, arithmetic floors가 전부 포함된 경우에만 actual trajectory bound가 된다. 현재 코드에서 이 값들을 export했다고 주장하지 않는다. 고정 dissipative metric과 uniform tau<=C h^6인 regime에서는 sum h^6<=T h_max^5이므로 stated fifth-order global bound가 맞지만 stiff-uniform C는 추가 조건이다.

G4의 KKT 해, upper-cap handling, a_i=0 처리 및 integer ceiling bound를 직접 검토했다. cost bound 비교의 대상이 realized early-stop trajectory가 아니라 같은 검증된 q를 사용하는 scheduled/certified iteration budget임을 유지해야 한다. 비용과 영향도가 residual 선택으로 변하면 같은 고정 최적화 문제라고 할 수 없으며, 문서의 공통 tube 조건을 먼저 확인한다. iteration floor 및 모든 allocator/certificate 비용을 공제한 B>0이 필요하다.

정확 예제는 a=(4096,1,1,1), B=1/16에서 common schedule k=17의 work68과 allocated k=(18,6,6,6)의 work36이다. weighted sum은 정확히1/16이며 continuous optimum의 모든 k가 정수여서 이 예는 해당 discrete 문제에서도 optimum이다. total work까지 줄이려면 extra cost<32 iteration units가 필요하다. 실제 VigilODE trajectory의 측정값으로 쓰지 않는다.

family m=2^r, L>=r, B=2^-b에서는 uniform work=m(L+b+1), allocation work=L+m(r+b)가 정확하다. b 고정, L≫m log m에서 Θ(mL)→Θ(L+m log m)이므로 nonempty positive complexity regime가 있다. fixed eight-stage RODAS 하나의 n-scaling을 이 식으로 바꿨다고 할 수 없고, 전역 m=N s를 online 배분하려면 future amplification의 사전 상계가 필요하다. exponent/weight bit 처리비를 무시한 arithmetic claim을 bit complexity로 확대하지 않는다.

## 승인 한계

수학적 조건이 만족될 때의 accuracy/work/span 결과는 승인한다. 조건부 정리를 실제 production PROMOTE로 연결하려면 exact target·metric·model epoch, full-space gain, coefficient-aware truncation, complete flow tubes, contraction/floor 및 모든 비용의 native witness가 필요하다. 이는 사용자에게 다시 이론만 반복하라는 의미가 아니라 승인된 결론을 적용할 정확한 입력 계약이다. 이 검토는 신규 실험의 검증 설계를 작성하거나 기존 연구를 재실행하지 않았다.

Richardson의 m_W=M_W 경계에서는 q=0이므로 log-based allocation을 직접 평가하지 않는다. 한 exact step과 arithmetic floor를 사용하거나 엄밀하게 더 느슨한 0<q<1 상계를 택한다. 이는 G4의 명시된 0<q<1 가정과 일치하는 적용 경계이며 최종 후보 본문에도 명시됐다. 기존 L-0048 특정 모델 global certificate를 재사용한다는 서술과 신규성 미확립 표기도 확인했다.

최종 통합 보고서 `docs/reviews/20261011_promotion_theory/REPORT_KO.md`의 §2.2와 §5도 범위만 확인했다. causal reference/actual leakage, 공통 lambda tube, 미계산 B/E/q 및 near-causal H4의 lifted-root tube 조건이 일치한다. 이 통합 확인은 새로운 증명이나 실험의 재수행이 아니다.
