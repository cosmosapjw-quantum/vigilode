# 공유 부분공간·병렬 stage·homotopy의 Rust 구현 및 비판 분석

기준일:2026-10-20KST. 원자료는 run39349031422, 고정 결과 검산은 closeout39349959195에 보존했다.

## 1. 산출물과 정확한 범위

새 standalone companion library `crates/vigilode-coupled`에 실제로 실행 가능한 native coupled step을 구현했다. 원본 보고서의 첫 native 단계인 autonomous identity-mass ODE용 한 step이며, 완성된 adaptive trajectory dispatcher가 아니다. root workspace의 기존 numerical crates, 기본 경로, PR71·PR73은 변경하지 않았다.

원본 ZIP `VigilODE_Integration_Research_20261020.zip`의 SHA256은 `54f4f4a99d552388fc28d01f5f367176e301c2af15c3cd8a9c7a30b4e5dfd4f1`이다. 첨부 Python13개 시험을 이 세션에서 재실행해 통과했다. 저장소 numerical base는 `c03ca90f19a9e64ba7751c069325a48b54b39c2e`, 실제 Rust benchmark source는 `1e4a993874d6930ea2bb41bb0ff7392f21c33df7`이다. 이후 문서·review·receipt commit을 새로운 측정 소스로 바꾸어 부르지 않는다.

구현은 공급된 JVP로 basis와 J0V를 만들고, 작은 전체 stage 방정식을 풀며, 같은 iterate에서8개 stage의 RHS/current-state JVP를 Rayon local pool로 평가한다. 원래 차원의 residual로 재검사하고 부족하면 basis를 확장한다. root 보정이 실패하면 lambda구간을 이분하고, 후보가 budget/수치 조건으로 거절되면 호출자가 명시적으로 선택한 기존 sequential JVP-only fallback을 쓸 수 있다.

이는 Jacobian을 원래 상태 좌표의 큰 행렬로 구성하지 않는 의미의 matrix-free이다. 일반적인 RHS-only finite-difference JFNK나 state Jacobian이 전혀 필요 없는 수학적 방법은 아니다. 작은 projected dense matrix와 stage derivative JVP는 실제로 계산하고 그 비용을 센다.

## 2. Stage target과 승인 의미

raw-U target은

`F_i = U_i - h*gamma*J0*U_i - gamma*sum(C_ij*U_j) - h*gamma*f(y+sum(A_ij*U_j)) = 0`.

반복의 중심은 전체 projected stage vector이며, 하나의 stage가 다른 worker의 갱신된 stage를 즉시 소비하지 않는다. 따라서 큰 공간의8sequential-W-solve를 실제로 대체한다. Basis 생성, reduced LU, continuation step, 출력 가중합에는 순차 작업이 남아 있으며 이를 병렬 speedup에서 숨기지 않았다.

후보는 projected residual만으로 승인하지 않는다. 공급된 원래 operator와 원래 차원의 RHS로 full residual을 계산한다. 하지만 그 residual gate도 nonlinear output enclosure가 아니다. 작은 inverse gain/비선형 remainder의 검증된 상계가 없는 한 작은 residual은 작은 출력오차를 보증하지 않는다.

중요하게, `candidate.is_some()`는 물리적 시간전진 승인과 다르다. `embedded_test_passes`를 별도로 반환한다. 이번80개 조건 중 root/reference gate는77개가 통과했지만, root와 embedded time-error test를 함께 통과한 것은20개뿐이다. 이는 사전에 고정한 큰 h의 stress cases가 다수 포함됐기 때문이다. 이 결과를 성공한 adaptive trajectory80개 또는 보장된 global tolerance로 읽으면 안 된다. [세부표](closeout_runs/39349959195/ROOT_VS_TIME_ERROR.csv)

## 3. 실제 시험

RED run39345503351에서 fail-closed scaffold는3개 candidate-completion 시험에 실제로 실패했고2개 거부 대조는 통과했다. 이후 실제 구현으로 계약5개, robustness12개, 추가 convergence/accounting2개, 총19개가 통과했다. 이는 new companion crate에 대한 실행 결과다. Root workspace 전체 시험은 수행하지 않았으며 companion이 자체 `[workspace]`를 가지므로 root `cargo test --workspace`만으로 자동 실행되지 않는다.

검사에는 explicit Jacobian callback의 의도된 panic이 호출되지 않는지, thread1/4비트 동일성, equilibrium 처리, 작은 projected residual만으로 broad-spectrum을 잘못 승인하지 않는지, residual 방향 basis확장, model epoch, 실패한 병렬 작업 전부의 attempted/completed 카운터, NaN으로 초기화된 미완성 JVP 출력, budget/shape/time/config, projected singularity, lambda끝점, 실패 fallback의 structured work가 포함된다.

정확 Riccati 해의 한 step 오차 refinement도 실행했다. h={0.2,0.1,0.05,0.025}에서 eps0.5의 최대 절대오차는 `{6.3721e-8,1.9240e-9,4.2504e-11,7.9266e-13}`로 감소했다. 이는 실행된 local-refinement 검산이며 네 점만으로 일반적5차/global-order theorem을 증명한 것은 아니다.

