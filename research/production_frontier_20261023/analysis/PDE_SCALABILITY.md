# 보존형 구면·2D·3D 확산: 계산량을 줄이는 조건과 구현

## 연구 질문과 판정

기존 구면 정보기하 연구는 격자 압축, 모멘트 역산, 저랭크 충돌을 다뤘다. 그 결과를 시간 의존 PDE 전체의 가속으로 해석하지 않고, 이번에는 공간 이산화와 시간 오차를 분리한 보존형 확산 문제를 실제로 구성했다. 서로 다른 세 후보인 양성 Euler, 정규화 Poisson uniformization, 2차 Legendre 안정화 다항식을 비교했다. 구면은 극점에서 셀 면적이 작아지는 실제 가중치를 사용하며, 2D·3D는 주기 경계의 정방 격자다.

선택은 하나로 수렴하지 않는다. **양성성이 필수이면 uniformization**, **자기수반 확산·매끄러운 해에서 적은 stencil 연산이 우선이면 RKL2**를 후속 구현 대상으로 유지한다. Euler는 단순한 대조군과 작은 시간 간격에서의 대체 방법으로 유지한다. 검증되지 않은 일반 비선형 PDE나 전체 VigilODE production 경로로 승격하지 않는다.

## 1. 이산 연산자와 기하

셀 평균을 열벡터 $u$로, 정확한 셀 부피·면적을 $w_i>0$로 둔다. 무방향 edge마다 대칭 전도도 $c_{ij}=c_{ji}\ge0$를 부여한다.

$$
(Qu)_i=\frac1{w_i}\sum_j c_{ij}(u_j-u_i),\qquad
Q=-W^{-1}D^TCD,\quad W=\operatorname{diag}(w_i).
$$

따라서 $Q\mathbf1=0$, $w^TQ=0$이고

$$u^TWQu=-\sum_{\{i,j\}}c_{ij}(u_j-u_i)^2\le0.$$

$W^{1/2}QW^{-1/2}$는 대칭 음의 준정부호다. $q=\max_i\sum_j c_{ij}/w_i$이면 Gershgorin으로 $\sigma(-Q)\subset[0,2q]$다. 이하 $\Lambda=2q$를 사용한다. 이는 Jacobian을 추정한 것이 아니라 선언된 보존형 공간 연산자의 기하 계수에서 계산한 상한이다.

2D·3D 단위 주기 상자에서는 $w_i=\Delta x^d$, $c_{ij}=D\Delta x^{d-2}$, $D=0.1$이다. 주기 Fourier mode의 정확한 **이산** 감쇠율은 $-4Dn^2\sum_k\sin^2(\pi k/n)$이며 연속 PDE의 $-4\pi^2D\sum k^2$와 분리한다.

구면은 $\mu=-\cos\theta$, $d\Omega=d\mu\,d\phi$를 사용한다. $\theta$ 등간격 face, $\mu$ 중점 cell center, 주기 $\phi$에 대해

$$
w_{ij}=\Delta\mu_i\Delta\phi,\quad
c^{\mu}_{i+1/2,j}=\frac{(1-\mu_{i+1/2}^2)\Delta\phi}{\mu_{i+1}-\mu_i},\quad
c^\phi_{i,j+1/2}=\frac{\Delta\mu_i}{(1-\mu_i^2)\Delta\phi}.
$$

극점 face의 $1-\mu^2=0$으로 축 방향 경계 flux가 0이며 극점을 별도 중복 node로 만들지 않는다. 면적 합은 $4\pi$다. $u=\mu$는 이 격자에서도 정확히 $Q\mu=-2\mu$를 만족한다. 반면 $x=\sqrt{1-\mu^2}\cos\phi$는 정확한 이산 고유벡터가 아니다. 후자는 위도 방향의 작은 대칭 고유값 문제로 독립 이산 기준해를 만들고, $e^{-2t}x$의 정확한 cell average와 비교해 공간 오차를 따로 측정했다.

## 2. 후보 A: Euler의 명확한 기준 비용

$q=0$이면 $Q=0$이고 수학적으로 정확한 흐름은 항등 연산이다. 이하 $q>0$에서 $P=I+Q/q$는 행 확률행렬이며 $w^TP=w^T$다. $h q\le1$일 때

