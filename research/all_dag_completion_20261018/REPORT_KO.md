# Vigilode 전체 DAG 구현·적대적 재감사 보고서

## 판정과 이번 변경의 범위

**원래 DAG A01–A12를 모두 전담 agent에 배정하여 구현·증명·표적 검증을 진행했고, 별도 독립 심사를 통과했다.** 이번 결과는 특정 노드 하나의 연구 제안이 아니라, native Rust 코드와 요청 출력의 인증 경로, 실패 작업량을 보존하는 제어기, 재현 가능한 실행 도구를 함께 포함한다. 기능별 선언된 인증 API 범위는 통과했다. 범용 production solver 및 일반적인 속도 우위는 아직 승격하지 않는다.

원래 `NEXT_STEPS.json`의 12개 노드·acceptance·dependency는 `ORIGINAL_DAG.json`과 이번 canonical `DAG.json`에서 대응한다. 과거 루트 `DAG.json`이나 역사적 continuation 문서를 이번 판정으로 덮어쓰지 않았다. 판단의 주체는 `nodes/A12/DECISION.json`, 실행 소스의 주체는 `SOURCE_MANIFEST.json`, 최종 실행은 `evidence/final_native/RESULT.json`이다.

시작 시 cloud `ac89a9a…`와 audit `7e28738…`를 통합했다. 작업 중 추가된 cloud `86afb40…`와 audit `59d95eb…`도 최종 트리에 병합했다. 후자는 새 solver 소스를 바꾸지 않았지만, 감쇠 고차 확장 연구와 Leja oracle fixture를 추가했다. 신규 oracle는 실제로 실행했고, 변하지 않은 과거 대규모 캠페인은 반복하지 않았다. `REMOTE_DELTA_REVIEW.json`에 이 구분을 기록한다.

## 전체 노드별 실제 산출물

| 노드 | 이번에 구현·연결한 내용 | 확인된 범위와 한계 |
|---|---|---|
| A01 | weighted initial ball의 불변 owner, 반환 중심과 radius 결합 | 실제 저장 weight를 사용한 κ=0 packet 인증. 80개 독립 Fraction disk 확인 |
| A02 | transactional advance, bounded adaptive splitting, headroom 예약, A10 선택적 fallback | 원래 mesh·허용오차·초기 불확실성 보존. 거절 시 state 불변, 작업량 유지 |
| A03 | 정확한 감쇠 frame, Q00 damped moment 평균 forcing, 명시적 시간변동 오차, multiwindow | κ≥0·carrier=0. 일반 주파수에서는 보수적 variation bound로 subwindow가 많을 수 있음 |
| A04 | complete arithmetic certificate의 immutable budget-admission token | 원래 budget을 넘는 결과·legacy estimate-only 결과를 소비하지 못하게 함 |
| A05 | immutable model capability를 받는 generic continuous-residual driver | PR101·collective·A11 GrayScott 지원. 임의 callback은 typed unsupported |
| A06 | Schur inverse fallback, 8개 stage의 선택적 residual witness, 비교 작업 계수 | exact represented weight sum=1 및 carrier=0에서 O(N) structured 경로 유지 |
| A07 | source/build/binary binding, 실패 행 보존, achieved-error matching, process bootstrap | 새 36행 검증 완료. 이번 8-mode 측정에서는 structured가 더 느림 |
| A08 | 동일 요청의 Leja/Laguerre/Chebyshev guarded selector | 18개 exact Fraction containment. Krylov estimate-only와 PY02 ABSTAIN은 승격하지 않음 |
| A09 | 원래 actual-K H5 stage client와 별도 nonlinear window homotopy 모두 구현 | floating α/β 누출 보존. stage root 인증과 true-ODE endpoint 인증을 구분 |
| A10 | frozen continuous residual의 outward Bernstein refinement, A02 fallback 연결 | 같은 물리 budget에서 거절→수용 구성 사례. 비용은 depth에 따라 증가 |
| A11 | 실제 GrayScott trajectory tube와 full symmetric-part bound 연결, callback 임시 배열 제거 | μ₂ bound가 O(C). 목표 grid/horizon에서 시간·메모리 실측은 별도 |
| A12 | 별도 agent의 수학·코드·증거 독립 심사와 scope별 결정 | 차단 결함 없음. broad production·speed는 HOLD |

