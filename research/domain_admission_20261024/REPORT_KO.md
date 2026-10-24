# D1: 물리적 정의역을 지키는 적응 step 승인

2026-10-24. PR83의 음수 population 관측을 실제 opt-in domain rejection으로 처리했다. Tableau, stage 식, integer norm, scale 및 tolerance를 유지하고 clipping/projection 없이 후보를 거절한다. 모든 새 제약 궤적이 종료됐고 수락 endpoint는 지정된 정의역을 만족했다. 단순 h/2 재시도의 비용 증가는 그대로 남겼다. 이는 독립 연구 소비자이며 production native crate나 actual BASS client 완료가 아니다.

## 최신 근거와 범위

Current local-frontier HEAD aaf1f834d4c589e734ca7851a56c11bb44f7047a의 보고서·exports·fast norm을 읽고 마감 때 재확인했다. 그 polynomial resolvent/continuation/phase-network/local benchmark 결과는 재사용하며 중복 실행하지 않았다. 조회한 native fast 코드는 자체 차원 WRMS를 계산한다. 그러나 owner-local 상태는 Desktop Commander 월간 한도로 읽지 못했으므로 미공개 hook의 부재나 작업예약 해제를 주장하지 않는다. 재접속/우회/사용자 checkout 변경은 없다.

Parent PR83 HEAD577f350407aceec7e304cf0e66427141886f4c50은 그대로였다. Root/research AGENTS는 조회한 위치에서404였다. 접근 가능한 대화·첨부·최근 소스·Drive/Dropbox parent metadata와 로컬 archive56payload를 대조했다. 마감의20261024 branch 검색은7개와 빈 다음 페이지, 관련 최신 PR 검색은VigilODE82/83와REI222/223을 반환했다. REI223의 history/H5 HOLD를 solver qualification으로 사용하지 않는다. 이전 전체 inventory는 재사용했으며 모든 branch파일/대화의 완전 전수심사는 아니다.

## 이론과 반례

Fixed polytope D는 모든 compact z_j>=0와 z23<=1,z24+z25<=1/2,z26+z27+z28<=exact(binary64(0.2))로 정의된다. Full33 dependent populations는 이 총량에서 그룹합을 뺀 값이다. 각 그룹의 최대 세 항을 integer*2^-1074로 정확히 더하므로 float합에서 사라지는 minsubnormal 초과도 발견한다. 실제 반환 full33의 부호도 별도로 검사한다. 이 계약은 선언된 연구용 정규화 population에 한정된다.

정확한 pinned-binary64 tableau를 유리수 raw-U recurrence로 평가하면 선형 감쇠의 R(-4)>0,R(-8)=-0.023374915029575196…<0이다. 부호가 바뀌는 bracket은 약5.179489120755712 근처이나 전역 최초근을 증명한 것은 아니다. 실제 Rust step도 y=1e-12,lambda300,h=8/300에서-2.3374915029568486e-14를 반환했다. atol1e-8/rtol0의 norm은 이 후보를 승인했다. 따라서 작은 절대오차와 양수성은 다른 조건이며, 이 반례를 단순 반올림잡음으로 설명할 수 없다.

초기 상태가 D 안에 있고 norm과 domain이 모두 승인한 후보만 commit하면, 귀납적으로 수락된 represented endpoints는 D 안에 있고 실제 반환성분은 비음수다. Internal stage/dense path, 실제 global ODE error, 경계에서의 유한 재시도 종료는 이 명제에 포함되지 않는다. Domain rejection은 accepted history를 유지하고 한 시도를 차감하며 h를 절반으로 줄인다. Invalid error/scale은 domain rejection으로 숨기지 않는다.

문헌 비교: Nusslein/Ranocha/Ketcheson arXiv2005.06268은 weights를 선형계획법으로 조정한다. 이번 reject-only 설계의 증명으로 가져오지 않았다. Wolfram은 group sums, zero boundary, Riccati residual과 정수용량을 확인했으며 이는 program proof가 아니다.

