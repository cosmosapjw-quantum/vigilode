#!/usr/bin/env python3
"""Writes REPORT_KO.md of loop node V07 (stage a) from RESULTS.json.

The text is fixed in this script; every number in the report is formatted from
RESULTS.json (built by build_results.py from the raw logs), so no result is typed in.

Run:  python3 -I build_report.py
"""
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent


def f3(x):
    return "-" if x is None else "%.3f" % x


def f2(x):
    return "-" if x is None else "%.2f" % x


def sci(x):
    return "-" if x is None else "%.1e" % x


def rt(x):
    return "%.0e" % x


def table(header, rows):
    lines = ["| " + " | ".join(header) + " |", "|" + "|".join("---" for _ in header) + "|"]
    lines += ["| " + " | ".join(str(c) for c in row) + " |" for row in rows]
    return "\n".join(lines)


def cell_list(cells):
    return ", ".join("%s rtol %s (%s)" % (c[0], rt(c[1]), f3(c[2])) for c in cells) or "없음"


def main():
    results = json.loads((HERE / "RESULTS.json").read_text())
    predeclared = json.loads((HERE / "PREDECLARED.json").read_text())
    verdicts = results["verdicts"]
    summary = results["adaptive"]["summary"]
    cells = results["adaptive"]["cells"]
    order = results["order"]
    explore = results["exploratory_no_gate"]
    checks = results["protocol_checks"]
    inputs = results["provenance"]["inputs"]
    order_class = {p["id"]: p["class"] for p in predeclared["order_set"]["problems"]}
    ko = {"PASS": "PASS", "FAIL": "FAIL", "KILL": "KILL", "NOT_DECIDABLE": "판정 불가",
          "NOT_DISCRIMINATING": "비판별"}
    reference = json.loads((HERE / "reference" / "reference.json").read_text())
    jitter = ("protected_rtol_up", "protected_rtol_down")
    discriminating = [p for p in order["problems"] if p["fit"]["exact_discriminating"]]
    others = [p for p in order["problems"] if not p["fit"]["exact_discriminating"]]

    def slope_gap(problems, arm):
        return max(abs(p["fit"]["slopes"][arm] - p["fit"]["slopes"]["exact"]) for p in problems)

    jitter_ratios = [c["arms"][a]["error"]["error_ratio"] for c in cells for a in jitter
                     if c["arms"][a].get("error", {}).get("error_ratio") is not None]
    reject_fractions = [c["protected"]["rejected"] / c["protected"]["attempts"] for c in cells]
    numerical = {k: v["differences_max_rel"] for k, v in results["reference_summary"].items()
                 if v["kind"] == "numerical" and k != "e5"}
    b_over_c = [d["B"] / d["C"] for d in numerical.values()]
    a_and_c = [d[k] for d in numerical.values() for k in ("A", "C")]
    fixed_runs = sum(len(p["grid"]) for p in order["problems"])
    e5_scale = max(abs(v) for v in reference["adaptive"]["e5"]["y_ref"])
    by_problem = explore["E4_error_ratio_context"]["by_problem"]
    e5_control_fail = sum(by_problem["e5"][a]["FAIL"] for a in jitter)
    e5_control_cells = sum(sum(by_problem["e5"][a].values()) for a in jitter)
    fit_h = [h for p in discriminating for h in p["fit"]["h_used"]]
    cell_of = {(c["problem"], c["rtol"]): c for c in cells}

    out = []
    add = out.append
    add("# V07 투기적 Jacobian, 단계 a (순차 모사) 결과 보고")
    add("")
    add("- 노드: `V07_SPECULATIVE_JACOBIAN`, 단계 a만 수행 (thread 없음, wall time 주장 없음). 단계 b는 만들지 않았다.")
    add("- 브랜치 `research/production-loop-20261116-w2-v07`. 사전선언 `PREDECLARED.json`은 첫 commit(`16cd264d`)이고, "
        "측정 로그는 코드 commit `%s`에서 만들었다(로그 머리말에 기록)."
        % (inputs["logs/measure_adaptive.log"]["git_head"] or "?")[:8])
    add("- 이 보고서의 수치는 전부 `RESULTS.json`에서 `build_report.py`가 찍은 것이다. `RESULTS.json`은 "
        "`build_results.py`가 raw 로그(`logs/`)와 `reference/reference.json`만 읽어 만든다.")
    add("")

    # ------------------------------------------------------------ 1. conclusion
    add("## 1. 결론")
    add("")
    rows = []
    for mode in ("g8", "g7"):
        v = verdicts[mode]
        step, err = summary[mode]["step"], summary[mode]["error"]
        rows.append([
            mode.upper(),
            "%s (중앙값 %s, 최대 %s)" % (ko[v["criteria"]["step_ratio"]], f3(step["median"]),
                                    f3(step["max"])),
            "%s (33셀 중 PASS %d, FAIL %d, 판정 불가 %d)" % (
                ko[v["criteria"]["error_ratio"]], err["cells_pass"], err["cells_fail"],
                err["cells_not_decidable"]),
            "%s (판별 문제의 최소 기울기 %s)" % (
                ko[v["criteria"]["observed_order"]],
                f3(order["summary"][mode]["min_slope_on_discriminating"])),
            "해당 없음" if mode == "g8" else ("발동" if v["kill"] else "미발동"),
            "**%s**" % ko[v["verdict"]],
        ])
    add(table(["모드", "step 비 (중앙값 ≤ 1.02, 최대 ≤ 1.05)", "오차 비 ([0.67, 1.5])",
               "관측 차수 (≥ 4.8)", "KILL (g7 step 비 > 1.10)", "단계 a 판정"], rows))
    add("")
    g8e, g7e = summary["g8"]["error"], summary["g7"]["error"]
    up, down = summary["protected_rtol_up"], summary["protected_rtol_down"]
    add("요점은 네 가지다.")
    add("")
    add("1. **step 수와 차수는 유지된다.** 33개 셀(11문제 × rtol 3개)에서 시도 수 비의 중앙값은 두 모드 모두 "
        "%s이고 최대는 G8 %s(%s, rtol %s), G7 %s(%s, rtol %s)이다. 1.10을 넘는 셀은 없다(KILL 미발동). "
        "판별 가능한 %d개 문제에서 관측 차수는 정확 Jacobian과 G8 %s, G7 %s 이내로 같다."
        % (f3(summary["g8"]["step"]["median"]), f3(summary["g8"]["step"]["max"]),
           summary["g8"]["step"]["max_cell"][0], rt(summary["g8"]["step"]["max_cell"][1]),
           f3(summary["g7"]["step"]["max"]), summary["g7"]["step"]["max_cell"][0],
           rt(summary["g7"]["step"]["max_cell"][1]), len(discriminating),
           f3(slope_gap(discriminating, "g8")), f3(slope_gap(discriminating, "g7"))))
    add("2. **오차 비 기준은 사전선언한 엄격 해석(모든 셀이 [0.67, 1.5] 안)으로 두 모드 모두 FAIL이다.** "
        "G8은 %d셀(전부 e5), G7은 %d셀이 띠 밖이다. 사전선언 규칙대로 판정을 FAIL로 적는다."
        % (g8e["cells_fail"], g7e["cells_fail"]))
    add("3. **그러나 같은 규칙은 보호 방법 자신도 통과하지 못한다.** 방법은 그대로 두고 rtol과 atol만 1.02배 "
        "또는 1/1.02배로 바꾼 대조 실행(탐색적 E4)의 오차 비도 %d셀과 %d셀에서 띠를 벗어난다. "
        "즉 이 문제 집합에서 끝점 오차 비의 셀별 잡음이 띠 폭보다 크고, 이 기준은 투기적 Jacobian의 효과와 "
        "step 열이 조금 달라진 효과를 구분하지 못한다. e5에서는 보호 방법의 끝점 오차 자체가 허용오차의 "
        "%s배에서 %s배다."
        % (up["error"]["cells_fail"], down["error"]["cells_fail"],
           sci(explore["E4_error_ratio_context"]["by_problem"]["e5"][
               "protected_error_tolerance_units_min"]),
           sci(explore["E4_error_ratio_context"]["by_problem"]["e5"][
               "protected_error_tolerance_units_max"])))
    lag = summary["lagged"]["step"]
    add("4. **한 step 지연된 Jacobian(탐색적 대조)은 이론 S4대로 무너진다.** 시도 수 비 중앙값 %s, 최대 %s, "
        "관측 차수 %s(정확 %s)와 %s(정확 %s). g7·g8에서 평가한 Jacobian과는 질적으로 다르다."
        % (f3(lag["median"]), f3(lag["max"]),
           f3(order["problems"][0]["fit"]["slopes"].get("lagged")),
           f3(order["problems"][0]["fit"]["slopes"].get("exact")),
           f3(order["problems"][1]["fit"]["slopes"].get("lagged")),
           f3(order["problems"][1]["fit"]["slopes"].get("exact"))))
    add("")
    add("판정을 바꿀 권한은 소유자에게 있다. 오차 비 기준을 셀별 엄격 해석으로 유지하면 두 모드 모두 FAIL이고 "
        "단계 b는 시작하지 않는다. 대조군 잡음을 근거로 기준의 집계 방식(예: 중앙값, 대조군 대비 분포, "
        "허용오차를 못 맞춘 문제의 처리)을 정하는 것은 이 노드가 사후에 할 수 없는 결정이다(9절).")
    add("")

    # ------------------------------------------------------------ 2. what was built
    add("## 2. 만든 것")
    add("")
    add("`crates/rodas5p-integrators/src/stage_clients/speculative.rs` (보호 파일 무수정, trait 그대로 사용).")
    add("")
    add("- `StageArgumentRecorder`와 `wrap_rhs`: 우변 callback을 감싸 모든 호출을 그대로 전달하고, "
        "`factor` 이후 처음 7번 호출의 시간·상태 인자를 복사한다. `OdeProblem`의 callback이 private이라 "
        "문제를 만들기 전에 우변에 씌운다.")
    add("- `SpeculativeJacobianClient<C>`: 임의의 `StageLinearSystem` client를 감싼다. `factor`와 "
        "`solve_in_place`는 그대로 넘기고, `linearize(t, y)`만 모드의 점에서 호출한다. "
        "모드: `Exact`(대조), `G8`, `G7`, `LaggedOneStep`(탐색적 대조).")
    add("- trait만으로 표현됐으므로 별도 연구 driver는 만들지 않았다. driver의 stage, controller, 출력은 "
        "보호 driver 그대로다.")
    add("")
    add("### 2.1 어느 호출이 어느 stage인가 (가정하지 않고 테스트로 증명)")
    add("")
    add("fast driver의 fresh attempt는 `linearize` → `f(t, y)` → (비자율이면 시간 미분) → `factor` → "
        "stage 2..8의 우변 평가 7회 순서다. `factor`와 attempt 끝 사이에 다른 우변 호출은 없다. "
        "`tests/stage_clients_speculative_contracts.rs`가 이를 두 방법으로 확인한다.")
    add("")
    add("- 변환형(U-form) 거울: 매 accepted step을 공개 kernel로 driver와 같은 연산 순서로 다시 계산한다. "
        "거울의 새 상태가 driver의 accepted 상태와 bit 단위로 같고, `factor` 뒤 k번째 기록 호출이 "
        "stage k+1의 인자와 bit 단위로 같으며 시간 인자는 `t + c_{k+1} h`다. "
        "HIRES, Robertson, 비자율 PR(해석적 f_t / 차분 f_t), Brusselator N=100(거절 뒤 수락된 step 포함)에서 확인.")
    add("- 독립 형식(k-form): 변환하지 않은 Rosenbrock 식으로 같은 step을 계산해 기록값과 반올림 수준으로 "
        "일치함을 확인하고, g7, g8, 새 상태 사이 간격이 그 불일치보다 천 배 이상 큼을 단언한다"
        "(수치는 테스트 출력 `SPECJ_KFORM_JSON`).")
    add("- 이론 S5의 항등식 `g7 = g6 + U6`, `g8 = g7 + U7`, `y_new = g8 + U8`이 부동소수 합으로 bit 단위 성립함도 "
        "확인했다. 즉 g8은 embedded 해이고 `y_new - g8`이 오차 추정 벡터다.")
    add("")
    add("### 2.2 예외 규칙 (정확 Jacobian을 쓰는 경우)")
    add("")
    add("1. 실행의 첫 linearization (이전 attempt가 없음).")
    add("2. 직전 linearization과 같은 시각의 linearization. driver는 error형 실패(`LinearSolve`, "
        "`NonlinearSolve`, `NonFinite`) 뒤의 재시도에서만 그렇게 한다. 실패 원인이 투기점일 수 있고, "
        "threaded driver도 그 시점에는 accepted 상태를 이미 갖고 있어 겹칠 것이 없다. "
        "stage 평가 실패, factorization 전 `f(t, y)` 실패, 투기점에서의 Jacobian 실패 세 경우를 테스트했다.")
    add("3. recorder에 그 시각에서 끝나는 완결된 attempt가 없을 때 (wrapper를 거친 여분의 우변 호출, "
        "`reset` 없이 재사용된 client 등).")
    add("")
    add("**국소 오차 거절 시**: driver는 linearization을 유지한다. 재시도 전에 `linearize`가 아니라 더 작은 h로 "
        "`factor`만 부른다. 따라서 재시도는 거절된 attempt가 쓰던 Jacobian(직전 accepted attempt의 g7/g8에서 "
        "평가한 것, 또는 예외 규칙의 정확 Jacobian)을 그대로 쓴다. 거절된 attempt의 stage 인자는 재시도의 "
        "것으로 덮이고 한 번도 쓰이지 않는다. 재시도가 수락되면 다음 linearization은 그 재시도의 g7/g8을 쓴다.")
    add("")
    add("**시간 인자**: 투기적 Jacobian은 기록된 stage 평가의 시간 인자 `t_n + c_i h`에서 평가한다. "
        "stage 7, 8은 `c_i = 1`이고 driver가 같은 합으로 시계를 전진시키므로 accepted 시각 `t_{n+1}`과 "
        "bit 단위로 같다(테스트로 확인, 다르면 예외 규칙 3).")
    add("")
    add("**시간 미분 f_t**: client의 일이 아니다. driver가 fresh attempt마다 accepted `(t_{n+1}, u_{n+1})`에서 "
        "직접 구한다(`partial_t` callback, 없으면 t에 대한 중심차분으로 우변 2회). 따라서 f_t와 "
        "`f(t_{n+1}, u_{n+1})`은 모든 모드에서 정확하고 Jacobian만 투기적이다. 테스트가 f_t 평가점이 "
        "전부 accepted 상태임을 bit 단위로 확인한다.")
    add("")
    add("### 2.3 테스트 (`cargo test`, release)")
    add("")
    add("- `stage_clients_speculative_contracts` 17개: Exact 모드와 보호 dense driver의 bit 동일성"
        "(in-place LU: Robertson, HIRES, Brusselator N=32/100, 비자율 PR 2종; faer LU: mean-field "
        "Brusselator n=200; 각 rtol 3개; counter 전 항목, 출력, linearization 점), wrapper 결과 중립성"
        "(호출 단위와 보호 driver 전체 실행), 호출-stage 대응, 모드별 linearization 점, 실패 뒤 정확 재시도, "
        "f_t 평가점, 오용 거부와 `reset`, 모델 검증(해석적 Jacobian 대 차분, 닫힌 해, 저장소의 기존 정의와 "
        "bit 일치).")
    add("- `stage_clients_speculative_alloc` 1개(전용 binary): 5문제 × 4모드 × rtol 3개에서 warm-up 뒤 "
        "accepted step 사이 할당 0, 실행 전체 할당 수가 step 수와 무관.")
    add("- `stage_clients_speculative_measure`: 측정 프로그램 3개(`#[ignore]`).")
    add("")

    # ------------------------------------------------------------ 3. protocol
    add("## 3. 측정 프로토콜 요약 (`PREDECLARED.json`)")
    add("")
    add("- arm: `protected`(보호 dense driver, wrapper 없는 문제), `exact`(client, Exact 모드; 매 셀에서 "
        "protected와 bit 동일해야 함), `g8`, `g7`, 탐색적 `lagged`와 tolerance 교란 대조 2개.")
    add("- 적응 집합 11문제 × rtol {1e-4, 1e-6, 1e-8}: robertson, vdpol-eps1e-6, oregonator, hires, "
        "pollution, e5, brusselator-1d-32/100, pr-nonlinear(비자율, 상태 의존 stiff rate), "
        "combustion-100, nldiff-100(준선형 확산). atol = rtol × 문제별 scale(e5는 1.7e-24 고정), "
        "initial_step 1e-6, 출력은 양 끝점만.")
    add("- step 비 = 시도 수(accepted + rejected) 비, 정수 산술로 비교. 끝점 오차 = controller 가중치의 "
        "WRMS(허용오차 단위). 오차 비 구간은 참조 불확실성 U로 계산.")
    add("- 참조: SciPy Radau rtol 1e-13(값), 불확실성 `u = |ref − A| + max(|ref − B|, |ref − C|)`, "
        "A = SciPy Radau rtol 1e-11, B = Julia RadauIIA5, C = Julia Rodas5P (둘 다 reltol 1e-13). "
        "pr-nonlinear와 차수 문제는 닫힌 해.")
    add("- 차수 집합 5문제, 고정 step h = 1.5/N (N 16개, 전부 이진 유리수). 고정 step은 "
        "`initial_step = max_step = h`, `rtol = atol = 1e10`으로 강제하고 실행마다 검증. "
        "맞춤 구간은 exact arm만으로 정하는 규칙(큰 h부터 내려가 exact 오차가 1e-11 아래로 떨어지기 전까지, "
        "그중 가장 작은 h 6개).")
    add("")
    yes = {True: "예", False: "아니오"}
    add("프로토콜 점검 (`RESULTS.json`의 `protocol_checks`): 셀 목록이 사전선언과 같음 %s, 차수 문제가 같음 %s, "
        "N 목록이 같음 %s, 두 로그 완결 %s/%s, 모든 exact arm이 protected와 bit 동일 %s, 유효 셀 %d/%d, "
        "Rust와 Python 모델 정의 일치 %s (probe %s개, 우변 최대 상대차 %s)."
        % (yes[checks["adaptive_cells_match_predeclared"]],
           yes[checks["order_problems_match_predeclared"]],
           yes[checks["order_steps_match_predeclared"]], yes[checks["adaptive_log_complete"]],
           yes[checks["fixed_step_log_complete"]], yes[checks["all_exact_arms_equal_protected"]],
           sum(1 for c in cells if c["valid"]), len(cells), yes[checks["model_definitions_agree"]],
           checks["model_probes"], sci(checks["largest_model_rhs_relative_difference"])))
    add("")

    # ------------------------------------------------------------ 4. results
    add("## 4. 결과 (동결 판별 기준)")
    add("")
    add("### 4.1 step 비와 Jacobian 수")
    add("")
    rows = []
    for c in cells:
        p, a = c["protected"], c["arms"]
        rows.append([
            c["problem"], rt(c["rtol"]), "%d (%d)" % (p["attempts"], p["rejected"]),
            "%d / %s" % (a["g8"]["attempts"], f3(a["g8"]["step"]["step_ratio"])),
            "%d / %s" % (a["g7"]["attempts"], f3(a["g7"]["step"]["step_ratio"])),
            "%d / %d / %d" % (p["jacobian_builds"], a["g8"]["jacobian_builds"],
                              a["g7"]["jacobian_builds"]),
            f3(a["protected_rtol_up"]["step"]["step_ratio"]),
            f3(a["protected_rtol_down"]["step"]["step_ratio"]),
            f3(a["lagged"]["step"]["step_ratio"]),
        ])
    add(table(["문제", "rtol", "protected 시도 (거절)", "G8 시도 / 비", "G7 시도 / 비",
               "Jacobian 수 prot / G8 / G7", "대조 rtol×1.02 비", "대조 rtol/1.02 비",
               "lagged 비 (탐색적)"], rows))
    add("")
    for mode in ("g8", "g7"):
        s = summary[mode]["step"]
        lin = summary[mode]["linearizations"]
        add("- %s: 중앙값 %s, 최소 %s, 최대 %s. 비 = 1인 셀 %d, < 1인 셀 %d, > 1인 셀 %d, > 1.02인 셀 %d, "
            "> 1.05인 셀 %d, > 1.10인 셀 %d. 전체 시도 수 합의 비 %s, Jacobian 수 합의 비 %s. "
            "linearization %d회 중 투기점 %d, 첫 step 정확 %d, 실패 뒤 정확 %d, 점 없음 %d. → **%s**"
            % (mode.upper(), f3(s["median"]), f3(s["min"]), f3(s["max"]), s["cells_equal_1"],
               s["cells_below_1"], s["cells_above_1"], s["cells_above_1_02"],
               len(s["cells_above_1_05"]), len(s["cells_above_1_10"]),
               "%.4f" % s["attempts_total_ratio"],
               "%.4f" % summary[mode]["jacobian_builds"]["ratio"], lin["total"],
               lin["speculative"], lin["exact_first"], lin["exact_after_failure"],
               lin["exact_no_point"], ko[s["verdict"]]))
    add("- KILL (g7의 step 비 > 1.10): %s." % ("발동" if summary["kill_g7"] else "해당 셀 없음, 미발동"))
    add("- 참고(탐색적): tolerance 교란 대조의 step 비 최대는 %s와 %s다. 투기 모드의 최대(%s, %s)와 같은 크기다."
        % (f3(up["step"]["max"]), f3(down["step"]["max"]), f3(summary["g8"]["step"]["max"]),
           f3(summary["g7"]["step"]["max"])))
    add("")
    add("### 4.2 끝점 오차 비")
    add("")
    add("`e`는 허용오차 단위의 끝점 오차(WRMS), `U`는 같은 단위의 참조 불확실성이다. 비 옆 글자는 셀 판정"
        "(P = PASS, F = FAIL, N = 판정 불가)이다.")
    add("")

    def err(entry):
        e = entry.get("error")
        if not e or e.get("error_ratio") is None:
            return "-"
        return "%s %s" % (f3(e["error_ratio"]), e["verdict"][0])

    rows = []
    for c in cells:
        a = c["arms"]
        rows.append([
            c["problem"], rt(c["rtol"]), sci(c["protected"].get("end_point_error")),
            sci(c.get("reference_uncertainty")), err(a["g8"]), err(a["g7"]),
            err(a["protected_rtol_up"]), err(a["protected_rtol_down"]), err(a["lagged"]),
        ])
    add(table(["문제", "rtol", "e(protected)", "U", "G8 비", "G7 비", "대조 rtol×1.02",
               "대조 rtol/1.02", "lagged (탐색적)"], rows))
    add("")
    for mode in ("g8", "g7"):
        e = summary[mode]["error"]
        bad = ", ".join("%s rtol %s (%s, %s)" % (c[0], rt(c[1]), f3(c[2]), ko[c[3]])
                        for c in e["cells_not_pass"])
        add("- %s: PASS %d, FAIL %d, 판정 불가 %d. 비의 중앙값 %s, 최소 %s, 최대 %s. PASS가 아닌 셀: %s. → **%s** "
            "(엄격 해석)"
            % (mode.upper(), e["cells_pass"], e["cells_fail"], e["cells_not_decidable"],
               f3(e["median"]), f3(e["min"]), f3(e["max"]), bad, ko[e["verdict"]]))
    add("")
    ctx = explore["E4_error_ratio_context"]
    add("해석에 필요한 사실 (탐색적, 판정에 쓰지 않음):")
    add("")
    rows = []
    for arm, name in (("g8", "G8"), ("g7", "G7"), ("protected_rtol_up", "대조 rtol×1.02"),
                      ("protected_rtol_down", "대조 rtol/1.02")):
        s = ctx["spread"][arm]
        e = summary[arm]["error"]
        rows.append([name, "%d / %d / %d" % (e["cells_pass"], e["cells_fail"],
                                             e["cells_not_decidable"]),
                     s["cells_outside_band"], f3(s["median_abs_log_ratio"]),
                     f3(s["q90_abs_log_ratio"]), f3(s["max_abs_log_ratio"])])
    add(table(["arm", "셀 PASS / FAIL / 판정 불가", "점 추정이 띠 밖인 셀",
               "|ln 비| 중앙값", "|ln 비| 90% 분위", "|ln 비| 최대"], rows))
    add("")
    rows = []
    for problem, v in ctx["by_problem"].items():
        rows.append([problem, "%s – %s" % (sci(v["protected_error_tolerance_units_min"]),
                                           sci(v["protected_error_tolerance_units_max"])),
                     v["g8"]["FAIL"], v["g7"]["FAIL"], v["protected_rtol_up"]["FAIL"],
                     v["protected_rtol_down"]["FAIL"]])
    add(table(["문제", "e(protected) 범위 (허용오차 단위)", "G8 FAIL 셀", "G7 FAIL 셀",
               "대조 ×1.02 FAIL 셀", "대조 /1.02 FAIL 셀"], rows))
    add("")
    add("- 대조 실행은 방법을 바꾸지 않는다. 그런데도 %d셀과 %d셀이 띠를 벗어난다. 끝점 오차는 step 열의 "
        "작은 변화에 민감해서, 같은 방법의 두 실행 사이에서도 비가 %s에서 %s까지 움직인다."
        % (up["error"]["cells_fail"], down["error"]["cells_fail"], f3(min(jitter_ratios)),
           f3(max(jitter_ratios))))
    add("- G8의 FAIL은 전부 e5다. e5에서는 보호 방법의 끝점 오차가 허용오차의 %s–%s배다. 참조 끝값의 "
        "크기는 %s(초기값은 1.76e-3)이고 보호 방법의 끝 상태는 자릿수부터 다르다. 원인은 조사하지 않았다. "
        "대조 실행도 e5 %d셀 중 %d셀이 FAIL이다. 이 문제의 오차 비는 방법 비교의 근거가 되지 못한다."
        % (sci(ctx["by_problem"]["e5"]["protected_error_tolerance_units_min"]),
           sci(ctx["by_problem"]["e5"]["protected_error_tolerance_units_max"]), sci(e5_scale),
           e5_control_cells, e5_control_fail))
    g7_other = [c for c in summary["g7"]["error"]["cells_not_pass"]
                if c[0] != "e5" and c[3] == "FAIL"]
    add("- G7의 e5 밖 FAIL 셀: %s. 비가 0.67보다 작은 셀은 투기 모드의 오차가 더 **작은** 경우다."
        % "; ".join("%s rtol %s (오차 비 %s, step 비 %s)" % (
            c[0], rt(c[1]), f3(c[2]),
            f3(cell_of[(c[0], c[1])]["arms"]["g7"]["step"]["step_ratio"])) for c in g7_other))
    add("- oregonator rtol 1e-8의 판정 불가는 참조 불확실성 때문이다. 사전선언 규칙이 두 Julia 실행 중 "
        "큰 차이를 쓰는데, RadauIIA5(B)가 요청 허용오차보다 훨씬 부정확하다(7절). B를 빼면 이 셀은 PASS다"
        "(탐색적 E6: G8 PASS %d / FAIL %d, G7 PASS %d / FAIL %d). 다른 셀의 판정은 바뀌지 않는다."
        % (explore["E6_error_verdicts_without_reference_run_B"]["g8"]["PASS"],
           explore["E6_error_verdicts_without_reference_run_B"]["g8"]["FAIL"],
           explore["E6_error_verdicts_without_reference_run_B"]["g7"]["PASS"],
           explore["E6_error_verdicts_without_reference_run_B"]["g7"]["FAIL"]))
    add("")
    add("### 4.3 관측 차수 (고정 step)")
    add("")
    rows = []
    for p in order["problems"]:
        fit = p["fit"]
        s = fit["slopes"]
        rows.append([
            p["problem"], order_class.get(p["problem"], ""),
            "%d–%d" % (fit["steps_used"][0], fit["steps_used"][-1]) if fit["steps_used"] else "-",
            f3(s.get("exact")), f3(s.get("g8")), f3(s.get("g7")), f3(s.get("lagged")),
            "예" if fit["exact_discriminating"] else "아니오",
            "%s / %s" % (ko[fit["mode_verdicts"]["g8"]], ko[fit["mode_verdicts"]["g7"]]),
        ])
    add(table(["문제", "분류", "맞춤에 쓴 N (6점)", "exact", "G8", "G7", "lagged (탐색적)",
               "판별 문제 (exact ≥ 4.8)", "G8 / G7"], rows))
    add("")
    add("- 고정 step 실행 %d개(문제 × N)의 다섯 arm이 모두 사전선언한 고정 step 검증을 통과했고"
        "(accepted = N, 거절 0, 최소·최대 accepted step = h bit 단위), exact arm은 전부 protected와 bit 동일하다: %s."
        % (fixed_runs, all(p["all_runs_fixed_step"] and p["all_exact_runs_equal_protected"]
                           for p in order["problems"])))
    add("- 판별 문제는 %s (비stiff). G8의 기울기는 exact와 %s 이내, G7은 %s 이내로 같고 모두 4.8 이상이다. "
        "→ G8 **%s**, G7 **%s**."
        % (", ".join(p["problem"] for p in discriminating), f3(slope_gap(discriminating, "g8")),
           f3(slope_gap(discriminating, "g7")), ko[order["summary"]["g8"]["verdict"]],
           ko[order["summary"]["g7"]["verdict"]]))
    add("- stiff 문제 %s (Prothero–Robinson형 두 개 포함)는 exact arm 자체의 관측 차수가 %s–%s이라 사전선언대로 "
        "비판별이다(stiff 영역의 차수 저하). 이 문제들에서도 G8·G7의 기울기는 exact와 같다"
        "(차이 G8 %s, G7 %s 이내). pr-linear는 Jacobian이 상수라 모든 arm이 같은 값을 낸다."
        % (", ".join(p["problem"] for p in others),
           f2(min(p["fit"]["slopes"]["exact"] for p in others)),
           f2(max(p["fit"]["slopes"]["exact"] for p in others)),
           f3(slope_gap(others, "g8")), f3(slope_gap(others, "g7"))))
    add("- 판별 문제의 맞춤 구간은 h = %s–%s로, 약간 점근 전 영역이다. 더 작은 h에서의 국소 차수는 "
        "`RESULTS.json`의 `order.problems[].local_orders`에 있다."
        % (f3(max(fit_h)), f3(min(fit_h))))
    add("")

    # ------------------------------------------------------------ 5. exploratory
    add("## 5. 탐색적 결과 (판정에 쓰지 않음)")
    add("")
    add("### 5.1 한 step 지연된 Jacobian (E1)")
    add("")
    le = summary["lagged"]["error"]
    add("- 적응 실행: 시도 수 비 중앙값 %s, 최대 %s(%s, rtol %s), > 1.10인 셀 %d/33. rtol이 작을수록 나빠진다"
        "(오차 추정이 지연 항 O(h³)에 지배됨). 오차 비 PASS %d, FAIL %d."
        % (f3(lag["median"]), f3(lag["max"]), lag["max_cell"][0], rt(lag["max_cell"][1]),
           len(lag["cells_above_1_10"]), le["cells_pass"], le["cells_fail"]))
    add("- 고정 step 관측 차수: %s. 이론 S4의 예측(고전 차수 3)과 맞는다."
        % ", ".join("%s %s" % (p["problem"], f3(p["fit"]["slopes"].get("lagged")))
                    for p in order["problems"]))
    add("")
    add("### 5.2 Jacobian 차이의 크기 (E2, E3)")
    add("")
    add("고정 step, 차수 맞춤과 같은 6점에서 step별 최대 Frobenius 노름의 h에 대한 기울기 "
        "(예측: g7 4, g8 5, 지연 1; 괄호는 점 자체의 거리 `max|g − u_{n+1}|`의 기울기):")
    add("")
    rows = []
    for problem, slopes in explore["E2_jacobian_differences_fixed_step"].items():
        if not slopes:
            rows.append([problem, "-", "-", "-"])
            continue
        rows.append([problem,
                     "%s (%s)" % (f2(slopes["e7_abs_max"]), f2(slopes["d7_inf_max"])),
                     "%s (%s)" % (f2(slopes["e8_abs_max"]), f2(slopes["d8_inf_max"])),
                     "%s (%s)" % (f2(slopes["e_lag_abs_max"]), f2(slopes["d_lag_inf_max"]))])
    add(table(["문제", "J(g7) − J(u)", "J(g8) − J(u)", "J(u_n) − J(u_{n+1})"], rows))
    add("")
    e2 = explore["E2_jacobian_differences_fixed_step"]
    add("- 비stiff 문제에서는 예측(4, 5, 1)에 가깝다. stiff 문제에서는 stage 인자 자체의 차수가 떨어진다"
        "(pr-nonlinear의 거리 기울기: g7 %s, g8 %s). pr-linear는 Jacobian이 상수라 차이가 0이다."
        % (f2(e2["pr-nonlinear"]["d7_inf_max"]), f2(e2["pr-nonlinear"]["d8_inf_max"])))
    series = explore["E2_series"]
    if series:
        add("- 적응 실행 한 개(%s, rtol %s, %d step)의 상대 크기 `‖J(점) − J(u)‖_F / ‖J(u)‖_F`: "
            "g8 중앙값 %s 최대 %s, g7 중앙값 %s 최대 %s, 지연 중앙값 %s 최대 %s."
            % (series["problem"], rt(series["rtol"]), series["steps"],
               sci(series["e8_abs_over_norm_j"]["median"]), sci(series["e8_abs_over_norm_j"]["max"]),
               sci(series["e7_abs_over_norm_j"]["median"]), sci(series["e7_abs_over_norm_j"]["max"]),
               sci(series["e_lag_abs_over_norm_j"]["median"]),
               sci(series["e_lag_abs_over_norm_j"]["max"])))
    add("")
    add("적응 실행의 셀별 요약 (중앙값 / 최대). 거리는 허용오차 단위 WRMS다.")
    add("")
    rows = []
    for c in cells:
        pr = c["jacobian_probe"]

        def mm(q):
            return "%s / %s" % (sci(q["median"]), sci(q["max"]))
        rows.append([c["problem"], rt(c["rtol"]), mm(pr["d8_tolerance_units"]),
                     mm(pr["d7_tolerance_units"]), mm(pr["e8_rel"]), mm(pr["e7_rel"]),
                     mm(pr["e_lag_rel"])])
    add(table(["문제", "rtol", "거리 g8", "거리 g7", "상대 J차 g8", "상대 J차 g7", "상대 J차 지연"],
              rows))
    add("")
    probes = [c["jacobian_probe"] for c in cells]

    def over_cells(name, statistic, pick):
        return pick(pr[name][statistic] for pr in probes)

    add("- g8은 accepted 상태에서 오차 추정 벡터만큼 떨어져 있다(수락 조건이 이 벡터의 노름 ≤ 1). "
        "거리의 셀별 중앙값은 g8 %s–%s, g7 %s–%s 허용오차 단위이고, 셀별 최대값 중 가장 큰 것은 g8 %s, g7 %s다."
        % (sci(over_cells("d8_tolerance_units", "median", min)),
           sci(over_cells("d8_tolerance_units", "median", max)),
           sci(over_cells("d7_tolerance_units", "median", min)),
           sci(over_cells("d7_tolerance_units", "median", max)),
           sci(over_cells("d8_tolerance_units", "max", max)),
           sci(over_cells("d7_tolerance_units", "max", max))))
    add("- Jacobian의 상대 변화(셀별 최대값 중 가장 큰 것): g8 %s, g7 %s, 지연 %s. 셀별 중앙값 중 가장 큰 것: "
        "g8 %s, g7 %s, 지연 %s."
        % (sci(over_cells("e8_rel", "max", max)), sci(over_cells("e7_rel", "max", max)),
           sci(over_cells("e_lag_rel", "max", max)), sci(over_cells("e8_rel", "median", max)),
           sci(over_cells("e7_rel", "median", max)), sci(over_cells("e_lag_rel", "median", max))))
    add("")
    add("### 5.3 matched error에서의 step 비 (E5)")
    add("")
    rows = []
    for arm in ("g8", "g7", "lagged"):
        m = explore["E5_matched_error"][arm]
        skipped = [r["problem"] for r in m["rows"] if r["status"] != "ok"]
        rows.append([arm.upper() if arm != "lagged" else "lagged", m["values"], f3(m["median"]),
                     "%s (%s, rtol %s)" % (f3(m["min"]), m["min_cell"][0], rt(m["min_cell"][1])),
                     "%s (%s, rtol %s)" % (f3(m["max"]), m["max_cell"][0], rt(m["max_cell"][1])),
                     ", ".join(skipped) or "없음"])
    add(table(["arm", "평가된 셀", "중앙값", "최소", "최대", "평가 불가 문제 (오차가 rtol에 단조가 아님)"],
              rows))
    add("")
    add("- 문제별 세 점 (오차, 시도 수)을 log-log 보간해, protected가 달성한 오차에서 그 arm이 필요로 하는 "
        "시도 수를 추정한 값이다. 오차와 step 수를 한 번에 본 비용 비교다. e5의 오차는 4.2절에서 본 대로 "
        "잡음이므로 e5가 포함된 arm의 최소·최대는 의미가 없다.")
    add("")

    # ------------------------------------------------------------ 6. reference
    add("## 6. 참조해")
    add("")
    rows = []
    for problem, item in results["reference_summary"].items():
        if item["kind"] != "numerical":
            rows.append([problem, "닫힌 해", "-", "-", "-"])
            continue
        d = item["differences_max_rel"]
        rows.append([problem, "SciPy Radau 1e-13", sci(d.get("A")), sci(d.get("B")),
                     sci(d.get("C"))])
    add(table(["문제", "참조값", "A: Radau 1e-11와의 최대 상대차", "B: Julia RadauIIA5",
               "C: Julia Rodas5P"], rows))
    add("")
    add("- 모든 참조 실행이 성공했다. e5를 뺀 문제에서 SciPy 두 허용오차와 Julia Rodas5P의 최대 상대차는 "
        "%s–%s다. Julia RadauIIA5(B)의 차이는 Rodas5P(C)의 %.0f배에서 %.0f배이고 불확실성 `u`를 지배한다"
        "(사전선언 규칙: 둘 중 큰 값)."
        % (sci(min(a_and_c)), sci(max(a_and_c)), min(b_over_c), max(b_over_c)))
    add("- e5의 상대차 열은 0에 가까운 성분(참조값 1e-70 이하) 때문에 의미가 없다. e5의 U는 허용오차 단위로 %s이고 "
        "측정된 오차(%s–%s 단위)보다 훨씬 작다."
        % (sci(max(c["reference_uncertainty"] for c in cells if c["problem"] == "e5")),
           sci(by_problem["e5"]["protected_error_tolerance_units_min"]),
           sci(by_problem["e5"]["protected_error_tolerance_units_max"])))
    add("- Rust와 Python 정의는 probe %s개에서 우변의 최대 상대차가 %s다(`logs/check_models.log`). "
        "Julia 정의는 세 번째로 옮겨 적은 것이며, SciPy 참조와의 일치(C 열)가 그 검증이다."
        % (checks["model_probes"], sci(checks["largest_model_rhs_relative_difference"])))
    add("")

    # ------------------------------------------------------------ 7. deviations
    add("## 7. 사전선언 대비 편차와 주의")
    add("")
    add("1. 고정 step 검증에서 protected arm에는 accept control이 없어 `AcceptedStepView::h_used`를 읽을 수 없다. "
        "protected의 accepted step은 controller telemetry trace"
        "(`integrate_rodas5p_fast_observed_with_telemetry`, scalar atol 경로)에서 읽었다. "
        "exact arm이 %d개 실행 전부에서 protected와 bit 동일하다." % fixed_runs)
    add("2. E2의 고정 step 기울기를 어느 h 구간에서 맞출지 사전선언에 없었다. 차수 맞춤과 같은 6점을 썼다.")
    add("3. E5(matched error)의 보간 세부(오차를 셀의 허용오차 단위 크기로 환산, 끝 구간 외삽, 오차가 rtol에 "
        "단조가 아니면 제외)는 측정 뒤에 정했다. 탐색적 항목이다.")
    add("4. 4.2절의 대조군 대비 분포 표, 문제별 FAIL 표, B를 뺀 불확실성(E6)은 사전선언에 없던 사후 분석이다. "
        "판정에 쓰지 않았다.")
    add("5. 계약 테스트는 사전선언 commit 뒤, 측정 실행 전에 돌렸다. 이 테스트는 HIRES, pr-nonlinear, "
        "Brusselator N=100의 rtol 1e-6에서 G8·G7·lagged의 시도 수를 출력한다. 그 뒤 프로토콜은 바꾸지 않았다.")
    add("6. 사전선언 전에 실행한 것은 `PREDECLARED.json`의 `status_at_commit`에 적은 두 가지뿐이다"
        "(SciPy만 쓰는 PDE 특성 파악, Julia 환경 smoke 실행).")
    add("7. 참조 스크립트는 `python3 -I`(시스템 SciPy)로 돌렸다. 버전은 `reference/reference.json`에 있다. "
        "두 참조 스크립트는 실행 당시 commit 전이었고 그 뒤 수정 없이 commit했다(Julia 스크립트의 SHA-256은 "
        "로그 머리말에 있다).")
    add("")

    # ------------------------------------------------------------ 8. stage b
    add("## 8. 단계 b(threaded overlap)에 추가로 필요한 것")
    add("")
    add("단계 a가 답한 것은 \"J를 g7/g8에서 평가해도 방법이 유지되는가\"뿐이다. 실제로 겹쳐 돌리려면 다음이 더 필요하다.")
    add("")
    add("1. **G8만으로는 단계 b 통과 기준에 닿지 못한다.** g8은 stage 7의 solve가 끝나야 알 수 있으므로 "
        "겹칠 수 있는 구간은 stage 8 하나(사슬의 1/8)다. 이론 S6의 모델에서 그 상한은 9/8 = 1.125로, "
        "동결된 단계 b 기준 1.15보다 작다. 기준에 닿을 수 있는 것은 두 stage를 겹치는 G7뿐이다"
        "(상한 1.25, r ∈ [0.15, 0.917]에서 ≥ 1.15).")
    add("2. **factorization을 겹치려면 step 크기 예측이 필요하다.** `W = I/(hγ) − J`이고 다음 h는 stage 8과 "
        "controller가 끝나야 정해진다(오차 추정이 U8). 예측 없이 겹칠 수 있는 것은 Jacobian 평가뿐이며, "
        "그 경우 S6의 r은 `T_J / T_chain`이다. LU까지 겹치려면 (a) 보호 controller와 양립하는 h 예측기와 "
        "(b) 예측이 빗나갈 때의 재factorization 비용, (c) 적중률 측정이 필요하다. 적중률은 이번에 재지 않았다. "
        "controller 상수는 보호 대상이라 \"예측한 h를 그대로 쓴다\"는 선택은 할 수 없다.")
    add("3. **stage 인자가 준비된 시점을 알리는 통로.** 현재 trait에는 그런 호출이 없다. 단계 a는 우변 wrapper가 "
        "`factor` 뒤 호출 순번으로 stage를 식별했다. 단계 b는 같은 wrapper가 6번째 호출(stage 7의 인자 g7)에서 "
        "worker를 깨우고, 다음 `linearize`에서 결과를 받으면 된다. 보호 파일을 고치지 않고 client로 만들 수 있다.")
    add("4. **버리는 작업의 처리.** 거절된 attempt와 error로 실패한 attempt에서 시작한 투기 계산은 쓰지 않는다"
        "(단계 a의 규칙과 같다). 이번 측정에서 protected의 거절 비율은 셀에 따라 %.0f%%–%.0f%%다(4.1절 표). "
        "worker 취소 또는 결과 무시가 필요하고, 그 시간은 이득에서 빠진다."
        % (100 * min(reject_fractions), 100 * max(reject_fractions)))
    add("5. **이중 버퍼.** stage 7–8이 W_n의 인자를 쓰는 동안 worker가 J_{n+1}을 쓰므로 Jacobian 행렬이 "
        "둘 필요하다(LU까지 겹치면 인자 저장도 둘).")
    add("6. **callback의 동시 호출 가능성.** worker의 Jacobian callback과 main thread의 우변 callback이 "
        "동시에 돈다. 타입은 `Send + Sync`지만 사용자 모델의 내부 가변 상태가 있으면 계약으로 금지하거나 "
        "epoch로 막아야 한다.")
    add("7. **결정성 검증.** threaded G7의 결과는 이번 순차 G7 모사와 bit 단위로 같아야 한다. "
        "이번 client가 그 oracle이다.")
    add("8. **착수 조건(동결).** V04가 (J+LU)/chain ∈ [0.15, 0.92]이고 step당 wall ≥ 500 µs인 영역을 "
        "보여야 한다. 2번 때문에 LU 예측이 없으면 조건은 사실상 J/chain에 대한 것이 된다. "
        "wall 측정은 고정 core, arm 교차, warm-up 2 + 반복 7, load 기록이 필요하다.")
    add("")

    # ------------------------------------------------------------ 9. open issues
    add("## 9. 한계와 소유자 결정이 필요한 사항")
    add("")
    add("1. **오차 비 기준의 해석.** 동결 행은 \"error ratio in [0.67, 1.5]\"라고만 적고 집계 방식을 정하지 않았다. "
        "이 노드는 측정 전에 \"모든 셀\"로 사전선언했고 그 결과는 두 모드 모두 FAIL이다. 대조군이 같은 규칙에서 "
        "FAIL이므로, 이 FAIL이 투기적 Jacobian 때문이라는 증거는 없다. 반대로 \"문제없다\"는 것도 이 기준으로는 "
        "증명되지 않았다. 집계 방식을 다시 정할지(예: 대조군 분포와의 비교, 보호 방법이 허용오차를 맞춘 셀로 제한)는 "
        "소유자 결정이다.")
    add("2. **e5의 끝점.** 보호 RODAS5P는 이 설정(t_end = 1e13, atol 1.7e-24, 배정밀도)에서 e5의 끝값을 "
        "맞추지 못한다(오차가 허용오차의 %s–%s배). 이것은 V07과 무관한 보호 방법의 관찰이며 원인은 조사하지 "
        "않았다. 별도 기록이 필요할 수 있다."
        % (sci(by_problem["e5"]["protected_error_tolerance_units_min"]),
           sci(by_problem["e5"]["protected_error_tolerance_units_max"])))
    invariant = explore.get("E7_e5_linear_invariant")
    if invariant:
        protected_rows = [r for r in invariant["runs"] if r["arm"] == "protected"]
        add("   단서 하나(탐색적 E7): 이 정식화(f3 = f2 − f4)에서 `y3 − y2 + y4`는 값이 0인 선형 불변량이고, "
            "정확한 산술에서는 Rosenbrock step이 이를 보존한다(Jacobian의 행도 같은 관계를 만족한다). "
            "protected 끝 상태의 잔차는 %s이고, 참조 끝값의 크기는 %s, 참조의 잔차는 %s다. 즉 끝점 오차는 이 "
            "불변량의 부동소수 표류와 같은 크기다. 조건수가 큰 W의 solve에서 증폭된 반올림으로 추정하지만 "
            "확인하지 않았다."
            % (", ".join("%s (rtol %s)" % (sci(r["y3_minus_y2_plus_y4"]), rt(r["rtol"]))
                         for r in protected_rows),
               sci(invariant["reference_end_value_size"]), sci(invariant["reference_residual"])))
    add("3. **차수 기준의 근거 폭.** 판별 문제가 비stiff %d개뿐이다. stiff 문제에서는 보호 방법의 관측 차수가 "
        "%s–%s이라 4.8 기준을 적용할 수 없었다(사전선언대로 비판별). stiff 영역의 근거는 적응 실행의 "
        "step 비와 고정 step 기울기의 일치다."
        % (len(discriminating), f2(min(p["fit"]["slopes"]["exact"] for p in others)),
           f2(max(p["fit"]["slopes"]["exact"] for p in others))))
    add("4. **문제 크기.** n ≤ 200, dense 저장, 해석적 Jacobian만 썼다. banded·sparse client나 차분 Jacobian은 "
        "다루지 않았다(wrapper는 inner client와 무관하게 동작한다).")
    add("5. **참조 불확실성.** B(Julia RadauIIA5)가 부정확해 한 셀이 판정 불가가 됐다. `u`는 추정이지 증명된 "
        "상계가 아니다.")
    add("")

    # ------------------------------------------------------------ 10. reproduce
    add("## 10. 재현")
    add("")
    add("저장소 루트에서, `export CARGO_TARGET_DIR=/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w2-v07`와 "
        "`taskset -c 0-9,12-21`을 앞에 붙여 실행했다.")
    add("")
    add("```")
    add("cargo +1.94.1 test --locked -j 4 --release -p rodas5p-integrators \\")
    add("  --test stage_clients_speculative_contracts --test stage_clients_speculative_alloc \\")
    add("  --test stage_clients_speculative_problems --test stage_clients_speculative_measure")
    add("D=research/production_loop_20261116/speculative_jacobian")
    add("bash $D/run_measurements.sh                 # logs/measure_{model_probes,adaptive,fixed_step}.log")
    add("python3 -I $D/reference/check_models.py $D/logs/measure_model_probes.log > $D/logs/check_models.log")
    add("(cd $D && python3 -I reference/reference.py > logs/reference_scipy.log 2>&1)")
    add("JULIA_PKG_OFFLINE=true julia --startup-file=no --project=tools/benchmarks/component_julia \\")
    add("  $D/reference/reference.jl > $D/logs/reference_julia.log 2>&1")
    add("python3 -I $D/reference/build_reference.py  # reference/reference.json")
    add("python3 -I $D/build_results.py              # RESULTS.json")
    add("python3 -I $D/build_report.py               # REPORT_KO.md")
    add("```")
    add("")
    add("입력 로그의 SHA-256과 각 로그의 commit, 명령, exit code는 `RESULTS.json`의 `provenance`에 있다. "
        "wall time은 재지 않았고 수치는 전부 결정적 counter와 부동소수 상태다. 검증 명령(테스트, sentinel, "
        "clippy, rustfmt, loop checker, 엄격 JSON, 허용 경로, 측정 재실행의 byte 동일성)과 그 exit code는 "
        "`run_checks.sh`가 쓴 `logs/checks.log`에 있다.")
    add("")
    (HERE / "REPORT_KO.md").write_text("\n".join(out))
    print("REPORT_KO.md written: %d lines" % len(out))
    return 0


if __name__ == "__main__":
    sys.exit(main())