각 노드의 전체 유도·API·원시 로그·오류와 수정 내역은 `nodes/` 아래에 있다. 테스트 숫자를 여러 실행에서 중복 합산하지 않도록, 최종 실행 결과와 노드별 고유 테스트 수를 구분했다.

## 정확도 향상: 실제 문제를 발견하고 수정한 부분

### 연속 출력의 interval dependency

A05의 첫 vector 시험은 단순히 h를 줄여도 통과하지 못했다. 기존 전체 구간 평가에서 `(1−θ)y₀ + θy₁`의 두 항을 독립 interval로 취급하면, 실제 차이 `y₁−y₀`가 작아도 O(|y₀|) 폭이 남는다. 새 구현은 같은 quartic을 outward power coefficient로 변환하고 Horner 방식으로 평가한다. 점값 계산식과 수치 target은 그대로 두고, 구간 포함성의 불필요한 폭을 줄였다.

PR101의 원래 101개 출력·atol=1e−11·rtol=1e−8을 그대로 적용한 실행은 492 accepted/292 rejected였다. 최대 관측 오차는 8.382183835919932e−15, 최대 인증 출력 radius는 4.514198597243331e−12였다. 같은 입력의 기존 Off/Enforce 경로는 최대 정규화 관측 오차 5.3829/1.9606을 보였다. 이 비교는 새 residual 제어의 정확도 필요성을 보여주며, 그 계산 비용까지 포함한 속도 우위를 주장하지 않는다.

### 전역 packet의 budget starvation

A02 첫 실행은 앞 window들이 전체 허용오차의 여유를 거의 사용하여, 뒤 window가 계속 세분화되어도 완료되지 못했다. elapsed-time headroom으로 더 엄격한 중간 예산을 예약하고, 최종 accept에는 원래 physical gate를 그대로 적용했다. Headroom은 step 선택 heuristic이며 인증 정리의 전제가 아니다.

동일 비선형 fixture가 16 accepted/14 rejected로 완료되었다. 최대 mean/mode radius는 각각 2.178607416139314e−5/4.8715142787641056e−5로 원래 1e−4 이하였다. A10을 켠 같은 fixture도 16/14여서, 이 실행에 대한 거절 수 감소는 주장하지 않는다.

### 감쇠를 포함한 인증 경로

A03은 `a(t)=exp(−κt)b(t)` 변환 후의 계수 `χ exp(−2κt)`를 Q00 moment로 평균하고, 원래 시간 의존 계수와의 차이를 명시적 model error로 더한다. 평균 계수만 사용하고 원래 문제를 바꾸는 방식이 아니다. 동일 주파수·1모드의 commuting 경우에는 이 평균이 정확한 흐름을 준다. 일반 경우의 변화량 경계는 보수적으로 1차여서, 예제의 successive window에 8/16/128개 subwindow가 필요했다.

3모드·3출력의 원래 명시적 mean/mode budget 1e−6에서 최대 radius는 4.664373027753387e−7/9.328746054396551e−7이었다. 이 budget은 이번 fixture의 선언값이다. 과거 damped-local 요구를 PR101의 rtol=1e−8·atol=1e−11로 소급해서 바꾸지 않았다.

## 계산량을 줄이는 구성적 결과

### A06: 보수적인 Neumann 거절 제거

동결 stage 행렬의 Woodbury 표현에서 `P=D⁻¹`, `S₂=I−sWPUC`라 하자. exact weight sum=1인 현재 admitted domain에서 `||U||=||W||=1`, `p≥||P||`, `L≥||C||`이면

\[
\|A^{-1}\|_w\le p+s p^2 L\,\|S_2^{-1}\|_2,
\qquad
\|S_2^{-1}\|_2\le\frac{\|\operatorname{adj}S_2\|_F}{|\det S_2|}.
\]

증명은 Woodbury 항 각각의 submultiplicativity와 2×2 inverse 식에서 바로 나온다. 구현은 모든 오른쪽 항과 determinant의 0 배제를 outward arithmetic으로 확인한다. 기존 Neumann 경계가 더 좋은 영역에서는 그대로 재사용하고, 그 조건이 실패해도 Schur가 검증 가능하면 새 경계로 진행한다. 이로써 유효한 비선형 stiff 사례를 불필요하게 거절하지 않는다. Dense inverse를 만들 필요 없이 setup/RHS 처리를 O(N)으로 유지한다. 원래 singular Schur 거절은 유지한다.

