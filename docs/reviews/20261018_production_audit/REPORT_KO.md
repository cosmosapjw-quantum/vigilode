# VigilODE 최신 최적화의 production 적격성 감사

감사일: 2026-10-18 KST. 기준 코드는 `docs/diffsol-benchmark`의 `2fc61b944fe431d40ba455c373579bce00b0c39a`이다. 결과와 수정은 별도 `audit/production-readiness-20261018` 브랜치에 보존한다. 기계가 사용할 현재 판정은 [DECISION.json](../../../research/production_audit_20261018/DECISION.json), 개발 순서는 [NEXT_STEPS.json](../../../research/production_audit_20261018/NEXT_STEPS.json), 실행 기록은 [VALIDATION.json](../../../research/production_audit_20261018/VALIDATION.json)에 있다.

## 판정

**최신 최적화 전체를 production solver로 승격할 자격은 아직 충족되지 않았다.** 이번 감사는 실제 반례를 확인하고 좁은 코드 수리를 수행했다. 범용 RODAS5P의 기존 문제별 수치 사용, 새 특수 패킷 경로의 엄밀 오차 인증, 실제 속도 우위는 서로 다른 판정 대상이다.

| 사용 범위 | 현재 판정 | 근거와 남은 조건 |
|---|---|---|
| 기존 generic RODAS5P/MF의 검증된 문제별 사용 | SCOPED_USE | 기존 연구와 새 stiff 예제는 유지한다. 사용자 문제·관측 시각·허용오차를 별도로 검증한다. |
| 모든 관측 시각에 대한 전역 허용오차 보장 | HOLD | endpoint embedded controller와 dense interpolation은 별개다. 새 exact-reference 예제에서 차이를 수치 확인했다. |
| LP01–LP04 특수 collective packet의 엄밀 인증 | HOLD | 감쇠 무시·시간 restriction·전달식 결함을 수리했다. full outward residual·재구성·입력 model binding이 남는다. 반환 `roundoff_enclosed=false`로 명시한다. |
| public Leja action의 무조건적인 binary64 total certificate | HOLD | 위조 cache·suffix 전파를 수리했다. subnormal을 포함한 산술 의무를 닫기 전에는 추정값 지위를 유지한다. |
| Diffsol 대비 production 속도·zero-allocation 우위 | HOLD | 데이터/보고서 불일치, 상수/추정 allocation, 출력·setup 차이, 실패 arm 누락이 확인됐다. |
| 이번에 증명한 조건부 수학 개선 | ADOPT_SCOPED | 가중 lognorm, Bernstein certificate, beta-prefix 및 suffix 비용 개선, rank-two 직접해법을 해당 전제에서 채택한다. |

LP10의 `PROMOTE_SPECIALIZED_MODULE_RETAIN_PRODUCTION_DEFAULT` 중 certified packet 승격은 이번 판정으로 대체한다. 당시 JSON과 성공 로그는 원본으로 남긴다. 전문적인 사용자가 문제별 검증을 붙여 generic solver를 쓰는 것은 가능하지만, 현재 자료로 범용 production 적격성과 전역 오차 보장을 함께 선언할 수는 없다.

## 기준과 근거 재사용

최신 구현은 LP01–LP04/LP09–LP10, PY01–PY05, SP02/SP04, AS04/AS05, PC01, HT01/HT02, EX01, ME01과 최신 Diffsol 보고서를 포함한다. 이전 native `76e59c691845851274cf27b26678cdb128daa408` 대비 변경된 crate 파일 63개를 [SOURCE_IDENTITY.json](../../../research/production_audit_20261018/SOURCE_IDENTITY.json)에 결속했다. 이는 63개 각각에 동일 깊이의 형식 검증을 수행했다는 뜻이 아니라 변경 영향의 경계다. packet·polynomial·sparse·benchmark·기본 driver의 중요한 경로를 독립 분담하고, 실제 실패를 만드는 새로운 입력에 집중했다.

