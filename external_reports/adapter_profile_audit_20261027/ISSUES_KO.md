# VigilODE adapter·switcher 잔여 문제 목록

2026-10-27 · 기준 commit `46939bbe` (`research/adapter-checkpoint-followup-20261027`) · AMD Ryzen 9 5900X, 단일 스레드, core 11 고정 · Rust 1.94.1 locked offline

모든 벽시계 시간은 진단용(`diagnostic_only`)이다. 판정 근거는 결정론적 작업량(WorkCounters), callgrind 명령어 수(Ir), heaptrack/계수 할당자 할당 수다. 수치는 v2(`46939bbe`) 재측정값이며, v1 workflow 분석에서만 온 값은 "(v1)"로 표시했다. 선택기 항목(B1·B3·B7·B8·C4·C9)은 opt-in preset `SwitchingConfig::adaptive_evidence(ProvidedOnly)`에만 해당한다. 기본 `SwitchingConfig::default()`는 DOPRI 전용이라 pilot·재확인을 하지 않는다. 각 항목의 증거, 위치, 수정안, 완료 기준 원문은 [ISSUES.json](ISSUES.json)에 있고, 분석과 적대적 검증의 원 기록은 [workflow/WORKFLOW_RESULT.json](workflow/WORKFLOW_RESULT.json)에 있다.

## 한눈에 보기

