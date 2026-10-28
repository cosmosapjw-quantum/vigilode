# 이전 연구의 재사용과 현재 자동 선택의 연결

2026년 10월 25일. 기준은 원격 `4fe67602502440d76aa9dd245c6b1d67558c14bf`, tree `8bab954cffcf7abd9cdd0ca528567ece47e43bc7`이다. 최신 로컬 측정 소스는 그 부모 `d914b4c8a53405ea846f710e07f77fff5f45b16f`이다. 이 문서의 실제 작업은 소스·게시 결과 열람, 기존 가정의 대응, 새 통합식 유도다. 과거 수치 실험을 재실행하지 않았다. 각 파일의 SHA-256과 구현 층위는 `REUSE_MATRIX.json`에 담았다.

## 확인된 핵심

VigilODE 고유 알고리즘은 살아 있다. 다만 현재 일반 자동 선택이 그 전체를 후보로 사용하지 않는다. `AutomaticOde`는 DOPRI와 허용된 RODAS 경로의 실측 작업량을 비교한다. 공유 단계 공간, 인과적 제거, 동시 Picard, homotopy, 몫 다항식, 회전·cofactor chart는 별도 API 또는 커널이다. 그러므로 다음 개발의 핵심은 새 방법을 이름으로 더하는 일이 아니라, 이 경로들이 공유하는 **원래 상태의 오차, 현재 연산자/모델 식별값, 실제 진전량과 총비용**을 선택기에 전달하는 것이다.

현재 인과적 투영 Rosenbrock는 기준점 JVP와 작은 LU를 사용한다. 각 현재 단계의 도함수와 Newton 반복은 없앤다. 선언 회전/감쇠 흐름, chirp·cofactor chart와 명시 RK는 우변만으로 실행된다. 잔차 우선 homotopy의 일반 보정은 필요한 경우 현재 단계 JVP/Newton을 호출한다. 이 경로별 구분을 유지하면 사용자의 목표인 “완전 무미분 우선, 필요한 위치에서만 선택적 JVP/AD”를 실제 권한·경제성 정책으로 구현할 수 있다.

## 최신 결과가 가리키는 수정 지점

게시된 로컬 후속 보고서는 AMD Ryzen 9 5900X, 단일 스레드 및 동일 물리 오차 상한에서 다음을 보였다. 이 수치는 해당 로컬 측정 소스의 결과이며 이번 연구의 새 실행 수치가 아니다.

| 대상 | 실제 관측 | 원인 구분과 조치 |
|---|---|---|
| rotation | 버린 RODAS pilot 6개가 1,150 RHS. 총 6,395 대 직접 DOPRI 5,227 RHS | 선택 결과가 명시법이어도 반복 탐색이 비싸다. 정상 구간의 불리한 후보를 억제하되 변화 검출·유한 sentinel을 유지한다. |
| gyromotion | 버린 RODAS pilot 4개가 1,019 RHS. 총 26,018 대 직접 DOPRI 24,403 RHS | 위상 정확도 요구와 강성을 분리한다. 작은 h만으로 implicit을 재탐색하지 않는다. |
| case5 | switch 1,633 RHS/232시도 대 직접 RODAS 307 RHS/39시도 | 초기 명시법의 안정성 제한을 빨리 감지하고, 선언된 cofactor가 있으면 이를 implicit보다 먼저 검토한다. |
| case5/KS 시도 trace | 각각 최근 수락 h가 선언 선형 안정 한계의 0.999668/0.999630배; 거절률 약 3%, clipped 0 | 실패는 거절 폭증이 아니라 수락된 작은 스텝의 누적이다. 거절률만을 사용하는 전환 기준이 놓친다. |
| chirp 129점 | 선언 chart 최대 오차 9.04e-9, 물리 좌표 비교군 5.83e-5; 공통 cap 1e-4 | 구조는 유효한 특수화 근거다. 두 경로의 달성 오차가 같다는 의미는 아니다. |

지속 DOPRI 세션은 rotation의 RHS를 6,720에서 6,395로 줄였다. 따라서 이미 이식된 세션 수정을 반복할 필요는 없다. 남은 비용은 모델 변화와 실제 성능 변화에 연결된 탐색 주기로 다룬다.

이때 과거의 실패를 함께 기억해야 한다. `CAUSAL_SWITCH.json`의 이전 broad 사례에서 후보 실패를 50회에서 4회로 줄였지만 총 JVP는 22,657에서 25,443으로 늘었다. changing-rate에서도 실패 33→4와 동시에 JVP 11,131→13,993이 관측되었다. 긴 cooldown이 유리해진 후보를 숨길 수 있기 때문이다. 탐색 횟수 감소를 그 자체로 최적화 성과로 삼지 않고, 현재 물리적 진전당 총비용을 함께 평가해야 한다.

