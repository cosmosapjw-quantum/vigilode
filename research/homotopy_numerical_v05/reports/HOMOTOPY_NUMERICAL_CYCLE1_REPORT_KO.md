# RODAS5P partial-coupling homotopy — numerical experiment cycle 1

Date: 2026-08-07 Asia/Seoul
Branch: `homotopy-numerical-experiments-v0.5`
Frozen predecessor: `homotopy-coding-loop2-v0.4.0-alpha1` (`ce1cc510...`)

## 1. 연구 질문과 판정

이번 cycle의 질문은 다음 하나로 제한했다.

> RODAS5P의 strictly lower-triangular nonlinear stage coupling을 fixed partial-coupling
> homotopy와 low-depth nilpotent propagation으로 근사할 때, 원래 RODAS5P output defect를
> 안전하게 제어하면서 sequential depth 8보다 작은 common-`W` batch depth에서 실용적인
> fast path를 얻을 수 있는가?

첫 수치 결과는 다음과 같이 나뉜다.

1. **한 timestep 안전성:** exact nonlinear output certificate는 canonical 8,208개 candidate
   row에서 false acceptance 0을 기록했다.
2. **구조적 가능성:** `w_solve_batches < 8`인 fast acceptance가 401개 row, 16개 problem
   case에서 실제로 존재했다.
3. **전역 정확도:** 고정된 `output_wrms <= 0.1` acceptance rule은 low-depth path의 fifth-order
   global convergence를 보존하지 못했다.
4. **경제성:** fast-accepted candidate 중 어느 것도 RHS evaluations와 JVP vectors 두
   scalar-work 지표에서 sequential RODAS5P 또는 3-sweep SABR를 동시에 지배하지 못했다.
5. **정확도 대조군:** `theta=1,q=7,correction=1`은 protected sequential RODAS5P의 fifth-order
   trend를 재현했지만, sequential보다 훨씬 많은 scalar work를 사용한다.

따라서 현재 판정은 다음이다.

- exact nonlinear certificate: **PROMOTE — one-step reference oracle**;
- `q=7`: **PROMOTE — correctness oracle only**;
- low-depth fixed-budget fast path: **HOLD — global-order failure**;
- fixed-schedule method 전체: **HOLD — equal-work advantage 없음**;
- production speedup claim: **금지**.

## 2. 보존된 수학적 구조와 convention

Stage vector를

\[
K=(k_1,\ldots,k_8)
\]

로 두고 frozen step context에서

\[
W=M-h\gamma J_n,
\qquad
D=I_8\otimes W,
\qquad
C=hL\otimes J_n
\]

를 사용했다. Target stage residual은

\[
\mathcal R(K)
=(D-C)K-g-hN(K)
\]

이다. 부호는 기존 `StructuredBlockSystem::apply`와 동일하게 잠갔다.

Partial-coupling path parameter는

\[
\eta(\theta,\lambda)=\theta+\lambda(1-\theta),
\qquad
0\le\theta,\lambda\le1
\]

이고, truncated normalized coupling inverse depth는

\[
q\in\{0,1,2,7\}
\]

로 제한했다. `q=7`은 8-stage nilpotent inverse의 reference endpoint이며 fast-path 후보로
해석하지 않는다.

차원상 `theta`, `lambda`, `q`, normalized WRMS defect는 무차원이다. `h`는 원래 ODE의
시간 단위를 가지며, stage increment `k_i`와 output correction은 state `y`와 동일한 단위를
갖는다. Mass-matrix problem에서도 `W`와 target Jacobian의 각 block은 동일한 operator
차원을 갖도록 구성했다.

## 3. 구현한 실험층

### 3.1 Read-only nonlinear block decomposition

`StructuredBlockSystem`에 다음 read-only decomposition을 추가했다.

- `diagonal_apply(K)=DK`;
- `coupling_apply(K)=CK`;
- `partial_linear_apply(K,eta)=(D-eta C)K`;
- nonlinear remainder snapshot;
- original target residual;
- exact current target Jacobian.

