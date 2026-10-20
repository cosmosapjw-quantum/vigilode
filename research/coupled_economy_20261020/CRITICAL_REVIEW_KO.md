# Residual-first coupled 보정 및 제한된 후보 시도의 비판 분석

기준일: 2026-10-20 KST. 기준 companion은 `15c43ce76734d0a560fcec5909ac49f1e6f2d347`, 실제 측정 소스는 `f08d152a35fb2db748692570969cb1723c287c26`, 측정 run은 `39355449893`이다. 이후의 문서/보존 커밋을 새 측정 소스로 바꾸어 표시하지 않는다.

## 결론

이전 단계의 다음 과제 중 두 가지를 구현했다. 첫째, projected residual을 먼저 계산하고 실제 Newton 갱신에 필요할 때만 stage derivative를 계산하는 opt-in 경로다. 둘째, 후보의 JVP 작업 상한을 제한한 뒤 기존 순차 matrix-free fallback을 사용하는 명시적 transaction API다. 기존 eager API와 기본 dispatcher는 그대로 유지했다.

잔차 우선 경로는 검증된 성공 사례에서 stage 전체, 출력, embedded vector와 최종 full-space residual을 바꾸지 않고 작업량을 줄였다. 반면 고정 JVP cap64/256을 최적 선택 정책으로 채택할 근거는 없다. 특히 cap64는 성공 가능한 저렴한 chain 후보를 잘라 비싼 fallback으로 보냈으며, 넓은 spectrum 사례에서는 비싼 projected LU 작업을 조기에 차단하지 못했다. 안전한 작업 예산과 경제적으로 최적인 정책은 다르다.

이번 범위는 고정 h의 완전한 coupled root attempt다. Adaptive trajectory, dense output, root/시간오차를 결합한 commit driver, 인증된 residual-to-output bound는 구현했다고 주장하지 않는다.

## 1. 변경된 계산 경로

기존 corrector는 매 iterate에서 H(z,lambda)와 그 Jacobian을 함께 만든 다음 잔차가 이미 수렴했는지 확인했다. 따라서 마지막 미분행렬이 쓰이지 않아도8m개의 current-stage JVP를 실행했다.

새 `coupled_step_residual_first`는 같은 stage 상태와 같은 순서의 잔차 계산을 먼저 수행한다. 수렴하거나 업데이트 한도가 소진되었다면 derivative 요청을 생략한다. 갱신이 필요할 때는 저장한 정확히 같은 stage 상태에서 원래의 미분행렬을 구성하고 같은 small solve/update를 수행한다. 새로운 근사 Jacobian을 쓰거나 homotopy 목표/허용오차를 바꾸는 방식이 아니다.

수학적 근거는 THEORY.md에 있다. 순수하고 결정론적인 callback, 고정된 model epoch, 동일한 부동소수점 환경, 바인딩되지 않은 작업 예산이라는 조건에서 accepted Newton iterate는 귀납적으로 같다. Full-space 검증은 그대로 수행하며, unused derivative를 생략했다고 그 JVP 수를 실제 수행 작업에 포함하지 않는다.

새로운 `coupled_or_sequential_budgeted(..., candidate_jvp_cap)`는 effective cap=min(사용자 기존 cap, 새 cap)만 적용한다. 입력/모델/callback 오류에는 숨은 fallback을 추가하지 않고, 수치/예산 거절은 기존 보호 경로로 처리한다. Fallback 비용은 후보 cap의 적용 대상이 아니고 별도 카운터로 보존된다.

## 2. 실제 TDD 및 테스트

RED run39353982085에서 세 비용 계약이 실제로 실패했다. 수렴한 affine seed에서 stage JVP24회, 업데이트 한도0에서도24회가 쓰였다. 비선형 h0.01의 두 업데이트에서는 필요한48회 대신72회였다. 출력 동일성 대조 하나는 통과했다. 이는 불필요한 계산의 재현이지 기존 수치해가 틀렸다는 판정이 아니다.

실제 구현 후 기존19개와 신규14개, 총33개 Rust 시험이 통과했다. 새14개는 RED계약4개와 경계/실패검사10개다. 작은 caller cap을 새 cap이 확대하지 않는지, cap0에서 candidate JVP 없이 fallback하는지, 실제 derivative dispatch가 batch 예산을 소비하는지, 실패한 RHS/JVP batch의 모든 작업을 합류하고 계수하는지, phase 사이의 epoch 변경, 최종 full-space 검증 유지, theta/worker 변화에서 eager와의 비트 동일성을 검사했다.

h0.01의 few-mode 회귀에서는 eager가 JVP cap64에서 거절되는 반면 잔차 우선은 총59JVP로 같은 유효 후보를 만들었다. 단계 미분48+base basis3+검증8이며, RHS33회와 원래 차원의 검증은 유지했다.

