# VigilODE / Netlib DVODE / Diffsol: 엄격 비교 최종 비판 분석

기준일: 2026-10-19 KST. 이 문서는 실제 CI 실행과 저장된 관측량의 읽기 검산에 근거한다. 요약에서 유도한 새로운 실측을 만들지 않았다.

## 결론

동일 실행환경에서 실제 달성 오차를 2배 이내로 맞추면, Robertson·HIRES·van der Pol의 선택된 설정에서는 VigilODE fast가 비교한 두 BDF 구성보다 짧은 시간을 보였다. 반대로 Brusselator에서는 VigilODE에도 banded-slices 경로를 적용했지만 Netlib 원본 DVODE의 banded BDF가 약 1.77~2.19배 짧았다. 이는 특정 문제·출력 계약·backend·오차 범위·호스트에서의 관측이며, 세 알고리즘의 보편적 순위가 아니다.

기존 비교의 큰 VODE step 수를 원본 Fortran 알고리즘의 열세로 해석하는 주장은 철회한다. 이전 실행은 SciPy 1.17.0의 C 이식판이었다. dense Jacobian 전치 버그가 upstream에서 확인되어 있으며, 본 작업에서 동일 증상과 추가 dense-path 이상을 재현했다. 새로운 주 비교에는 그 wrapper가 아닌 Netlib 원본 Fortran을 사용했다.

최종 수치 읽기 검산은 **PASS_SCOPED_READBACK_WITH_INITIAL_ROUNDING**이다. 최초 auditor의 초기값 비트 동일성 FAIL은 그대로 보존한다. 전체 workspace 회귀시험, 연속 시간 인증, 생산 배포 승격은 주장하지 않는다.

## 1. 실행 정체성과 근거

|구분|식별자|상태|
|---|---|---|
|생산 numerical source|a21d573980f66fe02891bd216153443b5352f921|crates와 root Cargo 파일 무변경 검사를 CI에서 수행|
|주 스크리닝 측정 소스|87df9b071c3a3a7828e999e553a088af5860d274|실제 빌드·실행|
|주 스크리닝 실행|39257390707|756 accuracy cells, 1152 timed processes|
|달성 오차 정합 측정 소스|9c6a6d9145795e35b978652a567b806ddc84a6bd|실제 빌드·실행|
|달성 오차 정합 실행|39261895983|270 calibration solves, 408 timed processes, 4896 timed integrations|
|후처리 소스|65ef8cde8b3c6fdbfef4d06b52c63df673467651|새 적분·시간 측정 없음|
|최종 읽기 검산 실행|39263860973|678 rows, 189 state objects, 42 paired medians 검산|

주요 원자료: [정합 실행](refined_runs/39261895983), [검산·표·그림](readback_runs/39263860973), [최종 audit](readback_runs/39263860973/AUDIT.json), [raw→metric→selection→timing 연결 검사](refined_runs/39261895983/LINK_AUDIT.json).

정합 실행 호스트는 AMD EPYC 9V74가 노출된 Microsoft VM, Ubuntu 24.04, 4 vCPU였다. Rust/Cargo 1.94.1, LLVM 21.1.8, GNU Fortran 13.3.0을 사용했다. Rust measurement profile은 opt-level 3, LTO off, CGU 1이고 Fortran은 -O3이다. CPU affinity와 OMP/BLAS/Rayon thread=1을 고정했다. 전용 호스트나 governor의 안정성이 인증되지 않았으므로 wall-clock authority는 diagnostic이다. 주 스크리닝의 EPYC 9V45 호스트와 정합 실행의 시간을 섞지 않았다.

## 2. 비교 계약과 재현성

세 solver는 공통 Rust RHS와 해석 Jacobian entry 함수를 호출한다. DVODE는 ISO_C_BINDING bridge를 통해 그 함수를 사용한다. Python 함수 호출 비용은 native timing에 포함되지 않는다. 라이브러리 API dispatch·FFI·workspace 초기화·결과 저장 비용은 각 구성의 실제 비용에 포함된다. 이 때문에 순수한 이론적 stepper 연산량만의 비교는 아니다.