$$u_{n+1}=(1-hq)u_n+hq Pu_n$$

은 양성성, 가중 질량, weighted $L^1,L^2$ 및 sup norm 수축을 보존한다. 코드도 대각에서 큰 값을 빼는 대신 이 convex 조합을 계산한다.

고정된 $Q$에서 양성 시간 간격을 사용하면 $T=Mh$에 대해

$$\|u_M-e^{TQ}u_0\|_W\le \tfrac12 T h\,\|Q^2u_0\|_W.$$

이는 각 고유값에서 $|1-x-e^{-x}|\le x^2/2$와 수축 연산자의 telescope를 적용한 결과다. Euler는 작은 $h$에서는 매우 저렴하지만, 정확도 또는 CFL이 요구하는 반복 수가 곧 비용이다.

## 3. 후보 B: 양성·질량 보존 Poisson 다항식

$\mu=qh$이면

$$e^{hQ}=e^{-\mu}\sum_{k=0}^\infty\frac{\mu^k}{k!}P^k.$$

$S_K=\sum_{k=0}^Kp_k$, $p_k=e^{-\mu}\mu^k/k!$라 두고

$$\widetilde E_K=S_K^{-1}\sum_{k=0}^Kp_kP^k$$

를 계산한다. 단순 truncation은 질량을 $S_K$배로 줄인다. **계수 합으로 나누는 정규화**는 양성성과 질량 보존을 동시에 복구한다. 이는 음수 출력을 잘라내는 사후 clipping과 다르다.

### 오차 정리

$\tau=1-S_K$이면 모든 Markov 수축 norm에서

$$\|\widetilde E_K-e^{hQ}\|\le2\tau.$$

증명은 $A=\sum_{k\le K}p_kP^k$, $B=\sum_{k>K}p_kP^k$로 놓고 $\|A/S_K-A\|\le\tau$, $\|B\|\le\tau$를 합하면 끝난다. 여러 구간에서도 각 연산자가 수축하므로 오차 상한은 $2\sum_j\tau_j\,\|u_0\|$다.

코드는 $p_{K+1}$ 다음 항의 비 $r=\mu/(K+2)<1$일 때

$$\tau\le\frac{p_{K+1}}{1-r}$$

를 사용해 종료한다. $e^{-\mu}$ underflow를 피하려고 $\mu\le32$로 분할한다. 따라서 이 구현의 전체 stencil 비용 상한은

$$O\!\left(|E|(1+Tq)\left[1+\log\frac{1+Tq}{\varepsilon}\right]\right)$$

이다. 분할하지 않는 최적 Poisson quantile 알고리즘의 비용과 동일하다고 주장하지 않는다. 경직성이 매우 강하면 $Tq$에 비례하는 부분이 여전히 병목이다. 상한은 **실수 연산의 truncation**에 대한 것이며 floating-point roundoff를 포함하는 interval certificate가 아니다.

## 4. 후보 C: Legendre 안정화로 CFL 연산 수를 제곱근 수준으로

$s\ge2$, $K_s=s^2+s-2$에 대해

$$R_s(z)=a_s+b_s P_s(1+\omega_s z),\qquad
b_s=\frac{K_s}{2s(s+1)},\quad a_s=1-b_s,\quad \omega_s=\frac4{K_s}.$$

여기서 $P_s$는 Legendre 다항식이다. $P_s'(1)=s(s+1)/2$, $P_s''(1)=(s-1)s(s+1)(s+2)/8$이므로

$$R_s(0)=1,\quad R_s'(0)=1,\quad R_s''(0)=1.$$

즉 고정 선형 연산자에 대해 2차 방법이다. $h\Lambda\le K_s/2$이면 $1+\omega_s h\lambda\in[-1,1]$이며 $|P_s(x)|\le1$에서

$$0\le1-2b_s\le R_s(h\lambda)\le1.$$

따라서 weighted $L^2$ 수축과 질량 보존이 성립한다. 최소 stage 수는

$$s=\max\left(2,\left\lceil\frac{-1+\sqrt{9+8h\Lambda}}2\right\rceil\right),$$

즉 $O(\sqrt{h\Lambda})$다. 세 항 Legendre recurrence는 $s$회의 edge 연산과 $O(N)$ workspace만 사용한다.

