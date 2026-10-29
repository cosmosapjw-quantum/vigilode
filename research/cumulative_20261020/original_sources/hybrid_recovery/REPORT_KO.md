# 하이브리드 구조 축약·국소 AD·연속 인증 연구의 복구 및 최종 비판 분석

**기준일: 2026-10-20 KST.** 이 문서는 중단 전 이미 구현·실행·게시된 PR #77을 회수하여 첨부 연구와 연결한 종합 분석이다. 새 solver 실행이나 새 성능 표본을 만들었다고 주장하지 않는다. 이번 복구에서 수행한 계산은 원본 CSV 바이트/요약 검산, 정확한 유리수 모델 대조, Wolfram 기호 항등식 검산이다.

## 1. 완료된 것과 이번 게시 제한

|항목|확인 상태|
|---|---|
|기존 PR|#77, `research/hybrid-structure-ad-20261020`, Draft, 미병합|
|실제 측정 소스|`49ca3150c68d527a275c855ecf814122ff9c533a`|
|기존 원격 게시 HEAD|`f738fb481bad3f086b568c8493105576851093f2`|
|원래 구현·비교 실행|GitHub Actions `39378576576`|
|두 SHA의 차이|실측 이후 한 commit은 run 디렉터리의 결과 파일 추가뿐. 수치 소스는 동일|
|이번 새 검산|9개 로컬 시험 통과, CSV 40행 byte identity 및 32개 오차비 분류 확인|
|이번 추가 commit/push|**미수행**. 현재 GitHub 연결은 read-only 도구만 제공하며 인증된 CLI도 없음|

기존 구현과 성능 자료는 이미 원격에 있다. 이 보고서·재현 문서·새 검산 코드는 아직 미게시이며, 제공한 patch로 별도 적용할 수 있다. 기존 PR의 HEAD를 새로 만들었다거나 기존 Rust 테스트를 이번 세션에서 다시 실행했다고 표시하지 않는다.

## 2. 두 첨부 연구를 어떤 방식으로 합쳤는가

앞선 통합 연구는 `matrix-free operator session + all-at-once stage target + adaptive homotopy + shared error/work budget`을 제안했다. JVP 공유 부분공간이 작으면 유리하지만 rank가 작다는 사실만으로 Krylov 차원이 작지 않으며, 실제 stage 병렬성도 항상 이득은 아니라는 부정 결과를 포함했다.

뒤의 derivative-free 연구는 생성형 보존 flux, 몫대수·collective/transverse 소거, 국소 반응 ideal, 조건부 Schur waveform과 Bernstein 연속 defect 인증을 제안했다. 그 보고서의 강한 `runtime nonlinear-derivative-oracle-free` 계약은 일부 구조적 모델을 위한 후보였다. 현재 사용자는 이 제약을 완화했고, 불필요한 미분은 제거하되 필요한 국소 AD를 허용했다.

이번 구현은 다음처럼 두 연구를 결합했다.

|탐색 후보|이번 수렴 결과|남는 제한|
|---|---|---|
|몫대수·collective/transverse packet|정확한 closure가 성립할 때만 endpoint generator로 채택|epsilon=0, 지원된 반사 격자에 한정|
|물리 연산자와 국소 반응 분리|알려진 stencil은 직접 적용하고 비선형 cell graph만 dual AD|AD-free 전체 경로가 아니며 derivative 계수는 실제로 저장|
|조건부 Schur/low-rank 처리|정확한 tridiagonal+rank-one shifted solve|임의 비선형 waveform Schur 전체는 아직 아님|
|동시 stage homotopy|기존 eager/residual-first 후보를 같은 trajectory gate에 연결|이번 최속 경로는 이 경로가 아니라 순차 structured stage|
|연속 certificate|실제 반환 다항식의 원래 방정식 defect와 상속 반경을 연결|생성형 모델의 반이산 ODE에 한정|
|자동 선택|현재 모델의 정확한 algebraic capability로 분기|학습·비용 기반 최적 router가 아님|

