# Regular quadratic homotopy의 native 계산

`quadratic_homotopy_root(a,b,c,lambda)`는 정확히 주어진 식
`lambda*c*u^2+a*u-b=0`, `a>0`, `0<=lambda<=1`을 푼다.
첨부 reflection-wall 초안의 degree-drop와 finite-root collision 구분을
일반 chart/예외 chart 분기로 구현했다. AT3의 증명과 잔차 항등식은
`ALGEBRAIC_TRANSFER_KO.md`에 있다.

1. 유한 입력과 양의 a를 확인한다.
2. b=0이면 정확한 root0 chart를 사용한다. c=0 또는 lambda=0이면
   linear chart b/a를 직접 계산하고 원래 식의 FMA 잔차를 보존한다.
3. 일반 chart는 C=lambda*c, M=max(a,|b|,|C|)로 공통 정규화한다.
   비영 계수가 underflow로 사라지면 이를 차수 하락으로 오인하지 않고
   `ScaleUnderflow`를 반환한다.
4. 정규화한 discriminant가 양수이고 roundoff 규모에 비해 분리되어 있으면
   상쇄를 피하는 `2B/(A+sqrt(A^2+4DB))`를 계산한다.
5. 원래 정규화 식의 잔차, 선택 branch의 양의 도함수 부호,
   relative residual screen을 확인한다. 보고하는 error indicator는
   binary64 산술 진단이며 outward-rounded enclosure가 아니다.

이 절차에는 derivative callback, 행렬, Newton iteration 또는 root search가
없다. 일반 chart의 작업량은 scalar 산술과 square root 한 번으로 상수다.
검사는 5개 test group에 여러 계수·lambda·scale을 넣어 수행했고,
추가로 a=1e-200의 linear chart와 a=1e-300,c=1e300,b=0의 zero chart를
독립 검토 후 보강했다. 초기/교정 로그를 모두 보존한다.

실제 물리 연결은 `y'=-alpha*y-beta*y^2`의 backward-Euler 잔차다.
`a=1+h*alpha`, `c=h*beta`, `b=y_old`, `lambda=1`를 넣으면
양의 해를 iterative solve 없이 구한다. alpha,beta,h>=0에서
`d y_new/d y_old=1/(1+h*alpha+2h*beta*y_new)<=1`이고
`0<=y_new<=y_old`다. 이 시간 이산화는 1차이며, root의 정확한 계산을
RODAS의 5차 시간 정확도로 바꾸어 부르지 않는다. 고차 integrator의
정확히 quadratic인 작은 closure에도 같은 chart를 직접 재사용할 수 있다.

일반 nonlinear callback을 polynomial로 자동 근사하거나, fold 너머의
branch를 자동 결정하는 코드는 포함하지 않는다. 그 확장은 원래 residual과
continuation domain을 제공하는 별도 client 계약을 요구한다.