이전 `bbb8ef42ad4e9b49704b165a54a26c435c575ae3`의 Loop12 연구 폴더와 문서는 최신 기준에서 변경되지 않았다. 그 조건부 정리, 기존 실패 범위, 이전 audit의 종료된 결과를 재사용했다. published campaign의 일괄 재실행은 하지 않았다. 이번 실행은 새 반례, 이번 수정의 회귀, 새 수식의 독립 검산이다. 사용자 제공 Rust 1.94.1과 offline vendor로 locked build를 수행했다. 첨부 Bianchi/BASS 문서의 프로젝트 한정 규칙은 VigilODE의 연구 대상을 바꾸는 지시로 적용하지 않았다.

운영 규약은 현재 `physmath-research-loop` 내장 1.0.0의 R1–R10/C1–C10이다. 이전 Astra 원본은 역사적 provenance로 유지한다. SciSpace의 논문 탐색, Exa의 Diffsol 0.16.1 공식 API 확인, Wolfram의 모델 Jacobian 검산을 수행했다. 모델 이름이나 상태 파일의 개수로 과학적 타당성을 판단하지 않았다.

## 적대적 감사에서 확인한 오류

전체 원자료·라인 위치·수정 범위는 [PACKET_AUDIT_KO.md](PACKET_AUDIT_KO.md), [SOLVER_AUDIT_KO.md](SOLVER_AUDIT_KO.md), [PERFORMANCE_AUDIT_KO.md](PERFORMANCE_AUDIT_KO.md), [DEFAULT_PATH_KO.md](DEFAULT_PATH_KO.md)에 있다. 다음 수치는 감사 기준 commit 또는 명시된 수정 후 실행에 대응한다.

### 1. 방정식과 출력·오차 반경의 연결

감쇠 모델을 κ=0 compiler에 넣으면 감쇠가 사라졌다. 예를 들어 한 mode, 초기값 0.5, κ=1, width=χ=0, h=0.1이면 기존 출력은 0.5이고 certificate는 0이었다. 실제 값은 `0.5 exp(-0.1)`이며 오차는 약 0.04758이다. 이 브랜치에서는 κ>0를 해당 compiler에서 명시적으로 거절한다. scalar damped-moment table의 존재만으로 damped packet solver가 완성되는 것은 아니다.

중간 시각 출력은 horizon을 hτ로 바꾸면서 원래 정규화 시간 다항식의 계수를 그대로 사용했다. 올바른 restriction은 `c_k → τ^k c_k`이다. 독립 해 `a(t)=0.5 exp(i·0.25t)`에서 기존 τ=0.25 오차는 약 `2.93e-5`였다. 이 변환과 endpoint/초기값 회귀를 구현했다.

trajectory는 이미 tail이 곱해진 `kernel_model`을 비선형 force 상수 G 자리에 다시 사용했다. 수정된 전달식은, 원래 unit-tube 및 정규화 가정 아래,

\[
B_m=\frac{D+K_{\rm model}+E_0}{1-hL},\qquad
B_j=B_{j,\mathrm{local}}+\frac{hL E_0}{1-hL}+\rho_j.
\]

초기 불확실성을 포함하는 true-state tube도 확인해야 한다. 새 경계 검사는 outward로 `S·Σw_j(|â_j|+ρ_j)^2≤1`, `S=Σw_j`를 확인한다. 큰 초기 반경을 받은 뒤 unit-domain Lipschitz 상수를 그대로 쓰는 사례를 차단했다. NaN candidate와 변조된 public config, 아주 짧은 양의 시간 구간도 별도 회귀로 다뤘다.

다만 현재 packet residual·tail·재구성은 전체가 구간 계산으로 연결되어 있지 않다. public packet field의 identity와 가중치/주파수의 softened admission도 남는다. 이러한 남은 의무 때문에 이번 수정 뒤에도 `roundoff_enclosed=false`를 유지한다. 상태가 실제로 전진했다는 사실과 검증된 반경이 참해를 포함한다는 사실은 별도 계약이다.

### 2. 인증된 계수의 출처와 recurrence 산술