`cargo clippy --all-targets`도 실행했으나 warning-free라고 주장하지 않는다. 경고 로그를 보존했다. 수치 builder/default를 변경한 결과와 단순 formatting은 별도 commit으로 남겼고, 실제 측정은 formatting commit을 게시한 뒤 수행했다.

## 4. 벤치마크 계약

16개 문제 설정,5arms,80accuracy cells이다. few-mode n96/384/12288, 넓은 spectrum n64, chain·periodic diffusion n96/384를 포함한다. Epsilon과 h는 BENCHMARK_PREREGISTRATION.md와 MANIFEST.json에 고정했다. 후보 입력 scale은 atol1e-10,rtol1e-6이다.

독립 Python raw-stage recurrence와 별도 Rust raw reference를 교차 확인했다. 작은 비대각 reference만 untimed dense matrix를 조립한다. Candidate와 sequential baseline에는 그 행렬이나 참조 상태를 전달하지 않는다. Reference mismatch의 경험적 폭을 더한 output WRMS<=0.1,embedded-vector difference WRMS<=0.1을 고정-step 비교조건으로 삼았다. 이는 nonlinear root 정확도 예산의 공통 상한을 적용한 것이며, 서로 실제 달성 root 오차가 정확히 같은 조건을 의미하지 않는다. 특히 native-K baseline과 raw-U candidate는 target 좌표/반올림도 다르다.

800fresh-process timing units,3700내부 timed step attempts,discarded warmup1,10balanced paired blocks를 실행했다. Pool 생성,reference,JSON,state검사,result drop은 timed region 밖이고 basis/root/full-residual검사는 안이다. Pool 생성 시간은 별도 field에 남는다. 호스트는4vCPU AMD EPYC9V45 Microsoft VM,Ubuntu24.04,Rust1.94.1/LLVM21.1.8. Thread count별 실제 겹치는 stage tasks도 관측했다. 결과는 공유 runner의 조건부 진단이며 다른 머신의 speedup으로 일반화하지 않는다.

64개 사전 비교 중61개가 완전한 admissible paired sample을 가졌다.3개는 broad-spectrum coupled후보 실패로 HOLD다. Bootstrap의 표본 단위는 개별3700회가 아닌10paired blocks이고,pointwise와64비교의 Bonferroni-family 구간을 저장했다. 공유호스트의 자기상관과10block coverage는 엄밀히 증명되지 않았다.

## 5. 성능과 정확도 결과

단위 ms/고정 h의 전체 root attempt. 각 값은 process median들의 median이다.

|문제|Coupled1|Coupled2|Coupled4|기존 sequential MF|embedded 시간오차 통과|
|---|---:|---:|---:|---:|---|
|few-mode n96,eps1,h0.001|0.094713|0.132530|0.104152|0.088063|예|
|few-mode n12288,eps1,h0.001|12.846546|6.587397|6.172241|5.532020|예|
|few-mode n12288,eps1,h0.01|16.449693|8.695530|7.859994|5.555025|아니오|
|chain n384,eps1,h0.01|1.882242|1.286772|0.984021|6.710258|아니오|
|diffusion n96,eps1,h0.001|1.284279|1.090340|0.972373|3.926156|예|
|diffusion n384,eps1,h0.01|12.964167|8.638934|6.527793|6.841987|아니오|

### 실제 병렬 효과와 algorithm 효과를 분리

few-mode n12288,h0.001의 같은 coupled 알고리즘은1→4workers에서 paired ratio2.0765, family구간[2.0202,2.1531]이었다. h0.01은2.0895,[2.0497,2.2425]였다. 결과 비트는 동일하므로 이 비교는 정확도를 바꿔 얻은 worker-speedup이 아니다. 반면 n96,h0.001은1→4ratio0.9055로 더 느렸다. 작은 callback에서는 scheduling/dispatch 비용을 회수하지 못한다.

하지만 큰 few-mode 사례에서도 기존 sequential MF가 더 짧았다. 예컨대 n12288,h0.001은5.53ms 대6.17ms다. 따라서 'native parallelism을 실제로 달성했다'와 '기존 solver보다 빠르다'는 구분해야 한다. n12288,eps1,h0.01의 shared basis차원3, reduced system24, Newton2updates이지만 실제 RHS33/JVP83이 필요했다. Basis3JVP만을 총작업으로 주장하지 않는다. 그83JVP에는 stage derivative와 original-space검사가 포함된다.

chain/diffusion에서는 공유부분공간이 반복적인 sequential Krylov풀이보다 경제적인 사례를 보였다. diffusion n96,h0.001은 baseline/coupled4 paired ratio3.9874,[3.1984,4.0927]이고 embeddedtest도 통과한다. 다만 비교한 baseline은 기존 generic sequential GMRES설정(rtol1e-12 등)이며 최적화된 모든 matrix-free backend 또는 strongest external solver를 대표하지 않는다. Root 실제 오차도 다르다. 이것을4배 더 빠른 완성형 적분기라고 부르지 않는다.

