# VigilODE 재감사와 RVJ 연구 통합 리뷰

작성일: 2026-10-04. 구현 기준은 `audit/rvj-native-followup-20261003`의
`826fa05cc1fdaf7b1a0cf7ac45116596b4ddff65`다. 산출물은 별도 새 브랜치
`audit/rvj-research-integration-20261004`에 놓았다.

## 1. 판정과 실제 변경

**권고는 “보편적인 RVJ solver 승격”이 아니라 검증 가능한 공통 연산을 먼저
이식하고, 실제 coupled client의 오차·비용 계약으로 이어 가는 것이다.** 이번
변경은 그 첫 단계를 코드로 만든다. 고정된 현재 dissipative J에 대한 여러
positive real shifted solves를 하나의 중심 LU와 정규화 jet으로 계산하고,
각 결과는 현재 원 target의 outward residual을 별도로 통과해야 한다.
기존 ODE solver의 기본 선택·허용오차·물리 closure는 바꾸지 않았다.

이번에 완료한 구현은 다음과 같다.

- `crates/rodas5p-core/src/shared_shift_jet.rs`: 새로운 opt-in 연구 API.
  제공된 RHS columns를 모두 보존하고 current J/h/γ/B를 한 호출 안에서
  사용한다. 과거 W나 캐시 증명서를 주입하지 않는다.
- `crates/rodas5p-core/examples/shared_shift_jet_study.rs`: 새 fixture와
  명시적인 negative controls를 JSON 및 IEEE-754 bit identity로 내보낸다.
- `crates/rodas5p-integrators/src/stage_chart_candidate.rs`: NaN 잔차의
  거짓 수렴, malformed target/provider 출력의 panic 경계를 닫는다.
- `crates/rodas5p-integrators/src/chart_transport.rs`: 비유한 설정과
  `t+h==t`일 때의 물리 상태 갱신을 거절한다.
- `research/rvj_integration_20261004/`: 사전등록, 원본 결속, 정확 유리수
  oracle, 전체 이력 재사용 지도, 증거와 machine-readable 후속 DAG.

이 API의 `Certified`는 **각 RHS의 현재 shifted linear solve에 대한
Euclidean absolute error bound**다. 비선형 ODE 전체 시간 구간의 오차,
embedded estimator의 신뢰성, solver order, 성능 우월성은 뜻하지 않는다.
독립 최종 판정은 **ACCEPT_NATIVE_RESEARCH**다. 검토자는 후보 구현에 참여하지
않았고, 실제 최종 source·raw evidence·exact oracle·23개 fresh 테스트를 읽었다.
실행 수치는 아래 검증 절 및 `FINAL_VERIFICATION.json`에 결속한다.
생산 기본 경로 활성화와 속도 승격은 HOLD다.

## 2. 어느 업데이트와 어느 연구를 통합했는가

| 대상 | 정확한 식별자 | 이번 처리 |
|---|---|---|
| 이전 구현 WU25 | `5a8d7fe9ffc681bca98a98a2f9889a2a05505783` | 최신 구현의 조상임을 확인 |
| 최신 구현·테스트 브랜치 | `826fa05cc1fdaf7b1a0cf7ac45116596b4ddff65` | 이 브랜치에서 새 작업 분기 |
| 별도 연구 브랜치 | `6852b4af9ecadfb8c3aa50c4d240301dd1f3a4db` | 원격은 metadata-only임을 확인 |
| 첨부 full-history archive | SHA256 `de25f591272d1b06d5ff597a43cf73ca026959997baa87b05eceaa70d687f0ca` | 4,550경로 byte identity 검사, 불일치 0 |
| Loop11 원본 ZIP | SHA256 `a7d66500012f2c982470afcbd96b489c2dc7da4391f2e0f0f8aa7eaefd326fe6` | 이식 source로 원본 그대로 포함 |
| 이번 사전등록 | `3e784982104274312027490d481de4929e4f3cf6` | 새 수치 검증 이전 원격 게시 |