VigilODE는 fast 및 선택된 공간 문제의 banded-slices, Diffsol은 0.16.1 BDF/FaerLU dense, 원본 DVODE는 작은 문제의 MF21 analytical dense 및 Brusselator의 MF24 analytical banded이다. Diffsol의 다른 sparse/matrix backend는 이번 결과로 판정할 수 없다.

|문제|상태 차원|구간|s: atol/rtol|
|---|---:|---|---:|
|Robertson|3|[0,40]|1e-4|
|HIRES|8|[0,321.8122]|1e-4|
|van der Pol, mu=1000|2|[0,2000]|1|
|Brusselator 50 cells|100|[0,10]|1|
|Brusselator 200 cells|400|[0,10]|1|

주 스크리닝은 추가 정확해 pivot-linear 사례도 포함했다. 모든 사례는 autonomous identity-mass ODE이며, 질량행렬·DAE·실제 우주론 client의 성능으로 확대하지 않는다. 출력은 endpoint와 uniform101이며 h0=1e-6×{0.5,1,2}. VigilODE는 출력 시각에 step을 맞추고, Diffsol/DVODE는 dense interpolation을 사용할 수 있다. 필요한 출력은 같지만 내부 알고리즘은 같지 않다.

주 스크리닝의 rtol은 1e-4부터 1e-10까지다. 독립 참조해는 NumPy의 별도 RHS/Jacobian 구현과 complex-step JVP parity를 거쳐 SciPy 1.16.3 Radau tolerance ladder 및 LSODA 교차 계산으로 얻었다. 별도 symbolic Wolfram 계산으로 Robertson의 보존식/Jacobian과 비대칭 pivot fixture의 정확해를 확인했다. 후속 Wolfram 고정밀 Robertson 수치 요청은 502 오류였고 사용하지 않았다.

## 3. 오차 의미와 달성 정확도 정합

각 출력 시각의 가중 RMS를 사용한다. E_raw는 (y-y_ref)/(s+|y_ref|)의 WRMS를 출력 격자에서 최대화한 값이다. E_reference는 경험적 참조 폭 rho를 같은 분모로 평가한 값이다. 저장 지표 E_up=E_raw+E_reference이며, R_up는 atol+rtol|y_ref|를 분모로 사용한다. 여기서는 atol=s*rtol이므로 **R_up=E_up/rtol**이 성립하고 이를 별도 구현으로 재검산했다.

rho는 Radau ladder 차이 및 Radau–LSODA 차이의 두 배를 이용한다. 두 참조 코드의 일치는 공통 오차를 배제하지 못한다. 따라서 E_up라는 필드명을 엄밀한 certified upper bound로 읽으면 안 된다. 성분별 실제 참조 오차가 rho 이하라는 추가 전제가 있을 때만 triangle inequality에 의한 상계 논리가 적용된다.

또한 **R_up>1을 local-tolerance solver의 명세 위반으로 자동 분류하지 않는다.** DVODE 원문은 실제 global error가 local tolerances를 초과할 수 있다고 명시한다. 여기서 R_up는 우리 연구의 요청 출력/global target 충족 여부를 평가하는 독립 관측량이다.

정합 캠페인은 먼저 모든 solver에 E_up<=1e-6을 요구하고, 그 통과 설정의 최악 h0 오차 중 최소값을 case/grid별 anchor로 정했다. 각 arm의 rtol을 bounded log bisection으로 조정하여 모든 h0에서 E_up<=anchor, nominal h0에서 anchor/2<=E_up<=anchor, E_reference<=0.1 anchor를 동시에 만족시켰다. 시간값은 이 선택에 쓰지 않았다. 10 case/grid 모두 모든 적용 가능한 arm의 정합에 성공했다. 실제 E_nominal은 약 8.18e-8~4.66e-7이다.

정합은 정확히 같은 오차가 아니라 최대 2배 범위라는 점도 남는다. 특히 수 %의 시간차를 해당 오차 차이와 무관한 알고리즘 우위로 보지 않는다. 출력 시각 사이의 sup error, 급격한 van der Pol transition의 시각/위상 오차는 이번 101점 지표만으로 보증되지 않는다.