Affine limit에서 target Jacobian이 기존 explicit structured block matrix와 일치하는지 직접
검사했다.

### 3.2 Fixed homotopy path

각 path point에서 Euler 또는 AB2 predictor와

\[
\sum_{m=0}^{q}Q^mD^{-1}
\]

형태의 truncated finite inverse action을 사용했다. `corrections_per_point`는 0과 1을 모두
screen했다. 여기서 `q<7`이면 `lambda=0`의 시작점에도 truncated inverse를 사용하므로 초기
homotopy residual이 일반적으로 0이 아니다. 따라서 이 lane은 exact-start classical continuation이
아니라 **fixed-schedule approximate causal propagation**이며, 모든 초기·중간 residual을 결과에
노출한다.

Work ledger는 최소한 다음을 분리한다.

\[
N_{\rm path},\quad N_{\rm correction},\quad N_{\rm snapshot},
\quad N_{C\text{-action}},\quad N_{W\text{-batch}},\quad N_{W\text{-vector}}.
\]

### 3.3 Exact nonlinear output certificate

Candidate stage vector에서 original RODAS target Jacobian

\[
\mathcal J_R(K)=D-C-hDN(K)
\]

을 정확히 조립·분해하고

\[
\mathcal J_R\delta K=-\mathcal R(K)
\]

을 풀었다. Output-direction correction은

\[
E_{\rm out}
=
\left\|
(b^T\otimes I)\delta K
\right\|_{\rm WRMS}
\]

로 계산한다. 이 solve는 fast implementation이 아니라 reference oracle이다.

### 3.4 Transactional fallback

Candidate가 finite하지 않거나 certificate budget을 통과하지 못하면 같은 timestep을
protected sequential RODAS5P로 다시 계산한다. 실패한 speculative path는 외부 Krylov state를
commit하지 않는다.

## 4. Canonical experiment matrix

Canonical screen은 다음 57개 one-step case를 사용했다.

| Family | Cases | 주요 변화축 |
|---|---:|---|
| affine noncommuting mass | 1 | exact affine oracle |
| scalar linear | 4 | \(\lambda=-10^2,-10^4\), \(h=10^{-3},10^{-2}\) |
| Prothero–Robinson | 12 | stiffness \(10^2,10^4,10^6\), nonlinearity 0/1000, two \(h\) |
| manufactured noncommuting mass | 16 | stiffness, cubic nonlinearity, nonnormality, \(h\) |
| manufactured vector | 24 | stiffness, cubic nonlinearity, nonnormality, \(h\) |

Candidate grid는

\[
\theta\in\{0,0.5,1\},
\quad q\in\{0,1,2,7\},
\]

\[
r\in\{2,3,4\},
\quad predictor\in\{Euler,AB2\},
\quad correction\in\{0,1\}
\]

으로 총 144개 config이다. 따라서 candidate row는

\[
57\times144=8208
\]

이다. 각 case에 protected sequential direct와 SABR nilpotent-3 control을 하나씩 두어 control
row는 114개다.

공통 tolerance는

\[
atol=10^{-7},
\qquad
rtol=10^{-6},
\qquad
E_{\rm out}\le0.1
\]

이다. `force_accept=true`로 timestep controller의 rejection을 제거해 stage algebra만 분리했다.
이 설정은 global integration acceptance policy가 아니다.

## 5. One-step safety와 acceptance

Canonical 결과:

```text
cases          57
controls       114
candidates     8208
fast accepts   3510
fallbacks      4698
failures       0
false accepts  0
```

Fast-accepted candidate의 sequential-oracle output WRMS 최댓값은

\[
0.0999910161
\]

으로 absolute normalized output-WRMS budget 0.1 아래였다. 반대로 oracle defect가 0.1 이하인데 certificate가 reject한 row도
0개였다. 이 결과는 현재 exact certificate가 이 finite one-step grid에서 acceptance boundary를
정확히 추적했음을 뜻한다.

다만 certificate-to-oracle ratio는 fast-accepted rows에서도