`research/rvj-full-history-20261004`의 날짜가 가장 최근이어도 제품 구현이
가장 최근인 것은 아니다. 해당 tree는 보존용 인덱스이며, 실제 최신 Rust
변경은 native-followup에 있다. 별도 연구의 초기 `Vigilode_mutation_authorized=false`
등은 그때의 기록이다. 이번 이식·새 브랜치 게시 권한은 현재 사용자 요청이다.
과거 TRANSFER_STATUS나 push 영수증을 현재 HEAD 또는 현재 PASS로 읽지 않았다.

이번 새 브랜치는 모든 역사 archive의 원격 payload 이전까지 완료했다고
주장하지 않는다. 전체 이력의 인덱스·읽기 범위·해시와 이번 이식에 사용한
Loop11 원본 ZIP을 포함한다. 전체 약21MiB 원본은 사용자 첨부에서 검사했다.
원본 geometry missing inputs나 pre-package chat-only 기록은 복원하지 않았다.

## 3. 이미 게시된 연구의 한 번 감사와 재사용

13개 단계(01–11, V01/V02)의 primary note·handoff·claim·negative 기록을
한 번씩 감사했다. 모든 nested source와 원시 oracle를 line-by-line 검사했다는
뜻은 아니다. 정확한 읽기 파일·hash·부분 범위는 `evidence/archive_claims.json`,
단계별 지도는 `evidence/history_matrix.json`과 `.md`에 있다.
**과거 과학 캠페인 전체 재실행은 0회**다. 기존 결과는 원래의 조건·실패·HOLD를
유지해 사용한다. 새 실행은 새 native 코드와 새 입력 경계에만 묶었다.

계보는 다음처럼 이해하는 것이 다음 개발에서 중복과 잘못된 승격을 줄인다.

| 계보 | 유지할 성과 | 금지해야 할 일반화 | 현재 연결 |
|---|---|---|---|
| 01–03: rational jet/RVJ | PR군의 조건부 차수 정리, complex-shift gain, current J 계약 | generic stiff-uniform5, approximate W 무조건 대체, embedded 차이=오차 | raw RVJ를 기본 solver로 추가하지 않음 |
| 04–07: chart/closure/feedback | branch witness, whole-path tube, initial impulse와 bidirectional feedback | endpoint가 정확하면 trajectory/feedback도 정확 | typed provider·물리 오차 전달 API로 연결 |
| 08–09: exp/convolution/carrier | full layer 표현, 알려진 carrier의 phase 분리 | exact exp만으로 forcing alias 해소, unit modulus=phase 정확 | whole-target defect와 phase error 예산 보존 |
| 10–11: Fourier/FFT/공유 action | 실제 비선형 FFT predictor, unchanged exact certificate, nonreduced algebra와 shared shift theorem | FFT roundtrip=정확성, 한 scalar로 모든 모드 복원, LU 횟수=속도 | 이번 real native action, 이후 실제 coupled client |
| V01/V02와 현행 native | Laguerre guard, callback identity, radius 검사, chart helper | 오래된 TODO를 다시 개발하거나 과거 overlay 무조건 적용 | 현재 source와 비교해 새 경계만 수정 |

특히 보존할 반례는 세 가지다. RJ5의 PR recurrence는 classical order5와
stiff-uniform order3가 양립함을 보였다. 고정 상대 W 오차에서는 RJ/RVJ
증폭이 각각 z²/z⁴로 성장한다. Exact current J를 사용한 raw RVJ도 bounded-g
semilinear 예에서 uniform5를 보장하지 않으며 embedded effectivity가 0으로
갈 수 있다. 이 부정 결과를 새로운 표현이나 작은 PASS 표로 덮지 않았다.

이미 완료된 일도 정확히 인정한다. Loop07의 exponential full-path 과제는
Loop08에서, Loop10의 FFT predictor 과제는 Loop11에서 수행됐다. Complex
positive-real-part shift의 inverse gain `|γ|/Re γ` 증명은 이미 Loop02에 있다.
다음 일은 재증명보다 native directed arithmetic과 물리 target 결속이다.

## 4. 적대적 감사 findings

