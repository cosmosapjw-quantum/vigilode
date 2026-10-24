# V1r: full33 오차 판정에서 적응 궤적까지

2026-10-24 KST. 이번 새 결과는 독립 Rust 연구 소비자와 제조모형 RODAS5P 궤적이다. 실제 생산 crate 빌드, native V1 hook 완료, 실제 BASS client 검증으로 부르지 않는다. 아래는 게시 요약이며 전체 상세 보고서·원자료는 FINAL_DELIVERY.json의56-payload companion에 있다.

## 조사·재사용

조사 당시 브랜치 이름100/100/0페이지,200개를 확인하고 관련 최신 소스·PR·접근 가능한 대화/라이브러리·Drive/Dropbox를 대조했다. 이는 모든 branch 파일·대화의 전수 심사와 다르다. Latest candidate-reconsideration94db44d1e7487a3b5e4f8b97fea9504b2c2e396c는 마감 때도 같았다. 그 근사 몫대수·prepared LU·causal 전환 결과와 GeneralOde의 callback/scale/session 작업은 재사용하고 중복 집계하지 않았다.

Parent PR82=e41e4721798bba8c46b3256467b3279fafe9918d. Native fast attempt의 inspected source blob5fad7f22d92a668b12920375c0c43858180335a4는 reduced WRMS를 직접 계산했다. 현재 owner-local 수정은 unobserved이며 다른 미공개 hook의 부재를 증명하지 않는다. Full native workspace/dependencies를 복구하지 못해 production core를 건드리지 않고 연구 소비자를 구축했다. Root/research AGENTS404는 해당 pin/path만의 결과다.

## 이론·계산 계약

고정 y=Lz+b에서 e_full=Le, E²=sum((Le)_i/s_i)²/33이다. s_i=atol_i+rtol_i max(|Yold_i|,|Ytrial_i|)는 공급된 binary64 physical states/tolerances의 exact-real 식이다. atol>0,rtol>=0,finite 상태를 요구한다. Upstream affine 복원 반올림을 자동으로 인증하는 것은 아니다.

Guarded RN/gradual-underflow/no-fast-math 아래 outward 곱·합으로0<slo<=s<=shi<infinity를 만든다. Scale monotonicity에서 E²(shi)<=E²(s)<=E²(slo). Integer upper at slo<=1일 때만Accept, integer lower at shi>1일 때만Reject, 나머지는Indeterminate다. Floating estimate는 h 제안에만 사용한다. Scale 표현범위 실패는 typed refusal이다. x86_64 read-only MXCSR guard는 비지원rounding/FTZ/DAZ를 거절하고 다른architecture도 명시적으로 거절한다.

RL=I,F(Lz+b)=Lf(z),J_F L=L J_R와 shifted solve 가역성 아래 W_F L=L W_R이므로 exact arithmetic에서 각 raw-U stage가u_F=L u_R로 대응한다. Wolfram은RL=I,Jacobian/shifted잔차0,Riccati해잔차0,scale미분-2u²/s³를 확인했다. Float 경로의bit동일성이나 전체컴파일러형식검증은 아니다.

계수snapshot SHA256211255f6125b648c6955833c3da537d9c031ec78168b2918563c29fa88c8bce6의8-stage raw-U식이다. 새로fitting한표가 아니다. 실제제조RHS z'_j=-lambda_j z_j-alpha z_j²는lambda1/20/300반복,alpha0또는4이며 analytic diagonal Jacobian을 사용한다. **판정기에미분이없다는 사실을 전체Rosenbrock경로의derivative-free달성으로바꾸지않는다.** 시간·상태는제조모형에서무차원이고일반client의component단위는별도계약이다.

## 실제 소비자와 상태 보존

Session은accepted t/state/epoch/마지막accepted estimate와별도pending lease/누적attempt를분리한다. Begin에서실제표현가능한전진과예산을확인한다. Finish는epoch/id/h/nexttime binding을검사한다. Rejection은accepted history를유지하고h만줄인다. Invalid scale/state/environment와Indeterminate는상태를commit하지않는다. 후자는sticky NeedsPrecision으로무한재시도를막는다. Stale/forged lease가정상pending을없애지않으며최종도착은예산소진보다먼저확인한다.

