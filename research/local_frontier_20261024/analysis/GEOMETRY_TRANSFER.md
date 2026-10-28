# 계산기하·대수기하 초안에서 VigilODE로의 구체적 이식

2026-10-24. 대상: 고정 연산자의 shifted linear solve와 정칙한 비선형 continuation. 구현: `crates/rodas5p-core/src/invariant_module.rs`; 관련 이식: conditioned polynomial provider와 fixed-P continuation. 이 문서는 기존 기하 논문의 결과를 ODE 성능 주장으로 옮기는 데 필요한 사상과 가정을 명시하고, 이번에 실제 구현한 유한 불변 모듈 해법의 증명·시험·비용을 제시한다.

## 결과

기존 초안에서 가장 직접적으로 이식할 수 있는 구조는 **정칙 chart에서의 유한 대수 계산, 중복도를 보존하는 축약, 퇴화 경계의 분리, 원래 변수로 돌아온 뒤의 검증**이다. 이 원리에 따라 `B,V,W,H`를 불변 자료로 소유하는 축약 선형 해법을 구현했다. 차원 `n`의 행렬 분해를 차원 `m`의 분해로 바꾸고, 최종 후보의 오차는 원래 `n`차원 방정식에서 계산한다. 동일한 실제 shift는 호출 내에서 한 번만 분해한다.

15개 새 계약 시험과 기존 shared-shift 6개 회귀 시험이 통과했다. 정수·유리수 기반 독립 검산 6개도 통과했다. 작은 성능 시험에서는 재사용된 축약 해법이 매번 전체 LU를 새로 만드는 비교군보다 빨랐지만, 재사용된 전체 LU보다 일관되게 빠르지는 않았다. 차이를 만드는 병목은 현재의 조밀한 outward residual 평가이다. 따라서 현재 선택은 **구조가 제공되고 setup을 재사용하는 경우에 한정한 명시적 선택 경로**다. 일반 ODE 자동 선택이나 전체 적분 속도의 개선 여부는 후속 실측 대상이다.

## 1. 원문 확인과 이식 위치

원문 파일의 이름·Library ID·SHA256은 `../evidence/GEOMETRY_SOURCES.json`에 있다. 원본 PDF/ZIP은 공개 저장소에 복제하지 않았다. PDF를 실제로 읽고 stage5 ZIP의 관련 TeX 원문을 대조했다. 기존 논문의 수치 인증을 모두 재실행하지 않고, 이번 코드가 의존하는 연결식만 새로 유도·검산했다. v7은 이전 판의 문맥에 사용하고 v13 및 minor revision의 정정 내용을 우선했다.

| 원문과 위치 | 실제 수학적 내용 | solver로의 연결 및 이번 조치 |
|---|---|---|
| `two_step_normal_reflection_walls_minor_revision_v0_1.pdf`, §2, §3.4–4.1 | 서로 다른 두 발점과 독립 법선이 정칙 carrier를 정의한다. Cramer 분모가 0인 층은 별도로 다룬다. | chart의 분모·rank·conditioning 조건을 명시한다. N1/N2의 native polynomial chart와 N6의 `WV≈I`/증폭률 검사로 연결했다. |
| 같은 원문, 정리 4.2 및 §6.2 | `0<r<1/3`에서의 정칙 wall과 `r=1/3`에서 carrier 밖의 퇴화를 구별한다. 판별식이 0이어도 다항식 차수 하락일 수 있다. 정칙 영역에서는 비영 Jacobian으로 continuation이 성립한다. | degree drop과 repeated root를 혼동하지 않는다. N4는 같은 root enclosure가 다음 contraction ball에 포함될 때만 branch를 이어 간다. |
| `curved_mirror_integrated_research_report_v13.pdf`, §9.1 | 포물선의 normal-foot cubic `u³−3u−λ=0`과 판별식 `27(4−λ²)`가 branch chamber를 구별한다. | branch label만 저장하지 않고 domain·epoch·root enclosure를 함께 유지한다. singular wall을 넘어가는 데는 새로운 chart/증명 의무가 필요하다. |
| v13 §11–14 및 stage5 `sections/04_residual.tex`, `06_affine_projective.tex` | saturation·잔여 scheme·유한 quotient algebra를 사용하며, 중복도와 boundary component를 구별한다. `k[x]/((x²−1)²)`의 길이는 4이고 단순한 서로 다른 두 점과 같지 않다. | Jordan/confluent 구조를 보존한다. N6는 `H` 전체를 저장하며 eigenvalue 목록으로 치환하지 않는다. |
| stage5 `sections/07_smooth_cubic.tex` | common-chord 좌표의 `a L_H≠0` chart에서 `H=−K/L_H`; 분모 0·수평 chart는 별도로 다룬다. | 조건이 나쁜 좌표 변환을 즉시 폐기하거나 보완 chart로 전환한다. N1/N2에는 원래 연산자의 잔차를 남기고 native chart에서 relation을 평가하도록 연결했다. |
| v13 및 stage5의 finite collision 계산 | affine ordered/unordered 길이 116/58은 해당 기하 대상의 결과다. 과거 projective total은 boundary 문제로 정정됐다. | 해당 수치는 solver의 연산량이나 성능의 근거로 사용하지 않는다. 이번 선택은 아래의 새로운 행렬 항등식과 실제 코드 실행에 따른다. |