| ID | 등급 | 상태 | 문제 | 측정된 크기 | 위험 분류 | 규모 |
|---|---|---|---|---|---|---|
| A1 | 차단 | 확인 | 이식된 packet 노드의 거버넌스 gate가 red | `check-research-node --base e8572eac` FAIL 4건, authority-ref FAIL 18건(joint_controller가 로컬·원격 추적 ref 어디에도 없는 `040b6764` 인용; 복구 가능성 낮음). 저장소 전체 authority-ref는 다른 11개 노드의 기존 FAIL 310건으로 계속 red | api/governance | M |
| A2 | 차단 | 확인(기지, L-0172 G2가 잔여로 허용) | matrix-free 재선형화마다 heap 할당(VG-1) | 시도당 case5 3.0회(+고정 73), transition-39 2.55회, gyro-tight 1.97회; dense는 평탄(`ALLOC_SWEEP.json`) | 첫 사이트 bitwise-safe·S, 둘째 사이트 api/governance·M | S+M |
| A3 | 차단 | 확인(기지, 이번에 첫 계측) | 기본 FD Jacobian 경로가 Jacobian 구성마다 할당(VG-1) | 구성당 4.0회(case5 63→82→122→162→198 at 5/10/20/30/39 시도), gyro-tight 3.93회(`ALLOC_SWEEP.json`) | bitwise-safe | S |
| A4 | 차단 | 확인 | 선택기 루프에 VG-1 계약이 없음 | 정상상태 할당/solve: decay 78, stiff-forced 342, rotation 478, case5 595, transition-39 3,041 (direct DOPRI 16, dense RODAS 63) | api/governance | M |
| A5 | 차단 | 확인 | lower-bound 복귀 승리가 완료 비교 evidence로 기록되고, 미완료 pilot이 완료 비교 같은 비율을 게시 | rotation pilot 비율 0.984·gyro 0.932, 실제 prefix 진행 <0.05%·<5% | api/governance | S |
| A6 | 차단 | 부분 | 정확도 gate가 종료점만 검사 | 31/34 packet 기록, G3·G4 모두 종료점 전용 | bitwise-safe | M |
| A7 | 차단 | 부분 | 기본 허용오차 gyromotion 실패는 위상 표류 | 오차 3.16e-2 vs 상한 1e-4, 상대 위상 오차 ≈ 92×rtol; 기본·엄격 두 점의 오차비 811(허용오차비 1000, 기울기≈0.97) | policy | M |
| B1 | 큰 비용 | 확인 | 안정 explicit 구간에서 가망 없는 MF entry pilot 반복 | 선택기−직접 Ir 차이 중 pilot 몫: rotation 85%, gyro-tight 80%, gyro-default 88% | policy | M |
| B2 | 큰 비용 | 확인 | 작은 n의 matrix-free RODAS가 같은 callback의 조립 dense보다 크게 느림 | Ir/solve: case5 MF 4.60M vs dense 0.67M(6.9×) vs FD-dense 0.74M; gyro-tight MF 2,160M vs FD-dense 332M(6.5×); v2 시간 최소값 5.9×·8.1× | api/governance | M |
| B3 | 큰 비용 | 부분 | entry pilot이 opts.h0에서 냉시동 | 반사실 replay: transition-39 WU −21~22%, transition-79 −28% | policy | S |
| B4 | 큰 비용 | 부분 | transition entry horizon 누락(기지) | B3 이후에도 [0.40452, 0.42610]에 DOPRI 약 6,144 WU 잔존 | policy | L |
| B5 | 큰 비용 | 확인 | 고정 작업량 대리지표가 MF를 싸게 평가 | ns/WU(v2 최소값): DOPRI 40–100, dense 43–92, MF 180–292; case5 MF/dense WU 2.6× vs 시간 5.9× | api/governance | M |
| B6 | 큰 비용 | 확인 | GMRES 최소제곱의 소형 column-pivot QR | transition-39 MF RODAS: 최소제곱 보정 전체 39%(48.5M/122.8M), 그중 QR factor 18%(22.3M); Givens 대체 절감 상한 약 33%(추정) | solution-changing(비 bitwise) | M |
| B7 | 큰 비용 | 부분 | 강성 진입 전 32회 explicit 배치 + 냉시동 pilot | stiff-forced adaptive 588.6k vs RODAS 176.7k Ir(3.33×), 초기 초과 약 283 WU | policy | S |
| B8 | 큰 비용 | 확인 | 이미 통과 불가능한 pilot이 계속 진행 | rotation pilot 568 WU 중 456 WU 절감 가능(결정 동일은 논증, 전 cell 시험 필요) | counter-changing | M |
| B9 | 큰 비용 | 부분 | implicit 배치·pilot마다 RODAS facade 전체 재구성 | 호출당 약 16.1k+8.5k Ir, 약 63 할당; stiff-forced adaptive solver Ir의 17% | bitwise-safe | M |
| C1 | 중소 | 확인 | MF facade가 RecomputeFinal 회계로 진단용 JVP 중복 | jvp −20~26%, Ir −5.4~8.7% (B5와 함께만 채택) | counter-changing | S |
| C2 | 중소 | 확인 | GMRES 호출별 부가 비용(전체 counter delta, 중복 norm, 단계별 검증 등) | n=2 MF Ir 약 7–9% | bitwise-safe | S |
| C3 | 중소 | 확인 | explicit 배치·tenure 기록 비용(Box stats, clone, snapshot 3회) | decay adaptive 123.7k vs DOPRI 77.6k Ir(1.59×, v2); 기록 비용 약 33.6k(v1 116.4k 기준 분해) | bitwise-safe | M |
| C4 | 중소 | 확인 | pilot ceiling은 callback만, 판정은 solve까지 셈 | 패배 pilot WU의 14–19%가 ceiling 밖 linear solve(rotation 105/568, transition-39 약 257/1,613, gyro-tight 125/896) | counter-changing + api/governance | M |
| C5 | 중소 | 부분 | callback·JVP wrapper 계층 비용 | dense callback당 약 193 Ir(사용자 RHS 174), MF JVP당 690 Ir (v1) | bitwise-safe | M |
| C6 | 중소 | 부분 | dense 드라이버가 SPD01 검증 가속(사전검증 제어기)을 쓰지 않음; SPD06은 small 드라이버에서만 검증 | 시도당 167 Ir(1.3%, v1); SPD06 dynamic-n 이식 최대 12–26%(미검증 가설) | api/governance | M |
| C7 | 중소 | 부분 | 지속 DOPRI session이 직접 경로보다 명령어 약 1–2.5% 많음 | rotation +2.4%, gyro-tight +1.2% Ir (v1); 시간 효과 미증명 | bitwise-safe | M |
| C8 | 중소 | 부분 | DOPRI 착지 보조함수 호출 비용 | 저비용 RHS에서 perf self 약 20%, inline 회수 ≤ 약 3% | bitwise-safe | S |
| C9 | 중소 | 부분 | 복귀 재확인이 opts.h0에서 냉시동, 일부는 통과 불가능 | 재확인당 25–97 WU | counter-changing | S |
| D1 | 측정 | 부분 | 시간 분류가 host 의존이고 label이 빠른 쪽을 숨김 | packet 26쌍 중 15 미판정, 12 cell host 간 분류 변경; v2에서도 34 cell 중 9개 spread >5% | api/governance | S |
| D2 | 측정 | 확인 | trace 도구 결함(정규화 없는 동일성, 반올림 기반 landing_clipped, entry_point 문자열) | case5 clipped 37,553 표시 vs 실제 1 | bitwise-safe | S |
| D3 | 측정 | 부분 | 선택기 출력 행이 32회 배치에 종속, 착지 비용 분리 안 됨 | rotation 33행/8 window, transition-39 148행 | api/governance | S |
| D4 | 측정 | 부분 | trace 정확도는 max-norm, 제어기는 WRMS | case5 복귀 window: WRMS>1 4.2%(최대 1.076), max-norm 최대 1.52 | bitwise-safe | S |
| D5 | 측정 | 부분 | return sentinel의 양의 근거가 사후 fixture 하나 | sentinel/adaptive 작업량: case5 1.022, stiff-forced 1.039, transition-39 1.074 | policy | M |
| D6 | 측정 | 확인 | 등록 fixture가 모두 n≤6·저비용 RHS | 고정 오버헤드 순위와 대리지표 가중치 일반화 불가 | api/governance | M |

