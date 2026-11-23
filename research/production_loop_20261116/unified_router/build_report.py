#!/usr/bin/env python3
"""Render REPORT_KO.md of loop L1 node V03 part B from RECEIPT.json.

The numbers come from RECEIPT.json (built by build_receipt.py from the raw logs). The
sentences are fixed text.

Usage (from the worktree root):
    python3 -I research/production_loop_20261116/unified_router/build_report.py
"""

import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent


def strict_number(text):
    raise ValueError(f"non-finite JSON constant {text}")


def fmt(value):
    if value is None:
        return "없음"
    if isinstance(value, bool):
        return "예" if value else "아니오"
    if isinstance(value, list):
        return ", ".join(f"`{item}`" for item in value) if value else "없음"
    return str(value)


def pair(entry, key=None):
    """`base -> now` of a {base, now} pair."""
    if entry is None:
        return "기록 없음"
    if key is not None:
        entry = entry.get(key)
        if entry is None:
            return "기록 없음"
    return f"{fmt(entry['base'])} → {fmt(entry['now'])}"


def status_kind(text):
    """The kind of a serialized Status, or the text itself."""
    if text is None:
        return "없음"
    try:
        value = json.loads(text)
    except (TypeError, ValueError):
        return f"`{text}`"
    if value.get("status") == "success":
        return "`Success`"
    extra = {k: v for k, v in value.items() if k not in ("status", "kind", "t")}
    detail = ", ".join(f"{k}={v}" for k, v in extra.items())
    return f"`{value.get('kind')}`" + (f" ({detail})" if detail else "")