사용자의 고유 PR에 있는 packet, chart/atlas, causal radius 계약을 확장했다는 의미이지, 새로운 조합의 모든 항목이 과거 PR에서 이미 검증되었다는 뜻은 아니다. 일반 PDE, variable mesh, ALE, 공간별 다종 물질계에는 추가 작업이 필요하다.

## 3. 실제 생성형 모델과 가정

정규화한 상태 \(q_i\)와 \(m=N^{-1}\sum_iq_i\)에 대해

\[
\dot q=d\Delta_{\mathrm{path}}q-\lambda q+\alpha m(1-q)-\epsilon q^2
\]

를 구현했다. \(\Delta_{\mathrm{path}}\)는 반사 경계의 nearest-neighbor path Laplacian으로 행·열 합이 0이다. \(d\)는 실제 edge rate이며, 비교에서는 \(d=0.01N^2\), \(\lambda=2\), \(\alpha=0.4\), \(\epsilon\in\{0,1\}\)이다. 시간과 상태는 이 제조 모델에 대해 정규화되어 있고, rate는 해당 시간의 역수 단위다. 실제 광자 에너지·온도·이온화율을 같은 raw norm에 넣은 물리 모형은 아니다.

조건은 \(d\ge0,\lambda\ge\alpha\ge0,\epsilon\ge0\)와 초기 상태 및 초기 불확실성의 \([0,1]^N\) 포함이다. \(q_i=0\)에서 유입은 비음수, \(q_i=1\)에서 유출은 비양수이므로 참해의 상자 불변성을 얻는다. 모델 field는 private이며 certificate의 모델 식별자에는 차원과 모든 계수의 bits가 포함된다.

참고로 이것은 예전 periodic scalar-feedback Fourier 예제와 다르다. 이번 코드가 사용하는 것은 **반사 path의 even extension FFT**다. 서로 다른 경계조건·모델의 성능 자료를 결합하지 않았다.

## 4. 정확히 소거할 수 있을 때는 AD도 필요 없다

평균 방정식은

\[
\dot m=(\alpha-\lambda)m-(\alpha+\epsilon)m^2-\epsilon\operatorname{Var}(q).
\]

따라서 \(\epsilon\ne0\)에서 분산을 버리고 같은 평균만 적분하면 다른 모델이다. 코드의 `admits_packet`은 epsilon이 **정확히 0**이고 격자가 지원되는 경우만 허용한다.

\(\epsilon=0\)이고 \(w=q-m\mathbf1\)이면

\[
\dot m=(\alpha-\lambda)m-\alpha m^2,
\qquad \dot w=d\Delta_{\mathrm{path}}w-(\lambda+\alpha m)w.
\]

\(\beta=\alpha-\lambda\), \(I_\beta(t)=\operatorname{expm1}(\beta t)/\beta\), \(I_0(t)=t\), \(D(t)=1+\alpha m_0I_\beta(t)\)로 두면

\[
m(t)=\frac{m_0e^{\beta t}}{D(t)},\quad
w(t)=\frac{e^{-\lambda t}}{D(t)}e^{dt\Delta_{\mathrm{path}}}w_0.
\]

모든 초기 cell 값을 \(w_0\)에 보존한다. 정확한 모델 축약이지만 FFT·지수함수의 부동소수점 후보가 자동 인증되는 것은 아니다. 최종 실제 곡선의 defect가 그 오차를 함께 검사한다.

여기서 용어를 과장하면 안 된다. Packet endpoint 자체는 비선형 미분을 요청하지 않는다. 그러나 **quintic dense curve의 acceleration은 대수적으로 계산한 \(Jf\)**를 사용한다. epsilon=0 auto의 기록에도 JVP/action 카운터는 8회이며 AD cell 카운터만 0이다. 이 경로 전체를 “모든 상태 미분 작용 0회”로 재명명하지 않는다.

## 5. 소거가 깨지는 곳은 국소 AD와 구조화된 선형대수로 처리한다

비선형 graph \(n_i=\alpha m(1-q_i)-\epsilon q_i^2\)를 cell 값과 mean에 대해 미분하면

