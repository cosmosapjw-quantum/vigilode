# 구면 모멘트 역산의 Fisher 전처리: 작은 구현 검증

이 실험은 Paper II의 열적·화학적 susceptibility로 만든 **유한 양의 구적 격자의 모멘트 역산 Hessian**을 계산한다. 선형계의 반복 수 감소를 확인했으며, 전체 VigilODE 궤적 적분이나 실제 BASS 물리 클라이언트의 속도를 측정한 실험은 아니다.

## 정의와 구현 대응

`src/fisher_demo.py`에서 $x=E/E_*$, $z=mc^2/E_*$, $v=(1-z^2/x^2)^{1/2}$이며, 공통 양의 물리 상수 인자는 생략했다. 이 인자는 조건수와 상대 잔차에 영향을 주지 않는다. 실구면조화함수는 $\int Y_{\ell m}^2\,d\Omega=1$로 정규화한다. 정규화 면적 $d\omega=d\Omega/(4\pi)$와 $Z=\sqrt{4\pi}Y$를 사용하는 이론 문서와는 이 인자로 대응한다.

모드별 자연 변수 순서는 $(\eta,b)$이며 점유수의 자연 점수는
$$
\psi=\eta-xb,\qquad
t_{\ell m}=v^\ell Y_{\ell m}(1,-x).
$$
Paper II의 $(b,\eta)$ 순서와 다르므로 부호와 순서를 코드에 명시했다. $W=f(1+\xi f)>0$에 대해
$$
H=A^{\mathsf T}\operatorname{diag}(wW)A.
$$
MB, FD, 정칙 BE를 지원한다. BE에서는 모든 점의 $\psi<0$를 요구하며, NaN, 무한값, 표현 가능한 양의 susceptibility가 없는 입력을 거부한다.

등방 기준에서 한 rank의 블록은
$$
G_\ell=M_0
\begin{pmatrix}1&-\mu\\-\mu&\mu^2+\sigma^2\end{pmatrix},
\quad
\mu={\sum_r w_rW_{0,r}v_r^{2\ell}x_r\over M_0},
\quad
\sigma^2={\sum_r w_rW_{0,r}v_r^{2\ell}(x_r-\mu)^2\over M_0}.
$$
분산은 $M_2/M_0-(M_1/M_0)^2$ 차분으로 계산하지 않는다. 특히 좁은 에너지 구간에서 중앙 분산식을 사용한다. 실제 PCG에는 이 $2\times2$ 블록의 역행렬을 적용한다. 별도 spectral 검산에는 중심화된 score $1/\sqrt{M_0},(\mu-x)/\sqrt{M_0\sigma^2}$를 직접 구성한다.

실험의 비등방 $\Delta\psi$는 실제 보유하는 $v^\ell Y_{\ell m}(\eta-xb)$ span에 속한다. 이를 독립 최소제곱 잔차로도 확인한다. 따라서 임의의 양의 가중 Gram 행렬만을 검사한 것이 아니라 해당 유한 자연변수 family의 Hessian을 검사한다.

양의 구적 합에서 $a\le W/W_0\le b$이면 $aH_0\preceq H\preceq bH_0$이며, whitened Hessian의 모든 고유값은 $[a,b]$에 있다. MB·FD에서는 $\|\Delta\psi\|_\infty\le\delta$로 $a=e^{-\delta},b=e^\delta$를 사용할 수 있다. BE에서는 점수의 음의 경계 $\psi,\psi_0\le-\gamma<0$와 $\coth(\gamma/2)$ 인자를 사용한다. 코드의 점별 실제 ratio는 이 분석 경계보다 강한 유한 격자 경계를 제공한다. 부동소수점 허용 여유를 둔 검산이며, 방향성 반올림을 사용한 구간 인증은 아니다.

## 실행과 결과

```bash
OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 python3 src/test_fisher.py
OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 python3 src/fisher_demo.py
```

7개 테스트가 통과했다. 테스트는 구면조화 구적 직교성, 점유수 미분과 susceptibility의 독립 중앙차분, BE 경계 입력 거부, 좁은 에너지 영역의 기준 whitening, generalized 고유값 경계, 실제 자연좌표 잔차, 영 우변 처리를 포함한다.

13개 실험은 MB의 (z=0,2,8), (L=1,3,5), FD·BE 추가 사례, 좁은 에너지 구간, 등방 기준으로 구성했다. 양 방법 모두 $\|Hx-b\|_2/\|b\|_2\le10^{-9}$에서 종료한다. CG 내부의 재귀 잔차만을 신뢰하지 않고 매 반복 실제 행렬곱으로 이 잔차를 평가한다.

