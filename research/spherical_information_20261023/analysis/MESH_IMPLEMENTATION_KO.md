# 정보 손실로 제어하는 구면 격자 압축: 구현과 작은 검증

## 대상과 입력

이 실험은 Paper I의 `prop:angular-power-contraction`,
`eq:angular-power-functional`에서 얻는 질량 없는 zero-log-fugacity 열적
에너지 단면을 사용한다. 물리 단위를 정규화한 양의 방향별 에너지를
\(\rho\)라 하고, 코드에서는 \(F(\rho)=-\rho^{3/4}\), 즉 \(\kappa=1\)로 둔다.
구면의 측도는 정규화 확률측도가 아니라 \(d\Omega=d\mu\,d\phi\),
\(\mu=\cos\theta\)이며 가중치 합은 \(4\pi\)다.

기준 데이터는 \(64\times128=8192\)개의 동일 면적 fine cell 중심값이다.
모든 압축 오차와 관측량 경계는 **이 이산 데이터에 대한 경계**다.
연속 함수의 quadrature 오차, fine cell 내부의 미관측 구조, 시간 적분 오차,
열적 단면 밖의 spectral-shape 오차는 포함하지 않는다. 초기장 다섯 개는
매끄러운 저차 모드, 국소 beam, 두 beam, 거의 진공인 beam,
\(10^{-8}\) 크기의 섭동으로 코드에 결정론적으로 정의되어 있다.

## 알고리즘과 계산식

각 직사각형 cell \(C\)에 대하여
\[
 W_C=\sum_{i\in C}w_i,\qquad
 \bar\rho_C=W_C^{-1}\sum_{i\in C}w_i\rho_i,
\]
\[
 \eta_C=\sum_{i\in C}w_iF(\rho_i)-W_CF(\bar\rho_C)
       =\sum_{i\in C}w_iD_F(\rho_i\Vert\bar\rho_C)\ge0.
\]
출력은 cell마다 양의 평균 하나를 보유한다. 전체 에너지는 보존된다.
중첩된 분할에 대한 엔트로피 손실의 telescope identity를 사용하여,
현재 cell을 자식으로 쪼갰을 때 제거되는 \(\eta\)를 추가되는 cell 수로
나눈 값을 우선순위로 삼는다. 주어진 예산 \(B\)에 대해 실제 leaf 손실의
합이 \(B\) 이하가 될 때까지 진행한다. 이것은 구현 가능한 greedy
전략이며 전역 최소 cell 수를 보장하는 최적화 알고리즘으로 주장하지 않는다.

평균 및 entropy aggregate에는 2차원 prefix sum을 사용한다. 거의 같은
수의 차로 계산되는 Jensen gap은 직접 Bregman 합으로 교체한다. 구체적으로
\(r=\rho/\bar\rho-1\)에 대해
\[
 D_F=\bar\rho^{3/4}\left[\tfrac34r-
       \operatorname{expm1}\{\tfrac34\log(1+r)\}\right]
\]
를 쓰며, \(|r|<10^{-3}\)에서는 8차까지의 이항급수에서 2차 이상 항만
평가한다. 이 방식은 \(10^{-8}\) 섭동의 \(O(10^{-16})\) 정보 손실을
큰 entropy 값의 단순 뺄셈 없이 보존한다. 음수 입력은 거절하며, 의미 있는
음의 divergence를 0으로 잘라 숨기지 않는다. 완전한 fine cell은 입력값을
그대로 복사한다.

## 관측량 검증

관측량 \(O_K=\sum_iw_iK_i\rho_i\)와 cell별 평균 \(\bar K_C\)에 대해
\[
 |O_K(\rho)-O_K(\bar\rho)|\le
 \sum_C\sqrt{\frac{2\eta_C}{m_C}
                 \sum_{i\in C}w_i(K_i-\bar K_C)^2},
 \qquad m_C=\frac3{16}(\max_{i\in C}\rho_i)^{-5/4}.
\]
이는 \(F''\ge m_C\)인 강한 볼록성과 cell별 에너지 보존으로부터 나온다.
코드에서는 dipole, quadrupole, 국소 response의 세 kernel에 적용했다.
analytic inequality와 부동소수점 검사는 별개다. 검사에는
\(2\times10^{-13}\max(1,\sum_iw_i\rho_i)\)의 roundoff 여유를 사용했으며
interval arithmetic으로 보증한 결과는 아니다. 모든 case에서 직접 계산한
실현 entropy 손실도 예산 이하인지 별도로 확인한다.

## 실행 결과