| ID | 중요도·현재 상태 | 근거와 영향 | 조치 |
|---|---|---|---|
| INTAKE-NATIVE-01 | P2 · 수정 | NaN을 `f64::max`가 버리면서 residual norm0 및 Converged 가능. malformed target/JVP는 indexing panic | 입력/shape·각 residual/JVP의 finiteness 확인. 새 RED→GREEN 보존 |
| INTAKE-NATIVE-02 | P2 · 수정 | `t0=2^53,h=1,t_end=t0+2`에서 시간은 정지하고 물리 상태만 전진 가능 | representable time progress를 advance 이전 검사 |
| INTAKE-NATIVE-03 | P2 · 미완료 | REV-01c refresh repair는 존재하지만 fast matrix-free caller는 default options 경로를 사용 | 안전한 recycle policy의 명시적 caller 연결을 다음 P1 개발 단계로 지정 |
| A-PORT-01/02 | 이식 경계 · scoped 해결 | Python의 bound는 fixture dissipativity를 가정하고 f64 residual은 diagnostic | native에서는 sufficient dissipativity proof와 directed current-target residual |
| A-PORT-03 | P2 · 미완료 | FFT certificate가 phase/phase_error 공급자의 올바른 구성에 의존 | native typed phase-enclosure identity와 constructor 증명 필요 |
| A-PORT-04/05 | P2 · 유지 | recurrence depth·optimal baseline 미평가; Fourier conjugation/padding 이식 위험 | 비용 gate와 full original-target negative controls 필수 |
| INTAKE-NATIVE-O2/O3 | P2 · proof obligation | 감소함수 exp(−x)에 rounded-upper x 대입, 음수 성장률과 upward 시간 곱, rounded midpoint 반경의 조합 방향 | false enclosure를 관측했다고 쓰지 않음. 해당 local interval composition만 먼저 점검 |

중요도는 실제 적용 범위에 따른 것이다. chart와 새 action은 연구 helper/명시적
호출 경로다. 이 증거로 default RODAS5P가 잘못된 해를 수락했다고 결론내리지
않는다. GCRODR 문제도 true residual check를 우회한 false convergence가 아니라
이미 알려진 convergence 실패·추가 work의 caller 통합 부채다.

독립 새 코드 검토에서는 메모리 예산에 결과의 gamma와 certificate tolerance
스칼라 슬롯이 빠진 점도 발견했다. 계산식과 경계 테스트를 수정하고 해당 최초
finding을 남긴다. 예산은 **명시적 f64 슬롯 상한**이며 faer workspace·allocator
capacity를 포함하는 프로세스 RSS 상한은 아니다.

## 5. 수학에서 native code까지

현재 J에 대해 `J+Jᵀ≤0`, h≥0, γ0>0라 하자. P=I−γ0hJ, R0=P⁻¹다.
Loop11의 K=hR0J 대신 이번 구현은 T=γ0K=R0−I를 쓴다.

\[
z=\frac{\gamma-\gamma_0}{\gamma_0},\qquad
W_0=R_0B,\quad W_{k+1}=R_0W_k-W_k,\quad
U_p=\sum_{k=0}^{p}z^kW_k.
\]

`||(I−γhJ)⁻¹||2≤1`이고 `||T||2≤1`이므로 |z|<1에서 올바른 resolvent의
기하급수다. `γ0^−k`를 중간 저장하지 않아 작은 γ0에 의한 표현상 overflow를
피한다. R0≈I일 때 cancellation이나 f64 LU 오차가 없어진다는 뜻은 아니다.

수용은 이 급수의 사전 꼬리만으로 결정하지 않는다. 실제 저장된 U에 대해

\[
r=B-(I-\gamma hJ)U,\qquad
\|e\|_2\leq\|r\|_2\leq\|r\|_1
\]

을 outward interval 연산으로 계산한다. 각 column의 L1 residual upper가
고정 absolute tolerance 이하여야 Certified다. LU, recurrence, ratio와 Horner
반올림, truncation의 영향은 모두 U의 원 target residual에 남는다. Matrix와
각 입력 f64는 exact binary real로 해석한다. Oracle도 이 동일한 원 target을
Fraction으로 직접 풀지만, LU/jet/interval 구현은 공유하지 않는다.