범용callback/cancel API, 비동기scheduler 또는 higher-precision resume는이번범위밖이다. Norm통과와strict physical-domain통과는별도다.

## 실행과 독립 계산 경로

최초19개RED가실제로실패한뒤기본GREEN19와추가2개를포함한최종고유21개시험이통과했다. 중복호출을고유시험으로합산하지않았고warnings/RED를보존했다.

2models×3tolerances(1e-4,1e-7,1e-9)×3arms=18개새adaptive IVP. Arms는physical integer30,naive30point,독립full33point이며t=[0,1],h0=.05,20landingtargets,cap10000attempts다. Initial포함378출력행을보존했다. 별도fixed-step5개와same-step8pairs도실행했다. Full33는dependentRHS와triangular shifted system을실제로적분하며reducedendpoint복사reference가아니다.

Physical30의740trials=694Accept+46Reject에대해저장bits에서Fraction exact식과24,420component scale enclosure를재계산했다. 전부판정과일치했다. 이전PR81/82의완료corpus와NCP는재실행하지않았다. 별도198scaleprobes는158enclosed/40typedrefusal이다. Refusal40을모두진짜수학적overflow라고하지않는다.

MP80해석식비교의최대absolute error6.384068352401881e-8,최대componentwise scaled error0.11331076951310468이다. 후자는WRMS가아니라성분별max다. 유한testceiling20tol을만족했지만globalcertificate는아니다. Same-stepstate차이최대1.687538997430238e-14,error차이3.101685575046531e-15;다르게적응한physical30/full33의공통출력차이는1.6667399370395852e-9다. Fixed-step관측order2.5794,4.4466,4.7890,4.9077은작은h에서5에접근하지만stiff-uniformorder보장은아니다.

16MXCSR child조합중1지원/15거절,0x1f80복구확인. Assessment128calls에서heaprequests0이다. Parser/output/전체solver의무할당은아니다. Fraction/MP80/CAS는다른계산경로지만같은작성자이고독립인원/native subagent NOT_RUN이다.

## 음성 결과와 의사결정

정확해는양수지만수치저장성분1180표본이음수였고min=-3.5596679445611166e-11이었다. 이는unique physical events가아닌component/time/arm표본이다. Clipping0회. 보존합maxresidual1.3322676295501878e-15와작은오차를동시에만족해도positivity는보장되지않는다. Strictpopulationclient에는별도domainadmission 또는positivity-preservingcandidate가필요하다.

큰row판정의proposal estimate8이라는단순정책을썼다. Physical경로는tol별5/8/10rejections,full33point는각3이었다. 안전한수락판정과경제적인h제안은별개의연구다. 원인별walltime기여는분리하지않았다.

RUNS.csv timers는physicalarm만전체trialbits를출력하는등로깅의무가달라속도비교에사용하지않는다. 새DVODE/Diffsol/whole-solver가속은측정하지않았다. 현재채택은기능소비자의실행가능성이고생산기본값교체가아니다.

## 보존·남은 목표

S0/S1/S2/S3macro단위별backup과expected-SHA게시. S2 sourcecommit078eed34be19ff89c254b2b4c37fe49362b39be2의9개Rust/Cblob이실행원본과일치했다. 실행후게시commit을실행당시commit으로부르지않는다. 다른branch/localdirty/NCP/default는변경없고merge없다.

Closed new subprocessbudget20.620188011000437s(cap120)에는compiler복원16.949s가포함된다. Prior23.030350177+10.984972582+7.354545033와olderremainingnull을초기화하지않았다. 조사/집필/전송전체시간은이값이아니다.

완료는V1rconsumer와manufacturedtrajectory단위다. Nativehook/actualBASS/default-WRMSdigest/독립인원/production은열려있다. V3harmonicpriorPASS재사용,globalpercentnull. 다음은현재nativeowner의source/dirty상태/완료근거를연결한뒤진짜누락을이식하는일이다. 또다른norm-onlychild를자동생성하지않는다.

문헌보조: Ranocha & Giesselmann arXiv2307.12677v3(primaryabstract확인). 그explicit-RK controller안정성결과를이번Iproposal의보장으로수입하지않았다. 전체연속오차·실제BASS물리·positivity보장은이번결과에없다.