추가로 saturation만으로 모든 bad-locus 접점이 없어지는 것은 아니다. 예를 들어 `(xy):(x)^∞=(y)`는 여전히 `x=0`과 만난다. 실행 코드에서는 같은 원리를 적용해 algebraic relation의 존재와 실제 현재 입력의 허용성을 분리했다. 이 연결은 singular chart를 잘못 통과하는 구현을 방지하면서 정칙 chart의 계산을 재사용하게 한다.

## 2. 탐색한 후보와 선택

| 후보 | 구체적인 첫 연결 | 이번 선택과 남은 조건 |
|---|---|---|
| 정칙 affine chart의 polynomial resolvent | `C=(B−cI)/s`에서 `q(C)`와 실제 저장 다항식의 defect를 계산 | N1/N2에서 구현. 원래 `B` 잔차와 chart identity가 필요하다. |
| 유한 불변 모듈/국소대수 | `BV=VH`, `WV=I`에서 `V(I−αH)⁻¹Wb` 구성 | N6로 구현·검증. 차원 감소와 setup 재사용이 실제로 존재하는 문제에 선택한다. |
| branch chamber를 통한 fixed-P continuation | whole-ball contraction과 overlap inclusion으로 같은 root를 이어 감 | N4로 구현. 원래 방정식의 마지막 잔차를 검사한다. |
| CRT·대칭 분해 | coprime factor와 `P_i²=P_i`, `ΣP_i=I`, `BP_i=P_iB`에서 block solve 구성 | 다음 structural provider 확장 후보. projector 증폭률·정확한 relation·분리 여유가 필요하다. 단순 eigenvalue 분류만으로 dispatch하지 않는다. |
| 반복근을 보존한 local-algebra jet | `H=λI+N`, `N^k=0`이면 resolvent가 유한 합 | N6는 Jordan entries를 그대로 보존한다. 보다 저렴한 직접 nilpotent kernel과의 crossover를 비교해야 한다. |
| affine reflection의 associative prefix | `(A₂,b₂)∘(A₁,b₁)=(A₂A₁,A₂b₁+b₂)` | 합성 자체는 정확히 associative하다. 각 stage map이 실제로 affine이고 사전에 알려진 경우에만 sequential-depth 감소 후보다. 일반 nonlinear flow의 stage 의존성은 별도 모델링이 필요하다. |
| singular wall 근처의 다른 parameterization | fold에서 pseudo-arclength 및 보완 chart 구성 | 확장 연구 후보. arc-length 제약과 augmented solve 비용을 포함한 비교가 필요하다. 현재 fixed-P ball 밖으로 자동 연결하지 않는다. |

이 후보군에서 삭제한 것은 기하 아이디어 자체가 아니라, 검증된 사상 없이 기하적 정리의 수치를 실행시간으로 해석하는 전이이다. 유한 quotient, 정칙 atlas, multiplicity, branch chamber는 모두 각각의 구현 계약으로 남겼다.

## 3. 불변 모듈 축약의 정리와 증명

### 3.1 설정

실수 행렬 `B∈R^{n×n}`, `V∈R^{n×m}`, `W∈R^{m×n}`, `H∈R^{m×m}`와 `n≥m>0`을 고정한다. 풀 방정식은

\[
 A_\alpha x=b,\qquad A_\alpha=I-\alpha B,\quad\alpha\ge0.
\]

`V`는 축약 좌표의 lift, `W`는 restriction, `H`는 그 좌표에서의 연산자이다. 일반 nonlinear vector field에서 자동으로 추출되는 자료를 전제하지 않는다. 현재 구현은 고정된 조밀 행렬을 소유하는 구조적 reference kernel이며, 행렬이나 Jacobian을 자동 생성하는 경로는 없다.

**정리 1 (rank와 chart).** `||WV−I||∞<1`이면 `WV`는 가역이고 `V`의 열 rank는 `m`이다.