public Leja cache에 외부 계수를 넣을 수 있어 잘못된 0 계수로 `Certified`를 받을 수 있었다. 실제 실행에서는 `exp(diag(-1,-2))·(1,1)`의 출력이 `(0,0)`, 표시 bound가 약 `1.34357e-7`, 실제 오차가 약 `0.39198`이었다. 인증 cache의 생성 경로를 제한하고 내부 검증된 coefficient 생성 API를 제공했다.

recurrence rounding의 전파에 prefix node polynomial bound를 suffix product에 대입하는 오류도 확인했다. 첫 두 node의 prefix bound 약 4.0625를, 값 약 8을 갖는 다른 두 node의 suffix에 사용할 수 없다. 실제 연속 node 구간별 outward bound를 저장하고 올바른 suffix를 참조한다. 독립 검토에서 coefficient radius와 recurrence error의 교차항도 발견해 계수 상한을 `|d_mid|+d_rad`로 교정했다.

수치 계산값은 계속 제공하되, unrestricted binary64의 underflow 항까지 닫히지 않은 total certificate는 현재 production authority를 부여하지 않는다. 이 판정은 정상 입력의 다항식 수렴 정리를 폐기하는 것이 아니라, 그 정리를 반환 산술에 연결하는 남은 항을 특정한 것이다.

### 3. sparse 경로의 입력과 상태

CSC의 row 범위·column pointer 순서·finite value 검사, 실제 symbolic pattern identity, numeric factorization 완료 상태가 필요하다. 기존 코드에는 잘못된 sparse 입력의 panic, 같은 길이지만 다른 pattern의 재사용, 미분해 상태의 solve 허용이 있었다. 새 경계 검사는 잘못된 입력과 상태를 typed error로 거절한다. RHS split 검증의 NaN 비교도 검사한다.

Gray–Scott의 `n_grid=2`에서는 주기 경계의 이웃이 겹친다. sparse assembly가 같은 index를 중복 제거하면서 계수를 더하지 않아 본래 RHS와 약 0.032 차이가 났다. 이제 겹친 항을 합산한 canonical CSC를 만든다. 이 수리는 실제 방정식을 유지하기 위한 것이며 성능 승격 근거로 세지 않는다.

### 4. 벤치마크에서 실제로 측정한 것

LP09는 같은 tolerance 설정만으로 `fairness_verified=true`와 `PASS`를 반환했다. packet 반경과 두 solver 사이의 차이를 모두 reference error처럼 표시했고 allocation은 두 arm에서 0 상수였다. 수정한 v2 report는 독립 reference error와 allocation을 미측정 `null`로 표현한다. 계산된 반경, cross-method discrepancy, 구조적으로 제거한 연산, 실제 계측한 work를 분리하고 `DIAGNOSTIC_ONLY`를 반환한다.

최신 Diffsol 보고서의 timing 52개 중 49개는 연결된 JSON과 값이 다르다. 예를 들어 Robertson BDF, rtol=1e-4는 표 77μs와 JSON 146μs가 대응한다. allocation은 일부 짧은 구간의 외삽 또는 step 수에 상수를 곱한 값이며 VigilODE는 0을 직접 쓴다. 성공 52개 row만 JSON에 남고 실패 arm은 보존되지 않는다. 출력 보관과 setup timing도 다르고 Robertson atol은 한쪽이 component vector, 다른 쪽이 scalar다. Brusselator의 문서상 계수·경계조건도 양쪽 구현이 실제 계산한 방정식과 다르다.

따라서 3배·7배 등의 수치를 동일 정확도·동일 출력의 속도 우위로 인용하지 않는다. 이는 비교 solver가 더 빠르다는 판정도 아니다. 정확도-시간 Pareto 비교의 입력 계약과 데이터 계보를 바로잡아야 한다. 공식 API상 Diffsol `solve`는 adaptive 시간의 해를 반환하고, `solve_dense`로 공통 관측 격자를 지정할 수 있다. 다음 비교에서는 이 차이를 실제 실행 비용에 반영한다.

## 기본 solver의 실제 사용 가능 범위

JVP-only public stiff 예제는 이번 실행에서 정상 종료했다. endpoint의 absolute error는 약 `2.44e-9`, accepted step은 93이었다. nonfinite 설정·초기값·RHS 실패에 대한 주요 거절 경로도 확인했다. 이것은 기존 protected 경로의 실용성을 지지한다.