\[
J=d\Delta_{\mathrm{path}}
-\operatorname{diag}(\lambda+\alpha m+2\epsilon q_i)
+u v^T,\quad u_i=\alpha(1-q_i),\quad v_i=1/N.
\]

이는 실제 소스의 국소 dual AD와 일치한다. FullAd는 알려진 stencil까지 dual 수로 통과시키며, LocalAd는 물리적 stencil 작용을 분리하고 비선형 graph만 dual 수로 계산한다. 이 구현은 아직 일부 활성 cell만 동적으로 선택하는 sparse AD는 아니다.

\(s=h\gamma\), \(W=I-sJ=T-su v^T\)에 대해 tridiagonal \(T\)를 분해하고

\[
z=T^{-1}su,\quad x_0=T^{-1}b,\quad
x=x_0+z\frac{v^Tx_0}{1-v^Tz}
\]

를 적용한다. 코드가 분모·pivot·유한성을 검사하며, stage solve 뒤 현재 원래 target residual도 확인한다.

전체 상태 Jacobian은 조립하지 않지만 국소 derivative 계수·band diagonal·rank-one 벡터는 실제로 구성한다. 따라서 정확한 표현은 **global-Jacobian-assembly-free structured AD**이다. 원래의 절대 미분 금지보다 약하지만 이번 사용자가 요청한 계약과 맞는다.

이번 복구에서 Wolfram으로 N=3 기호 mean/분산, epsilon=0 transverse, band+rank-one 분해와 row-sum 항등식을 확인했다. 별도 Fraction 검산은 32개 유리수 모형에서 dual 방향미분과 분해식 및 shifted solve를 exact arithmetic으로 비교했다. 이는 새 기호/대수 검산이지 Rust 재실행이나 전체 미분계수 domain의 자동 정리증명이 아니다.

## 6. Root·시간·dense-output를 하나의 실제 곡선으로 검사

Endpoint generator가 만든 \(y_1\)을 이용해 cubic 또는 quintic Hermite/Bernstein 곡선을 만든다. Quintic은 양끝의 값·RHS·acceleration을 사용한다. **저장된 binary64 Bernstein controls 자체가 실제 곡선**이며, 그 곡선에 대해

\[
d(t)=\dot{\widehat q}(t)-F(\widehat q(t))
\]

의 Bernstein 계수를 outward binary64 구간연산으로 감싼다. \(F\)가 이차 graph이므로 차수 \(p\) 곡선의 defect 차수는 최대 \(2p\)다. 단순 출력 샘플 비교로 sup defect를 선언하지 않는다.

상자 안에서 \(J\)의 비대각 원소가 비음수이며

\[
\sum_jJ_{ij}=\alpha-\lambda-\alpha(m+q_i)-2\epsilon q_i
\le\alpha-\lambda.
\]

따라서 무한대 norm의 비교 gain은 \(\mu\le\alpha-\lambda\le0\)다. 큰 diffusion rate를 양의 증폭으로 부풀리지 않는다. \(a\le\lambda-\alpha\)라는 안전한 하한과 defect 상한 \(\delta\)로

\[
r(t)\le e^{-a(t-t_0)}r_0+
\frac{\delta}{a}\bigl(1-e^{-a(t-t_0)}\bigr)
\]

를 계산하며 \(a=0\)에서는 \(r_0+\delta(t-t_0)\)를 사용한다. 참해의 invariant box와 represented curve의 Bernstein box 포함이 이 비교식의 전제다.

불완전한 endpoint root, 시간 절단, curve fit의 오차는 **같은 실제 곡선의 전체 defect**에 반영된다. 같은 효과를 embedded error라는 이름으로 또 무조건 더하지 않는다. 하지만 원래 ODE가 아닌 근사 opacity/limiter/계수표에 대해서만 defect를 계산한다면 그 모델 차이는 따로 포함해야 한다.

승인 순서는 candidate → actual curve → continuous tube/radius → 요청 시각의 반환 rounding 반경이다. 이 단계들이 모두 통과한 뒤만 상태·시간·상속 반경을 commit한다. 실패한 trial은 rollback되고 그 비용은 장부에 남는다. 추가 dense query도 반환 rounding이 예산을 넘으면 거절할 수 있다.