## A. 승격 차단: 규칙·정확성·증거 무결성

- **A1 거버넌스 gate.** `research/adapter_integration_20261027`과 `research/joint_controller_20261025`에 PREREGISTRATION과 ledger 행이 없다. joint_controller는 확인한 모든 로컬 clone과 원격 추적 ref에 없는 commit `040b6764`를 18회 인용한다(복구 가능성 낮음, 불가능이 증명되지는 않음). 저장소 전체 `check-authority-refs`는 다른 11개 노드의 기존 FAIL 310건 때문에 이 항목을 해결해도 red로 남는다. 이번 v1 진단 자료도 미커밋 harness로 만들어졌으므로 폐기 표시했고, v2는 커밋 `46939bbe`에서 재생성했다. **완료 기준:** `check-research-node --base e8572eac` exit 0, joint_controller 아래 authority-ref FAIL 0, 결과 사후 감사 영수증.
- **A2 MF 재선형화 할당(기지 미해결).** L-0172 G2는 이 잔여(시도당 약 3회)를 허용하고 통과했다. `rodas5p_matrix_free_fast.rs:2657`(`eval_partial_t` Vec)와 `:2660`(`Arc<ClosureOperator>`, `y.to_vec()`). 첫째는 time derivative callback이 있거나 autonomous인 문제에서 기존 `eval_partial_t_into`로 bitwise-safe하게 제거된다. time derivative가 없는 비자율 문제는 FD f_t 분기(`partial_t_finite_difference`)도 할당하므로 별도 in-place FD f_t가 필요하다. 둘째는 재사용 가능한 구체 JVP 연산자 타입과 rodas5p-core 공개 token API가 필요하다(api/governance, M).
- **A3 FD Jacobian 할당(기지 미해결, 이번에 첫 계측).** 2026-10-21 WAVE3 task-3 보고서(open question 2)가 이미 기록한 항목을 `rodas-fd` 대조 arm으로 처음 계측했다. `Problem::new` 기본값이 FD(`problem.rs:144`)이고 dense FD(`fd.rs:250-254`, f0·shifted·f1·n×n)와 banded FD(`banded_fd_fill`, f0·shifted·f1·deltas)가 구성마다 할당한다. 사용자가 Jacobian을 주지 않은 모든 RODAS 풀이에 해당한다. fast driver의 무할당 문구(`rodas5p_fast.rs:19-20`)는 in-place Jacobian을 전제로 하므로 이 경로는 그 약속 밖의 VG-1 공백이다. workspace 소유 버퍼로 같은 증분·평가 순서를 유지하면 bitwise-safe하다.
- **A4 선택기 VG-1 범위.** 유일한 switching 할당 계약은 pilot·재확인·implicit tenure를 끈 상태에서 "배치당 6 + 32"를 허용한다. 배치·facade 호출당 할당이 VG-1 "hot loop"인지 먼저 거버넌스 결정이 필요하다.
- **A5 증거 기록 왜곡.** lower-bound 복귀 승리가 `ExplicitCheaperOnSameWindow`로 기록되어 그 변형의 문서(`switching.rs:283-285`)와 모순되고, 계약 시험(`return_sentinel_contracts.rs:267-270`)이 이를 고정한다. 미완료 pilot은 evidence가 `ImplicitPilotIncomplete`로 맞게 표시되지만 `comparator_work_ratio`가 0.98 같은 완료 비교형 비율을 게시한다. 결정 산술은 결정론적 callback과 prefix 가정 아래에서만 하한으로 유효하다. `ObservedLowerBound`와 별도 evidence 변형을 추가하고 미완료 pilot 비율은 None으로 한다.
- **A6 종료점 전용 정확도.** 선택기의 모든 반환 행과 정확해가 있는데도 종료점만 채점한다. 행별 성분 척도 오차와 같은 시각의 DOPRI observed 대조를 추가한다(RODAS observed 공개 API는 별도 결정).
- **A7 gyromotion 기본 허용오차.** 선택기와 직접 DOPRI·MF RODAS가 모두 상한을 넘는다. 원인은 허용오차에 비례하는 위상 표류이며 선택기 결함이 아니다. 상한은 넓히지 않고 rtol 1e-6–1e-10 work-precision ladder로 위상·진폭을 분리 보고한다.

