# VigilODE 적대적 감사·이론 연구·성능 개선

기준: `3064d453091af1dec7f99f0c8afabd98b447ea34` (`research/adapter-checkpoint-followup-20261027`). 작업 브랜치: `research/rigorous-performance-audit-20261027`.

## 결과와 판정

수치 결과를 유지하는 코드 수정, adapter 비교 비용의 조기 종료, 엄밀한 조건부 정리와 실행 가능한 연구 primitive를 완료했다. **실제 프로파일링에서 명령어 작업량은 감소했지만, 이 공유 호스트의 반복 시간 측정으로 안정적인 벽시계 가속률은 입증하지 못했다.** 두 사실을 구분해 보고한다.

| 새로 실행한 Callgrind 비교 | 기준 Ir/풀이 | 수정 Ir/풀이 | 감소 |
|---|---:|---:|---:|
| Faer LU kernel, n=32 | 13,489.06 | 11,709.05 | 13.196% |
| 전체 dense RODAS 궤적, n=65 | 59,721,066.25 | 58,165,078.00 | 2.605% |
| 전체 adapter 풀이, n=16 | 4,139,254.63 | 4,067,621.88 | 1.731% |

각 수치는 두 반복 길이의 총 명령어 수 차이로 초기화·진단 고정비를 제거한 값이다. Callgrind 3.22.0의 계측이며 native CPU의 SIMD dispatch, IPC 또는 초 단위 가속률로 동일시하지 않는다. 전체 profiler 원자료와 실행 명령을 보존했다.

Adapter n=8에서는 work가 2,119→2,102, RHS가 1,120→1,112로 줄었다. n=16에서는 work가 2,655→2,605(**1.883% 감소**), RHS가 1,216→1,200, attempts가 182→181로 줄었다. 두 경우의 **전체 accepted 시간·상태 궤적이 동일**하고 모든 반환 성분의 정확해 오차는 약 `6.1006e-8`로 공통 상한 `1e-5`를 충족했다. n=1은 정확도·동일성에는 통과했으나 새 조기 종료가 발동하지 않아 시간 개선 셀로 채택하지 않았다. Sentinel 대조에서는 상태와 카운터가 모두 같았다.

Dense n=65, 96, 192는 33개 공통 관측 시각의 모든 상태값·카운터·attempt 수가 기준 바이너리와 동일했다. 최대 성분 오차는 각각 약 `4.595e-10`, `4.617e-10`, `4.605e-10`으로 상한 `1e-7` 이내다. 이는 관측 격자 검증이며 연속 시간 supremum의 인증은 아니다.

LU 재사용 버퍼의 7개 차원(1, 2, 7, 32, 65, 96, 192) 검증에서는 기존 풀이와 비트 단위로 같았고 준비된 버퍼의 solve 구간에서 추가 할당은 **0회**였다. 작은 dense 기본 경로는 이미 최적화되어 있으므로 이번 Faer 개선이 n≤64 기본 RODAS 전체에 적용된다고 주장하지 않는다.

## 시간 결과를 어떻게 읽어야 하는가

첫 짧은 배치의 모든 유효 셀에서 5개 paired block의 시간 비율 범위가 1을 가로질렀다. 이를 확인한 뒤 한 번만 보완 측정을 등록하여 약 1초 배치와 process CPU 시간을 사용했다. 이 보완 측정에서도 모든 범위가 1을 가로질렀다.

| 보완 측정 | 벽시계 기준/수정 중앙 비율 | block 범위 | process CPU 중앙 비율 |
|---|---:|---:|---:|
| LU n=32 | 1.126 | 0.647–1.401 | 1.125 |
| dense 전체 n=65 | 1.033 | 0.626–1.202 | 1.028 |
| dense 전체 n=192 | 0.923 | 0.585–1.244 | 0.935 |
| adapter 전체 n=16 | 0.980 | 0.780–1.087 | 0.970 |