Dissipativity는 symmetric-part Gershgorin row upper로 충분조건을 확인한다.
따라서 일부 실제 dissipative J도 거절할 수 있다. 이는 안전한 범위 제한이며,
고유값이 음수라는 이유로 nonsymmetric J를 통과시키는 정책보다 좁다. General
H metric·complex residual·matrix-free action oracle는 다음 이식 단계다.

API는 `shared_shift_jet(&J,h,gamma0,&rhs_columns,&gammas,config)`와 독립
`certify_shift_candidate(...)`다. Report/candidate/certificate는 private fields,
읽기 accessor와 Serialize만 제공한다. deserialize된 자기선언 증명서를 믿지
않는다. RHS columns의 rank를 추정하거나 몰래 압축하지 않는다. 단일 호출 중
현재 J를 고정 borrow하며 cross-call cache identity 문제를 만들지 않는다.

## 6. Laguerre와 다른 polynomial을 어떻게 이어갈 것인가

“다항식 대 Krylov”라는 구분만으로는 구현 선택이 정확하지 않다. p(A)b도
수학적으로 Krylov span 안에 놓일 수 있다. 여기서 원하는 대안은 **Arnoldi
직교기저를 구성하지 않고 recurrence·interpolation·rational action을 계산하는
경로**다. 각각의 domain witness와 전체 오차를 함께 비교해야 한다.

| 후보 | 적합한 조건 | 실제 남은 작업·중단 기준 |
|---|---|---|
| Laguerre | 현행 verified symmetric nonpositive domain, 유효한 scale과 degree | tail·coefficient·recurrence·transport·normalization 전체가 tolerance를 닫아야 함. RNEXT04 admitted 결과를 재발견하지 말고 hρ≈48의 과대 경계 영역에서 새 개선을 분리 |
| Chebyshev | verified real interval의 self-adjoint/normal action | Clenshaw/recurrence와 interval 밖 adversarial controls. 비정규에는 spectrum만으로 total error 인정 금지 |
| Leja/Newton | 검증된 spectral/numerical-range 영역과 interpolation remainder | divided difference·rounding·scaling을 포함한 bound가 없으면 candidate-only 유지 |
| Scaled Taylor | 현재 검증된 action/stepping bound를 재사용 가능 | REV02의 유효 absolute certificate와 실패한 relative gate를 구분. 큰 decay에서 상대 정확성 과장 금지 |
| Shared resolvent jet | current dissipative J, 좁은 real-positive shift cluster, 여러 공통 RHS | 이번 primitive. 복수 LU 대비 counter 절감만으로 Schur/multishift보다 빠르다고 판단 금지 |
| Rational/partial fraction | pole-free resolvent witnesses, 적합한 shift cluster | 독립 shift action 병렬화 가능. complex inverse gain·rounding·physical transport 필요 |

상세 source-to-API 표와 개발 gate는 `POLYNOMIAL_PARALLEL_KO.md` 및 DAG에
있다. 실패한 REV02의 상대 오차 gate를 PASS로 바꾸거나 원래 RNEXT04 범위를
nonnormal 전체로 확장하지 않는다.

## 7. homotopy를 통한 parallelism: 이득이 생기는 곳

Loop11의 공유 중심은 서로 가까운 target operator에 대한 setup을 재사용한다.
공유 jet이 준비되면 target별 Horner evaluation/residual, RHS columns의 계산은
독립이다. 하지만 W(k+1)가 Wk를 필요로 하는 깊이는 남는다. 알려지지 않은
다음 nonlinear stage RHS를 이 방법이 미리 생성하지도 않는다.

원하는 가속 방향은 다음 세 층을 분리해야 한다.

1. **Operator-family reuse**: 여러 γ가 실제 client에 이미 존재하면 중심 LU와
   RHS span을 재사용한다. 기존 common-γ RODAS는 이미 LU를 공유하므로 새
   factorization 이득이 없는 경우를 control로 둔다.
2. **Predictor parallelism**: Fourier/FFT predictor, independent shifts 또는
   homotopy sample들을 병렬 생성한다. 마지막 판단은 λ=1 원 target certificate다.
