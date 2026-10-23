# V1q: 정수만으로 판정하는 full33 물리 오차 norm

2026-10-24 KST. PR81 다음의 새로운 산술 연구 단위다. 새 Rust 구현·exact oracle·FP-mode 대조·제한된 비용 측정을 실행했으며, native RODAS owner hook이나 기존 물리/NCP 캠페인은 변경·재실행하지 않았다. 전체 상세 보고서와 raw는 FINAL_DELIVERY.json의66-payload ZIP에 있다. 이 Git 문서는 같은 결과를 요약한 출판본이며 ZIP 보고서와 byte identity를 주장하지 않는다.

## 1. 조사와 선택

원격 matching-refs의 브랜치 metadata를 읽고 관련 최신 소스·PR·대화 맥락·백업을 선택적으로 대조했다. 모든 branch 이름/tip의 조회를 모든 코드·모든 대화의 전수 심사로 표현하지 않는다.

새 general-ode-selection 소스14171ed0c4f7dc453f47870cf6c1399f41afc4f2에는 보호된 GeneralOde 선택, callback 완전성 검사, scale-overflow 거절, persistent DOPRI5와 schedule 검색이 이미 있었다. 해당 작성자의 결과는 재사용했고 새 실행으로 합산하지 않았다. 이는 native V1 full33 hook 완료의 근거와는 별개다. 관련 VigilODE79–81, REI220–222, BASS182–184의 서로 다른 물리/수신 승인 범위도 구분했다.

Dropbox VIGILODE_V1 검색은11개에서 pagination이 끝났다. Drive 날짜조건 검색은0건이었으나 직접 지정한 기존S3 metadata818341bytes는 확인됐다. 검색0건을 백업 부재로 처리하지 않았다. PR81에는 새owner완료 comment가 없었다. 사용자 로컬quota를 우회하거나 dirty checkout을 변경하지 않았으며 현재owner예약은 미확정이다. Parent root/research AGENTS 조회404는 그 pin/경로의 결과일 뿐 다른 위치의 지침부재를 뜻하지 않는다.

따라서 native hook을 중복 구현하지 않고 V1q-integer-admission을 선택했다. V1p의 FP-mode 전제와 경계 미결정에 대해, 입력 bit와 정수연산만 사용하는 별도 수락 판정기를 만든다. 이 목적은 전체 solver를 derivative-free로 만드는 연구와도 구분한다.

## 2. 이론과 계약

고정 affine복원 y=Lz+b에서 e_full=Le다. Caller가 공급한 양의 finite 물리scale33개로 E²=(1/33)sum_i((Le)_i/s_i)²를 계산한다. PR79 fixed±1map의 dependent rows23/25/28은 각각-e23,-(e24+e25),-(e26+e27+e28)이고 나머지는 복사다. Offset을 error에 더하지 않으며 분모33과 상관항을 보존한다. 각 ratio는 무차원이다.

Finite binary64는 정수×2^-1074로 해석된다. <=3개의 signed inputs를 정확히 누적한 magnitude A<2^2100, scale정수B>0를 얻는다. 모든 입력을 먼저 검증한다. 어떤 row가A>=8B이면 E²>=64/33>1이므로 LargeRow 거절증거를 돌려준다. 이는 전체 norm upper bound가 아니다.

그 외 p=56, q=floor(2^56*A/B), u=q+[remainder!=0], L_int=sum(q²), U_int=sum(u²), T=33*2^112로 두면 L_int/T<=E²<=U_int/T다. U_int<=T이면Accept, L_int>T이면Reject, 나머지는Indeterminate다. A*2^56은34u64words에 들어가고 q,u<=2^59 및33*2^118<2^128이어서 squared sum은u128에 담긴다. Wolfram이 용량·64/33·width identity·exactboundary를 확인했다.

Absolute E² enclosure width는16*2^-56+2^-112보다 작다. 이는 tiny norm의 relative bound가 아니다. Exact division일 때q=u이므로E²=1을 정확히 승인할 수 있지만, 경계+tiny양수처럼 구간이1을 가로지르면 보류해야 한다. Threshold를 느슨하게 하지 않는다.

Exact superaccumulation 자체는 기존 기법이다(R.M.Neal2015, arXiv1505.05571). 새 일반 summation 정리나 novelty를 주장하지 않는다. 산술 증명은 고정 lift와 supplied binary64값에 대한 것이며 scale구성·embedded estimator의 타당성·실제globalODE오차는 포함하지 않는다.

## 3. 실제 구현과 비용을 줄인 정확한 재표현

`evaluate(&[u64;30], &[u64;33])`의 입력은binary64encodings다. Core는unsafe금지, nofloat/noheap/no callback/noJacobian/JVP다. Certificate는 숫자증거이며 봉인된 driver-commit token은 아니다.

첫 구현은 정확한 큰정수 긴나눗셈으로 작동했으나 매우 느렸다. 이를 scale B=M*2^k, M<=53bits로 분해하여

    floor(N/(M*2^k)) = floor((N>>k)/M), N=A*2^56

