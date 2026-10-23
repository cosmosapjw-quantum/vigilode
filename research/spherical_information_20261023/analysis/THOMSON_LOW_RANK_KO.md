# 구면 충돌 연산: 기존 rank-6 구현의 정확한 시간 적분과 Fisher 해석

이 문서의 새 구현은 고정 구면 격자에서 충돌 부분흐름을 유한 개의 모멘트로 정확하게 적분한다. 기존 VigilODE에는 이미 scalar rank-6 Thomson 우변이 있으므로, rank-6 자체를 새 발견이나 현재 코드 대비 $N^2\to N$ 개선으로 주장하지 않는다. 새로운 연결은 **기존 유한체적 연산자에 맞는 작은 행렬 지수함수**, **편광 coherency의 양의 준정부호 구조**, **등방 기준 Fisher 계량에서의 모드별 정확 감쇠**다.

## 1. 사용한 기존 결과와 범위

첨부 Paper II `sections/03_nested_information.tex`, `eq:kinetic-bregman`은 $h''_\xi(g)=1/[g(1+\xi g)]$인 occupation entropy의 국소 계량을 준다. `sections/06b_quantitative_resolution.tex`, `eq:massive-fisher-block`은 등방 기준에서 각 모드가 분리되는 원리를 설명한다. 아래에서는 이 원리를 photon의 한 에너지 shell, cold electron rest frame의 충돌 연산에 적용한다.

기존 코드 기준은 `cosmosapjw-quantum/vigilode`, commit `14c74978b93339a79d7228a324fa77552dae548e`다.

- `crates/rodas5p-integrators/examples/sphere_photon_fv_support/mod.rs`, `thomson_group`는 이미 monopole + 다섯 quadrupole 누적을 사용한다.
- `research/bassrei_p06_full_sphere_scoped_20261022/FULL_SPHERE_FVM_REFERENCE.py`, `thomson6_factors`는 유한체적 gain의 $UV^{\mathsf T}$ 인수를 제공한다. 이 파일의 SHA-256은 `773552acc6c42de5f65972453a32e9f9dff7ac96bde33b358b05e18edd8de891`이다.
- 같은 파일의 `polarized_thomson_iq_rhs`, `polarized_thomson_lr_rhs`는 기존 축대칭 $I,Q$ 편광 우변이다. 기존 정의와 결과를 재사용하고 full suite를 다시 실행하지 않았다.

물리적 충돌식의 원전은 Portsmouth–Bertschinger, arXiv:astro-ph/0412094, 식 (160), (168)이다. 원전의 자연단위 rate를 여기서는 $\kappa=n_e\sigma_T c$로 복원한다. $\tau=\int\kappa\,dt$는 무차원 optical depth다. 고정 tetrad/electron rest frame, elastic Thomson limit, recoil 및 열적 재분배가 없는 한 shell을 가정한다. 실제 Bianchi streaming, 에너지 적색편이, electron tilt가 결합되면 분할 오차 또는 결합 적분이 따로 필요하다.

## 2. scalar 모형: 정확한 rank-6 부분흐름

먼저 편광을 매 산란마다 제거하는 intensity-only 모형을 구분한다. $d\omega=d\Omega/(4\pi)$, $\int d\omega=1$로 쓰면