chain n384에서는6.8196배의 큰 root-latency 차이가 있지만 해당 h의 embedded WRMS는약49.90으로 둘 다 물리적 step을 거절해야 한다. 이 숫자는 root solve비용의 후보성만 보여 준다. 실제 trajectory이득은 더 작은 h와 adaptive driver에서 별도로 검증해야 한다.

### 실패와 fallback의 비용

Broad-spectrum64에서는 coupled의 세 worker설정이 모두 projection-leakage로 거절됐다.4회의 basis확장 이후에도 원래 차원 residual조건을 만족하지 못했다. 실패한 coupled4비용은3.105ms, 직접 sequential은3.301ms, coupled후 sequential fallback은6.617ms였다. 실패한 후보를 제외한 speedup을 만들지 않았다. '항상 먼저 coupled를 시도한다'는 정책은 이 negative control에서 약2배 비용을 발생시키므로 지지되지 않는다.

또한 모든 성공한 benchmark에서는 lambda-subdivision없이 direct lambda1corrector가 충분했다. Subdivision의 구현·실패예산은 테스트했지만 어려운 실제물리경로에서 continuation이 이득을 내는 사례를 이번 timing으로 증명하지는 않았다. Homotopy기능이 있다는 것과 positive production-use evidence가 있다는 것은 다르다.

## 6. 검산과 보존

독립 scalar-fsum 경로로847개 finite complete output metric을 재계산했고,33개 declined observation(accuracy3+timing30)을 null metric으로 유지했다.528개 coupled기록의 callback/work accounting,770개 admitted timing/calibration identity,30개 worker-count 비트비교,61paired median을 확인했다.82개 content-addressed state object의 SHA-256도 검증했다. compressed states는931878bytes였다.

고정 closeout은112개 archived 파일의 SHA를 확인하고16MiB이상 reference파일1개를 분할자료로 재구성했다. 원자료 자체나 숫자·시간선택 기준은 바꾸지 않았다. `RECONSTRUCTION.json`과 `SHA256SUMS`를 먼저 적용하면 읽기 검산을 재실행할 수 있다. 재생성 가능한 compiler/cache/binary는 push하지 않았다. Full raw timing samples는 재실행으로 동일하게 만들 수 없으므로 보존했다.

독립성은 별도의 계산경로를 뜻하며 동일 agent의 작업이다. 독립인원·native subagent review를 실행했다고 주장하지 않는다. 검산의 PASS가 그 자체로 수치해석 정리나 global enclosure를 인증하지 않는다.

## 7. CodeRabbit 및 실행 실패

요청한 CodeRabbit은 설치·인증 절차를 실제 시도했다. 로컬에서는 CLI미설치 및 DNS오류가 있었다. Runner에서는 CLI설치가 성공했지만 auth상태를 의미론적으로 확인한 결과 미인증이었고, `coderabbit auth login --agent`가 exit1로 실패했다. `auth status` exit0만으로 로그인 성공을 추정하지 않았다. 최종 상태는 `BLOCKED_AUTHENTICATION_NOT_CONFIRMED`, review_executed=false, issues=null이다. PR74에 `@coderabbitai full review`도 요청했지만 확인 시점에 bot리뷰 결과는 없었다. 이를 'CodeRabbit0issues'나 완료된 코드리뷰로 포장하지 않는다.

인증된 환경에서 다음 명령이 필요하다: `coderabbit auth login --agent`, 그 다음 `coderabbit review --agent -t committed --base-commit c03ca90f19a9e64ba7751c069325a48b54b39c2e`. 인증 payload나 토큰은 공개 결과에 저장하지 않았다. 수치 readback은 이 외부 리뷰를 대체한다고 주장하지 않는다.

별도 CI run39348756394는 numerical code실패가 아니라 workflow편집을 포함한 push가 contents-only token에서 거부된 사건이다. 다음 실행은 workflow를 수정하지 않도록 고쳐 정상 실행했다. 실패한 원래 run/log는 보존했고 잘못된 privileged token 우회는 하지 않았다.

## 8. 종료 판단과 다음 native 단계

완료: JVP-only shared basis, all-at-once reduced stage target, actual stage parallelism, bounded homotopy/root correction, full-space residual screen, explicit counted fallback,19tests,80/800benchmark+원자료검산,push.

미완료 또는 미주장: native adaptive trajectory/controller·history reuse·dense output통합, residual→output certified budget, mass matrix/nonautonomous/DAE, RHS-only JFNK, cost-aware automatic selection, strongest MF backend재대조, CodeRabbit외부리뷰, 전체workspace suite. 기존 기본solver는 바꾸지 않았다.

다음 우선순위는 기능 수를 늘리는 것이 아니라, (a) basis성장·후보실패를 빨리 감지하는 경제성 gate, (b) 이미 수렴한 projected iterate에서 불필요한 stage derivative 재평가 감소, (c) root/embedded/dense 오차를 분리한 adaptive commit/rollbackdriver, (d) m증가와 callback비용에 따른 worker선택이다. 이들은 이번 실측으로 지지되는 후속 목표이며 아직 구현했다는 뜻은 아니다.