로 바꾸었다. Ratio<8이므로N>>k<2^112이며u128나눗셈이면 충분하다. 버린lowbits와 integer remainder를 함께 검사하여 upperendpoint를 결정한다. 이는 정확한 항등변환이고 precision/target을 바꾸지 않는다. 최초소스와실패/비용을 남겼으며 새681출력은 바이트까지 동일하다.

## 4. 검증 결과

RED는14시험 중12feature실패와2control통과를 실제 기록했다. 초기판과 최적화판은 각각14GREEN이다. Unique test14이지28이 아니다. Final128kernelcalls의 heap요청0이며 parser/caller/solver전체의 무할당 주장은 아니다.

새 corpus681개: priorambiguous6 + wide-random256 + controlled-scale256 + near-sqrt33 129 + exactboundary1 + boundary+minsub1 + cancellation32. 원래741전체를 재실행하지 않았다. Fraction대조 결과Accept343/Reject337/Indeterminate1, falseAccept0/falseReject0이다. 기존미결정6개는3Accept/3Reject로 해소됐다. ExactE²=1은lo=hi=T로Accept; E²=1+tiny²/33은Indeterminate로 남겼다. 모든 finite입력에서 반드시 이진판정하는 exactrationalcomparator는 아니다.

격리된 child process에서4rounding×FTZ2×DAZ2=16MXCSR조합을 설정하고 복구했다. Final681×16=10896certificate/decision비교가 모두 같았고 원래0x1f80으로 복구됐다. 같은실험의초기판/최적화판회귀를 새uniquephysicalcase로 합산하지 않았다. FinalLLVM모듈에floating arithmetic/conversion/compareopcode0개, core float타입0개를 확인했다. 이는 해당compiler/x86_64실험이며 임의platform/compiler의 형식검증은 아니다.

Mode-test첫link는 실제생성libffi.so와 지정basename불일치로 실패했다. 해당link와그다음실행없음 기록을 보존한 채 실제출력명으로수정했다. Numerical target변경은 없었다.

중요한 경계: 입력bit가 이미 upstream FP연산에서 손상됐다면 고치지 않는다. Predicate의FP-mode독립성과 전체nativeODE의FP환경권한은 다르다. Scale builder, accepted-statecommit 및 next-h proposal도 별도다.

## 5. 비용: 최적화는 유효하지만 기본 승격은 하지 않음

같은최종바이너리에서oldFP enclosure+decision, optimizedinteger+decision, preservedlongdivision+decision을 비교했다. Inputconversion은timer밖이다.3case×6block×3arm=54cells,300회/cell=16200timedcalls이며warmup20회/cell은별도다. 초기측정10800회를더한총호출27000은장부에만보존하고통계표본을합치지않았다.

|입력|기존FP μs|최적화정수 μs|최초정수 μs|
|---|---:|---:|---:|
|보통크기|0.82157|3.83366|75.61643|
|큰양음항상쇄|0.84260|4.06923|57.74463|
|원시복원overflow|0.92421|3.88128|57.19263|

Paired 최초/최적화는20.0110,14.2510,14.8726으로 자체구현내14–20배개선이다. 그러나 최적화/oldFP는4.7722,5.0455,4.2084로 더느리다. Case0oldFP의 큰지연outlier도 삭제하지않았고 paired범위0.3456–5.2307을 보존했다. 단일process공유호스트의진단이며 독립processCI·보편적순위·whole-solver speedup은 아니다. 계산정밀도와FP환경의무도 다르다.

채택은 선택형 robustpredicate 또는 경계 fallback연구후보다. Default교체·optimalrouter주장은 하지않는다. Acceptance결정과 controller의h제안에 필요한숫자estimate는 같은출력이 아니다.

## 6. 별도 검산·DAG·누적 예산

최종 Pythonarbitraryinteger/Euclideandivision 경로가681저장certificate를 직접재구성하고 초기/최적화byteidentity와budgetlogs를검사했다. REVIEW.json=PASS_SEPARATE_INTEGER_CERTIFICATE_READBACK. Fractionvalue포함검사와Rustfixed-word구현과는 다른계산경로지만 같은작성자이며 독립인원/native-subagent는NOT_RUN이다.

S0조사/계약→S1이론/RED→S2구현/검증/비용→S3검산/문서/게시의4단계다. S0,S1,S2,S3모두별도backup을만들었으며 최종게시후local4/4,globalpercentnull이다. NativeV1hook/actualBASS/defaultWRMS/scale/controller독립review/production은열려있다. V3기존PASS와NCP는재실행하지않았다.

Closednewsubprocessbudget은23.030350177초(cap120초)로compiler복원·오류·repair·검산을포함한다. 조사/집필/전송전체시간이아니다. PriorV1p10.984972582초,PR807.354545033초,olderremainingnull을초기화하지않았다. 새ODE/IVP/NCP/cloudcampaign0이다.

다음첫단계는native-affine담당자의정확한source/예약/dirty상태/완료근거를연결하는것이다. 이미있는hook은재사용하고 실제빠진연결만시행한다. 정확한raw와모든코드/실패/재생성절차는FINAL_DELIVERY의S3ZIP에있다. Compiler/binaries/.so/.ll은재생성가능해제외했다. 원래S2시험소스의Gitblob은3cba0e3c8bff1ce104379e69a1233807ccc0af99이며S2게시commit을과거execution-timecommit으로재명명하지않는다.
