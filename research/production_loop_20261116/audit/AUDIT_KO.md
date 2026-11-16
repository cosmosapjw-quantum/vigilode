# 루프 L1 감사 종합 (V01, 2026-11-16)

대상: LOOP `research/production-loop-20261116` @ `2dc9a2c5`(코드는 `074a5699`와 동일), switcher 계열 `93392249`(읽기 전용).
경로 표기: 접두 없는 경로는 LOOP worktree 기준, `SW:`는 `/home/cosmosapjw/vigilode`(switcher 체크아웃), `audit/`는 `research/production_loop_20261116/audit/`.
기계 판독본: `audit/audit_findings.json`(30건 전부, 검증 원문 요약 포함), `audit/R_GATE_PROPOSAL.json`, `audit/LINEAGE_ADDENDUM.md`. 이 문서는 제안과 기록이며 판정 변경이 아니다.

## (a) 판정 요약

- finding 30건(critical 1, major 19, minor 10; 신규 21건)을 병합했다. 독립 검증에 보낸 9건은 9건 모두 confirmed이고 전부 교정 사항이 붙었다. refuted는 0건이다.
- 검증에 보내지 않은 21건 중 9건은 계획 단계 항목을 차원 감사자가 명령과 경로로 재도출한 것이다. 나머지 12건은 감사자 1명의 finding이며 사전등록 기준에 따라 lead로 남긴다.
- 계획 단계 항목 38건: confirmed 24, corrected 8, refuted 1(V13의 `workflow_dispatch` 경로), not_checked 5.
- 증거로 무효인 기록은 switcher 계열 ledger 6행(L-0209..L-0214)이다. 통합 계열(U03/U04/U09/U10/U11)은 결과 파일 재생성과 재실행으로 재현됐고 무효화된 판정이 없다.
- U11 HOLD는 유지된다. 다만 step 비(1.9–7.1)는 controller 설정의 산물이고, 소형 문제의 step당 wall 비는 attempt마다 반복되는 tableau 재계산이 지배한다. 둘 다 PPSW1-6b 방법의 속성으로 읽을 수 없다.
- 공개 adapter `PeerOde`에서 실행으로 확인된 결함 2건: 허용오차의 9.0e6배 오차에 success=true(COD-02), 권한 우회와 사유 없는 실패(COD-04).
- R-gate 제안: 8축 중 NOT_MET 6, PARTIAL 2(R5, R8), MET 0. G0는 이 감사에서 다시 판정하지 않았다.
- 실행 범위: 감사는 U 결과 스크립트 재실행, 테스트 target 3개 재실행, U11 고정 바이너리 재실행과 profile, scratch probe를 실행했다. switcher 코드는 빌드도 실행도 하지 않았다.

## (b) 확정된 finding (심각도순)

"검증"은 독립 검증 에이전트 confirmed, "계획+감사"는 계획 단계 항목을 감사자가 재도출(검증 미전송)한 것이다.

