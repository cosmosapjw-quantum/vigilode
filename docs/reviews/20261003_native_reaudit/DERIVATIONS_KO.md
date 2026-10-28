# 실행한 수학 점검과 정확한 적용 범위

이 문서는 미래 계획이 아니라 native chart transport의 수학적 근거와 구조 검증을 정리한다. 새로운 범용 RVJ 적분기의 존재나 속도를 주장하지 않는다.

## T1. 모델별 물리 오차 운반

무차원 변수, 고정된 kappa>0, y=x²(w+1/kappa)를 사용한다. 근사 중앙값 x,w 및 같은 시간/branch/parameter의 exact coordinates x+dx,w+dw를 고려한다. |dx|<=rx, |dw|<=rw가 **외부에서 이미 입증되어 있어야 한다**. a=w+1/kappa라 하면 직접 전개로

    (x+dx)²(a+dw)-x²a = (2x dx+dx²)a+(x+dx)²dw.

따라서

    B_coord=(2|x|rx+rx²)|a|+(|x|+rx)²rw

가 exact central reconstruction에서 exact physical value까지의 거리 상계다. 실제 저장된 physical y_hat가 x²a와 비트 수준에서 같을 필요는 없으므로

    B_phys=B_coord+|x²a-y_hat|

를 사용해야 한다. native 함수는 각 항을 Interval 또는 upward arithmetic으로 감싸며, denominator margin은 전체 x-box [x-rx,x+rx]에 대해 lower rounded min|x|²를 계산한다. whole box가0을 포함하거나 지정된 lower margin을 만족하지 않으면 거절한다.

단위가 있는 변수에서 같은 식을 쓰려면 x,w 및 kappa의 단위를 chart 정의와 함께 지정해야 한다. 이 구현은 무차원 특정 모델 전용이다. nonlinear parameter uncertainty, time-varying kappa, coordinate errors의 실제 생성 또는 true ODE local/global error는 이 정리에 포함되지 않는다.

## T2. 근사 cofactor의 오차항

이전 thread의 정의 L_F C=(-kappa+a)C+r_C, L_F D=aD+r_D에서 D!=0이면 quotient rule로

    L_F(C/D)=-kappa(C/D)+(r_C-(C/D)r_D)/D.

이 항등식은 재확인한 입력 이론이며 새로운 general solver 성능 결과가 아니다. numerator residual을 transformed forcing으로 변환하기 위해 denominator의 하한과 실제 w tube가 필요하다는 점이 T1의 conditional-input 계약과 연결된다. forcing을 생략한 numerical step의 error bound는 별도 작업이다.

## T3. 유한 path sum의 전제는 constructor 한 번으로 보존되지 않는다

유한 path sum의 근거 H^s=0은 strictly lower 구조에 의존한다. H의 diagonal/upper entry가 epsilon만큼이라도 생기면 일반적으로 H^s=0은 더 이상 성립하지 않는다. 실제 선언된 target이 upper entries를 무시하도록 정의되었는지와, 엄격한 삼각행렬이라고 선언한 입력을 잘못 바꾼 경우를 구별해야 한다.

AffineMajorant는 H0,H1,alpha_abs,seed를 public arrays로 보유한다. constructor 직후에는 valid해도 이후 배열 길이/숫자/구조를 바꾸면 같은 object가 더 이상 contract를 만족하지 않는다. 따라서 indexed access 전에 mutable storage를 다시 검사하는 것이 필요하다. 현재 보완은 built-in AffineMajorant의 shape, seed, sign 및 strict-lower 조건을 재검증한다. 사용자 정의 trait 구현이 고의로 불안정한 accessor를 제공하는 경우까지 type-theoretic proof를 제공하는 것은 아니다.

## 검증 구분

T1은 quotient polynomial의 전개와 삼각부등식으로 증명된다. Native dyadic corner/interior tests는 implementation regression이며 증명 자체를 유한 grid로 대체하지 않는다. Exact symbolic replay는 `verify_algebra.py`에 있다. T3의 실제 failure는 RED/GREEN logs에 남긴다. source tree identity, test success, correct theorem scope, production admission은 서로 다른 판정이다.