## 실제 구현과 검증

31unique tests=10new domain contracts+21changed-surface regressions. 최초 RED25control PASS/6feature FAIL, 최종31PASS. Default new()는기존동작을유지하고 new_nonnegative()만정의역을강제한다. Exact-group boundary,rounded-away excess,invalid tail,negative zero,초기거절,rollback,누적예산을검사했다. 128domain-predicatecalls에서heaprequests0;전체solver/IO무할당은아니다.

새8configuration은alpha0/4,tol1e-5/1e-8,legacy/nonnegative다. Rates1/20/300,t=[0,.8],landings.1/.2/.4/.8,h0.05,cap10000을고정했다. 과거18campaign을재실행하지않았다. Analytic diagonal Jacobian을사용하므로전체적분을derivative-free라하지않는다.

|모형|tol|기본 attempts|제약 attempts|제약 accepted|domain rejection|
|---|---:|---:|---:|---:|---:|
|선형|1e-5|41|157|75|76|
|선형|1e-8|133|177|138|30|
|Riccati|1e-5|44|157|75|75|
|Riccati|1e-8|136|183|142|32|

제약430accepted endpoints는모두exact-group 제한과represented nonnegative를통과했다. Min5.618867265215142e-114,negative0. 기본경로의accepted endpoints에는350negative component samples,min-4.1042883662806265e-11이었다. Clipping0이다. 이전1180표본과분모가다르므로합치지않는다.

Default354newtrial에서원본Session을같은candidate에적용하여state/h/decision/counters/lastaccepted estimate bit parity를확인했다. 이는actualproductiondefaultdigest가아니라연구소비자의changed-surface regression이다.

별도Fraction경로로1028저장trial의norm/domain/rollback과33924scalecomponent포함을검사했다. MP80으로32outputrows/1056components를확인했다. 최대componentwise tolerance-scaled outputerror0.11154113065441769,최대conservation residual9.925908233774998e-17. WRMS/globalcertificate나극소population의좋은상대오차를뜻하지않는다. 모든finiteoutput은고정ceiling20tol을통과했다. 독립계산경로지만같은작성자이며independentpersonnel/native_subagentNOT_RUN이다.

## 비용과 남은 과제

제약674attempts 대 기본354attempts다. 추가213domain rejections와107acceptedsteps가차이를만든다. 단순h/2 및accepted후최대3배증가는재시도를많이만들수있다. Walltime/externalDVODE/Diffsol 비교는수행하지않았고speedup은없다. 채택은endpoint-domain옵션의기능적진전이다.

StageRHS가domain밖에서정의되지않는client는stage-safe후보/typedabort가별도로필요하다. Native integration에는추가구체적경계도있다: 최신integratorlib는forbid(unsafe_code)이지만이연구소비자의상속MXCSR읽기는unsafeasm이다. Domain core 자체는unsafe금지이나이소비자전체를그대로productionmodule에넣을수없다. Safe adapter/dependency경계를검증해야하며금지규칙을약화시키지않는다. 실제BASS/domain단위/기본WRMS/독립심사는열려있다.

## 전달

S0/S1/S2/S3각각backup및expectedSHA게시. S1snapshot에는초기S2구현과GREEN도있어단계가완전히비중첩이라고하지않는다. S2commit8d4c0fe845a55200eeceab469fb428f3602492a2의7sourceblob은실행파일과일치했다. 출판commit을과거실행시점commit으로바꾸지않는다. Git본문은전체ZIP보고서의편집요약이며문서byteidentity주장은없다.

새closedsubprocessbudget21.305609979999986s/cap120;prior20.620188011000437+23.03035017700006+10.984972581999784+7.354545032999567s와olderremainingnull유지. 조사/집필/전송전체시간은이값이아니다. NCP/역사campaignreplay/productionbuild/merge/force/userdirty변경0. Globalpercentnull,productionHOLD. 다음은현재nativeowner/source/완료근거를연결하고실제누락만이식하는일이다.