| ID | 심각도 | 확인 | 내용 | 근거 경로 | 최소 교정 |
|---|---|---|---|---|---|
| EVD-01 | critical | 계획+감사, spot C01/C02 | L-0209(K1)·L-0210(B7)·L-0213(D1)·L-0214(C1)의 스크립트는 프로세스를 실행하지 않고 파일도 읽지 않는다. 수치는 리터럴과 그 산술이다. C1 ledger 문구의 2.46–2.87x는 node의 어느 파일에도 없다(RESULTS는 10.94, 6.34). | `SW:research/{k1_shared_krylov_basis,b7_first_batch,d1_stiffness_gate,c1_coloured_assembly}_20261104/eval_*.py`, `SW:research/LEDGER.jsonl` | INVALID 대체 행 추가(원 행 보존). V06/V08 사전등록에서 이 수치를 인용하지 않는다. |
| EVD-02 | major | 검증 | L-0212(H1)의 \|b\|=1.2069는 입력 리터럴의 산술 ln(185/192)−ln(98/340)이다. 일부 리터럴은 10-27 측정 산출물 값과 같지만 스크립트가 읽거나 해시하지 않고, 340·118·182·355·47·0.18은 출처가 없다. | `SW:research/h1_hysteresis_band_20261104/eval_h1.py:35-50,87-100` | V02 INVALID 집합에 L-0212 추가. |
| EVD-03 | major | 검증(저장된 RESULTS 판독, callgrind 미재실행) | L-0211(E1R)의 callgrind 대상은 `work_model_contracts` 단위 테스트이고 이름 붙은 solver 사례는 실행되지 않았다. 두 사례가 같은 테스트다(633716, 633728 Ir). 상대오차 182.97과 Spearman 0.7667은 보정 결과가 아니다. supersedes가 L-0205를 가리킨다. | `SW:research/e1r_work_model_v2_20261104/eval_e1r.py:160-161,163-304`, `SW:crates/vigilode/tests/work_model_contracts.rs:87-97` | INVALID 대체 행. 실제 E1R에는 사례별 실행 바이너리가 필요하다. |
| EVD-04 | major | 검증 | 11-04의 6개 node는 사전등록·스크립트·ledger 행이 한 commit에 들어왔다(D1은 RESULTS가 31분 뒤 commit). 6행이 인용하는 `4339facc`에는 해시된 입력 파일 4개가 없다. bench example 2개는 어느 ref 이력에도 없다(DESIGN.md에는 허용 경로로 기재). | commit `8800fb9a`, `68692104`, `157586cf`, `117e2870`, `400cd0cc`, `16eb1b87`; `SW:docs/agent/switcher_v2_remediation/DESIGN.md:1956` | INVALID 행에 실제 commit과 "사전등록·결과 동일 commit"을 기록. |
| DSG-02 | major | 검증 | U11 고정 바이너리 profile에서 `ppsw1::mm`+`tableau`가 main-thread cycle의 약 51–54%(세 arm 합산), attempt당 78k–89k cycle이다. 문제별 비중은 n=3 68–71%, n=8 59–62%, n=64 16%. n=64의 wall 비 2.12–2.51은 대부분 이 비용이 아니다. | `crates/rodas5p-integrators/src/ppsw1.rs:192-284,408`; `audit/findings_design.json` data.perf_agg | V10 동등 구현 arm은 tableau cache 후 비교하고 재계산 유무별 instruction/attempt를 보고한다. |
| COD-01 | major | 검증 | DSG-02와 같은 원인의 instruction 측정. Oregonator n=3, tol 1e-6에서 `tableau(sigma)`가 PPSW1 instruction의 79.87%(158,215 Ir/attempt). PPSW1 197,907 대 RODAS5P 13,357 instr/attempt(14.82x), tableau를 빼면 2.96x. heap 할당 61.02 대 0.06회/attempt. 비중은 n=3 한정이다. | `ppsw1.rs:408`; `audit/findings_code.json` measurements·probe_sources; `audit/raw_scratch/cod/` | 새 opt-in 세션에서 (σ, Tableau) cache. cache 적중률을 먼저 측정한다. 기본 경로는 bit-동일 유지. |
| DSG-03 | major | 검증 | switcher UNIFIED_BENCHMARK_REPORT는 같은 rtol끼리의 표다. VigilODE arm 15행은 모두 격자 착지 arm이고 accepted의 10.3–94.3%가 강제 착지다. Julia 36행에는 오차가 없다. VigilODE/Julia step 비는 rtol 1e-4에서 1.44–3.08, 1e-8에서 0.943–1.012. | `SW:bench/out/{UNIFIED_BENCHMARK_REPORT.md,rust_bench_results.json,julia_bench_results.json}`, `SW:crates/vigilode-bench/src/arms.rs:125-127` | V11/V13에서 dense-output arm, Julia 오차, tol 5개 이상, matched achieved error 표로 다시 만든다. |
| DSG-05 | major | 검증 | battery v3의 matched 행 2340개 중 MATCHED는 88개(sphere-photon-2d 52, -3d 36)다. 나머지 7문제는 reference-disputed(1820행). 자유 step arm은 실행당 6–47 step, n=64–302. | `SW:bench/out/adversarial_spatial_battery_v3_results.json` | V11: 불확실성을 명시한 참조 정책을 먼저 고정하고, 수백 step·n 1e4–1e5 규모로 잡는다. |
| COD-02 | major | 검증(probe 재실행) | `PeerOde`가 오차 8.31(허용오차의 9.0e6배)과 50.2(5.7e7배)인 해에 success=true를 반환한다. starter 구간 [t0, t0+2h0]에 오차 제어가 없고 h0 규칙이 tol을 보지 않는다. h0=NaN은 조용히 상한값으로 대체된다. | `ppsw1.rs:1023-1035,1125-1256`, `crates/vigilode/src/selection.rs:959-979,1028` | opt-in adapter 수정: 비유한 h0 거절, starter 오차 추정, Peer step 0회 종료를 별도 결과로 보고. |
| COD-04 | major | 검증(코드 판독, probe 미재실행) | `PeerOde` admission이 provider 형태를 보지 않아 JVP를 불허한 호출자도 JVP callback을 받는다. 실패 사유가 `Option<String>`이다. span (0.3, 0.9)에서 success=false, failure=None. Recoverable 오류가 무관한 InvalidInput으로 끝난다. band 선언은 admission 통과 뒤 solve에서 거절된다. | `selection.rs:815,837-864,870-898,1028`; `ppsw1.rs:786-794` | variant를 `problem.jacobian`에서 도출, failure를 enum으로, endpoint 착지 flag로 success 정의. |
| EVD-07 | major | 계획+감사, spot C04 | U01–U11의 ledger 행이 0개다. `tools/check-research-node.py`가 통합 node에서 exit 1. | `research/LEDGER.jsonl`, `tools/check-research-node.py` | 결과 파일이 있는 U node마다 L-0301 이상 행 추가. node 수준 PREREGISTRATION.md 또는 사유를 적은 면제. |
| DSG-01 | major | 계획+감사 | U11 step 비 1.9–7.1은 성장 제한(7 step당 1회, 최대 1.10)과 h0 규칙을 잰 값이다. tol 1e-4..1e-5에서 limiter 포화율 0.911–1.0(감사자 scratch 재빌드). | `ppsw1.rs:57,841-849,935-956,1023-1036`; `audit/findings_design.json` data.step_stats | V10: step별 (t, h, σ, err) 기록, step 비 옆에 포화율 병기, cap·history·h0를 따로 바꾸는 arm. |
| DSG-04 | major | 계획+감사 | 평가 87건 중 n>400의 stiff matched-error 교차 비교 0건, 고비용 RHS 0건, 전처리 Krylov arm 0건. 표의 범주 열은 감사자 판독이다. | `audit/findings_design.json` data.regime_table | V04/V11 격자대로 실행. 결과 행마다 n, 연산자 차원, Jacobian 종류, backend, RHS 비중을 기록. |
| COD-03 | major | 계획+감사(실행 probe) | `MethodRegistry` admission과 `GeneralOde::select`가 실행 사례 9개에서 양방향으로 불일치한다. | `selection.rs:176-227,509-665`; `audit/findings_code.json` raw_evidence.registry_vs_select | V03 전에 request에 regime·차원·band·provider 형태 등을 추가하고 select==registry 계약 테스트를 둔다. |
| COD-05 | major | 계획+감사, spot C09, 미빌드 | switcher unified frontend에서 Tier2/3/4/Protected가 같은 `Rodas5p::solve`로 간다. 읽히지 않는 옵션과 쓰이지 않는 buffer가 있고 문서의 엔진 주장에 대응 코드가 없다. | `SW:crates/vigilode/src/unified/{router.rs:63-75,175-193,analyzer.rs,options.rs,workspace.rs}` | 채택 시 driver 없는 tier와 미사용 옵션 제거, 실행 경로는 `Stats.route`로 기록. |
| DSG-07 | minor | 계획+감사, spot C13 | U11 속도 기준의 thread arm이 사전등록에 없고 evaluator가 결과와 같은 commit이다. 1-thread 중앙값 0.0166, 6-thread 0.00634. | `production_unification_20261115/production_holdout/PREREGISTRATION.json:6,32`, `evaluate.py:88-89` | 이후 사전등록에 arm, 집계 방식, evaluator 해시를 실행 전에 적는다. |
| COD-08 | minor | 계획+감사 | 6-thread arm은 attempt당 rayon 왕복 2회, solve마다 pool 생성. t6/t1 wall 1.59–2.94(diagnostic). | `ppsw1.rs:502,597`, `selection.rs:945` | V10: 소형 n은 1-thread arm, thread 시험은 상주 pool과 step당 fork-join 1회. |
| COD-09 | minor | 계획+감사, spot C11 | `.rej` 2개 추적, gate·campaign 이름의 모듈 27개(21,142 LOC, 이름 패턴 기준)가 무조건 컴파일, 2개 crate가 workspace 밖. | `crates/rodas5p-integrators/src/lib.rs`, `Cargo.toml:3-11` | `.rej` 제거, 연구 모듈을 feature 뒤로(보호 driver bit-동일 확인), 두 crate는 별도 job 또는 exclude 문서화. |