비율이 1보다 크면 수정 쪽이 빠르지만, 이 표는 안정적인 우열을 입증하지 않는다. 특히 n=192의 첫 중앙값 1.252가 보완 측정에서 0.923으로 반전됐다. 첫 결과만으로 25% 가속을 주장하는 것은 부당하다. Process CPU는 명령 초기화·진단 풀이·직렬화까지 포함하며 내부 적분 전용 CPU 시간과 다르다. 세 번째 시간 측정은 하지 않았다.

시간 두 패스와 명령어 프로파일링의 실제 경과 시간은 합계 약 167.86초로 등록한 300초 이내다. 컴파일·검증 명령은 약 821.51초로 900초 이내였다. Python, C++와 복구한 Rust 1.94.1 runtime을 실제 사용했다. 초기 손상된 LLVM 배포본과 공유 target의 stale artifact 때문에 발생한 빌드 실패도 보존했다.

## 최신 보고서를 비판적으로 재사용한 방식

`external_reports/adapter_profile_audit_20261027`의 31개 항목과 현재 코드를 대조했다. 그 packet은 `46939bbe`/AMD 5900X의 결과이며 현재 Intel Xeon 8573C 실행과 섞지 않았다. 기존 return sentinel, incomplete comparator의 비용 하한 판단, 약 90→3회 MF 할당 개선은 이미 기준 코드에 있으므로 새 성과로 세지 않았다.

기존 rotation·gyro의 초과 명령어 중 losing entry pilot 몫 85%·80%는 **adapter 비용**이다. 이는 `adaptive_evidence(ProvidedOnly)` opt-in의 관측이고, DOPRI-only 기본 설정의 비용이 아니다. 별도로 작은 case5에서 MF가 dense보다 약 6.9배 비싼 명령어 비용, least-squares correction 39%와 그 안의 QR factorization 18%는 **모드 내부 비용**이다. 후자의 18%도 같은 MF 전체 instruction denominator를 사용한다. 이번 할당 한 곳 제거가 그 격차를 모두 닫았다고 해석해서는 안 된다.

기존 보고서의 기본 gyromotion 오차 약 `3.16e-2`는 물리 상한 `1e-4`를 넘는다. 두 tolerance의 오차비에서 얻은 기울기 약 0.97은 경험적 관측이며 일반적인 위상 정리가 아니다. 현재 이상적 rational DOPRI5 tableau에서 안정성 다항식과 위상식을 직접 유도하여 그 적용 조건을 분리했다. 물리 상한을 넓히지 않았고, 기존 물리 캠페인을 재실행하지 않았다.

## 코드 수정

| 계층 | 변경 | 남은 제한 |
|---|---|---|
| adapter/switcher | `stop_dominated_return_comparator`로 이미 비용상 패배한 ordinary return comparator 중단 | 기본 false. 반복 losing entry pilot, horizon, cold seed 전체를 해결한 것은 아님 |
| evidence | `ExplicitCheaperThanComparatorLowerBound`, incomplete challenger ratio 제거, 완료 후보/미완료 비교 ratio의 외향 상계 | 새 enum과 config field는 외부 exhaustive match/struct literal 조정이 필요할 수 있음 |
| Faer dense | `LuSolveWorkspace`로 aligned RHS·permutation scratch 재사용 | factorization 재사용과 다름. O(n²) solve와 O(n³) factorization 산술은 남음 |
| MF RODAS | analytic supplied partial-time callback을 기존 `eval_partial_t_into`로 직접 실행 | FD partial_t, JVP state copy/Arc 할당은 남음 |
| coupled residual | row를 Vec에 복사하지 않고 동일 compensated dot에 빌려 전달 | projected dimension d당 d회 할당과 d²개 복사 제거. basis/인증 수학은 변하지 않음 |

모든 incomplete comparator 판단은 별도 prefix-cost predicate를 사용하며 미완료 결과를 적합한 출력으로 취급하지 않는다. Checked u128 합과 외향 반올림은 일반 정수의 정확한 동률에서도 보수적으로 incumbent를 유지할 수 있다. 이미 관측한 fatal callback은 우선하며 부분 implicit 상태는 commit하지 않는다. 생략한 후반부의 미래 fatal을 예측한다고 주장하지 않는다.