인증은 **이 생성형 reflecting-grid 반이산 ODE**, initial true state의 상자 포함, IEEE round-to-nearest·gradual underflow·no-fast-math/FTZ 조건부다. PDE 공간 오차·remap·일반 callback·비평형 물리 모형 전체의 보증이 아니다. 또한 이번 복구에서 전체 87,056개 계수 replay를 다시 실행하지 않았다. 그 결과는 원격의 원래 실행 로그로 확인한 별도 증거다.

## 7. 원래 실행 규모와 이번 복구의 차이

|원래 게시된 실행|기록|
|---|---:|
|Rust 시험|19 passed: extended12 + initial4 + native3|
|정확도 보정·대조 실행|102회, 경험적 출력 ceiling 통과84회|
|선택된 설정|40개|
|독립 프로세스 시간 측정|160개, 총480회 full trajectory|
|비교|32pair, 경험적 오차비 2배 이내22pair|
|저장된 exact-rational replay|87,056 residual 계수, 203연속 segment, 600반환 출력|

표의 수치는 run39378576576의 SUMMARY/TESTS/AUDIT에서 가져왔다. Rust 시험은 warning-free나 전체 workspace PASS를 뜻하지 않는다. 외부 comparator가 같은 continuous certificate를 계산했다는 뜻도 아니다.

이번에 **새로 실행한 복구 검산**은 다음으로 한정된다.

- 원본 RESULTS.csv 5,048bytes의 SHA256와 Git blob SHA 일치.
- 40개 cell identity·출력오차·기록된 반경·시간 요약의 유효성.
- 32개 pair의 경험적 오차비 재계산. 22개가 2배 이내이고, 그중 인증 계약도 같은 것은18개.
- 32개 유리수 모델에서 국소 AD·mean/분산·band+rank-one·정확 shifted solve, 136개 row-gain 검사.
- 변조, line-ending 변경, NaN/Inf, 평균 closure 반례 등을 포함한9개 로컬 시험 통과.

원래 full raw trajectory/interval audit를 이 환경에서 재실행했다고 하지 않는다. 현재 도구는 GitHub 원격 읽기는 가능하지만 원자료를 모두 내려받아 해당 전체 script를 재실행하거나 쓰기 commit을 할 수 있는 인증된 실행 경로를 제공하지 않는다. 새로운 per-arm median의 비율을 원래 paired-block median과 혼동하지 않는다.

## 8. 실제 full-trajectory 성능

제조 모형은 \(t\in[0,0.2]\), 요청 출력21개, 초기상태

\[
q_i(0)=0.45+0.12\cos\frac{\pi(i+1/2)}N
+0.03\cos\frac{7\pi(i+1/2)}N
\]

다. Native7개 경로는 absolute budget1e-6을 공유한다. 외부 fast/Diffsol/DVODE의 tolerance는 미리 정한1e-4..1e-9 사다리를 실행한 뒤, 유효 후보 중 auto의 실제 출력오차에 가장 가까운 값을 정확도만 보고 선택했다. 시간 최솟값을 골라 설정하지 않았다.

아래 시간은 **ms/전체 trajectory**, 4개 새 프로세스 블록과 각3회 적분의 per-arm median 요약이다. Root1step latency나 예전 호스트 시간과 직접 나누면 안 된다.

|N,epsilon|Eager full-AD|Residual full-AD|Residual local-AD|Structured local-AD|Auto|Diffsol dense|원본 DVODE dense|
|---|---:|---:|---:|---:|---:|---:|---:|
|16,0|1.07819|1.04142|1.05500|0.85785|0.79861|0.08755|0.04589|
|16,1|10.61555|9.48662|9.46909|1.02876|1.01906|0.08047|0.04887|
|64,0|5.15909|4.96874|4.96820|3.22203|3.04952|0.51822|0.41482|
|64,1|44.76805|39.27822|39.38370|3.93385|3.93018|0.54095|0.46358|

### 내부 개선은 주로 구조화된 solve에서 왔다