3. **시간 또는 stage 병렬화**: waveform relaxation/Parareal/SDC 류의 별도
   predictor-corrector 계약과 stability·iteration-depth 검증이 필요하다. 단순히
   여러 mode를 한 algebra packet으로 묶었다는 사실만으로 해결되지 않는다.

Loop10의 direct6 대 continuation18 RHS 반례는 계속 유효하다. Continuation
step을 더했으면 그 전부, rejections, reconstruction, certification, dispatch와
synchronization을 비용에 넣어야 한다. Local algebra의 support가 한 점이 되어도
multiplicity와 independent amplitudes는 없어지지 않는다. ± involution도
family와 initial data의 호환성을 먼저 확인해야 한다.

## 8. 구체적인 다음 개발 순서

Machine-readable `NEXT_DEVELOPMENT_DAG.json`은 각 작업의 prerequisite,
source seam, 산출물, acceptance, kill condition과 실행 범위를 담는다.
우선순위는 다음과 같다.

1. **안전한 현재 native caller를 완성한다.** GCRODR refresh/cold 정책을 명시적으로
   연결하고, 기존 알고리즘 비교 식별자를 보존한다. 과거 attribution campaign을
   다시 돌리는 대신 policy wiring과 실제 charged products를 검사한다. chart와
   nonnormal certificate의 directed composition 의무는 별도 exact local probes로
   닫는다. 관측되지 않은 false bound를 사실처럼 전제하지 않는다.
2. **새 primitive의 비용·지원 범위를 확장한다.** cluster degree selector, full work
   accounting, snapshot-bound matrix-free action, known complex inverse-gain의 native
   구현을 각각 분리한다. degree cap/unsupported metric/pole이면 명시적으로
   분할하거나 거절하며 tolerance를 풀지 않는다.
3. **진짜 coupled client를 선택한다.** 기존 Fourier primitive는 scalar analytic
   moments를 이미 사용하므로 불필요한 matrix solve를 끼워 넣지 않는다.
   실제 semilinear coupled block L(k)와 independent amplitudes, nonlinear
   convolution을 가진 문제에서 원래 필요한 shifted actions를 이 primitive에
   연결한다. Physical original-target residual, phase error, closure tube를 유지한다.
4. **같은 물리 오차에서 경쟁한다.** repeated LU 외에 Schur reuse, unrelated-RHS
   block multishift Krylov, admission 가능한 polynomial action을 포함한다. 먼저
   accuracy와 rejection/coverage를 통과하고, 그 뒤 prospective heldout timing을
   수행한다. 현재 timing authority HOLD를 문서와 counter로 우회하지 않는다.
5. **마지막에만 default activation을 논의한다.** 성능·오차·fail-closed 경로가 모두
   실제 client에서 확인되고 독립 reviewer가 승인하기 전에는 q=2/RVJ/새 action을
   기존 solver의 기본 선택으로 바꾸지 않는다.

## 9. 새 실행 증거와 제한

| 항목 | 이번 실행 결과 | 근거 |
|---|---|---|
| 새 native fixtures | 15개 입력군, 591 target, 1,341 RHS–target rows | `evidence/native_study.json` |
| 인증/거절 | target 559 Certified·32 Rejected; RHS rows 1,309·32 | `RESULTS.json` |
| 정확 oracle | error enclosure·수용 위반 0; IEEE bits/요청 gamma 일치 확인 | `evidence/exact_oracle.json` |
| Negative controls | 11/11 거절 | `evidence/native_study.json` |
| Fresh Rust 테스트 | 새 core6 + 새 chart9 + 관련 기존8 = 23 PASS | `evidence/core.stdout`, `chart.stdout` |
| fmt / affected clippy | 각 exit0 | `evidence/COMMANDS.jsonl` |
| Wolfram 보조 검산 | 정규화 identity 및 degree0–5 잔차식 True; undefined-symbol 사전 경고 보존 | `evidence/wolfram_normalized_identity.json` |
| 최초 chart RED | 6 실패·2 통과, exit101; 수정 후 통과 | `evidence/chart_boundaries_red.log` |
| 과거 full campaign | 재실행 0 | `REUSE_POLICY.json` |
| 독립 판정 | ACCEPT_NATIVE_RESEARCH; 생산/속도 HOLD | `evidence/independent_decision.json` |