## 이론 결과와 실제 이식 조건

### 1. 비용 하한에 의한 비교 중단

같은 checkpoint·구간·출력 계약의 완료 후보 비용이 E, 비교자의 현재 비감소 prefix 비용이 L이고 0<ρ<1이면, E≤ρL에서 모든 완료 비용 I≥L에 대해 E≤ρI가 따른다. 이는 후보가 이미 충족한 정확도 계약을 보존한다. Native local-tolerance 성공을 물리 전역오차 인증으로 승격시키지는 않는다. 최신 코드에 이미 있던 논리를 보수적 산술과 proactive stop으로 연결했다.

### 2. 소산 shifted system의 minimax scalar relaxation

고정 양의 Hilbert metric M에서 Re⟨v,Jv⟩M≤0이고 a,a₀>0일 때 native 시스템 P=I/a₀−J, W=I/a−J에 대해

ω*=2a/(a+a₀),  q*=|a−a₀|/(a+a₀)<1

를 사용한 `x <- x + omega*P^{-1}(b-Wx)`는 정확한 역연산에서 q*로 수축한다. 핵심 항등식은

`I−ω*P^{-1}W = ((a−a₀)/(a+a₀)) [2(I−a₀J)^{-1}−I]`.

소산성으로 reflected resolvent의 노름이 1 이하이며, J=0과 J→−∞의 두 극한을 균형화하면 minimax sharpness가 따른다. Reflected-resolvent contractivity 자체는 기존 연산자론의 사실이다. 새 일반 수학의 발견으로 주장하지 않고 native scaling과 구현 가능한 재사용 후보에 연결했다.

J=J₀+Δ, cached dissipation margin α≥0에는 `q <= q* + ω*a₀||Δ||/(1+a₀α)`가 필요하다. 실제 scaled residual `r=b-Wx`의 상태 오차에는 `a/(1+aα')`를 곱해야 한다. 시간 단위를 가진 a를 빼거나 normalized residual과 혼합하면 잘못된 bound다. Metric이 물리 노름과 다르면 conditioning 비용도 운반해야 한다.

`theory/cayley_reuse.rs`는 overflow-safe scalar 식, 외향 상계, 모델/metric/domain identity, drift, scaled-residual 변환을 제공하며 7개 Rust 시험을 통과했다. 선언한 full-domain bound의 진실성이나 외부 LU action·residual evaluation의 오차를 자동 인증하지 않는다.

27개 실제 cached-LU 실험에서 finite operator-norm/identity checks는 모두 diagnostic tolerance 이내였다. 그러나 residual `1e-10` 목표는 **25/27만 달성**했다. ratio=10의 두 실패는 그대로 남겼으며, 다른 셀에서 반복수가 증가했다. 따라서 생산 기본값에는 이식하지 않았다. 이론적 균일 수렴영역과 실제 경제성은 다른 판정이다.

### 3. 근사 역연산을 포함하는 causal stage 오차

각 stage에서 `||I−R_i W_i|| <= δ_i < 1`, `||R_i r_i|| <= d_i`, 선행 stage에 대한 preconditioned coupling K_ij가 유효한 domain에서 성립하면

`E_i = (d_i + sum_{j<i} K_ij E_j)/(1−δ_i)`

가 stage 오차의 상계다. 유한한 causal 순서를 따라 귀납하므로 interstage 행렬 전체에 대한 contraction은 필요 없다. 기존 causal majorant를 재사용하면서 approximate inverse의 결함을 명시한다. 출력 가중합의 추가 오차가 O(h^(p+1))이고 ideal map의 안정성과 local defect가 통제되면 discrete Gronwall로 p차 정확도가 유지된다. 강성에 균일한 정확도에는 상수도 강성에 균일해야 한다.

반례 `J=[[-1,10^8],[0,-1]], b=(0,10^-8)`에서는 zero candidate의 residual이 `10^-8`이어도 `(I-J)^(-1)b=(1/4,1/(2×10^8))`이다. 음의 고유값과 작은 residual만으로 작은 forward error를 주장할 수 없다. 현재 coupled code가 operational residual을 별도 표시하는 것은 올바른 구분이다.