전역 관측값 보장은 더 강한 조건이다. exact solution `y(t)=sin(t)`인 nonlinear Prothero–Robinson 방정식

\[
y'=-10^6(y-\sin t)+\cos t+10(y-\sin t)^2
\]

을 [0,1]에서 rtol=1e-8, atol=1e-11, 관측점 101개로 실행했다. default dense control Off에서는 6개 accepted step으로 성공하고 endpoint 오차는 약 `5.23e-10`이지만, 최대 dense 오차는 `1.3083e-5`, 정규화 관측오차는 약 1801.616이었다. 이는 문서화된 endpoint 제어와 cubic dense extension의 범위 차이를 보여준다.

같은 문제에 기존 dense control Enforce를 사용하면 최대 absolute error는 `7.10284e-9`로 줄었지만 최대 정규화 오차는 약 1.498574였다. 17 accepted/24 rejected, 추가 LU와 Jacobian 비용도 발생했다. 이 방법은 유용한 개선이나 전 관측점 인증은 아직 충족하지 않는다. exact callback을 solver에 제공하지 않고 참값은 사후 검증에만 사용했다. 자세한 설정과 결과는 DEFAULT_PATH_KO.md 및 원본 JSONL에 있다.

## 정확도와 속도 개선을 보장하는 새 연결

완결된 증명은 [THEORY_KO.md](THEORY_KO.md), 저랭크 직접해법은 PERFORMANCE_AUDIT_KO.md 및 exact reference code에 둔다. 학술적 최초성을 주장하지 않는다.

1. **가중 기하를 이용한 sharp gain.** `m=Σw_j a_j`, `S=Σw_j`, weighted norm에서 collective cubic vector field의 logarithmic norm은 정확히 `−κ+S|χ||m|²`이다. 일반 Lipschitz 추정의 factor 3을 제거한다. 초기 에너지 E를 함께 사용하면 두 해의 flow gain은 `exp(−κh+S²|χ|E θκ(h))`이다. κ=0, S=E=|χ|=h=1의 경우 증명 gain이 e³에서 e로 줄어든다. tube 계산 O(N), window gain O(1)이며 JVP 조립이 필요 없다. 실제 출력 곡선의 continuous residual과 outward 계산을 연결하면 더 큰 residual 예산과 더 적은 불필요한 rejection을 허용한다.
2. **Bernstein 전 구간 상계.** coefficient sum 대신 Bernstein convex hull 상계를 사용하면 exact arithmetic에서 bound가 커지지 않고 dyadic subdivision으로 수렴한다. `(1−τ)^12`의 coefficient bound 4096을 정확한 1로 줄일 수 있다. 변환 O(d²) 비용이므로 싼 상계가 이미 통과하면 생략하고, 인증 실패를 바꿀 때만 사용한다. 새 bound는 point sampling을 대신하는 전 구간 계산이다.
3. **감쇠 모멘트 포함성과 비용.** exact API 입력의 `κh`를 interval로 만들고 beta seed, exponential, remainder, binomial 연산을 outward 연결했다. `B_{n+1}=B_n(j+n+1)/(j+k+n+2)`를 이용해 Taylor 계수 생성 비용을 O(Nk)에서 O(N+k)로 낮췄다. N=28,k=64의 세부 연산 감소와 실제 테스트는 THEORY_KO.md에 명시한다. 큰 감쇠에서 interval 폭이 커지는 문제에는 추가 알고리즘 선택이 필요하다.
4. **Leja recurrence certificate 비용.** 올바른 suffix bound를 한 번 O(128m²)에 계산해 저장하고 누적 tail coefficient를 갱신하면 반복 scalar update가 `m(m+1)(m+2)/6`에서 `m(m+1)/2`로 줄어든다. O(m²) 메모리와 cold setup을 비용에 포함해야 한다. 아직 wall-clock speedup으로 부르지 않는다.
5. **공정한 O(N) 직접해법 baseline.** collective real Jacobian은 block diagonal + rank two다. Rosenbrock stage 행렬을 2×2 Schur complement와 mode별 2×2 solve로 처리하면 assembly·factorization·stage 적용이 O(N)이 된다. exact Fraction prototype은 full dense reference와 비교한다. current LP09의 dense LU 비교만으로 구조에 기반한 최선의 직접해법에 대한 우위를 판단할 수 없다.