**증명.** `WV=I−(I−WV)`에 Neumann 급수를 적용하면 역행렬이 존재한다. `Vz=0`이면 `WVz=0`이므로 `z=0`이다. 따라서 lift의 좌표가 rank 붕괴 없이 정의된다. 정확히 `WV=I`인 경우에는 `P=VW`가 projector이고 `||P||∞≤||V||∞||W||∞`이다. 근사 left inverse의 경우에도 우변의 lift/restriction 증폭률은 유효하지만 `VW`를 정확한 projector라고 가정하지 않는다. ∎

### 3.2 원래 방정식의 defect 분해

\[
 E=BV-VH,\qquad \delta=Wb-(I-\alpha H)z,\qquad x=Vz
\]
라 두자.

**정리 2 (lifted residual 항등식).** 임의의 `b,z,α`에 대해

\[
 r=b-(I-\alpha B)x
   =(I-VW)b+V\delta+\alpha Ez.
\]

**증명.** `Vδ=VWb−Vz+αVHz`와 `αEz=αBVz−αVHz`를 더하면 `b−Vz+αBVz`가 남는다. 이것이 원래 잔차다. ∎

따라서

\[
 \|r\|_\infty\le
 \|(I-VW)b\|_\infty+\|V\|_\infty\|\delta\|_\infty
 +\alpha\|E\|_\infty\|z\|_\infty.
\]

이 식은 RHS의 모듈 밖 성분, 축약 solve 오차, invariance leakage를 구분한다. `E=0`, `WV=I`, `b∈range(V)`, `δ=0`이면 원래 방정식을 정확히 푼다. 실제 구현은 이 세 항의 상쇄를 가정하지 않고 원래 방정식의 잔차를 outward arithmetic으로 다시 계산한다. 모듈 밖 RHS는 원래 길이를 유지하므로 projection으로 오차를 숨기지 않는다.

### 3.3 잔차에서 해 오차로

**정리 3 (dissipative resolvent).** `B+Bᵀ≤0`이고 `α≥0`이면 `A_α`는 가역이며 `||A_α⁻¹||₂≤1`이다. 따라서 임의의 후보 `x`에 대해 `||x−A_α⁻¹b||₂≤||r||₂≤||r||₁`이다.

**증명.** 모든 `u`에 대해

\[
 \|A_\alpha u\|_2\|u\|_2
 \ge \langle u,A_\alpha u\rangle
 =\|u\|_2^2-\alpha\langle u,Bu\rangle
 \ge\|u\|_2^2.
\]

따라서 `||A_αu||₂≥||u||₂`; 유한차원에서 injectivity와 inverse bound가 따른다. `A_α(x−A_α⁻¹b)=−r`에 적용하면 오차 추정이 나온다. ∎

코드는 `(B+Bᵀ)/2`의 outward Gershgorin row bound가 모두 0 이하인지 검사하는 충분조건을 사용한다. 이 충분조건이 통과하지 않는 dissipative 행렬도 있을 수 있으므로 향후 더 강한 인증 provider로 적용 범위를 넓힐 수 있다. 현재 입력행렬이 통과하지 않으면 해당 축약 경로를 사용하지 않는다.

### 3.4 chart 변경과 중복도

가역 `S∈R^{m×m}`에 대해 `V'=VS`, `W'=S⁻¹W`, `H'=S⁻¹HS`이면 정확한 lifted resolvent는 변하지 않는다. 실제 계산 비용과 오차는 `S`의 conditioning에 영향을 받으므로 `||V||||W||`를 기록하고 제한한다. 이 점이 정칙 atlas를 그대로 실행 계약으로 옮긴 부분이다.

길이 2의 local algebra를 나타내는 `H=−2I+N`, `N²=0`에서는

\[
 (I-\alpha H)^{-1}
 =(1+2\alpha)^{-1}I+
 \alpha(1+2\alpha)^{-2}N.
\]

`α=1`, `b=(1,2)`이면 해는 `(5/9,2/3)`이다. 중복된 고유값을 하나로 합쳐 `N`을 버리면 `(1/3,2/3)`을 얻어 첫 성분에 `2/9`의 오차가 생긴다. 이는 새로운 불가능성 정리가 아니라 특정 잘못된 축약에 대한 정확한 판별 예제이다. 실제 `H`의 entries를 보존하는 구현 선택을 정당화한다.

### 3.5 연산량 감소의 정확한 범위