\[
1.99\times10^{-3}
\le
\frac{E_{\rm certificate}}{E_{\rm oracle}}
\le
1.218\times10^2
\]

로 넓다. Median은 거의 1이지만, one-step safety가 곧 tight bound나 global error control을
뜻하지 않는다.

## 6. \(q\), correction, \(\theta\)에 따른 결과

### 6.1 Correction이 없는 경우

| q | fast accept rate | median oracle WRMS | median W-batch depth |
|---:|---:|---:|---:|
| 0 | 8.19% | \(1.231\times10^3\) | 4 |
| 1 | 25.24% | 9.840 | 8 |
| 2 | 24.56% | 46.34 | 11 |
| 7 | 50.49% | \(7.835\times10^{-2}\) | 26 |

`q=0`은 batch depth가 가장 작지만 대부분 정확하지 않다. `q=1,2`는 acceptance를 늘리지만
median critical depth가 이미 sequential depth 8과 같거나 더 크다. `q=7`도 finite number of
path rounds 때문에 nonlinear severe cases에서 자동으로 exact endpoint가 되지는 않는다.

### 6.2 Point correction이 한 번인 경우

| q | fast accept rate | median oracle WRMS | median W-batch depth |
|---:|---:|---:|---:|
| 0 | 28.17% | 21.64 | 7 |
| 1 | 51.27% | \(6.288\times10^{-2}\) | 14 |
| 2 | 59.06% | \(3.644\times10^{-3}\) | 21 |
| 7 | 95.13% | \(1.260\times10^{-10}\) | 56 |

한 번의 correction은 정확도를 크게 개선하지만, `q=1,2,7`의 median W-batch depth를
sequential depth 8보다 크게 만든다. 이 결과는 “더 많이 수렴한다”와 “병렬 critical path가
짧다”를 분리해야 함을 보여준다.

### 6.3 \(\theta\) trade-off

`correction=1`에서 `theta=1,q=2`는 fast accept rate 78.95%, median oracle WRMS
\(6.80\times10^{-6}\)을 보였으나 median W-batch depth는 21이었다. Frozen linear coupling을
시작 문제에 포함하면 path가 정확해지지만 start solve와 propagation이 다시 무거워진다.

반대로 `theta=0`은 낮은 W-batch depth를 얻기 쉽지만, strong stiffness와 large step에서
정확도와 acceptance가 급격히 악화됐다.

## 7. Low-depth viability 영역

Low-depth를

\[
\text{fast accepted}
\quad\land\quad
N_{W\text{-batch}}<8
\]

로 정의하면 401개 row, 16개 case가 해당한다.

| Family | covered cases |
|---|---:|
| affine mass | 1 |
| manufactured mass | 4 |
| manufactured vector | 8 |
| Prothero–Robinson | 2 |
| scalar linear | 1 |

Low-depth rows의 median은

\[
N_{W\text{-batch}}=5,
\qquad
N_{W\text{-vector}}=40,
\qquad
N_{\rm snapshot}=5.
\]

하지만 covered cases는 주로 stiffness 100과 작은 \(h=10^{-3}\), 또는 mild/normal cases에
집중된다. Stiffness \(10^4\) 이상이나 large step, 강한 nonnormality에서 일반적인 low-depth
coverage는 관측되지 않았다.

## 8. Equal-work control 비교

Fast-accepted rows의 scalar work를 control과 비교했다.

Sequential 대비 median ratio:

\[
N_f:6.125,
\qquad
N_{Jv}:53.71,
\qquad
N_{\rm direct\ solve\ calls}:2.75.
\]

Low-depth subset에서도 sequential 대비 median은

\[
N_f:6.125,
\qquad
N_{Jv}:29.71,
\qquad
N_{\rm direct\ solve\ calls}:0.75.
\]

`direct_solve_calls`는 `solve_rows`로 8개 RHS를 푼 경우에도 한 번으로 센다. 반면 sequential
RODAS5P는 각 stage solve를 8회로 센다. 따라서 0.75는 scalar linear-solve work가 아니라 API
call-depth proxy다. Low-depth rows의 path ledger에서 common-`W` solve vector median은 40개로,
sequential의 8개보다 5배 많다. 즉 W-solve batch depth는 줄었지만 scalar nonlinear/JVP 및
solve-vector work가 크게 늘었다.