`results/mesh_validation.json`의 실제 명령으로 8개 unittest와
20개 구성(5개 초기장 × 4개 예산)을 실행했다. 두 분할 방식에 대한
40개 직접 entropy 예산 검사와 120개 관측량 경계 검사가 통과했다.
최종 실험 자체의 기록된 wall time은 약 8.01초다. 이는 진단/압축 비용이며
PDE solver wall time이 아니다.

| 장, root loss 대비 예산 | 중첩 균일 cell 수 | 적응 cell 수 | cell 수 비율 |
|---|---:|---:|---:|
| smooth, 1% | 1024 | 445 | 2.30 |
| localized beam, 1% | 4096 | 106 | 38.64 |
| two beams, 1% | 4096 | 229 | 17.89 |
| near vacuum, 1% | 4096 | 112 | 36.57 |
| tiny perturbation, 1% | 1024 | 439 | 2.33 |

균일 baseline은 같은 허용 entropy 예산을 만족하는 첫 중첩 4분할 level이다.
따라서 표는 같은 **허용 예산**의 비교이며, 실현 오차가 정확히 같지는 않다.
격자 단계의 이산화 때문에 균일 격자가 예산보다 훨씬 작은 오차를 갖기도 한다.
예를 들어 localized beam의 1% 예산은 0.0108778이고, 적응/균일 손실은
각각 0.0105917/0.00468740이다. 이 표의 비율을 시간 가속률로 부르면 안 된다.

모든 구성·두 방식에서 최대 상대 에너지 drift는
\(2.92\times10^{-15}\)였다. 양의 재구성도 유지되었다.
이 결과의 실제 solver 이익은 뒤의 RHS가 활성 cell 수에 선형에 가깝게
비례하고, 이 압축 구조를 쓰는 conservative flux/transfer가 구현될 때
얻어진다. 현재 prototype의 extrema 및 안정화 fallback 스캔까지 포함한
diagnostic 비용은 worst case \(O(N_\mathrm{fine}\log N_\mathrm{fine})\)이며,
prefix aggregate 자체만 \(O(1)\) query다. heap 비용은 추가로
\(O(N_\mathrm{active}\log N_\mathrm{active})\)다.

## 입자수 보존이 요구되는 경우의 수정

한 개의 \(\rho\)만 평균하면 zero-fugacity 평형 단면의
\(n\propto\rho^{3/4}\)를 보존하지 않는다. 이 구현은 그 차이를 삭제하지 않고
모든 결과에 기록한다. 정규화된 \(n_\mathrm{scaled}=\rho^{3/4}\)에서는
\(\Delta N_\mathrm{scaled}=\sum_C\eta_C/\kappa\)다. C=1인 물리 MB
정규화에서는 \(n=2(\rho/6)^{3/4}\), 따라서 물리 entropy 손실과는
\(\Delta N=\eta_\mathrm{physical}/4\)로 연결된다.
near-vacuum의 느슨한 10% 예산에서 상대 입자수 drift는 14.38%,
1% 예산에서도 1.474%였다. photon-number 보존 계산에는 이 one-field
평균을 그대로 도입할 수 없다.

이를 고치는 joint MB 단면의 작은 검증도 포함했다:
\[
 F(n,\rho)=n\log\frac{27n^4}{2C\rho^3}-4n,
 \qquad \nabla^2F=
 \begin{pmatrix}4/n&-3/\rho\\-3/\rho&3n/\rho^2\end{pmatrix},
 \quad \det\nabla^2F=3/\rho^2>0.
\]
양의 가중 평균으로 \((n,\rho)\)를 함께 옮기면 두 적분량을 모두 보존하고
Jensen loss가 양수다. 네 thermochemical 상태의 독립 검사에서
Jensen/Bregman loss는 0.50052746254560으로 일치했고 두 보존 오차는
부동소수점 출력에서 0이었다. 이 검증은 joint 적응형 solver 전체의
구현을 뜻하지 않는다. 다음 실제 이식에는 joint state 또는 energy-bin별
occupation을 보존하는 transfer가 필요하다.

## 재현 및 적용 범위

```bash
OPENBLAS_NUM_THREADS=1 python3 -m unittest discover -s src -p test_spherical_mesh.py -v
OPENBLAS_NUM_THREADS=1 python3 src/spherical_mesh.py --output results/mesh_results.json
```

NumPy 2.3.5와 Python 환경은 결과 JSON에 기록했다. source/result SHA256와
실제 stdout/stderr는 `mesh_validation.json` 및 `mesh_run_*`에 있다.
본 실험은 파일 기반으로 독립 실행하며 VigilODE repository를 수정하지 않는다.