## 실제 구현과 재사용할 구조

| 연구 구조 | 현재 구현 | 계산량을 줄이는 부분 | 선택기에 필요한 정보 |
|---|---|---|---|
| 인과적 공유 단계 공간 | `vigilode-coupled/src/causal.rs` | 한 번의 공통 작은 분해와 여덟 단계 풀이 | 기준점 JVP 허용, 기저 크기·누출, 원래 단계 잔차 |
| 동시 Picard 유한 전파 | 같은 파일 | 독립 RHS 묶음의 의존 깊이 단축 | 파동 수, 실제 RHS 비용, 동기화·추가 평가 비용 |
| 잔차 우선 homotopy | `residual_first.rs`, `solver.rs` | 이미 충분한 근에는 현재 단계 도함수 생략 | λ=1 원래 잔차, 도함수 권한, 보정 예산 |
| 낮은 차수 몫 대수 | `polynomial_resolvent.rs` | 일반 분해를 짧은 작용과 결함 보정으로 대체 | 전체 공간 관계 상한, h, 분모 여유, 재사용 횟수 |
| 회전·감쇠 quotient flow | `quotient_flow.rs` | O(n) 정확 선형 흐름과 계수 공유 | 선언된 split, 변환 후 나머지의 변화율 |
| Involution/chirp | `oscillatory_chart.rs` | ±sector 계수 공유 및 빠른 carrier 제거 | sector/charge, 가환성, 물리 norm과 위상 오차 |
| 불변 ideal/cofactor | `cofactor_chart.rs` | 강성 횡방향 모드와 초기 충격을 해석적으로 처리 | 정확한 초기 진폭, 재구성식, 모델 epoch |
| Fourier–Volterra/FFT | `fourier_path_candidate.rs` | sparse 곱과 padded FFT 사이의 convolution 비용 선택 | harmonic·차수 support, padding, 인증 비용 |
| Fourier 원래 목표 승인 | `fourier_path_certificate.rs` | 비싼 정확성을 후보 생성과 분리 | 현재 경로·초기값·h·phase·epoch 결속 |
| Laguerre signed adjoint | `laguerre_adjoint.rs` | 부호 소거를 살려 recurrence 오차 상한 축소 | spectrum·다항식 차수, 나머지 오차 항 전체 |
| Confluent jets/shared shift | Loop11 연구노트 및 native 특수 구성 | 근접/중복근 계수와 실제 다중 shift 연산 공유 | 중복도, 공통 RHS rank, cluster·pole 여유 |
| 매개변수 certificate atlas | `CERTIFICATE_ATLAS_KO.md`의 유도 | 셀 내부의 역연산자 검증 재사용 | 모델·metric·operator identity와 셀 소속 |
| 폐쇄 moment/affine lift | 기존 유도와 특수 client | 작은 폐쇄 비선형 상태의 적분 | closure, transverse 초기값, 전체 상태 출력 여부 |
| Schur/nilpotent | 기존 유도·특수 커널 | band+작은 border의 직접 축약 | 정확/근사 블록 오차, 가장 강한 직접 풀이 비용 |

일반 자동 선택과 이 모든 provider를 연결하는 통합 레지스트리는 아직 완성되지 않았다. 표는 API와 커널 및 유도를 구분한 재사용 지도다. 실제 신규 통합 범위는 이번 연구의 코드 diff와 검증 기록으로 별도 판단한다.

### 과거 FAIL의 정확한 범위

PP05/PP06은 전체 아이디어가 반증된 기록이 아니다. PP05의 native/source 상한 비율 목표 1.01을 tight 사례에서 1.011159로 넘었고, alias negative control이 충분히 나쁘지 않아 정확한 후보가 승인되었다. 기존 enclosure 검사는 해당 corpus에서 참이었다. PP06의 FFT/direct 차이는 약 5.3e-16이었고, 기존 alias 사례 역시 물리 오차를 상한 안에 유지했다. 따라서 원래 FAIL을 보존하면서 더 강한 alias 입력과 full-product padding 계약을 재사용한다. 이 두 구현을 다시 미구현이라고 제안하지 않는다.

다항식 결함 보정은 새 LU 대비 이득을 보인 커널이지만 가장 강한 재사용 LU 비교군에는 졌다. 비용 모델에 이미 존재하는 factor reuse를 반드시 포함해야 한다. stage parallelism도 같은 방법의 1→2스레드 이득만으로 선택하면 안 된다. 과거 expensive-few-mode는 RHS마다 2,048개의 sin 연산을 넣은 합성 비용 모형이었다. Picard와 residual-first가 그 환경의 자기 직렬 경로보다 빨랐지만 조밀 직접 비교군보다 느렸다. 이를 실제 응용 가속으로 승격하지 않는다.