SABR 대비 low-depth median ratio는

\[
N_f:1.96,
\qquad
N_{Jv}:1.37,
\qquad
N_{\rm direct\ solve\ calls}:0.333.
\]

Batch call 수는 줄지만 RHS/JVP work가 더 많다. RHS와 JVP 두 scalar-work 지표만으로도 두
control을 동시에 넘지 않는 candidate는 sequential과 SABR 양쪽 모두 0개였다. Direct-call
counter까지 포함한 보수적 세 지표 판정도 0개다.

현재 Rust batch API는 동일 호출 안에서 여러 stage vector를 처리하지만 GPU/SIMD/thread의
실제 동시 실행을 증명하지 않는다. 따라서 scalar-work 증가를 “batch이므로 무료”라고 처리할
수 없다.

## 9. Global convergence-order screen

다음 exact manufactured problem을 \(T=0.2\)까지 고정 step으로 적분했다.

```text
n=6
stiffness=80
cubic nonlinearity=10
nonnormality=0
h=0.04, 0.02, 0.01, 0.005
```

### 9.1 Protected controls

Sequential RODAS5P의 관측 차수는 roundoff 이전에

\[
5.067,
\qquad
4.938
\]

이었다. `theta=1,q=7,correction=1`도

\[
5.067,
\qquad
4.936
\]

을 재현했다. `theta=1,q=2,correction=1`은

\[
4.963,
\qquad
4.847
\]

로 fifth-order trend를 유지했다. 그러나 마지막 두 homotopy controls는 sequential보다 RHS,
JVP, direct solve가 훨씬 많다.

### 9.2 Low-depth no-correction paths

`theta=0,q=0,r=2,AB2,c=0`은 fine steps에서 모두 fast accept됐지만 all-fast refinement
orders가

\[
1.095,
\qquad
1.450
\]

에 불과했다.

`q=1`은 finest all-fast interval에서 order 4.197을 보였지만 fifth order에 미달했다.

`q=2`는 한 interval에서 4.982를 보인 뒤 다음 refinement에서 0.695로 붕괴했다.

더 심각하게, coarse step에서 fallback하던 method가 fine step에서 fast path로 전환되는 순간
protected sequential error \(O(10^{-12})\)에서 homotopy error \(O(10^{-7})\)로 불연속적으로
악화됐다. 이는 fixed `E_out <= 0.1` rule이 local tolerance budget은 만족하지만 algebraic error가
step-size와 함께 충분히 빠르게 감소하도록 강제하지 않기 때문이다.

따라서 다음 cycle에서는 적어도

\[
E_{\rm out}
\le
\eta E_{\rm emb}
\]

또는 fifth-order local algebraic scaling에 해당하는 step-size-aware budget을 검증해야 한다.
이것은 adaptive path schedule을 의미하지 않으며, scientific acceptance policy의 수정 문제다.

## 10. Stiff/Prothero–Robinson screen

Prothero–Robinson에서 stiffness 100, \(h=10^{-3}\)이면 correction=1은 모든 72 config를
accept했다. 그러나 stiffness \(10^4\)와 \(10^6\)에서는 correction=1의 accept rate가 대체로
30–39%에 머물렀고 median oracle defect는 수십에서 수천까지 증가했다.

Scalar linear stiff screen에서도 `q=7,correction=1`만 72/72를 accept하며 sequential endpoint를
근사했다. Low-depth `q<=2`의 acceptance와 accuracy는 강한 stiffness에서 안정적으로 유지되지
않았다.

따라서 low-depth candidate의 stiff-decay gate는 **NOT PASSED**다. `q=7` 결과는 correctness
oracle이지 cheap stiff solver의 증거가 아니다.

## 11. Noncommuting mass와 nonnormal screen

새 manufactured mass problem은