## 4. 달성 오차 정합 후 실측 시간

단위 ms/적분 호출. 12개 fresh process에서 각각 warmup 1회를 버리고 얻은 per-process median들의 median이다. 각 arm의 선택 rtol과 E_nominal은 [전체 CSV](readback_runs/39263860973/COST_AT_MATCHED_ERROR.csv)에 있다. 서로 다른 case/grid의 anchor는 같지 않다.

|문제|출력|VigilODE fast|VigilODE banded|Diffsol BDF dense|Netlib DVODE|
|---|---|---:|---:|---:|---:|
|Robertson|endpoint|0.037920|해당 없음|0.108376|0.100469|
|Robertson|uniform101|0.089863|해당 없음|0.116443|0.117109|
|HIRES|endpoint|0.281925|해당 없음|0.536127|0.479670|
|HIRES|uniform101|0.373237|해당 없음|0.590071|0.533075|
|van der Pol|endpoint|0.265004|해당 없음|0.983506|0.753888|
|van der Pol|uniform101|0.331527|해당 없음|1.152068|0.933415|
|Brusselator N=100|endpoint|2.819666|1.999169|5.156916|0.943856|
|Brusselator N=100|uniform101|3.883919|2.759208|5.424570|1.562516|
|Brusselator N=400|endpoint|27.411388|7.900871|141.289589|3.610742|
|Brusselator N=400|uniform101|44.820667|12.830880|131.609302|6.154194|

[그림 endpoint](readback_runs/39263860973/MATCHED_endpoint.png), [그림 uniform101](readback_runs/39263860973/MATCHED_uniform101.png).

### Pairwise 통계

사전에 정한 42개 pair에 대해 12개 블록 전체를 재표집했다. 100000 bootstrap resamples, pointwise 95%와 Bonferroni family-adjusted 구간, exact paired sign test와 그 Bonferroni 보정값을 함께 저장했다. 세 arm은 여섯 순열을 두 번, 네 arm은 Williams balanced order를 세 번 사용했다. 4896개의 내부 적분을 4896개의 독립 통계 표본으로 세지 않았다.

아래 비율은 baseline/candidate다. 1보다 크면 candidate의 시간이 짧다.

|문제·출력|Baseline/Candidate|비율|family-adjusted bootstrap 구간|
|---|---|---:|---|
|Robertson endpoint|Diffsol/VigilODE fast|2.8575|[2.8460,2.8740]|
|Robertson endpoint|DVODE/VigilODE fast|2.6487|[2.6327,2.6571]|
|HIRES uniform101|DVODE/VigilODE fast|1.4280|[1.4246,1.4405]|
|van der Pol uniform101|Diffsol/VigilODE fast|3.4799|[3.4554,3.5156]|
|Brusselator N=100 endpoint|VigilODE banded/DVODE|2.1278|[2.1014,2.1445]|
|Brusselator N=100 uniform101|VigilODE banded/DVODE|1.7676|[1.7525,1.7880]|
|Brusselator N=400 endpoint|VigilODE banded/DVODE|2.1886|[2.1821,2.1956]|
|Brusselator N=400 uniform101|VigilODE banded/DVODE|2.0831|[2.0768,2.0931]|

이는 좁은 조건부 구간이며 12블록 bootstrap coverage, 호스트 autocorrelation, CPU frequency/가상화 공통 변동이 엄밀히 통제되었다는 뜻은 아니다. 여러 독립 호스트에서의 비회귀 보증으로 일반화할 수 없다. 모든 수치는 [원본 pair ratios](refined_runs/39261895983/paired_comparisons.json)와 연결되어 있다.

## 5. 왜 승자가 바뀌는가

[WORK_AT_MATCHED_ERROR.csv](readback_runs/39263860973/WORK_AT_MATCHED_ERROR.csv)는 정확도 선택에 사용한 nominal-h0 run의 작업량을 보존한다. 서로 다른 solver의 모든 카운터가 동일한 연산을 세는 것은 아니므로 단계별 의미를 유지한다.