## 적응 제어와 구조 선택을 직접 연결하는 구성

### 1. 스텝 크기에 따른 몫 다항식 풀이의 적격 영역

선언된 연산자가 A=−αI+E, α≥0이고 ||E||≤e를 같은 노름에서 안다고 하자. Rosenbrock 단계에 B=hγA, h>0, γ>0를 쓰면

\[
P_h=\frac{I}{1+h\gamma\alpha},\qquad
D_h=I-(I-B)P_h=\frac{h\gamma E}{1+h\gamma\alpha}.
\]

따라서 η(h)=hγe/(1+hγα)는 결함 노름의 상한이다. 원하는 내부 수축 상한 0<η_*<1에 대한 적격 영역을 나누면

\[
 e\le\eta_*\alpha \Rightarrow \eta(h)\le\eta_*\quad(h>0),
\]

\[
 e>\eta_*\alpha \Rightarrow
 h\le\frac{\eta_*}{\gamma(e-\eta_*\alpha)}.
\]

증명은 양수 1+hγα를 곱하여 hγ(e−η_*α)≤η_*로 정리하면 된다. 적응 제어기가 h를 바꿀 때마다 새 JVP로 구조를 탐색하지 않고, 이 식과 epoch로 기존 provider의 적용 가능성을 판정할 수 있다. 이 h 범위는 **내부 선형 풀이의 적격 조건**이다. 시간 이산화 오차는 기존 embedded 또는 검증된 defect 예산으로 따로 제어한다.

k번 결함 보정 후 ||r_k||≤η(h)^k||r_0||다. 원래 상태 출력에 대한 인과적 잔차 증폭 상한 G(h)가 있으면 G(h)||r_k||를 내부 오차 예산에 맞추어 k를 고를 수 있다. k가 늘어나는 원인이 h 변화인지, 구조 상한 악화인지, 실제 정체인지 구분하여 selector에 넘긴다. 이로써 inner solve를 불필요하게 정밀하게 풀거나 그 비용을 물리적 강성으로 오인하는 일을 줄인다.

### 2. Carrier 제거 후의 시간 해상도를 선택 기준으로 사용

회전된 비선형 항은

\[
 e^{i\phi}N(e^{-i\phi}w,e^{i\phi}\bar w)
 =\sum_{p,q}c_{pq}e^{i(1-p+q)\phi}w^p\bar w^q.
\]

charge p−q=1인 항은 빠른 phase를 제거한다. 다른 charge는 남은 빠른 phase를 그대로 운반한다. 따라서 원 RHS의 고주파 하나로 Lawson을 선택하지 말고, 선언된 charge support와 변환 후 실제 error/work를 사용한다. 고주파 자체가 원래 상태에서 작지 않은 위상 오차를 만들 수 있으므로 chart inverse의 물리 norm과 phase 불확실도도 동일한 성공 판정에 포함한다.

시간 의존 chart에서는 고정 affine lift의 정리를 그대로 재사용하면 안 된다. chirp의 p=(v+γx)/ω(t)는 −(ω′/ω)p를 추가한다. 이 항은 실제 구현에 포함되어 있으며, 구조 provider의 식별값은 이 시간 의존식까지 포함해야 한다.

### 3. Homotopy와 병렬화는 동일한 수치 목표의 실행 정책

엄밀한 하삼각 단계 의존성과 가역 W_m에서 인과적 전진 풀이는 유일한 투영 단계근을 직접 구성한다. 같은 목표의 동시 Picard는 초기 첫 단계가 정확하면 최대 일곱 번 갱신으로 모든 단계에 정확한 정보가 전파된다. 이 유한 전파 결과는 수축 상수와 무관하지만 실제 반올림·투영 잔차 검사는 남는다.

순차 전진의 새 RHS가 일곱 번이고 b개의 동시 RHS 묶음이 필요하다면, p작업자 병렬 경로의 비교 모델은

\[
 b\lceil7/p\rceil C_f+bC_{sync}+C_{small}<7C_f+C_{forward}.
\]

공통 원래 잔차 검사는 양쪽에서 동일하게 계상하고, 공통이 아닌 준비·검증·fallback은 따로 더한다. 이 부등식은 선택기를 위한 예측이며 실제 wall time의 대용 증명이 아니다. causal-forward를 먼저 비교 대상으로 두면 불필요한 homotopy/Newton 보정을 수행하지 않고, Picard가 이길 작업량에서만 병렬 실행을 탐색할 수 있다.