\[
M=\begin{pmatrix}2&1\\0.3&1.5\end{pmatrix}
\]

과 noncommuting linear part를 사용하며 exact manufactured solution을 갖는다. Test에서

\[
\|MA-AM\|_2>10^{-2}
\]

와 exact RHS consistency를 확인했다.

Canonical grid는 mass/nonmass 양쪽에서 nonnormality 0.2와 0.9를 포함한다. Low-depth acceptance는
stiffness 100, 작은 step에 국한됐고 stiffness \(10^4\)에서는 일반화되지 않았다. 이 결과는
triangular nilpotency가 full physical operator의 conditioning을 자동으로 해결하지 않음을
확인한다.

## 12. Negative results와 해석

이번 cycle의 중요한 negative results는 다음이다.

1. `q<=2`와 적은 path rounds가 존재 가능한 regime은 확인했지만 범위가 좁다.
2. exact one-step certificate의 fixed threshold는 global order certificate가 아니다.
3. one correction은 정확도를 높이지만 critical depth와 scalar work를 크게 늘린다.
4. q7 exactness는 nonlinear finite-round efficiency를 뜻하지 않는다.
5. 현재 candidate는 equal-work SABR를 이기지 못한다.
6. 현재 batch interface는 실제 parallel execution을 보장하지 않는다.

따라서 이번 결과를 “homotopy 방법 실패”라고 일반화해서는 안 된다. 반대로 “낮은 batch depth가
관측됐으므로 acceleration 성공”이라고 주장해서도 안 된다. 살아남은 연구 질문은 더 좁다.

> Step-size/LTE-aware output budget과 선택적 low-depth schedule이 mild regime에서 global fifth
> order를 보존하면서, 실제 concurrent batch backend에서 scalar-work 증가를 amortize할 수 있는가?

## 13. 다음 cycle gate

다음 단계는 바로 broad tuning으로 가지 않고 다음 순서를 따른다.

1. **Acceptance-policy screen**
   - fixed absolute output-WRMS budget 0.1;
   - \(E_{out}\le\eta E_{emb}\);
   - step-size power-law budget;
   - 모두 exact certificate를 oracle로 사용.
2. **Order-preservation screen**
   - all-fast trajectory만 별도로 분류;
   - fallback transition과 method order를 섞지 않음.
3. **Stiff/nonnormal kill screen**
   - low-depth path가 strong stiffness에서 즉시 reject되는지 확인;
   - false acceptance 0 유지.
4. **Actual concurrent batch backend**
   - stage-batched RHS/JVP를 실제 thread/SIMD/GPU에서 측정하기 전 wall-speedup 주장 금지.
5. **그 이후에만** nonstationary schedule, Anderson, deferred GCRO-DR tuning을 연다.

## 14. Validity range

현재 결과가 직접 지지하는 범위는 다음이다.

- binary64;
- regular ODE와 nonsingular constant mass matrix;
- explicit dense Jacobian;
- 8-stage RODAS5P;
- Rust 1.94.1 / faer backend;
- deterministic fixed schedule;
- exact target-Jacobian certificate;
- single-threaded scalar work accounting.

다음은 검증되지 않았다.

- JVP-only cheap certificate;
- singular mass/index-1 DAE;
- sparse/MPI/GPU;
- adaptive theta/lambda/q;
- Anderson/Padé/Chebyshev;
- production wall-time speedup;
- BDF/Radau ranking.

## 15. Durable evidence

- canonical raw JSON: `results/HOMOTOPY_EXPERIMENT_SCREEN.json`;
- byte-identical rerun: `results/HOMOTOPY_EXPERIMENT_SCREEN_RERUN.json`;
- machine-readable analysis: `tables/HOMOTOPY_EXPERIMENT_ANALYSIS.json`;
- configuration, family, case, low-depth, order, work tables under `tables/`;
- stdlib-only analysis script: `analysis/analyze_screen.py`.

Canonical JSON SHA-256:

```text
a665562e3088950e3a7d64e985f220708cac6cad7356f096baf378e4ee27026e
```
