# D2: RHS 호출 전 stage-domain 보호와 실패 transaction

2026-10-24 KST. PR84의 endpoint 검사를 내부 RHS 호출 전 검사로 확장한 독립 Rust 연구 소비자다. 실제 생산 native crate나 BASS client 완료가 아니다. 전체 상세 보고서·정확 다항식·raw·실패·예산은 FINAL_DELIVERY의95-payload companion에 있으며 이 파일은 출판 요약이다.

## 조사와 선택

Parent086e55d30c11a5ca9d6dd122c1840b725ccd5fda와 관련 local-frontier aaf1f834d4c589e734ca7851a56c11bb44f7047a를 확인했다. Frontier는 마감 때도 같았다. 공개 PR84에는 새owner완료comment가 없었다. 기존연구ref metadata,관련PR,접근가능대화·첨부·백업을 대조했지만 모든 branch파일/모든대화를 심사한 것은 아니다. BASS185 event-rate/REC source와 REI223 history는 별도물리근거이지 nativeV1완료가 아니다. DesktopCommander의 명시적quota재시도금지를 지켰고 사용자dirty/담당예약은 미확정이다. Parent root/research AGENTS404를 다른경로 부재로 일반화하지 않는다.

선택한 문제는 D1의 구체적 미완료였던 internal-stage domain이다. Endpoint가 양수여도 이미 금지된 RHS를 평가할 수 있다. Nativeowner코드 대신 독립소비자의 pre-RHS검사/partialwork/abort를 구현했다. 기존tableau/stage식/norm/tolerance/endpointdomain은 유지하고 clipping/projection/exactflowfallback을 쓰지 않았다.

## 새 이론과 근거

Scalar y'=-lambda*y,x=h*lambda,d=1+gamma*x에 대해 pinned binary64 계수를 정확한 유리수로 해석한다. Zero-based u_i=N_i/d^(i+1), P_i=d^i+sum A_ij*N_j*d^(i-j-1), N_i=gamma[-x*d^i+sum(C_ij-A_ij*x)*N_j*d^(i-j-1)]. RHS 입력multiplier는P_i/d^i다.

두번째 RHS 입력 G_1=(1-2gamma*x)/(1+gamma*x)는x=4에서-0.3764039563837087이지만 endpoint R(-4)=0.016497504408650183>0이다. Compiled unit test도 첫RHS/solve각1회후 두번째호출전에중단한다. Guard를끈polynomialextension대조는음수stage를평가하고양수endpoint를낸다. 그경로를안전한restrictedclient라고부르지않는다.

각8stage와endpoint의 numerator와denominator-minus-numerator 총18다항식은[0,2]에서nonnegative Bernstein coefficients를갖는다. 이는exact scalar-linear 전체구간에서0<=G_i,R<=1의충분조건이다. 저장검산은basis역변환및9개의정확rational node에서직접stage recurrence와다항식identity를결속했다. 샘플부호만의추정이아니다. Wolfram은stage2항등식과rational sign,partition unity를별도로확인했다. Nonlinear/matrix/compiled-roundoff/SSP전체정리는아니다.

제안상한1.8/max(lambda+2alpha*z)는linear구간2의90%를사용한다. Alpha0에서는그선형유도에연결되지만alpha4에서는frozen-rate heuristic이다. Runtime실제검사와norm/endpoint승인이권위이며cap은commit허가가아니다.

## 코드와 실제 실행

Stage입력복원과exactgroupdomain검사후에만보호RHS를호출한다. Callback출력은NaN으로초기화하고유한성검사한다. StageDomain/recoverable callback/fatal callback/InvalidInput/NonFinite와stage index,partialrhs/solves/checked를반환한다. 분석Jacobian은제조모형에고정돼있으며임의callback과호환된다는API가아니다.

Session abort는epoch/id/h/nexttime lease를검사하고한번만pending을정리한다. Recoverable중단은accepted state/history를유지하고h반감,explicitfatal은sticky다. Work는begin에서이미차감돼중복차감하지않는다. Stalelease가livepending을없애지않는다. Callback/observer의외부sideeffect나panic/environmentmutation의rollback은없다.

새15고유시험+변경Session경계31회귀가통과했다. 최초12RED의11featurefail/1controlpass와compilerwarnings를보존한다. No-IO guardedattempt128회에서heap요청0이며caller/IO/전체solver까지의무할당은아니다.

새12구성:alpha0/4,tol3e-6/3e-9,세arms,t=[0,.6],landings.075/.15/.3/.6,h0.04,cap20000. Rates1/20/300과analyticdiagonalJacobian을사용한다. HistoricalD1/다른ODEcampaign은replay하지않았다.

|합계|endpoint-only진단|guard+반감|guard+ratecap|
|---|---:|---:|---:|
|attempts|578|1032|602|
|accepted|440|598|585|
|stageabort|0|419|0|
|vectorRHS/diagonalstage solves|4624|5323|4816|
|금지RHS입력평가|507|0|0|

모든12궤적종료. Cap은guard대비attempts41.67%,RHS약9.52%감소지만전체walltime가속을측정하지않았다. 진단baseline은restricted-RHS안전의무를위반하므로동등계약속도승자로부르지않는다. 비선형cap의stageabort0은이번표본의관측이지보편적증명이아니다.

완료된새trial1793개에서기존reduced_step을같은입력에추가호출하여candidate/error bit parity를확인했다. 이검사호출은armwork와별도로보존한다. 별도Fraction경로가15182stage/2212trial의domain,callbackpermission,partialwork,rollback을검사하고1623acceptedendpoints와59169scaleenclosures를확인했다. MP80으로48outputs/1584components를대조하여maxabsolute4.624309496247777e-10,maxcomponentwise scaled0.09774487167548945였다. WRMS/globalcertificate/denseoutput인증은아니다.

## 검토·보존·남은 gate

다른Fraction/MP80/CAS계산경로지만sameauthor다. Independentpersonnel/native_subagentNOT_RUN. 기존nativeowner/localdirty/NCP/default/core/공유권한은변경없다. 생산forbidunsafe와상속researchMXCSRasm의safe경계는여전히미해결이다. 새stagecore자체는forbidunsafe지만소비자전체production적합성을뜻하지않는다.

이전science에부재했던stage영역보호를닫았지만finite-retrytermination,대규모/실제BASS,fullnativeintegration,callback부작용복구,dense-pathdomain은열려있다. 다음은현재nativeowner/source/완료receipt를결속하고실제로빠진integration만수행하는것이다. 또다른norm-onlychild나완료NCP/V3시험을자동생성하지않는다.

새closedsubprocess25.104395919s/cap120에는toolchain17.927572074s,실제오류(b/B키)와schema수정,RED/GREEN,캠페인과저장검산이포함된다. Prior83.295665783s와olderremainingnull유지;transport/drafting전체시간이아니다. S2publication1ef6c519262560c2281a58b4cbb8ed38725ad90f의9sourceblob이실행파일과일치한다. Publication을execution-timecommit으로소급하지않는다.

S0-S3각각Drive/Dropboxbackup과expectedSHA게시. Globalpercentnull,productionHOLD. Companion의게시전DAG와나중Gitreceipt를구분한다. SciSpace/arXiv2005.06268과SUNDIALSCVODE문헌은구별을위한문맥이며이번정리의대체증명이아니다.