## 구체적 다음 개발 순서

우선순위와 수용 기준을 [NEXT_STEPS.json](../../../research/production_audit_20261018/NEXT_STEPS.json)에 API·파일·의존성과 함께 썼다.

- **A01→A02: 실제 local certificate, 이어서 global trajectory.** packet coefficient와 output reconstruction의 interval enclosure, full residual, exact descriptor identity, S 및 spectral radius를 함께 결속한다. 그 후 T1 gain으로 inherited uncertainty를 전파한다. 반경은 반환한 바로 그 중심값을 감싸야 한다. boundary/subnormal/nonfinite/중간 출력에서 independent reference와의 포함성 검사를 수행한다.
- **A03: damped solver 연결.** scalar kernel을 연결하는 것뿐 아니라 변환된 비선형 forcing `exp(−2κt)g(u)`, initial state, carrier convention, output mapping까지 완성한다. κ=0 연속 극한과 κ>0 exact linear/cubic cases를 사용한다.
- **A04/A05: Leja 산술과 generic dense output.** matrix action·coefficient·recurrence·summation의 모든 반올림과 underflow를 닫고 certificate authority를 복구한다. generic solver에는 continuous-output defect와 검증된 one-sided gain을 실제 controller contract로 연결한다. endpoint estimator를 전역오차로 바꾸어 부르지 않는다.
- **A06→A07: 강한 baseline과 재현 가능한 비교.** rank-two direct solver를 구현한 뒤 같은 물리 모델·atol vector·관측 격자·reference·setup 경계를 사용한다. 실패를 포함한 전 arm을 보존하고 allocation은 allocator로 계측한다. 표는 검증된 단일 JSON에서 생성한다.
- **A08/A09: 다항식 선택과 homotopy 병렬성.** 기존 Laguerre quotient·Leja·근사 annihilator의 조건부 결과는 재사용한다. 먼저 scalar setup, operator products, certificate, 출력 비용을 포함한 차이를 측정한다. homotopy는 실제 비선형 target의 endpoint residual와 branch certificate를 유지하면서 독립 block/parameter work만 병렬화한다. 기존 finite-causal 정리를 현재 floating K의 정확한 유한 종료로 치환하지 않는다. `H5`의 perturbation bound 또는 실제 contraction witness와 모든 warmup/barrier 비용을 함께 검증한다.

모든 작업을 끝내야만 한 기능을 쓸 수 있다는 의존성은 두지 않았다. κ=0 local certificate는 damped solver를 기다릴 필요가 없고, Bernstein은 certificate bottleneck이 있을 때만 도입한다. production 승격은 개선된 구현·실제 모델·오차 계약·work/timing 계약을 충족한 범위마다 독립적으로 결정한다.

## 검증과 한계의 추적

새 검증 명령·exit code·baseline 실패와 수정 후 결과는 VALIDATION.json 및 evidence 로그에 저장한다. 독립 검토 원문은 [INDEPENDENT_REVIEW_KO.md](INDEPENDENT_REVIEW_KO.md)에 있다. 중요한 논증과 수정에는 별도 검토자가 관여했으며, 발견된 coefficient-radius 교차항과 S-aware tube 등은 해당 변경에만 재검토했다.

이 감사는 proof assistant에 의한 전 코드 형식 검증이 아니다. 현재 판단의 근거는 source, 완결된 조건부 유도, 새 실행 반례와 좁은 회귀, 입력 identity를 유지한 기존 근거다. native wall time 향상, 임의의 black-box ODE 전체에 대한 global tolerance 보장, 외부 cosmology client의 물리적 완성도는 이번 결과에서 승격하지 않았다. 저장·push 성공은 과학적 승격과 별도로 확인한다.