## B. 큰 측정 비용·결정 편향

- **B1 가망 없는 pilot 반복.** v2 callgrind에서 rotation 선택기−직접 Ir 차이 1.506M 중 1.285M(85%)이 버려진 MF pilot이다. 32회 시도 quota로 멈춘 모든 배치가 pressure로 간주되고 back-off는 패배 폭을 무시한다. rotation pilot은 ceiling 154 WU에 대해 전체 prefix를 풀려면 1,005–1,218 WU가 필요하다. quota-stop을 pressure에서 빼는 단순안은 강성 cell의 초기 진입 승리를 없애므로 **반박**되었다. 패배 폭 q에 비례한 back-off 증가(log₂(qρ))를 사전등록 정책으로 제안한다.
- **B2 작은 n의 MF.** 같은 callback으로 n개 JVP에서 dense Jacobian을 조립하는 대조가 6.9×(Ir) 싸다(MF와 시도·RHS 동일). 호출자 쪽에서 이미 가능한 조립이며(`Jacobian::dense`로 JVP를 감싸기), 라이브러리 opt-in(`allow_small_assembly(n_max)`)은 권한·provider identity만 더한다. 사용자가 provider를 FiniteDifference로 선언하면 직접 RODAS FD-dense도 같은 fixture에서 MF보다 Ir 6.2–6.5×(진단 시간 7.7–7.9×) 싸다. FD-dense는 차분 때문에 RHS 호출이 더 많다(case5 409 vs 307). 선택기의 `AllowFiniteDifference`는 FD를 선언한 문제만 허용하며 JVP provider를 치환하지 않는다. **허가되지 않은 provider 치환은 하지 않는다.**
- **B3 냉시동 pilot.** pilot h0=opts.h0라서 판정이 regime 비용보다 step 증가 ramp와 prefix 길이를 측정한다. h0=clamp(L/k̂) 시드와 거절 여유를 사전등록해 평가한다. h0 반응은 단조가 아니다(0.40452에서 L/3 196 WU, L/2 290 WU).
- **B4 transition horizon.** 기지 항목. prefix가 RODAS 점근 step 하나보다 짧다. ceiling에서만 실패하고 step이 증가 중일 때 window 끝까지 연장하는 2단계 pilot을 제안한다(비교 계약이 같은 구간에서 rate 외삽으로 바뀜).
- **B5 대리지표 가격.** v2 ns/WU는 DOPRI 40–100, dense 43–92, MF 180–292이다. 같은 ρ=0.8(WU)이 MF에서는 훨씬 큰 시간 비에 해당한다. 직교화·QR·설정 비용을 반영한 versioned proxy v2를 고정 가중치로 등록하되, n≤6 fixture만으로 가중치를 맞추지 않는다(D6).
- **B6 소형 QR.** (k+1)×k Hessenberg 최소제곱에 faer column-pivot QR을 매번 쓴다. Givens 회전 + 퇴화 시 기존 경로 fallback을 `least_squares_correction` 안에서만 적용한다. 해·step 순서가 반올림 수준에서 바뀌므로 bitwise가 아니며 사전등록된 동일 품질 비교(VG-2/VG-3)가 필요하다. `gmres_givens.rs` 전체 채택은 정지·breakdown 의미가 달라 제외.
- **B7 강성 진입 지연.** 첫 explicit 배치 32회와 냉시동 pilot이 진입 전 비용을 만든다. B3 이후 dense 경로에 한해 경로 크기 첫 배치 예산을 검토한다.
- **B8 도달 불가 pilot.** step 성장률 상한 5와 최소 시도 비용으로 남은 ceiling 안에 prefix 도달이 불가능해지면 중단한다. 결정은 바뀌지 않을 것으로 예상된다(성장률 상한과 c_min 하한에 근거한 논증이며 등록 cell 전체의 결정 동일성 시험으로 확인해야 한다). ceiling 정지 때 `format!` FatalCallback 경로의 할당도 피한다.
- **B9 facade 재구성.** tableau 파생표의 static 계산, `observed` 이동, implicit workspace 재사용으로 bitwise 동일성을 유지하며 호출당 고정비를 제거한다.