`d`를 서로 다른 실제 shift 수, `s`를 요청 target 수, `r`을 RHS 열 수라 하자. 축약 factorization 비용은 `O(d m³)`이고, 비교할 전체 factorization은 `O(d n³)`이다. setup은 dense admission 기준 `O(n²m+nm²)`, restriction/lift 및 solve는 `O(rnm+sr(nm+m²))`이다. 현재의 원래 잔차 인증은 `O(sr n²)`이다.

따라서 `m≪n`이고 고정 chart를 재사용하는 경우 **분해 부분의 산술 복잡도 감소**가 보장된다. 전체 실행시간에는 검증·메모리·호출 비용이 포함된다. 한 RODAS step의 공통 diagonal shift는 한 개의 실제 shift로 센다. stage 수를 서로 다른 shift 수로 세어 절약량을 부풀리지 않는다.

## 4. 구현 계약과 독립 검토

`InvariantModulePlan`은 입력 `B,V,W,H`를 소유하고 mutable accessor나 Deserialize를 제공하지 않는다. setup의 outward bound와 대상 행렬의 bytes가 분리되지 않는다. 외부 `expected_epoch`가 다르면 연산 전에 오류를 반환한다. 배열 차원·비유한 입력·rank defect·leakage·증폭률·setup 및 solve 자원예산을 검사한다. 분해 캐시는 호출 내부의 정확히 같은 `α`에만 적용하고 `+0/-0`을 같은 shift로 취급한다.

`evaluate`가 만든 모든 후보는 `accepted_values()`를 통해 인증된 경우에만 전달할 수 있다. 별도 `candidate_values()`는 진단용이다. `certify_candidates`는 같은 불변 행렬의 dissipativity를 재사용하면서 독립적인 전체 공간 후보에도 동일한 최종 잔차 검사를 적용한다. 누락된 RHS/후보 열을 zip으로 조용히 생략하는 경우는 명시적으로 차단했다. work counter는 caller가 소유하여 실패한 시도에서도 계산량을 보존한다.

독립 검토에서 parent가 지적한 반복 dissipativity scan을 round2에서 제거했다. 행렬이 불변이므로 이 bound를 매 shift마다 다시 계산할 이유가 없다. round3에서는 비교군도 같은 cached certificate를 사용하게 하여 benchmark의 인증 비용을 맞췄다. 변경된 의존성만 재시험했다. 기존 source-level derivative-free polynomial/fixed-P 구현은 각 담당 agent가 개발했으며, 이식 논증과 해당 핵심 증명의 별도 검토를 교환했다.

세 번의 수정 루프는 다음과 같다.

| 루프 | 변경 이유 | 검증 |
|---|---|---|
| 1 | 불변 모듈 후보와 full residual acceptance 구현 | 새 계약 12개 통과 |
| 2 | 반복 symmetry scan 제거, 입장 전 matrix nonfinite 검사 | 새 계약 13개 + 기존 6개 통과 |
| 3 | 공정한 cached-baseline 인증 API와 malformed-column 방어 | 새 계약 15개 + 기존 6개 통과; 150개 benchmark row 모두 인증 |

최종 시험 기록은 `../evidence/N6_tests_round3.json`, 원시 stdout/stderr와 연결된다. 유리수 검산 코드는 `../src/check_geometry_transfer.py`, 결과는 `GEOMETRY_EXACT_CHECKS.json`이다. 테스트는 Jordan multiplicity, direct solve 일치, 실제 shift dedup, 모듈 밖 RHS, 근사 leakage, stale epoch, rank collapse, oblique chart, 비유한 입력, 자원예산, 독립 full-space 후보 인증을 포함한다.

## 5. 작은 성능 실험과 병목

합성 문제는 동일한 damped Jordan 2×2 block을 반복한 `n=16,64,128` 행렬이며 `m=2`, `α=0.25`, 절대 오차 기준 `10⁻¹⁰`이다. mode마다 1회 warmup 후 9회 측정하고 순서를 회전했다. single-thread 환경변수를 고정했다. CPU affinity는 고정하지 않았다. 모든 candidate의 원래 잔차 인증을 포함했다. 재사용 mode에서는 setup을 제외했다. 새 module mode에는 입력 matrix clone과 admission이 포함되고 fresh full LU에는 사전에 만든 shifted matrix에서의 LU와 dissipativity scan이 포함된다. 이 서로 다른 cold-start 범위는 의도적으로 그대로 공개한다.

중앙값, 단위 µs:

| n | 새 module | 재사용 module | 매번 full LU | 재사용 full LU | 직접 Jordan block |
|---:|---:|---:|---:|---:|---:|
| 16 | 44.477 | 16.115 | 27.261 | 15.754 | 15.223 |
| 64 | 617.441 | 232.521 | 401.876 | 233.872 | 231.639 |
| 128 | 2524.355 | 1024.104 | 1777.359 | 969.592 | 1101.551 |