N64,epsilon1에서 같은 새 trajectory/certificate driver 아래 eager44.76805ms → auto3.93018ms, **기록된 두 중앙값의 비율11.3908**이다. Residual/local-AD39.38370ms → auto3.93018ms 비율은10.0208이다. 이것은 예전 프로그램 전체에 대한 보편적인11배 가속이 아니라, 이 모델에서 같은 증명 의무를 공유하는 후보 교체 비교다.

반면 local AD 자체의 이득은 확인되지 않았다. N64,epsilon1의 lazy-full/lazy-local paired median은0.998649, 관측 범위[0.991336,1.008576]이다. 선형 stencil을 AD에서 빼는 설계는 합리적이지만 이 작은 모형에서 wall-time 개선을 확정할 수 없다. 더 많은 최적화가 실제 병목에 도달했는지는 별도 phase profiling이 필요하다.

Eager의 JVP8,262회가 residual 경로에서5,606회로 줄고, structured 경로에서는50회가 된다. Structured의 국소 dual cells640, partial AD seeds640, bandfactor5와 기존 projected 경로의 수십만 dual-cell work를 비교할 수 있다. 하지만 단순히 AD cells의 수를 총 wall-time 예측값으로 취급하지 않는다.

### 최속 내부 경로는 아직8순차 stage다

`structured_endpoint`에는 raw-U stage `for i in 0..8`가 남아 있다. Coupled 후보의 큰8순차 solve를 없애려던 목표와 별개로, 이 대조에서 가장 경제적인 것은 구조화된 정확 solve를 공유한 순차 Rosenbrock 경로였다. 이 사실을 “all-at-once homotopy 병렬 경로가11배 빨라졌다”로 바꾸면 안 된다. 병렬 coupled/homotopy는 유지되지만, 새 auto는 정확한 capability test로 packet 또는 structured를 선택하는 제한된 분기다.

### 더 많은 미분이 오히려 전체 일을 줄인 경우

N64,epsilon1에서 cubic-auto는27accepted+1rejected window,10.41518ms였고 quintic auto는5accepted,3.93018ms였다. 실제 출력오차는 각각7.43784e-9와7.45625e-9로 가깝고, 같은 인증 계약의 paired cubic/quintic 시간비는2.67224였다. Quintic은 acceleration을 위한 AD/Jf를 더 쓰지만, 짧은 창과 반복적인 인증을 크게 줄였다. “미분을 적게 쓰기” 대신 **전체 승인 궤적의 비용을 줄이기**가 적절한 목표라는 긍정 사례다.

그러나 epsilon0의 cubic/quintic 오차비는 약10~15배라서 같은 달성 오차에서의 속도비라고 선언하지 않는다. 공통 budget 통과와 동일 달성 오차는 다르다.

## 9. 외부 solver 비교가 말하는 것과 말하지 않는 것

N64,epsilon1의 실제 최대 출력오차 지표와 반환 반경은 다음과 같다.

|경로|관측오차+경험적 참조폭|기록된 반환 반경|시간 ms|
|---|---:|---:|---:|
|Auto/structured|7.45625e-9|5.58137e-8|3.93018|
|Diffsol0.16.1 FaerLU dense|4.36323e-9|제공 안 함|0.54095|
|Netlib original DVODE dense|3.59921e-9|제공 안 함|0.46358|
|VigilODE fast dense|5.45452e-12|제공 안 함|0.96055|

Diffsol 대비 경험적 오차비는1.70888로 factor2 안이다. 그래도 **두 작업의 인증 계약은 다르다**. DVODE 대비 오차비는2.07163으로 사전 factor2 기준을 약간 넘지만 기준을 넓혀 통과시키지 않았다. Fast는 참조 precision 부근까지 과도하게 정확하므로 같은 달성 오차 비교가 아니다.