Robertson endpoint에서 VigilODE fast는 51 accepted/2 rejected, Jacobian 51회, factorization 53회, RHS 422회였다. DVODE는 310 steps, Jacobian 6회, LU 58회, RHS 427회이고 Diffsol은 185 steps와 linear-solver setup 30회였다. 여기서는 작은 행렬과 상대적으로 적은 macro-step 수의 fast 경로가 유리했다. 단순 RHS 횟수만으로 시간을 설명할 수는 없다.

Brusselator N=400 endpoint는 반대로 VigilODE banded가 100 accepted/8 rejected, Jacobian 100회, LU 108회, linear solve 864회, RHS 856회인 반면 DVODE는 272 steps, Jacobian 5회, LU 31회, RHS 313회였다. DVODE는 더 많은 step을 밟아도 매번 8-stage Rosenbrock 비용을 지불하지 않고 Jacobian/LU를 오래 재사용한다. 이 작업량 차이는 DVODE의 우세와 일관되지만, wall time의 몇 %를 특정 원인에 배분한 profiling은 하지 않았다.

N=400 uniform101에서도 VigilODE banded의 Jacobian/LU/RHS는 168/175/1393이고 DVODE는 8/41/520이었다. 저장 형식 개선만으로 이 격차가 모두 사라지지 않는 이유를 보여 준다. 단, RODAS5P의 Jacobian을 임의로 freeze하면 차수·안정성 조건이 달라질 수 있으므로 BDF 방식의 재사용을 무검증으로 이식해서는 안 된다.

Diffsol의 N=400 dense 비용이 큰 사실은 dense backend를 사용한 이번 구성의 결과다. 이를 Rust 언어의 열세나 Diffsol 전체 알고리즘의 열세로 확대하지 않는다. VigilODE fast와 banded는 같은 rtol, 같은 관측 오차, 같은 step 수에서 비교되어 저장 구조 효과를 더 직접적으로 보여 준다.

## 6. SciPy 1.17 VODE 진단의 정정

[24개 차등 진단](runs/39257390707/SCIPY117_DIAG_SUMMARY.json)은 analytical C/F layout, transpose control, internal finite difference, full-band analytic/FD를 비교했다.

- Robertson에서 정상 Jacobian C/F 반환은 94077 steps였고 transpose-control은 252 steps였다. Fortran-contiguous 반환만으로는 버그가 없어지지 않았다.
- HIRES는 전치 workaround 또는 내부 FD dense 경로에도 각각 45781/45353 steps가 남았지만 full-band analytic 경로는 440 steps였다.
- van der Pol은 dense 경로에서 100000-step ceiling에 도달하여 t≈26.5까지만 진행했다. 이런 행은 tf=2000 오차를 산출하지 않고 null로 보존했다. full-band analytic 경로는 972 steps로 tf에 도달했다.

SciPy PR #24934는 user dense Jacobian의 row-major 복사/column-major 해석 문제를 명시적으로 확인한다. 그 버그는 확립된 원인이다. 전치 workaround 이후 남은 dense-path 이상은 재현했지만 별도의 구체적 C pivot/linear-solve 결함으로 원인을 확정하지는 않았다. 외부 upstream mutation은 이번 요청 범위에서 수행하지 않았다.

새 native Netlib 비교의 252개 VODE cells는 모두 종료했다. 따라서 이전의 실패율·큰 비용을 VODE 알고리즘 자체의 순위 근거로 쓰지 않는다.

## 7. 검산·실패·보존의 정확한 범위

기록된 test-driven loop에는 초기 10개 contract test 중 3개 RED 실패, 구현 후 10개 GREEN 통과가 있다. NumPy int32 직렬화 실패를 재현하고 제한된 scalar 변환으로 고친 뒤 3개 serialization test를 실행했다. Native 최종 test target은 4 passed/0 failed이다. root workspace 전체를 테스트했다고 주장하지 않는다.

정합 run의 적분과 timing은 모두 완료했지만 최초 readback은 초기값 bit equality assertion에서 실패했다. 별도 조사 결과 Diffsol의 18개 행에서 최대 절대 차이 2.220446049250313e-16, 최대 요청오차 단위 1.1702897349381543e-10이었다. 이 결과는 floating rounding 규모와 일치하지만 정확히 어느 내부 연산이 만들었는지는 추적하지 않았다.

