# T02 현재 Global-Error Authority Replay

- 기준: `research/unified-t02-20261110` @ `bd82eff9d1d967ef489ac4eb0c302ef6a4102f31` (tree `a032d2f9fa615d776e0b31af9d672e62a998fbc2`), C5 `569ce026…`와 `crates/` 트리 동일.
- 범위: `rodas5p-fair-ab` achieved-error authority의 **소스 추적 + 계약 테스트**. 성능 순위/주장 없음.
- 판정: **PARTIAL_SCOPED / CONCERN** — 구체적 reference artifact replay는 NOT_RUN.

## 1. 소스 identity (blob)
- `crates/rodas5p-fair-ab/src/global_error.rs` — `c62f996fa18f9815f1f329766abe8fe2c25ca22b`
- `crates/rodas5p-fair-ab/src/adaptive_global_error.rs` — `ccae16353ed1dcc90c6511befea8b42f7d3ad9cd`
- `crates/rodas5p-fair-ab/src/numerical_reference.rs` — `b8adbdae154eaa243310136f78ec2eaa4d7073d9`
- `crates/rodas5p-fair-ab/src/output_accuracy.rs` — `57401289d7484f5927693b0b47333660f1f2644d`
- `crates/rodas5p-fair-ab/src/output_admissibility.rs` — `b1f084aad4c50770dcbae141e7ab1741f6ec37a8`
- `crates/rodas5p-core/src/work.rs` — `76577cf6cb0295f859efcad2606150a01961cae7`

## 2. 추적 결과 (라인 근거는 JSON `claims`)
1. **Reference identity/불확실성**: `ReferenceSolutionProvenance`가 problem_id, source_kind, output_grid_id, state_checksum, reference_uncertainty_wrms를 운반. 수치 reference는 manifest의 artifact SHA-256을 인증(`numerical_reference.rs:1030-1075`)하고 불확실성 = Richardson(D1·q/(1−q)) + 방법 불일치(비트 일치 강제, `:976-995`).
2. **Output grid**: `CommonOutputGrid` — 유한·엄격 증가, `grid_id = sha256(JSON(times))`; uniform은 간격이 구간을 나눌 때만 허용(`global_error.rs:30-80`).
3. **Global WRMS**: weight = abs + rel·|reference|, 격자점마다 `wrms = ‖diff/weight‖₂/√dim`; endpoint/max/rms 및 `conservative_max_wrms = max + u_ref`(`:130-134, 354-381`).
4. **Clipped/dense 쌍**: 두 정책의 max-grid WRMS 차이 > 0.1·dense 오차이면 `OutputPolicyDominated`(동등은 admissible, `:711-734`). 순위에는 두 행이 모두 Success이고 admissible일 때 **dense 행만** 진입(`adaptive_global_error.rs:1040-1047`).
5. **Reference dominance**: u_ref > 0.1·측정 max-grid WRMS이면 Dominated; 불완전/불일치 provenance는 fail-closed `ReferenceDominated`(`numerical_reference.rs:375-397`, `global_error.rs:864-900`).
6. **Two-arm v3 admissibility**: 정확합(E+U ≤ B → Within, E−U > B → Exceeds, 그 사이 Undecidable), B = 10.0 WRMS, interpolant Δ ≤ 2.0, 순서 reference→budget→interpolant(`output_admissibility.rs:25-27, 69-91, 188-219`).
7. **Work counters**: `WorkCounters`(RHS/JVP/Jacobian/비선형/선형/precond/분해/accepted·rejected·fallback 등) + `internal_steps`, `output_clipped_steps`, `stored_state_bytes`(`rodas5p-core/src/work.rs`, `global_error.rs:759-767`).

## 3. 상태값 (전수)
- `IntegratorRunStatus`: Success, ReferenceDominated, OutputPolicyDominated, SolverFailure, MissingOutput, NonFinite, TimingFailure
- Pair: Admissible, OutputPolicyDominated, Incomplete · ArmBudget: WithinBudget, ExceedsBudget, ReferenceUndecidable
- TwoArmRowStatus: Pass, ReferenceDominated, GlobalErrorExceedsBudget, InterpolantUnchecked, InterpolantDominated

## 4. 실행한 명령
- `global_error_contracts` -> exit 0 **PASS** - ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
- `adaptive_global_error_contracts` -> exit 0 **PASS** - ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
- `numerical_reference_contracts` -> exit 0 **PASS** - ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
- `numerical_reference_v2_contracts` -> exit 0 **PASS** - ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
- `output_accuracy_assessment_contracts` -> exit 0 **PASS** - ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
- `two_arm_admissibility_contracts` -> exit 0 **PASS** - ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
- `python3 -I docs/agent/unified_local_agent_20261109/validate_unified_dag.py` -> exit 0 **PASS** - PASS: 30 tasks, 48 edges, acyclic DAG; G0 immutable; mutation/evidence contract valid
NOTE: this validates package structure only; it does not run Rust/Cargo or certify scientific claims.
- `python3 -I docs/agent/solver_consolidation_20261108/validate_package.py` -> exit 1 **FAIL** - FAIL: invalid status for node V11-19-PEER-W

cargo 테스트 총 44개 (이름은 JSON `tests`), stdout/stderr SHA-256은 `EXECUTION_RECEIPT_T02.json`.

## 5. 발견 사항
- **T02-F1 (CONCERN)** Two different admissibility regimes coexist: (a) per-pair 10% rules (reference dominance, output-policy dominance) and (b) two-arm v3 fixed absolute budget B=10 WRMS. B is a pass/fail budget, not a matched-achieved-error comparison; PW06/T09 must not treat equal-rtol or B-pass as matched achieved error.
- **T02-F2 (CONCERN)** stored_state_bytes is retained output size, not peak RSS. WorkCounters has no explicit allocation, peak-memory, setup or startup-work field; the handoff requires these to be charged in PW06/T09, so they must be measured outside this ledger or added with owner approval.
- **T02-F3 (INFO)** Analytic reference problems carry reference_uncertainty_wrms = 0.0 (offload digest cited adaptive_global_error.rs L343; not independently re-read here); numerical references derive uncertainty only from Richardson + method disagreement.
- **T02-F4 (CONCERN)** Parent package validator validate_package.py FAILS on this immutable commit: 'invalid status for node V11-19-PEER-W' (PARALLEL_RESEARCH_PLAN.json status string is outside the validator's allowed set). Documentation-tooling mismatch; not repaired (no silent change to historical evidence).

## 6. NOT_RUN / 한계
- Concrete end-to-end replay of a specific numerical reference artifact + candidate run (reference identity/uncertainty of a real case): NOT_RUN
- Independent re-read of analytic_reference_problem uncertainty (adaptive_global_error.rs ~L317-343): taken from offload digest only
- cargo fmt/clippy/workspace-wide tests: NOT_RUN (docs-only mutation, no Rust change)
- GitHub readback of committed files: NOT_RUN (no push performed; push needs owner approval)
- Performance ranking: NOT_RUN by design (T02 gate)

## 7. 다운스트림 영향
Output/error authority는 현재 소스 기준으로 추적됐으나 T02-F1/F2 때문에 PW06/T09의 matched achieved-error 비교는 (a) 고정 rtol·고정 budget 통과를 대용하지 말고, (b) setup·startup·peak memory를 ledger 밖에서 측정해야 한다. 임계값/예산/정책은 변경하지 않았으며(오너 승인 필요) 이 문서는 어떤 solver의 정확도도 인증하지 않는다.