### A11: full μ₂를 O(C)로 계산

Periodic diffusion은 Euclidean norm에서 음의 준정부호이다. 전체 symmetric Jacobian의 상한은 각 grid cell의 2×2 반응 symmetric block 최대 고유값 상한으로 제어된다. 따라서 dense symmetric eigensolve 대신 C개 block의 outward 계산으로 충분하다. 실제 계수기는 finite 정상 domain에서 `24C+5` provider primitive를 기록했다.

이 경계를 단순 supplied box에만 적용하지 않고, 실제 continuous candidate와 초기 radius를 확장한 tube에 적용했다. RHS/JVP도 동일 stencil 순서를 유지하면서 각 callback의 임시 Vec 2개를 제거했다. 이는 한 RHS/JVP 쌍에서 4개 할당과 4C개의 zero initialization을 제거하는 코드 수준의 계산량 감소이다. 전체 solver의 wall time 승격과는 별개다.

### A09: nonlinear homotopy의 정확한 target과 병렬 구조

공통 cubic collective 모델에서 `m=Σwⱼaⱼ`, `S=Σwⱼ`라 하면 nonlinear subflow는

\[
B_\lambda^h(a)=a+\frac{m}{S}\bigl(e^{i\lambda\chi S|m|^2h}-1\bigr)\mathbf 1
\]

이다. 이 닫힌 식을 사용하는 symmetric split candidate, window별 병렬 fine evaluation, 복소 diagonal affine prefix를 구현했다. projection은 prefix 중간이 아니라 전체 raw recurrence 뒤에서 수행한다. M개 window·N개 mode에서 vector prefix의 work는 O(MN), serial mode loop를 포함한 span은 O(N log M)이다. setup·iteration·최종 독립 인증 비용도 포함해야 한다.

정확한 인과적 iteration은 global contraction 상수<1 없이 finite front와 binomial/factorial error bound를 갖는다. 이 현상 자체는 기존 parareal 이론이다. 이번의 기여는 모델별 상수, 실제 S, projection과 prefix 순서, 독립 true-ODE residual 인증, 구현된 failure/work 계약의 결합이다. floating iteration을 정확한 nilpotent 연산으로 취급하지 않는다. 최종 endpoint certificate는 candidate의 일반 binary64 sin/cos를 인증 authority로 사용하지 않는다.

원래 stage A09도 별도로 닫았다. 실제 represented α/β의 diagonal/upper leakage를 모두 보존하고, `A=B+E`, `S_B=(I−B)⁻¹`, `q=||S_BE||<1` 및 `d+Ar≤r`를 검증한다. λ∈[0,1]의 공통 tube가 branch와 endpoint를 묶는다. 이 인증은 actual stage target의 root에 관한 것이며, ODE discretization error를 자동으로 인증하지 않는다.

### A08/A10: 선택적으로 지불하는 인증 비용

A08 FirstAdmitted는 첫 complete-error admitted backend 뒤의 나머지 backend를 실행하지 않는다. CompareAll은 동일 target/budget에서 실제 시도 비용을 모두 보존한다. Laguerre degree가 signed-adjoint authority 한도 128을 넘으면 matvec 전에 abstain한다. degree 감소를 elapsed speed 감소로 바꾸어 말하지 않는다.

A10은 cheap bound가 실패한 뒤에만 선택적으로 비용을 지불한다. 고정된 residual polynomial의 각 mode Bernstein 상한 `Uⱼ`로 `rho_new=min(rho_old,sqrt_up(Σ_up wⱼUⱼ²))`를 얻는다. 같은 source·tail·초기 tube를 유지하므로 기존 transport 정리에 바로 대입할 수 있다. depth D의 현재 구현 비용은 mode별 O(d²(1+D·2ᴰ)), 추가 live storage O(d)이며 D≤8로 제한한다. 실제 nonlinear fixture의 endpoint radius는 1.96828196e−3에서 1.96740628e−3으로 줄었다. 이 사이의 동일 budget으로 거절→수용을 구성했지만, budget을 이 결과 이후 선택한 예제임을 명시한다.

## 속도 판정과 최우선 실행 방향