## C. 중·소 비용(대부분 bitwise-safe)

C1 RecomputeFinal 회계(B5와 결합 필수), C2 GMRES 부가 비용, C3 explicit 기록 비용, C4 pilot ceiling 단위 불일치(B5와 결합, Guard 의미 변경), C5 wrapper 계층, C6 SPD01 사전검증 제어기 활성화와 SPD06 이식(미검증), C7 session의 dyn 호출, C8 착지 보조함수 inline, C9 복귀 재확인 ramp 선별. C2·C5·C7·C8의 호출·inline 비용은 lto=false, codegen-units=16 measurement profile에서 측정했으므로 release profile(fat LTO, cgu=1)에서 순위를 다시 확인해야 한다.

## D. 측정·증거 품질

D1 시간 분류 label과 반복 요구, D2 trace 도구 결함 3건(`adapter_checkpoint_trace.rs:474-478` 기준, 46939bbe), D3 출력 행과 착지 비용 분리(직접 observed8 대조), D4 WRMS 지표 병기(`:398-413`), D5 sentinel k sweep과 독립 fixture, D6 n≥32·고비용 RHS의 비홀드아웃 scaling cell.

## 이번 감사 중 해결·정정된 사항

- **R02 측정 harness.** v1 light bench는 switching arm만 per-segment JSON을 timed loop 안에서 만들고 할당을 cold 첫 호출에서 셌다. "decay 3.1×"는 실제 Ir 1.59×, 시간 최소값 1.86×였다. v2 harness(`46939bbe`)로 재측정했고 v1 자료는 `data_v1_superseded/`에 보존했다.
- **반박된 하위 주장 28건**은 ISSUES.json `refuted_sub_claims`에 있다(예: quota-stop 제외안, callgrind 몫 88%/83% → 정상상태 87%/82%, 세션 경로 +7–11% 시간 이득). 별도로 perf 포함비율 69%/55%는 비평 C1과 v2 재측정에서 callgrind 85%/80%로 대체되었다.
- **출판 전 packet 검증.** 같은 세션의 Claude agent 2개(수치·주장/코드)가 이 packet을 검증해 42건(ERROR 8, MISLEADING 20, 참고 14)을 지적했고 모두 반영했다(`workflow/PACKET_VERIFICATION.json`). 이종 모델 감사는 아직 없다: codex는 2026-11-02까지 사용량 한도, Gemini CLI는 snap cgroup 오류로 실행되지 않는다.

## 우선 순서 제안

1. A1–A4: 규칙과 hard gate. A2·A3은 작고 bitwise-safe하다.
2. B1·B8: 안정 explicit 문제의 선택기 오버헤드 대부분을 결정 변화 없이 또는 사전등록 정책으로 제거.
3. B2(권한·provider 계약)와 B5(대리지표 v2)를 함께 설계하고, C1·C4는 B5 이후 적용.
4. B3 → B4 → B7 순서로 진입 정책을 사전등록 실험.
5. B6·B9·C2·C3: bitwise-safe 또는 동일 품질 비교로 커널·기록 비용 절감.
