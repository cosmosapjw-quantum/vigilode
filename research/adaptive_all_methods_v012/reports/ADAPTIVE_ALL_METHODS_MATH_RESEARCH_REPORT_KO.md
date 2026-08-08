# 모든 현재 method의 adaptive step — 수학/물리 연구 보고서

이번 연구는 adaptive step을 단일한 `h *= error^(-a)` 장치로 보지 않고, **estimator–controller–state–cost–external error**의 다섯 층으로 분리했다.

핵심 결론은 다음과 같다.

1. 모든 family에 같은 acceptance/rejection/rollback 계약을 적용할 수 있다.
2. 모든 family에 같은 error estimator를 적용하는 것은 바람직하지 않다.
3. RODAS5P endpoint family는 기존 5(4) embedded estimator를 primary lane으로 유지한다.
4. BDF1/2와 Radau IIA1/3에는 step doubling을 첫 공통 reference lane으로 사용한다.
5. BDF2 adaptivity는 nonuniform coefficient와 history transaction 없이는 성립하지 않는다.
6. SDC, Peer/W, ROK/BOROK, exponential/Leja는 향후 구현 시 `h`뿐 아니라 sweep/order/subspace/matrix-function work와 결합된 adaptivity가 필요하다.
7. 최종 공정성은 내부 tolerance가 아니라 external global error와 total cost로 판정한다.

## Nonuniform BDF2

\[
\frac{1+2r}{1+r}y_{n+1}-(1+r)y_n+\frac{r^2}{1+r}y_{n-1}=h_nf_{n+1},
\qquad r=\frac{h_n}{h_{n-1}}.
\]

## Universal step-doubling reference

\[
e_{\rm fine}\approx\frac{y_{h/2,h/2}-y_h}{2^p-1}.
\]

이 estimator는 계산비용이 크므로 correctness/reference lane이며, production default 승격은 별도 Pareto gate를 요구한다.