새 A07 campaign은 8 modes, 3 independent processes, process별 2 repeats, 3개 기존 tolerance로 36행을 생성했다. 모든 행의 schema/source binding과 empirical accuracy gate가 통과했고, achieved absolute-error ratio도 1.05 이내였다. allocator positive control도 통과했다.

그럼에도 paired process median의 **dense/structured 시간 비율은 약 0.392–0.437**이었다. 이 작은 문제에서는 structured 경로가 약 2.3–2.6배 느렸다. 계수기·allocator instrumentation이 포함된 diagnostic 측정이고, 작은 표본의 bootstrap CI이므로 일반적인 하드웨어 결론으로 확장하지 않는다. 이전 local 32-mode 결과가 다른 경향을 보인다는 사실도 현재 8-mode 반례를 없애지 않는다. 따라서 자동 backend routing의 다음 기준은 차수나 O(N) 표기 자체가 아니라, 동일 정확도·출력·setup 경계에서 확인한 dimension/horizon별 crossover여야 한다.

A09의 이론적 speedup은 조건부 cost inequality이다. 현재 native correctness와 span 구조를 확인했지만, 실제 end-to-end speedup을 검증한 것은 아니다. continuation stage를 늘리면 작업량도 늘기 때문에 direct λ=1 실행보다 빠르다는 주장을 하지 않는다.

## 최신 연구 결과와 남은 경계

최신 audit branch의 A03 stable confluent `U_Q` 연구, rate/time interval adapter, positive beta-cell fallback 및 full continuous reconstructed residual은 더 높은 차수의 별도 확장 경로로 통합 보존했다. 현재 Q00 경로의 `2κ`는 finite인 binary64에서 정확한 scaling이므로, 일반 `2κ(q+1)`의 곱 반올림 경고와 구분된다. 해당 고차 경로가 이미 구현되었다고 표시하지 않는다. 이것은 이번 original A03의 bounded endpoint/multiwindow 기능에 대한 선행 차단 조건이 아니다.

새 Leja legacy oracle는 218개 numeric comparison과 calibrated warmed allocation 검사를 통과했다. 그 API의 authority는 여전히 EstimateOnly이다. 새 immutable enclosed admission의 인증 범위와 혼동하지 않는다. F09의 overflow는 matvec 이전 initial norm에서 발생하므로 “matvec overflow 검증”이라고 서술하지 않는다.

역사적 ledger의 L-0124/L-0138 hash mismatch는 새로운 source-bound receipt와 분리해 보존했다. 최신 cloud commit은 기존 행의 JSON 의미를 유지하면서 공백 형식 126행을 바꾸고 L-0139를 추가했다. 이를 “과거 파일 bytes 불변”이라고 부르지 않는다.

## Local Codex에 남기는 일

`CODEX_HANDOFF_KO.md`는 그대로 전달할 prompt이고, `HANDOFF_TASKS.json`은 실행 가능한 command/dependency/acceptance를 제공한다. 기본 업무는 branch/source 확인, 해당 환경의 컴파일, 작은 환경 차이 수정, 기존 예제와 target hardware 측정이다. 각 노드의 핵심 증명이나 구현을 local에서 다시 시작할 필요는 없다. 변경 없는 published 연구를 재증명하거나 기존 큰 campaign을 반복하지 않는다.

후속 선택 연구는 `NEXT_STEPS.json`에 구현 위치·정확도/속도 목표·통과 기준까지 적었다. 이 선택 연구의 미완료를 이번 전체 DAG의 특정 노드를 빠뜨린 것으로 숨기거나, 반대로 scoped 결과를 broad production으로 승격하는 근거로 삼지 않는다.

## 참고하는 1차 문헌과 검증 권한

- Gander–Vandewalle, *Analysis of the Parareal Time-Parallel Time-Integration Method*, DOI 10.1137/05064607X: https://www.unige.ch/~gander/Preprints/parareal.pdf
- Gander–Hairer, *Nonlinear Convergence Analysis for Parareal*: https://www.unige.ch/~gander/Preprints/gander_plenary2007.pdf

SciSpace/Exa는 문헌 검색, Wolfram은 symmetric Jacobian과 combinatorial identity의 보조 검산에 사용했다. 실제 인증 권한은 각 proof의 전제와 outward 구현, 독립 oracle 및 source-bound 실행 증거에 둔다. 문헌·CAS 검색 결과를 전체 구현의 자동 증명으로 취급하지 않는다.