### lead (감사자 1명, 검증 미전송; 확정 아님)

| ID | 심각도 | 내용 | 종합 단계 spot check | 최소 교정 |
|---|---|---|---|---|
| EVD-05 | major | switcher gate 도구 `agent_gate.py`가 gate 대상 commit 안에서 수정됐다. PROGRESS.md가 D1을 PASS로 적어 ledger(FAIL)와 어긋난다. | C08: `4339facc` 이후 6개 commit이 파일을 수정(X1–X4, X7 포함). 수정 내용은 재판독 안 함. | V02에서 switcher gate 상태는 참고용으로만 쓰고 1.94.1 빌드·양쪽 suite·bit 오라클에 의존. |
| EVD-06 | major | 연구 브랜치 push로 도는 workflow가 없고 `production.yml`은 default 브랜치에 없어 dispatch가 불가능하다. | C03: origin/main(local ref)에 workflow 4개, `production.yml` 없음. head에는 31개. | 소유자와 draft PR 여부 결정. 아니면 CI를 NOT_RUN으로 기록하고 clean-clone 로컬 재현을 쓴다. |
| DSG-06 | major | U04 no-crossover는 전처리 없는 GMRES 대 dense LU, 1D tridiagonal 한 family, tol 하나의 진술이다. | 없음. EVIDENCE 감사자의 RESULTS 재생성 수치와 step 비가 맞는다. | V06/V10: banded 직접법 비교군, 전처리 arm, step 비도 묶는 admissibility, 2D 연산자. |
| COD-06 | major | switcher: 실패 사유 치환, counter 누락, 과금 안 된 preflight, 같은 solve를 반복하는 "fallback". | C09: `MaxStepsReached` 매핑과 항등 비교 `accounting_complete()` 확인. 나머지 지점 미확인, 미실행. | `WindowDriver::run`에 typed 오류, preflight 과금, leg 간 attempt 예산 공유. |
| COD-07 | major | switcher preflight가 수축성(μ₂≤0)을 non-stiff로 분류하고 모든 문제에 n×n을 할당한다. | C10: `analyzer.rs:174-191`, `workspace.rs:47` 확인. DOPRI5 step 수 결과는 손계산. | `PreflightAnalyzer`를 router로 채택하지 않는다. workspace는 경로별 지연 할당. |
| EVD-08 | minor | `sp02_workspace_charge` flake: 병렬 테스트 3개가 프로세스 전역 live-byte counter를 공유한다. | C06: 전역 counter 1개, `#[test]` 3개, 직렬화 장치 없음. baseline flake probe와 일치. | 테스트 본문 전체를 static Mutex로 직렬화하거나 thread-local 계수. |
| EVD-09 | minor | cross-solver bench는 실제 실행이지만 v3 receipt가 raw를 잘못 적었다(953행 전부 DIAGNOSTIC_ONLY라 했으나 1040행 = 952 + MATCHED 88). raw 4/5가 dirty tree 빌드. | C07: 1040 = 952 + 88 재현. | clean tree에서 dense-output arm으로 재실행, 실행마다 새 파일명, receipt 수치는 raw에서 생성. |
| EVD-10 | minor | U 증거는 재현되나 `evaluate.py`가 G0 기준 2개를 리터럴 pass로 두고 "SUNDIALS not installed"라 적는다. U09에 기계 판독 결과 파일이 없다. U03 입력 해시가 낡았다. | C05: `evaluate.py:113-121` 리터럴과 libsundials-dev 6.4.1 설치 확인. | depth·fallback은 runner 출력에서 읽거나 NOT_MEASURED로. U09 summariser 추가. |
| DSG-08 | minor | 비용 모델: stage 병렬 PPSW1이 1.15x에 닿으려면 J+LU 비중이 작고 step 비가 약 2 미만이어야 한다(모델, 측정 아님). | 없음 | V10 사전등록에 영역별 f(V04 실측)를 실행 전에 적는다. |
| DSG-09 | minor | 1.10 cap은 mipeer6용 OD-15에서 왔고 상수로 고정돼 Config로 σ를 바꿀 수 없다. | C12: OD-15 항목 존재만 확인, 문구 미재판독. | V10은 실험 전용 controller를 따로 두고 `ACCEPTED_STEP_SIGMA_MAX`는 그대로 둔다. |
| DSG-10 | minor | U11 timing 바이너리의 release profile이 저장소 profile(fat LTO, codegen-units 1)과 다르다. | C12: runner `Cargo.toml`과 root profile 확인. | timing runner는 workspace profile을 복사하고 RUSTFLAGS를 기록. |
| COD-10 | minor | README와 crate 문서가 "one solver"라 하지만 진입 타입 10개, `pub fn solve*` 18개다. | C11: 18개와 README 0건 확인. | V03 뒤 registry descriptor에서 README 재작성. |