모든 실행은 공급된 Rust 1.94.1과 lockfile 기준 offline dependencies를 사용했다.
118개 registry package와 4,382개 vendor file hash를 확인했다. 최초 toolchain
추출에서 truncated shared library가 발생했지만, byte-count-verified stream
추출로 복구했다. 이 환경 실패와 chart RED, 독립 리뷰 지적을 보존한다.

새 primitive 예제의 관측값은 robust wall-clock campaign이 아니다. 기존 전체
2,479 Rust PASS 등은 과거 작성자의 실행 기록이며 이번 fresh PASS 합계에
포함하지 않았다. 이번 gate는 변경된 source에 필요한 테스트·exact oracle·fmt/
clippy와 독립 검토다. 모든 과거 suite를 재실행하지 않은 것은 사용자 요청에
따른 범위 관리이며, 이 보고서를 repository-wide regression PASS로 읽으면 안 된다.

고정 holdout은 3차원·2RHS·33target에서 모두 수용됐고, 최대 residual upper는
2.1094×10⁻¹⁵로 원래 absolute tolerance10⁻¹⁰ 안에 있었다. 전체 사례별
차수·work·수용/거절은 `CASE_SUMMARY.csv`와 `RESULTS.json`에 있다.

## 10. 재현과 자료 형식

`research/rvj_integration_20261004/REPLAY.sh`는 새 native slice와 관련 경계만
실행한다. Rust1.94.1 및 dependency source를 준비한 다음 repository root에서
실행한다. 개인 절대경로나 vendor blob은 repository에 넣지 않았다.

- `AUDIT_BUNDLE.json`: 전체 판정·source·파일 포인터와 claim ceiling.
- `FINDINGS.json`: 결함, 이식 경계, proof obligation의 상태.
- `NEXT_DEVELOPMENT_DAG.json`: 실행 가능한 다음 개발 의존관계.
- `ACCEPTANCE_MATRIX.json`: 계약→코드→증거→판정.
- `RESULTS.json`, `FINAL_VERIFICATION.json`: 새 실행 집계·한계.
- `SOURCE_BINDING.json`, `REUSE_POLICY.json`: 원본 identity·재검증 사유.
- `evidence/`: raw logs, exact rational comparisons, 독립 심사와 역사 재사용 표.

## 11. 원전과 문헌 사용 범위

이번 방법을 일반적으로 새로운 matrix-function 알고리즘이라고 주장하지 않는다.
SciSpace 검색으로 후보 원전을 찾고 저자 원문 페이지를 대조했다. Shift invariance,
restarted non-Hermitian residual collinearity, unrelated RHS를 위한 block/Sylvester
정식화는 기존 연구이며 비교 baseline 선택에 사용한다. 검색 초록만 읽은 문헌은
그 범위만 표시한다. 이 문헌은 본 native 구현의 rounding certificate를 대신하지
않는다. Wolfram 결과도 새 정규화 항등식의 보조 검산으로만 사용하며 historical
정리를 다시 실행한 증거로 세지 않는다.

- Soodhalter, *Block Krylov subspace recycling for shifted systems with unrelated
  right-hand sides*, SIAM J. Sci. Comput. 38(1), 2016;
  <https://arxiv.org/abs/1412.0393> — abstract와 metadata 확인.
- Soodhalter, Szyld, Xue, *Krylov Subspace Recycling for Sequences of Shifted Linear
  Systems*, Applied Numerical Mathematics81, 2014;
  <https://arxiv.org/abs/1301.2650> — abstract와 metadata 확인. 특정 공통 augmented
  subspace/fixed-storage 조건의 한계를 모든 shift 알고리즘의 불가능성으로
  일반화하지 않는다.
- 추가 polynomial/parallel 원전과 읽기 범위는 `POLYNOMIAL_PARALLEL_KO.md`.