$$S f(n)=\int\frac34[1+(n\cdot n')^2]f(n')\,d\omega',\qquad C=\kappa(S-I).$$

$Q_a$, $a=1,\dots,5$를 $d\omega$에 대해 정규직교인 real quadrupoles로 잡는다. 덧셈 정리로 $S=P_0+\frac1{10}P_2$다. 따라서

$$e^{\tau(S-I)}f=P_0 f+e^{-9\tau/10}P_2 f+e^{-\tau}(I-P_0-P_2)f.\tag{T1}$$

일곱 단계 RHS 같은 반복을 거치지 않고 monopole/다섯 quadrupoles 한 번과 점별 재구성으로 계산한다. 양의 quadrature가 모든 차수 4 이하 다항식을 정확히 적분하면 이 이산식도 정확하다. 데모는 $N_\mu\ge3$, $N_\phi\ge5$인 Gauss–Legendre $\mu$와 균일 $\phi$ rule을 사용한다. 이 요건은 **cell average FVM basis에 자동 적용되지 않는다**.

$S$는 양의 Markov kernel이며 $S1=1$, $\int Sf=\int f$다. 따라서

$$e^{\tau(S-I)}=e^{-\tau}\sum_{j=0}^\infty\frac{\tau^j}{j!}S^j$$

는 모든 $\tau\ge0$에서 양성성과 질량을 보존한다. explicit Euler의 positivity 제한 $\kappa h\le1$이 충돌 부분흐름에서는 사라진다. 다만 비등방 unpolarized photon은 실제 산란 후 선형 편광을 생성하므로 (T1)을 반복한 결과는 일반적인 polarized Thomson solution이 아니다.

## 3. 실수 transverse coherency: 편광을 유지하는 정확 부분흐름

$F(n)$를 실수 대칭 $3\times3$ 행렬장으로 두고

$$F(n)\succeq0,\quad F(n)n=0,\quad P_n=I-nn^{\mathsf T}$$

를 가정한다. 이 표현은 Stokes $I,Q,U$를 포함하고 circular $V$는 0이다. intensity는 $\operatorname{tr}F$다. 전자 정지계의 gain은

$$S F(n)=\frac32P_n T P_n,\qquad T=\int F(n')d\omega'.\tag{T2}$$

$U(T)(n)=\tfrac32P_nTP_n$, $V(F)=\int Fd\omega$라 쓰면 $S=UV$다. 필요한 전역 자료는 대칭 $T$의 여섯 독립 성분뿐이다.

### 3.1 여섯 차원 적분 지도

등방 구면 적분

$$\langle n_i n_j\rangle=\delta_{ij}/3,\qquad
\langle n_i n_j n_k n_l\rangle=(\delta_{ij}\delta_{kl}+\delta_{ik}\delta_{jl}+\delta_{il}\delta_{jk})/15$$

를 대입한다. 대칭 $T$에 대해

$$\begin{aligned}
\langle PTP\rangle
&=T-\tfrac23T+\tfrac1{15}(2T+\operatorname{tr}T\,I)\\
&=\tfrac7{15}T+\tfrac1{15}\operatorname{tr}T\,I.
\end{aligned}$$

그러므로 $B=VU$는

$$B(T)=\frac7{10}T+\frac1{10}\operatorname{tr}T\,I.\tag{T3}$$

$T_0=\operatorname{tr}T\,I/3$, $T_2=T-T_0$로 분해하면 $BT_0=T_0$, $BT_2=(7/10)T_2$다. $\Pi_0,\Pi_2$를 이 두 tensor 부분공간의 사영으로 정의하고

$$R_0=U\Pi_0V,\qquad R_2=\frac{10}{7}U\Pi_2V$$

로 놓으면 $R_0^2=R_0$, $R_2^2=R_2$, $R_0R_2=0$이다. $S$의 eigenvalues는 $1,7/10,0$이므로

$$F(\tau)=R_0F_0+e^{-3\tau/10}R_2F_0+e^{-\tau}(I-R_0-R_2)F_0.\tag{T4}$$

편광 모형의 $3/10$ 감쇠율을 scalar 모형의 $9/10$과 혼동하면 안 된다. 같은 degree-four quadrature 조건에서 (T2)–(T4)는 이산 collocation 연산자에도 성립한다.

### 3.2 양의 표현과 실제 계산식

(T4)는 다음처럼 평가할 수 있다.

$$\begin{aligned}
a_0&=1-e^{-\tau},&a_2&=\frac{e^{-3\tau/10}-e^{-\tau}}{7/10},\\
F(\tau)&=e^{-\tau}F_0+U\left[a_2T+(a_0-a_2)T_0\right].\tag{T5}
\end{aligned}$$

$a_2\ge0$이고 $a_0-a_2\ge0$이다. 후자는 $x=e^{-\tau}\in[0,1]$에 대해 $x^{3/10}\le7/10+(3/10)x$를 적용하면 바로 따른다. $T,T_0\succeq0$이므로 (T5)는 양의 행렬들의 합이며 PSD와 transversality를 보존한다. 질량은 $\int\operatorname{tr}S F=\operatorname{tr}BT=\operatorname{tr}T$로 보존된다.

코드는 `expm1`으로 $a_0,a_2$를 계산한다. $a_0-a_2=O(\tau^2)$의 차감 소실을 피하려고 $\tau<10^{-3}$에서는

$$a_0-a_2=\sum_{k=2}^{12}\frac{(-1)^k[(3/10)-(3/10)^k]}{(7/10)k!}\tau^k+O(\tau^{13})$$

를 사용한다. 시험 범위에서는 나머지가 double roundoff보다 충분히 작다. 일반 PSD 보존 정리는 exact arithmetic 정리이며, 계산한 $3\times3$ 행렬의 영 longitudinal eigenvalue에는 $10^{-15}$ 수준 roundoff가 나타났다. Stokes 성분별 clipping은 적용하지 않는다.

## 4. Fisher 계량에서 정확하게 사라지는 성분

등방 $F_*=gP_n$, $g>0$ 근처에서 $\operatorname{tr}h_\xi(F)$의 이차 변화는 physical transverse plane에 한정하면

$$D^{(2)}(\delta F)=\frac1{2g(1+\xi g)}\int\operatorname{tr}[(\delta F)^2]d\omega.\tag{T6}$$

FD에서는 $0<g<1$, BE에서는 $g>0$, MB에서는 denominator의 $(1+\xi g)$ 대신 1을 사용한다. 이 식은 scalar occupation의 Hessian을 isotropic matrix reference의 두 고유값에 적용한 직접 유도다. anisotropic matrix reference의 full noncommutative Hessian으로 확장했다는 주장은 하지 않는다.

transverse $A,B$에 대해

$$\langle A,SB\rangle=\tfrac32\operatorname{tr}[(VA)(VB)]=\langle SA,B\rangle$$

이므로 $R_0,R_2,I-R_0-R_2$는 weighted Frobenius/Fisher inner product에서 정규직교 사영이다. 따라서

$$D^{(2)}(\tau)=D^{(2)}(R_0\delta F_0)+e^{-3\tau/5}D^{(2)}(R_2\delta F_0)+e^{-2\tau}D^{(2)}((I-R_0-R_2)\delta F_0).\tag{T7}$$

total intensity가 같은 perturbation이면 $R_0\delta F_0=0$이며 $D^{(2)}(\tau)\le e^{-3\tau/5}D^{(2)}(0)$. 충돌 중 고차 잔여를 제거하려면 그 정보 손실을 $e^{-2\tau}$ 가중치와 함께 명시할 수 있다. streaming이 새 고차 모드를 생성하면 이후 재계산해야 한다.

데모는 이차 근사를 넘어 MB matrix divergence $\langle\operatorname{tr}[F\log(F/g)-F+gP]\rangle$도 측정했다. 임의 anisotropy에 대한 이 값은 별도의 직접 수치 검사다. source-free collisional subflow의 unital, trace-preserving completely-positive 성질과 matrix convexity로도 단조성이 이어지지만, 이번 코드의 BE/FD nonlinear matrix entropy 검증으로 확대하지 않는다.

## 5. 기존 finite-volume 격자: 작은 행렬 지수함수가 정확한 이식 경로

실제 저장소는 cell contents $N_c$와 cell-average quadrupoles를 사용한다. 고정 격자에서

$$\dot y=\kappa(t)(UV^{\mathsf T}-I)y,\quad U,V\in\mathbb R^{N\times r},\quad r\le6$$

라고 쓴다. $B=V^{\mathsf T}U$는 일반적으로 $\operatorname{diag}(1,0.1,\ldots,0.1)$와 같지 않다. 계수의 $0.1$은 continuum/collocation 정규직교 결과이며 finite cell average의 Gram matrix를 대신하지 못한다.

고정 $U,V$에 대해서는 $(UV^{\mathsf T})^k=UB^{k-1}V^{\mathsf T}$, $k\ge1$이므로 직접 멱급수를 합하여

$$e^{\tau(UV^{\mathsf T}-I)}y=e^{-\tau}y+U\,g_\tau(B)V^{\mathsf T}y,\quad
 g_\tau(B)=e^{-\tau}\tau\varphi_1(\tau B),\quad\varphi_1(z)=\frac{e^z-1}{z}.\tag{T8}$$

$\varphi_1$은 $z=0$에서 연속적으로 1이며, $B$가 singular여도 성립한다. 이는 고정 grid에서 시간 가변 rate에도 정확하며 필요한 시간 정보는 $\tau=\int\kappa dt$다. $\tau$를 근사 quadrature로 계산했다면 그 오차는 별도 시간 오차다.

### 5.1 overflow 및 rank deficiency 회피

$e^{-\tau}\varphi_1(\tau B)$를 직접 두 인자로 계산하면 $\tau\gg1$에서 overflow가 생길 수 있다. 다음 $2r\times2r$ exponential의 우상단을 사용한다.

$$\exp\left[\tau\begin{pmatrix}B-I&I\\0&-I\end{pmatrix}\right]
=\begin{pmatrix}e^{\tau(B-I)}&g_\tau(B)\\0&e^{-\tau}I\end{pmatrix}.\tag{T9}$$

미분하여 우상단의 Duhamel 적분을 확인하면 (T8)과 같다. $B^{-1}$ 또는 inverse eigenvalue가 필요 없다. current Python prototype은 (T9)를 사용한다. 대칭 유사화가 가능한 기존 scalar Galerkin에서는 작은 Gram matrix의 `eigh`를 캐시하고

$$g_\tau(\lambda)=e^{-(1-\lambda)\tau}\frac{-\operatorname{expm1}(-\lambda\tau)}{\lambda},\quad g_\tau(0)=\tau e^{-\tau}$$

로 계산하는 것이 Rust 후속 구현의 유력 경로다. roundoff로 eigenvalue가 이론 범위를 벗어났을 때는 그 크기를 기록하고 원래 quadrature 계약에 따라 처리해야 한다.

### 5.2 비용과 보장되는 개선 범위

작은 대칭 행렬의 고유분해를 캐시한 fixed-grid 버전의 setup은 $O(Nr^2+r^3)$, step action은 $O(Nr+r^2)$, 저장은 $O(Nr+r^2)$다. 현재 범용 Python 데모의 (T9)는 매 호출마다 $2r\times2r$ exponential을 계산하므로 step 비용은 $O(Nr^2+r^3)$이며, $B$ 구성과 분해 캐시는 Rust 이식 단계의 구체적 개선이다. 현재 scalar RHS도 이미 $O(Nr)$이므로 공간 복잡도 개선을 중복 주장할 수 없다. 보장되는 변화는 **frozen collision subproblem을 한 번의 action으로 풀며 collision-induced explicit stability restriction을 제거**한다는 것이다. 표준 충분조건 $\kappa h\le1$을 사용하는 양성 Euler에서는 순수 collision 구간에 $J\ge\lceil\tau\rceil$ 단계가 필요하지만 (T8)는 한 action이다. 특정 유한 격자에서는 gain의 양의 대각 성분 때문에 더 큰 Euler 스텝도 양성성을 유지할 수 있으므로, 이 단계 수는 그 표준 충분조건을 채택한 방법에 대한 비교다. streaming과의 Strang splitting에서는 일반적으로 global order 2 및 비가환 commutator에 따른 오차가 남으므로, 작은 collision solve의 정확성만으로 전체 solve의 시간 차수를 선언하지 않는다.

## 6. 작은 실제 검증

명령:

```bash
python3 src/thomson_demo.py
# 선택: 현재 checkout의 oracle 바이트도 대조할 때
python3 src/thomson_demo.py --repo /path/to/vigilode
```

`--repo`를 생략하면 패키지에 함께 보존한 `reference/FULL_SPHERE_FVM_REFERENCE.py`를 사용하므로 외부 checkout 없이 재현할 수 있다. origin JSON에 원래 commit·경로·SHA-256을 기록했고, 원본은 `git show`와 바이트 단위로 대조했다. `--repo`를 지정하면 실제 HEAD와 source hash를 기록하며, frozen hash가 다르면 과학 비교를 실행하지 않고 `unmatched`로 분류한다. `--oracle PATH`로 명시한 파일도 같은 hash 검사를 받는다. 최초 과학 실행은 위 commit의 실제 checkout을 사용했다. NumPy 2.3.5, SciPy 1.17.0, BLAS thread limit 1, seed 20261023이다. 결과는 `results/thomson_results.json`과 `thomson_run.stdout`에 있다.

- $3\times6=18$ directions, polarized physical dimension 54에서 독립 dense `expm`과 비교했다. $\tau=0,10^{-10},0.1,1,20,1000$에서 최대 절대 차이 $3.89\times10^{-13}$ 미만, normalized difference $1.66\times10^{-13}$ 미만이다. 큰 $\tau$ dense reference의 누적 roundoff도 포함한 비교다.
- gain eigenvalues $1,(0.7)^{\times5},0$의 최대 차이 $3.34\times10^{-16}$; integrated map (T3)와 실제 quadrature를 별도로 대조했다.
- 18,128,512 directions의 mixed/near-pure beam에 대해 24개의 PSD·mass·entropy 검사 모두 통과했다. 정확한 수치는 JSON에 보존했다.
- 기존 FVM oracle에서 irregular 25-cell 및 rank-deficient 2-cell을 사용했다. (T8)–(T9)와 dense exponential의 최대 차이는 $8.11\times10^{-15}$ 이하. 기존 RHS와 factors의 비교도 함께 기록했다.
- irregular grid의 gain eigenvalues는 약 $1,0.083575,0.077077,0.069082,0.065127,0.023533$이다. continuum $0.1$ 값을 잘못 대입하면 같은 입력에서 $2.44\times10^{-6}$까지 오류가 났다. 이는 finite-volume 이식에 작은 실제 Gram matrix가 필요하다는 직접 증거다.
- 초기 $I=1+0.6P_2$, $Q=U=0$, $\tau=1$의 편광 예에서 생성된 최대 선형 편광은 약 $0.04795$, scalar 반복과의 intensity 차이는 약 $0.004376$이었다. 이것이 scalar closure와 편광 subflow를 분리한 이유다.

scalar dense matvec와 rank-6 matvec timing도 128,512,2048 directions에서 측정했다. 매우 작은 문제의 결과는 Python overhead와 동시 샌드박스 부하에 민감하며, 작은 크기에서는 dense가 더 빠른 실행도 있었다. 여러 개발 실행의 원자료가 `thomson_v1_pre_fvm.json`, `thomson_v2_fvm_signed_form.json`에 남아 있다. **이 timing은 기존 rank-6 VigilODE의 속도 향상 근거가 아니다.** 현재 구현 대비 속도는 충돌 time-step 절감과 full coupled problem의 같은 오류 상한에서 다음 단계에 측정해야 한다.

## 7. 바로 이식할 순서

1. `thomson6_factors`의 현재 normalization으로 grid별 $B$ 또는 대칭 Gram matrix를 한 번 구성하고, 소형 eigen/Schur decomposition을 캐시한다. 격자 변경 시 캐시를 폐기한다.
2. frozen collision을 처리하는 `apply_collision_exact(tau, input, output, workspace)`를 추가한다. 위 (T8)와 안정한 scalar $g_\tau$를 사용한다. 0/rank-deficient modes를 별도 inverse 없이 처리한다.
3. transport와의 결합은 우선 second-order split reference와 fully coupled 기존 경로를 동일 관측량 오차에서 비교한다. 최종 production 방법은 관측한 splitting-error cost로 결정한다.
4. 편광 lane에는 (T2)의 basis-free tensor prototype을 사용하여 full-sphere collision을 검증하고, grid remap은 공통 tensor basis에서 total coherency와 PSD를 확인한다. moving tetrad/screen transport의 별도 회전과 spin convention을 보존한다.
5. adaptive coarsening budget에는 (T7)의 충돌 정보 감쇠를 적용하되, 다음 transport 구간이 생성하는 unresolved modes를 별도의 residual estimator로 제한한다.

현재 상태: 정리 (T1)–(T9)는 명시한 가정 아래 `derived`; Python 구현과 finite-grid 대조는 `implementation-verified` 및 `numerically checked`; 실제 VigilODE Rust 부분흐름과 coupled Bianchi trajectory의 가속은 `unresolved` 후속 이식 항목이다.

### 독립 실행 패키지 보완 기록

`results/thomson_before_portable_loader.json` 및 `evidence/thomson_demo_pre_portable_loader.py`는 loader 변경 직전의 과학 결과와 정확한 실행 소스를 보존한다. 변경 뒤에는 `python3 src/thomson_demo.py --fvm-only`로 bundled oracle 경로를 시험했고 `results/thomson_loader_results.json`에 실제 결과를 기록했다. `src/check_thomson_portability.py`는 12개 수학 kernel 함수의 AST가 이전 소스와 동일함, 잘못된 oracle hash가 import 전에 거절됨, bundled origin과 실제 checkout provenance가 혼동되지 않음을 검사한다. 따라서 변경 의존성이 없는 기존 편광·timing 실험을 다시 실행하지 않았다.