## (c) 계획 단계 항목 검증표

| 절 | 항목 | 판정 | 비고 |
|---|---|---|---|
| 1.1 | ledger 분기(U 행 0개, switcher L-0205..L-0214) | confirmed | LOOP는 L-0300만 추가 |
| 1.1 | 양쪽이 함께 고친 파일은 integrators `lib.rs` 하나, switcher toolchain stable | confirmed | merge-base `569ce026`, switcher 고유 52 commit |
| 1.1 | switcher가 보호 파일 7개를 수정 | corrected (spot C14만) | 보호 목록 기준 8개. 계획에 `rodas5p_matrix_free_fast.rs`(78줄)가 빠졌다. 독립 검증 없음 |
| 1.1 | lineage 문서에 switcher 언급 0건 | confirmed (spot C14만) | `git_inventory.json`에는 두 ref가 기록돼 있다 |
| 1.1 | REI 계열 미병합(고유 287 commit) | confirmed (spot C14만) | commit 수와 비조상 관계만 확인. N02/N03/46k 내용은 미확인 |
| 1.2 | eval_k1/c1/b7은 simulate_cell, eval_d1은 리터럴 14/24 | corrected | simulate_cell은 K1·C1뿐. B7은 리터럴 표, D1은 전부 리터럴 |
| 1.2 | 합성 행은 L-0209/0214/0210/0213 | corrected | L-0212도 타이핑 값, L-0211은 오측정. 6행 |
| 1.2 | `k1_shared_basis_bench.rs` 부재 | confirmed | `b7_first_batch_bench.rs`와 E1R 사례별 바이너리도 부재 |
| 1.2 | shared basis·coloring 성능 미측정 | confirmed | 계약 테스트만 존재 |
| 1.2 | L-0211만 callgrind 실행 | corrected | 대상이 무관한 단위 테스트 |
| 1.2 | U11 (a) 6-thread가 1.6–2.9배 느림 | confirmed | 1.59–2.94, 상태는 bit 동일 |
| 1.2 | U11 (b) step당 wall 8–24배/2.1–2.5배, 구현 성숙도, starter 포함 | corrected | 비율은 재현. 원인은 tableau 재계산(n=3에서 79.9%, n=64에서 약 16%). starter 등은 n=3 solve의 0.19% |
| 1.2 | U11 (c) cap 1.10과 6-step hysteresis | confirmed | 포화율 0.54–1.00 |
| 1.2 | step 비 1.9–7.1 | confirmed | 1.911–7.108. 재실행 45/45 동일 |
| 1.2 | HOLD는 유효하나 방법 수준 결론 아님 | confirmed | 어느 arm으로도 HOLD |
| 1.2 | SUNDIALS 설치돼 있음 | confirmed | libsundials-dev 6.4.1 |
| 1.2 | 영역 불일치(n≤64, 가끔 1D n≤400) | corrected | n>1000 평가 16건이 있으나 stiff matched-error 교차 비교는 0. 2D/3D 9건은 switcher에 있고 7건이 참조 분쟁. 결론은 유지 |
| 1.2 | G0와 릴리스 준비도가 한 HOLD로 묶임 | not_checked | OD-L1-4로 처리 |
| 1.2 | 계보 문서 오류(L10, H14/H15, DICTIONARY) | not_checked | |
| 1.2 | `dense_output_v2.rs.rej` 추적 | confirmed | `.rej` 1개 더 있음 |
| 1.3 | frontend 10개 이상 병존 | confirmed | 진입 struct 10, 결과 struct 6, `solve*` 18 |
| 1.3 | registry가 라우팅에 쓰이지 않음 | confirmed | COD-03으로 확장 |
| 1.3 | facade에서 도달 불가한 method들, `Tier::Performance`=Protected | confirmed | |
| 1.3 | facade RODAS5P 경로는 순차, phase 타이머 없음 | confirmed | |
| 1.3 | switcher router의 tier가 같은 solve, 실패는 `MaxStepsReached` | confirmed | 판독만, 미빌드 |
| 1.3 | 3D·구면 연산자는 test에만, bench·hybrid는 workspace 밖 | confirmed | `src/mol/mod.rs`는 3줄 stub |
| 1.4 | PPSW1 MF W crossover 없음(n≤400) | confirmed | 영역 진술(DSG-06, lead) |
| 1.4 | R7 4.75배, C5 2.24배 느림 | confirmed | n≤3 모델 3개 |
| 1.4 | lagged LU kill gate, lane batch 0.93배 | confirmed | ledger 행 판독, raw 미재파싱 |
| 1.4 | R6, 대각 SDC, tableau 사실 5개, 비용 구조, Schwarz toy | not_checked | V05 범위 |
| 1.5 | 현재 head의 CI·독립 재현 없음 | confirmed | 이번 감사가 일부 재현을 실행 |
| 1.5 | Julia Rodas5P와 step 수 동일(228=228, 164 vs 162) | corrected | rtol 1e-8에서만. 1e-4에서는 1.44–3.08배 |
| 1.5 | Hairer RODAS와 step당 비용 비슷 | not_checked | |
| 3.1 | V02: 합성 4행 INVALID 대체 | corrected | 6행(L-0209..L-0214) |
| 3.1 | V02: 공개 이름 충돌 0건, `SparseJvp` match 비망라 | confirmed | |
| 3.1 | V13: `production.yml` workflow_dispatch | refuted | default 브랜치에 workflow가 없다. PR만 가능 |
| – | U03/U04/U09/U10/U11은 실행 증거가 있음 | confirmed | 결과 파일 byte 동일 재생성, 18/6/5 통과 |
| – | U11 사전등록이 실행 전에 push됨 | not_checked | local commit 순서는 맞음. push 시각은 확인 불가 |