후처리는 원본 초기 출력과 그 오차를 그대로 보존하고, 추가 16 epsilon floating-consistency 검사를 했다. 관측 최대는 (1+|y0|) scale에서 0.33334 epsilon이었다. 원래 bit-identity FAIL을 PASS로 덮어쓰지 않았고 새로운 적분·시간을 만들지 않았다. 변경은 추가 auditor 초기값 판정의 의미를 명시한 것이며, 이미 고정된 E_up 정합/시간 선택 기준과 원자료는 변경하지 않았다.

별도 vectorized metric 구현으로 678개 완전한 유한 출력, 189개 state object hash, 42 paired median을 검산했다. Robertson/HIRES 보존량 잔차의 최대는 4.21885e-15였다. 225개 보존 파일의 hash 검사도 통과했다. 독립성은 다른 계산 경로를 뜻하며 동일 agent가 작성했으므로 독립된 연구자/서브에이전트 review와 같지 않다.

주 스크리닝 run의 두 대형 raw gzip은 잘못 설정한 20MiB 게시 필터에서 빠졌다. 원래 756-row accuracy CSV, 1152 block-median units, references와 hashes는 보존됐지만 그 run의 개별 적분 원자료를 전부 재검산했다고 할 수 없다. 정합 run은 전체 상태를 content-addressed 방식으로 중복 제거했고, 상태 압축량 7540513 bytes, 189개 object, raw timing/calibration rows를 모두 보존했다. 재검산 때 필요한 분할 파일은 없었다(RECONSTRUCTION=[]).

## 8. 개선 순서와 종료 판단

1. 기존 SciPy 1.17 dense VODE 수치를 원본 알고리즘 순위에 재사용하지 않는다. 비교 baseline은 이번 source-pinned native DVODE로 고정한다.
2. 생산용 선택은 작은 stiff 계와 공간 이산화 계를 분리한다. 이번 결과만으로 dimension threshold router를 확정하지 말고 별도 holdout에서 검증한다.
3. 공간 계에서 다음 연구 대상은 banded 저장 형식뿐 아니라 step당 Jacobian/LU/RHS 비용과 재사용 정책이다. 새로운 reuse는 반드시 해당 Rosenbrock/W-method의 차수·오차 제어 검증을 동반해야 한다.
4. Diffsol은 가능한 sparse/구조화 backend와 별도로 비교해야 전체 라이브러리 수준의 판단이 가능하다.
5. 연속 출력·위상 정확도·질량행렬·JVP-only·물리 client와 다중 호스트 timing은 남은 scope다. 이 노드에서는 추가 확대하지 않고 현재 결과를 닫는다.

최종 판정: bounded comparison COMPLETE_SCOPED, numerical readback PASS_WITH_INITIAL_ROUNDING, shared-host speed PROMOTION_HOLD, continuous-time certification NOT_CLAIMED. 생산 solver 코드 변경·merge는 수행하지 않았다.

## 원전

- Brown, Byrne, Hindmarsh (1989), VODE: A Variable-Coefficient ODE Solver, SIAM J. Sci. Stat. Comput. 10(5), 1038–1051, DOI 10.1137/0910062.
- Netlib original source, local/global tolerance warning and MF21/MF24 contracts: https://www.netlib.org/ode/vode.f . 실제 다운로드한 bytes와 LINPACK sources는 runs/39257390707/sources 및 NETLIB_SHA256SUMS에 보존.
- SciPy 1.17 release notes: https://docs.scipy.org/doc/scipy/release/1.17.0-notes.html .
- SciPy fix PR: https://github.com/scipy/scipy/pull/24934 ; merge 9d88ff1739705d8c9f483eca014d6293d499c260, 2026-03-29.
- Diffsol0.16.1 및 종속 패키지의 정확한 resolved versions는 각 run Cargo.lock으로 고정. 본문 결론을 다른 Diffsol 버전에 확대하지 않는다.