외부 두 BDF 구현이 실제로 더 짧다는 관측은 그대로 보고한다. 다만 그 차이를 순수 solver efficiency 또는 인증 비용 하나만의 결과로 귀속할 근거는 없다. Diffsol은 slice-copy-to-common-model adapter 비용을 포함하며 이번 dense backend만 측정했다. DVODE는 전역 rank-one coupling 때문에 analytical dense로 계산한다. 다른 sparse backend나 band-plus-low-rank 특화 BDF까지 이 결과로 판정하지 않는다.

### 이번 회수에서 새로 확인한 타이머 주의점

`hybrid_bench.rs`는 `run()`을 측정한다. Native `run()` 내부에서 `serde_json::to_value(r)`로 전체 integration/인증/통계 객체를 변환하고, external 경로도 일부 JSON value를 구성한다. 따라서 타이머는 **내부 proof/통계 결과의 JSON-value materialization을 포함한다**. 바깥 `println!` JSON 문자열 출력과 결과 destruction은 제외된다. 저장된 `timer` label의 “excluding serialization”은 바깥 문자열 출력으로만 좁게 읽어야 한다.

이 차이는 correctness에 영향을 주지 않지만 성능 해석에는 중요하다. 인증·curve 생성·endpoint solver·보고용 객체 구성 비용을 분리해서 측정하기 전에는 외부와의 격차를 모두 인증 자체의 불가피한 비용으로 결론내리지 않는다. 원래 시간값을 소급해 보정하지 않았다.

## 10. 발산 후보의 우선순위와 종료 판단

이 구현은 두 이전 연구를 실제 실행 가능한 조건부 경로로 통합한 기준점이다. 그대로 일반 물리 코드에 승격할 단계는 아니다.

- **채택 가능한 근거:** exact closure가 참일 때 전체 초기모드를 보존하는 packet; 비선형 부분에 국한된 AD; 물리적 band와 낮은 rank의 정확 결합; 원래 실제 curve의 연속 defect; inherited radius와 rollback.
- **이번 측정에서 채택하지 않은 주장:** local AD만의 일반적 speedup, 고정 차원의 자동 최적 selector, coupled 병렬/Homotopy의 보편적 우위, 외부 BDF보다 빠르다는 주장.
- **범위 밖:** AMR/ALE와 remap 반경, 실제 E-theta phase-space 모형, 임의 domain/limiter, general high-rank opacity waveform, zero-allocation, 대규모N 성능, stiff-uniform5차 보장, 독립인원 또는 형식증명 심사.

다음 비용 측정에서는 인증 의무를 고정한 상태에서 endpoint/curve/interval/proof-materialization을 분리하는 것이 우선이다. 일반 stage solver를 계속 빠르게 만드는 연구와, 생성형 물리 구조로 stage solver 자체를 우회하는 연구를 같은 기여로 뭉치지 말아야 한다. 이번 회수는 새 기능을 무한 확장하지 않고, 기존 실행의 결과와 남은 증명·성능 범위를 문서화하는 것으로 닫는다.

## Source provenance

입력 연구: 현재 첨부된 통합 report와 derivative-free FVM report. 실측 source와 자료는 아래 immutable refs를 사용한다.

- PR77: https://github.com/cosmosapjw-quantum/vigilode/pull/77
- Measured: https://github.com/cosmosapjw-quantum/vigilode/tree/49ca3150c68d527a275c855ecf814122ff9c533a
- Published evidence: https://github.com/cosmosapjw-quantum/vigilode/tree/f738fb481bad3f086b568c8493105576851093f2/research/hybrid_20261020/runs/39378576576
- 실제 수식: `crates/vigilode-hybrid/src/{model,bounds,native,trajectory}.rs`
- 실제 계측: `crates/vigilode-hybrid/examples/hybrid_bench.rs`, `research/hybrid_20261020/measure.py`
- 원래 exact replay: `research/hybrid_20261020/audit.py`
- 이번 새 요약 검산: `recovery_20261020/verify_recovery.py`, `RECOVERY_CHECK.json`

이 문서는 원전·코드가 뒷받침하는 사실, 새 직접유도, 새로 수행한 제한된 검산, 아직 미검증인 확장을 구별한다. 추가 push 미수행 상태는 `RECOVERY_EVIDENCE.json`에 명시한다.