## (d) 기록된 판정에 미치는 영향

유효(변경 없음):
- U11 HOLD. step 기준(1.911–7.108, 한도 1.2)만으로 실패하고 속도 기준도 양쪽 arm에서 실패한다. 단 `evaluate.py`의 depth·fallback 통과는 리터럴이라 측정값으로 인용할 수 없다.
- U03 자격 수치, U04 `NO_CROSSOVER_MEASURED`(n≤400), U09 계약, U10 transport. 결과 파일이 재생성되고 테스트가 재현된다.
- L-0207(L-0206의 INVALID), L-0205/L-0208(solver 실행. 단 raw 216개 파일이 미추적이라 git에서 재현 불가), v3 결론 `NOT_ESTABLISHED`, L-0300 baseline FAIL(기록으로 유효, 원인은 테스트 harness 경합으로 보임).
- 기본값(WorkModel V1, entry/return margin None)은 무효 행의 영향을 받지 않는다.

범위 제한:
- U11 step 비: 현재 설정(cap 1.10, history 6, h0 규칙)의 값. PPSW1-6b의 step 효율 결론이 아니다(DSG-01).
- U11 step당 wall 비와 `speed_vs_production`: tableau 재계산, 6-thread arm의 pool 비용, 저장소와 다른 build profile이 섞였다. 다른 영역으로 외삽할 수 없다(DSG-02, COD-01, COD-08, DSG-10).
- U04: 전처리 없는 GMRES 대 dense LU의 1D 영역 진술(DSG-06, lead).
- U06 registry: 기술용 표. select를 재현하지 못한다(COD-03).
- U07 `PeerOde`의 "callback 전 typed 거절"과 "capability 제약" 주장, 그리고 모든 PPSW1 matched-error 수치(starter 오차 미제어)(COD-02, COD-04).
- 병렬 stage·matrix-free·LU 재사용의 음성 결과 전체: 소형 또는 1D, 저비용 RHS 영역에 한정(DSG-04).
- switcher: UNIFIED_BENCHMARK_REPORT와 계획 1.5의 Julia step 동등 문장(rtol 1e-8 한정), battery v3(9문제 중 2문제), "unified production frontend"(실행 엔진은 DOPRI5·`Rodas5p::solve`·SwitchingSolver), X1–X11의 gate 통과 진술(EVD-05, lead).
- L-0211의 세 번째 gate(w_v2 = 264.2 WU)는 callgrind와 무관하지만 타이핑된 counter로 계산됐다.