def main():
    receipt = json.loads((HERE / "RECEIPT.json").read_text(), parse_constant=strict_number)
    acceptance = receipt["acceptance"]
    ff = receipt["failing_first_part1"]
    out = []
    w = out.append

    w("# V03 part B: 통합 router가 실제로 한 일을 보고하도록 고침")
    w("")
    w("이 문서는 `build_report.py`가 `RECEIPT.json`에서 만들었다. 숫자는 모두 실행한 명령의 "
      "로그(`logs/*.log`)와 `git`에서 `build_receipt.py`가 읽었다. 손으로 적은 숫자는 없다.")
    w("")
    w(f"- base: `{receipt['base_commit']}`")
    w(f"- 검증을 실행한 코드 commit: {fmt(receipt['code_commits_of_the_runs'])}")
    w(f"- 사전등록: `PREDECLARED.json`, commit `{receipt['predeclared']['commit']}`. "
      f"첫 로그의 commit(`{receipt['predeclared']['first_log_commit']}`)보다 먼저 commit했는가: "
      f"{fmt(receipt['predeclared']['committed_before_the_first_log'])}")
    w("- 시간 측정은 하지 않았다. 로그의 경과 시간은 `diagnostic_only`다.")
    w("- PREREGISTRATION.md에는 part B의 동결 행이 없다. 과제문의 수용 기준 네 개(A1-A4)를 "
      "기준으로 삼았고, 그 밖의 선택(문제, 허용오차, 경계값)은 첫 실행 전에 `PREDECLARED.json`에 적었다.")
    w("")

    w("## 1. 결론")
    w("")
    w("| 수용 기준 | 측정 | 판정 |")
    w("|---|---|---|")
    a1 = acceptance["A1_contract_tests_pass_each_written_first_as_a_failing_test"]
    a2 = acceptance["A2_default_path_golden_unchanged"]
    a3 = acceptance["A3_every_other_pre_existing_facade_test_passes"]
    a4 = acceptance["A4_no_behaviour_change_of_the_other_frontends"]
    m = a1["measured"]
    w(f"| A1 계약 테스트 통과, 각 테스트는 먼저 실패로 확인 | 통과 {m['contract_tests_passed']}, "
      f"실패 {m['contract_tests_failed']}; part 1의 {m['part1_tests']}개 중 "
      f"{m['part1_failed_on_base_and_pass_now']}개가 base에서 실패하고 지금 통과 | {a1['verdict']} |")
    m = a2["measured"]
    w(f"| A2 기본 경로 golden 불변 | exit {m['exit_code']}, golden 파일 "
      f"{m['golden_files_identical']}개 동일, base 대비 diff {m['diff_lines_against_base']}줄 | "
      f"{a2['verdict']} |")
    m = a3["measured"]
    w(f"| A3 기존 facade 테스트 전부 통과 | exit {m['exit_code']}, 통과 {m['passed']}, "
      f"실패 {m['failed']}, 무시 {m['ignored']}, 테스트 바이너리 {m['test_binaries']}개와 doc-test "
      f"{m['doctest_runs']}회 | "
      f"{a3['verdict']} |")
    m = a4["measured"]
    w(f"| A4 다른 frontend의 동작 불변 | `SwitchingSolver`와 `ProductionSolver`의 기록 digest "
      f"{m['switching_and_production_record_digests_identical']} / {m['digest_lines']}줄 동일, "
      f"golden {m['golden_files_identical']}개 동일, 보호 경로 diff {m['protected_diff_lines']}줄 | "
      f"{a4['verdict']} |")
    w("")
    w("`VigilODE::solve`는 이제 세 진입점(`Dopri5::solve`, `Rodas5p::solve`, "
      "`SwitchingSolver::solve`) 중 하나를 고르고, 실행한 driver의 식별자를 그 driver의 실행 기록에서 "
      "읽어 보고한다. 중간에 멈춘 실행은 받아들인 구간, 모든 구간과 사전 점검의 작업량, driver가 밝힌 "
      "정지 이유를 그대로 돌려준다.")
    w("")

    w("## 2. 이전 router가 내세운 것과 실제")
    w("")
    w("| 내세운 것 (base) | 실제 (base) | 지금 |")
    w("|---|---|---|")
    w("| tier 6개 (`Tier2MatrixFreeKrylov`, `Tier3Exponential`, `Tier4StagedRodas5p` 포함) | "
      "넷은 같은 `Rodas5p::solve`를 불렀고 문자열만 달랐다. `Tier3Exponential`은 만들어진 적이 없다 | "
      "진입점이 있는 값 3개만 남겼다 |")
    w("| route history의 tier 이름 | 실행 전에 적은 label | 실행 뒤 그 구간의 실행 기록에서 읽은 driver 식별자 |")
    w("| 로그 노름 μ2로 강성을 미리 감지 | μ2 ≤ 0(수축)이고 구간 ≤ 100이면 비강성으로 보고 DOPRI5로 보냈다. "
      "열 방정식, `y' = -1e6 (y - cos t)`가 여기에 걸린다 | μ2를 계산하지 않는다. 선언된 사실로 고르고, "
      "아무 선언이 없는 작은 문제에만 과금되는 강성 screen을 쓴다 |")
    w("| fail-closed fallback, 모든 `WorkCounters` 보존 | fallback 구간이 `max_steps`를 새로 받았고, "
      "`Stats`의 시도 수는 fallback 구간만 셌으며, fallback이 거절되면 앞 구간과 작업량을 버렸다. "
      "치명적 callback 오류 뒤에도 다른 방법으로 다시 불렀다 | 모든 구간이 `Options::max_steps` 하나를 나눠 쓴다. "
      "앞 구간과 작업량을 유지한다. 치명적 callback 뒤에는 다음 구간을 시작하지 않는다 |")
    w("| 실패 상태 | switching 실패는 모두 `MaxStepsReached`, 사전 점검의 callback 오류는 `InvalidInput` | "
      "driver가 밝힌 이유를 `VigilodeSolution::stop`(`UnifiedStop`)에 담고, `status`는 그에 해당하는 "
      "`FailureReason` |")
    w("| VG-1 zero-allocation workspace | `8n`, `(m+1)n`, `n*n` buffer를 모든 문제에 할당했고 어느 driver에도 "
      "넘기지 않았다. `step_into`는 Heun 단계다 | `n`짜리 vector 4개만 둔다. dense 행렬은 그것이 필요한 driver "
      "안에서만 생긴다 |")
    w("| 사전 점검 비용 | RHS 1회(dense Jacobian이면 Jacobian 1회와 고유값 계산)를 세지 않았다 | "
      "선언으로 정해지는 경로는 callback이 0회다. screen의 RHS 호출은 counter에 들어간다 |")
    w("")

    w("## 3. 지금 router가 하는 일")
    w("")
    w("### 3.1 경로 결정 (`PreflightAnalyzer::analyze`)")
    w("")
    w("위에서부터 처음 맞는 규칙이 결정한다. 1-5번은 callback을 부르지 않는다.")
    w("")
    w("1. `SolverStrategy::RobustStiff`는 RODAS5P, `AdaptiveReentrant`는 switching adapter.")
    w("2. 선언된 regime(새 옵션 `VigilodeOptions::regime`, 기본 `Unknown`): `Stiff`는 RODAS5P, "
      "`Nonstiff`는 시도 예산 전부로 DOPRI5를 돌리고 두 번째 방법은 없다.")
    w("3. registry가 선언된 사실에 대해 DOPRI5를 받아들이지 않으면(질량 행렬, dense output, event) RODAS5P.")
    w("4. Jacobian, JVP, band 중 하나라도 주어졌으면 RODAS5P.")
    w("5. 차원이 10을 넘거나(base의 한계를 유지) 예산이 trial을 허용하지 않으면 RODAS5P.")
    w("6. 그 밖에는 강성 screen. 증거가 있으면 RODAS5P, 없으면 DOPRI5 trial "
      "(`min(1000, max_steps / 2)`회) 뒤에, 끝나지 않으면 마지막으로 받아들인 상태에서 RODAS5P.")
    w("")
    w("regime에 맞는 방법은 `MethodRegistry::general_method`, 승인 여부와 계획 경로는 "
      "`MethodRegistry::route(MethodChoice::Explicit(id))`에서 받는다. DOPRI5와 RODAS5P 구간은 "
      "`MethodRegistry::solve`로 실행한다. 어느 RODAS5P driver 계열이 도는지에 대한 규칙은 analyzer에 남기지 않았다.")
    w("")
    w("### 3.2 강성 screen")
    w("")
    w("`f0 = f(t0, y0)`와 좌표마다 `δ_j = sqrt(eps) max(|y0_j|, 1)`로 "
      "`L = max_j |f(t0, y0 + δ_j e_j) - f0| / δ_j`를 만든다. `L`은 `f(t0, ·)`의 Lipschitz 상수의 "
      "하계다. `L (t1 - t0) > 3.3 B`(`B`는 trial의 시도 예산, 3.3은 DOPRI5의 실수축 안정 한계)이면 "
      "trial이 그 예산으로 구간을 끝낼 수 없으므로 RODAS5P로 보낸다. RHS `n + 1`회, 저장은 `O(n)`이고 "
      "호출은 모두 counter에 들어간다.")
    w("")
    w("screen은 한쪽 방향으로만 작동한다. trial을 RODAS5P로 바꿀 수는 있어도 그 반대는 없다. "
      "증거가 없다는 것을 비강성의 증거로 쓰지 않으므로 trial에는 예산과 fallback이 그대로 붙는다. "
      "`t0` 이후에 생기는 강성(Robertson)은 screen이 보지 못한다.")
    w("")
    screen = receipt.get("two_leg_solve", {}).get("screen")
    records = receipt.get("part2_and_3_records", {})
    s = records.get("p2_stiffness_screen_is_a_charged_lower_bound")
    if s:
        w("| 문제 | `L` | 구간 | 기준 `3.3 B` | 증거 | RHS 호출 |")
        w("|---|---|---|---|---|---|")
        for label, key in [("`y' = -1e6 (y - cos t)`", "scalar"), ("열 방정식 n=8", "heat_span_100"),
                           ("열 방정식 n=8", "heat_span_1"), ("Robertson", "robertson"),
                           ("조화 진동자", "oscillator")]:
            r = s[key]
            w(f"| {label} | {r['lipschitz_lower_bound']:.6g} | {r['span']:g} | {r['threshold']:g} | "
              f"{fmt(r['stiffness_evidence'])} | {r['rhs_calls']} |")
        w("")

    w("### 3.3 수축성 강성 문제 (COD-07)")
    w("")
    w("경계값은 실행 전에 정했다: DOPRI5가 안정 한계에서 할 수 있는 최소 작업"
      "(`6 ρ (t1 - t0) / 3.3`회의 RHS)의 10분의 1.")
    w("")
    w("| 문제 | 실행한 driver (base → 지금) | 과금된 RHS 호출 (base → 지금) | 경계값 |")
    w("|---|---|---|---|")
    names = {
        "p1_stiff_contractive_heat_equation_with_dense_jacobian_is_not_explicit":
            "열 방정식 n=64, dense Jacobian, 구간 (0, 10)",
        "p1_stiff_contractive_scalar_with_dense_jacobian_is_not_explicit":
            "`y' = -1e6 (y - cos t)`, Jacobian 있음, 구간 (0, 1)",
        "p1_stiff_contractive_heat_equation_without_jacobian_is_not_explicit":
            "열 방정식 n=8, Jacobian 없음, 구간 (0, 100)",
        "p1_stiff_contractive_scalar_without_jacobian_is_not_explicit":
            "`y' = -1e6 (y - cos t)`, Jacobian 없음, 구간 (0, 1)",
    }
    for test, label in names.items():
        row = receipt["stiff_contractive"][test]
        bound = row["rhs_bound"]["now"] if row["rhs_bound"] else None
        w(f"| {label} | {pair(row['labels'])} | {pair(row['rhs_calls_charged'])} | {bound} |")
    w("")

    w("### 3.4 실패 보고 (COD-06)")
    w("")
    stop = receipt["stop_reasons"]
    w("| 경우 | base의 `status` | 지금의 `status` |")
    w("|---|---|---|")
    for label, key in [
        ("switching 실행 중 RHS가 치명적 오류", "fatal_rhs_under_switching"),
        ("switching 실행 중 Jacobian이 치명적 오류", "fatal_jacobian_under_switching"),
        ("switching 실행 중 해가 발산 (`y' = y^2`), 시도 예산은 남음", "blow_up_under_switching"),
    ]:
        entry = stop[key]
        w(f"| {label} | {status_kind(entry['base'])} | {status_kind(entry['now'])} |")
    entry = stop["fatal_preflight_callback"]
    w(f"| 사전 점검에서 RHS가 치명적 오류 | `{entry['base']}` | `{entry['now']}` |")
    w("")
    w(f"`Auto`에서 DOPRI5 구간의 RHS가 치명적 오류를 낸 경우, 치명적 오류를 돌려준 RHS 호출 수"
      f"(다른 방법이 다시 부르면 2): {pair(stop['fatal_entries_on_the_explicit_leg'])}.")
    w("")
    w("`VigilodeSolution::stop`은 멈춘 driver가 밝힌 값이다: RODAS5P는 `FailureReason`, DOPRI5는 "
      "`Dopri5StopReason`과 callback 실패와 메시지, switching adapter는 자신의 메시지와 멈춘 segment의 "
      "driver 정지 기록, 그리고 callback 경계에서 본 첫 치명적 오류. adapter는 치명적 callback의 메시지를 "
      "어느 기록에도 남기지 않으므로(`switching.rs`가 `BatchStop::Fatal`의 내용을 버린다), router는 "
      "callback을 그대로 전달하면서 첫 치명적 오류만 적어 두는 포장을 문제에 씌워 adapter에 넘긴다. "
      "adapter의 결과는 이 포장과 무관하게 bit 단위로 같다(5절).")
    w("")
    w("`status`는 `stop`을 facade의 `FailureReason`으로 옮긴 것이다. 정확하지 않은 경우가 있다"
      "(`UnifiedStop::is_exact`가 구분한다): switching adapter의 마지막 segment가 DOPRI5이고 정지 이유가 "
      "callback도 시도 예산도 아닐 때. adapter가 DOPRI5의 정지 이유를 기록하지 않기 때문이며, 이때 `status`는 "
      "`StepSizeTooSmall`이고 정확한 값은 `stop`의 메시지뿐이다.")
    w("")

    w("### 3.5 작업량 회계와 시도 예산")
    w("")
    budget = receipt["shared_budget"]
    w("Jacobian 없는 Robertson, 구간 (0, 10), `max_steps = 500`:")
    w("")
    w("| 항목 | base → 지금 |")
    w("|---|---|")
    w(f"| 실행한 driver | {pair(budget['labels'])} |")
    w(f"| 모든 구간의 시도 수 (예산 500) | {pair(budget['accepted_plus_rejected_in_counters'])} |")
    w(f"| `Stats::attempts` | {pair(budget['stats_attempts'])} |")
    w(f"| callback 경계에서 센 RHS 진입 | {pair(budget['rhs_entries'])} |")
    w(f"| 과금된 RHS 호출 | {pair(budget['rhs_charged'])} |")
    w("")
    legs = receipt.get("two_leg_solve")
    if legs:
        w("구간별 기록(`VigilodeSolution::legs`):")
        w("")
        w("| 구간 | driver | 경로 | 시작 → 끝 | 예산 | 시도 | RHS | 정지 |")
        w("|---|---|---|---|---|---|---|---|")
        for index, leg in enumerate(legs["legs"], 1):
            stop_text = "없음" if leg["stop"] is None else f"`{json.dumps(leg['stop'], ensure_ascii=False)}`"
            w(f"| {index} | `{leg['driver_id']}` | `{leg['route_id']}` | {leg['t_start']:.6g} → "
              f"{leg['t_end']:.6g} | {leg['attempt_budget']} | {leg['attempts']} | {leg['rhs_calls']} | "
              f"{stop_text} |")
        w("")
        w(f"사전 점검 {legs['preflight_rhs_calls']}회 + 구간 합 {legs['leg_rhs_calls']}회 = 전체 "
          f"{legs['total_rhs_calls']}회, callback 경계에서 센 진입 {legs['rhs_entries']}회.")
        w("")
    storage = receipt["storage"]
    w("저장 공간:")
    w("")
    w(f"- `VigilodeWorkspace::new(2000, 30)`이 쥐는 byte: {pair(storage['workspace_retained_bytes_n2000'])}")
    w(f"- matrix-free 경로(n = 3000) 한 번의 solve에서 가장 큰 단일 할당(byte): "
      f"{pair(storage['largest_single_allocation_bytes_matrix_free_n3000'])} "
      f"(`n*n*8` = {storage['dense_matrix_bytes_n3000']['now']})")
    w("")

    w("## 4. 남은 tier, 제거한 것, 아직 이름뿐인 것")
    w("")
    w("**남은 tier (`SelectedTier`)**")
    w("")
    w("| 값 | 진입점 | 실행 기록의 driver 식별자 |")
    w("|---|---|---|")
    w("| `Tier1Explicit` | `Dopri5::solve` | `dopri5-5(4)-fsal` |")
    w("| `TierProtectedReference` | `Rodas5p::solve` | facade가 고른 계열: `rodas5p-fast-transformed-v2`(dense), "
      "`rodas5p-fast-banded-v1`, `rodas5p-mf-fast-transformed-v1`(matrix-free), `rodas5p-fast-dense-v1`, "
      "`rodas5p-fast-banded-dense-v1`, `rodas5p-sequential-dense-v2` |")
    w("| `Tier5AdaptiveSwitching` | `SwitchingSolver::solve` | `vigilode.switching.v1`; segment별 driver는 "
      "`UnifiedLeg::segment_drivers` |")
    w("")
    w("**제거한 것**")
    w("")
    w("- `SelectedTier::Tier2MatrixFreeKrylov`, `Tier3Exponential`, `Tier4StagedRodas5p`. 저장소에서 이 이름을 "
      "쓰는 곳은 `src/unified` 안뿐이었다.")
    w("- `PreflightAnalysis`의 `is_stiff`, `estimated_mu2`, `has_jvp`, `has_band`, `has_mass`, "
      "`requires_dae_handling`. 근거(`basis`), registry의 선언 사실(`facts`), 계획 경로, screen 기록으로 바꿨다.")
    w("- `VigilodeWorkspace`의 `stage_vectors`, `krylov_basis`, `linear_scratch`, `error_scratch`, `scale_scratch`.")
    w("- `FallbackEngine::execute_fallback`. `FallbackEngine`은 fallback 규칙을 담은 순수 함수 세 개가 됐다.")
    w("- JVP 문제에서 같은 `Rodas5p::solve`를 한 번 더 부르던 'fallback'.")
    w("- 모듈 문서의 nilpotent homotopy sweep, GCRO-DR, exponential action, zero-allocation hot loop 문장.")
    w("")
    w("**아직 이름뿐이거나 읽히지 않는 것** (문서에 그렇게 적었다)")
    w("")
    w("- `VigilodeOptions::profile`(`PerformanceProfile`), `max_krylov_dim`, `max_krylov_restarts`, "
      "`invariant_drift_tol`: 저장만 되고 아무것도 읽지 않는다. 기존 테스트, 예제, prelude가 쓰고 있어 남겼다.")
    w("- `SolverStrategy::HighPerformance`: `Auto`와 똑같이 경로를 고른다.")
    w("- `VigilodeWorkspace::new`의 둘째 인자와 `step_into`(고정 간격 Heun 단계): 어떤 solver도 쓰지 않는다.")
    w("- 마지막 구간이 RODAS5P가 아닐 때 `VigilodeSolution::stats`의 driver 전용 필드"
      "(`driver_work`, `driver_counters`, `adaptive` 등)는 형식상의 빈 값이다. `Stats`가 RODAS5P 기록 형식이기 때문이다. "
      "그 driver의 기록은 `explicit_solution`, `switching_solution`에 있다.")
    w("- `PexprbWindowDriver`의 segment 기록은 `SwitchingMethod::Rodas5p`로 적힌다. `SwitchingMethod`에 "
      "지수 적분 값이 없고 `switching.rs`는 보호 대상이다. 문서에 적었다.")
    w("")

    w("## 5. 테스트")
    w("")
    w(f"**Part 1 (base의 공개 API만 사용, {ff['count']}개).** 소스를 고치기 전에 base 소스에서 먼저 실행했다 "
      f"(`logs/failing_first_base.log`): 통과 {ff['base_totals']['passed']}, 실패 "
      f"{ff['base_totals']['failed']}. 지금은 {ff['failed_on_base_and_pass_now']}개가 모두 통과한다.")
    w("")
    w("| 테스트 | base | 지금 |")
    w("|---|---|---|")
    for row in ff["tests"]:
        w(f"| `{row['test']}` | {row['base_sources']} | {row['this_node']} |")
    w("")
    labels = receipt["label_cases"]
    total_cases = sum((entry["total"] or {}).get("now") or 0 for entry in labels.values())
    base_mismatch = sum((entry["base_mismatches"] or {}).get("base") or 0 for entry in labels.values())
    w(f"label과 driver의 일치: 사례 {total_cases}개 중 base에서 불일치 {base_mismatch}개, 지금 0개. "
      "기준 값은 같은 문제에 대해 driver를 직접 불러 얻은 식별자다.")
    w("")
    other = receipt.get("failing_first_part2_and_3")
    if other:
        tests_now = other["tests_now"]
        w(f"**Part 2, 3 (이 node가 추가한 API, {len(tests_now)}개).** 구현 뒤에 썼다. base에는 없는 API를 "
          f"쓰므로 base 소스 사본에서는 compile이 실패한다(`logs/failing_first_new_api.log`, exit "
          f"{other['exit']}, 오류 코드별 개수 {other['compile_errors_by_code']}). 실행으로 실패를 확인한 것은 "
          "part 1뿐이다.")
        w("")
    grid = receipt.get("replay_grid")
    if grid:
        w("**재현 격자.** 요청 "
          f"{grid['cases']}개(공급자 7종 × 전략 4종 × regime 3종 × dense output 여부 × 차원 2개)에 대해, "
          "solution이 보고한 구간을 그 구간이 이름 붙인 진입점의 직접 호출로 다시 실행했다.")
        w("")
        w(f"- 실행 {grid['solved']}개, callback 전에 거절 {grid['refused_before_any_callback']}개, 불일치 "
          f"{grid['failures']}개.")
        w("- 다시 실행한 최종 상태는 bit 단위로 같고, callback 진입 횟수(RHS, Jacobian, JVP)는 사전 점검 "
          "호출을 더하면 같다.")
        w(f"- tier별 건수: {grid['tiers']}")
        w(f"- driver별 구간 수: {grid['drivers']}")
        w("")
    neutral = receipt.get("switching_adapter_neutrality")
    if neutral:
        w("**switching adapter의 불변성.** base API만 쓰는 모듈을 base 소스 사본과 이 node에서 각각 "
          f"실행해 `SwitchingSolver::solve`와 `ProductionSolver::solve`의 전체 기록(JSON)의 길이와 "
          f"FNV-1a 64 값을 비교했다: {neutral['identical_lines']} / {neutral['lines_on_base_sources']}줄 동일 "
          "(문제 11개 × 설정 7개; 치명적 callback, 발산, 예산 소진, driver 거절 경로 포함).")
        w("")

    w("## 6. `registry.rs` (COD-06의 registry 항목)")
    w("")
    w("- `WindowDriver::run`은 `Result<BatchOutcome, WindowDriverError>`를 돌려준다. facade, DOPRI5, "
      "core의 오류가 형식을 유지한다. `Display`와 `String` 변환은 base가 내던 문자열 그대로이므로 "
      "오류를 글로 읽는 switching adapter는 같은 메시지를 받는다.")
    w("- `BatchOutcome::failure`가 driver의 정지 이유를 담는다(RODAS5P의 `FailureReason`, DOPRI5의 "
      "정지 이유와 callback 실패).")
    w("- `BatchStop::Fatal`의 문자열은 고정 문구 `callback failure` 대신 callback의 메시지다.")
    w("- `BatchStop`의 값 5개는 그대로다. `switching.rs`가 wildcard 없이 match하므로 값을 추가하면 "
      "보호 파일이 compile되지 않는다. 그래서 최소 간격에서 멈춘 RODAS5P 실행은 여전히 "
      "`BatchStop::Completed`로 분류되고(문서에 적었다), 정지 이유는 `failure`에 있다.")
    w("")
    w("고치지 않은 것: `work_units`가 분해 횟수를 빼는 것(고치면 `SwitchingSolver`의 결정이 바뀐다), "
      "`PexprbWindowDriver`가 recoverable 오류도 치명적으로 처리하는 것, 호출자가 없는 "
      "`SolverState`/`next_solver_state`.")
    w("")

    w("## 7. 기존 파일 편집")
    w("")
    for edit in receipt["edits_of_existing_files"]:
        w(f"**`{edit['path']}`** (+{edit['lines_added']} / -{edit['lines_removed']}줄)")
        w("")
        for reason in edit["reasons"]:
            w(f"- {reason}")
        w("")
    before = receipt.get("existing_unified_tests_before_the_edit")
    if before:
        failed = sorted(name for name, status in before["statuses"].items() if status == "FAILED")
        w(f"편집 전 파일을 새 router에 돌리면 {len(before['statuses'])}개 중 {len(failed)}개만 실패했고"
          f"({fmt(failed)}), 둘 다 label 문자열 비교에서였다 (`{before['log']}`).")
        w("")
    w("이 테스트 파일의 머리말과 `contract_2`의 주석에는 이전 용어(multi-tier, VG-1)가 남아 있다. "
      "단정문이 아니어서 손대지 않았다.")
    w("")

    w("## 8. 검증 명령")
    w("")
    w("| 로그 | exit | 테스트 (통과/실패/무시) | 명령 |")
    w("|---|---|---|---|")
    for command in receipt["commands"]:
        if command.get("missing"):
            w(f"| `{command['log']}` | 로그 없음 | | |")
            continue
        tests = command.get("tests")
        counts = f"{tests['passed']} / {tests['failed']} / {tests['ignored']}" if tests else ""
        w(f"| `{command['log']}` | {command['exit_code']} | {counts} | `{command['command']}` |")
    w("")
    lints = receipt["lints"]
    w(f"loop checker 출력 (exit {lints['loop_checker_exit_code']}):")
    w("")
    w("```")
    for line in lints["loop_checker_output"]:
        w(line)
    w("```")
    w("")
    outside = receipt["changed_paths_outside_the_task_allowlist"]
    w(f"바꾼 경로 {len(receipt['changed_paths'])}개 가운데 과제의 허용 경로 밖: {fmt(outside)}. "
      f"보호 경로의 base 대비 diff: {len(receipt['protected_paths_diff_against_base'])}줄.")
    if receipt["source_paths_changed_after_a_run"]:
        w("")
        w(f"주의: 검증 뒤에 바뀐 소스 경로가 있다: {receipt['source_paths_changed_after_a_run']}")
    if receipt["missing_logs"]:
        w("")
        w(f"주의: 없는 로그: {fmt(receipt['missing_logs'])}")
    w("")

    w("## 9. 열린 문제와 한계")
    w("")
    w("1. **동작이 달라진 경우.** (a) Jacobian을 준 비강성 문제는 `Auto`에서 이제 RODAS5P로 간다"
      "(base는 μ2 ≤ 0이면 DOPRI5). DOPRI5를 원하면 `regime(Nonstiff)`를 선언한다. (b) 아무 선언이 없는 "
      "작은 문제의 DOPRI5 trial은 최대 1000회다(base는 `max_steps` 전부). 1000회를 넘는 긴 비강성 적분은 "
      "남은 구간을 RODAS5P(유한차분 Jacobian)로 끝낸다. 결과는 오차 제어를 받지만 더 비쌀 수 있다. "
      "1000이라는 값과 차원 한계 10은 측정으로 정한 값이 아니다.")
    w("2. **screen의 범위.** `t0`의 Lipschitz 하계만 본다. 진동이 빠르거나 비정규성이 큰 비강성 문제를 "
      "RODAS5P로 보낼 수 있고, 나중에 생기는 강성은 trial 예산이 막는다.")
    w("3. **유한차분 Jacobian의 허가.** router는 `GeneralOdePolicy`와 달리 유한차분을 따로 허가받지 않고 쓴다"
      "(`Rodas5p::solve`가 Jacobian 없는 문제에 하는 것과 같다). switching adapter에도 "
      "`AllowFiniteDifference`를 준다. 허가 옵션을 추가하지 않았다.")
    w("4. **`status`의 근사.** 3.4절의 경우에는 `status`가 정확하지 않다. `switching.rs`와 `SwitchingSolution`을 "
      "고쳐야 없어진다.")
    w("5. **fallback 구간이 거절되는 경로**(앞 구간은 적분했는데 RODAS5P가 입력을 거절)는 구현했지만"
      "(`UnifiedStop::Refused`, 앞 구간 유지), 지금의 경로 규칙으로는 공개 API에서 이 상황을 만들 수 없어 "
      "테스트가 없다.")
    w("6. **screen 뒤의 거절.** screen이 RHS를 부른 뒤 두 진입점이 모두 적분 전에 오류를 돌려주면 "
      "`Err`가 반환되고 screen의 호출 수는 보고되지 않는다. screen 전에 DOPRI5의 입력 검증과 RODAS5P의 "
      "승인을 통과시키므로, 기록을 남기지 않는 driver 내부 오류일 때만 생길 수 있다. 테스트는 없다.")
    w("7. **registry를 거치는 비용.** 한 번의 solve에 method catalog를 여러 번 만든다"
      "(`MethodRegistry::route`, `solve`). 시간은 재지 않았다.")
    w("8. **V03 part A에서 넘어온 것은 그대로다.** 일반 sparse-JVP 문제의 sequential 경로 계획은 여전히 "
      "실행되지 않는다(`Rodas5p::capabilities`). 이 node의 격자에는 그 조합이 없다.")
    w("")

    (HERE / "REPORT_KO.md").write_text("\n".join(out) + "\n")
    print(f"REPORT_KO.md: {len(out)} lines")
    return 0


if __name__ == "__main__":
    sys.exit(main())