### 4. 여러 provider가 공유할 최소 신호

구조가 성립하는지와 그 구조가 현재 빠른지를 다른 판정으로 유지한다. 공통 record는 다음을 포함한다.

- 원방정식·callback·operator·chart·metric·worker의 epoch 및 원래 상태 checkpoint.
- 현재 실제 h와 종료 시각, clipping 사유, 시도/수락 횟수 및 실진전 Δt.
- 원래 물리 norm의 error/defect, phase·reconstruction·inner solve 항의 출처.
- RHS/JVP/행렬·분해·근 보정/인증·할당/동기화 및 전체 wall time. 권한 없는 연산은 비용 비교 전에 제외.
- 실패 이유: 수학 조건 불일치, 시간 오차, 내부 잔차, 모델 변화, 자원 한도 또는 성능 열세.

h·상태 변화로 경제성 근거를 갱신할 수 있지만, 모델/metric/chart identity 불일치를 통계적 hysteresis로 덮어서는 안 된다. 반대로 엄밀 구조가 계속 유효해도 경제성은 workload 변화로 달라질 수 있다. 기존 `CausalReferenceGuard`의 h/state/age/model-epoch 조건과 일반 switcher의 지속 세션·attempt telemetry를 연결하는 것이 최소 변경 경로다.

## 이전 PR와 첨부 자료의 범위

PR #74의 native coupled stage homotopy, #75의 residual-first 경제성, #77의 hybrid structural packet·선택적 AD, #78의 원본 archive 보존은 현재 소스·유도와 연결된다. Fourier/involution의 유일한 원 PR 번호는 이전 검색에서도 확인되지 않았으므로 추정 번호를 붙이지 않았다. 본 조사는 저장된 원문·검증 데이터와 현재 코드의 대응을 읽었으며 전체 과거 PR diff를 새로 재감사하지 않았다.

별도 BASS/REI 계열 `72836e0290b66d33f4ca1771284b1c8070669a74`는 이미 기록된 D2–D5 원자료를 재사용했다. 공통 solver에 유용한 내용은 단계 전 domain 검사, 물리 norm의 lift, 수락 후 효과 commit, replay-safe shadow 계약이다. 해당 branch의 응용 물리 완성도나 전체 테스트를 일반 ODE solver 성능 근거로 이전하지 않았다.

이번 첨부물에서 CMB 감사 템플릿의 대상 보존·원래 observable 비교 원칙은 활용했다. `report(1).pdf`의 실제 제목은 HTT 수학·물리·통계 종합 이론 보고서이며 처음 두 페이지와 metadata를 확인했다. `literatures.zip`과 `papers.zip`의 목록은 우주론/운동론 원전이다. 제공된 이름 및 검사한 두 ZIP 목록에서 `two_step...`/`curved_mirror...` 원문은 확인되지 않았다. 따라서 Loop11의 이전 원본 source mapping과 그 가정이 유지되는 수학식을 재사용했고, 새 원문 열람을 했다고 표시하지 않았다.

## 다음 판별 실험

1. 정상 rotation에서는 불리한 implicit pilot 비용을 줄이되, 위치가 다른 smooth stiffness 전이에서는 유한 시간 안에 재탐색하는지 확인한다. 초기부터 강한 case5에서는 수락된 작은 h의 작업량을 근거로 진입 지연을 측정한다.
2. 차원·시간 구간·원래 상태가 같은 checkpoint에서 실제 선택 경로와 유력 대안 하나를 비교한다. 성공한 목표 구간, 물리 cap, 원자료·바이너리·epoch가 같아야 비용을 비교한다.
3. quotient provider는 동일 W에서 factor reuse가 가능한 강한 직접 풀이와 비교한다. 별도의 합성 h–η 스캔은 위 적격식 검산으로 표시하고 실제 ODE trajectory speed와 구분한다.
4. Picard 병렬 후보는 먼저 causal-forward, 다음으로 실제 더 강한 직접 풀이와 비교한다. 합성 비싼 RHS와 실제 격자 RHS를 다른 결과표에 둔다.
5. 모델·chart·metric/worker epoch 변경, source split 불일치, 비가환 carrier, harmonic alias, 거의 합쳐진 근, 내적 norm 변화가 각자 어떤 재평가를 일으키는지 계약시험으로 확인한다.

이 연구에서 채택하는 기여는 기존 대수적 구성을 실제 비용·오차·권한의 공통 판단으로 연결하는 것이다. 새 학술적 최초성 또는 전 문제에 대한 성능 향상은 이 파일에서 주장하지 않는다.