| 사례 | 자연좌표 조건수 | whitened 조건수 | CG → PCG 반복 |
|---|---:|---:|---:|
| MB, (z=0,L=5) | 97.70 | 1.83191 | 36 → 11 |
| MB, (z=2,L=3) | 681.38 | 1.66229 | 32 → 10 |
| MB, (z=8,L=5) | 356,678.31 | 1.59005 | 124 → 9 |
| MB, 좁은 (x\in[4.99,5.01]), (z=2,L=3) | 42,464,852.82 | 1.58573 | 26 → 9 |
| MB, 등방 기준 (z=2,L=3) | 579.17 | 1 | 11 → 1 |

실제 실행값과 모든 반대 결과는 `results/fisher_results.json`에 있다. 좁은 에너지 사례의 큰 조건수는 CG 반복 수와 일대일로 대응하지 않는다. 전체 스펙트럼 분포와 우변이 반복 수에 관여하기 때문이다.

짧은 시간 측정은 매 PCG 풀이의 블록 재구축과 적용 비용까지 포함한다. 공통 Hessian 구성과 spectral 검사는 측정 밖이며 그 비용은 별도 기록한다. 측정은 한 번의 warmup 이후 목표 약 5 ms인 7개 batch로 구성했다. 가장 어려운 (z=8,L=5) 사례에서는 중앙 풀이 시간의 CG/PCG 비가 약 5.07이었지만, (L=1) 사례의 전처리 비용은 반복 수 감소보다 커서 PCG가 느렸다. 작은 공유 호스트 측정으로 전체 솔버의 일반적 가속 배율을 주장하지 않는다.

구현은 작은 문제의 명시적 dense Hessian matvec를 사용한다. 블록 적용은 $O((L+1)^2)$, 블록 구성은 $O(n_r(L+1)+(L+1)^2)$다. 실제 구면 대규모 적용에서는 $Hv=A^{\mathsf T}[wW\odot(Av)]$의 matrix-free 경로가 별도로 필요하다.

## 정확도 해석과 적용 우선순위

같은 자연좌표 잔차를 요구해도 해의 오차가 같아지지는 않는다. 생성 시 알고 있던 참 해를 이용한 (H)-energy 상대 오차와 자연좌표 상대 오차를 결과에 모두 기록했다. 예를 들어 좁은 에너지 사례는 같은 잔차 기준에서도 plain CG의 energy 오차가 약 $1.98\times10^{-6}$, PCG는 약 $1.06\times10^{-9}$였다.

다음 구현에서는 물리 오차와 직접 연결되는 종료 기준을 사용할 수 있다. (x_*=H^{-1}b), (r=b-Hx)이고 위의 (a,b_\mathrm{up})가 알려졌다면
$$
{\|x-x_*\|_H\over\|x_*\|_H}
\le
\sqrt{b_\mathrm{up}/a}\,
{\|r\|_{H_0^{-1}}\over\|b\|_{H_0^{-1}}}.
$$
이는 $H^{-1}\preceq a^{-1}H_0^{-1}$와 $H^{-1}\succeq b_\mathrm{up}^{-1}H_0^{-1}$를 각각 분자와 분모에 적용한 결과다. 이 저비용 잔차 인증은 현 실험의 종료 기준에는 사용하지 않았으며 후속 구현 계약으로 남긴다.

우선 적용 대상은 각도·열화학 모멘트의 역산이나 최대엔트로피 closure의 Newton 내부 SPD 선형계다. 일반 시간적분기의 비대칭 Jacobian이나 GMRES에 같은 조건수 정리를 그대로 옮기지 않는다. 기준 state를 갱신하는 비용과 susceptibility ratio를 확인해, 기준 갱신 또는 직접 풀이가 더 싼 작은 문제를 함께 선택해야 한다.

## 근거 보존

`results/fisher_execution.json`에 실제 명령·종료 코드·로그와 최종 source/output SHA256을 남겼다. 최초의 일반 양의 가중 Gram probe는 수학적으로 유효하지만 현재 실제 자연변수 family의 실험보다 적용 범위가 넓어 별도 보존했다. 최초 단일 풀이 timing도 보존하고, 최종 결과에는 batched timing을 채택했다. 이 변경은 실패를 숨기거나 공차를 바꾼 것이 아니라 모형 대응과 시간 측정의 명료화를 위한 것이었다.