Clippy도 실행했지만 warning-free라고 표현하지 않는다. 기존 상위 numerical crate와 root workspace 전체를 새로 검증한 것은 아니며, companion 자체의 시험이다. 실제 명령과 결과는 runs/39355449893/TESTS.log, CLIPPY.log, BUILD.log에 있다.

## 3. 벤치마크 계약과 출처

10개 문제 설정,8개 arm,80개 accuracy 행이다. Few-mode96/384/12288, chain96/384, diffusion96/384, broad-spectrum64를 포함한다. h와 eps는 PLAN.md 및 MANIFEST.json에 고정했다. 기존 eager1/4, 잔차 우선1/4, 순차MF, 잔차 우선 무제한 transaction, cap64,cap256을 같은 새 binary에서 실행했다. 과거 호스트의 시간은 섞지 않았다.

각 arm/case당8개 fresh process, process마다 warmup1회와 timed3회다. 총640process,1920timed attempt다.8-arm Williams order는 각 위치에 각 arm이 한 번씩 오고56개의 방향성 인접쌍이 각각 한 번 나오는 것을 별도로 계산 확인했다. 통계 단위는1920개의 내부 반복이 아니라8paired block이다.50개 사전 지정 비교에 pointwise와 Bonferroni-family bootstrap 구간을 보존했다.8개 공유호스트 블록만으로 실제 coverage/자기상관/다른 하드웨어 재현성이 인증되는 것은 아니다.

호스트는 AMD EPYC9V74가 노출된 Ubuntu24.04 Microsoft VM,4vCPU였다. 표시 topology는2cores×2threads이므로4독립 physical core 실험이라고 부르지 않는다. Local stage pool1/4를 제어했다. Pool 생성, reference, serialization과 result destruction은 step timer 밖이고, basis/보정/full-space 검증은 안이다. 전체 애플리케이션의 모든 라이브러리 thread를 추적한 hardware profiler 결과는 아니다.

참조는 기존 독립 Python/Rust raw-stage recurrence를 새로 실행하여 교차 확인했다. 이 기준은 연속 ODE의 exact trajectory가 아니라 고정 step의 raw-U stage root다. 경험적 참조 차이를 포함한 output WRMS<=0.1 및 embedded-vector difference WRMS<=0.1을 유지했다.80개 중76개가 해당 root-reference 기준을 통과했고, embedded 시간오차까지 함께 통과한 것은48개였다.

## 4. 동일 출력에서의 성능 개선

단위 ms/고정 h의 전체 root attempt. Process median들의 median이다.

|문제|Eager1|잔차 우선1|Eager4|잔차 우선4|Embedded 시간오차|
|---|---:|---:|---:|---:|---|
|Few-mode n96,eps1,h0.001|0.144918|0.119875|0.166981|0.153661|통과|
|Few-mode n384,eps1,h0.001|0.495930|0.394383|0.352815|0.341368|통과|
|Few-mode n12288,eps1,h0.001|18.789624|13.286156|8.997115|7.737604|통과|
|Chain n384,eps1,h0.001|2.208214|1.519959|1.157966|0.993368|통과|
|Diffusion n96,eps1,h0.001|2.109009|1.595137|1.506539|1.411171|통과|
|Diffusion n384,eps1,h0.001|7.081587|4.818066|3.672809|3.119438|미충족|

큰 few-mode의 paired eager/lazy 비율은1worker에서1.4117, family구간[1.3969,1.4241],4workers에서1.1656,[1.1187,1.1921]였다.1worker의 중앙시간 감소는약29%,4workers는약14%다. 동일 state/stage/embedded bits이므로 이 비교에서 정확도를 희생해 얻은 개선은 아니다.

작은 n96의4worker 비교는 중앙시간만 보면 줄었지만 paired family구간[0.8788,1.1770]이1을 포함한다. 작은 병렬 case의 안정적인 개선을 일반화하지 않는다. 잔차 우선끼리 비교해도 n96은1worker0.119875ms가4workers0.153661ms보다 짧다. 반면 n12288의 잔차 우선1→4workers는 paired1.7196,[1.6546,1.7496]였다. 미분 작업을 줄이면 병렬화할 작업 자체도 줄기 때문에 이전 worker-speedup 배수와 무조건 같을 이유는 없다.

현재 같은-host generic sequential MF는 n12288,h0.001에서7.709782ms로 잔차 우선4의7.737604ms와 비슷했다. 이 차이로 어느 쪽의 보편적 우위를 주장하지 않는다. 외부 Diffsol/DVODE와 가장 강한 다른 MF backend를 새로 측정한 것도 아니다.

## 5. 작업량 감소가 어디서 생겼는가

|사례|전체 JVP eager→lazy|실제로 생략한 stage JVP|RHS 변화|
|---|---:|---:|---|
|Few-mode h0.001,m3|59→35|24|25회 유지|
|Chain h0.001,m8|144→80|64|25회 유지|
|Diffusion h0.001,m8→12|348→188|160|49회 유지|
|Broad-spectrum 거절,m8→24|704→64|640|81회 유지|