증거로 무효:
- L-0209, L-0210, L-0213(FAIL)과 L-0214(PASS): 실행 없음. FAIL을 음성 결과로, PASS를 지지 근거로 읽을 수 없다(EVD-01; 검증 미전송).
- L-0212(검증 confirmed), L-0211의 WorkModel V2 보정 결과(검증 confirmed). WorkModel V2는 확인도 반박도 되지 않은 상태다.
- v3 receipt의 "953행 전부 DIAGNOSTIC_ONLY", U11의 NOT_RUN 사유 "SUNDIALS not installed", 계획 V13의 dispatch 경로.

R-gate 제안(`audit/R_GATE_PROPOSAL.json`): R1 NOT_MET(COD-02), R2 NOT_MET(COD-04), R3 NOT_MET(진입점 10개, COD-03), R4 NOT_MET(flake, CI 없음; 재실행은 지지), R5 PARTIAL(소형 0D/1D만 과거 실행, 대규모·고비용 RHS·전처리 0건), R6 NOT_MET(COD-09), R7 NOT_MET(EVD-01..04, EVD-07), R8 PARTIAL(U09 수렴 계약은 재현, 연산자는 test 전용, 보존 계약 미확인).

## (e) 미실행 항목

- switcher 계열 코드의 빌드와 실행 전부. COD-05/06/07은 소스 판독이다. V02 시험 병합, 1.94.1 빌드, bit 오라클은 이 감사의 범위가 아니다.
- 전체 테스트 suite, clippy, fmt. 감사는 target 3개만 재실행했다(18/6/5 통과). integrators 251 target은 L1에서 아직 돌리지 않았다.
- clean-clone 빌드와 hosted CI.
- E1R의 callgrind 재실행(EVD-03은 저장된 RESULTS.json과 스크립트 판독에 근거).
- COD-04 probe 수치(4/52/54)의 검증자 재실행.
- lead 12건과 EVD-01의 독립 검증. EVD-01은 critical이므로 INVALID 행을 쓰기 전에 C1·D1 스크립트의 독립 확인을 권한다.
- 계획 항목 5건(계보 문서 오류, Hairer RODAS 비용, gate 혼재, 1.4 이론 사실, U11 사전등록 push 시각)과 REI 계열의 내용.
- switcher의 보호 파일 수정은 파일 목록과 줄 수만 확인했다(C14). 수정 내용은 보지 않았다.
- 같은 전역 counter 패턴을 쓰는 다른 테스트 파일 35개(EVD-08).
- U11이 NOT_RUN으로 남긴 CVODE·Julia 비교, 프로세스 격리 RSS.
- wall 수치는 전부 load average 15–34에서 잰 diagnostic이다. 결정적 counter와 instruction 수만 1차 지표로 썼다.

부속 파일: `audit/synthesis_spot_checks.{sh,log}`(종합 단계 읽기 전용 확인 C01–C14), `audit/build_audit_synthesis.py`(병합 생성기), `audit/raw_scratch/`(차원 감사의 scratch 원자료 중 소형 텍스트 사본과 SHA256SUMS; perf data·바이너리·target은 `/tmp/claude-1000/{evd-audit-l1,dsg-audit,cod-audit}`에만 있고 휘발성이다).