150행 모두 인증됐으며 가장 큰 오차 상계는 `1.7763568394002505e-14`였다. 원시 자료는 `../evidence/N6_microbenchmark.jsonl`, build/run identity는 같은 prefix의 `.json`, 요약은 `N6_BENCHMARK_SUMMARY.json`이다. `n=16` fresh-LU의 최대값에는 3.10 ms outlier가 있어 단일 표본의 speedup을 사용하지 않았다.

이 실험이 드러내는 구조는 다음과 같다. 재사용 module은 fresh LU 대비 중앙값 기준 약 1.69–1.74배 빠르지만 재사용 full LU와는 거의 동률이고 `n=128`에서 약 5.6% 느렸다. 새 module admission은 fresh LU보다 약 1.42–1.63배 느렸다. 직접 block formula도 인증을 포함하면 재사용 solve와 비슷한 시간이 든다. 따라서 축약 factorization 절약을 전체 적분 속도 개선으로 승격하기 전에 **원래 잔차를 싸게, 여전히 검증 가능하게 계산하는 경로**가 필요하다. 현재 코드의 dense interval loop는 모든 zero entry까지 순회하므로 이 구조적 문제에서도 `n²` 비용이 남는다.

## 6. 다음 개발 단계

1. **불변 sparse/banded residual provider.** 정적 nonzero pattern과 row ownership을 plan에 묶고, 원래 residual을 `O(nnz(B))` outward 연산으로 평가한다. implicit zero의 정당성을 입력 representation이 보장해야 한다. pattern/epoch 변경, 누락된 nonzero, overlap 중복 합산, cancellation 사례를 exact dense oracle과 대조한다. dense 입력에서 단지 작은 값을 임의로 zero로 취급하지 않는다.
2. **실제 다중 shift와 반복 RHS에서 crossover를 측정한다.** 같은 `α`의 RHS만 반복되는 경우에는 full LU 재사용과 비교한다. 서로 다른 `α`가 실제로 생기는 경우에는 module setup, shift별 작은 LU, 전체 잔차 비용을 각각 측정한다. 현재 단일-shift fixture의 결과를 일반화하지 않는다.
3. **native operator provider로 확장한다.** `BV−VH` 및 original residual action의 오류 상계를 제공하는 operator contract를 구현하면 dense `B` 소유를 제거할 수 있다. 함수 평가만으로 구조를 추측하는 대신, 모델이 제공하는 불변 module을 받아 실제 action에서 확인한다. 일반 nonlinear 문제에서는 epoch마다 바뀌는 operator와 module 적합성의 비용을 포함한다.
4. **적응 선택은 검증된 조건과 관측 비용을 함께 사용한다.** full-LU 재사용이 가능한지, expected reuse, reduced dimension, setup cost, final certification cost를 비교한다. module RHS leakage가 커서 후보가 거절되면 원래 범용 solver로 되돌린다. 거절 비용과 재선택 횟수에 budget/cooldown을 둔다. 현재 새 kernel은 명시적 opt-in이며 자동 production dispatcher 변경은 하지 않았다.
5. **CRT/대칭 chart는 module provider로 붙인다.** `V_i,W_i,H_i`와 projector amplification을 생성하는 별도 공급자를 작성하면 이번 잔차·rank·leakage 검사를 그대로 재사용할 수 있다. 군 작용의 교환관계나 polynomial ideal closure를 실제 현재 연산자에서 검증한다.
6. **fold 근처는 독립 continuation 확장으로 다룬다.** `F(x,λ)=x²−λ`에서 fixed-P map의 x-미분은 x=0에서 1이므로 그 점을 포함하는 동일 contraction ball은 q<1을 만족하지 않는다. 이 정확한 국소 사실은 wall에서 patch를 바꾸거나 augmented pseudo-arclength formulation을 구성할 이유다. 다른 정칙 chamber의 fixed-P 속도 이득을 취소하지 않는다.

지역 CPU에서 수행할 명령은 `cargo test --locked --offline -p rodas5p-core --test invariant_module_contracts --test shared_shift_jet_contracts` 및 `cargo run --locked --offline --profile measurement -p rodas5p-core --example invariant_module_bench`이다. 새로운 profiling에서는 같은 source identity와 single-thread 설정을 기록하고 기존 geometric manuscript 계산은 다시 실행하지 않는다. 전체 local benchmark 인계의 host/toolchain/receipt 절차는 본 패키지의 공통 handoff를 따른다.