뉴턴 갱신 수, 원래 공간검증, 출력은 바뀌지 않는다. RHS 상태를 저장하므로 derivative단계에서 같은 상태의 RHS를 다시 호출하지 않는다. 다만 필요한 갱신에서는 별도의8-task derivative dispatch/barrier가 생기고 저장 상태를 더 오래 보관한다. 따라서 JVP 감소율을 wall-time 또는 peak-memory 감소율로 읽으면 안 된다. Allocation/peakRSS를 새로 계측한 것은 아니다.

Work에 derivative_batches,residual_only_stops,unused_stage_jvp_avoided가 추가됐다. 실제 stage task 식은8*(stage_batches+derivative_batches+validation_batches)이다. avoided값은 수행한 작업이 아니라 생략된 eager 작업의 대조 계수다. 엄격한 구형 JSON consumer는 새 counter필드/식에 맞추어 갱신해야 한다.

## 6. 제한된 후보 예산의 부정 결과

Cap은 명세대로 작동했지만 최적 경제성 정책은 아니었다. Chain n384,h0.001은 잔차 우선 transaction이 총80JVP와1.002387ms로 끝났다. cap64는 basis8JVP를 쓴 뒤 다음64-JVP batch를 예산 안에서 실행할 수 없어 후보를 거절했고, fallback까지10.754173ms가 들었다. 후보가 수학적으로 수렴 불가능해서 거절된 것이 아니라 사용자가 지정한 budget때문이다. 이 비교는 coupled raw-U와 fallback native-K 사이의 reference-threshold 비교이지 bit-identical-method 속도비가 아니다.

Diffusion n96,h0.001도 cap64가 유용한 후보를 거절하여1.425858ms 대신6.181441ms가 들었다. Cap256은 이 사례의188JVP를 허용하지만, h0.01에서는244JVP를 이미 쓴 뒤 나머지 작업 전에 거절되어 fallback포함7.731703ms, 무제한 transaction은4.395797ms였다. 어느 cap도 모든 case에서 유리하지 않았다.

더 중요한 반례는 broad-spectrum이다. 이 affine case에서는 잔차 우선이 모든 사용하지 않는 stage derivative640회를 제거했다. 남은 JVP는basis24+원래 공간검증40=64회여서 cap64와cap256둘 다 조기 차단하지 않았다. 비싼 작은 projected 시스템의 반복적인 구성/factorization은 JVP상한으로 제한되지 않는다. 거절된4worker후보는4.674370→3.767347ms로 저렴해졌지만 여전히 실패한다. 무제한 transaction은8.861882ms, cap64는8.831947ms, 직접 순차MF는4.874071ms였다. 후보+fallback 손해는 남는다.

따라서 고정 JVP cap을 기본 최적 router로 승격하지 않는다. API는 명시적인 자원제약을 제공한다는 의미에서 완료됐고, 비용 예측에는 basis차원/작은시스템 비용, residual leakage 감소와 필요한 추가 작업까지 들어가야 한다. 이는 후속 개발 대상이며 이번에 임계값을 결과에 맞춰 재조정하지 않았다.

## 7. 정확도 및 검산

27개의 성공한 비교에서 전체 stage,출력,embedded bits와 최종full-space residual이 일치했다.18개 eager→lazy대조에서는 실제JVP감소와 avoided카운터가 정확히 같았다. 모든640개 시간 프로세스는 유효한 시간 표본을 보존했으며,608개만 root-reference승인 표본이다.나머지32개는 넓은 spectrum의4개후보arm×8blocks로, 실패비용으로만 비교한다.

독립 scalar-fsum metric경로로684개 완료관측의 오차,630개작업장부,270개후보cap,608개timing/calibration 연결,50개paired median을 확인했다.71개내용주소화 상태객체를 검증했다.4accuracy거절+32timing거절=36개실패기록의 metric은null로 유지됐다.50개비교 중3개는 failed-attempt latency이므로 solver speedup으로 부르지 않는다.

이 독립성은 서로 다른 계산 경로이며 같은 작성자다. 독립인원/서브에이전트 review를 했다는 뜻이 아니다. 이번 요청에는 CodeRabbit재인증을 포함하지 않았고 이전 미인증 상태를 리뷰PASS로 바꾸지 않았다. 기존19시험을 포함해33개companion시험을 새로 실행했지만 rootworkspace 전체검증은 하지 않았다.

## 종료 판단

완료: opt-in residual-first correction, 동일 snapshot 재사용, 단계별 병렬 work계수/실패/epoch검사, 후보JVP상한과 기존fallback결합,33tests,80accuracy/640process benchmark,원자료검산,push.

미해결: 경제적으로 최적의 자동 선택, 전체 adaptive trajectory와accepted시간전진당비용, 인증된 root/시간/dense오차 계약, 전체workspace,다중호스트 성능,독립인원 review. 이 노드는 그 범위를 몰래 확대하지 않고 종료한다.