### 정확도까지 포함하는 조건부 비용 보장

$P_n'=\sum_{k=n-1,n-3,\ldots\ge0}(2k+1)P_k$를 반복 적용하면 $P_s^{(3)}$도 Legendre 다항식의 비음수 계수 합이다. $|P_k(x)|\le1$과 $P_k(1)=1$을 적용하여 $[-1,1]$에서 $|P_s^{(3)}(x)|\le P_s^{(3)}(1)$를 얻는다. 따라서

$$\sup_{-K_s/2\le z\le0}|R_s'''(z)|\le\frac23.$$

Taylor의 적분 나머지로 $|R_s(-x)-e^{-x}|\le(1/9+1/6)x^3=5x^3/18$다. 자기수반 functional calculus, $Q$와의 가환성, 수축 telescope를 합하면

$$\boxed{\|R_s(hQ)^Mu_0-e^{TQ}u_0\|_W\le\frac5{18}T h^2\|Q^3u_0\|_W.}$$

이 정리는 최악의 $\Lambda^3\|u_0\|$ 대신 실제 초기 자료의 세 번 작용 $\|Q^3u_0\|$를 사용한다. 매끄러운 자료와 일관된 격자에서 이 양이 균일하게 제어되어 정확도가 요구하는 $h$가 격자 간격과 무관하게 유지되면, 총 비용은 $O(|E|T/h+|E|T\sqrt{\Lambda/h})$다. $\Lambda=O(\Delta x^{-2})$인 정방 격자에서는 CFL만으로 $O(|E|T\Delta x^{-2})$가 필요한 Euler보다 한 차수 낮은 $O(|E|T\Delta x^{-1})$의 stage 증가를 얻는다. 불연속 자료, 비정규 advection, 시간·상태 의존 계수에는 이 가정을 그대로 적용할 수 없다.

### 양성성은 별도 조건이다

스펙트럼에서 $R_s(h\lambda)\ge0$인 것과 행렬의 각 entry가 양수인 것은 다르다. 2D 주기 격자에서 $s=3$, $hq=2.5$, $P$를 네 이웃 평균으로 두면

$$R_3(hQ)=\frac7{12}I-\frac58P+\frac{25}{24}P^3.$$

점원천의 바로 옆 셀 계수는 $-5/512$다. 실제 Python과 Rust 계산은 $-0.009765625$를 재현했고, 별도 감사자의 정확 유리수 계산도 같은 값을 확인했다. 그러므로 이 recurrence를 **일반 양성 PDE 기본 경로로 채택하는 후보는 제외**한다. 이는 여기서 명시한 다차원 graph·stage 선택에 대한 반례이며, 기존 문헌의 다른 경계·시간 제한·비선형 조건을 검토하지 않고 문헌 전체가 틀렸다고 주장하는 것이 아니다.

## 5. 실행 결과와 동등 오차 비교

`PDE_CAMPAIGN.json`에는 166개의 원시 계산 행, 9개 공간 refinement 행, 81개 오차 기준별 선택 기록이 있다. Python 검사는 10개, native Rust 검사는 9개가 통과했다. 시간 오차는 동일한 weighted 상대 $L^2$ 기준 $10^{-5}$ 이내인 **실제로 시험한 설정 중 최소 edge 연산 수**로 비교했다. 각 방법의 달성 오차는 서로 다르며 oracle는 offline 비교·설정 선택에만 사용했다. 실제 실행 solver에 기준해를 제공하지 않는다.

| 격자 | Euler 작용 수 | RKL2 작용 수 | 양성 Poisson 작용 수 |
|---|---:|---:|---:|
| 2D 8² | 1536 | 64 | 11 |
| 2D 16² | 1408 | 64 | 25 |
| 2D 32² | 2624 | 64 | 82 |
| 3D 4³ | 2048 | 64 | 7 |
| 3D 8³ | 2048 | 64 | 14 |
| 3D 12³ | 2304 | 64 | 22 |
| 구면 6×12 | 224 | 12 | 18 |
| 구면 10×20 | 352 | 28 | 86 |
| 구면 16×32 | 277 | 68 | 522 |

![보존형 확산의 작용 수와 독립 공간 오차](../figures/pde_scalability.png)

위쪽은 $T=0.1$에서 동일한 상대 weighted $L^2$ 시간 오차 한계 아래의 작용 수이고, 아래쪽은 별도의 연속 PDE 기준과의 오차다. 주기 상자는 cell center에서 표본화한 Fourier product를 사용한다. 구면 위쪽은 축대칭 $\mu$ mode, 아래쪽은 비축대칭 $x=\sqrt{1-\mu^2}\cos\phi$ mode이므로 서로 다른 시험이다.

작용 수는 wall-time과 같지 않다. Euler와 uniformization은 convex Markov 작용, RKL2는 flux 작용과 추가 vector recurrence를 사용한다. 작은 smooth mode에서 uniformization이 가장 적은 작용으로 끝나기도 하고, 큰 구면 격자에서는 극점의 큰 $q$ 때문에 RKL2보다 많은 작용을 요구한다. 전체 campaign의 최대 상대 질량 drift는 약 $7.95\times10^{-14}$다.

주기 상자의 공간 refinement는 동일한 Fourier mode의 **cell-center 표본값**을 이산 감쇠율과 연속 감쇠율로 각각 진화시켜 비교했다. 이는 이산 Laplacian의 Fourier symbol 오차를 측정한다. 고정된 연속 초기 함수의 정확한 cell average에 필요한 $[\sin(\pi/n)/(\pi/n)]^d$ 인자는 코드에 없으므로, 초기 cell-average projection 및 출력 재구성 오차까지 포함한 전체 FVM 오차라고 부르지 않는다. 2D 8²→16²→32²의 상대 공간 오차는 $1.84\times10^{-3}\to4.60\times10^{-4}\to1.15\times10^{-4}$로 약 2차다. 3D 4³→8³→12³에서도 $5.44\times10^{-3}\to1.33\times10^{-3}\to5.88\times10^{-4}$로 감소했다.

구면의 비축대칭 mode에서는 실제로 $\mu$ 및 $\phi$ 적분으로 계산한 정확한 cell average를 초기값과 연속 기준해에 사용했다. 이 오차는 $3.16\times10^{-3}\to1.88\times10^{-3}\to1.06\times10^{-3}$로 더 천천히 감소했다. 같은 계산의 시간 오차는 $1.20\times10^{-13}$에서 $5.21\times10^{-13}$ 수준이므로 시간 적분기의 오차가 이 공간 오차를 설명하지 않는다. 극점 근처의 metric·cell average 근사 및 좌표 격자의 비균일성은 독립적인 공간 이산화 개선 문제다. $q$도 63.8→439.6→2764.9로 증가한다. 이 $\theta$ 등간격 격자에서는 극점 azimuthal rate가 $O(N_\theta^2N_\phi^2)$까지 커질 수 있다.

## 6. Native 이식 상태와 다음 단계

`src/pde_kernels.rs`는 외부 dependency가 없는 Rust 구현이다. graph는 $O(N+E)$ 저장공간을, workspace는 6개 길이 $N$ buffer를 한 번 할당한다. 성공한 step에서는 할당 API를 호출하지 않는다. `pde_native_driver.rs`는 실제 global allocator 계수와 동일 오차 기준의 결과 검사를 포함한다. root 통합자가 따로 실행하는 `PDE_NATIVE_BENCHMARK.jsonl`의 timing을 읽어야 wall-time 주장을 할 수 있다. 본 절의 작용 수를 native 속도비로 바꿔 부르지 않는다.

1. **양성 확산 subflow를 연결한다.** Client가 고정 graph와 기하 가중치를 선언하는 별도 API를 만들고, 강한 collision/affine 부분흐름과 Strang 결합한다. 반응 부분흐름의 질량·양성 계약을 따로 명시한다. 현재 standalone prototype는 adaptive solver·event·mass matrix·dense output과 연결되지 않았다. 독립 검토 뒤 native Euler와 RKL2에도 $q=0$ 또는 $t=0$에서 검증을 마친 입력을 그대로 반환하는 경로를 추가했다. $\pm1.5\times10^{308}$ 입력의 두 회귀 검사로 항등 흐름에서 불필요한 중간 overflow를 피함을 확인했다. $Q\ne0$, $t>0$인 일반 극단 진폭에서의 recurrence 크기 제어는 아직 남아 있으므로 모든 유한 입력에 대한 강건성을 주장하지 않는다.
2. **RKL2를 조건부로 연결한다.** 고정 자기수반 확산으로 범위를 제한한다. step doubling 및 위 $Q^3u_0$ 상한을 비교해 temporal budget을 정하고, 양성 위반이 중요한 client는 감지 후 uniformization으로 재실행한다. fallback 비용과 rejected work를 모두 기록해야 한다.
3. **극점 비용과 공간 오차를 분리한다.** cubed-sphere/등면적 계열 또는 longitude exact subflow를 후속 가지로 비교한다. exact longitude + latitude RKL2의 split은 비가환 오차가 있으므로 CFL 해소만으로 성능을 인정하지 않는다. root의 `sphere_pole_split.py`는 이 후속 가지이며 본 세 후보와 다른 네 번째 root 후보로 중복 계산하지 않는다.
4. **수송과 비선형으로 확장한다.** flux limiter, 변하는 계수, anisotropic tensor diffusion에서는 현재 대칭 graph 증명을 다시 적용할 수 있는지 확인한다. AMR은 accepted-step 사이에서 weighted conservative remap, 누적 remap 오차 및 재설정 후 controller 조건을 검증한 다음에 연결한다.
5. **큰 격자에서 검증한다.** 고정 물리 모드뿐 아니라 고주파·spike·경계층, 혼합 diffusion/reaction에 대해 spatial/temporal/splitting error를 별도 예산으로 기록한다. SIMD edge layout, 색칠에 따른 병렬 reduction은 이후의 성능 후보이며 현재 native 구현은 단일 스레드다.

## 7. 연구 루프와 근거 재사용

이론 루프는 기존 구면 연구의 재사용 범위 확정→고정 graph 문제 설정→세 후보 전개→질량·수축 항등식→Poisson 오차 상한→RKL 안정성·정확도 비용 정리→정확 positivity 반례→2D/3D/구면 예상 비용→독립 검토→두 조건부 후보 유지의 순서로 완료했다. 코딩 루프는 weighted graph prototype→독립 이산 oracle→실패 입력 검사→보존/양성 테스트→166행 sweep→continuum refinement 분리→정확 반례 재현→native workspace 구현→9개 native 검사→후속 통합 계약으로 이어졌다. 기존 8192-cell 정보 압축, Fisher, Thomson 증거는 다시 실행하지 않았다.

문헌: Meyer, Balsara, Aslam, *A stabilized Runge–Kutta–Legendre method for explicit super-time-stepping of parabolic and mixed equations*, JCP 257 (2014), DOI [10.1016/j.jcp.2013.08.021](https://doi.org/10.1016/j.jcp.2013.08.021), [OSTI 서지·초록](https://www.osti.gov/biblio/22230848). Exa로 원 논문 서지·초록을 확인했다. 본 문서의 행렬 조건과 상한은 위 식으로 직접 증명했으며, 새 방법을 발명했다는 주장은 하지 않는다.

## 재현 명령

다음은 `research/production_frontier_20261023/`에서 실행한다. Python에는 NumPy와 그림을 위한 Matplotlib이 필요하다. Rust kernel 자체에는 Cargo package가 필요 없다.

```bash
OPENBLAS_NUM_THREADS=1 python src/pde_frontier.py --test
OPENBLAS_NUM_THREADS=1 python src/pde_frontier.py --output results/PDE_CAMPAIGN.json
rustc --edition=2021 --test src/pde_kernels.rs -o /tmp/pde-kernel-tests
/tmp/pde-kernel-tests
rustc --edition=2021 -C opt-level=3 src/pde_native_driver.rs -o /tmp/pde-native-driver
/tmp/pde-native-driver 5 > results/PDE_NATIVE_BENCHMARK.jsonl
OPENBLAS_NUM_THREADS=1 python src/pde_plot.py
```

Native driver는 graph/workspace setup과 warmup을 timing 밖에 두고, 각 시험에서 0개의 hot allocation, 상대 시간 오차 $\le10^{-5}$, 질량 drift $<10^{-9}$를 검사한다. 이 설정은 setup 포함 end-to-end 비교와 다르다. 실제 comparator가 추가될 때 두 비용을 모두 보고해야 한다.