### 4. 원래 물리 좌표로의 오차 운반

`y=S(t)z+c(t)`와 정확한 transformed RHS이면 `d_y=S d_z`이다. 구현 RHS의 차이가 η이면 `d_y=S(dtilde_z+η)`다. 물리 전역오차에는 `||S||`, model defect, reconstruction rounding/jump, 전체 tube의 one-sided Lipschitz 또는 flow sensitivity를 적분해야 한다. Endpoint 샘플이나 작은 transformed residual은 이 의무를 대신하지 못한다. 비가환 time-dependent carrier 확장에는 time-ordered transformation 또는 그 defect bound가 필요하다.

### 5. DOPRI5의 조건부 위상식

이상적 rational tableau의 정확한 연산에서 `R(z)=sum_{k=0}^5 z^k/k!+z^6/600`, `log R=z+z^6/3600−z^7/2100+...`이다. y₀≠0인 상수 주파수 oscillator, 상수 step θ=ωh에 대해 N step의 log-amplitude defect는 `−Nθ^6/3600+O(Nθ^8)`, phase defect는 `+Nθ^7/2100+O(Nθ^9)`다. 이는 adaptive gyromotion의 일반 tolerance 법칙이나 binary64 runtime의 비트 정확 다항식이 아니다.

## 아직 별도 연구가 필요한 부분

수학에서는 full-domain provider, inverse-action defect, tube/radius closure, 물리 노름으로의 reconstruction과 propagation이 가장 중요하다. Krylov 투영을 임의로 기존 Rosenbrock tableau에 끼우면 order가 유지된다고 가정할 수 없으므로, 실제 stage defect를 계측·상계하거나 W/K order theory와 일치시켜야 한다.

코드에서는 FD Jacobian/partial_t 버퍼, 재사용 가능한 JVP operator, persistent implicit session, losing entry-pilot scheduling, 작은 시스템의 명시적으로 허용된 dense assembly, rank-aware least squares가 남아 있다. Work proxy와 diagnostic JVP/callback ceiling 변경은 함께 설계해야 한다. 이번 profile 수치를 모든 문제 크기·provider·host에 일반화하지 않는다.

V8 query membership의 기존 한정 성과와 그 결손은 그대로다. uniform query rounding, actual ODE enclosure, full-domain 물리 인증을 이번 solver 성능 결과로 닫지 않았다. 기존 중앙 현황 27/88, global physical completion null, L4 false를 변경하지 않았다.

## 검증·산출물

- Native targeted tests: **51 PASS** = mode 32 + adapter 19. Composite LU test 안에 7개 차원이 포함된다.
- 독립 research Rust primitive: **7 PASS**.
- Shifted-system finite checks: 27; strict residual success: 25; 실패 2 보존.
- Dense 관측 동일성: 3개 차원 × 33개 공통 시각, 모든 성분과 native counters.
- 시간 실험: 첫 pass 및 단 한 번의 confirmation; 모든 raw sample, 실패/무효 cell 보존.
- 명령어 profile: 3개 cohort, 두 repetition lengths, 원시 Callgrind 파일 보존.
- 실제 REVTeX 4.2f 보고서: `report/vigilode_audit.tex` 및 생성된 PDF. 문헌은 Tranquilli–Sandu의 Rosenbrock–Krylov 원전과 Liu–Moursi–Vanderwerff의 reflected resolvent 원전을 확인했다.
- 원문 감사: `ADAPTER_AUDIT.md`, `MODE_AUDIT.md`, `theory/THEORY_AUDIT.md`. 저자 self-review와 같은 맥락의 상호 검토를 독립 이종 모델 검증이라고 부르지 않는다.

모든 production 수정은 별도 연구 브랜치에 있으며 기존 owner branch나 닫힌 물리 캠페인을 덮어쓰지 않았다. 연구용 contraction primitive는 production selector 기본값으로 활성화하지 않았다.
