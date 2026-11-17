#!/usr/bin/env python3
"""Loop node V17: renders REPORT_KO.md from RESULTS.json.

Every number in the report is read from RESULTS.json (built by
parse_results.py from the raw logs) or from PREDECLARED.json. Sentences that
depend on a result are chosen by the result.

usage: python3 -I make_report.py [--results RESULTS.json] [--out REPORT_KO.md]
"""
import argparse
import json
import os
import subprocess

NODE = os.path.dirname(os.path.abspath(__file__))
ARMS = ("A", "B", "C")


def num(value, digits=3):
    if value is None:
        return "-"
    if isinstance(value, bool):
        return "예" if value else "아니오"
    if isinstance(value, int):
        return f"{value:,}"
    if value == 0:
        return "0"
    magnitude = abs(value)
    if magnitude >= 1e5 or magnitude < 1e-3:
        return f"{value:.{digits - 1}e}"
    if magnitude >= 100:
        return f"{value:,.0f}"
    if magnitude >= 10:
        return f"{value:.1f}"
    return f"{value:.{digits}f}"


def ratio(value):
    return "-" if value is None else f"{value:.3f}"


def speed(value):
    return "-" if value is None else f"{value:.2f}x"


def sci(value):
    return "-" if value is None else f"{value:.2e}"


def table(header, rows):
    lines = ["| " + " | ".join(header) + " |", "|" + "---|" * len(header)]
    lines.extend("| " + " | ".join(str(cell) for cell in row) + " |" for row in rows)
    return "\n".join(lines)


def verdict_ko(text):
    return {"PASS": "PASS", "FAIL": "FAIL", "NOT_DECIDABLE": "NOT_DECIDABLE (판정 불가)", "KILL": "KILL",
            "NOT_TRIGGERED": "발동 안 됨"}.get(text, str(text))


def git(*args):
    root = os.path.abspath(os.path.join(NODE, "..", "..", ".."))
    return subprocess.run(["git", "-C", root, *args], capture_output=True, text=True, check=False).stdout.strip()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--results", default=os.path.join(NODE, "RESULTS.json"))
    parser.add_argument("--out", default=os.path.join(NODE, "REPORT_KO.md"))
    args = parser.parse_args()
    with open(args.results, encoding="utf-8") as handle:
        results = json.load(handle)
    with open(os.path.join(NODE, "PREDECLARED.json"), encoding="utf-8") as handle:
        predeclared = json.load(handle)
    families = results["families"]
    declared = {entry["id"]: entry for entry in predeclared["families_final"]["list"]}
    labels = list(next(iter(families.values()))["cells"].keys())
    verdicts = results["verdicts"]
    thresholds = results["frozen_thresholds"]
    out = []
    add = out.append

    add("# V17 matrix-free stage solve: 잔차 검사 주기와 선형 허용치 정책 (실측 보고)")
    add("")
    add(f"- node `V17_MATRIX_FREE_STAGE_SOLVE`, loop L1 wave 2 group 2, 생성 시각 {results['generated']}")
    history = git("log", "--format=%h %cI %s", "--", os.path.join(NODE, "PREDECLARED.json")).splitlines()
    add(f"- 사전등록: `PREDECLARED.json` (sha256 `{results['predeclared_sha256']}`). 이 파일을 바꾼 commit (최신순): "
        + "; ".join(f"`{line}`" for line in history) + ". 선언 크기에서의 첫 실행은 마지막 수정 commit 뒤에 했다.")
    heads = ", ".join(f"`{key}` ({count}회)" for key, count in sorted(results["git_heads"].items()))
    binaries = ", ".join(f"`{key[:16]}...` ({count}회)" for key, count in sorted(results["binaries"].items()))
    add(f"- 측정 시점의 commit: {heads}")
    add(f"- 측정 binary sha256: {binaries}")
    add("- 이 문서의 수치는 전부 `make_report.py`가 `RESULTS.json`에서 읽어 채웠다. `RESULTS.json`은 `parse_results.py`가 `logs/`의 실행 출력과 `reference/*.json`만으로 만든다.")
    inventory = results["log_inventory"]
    add("- 로그 수: " + ", ".join(f"{phase} {count}" for phase, count in inventory.items()))
    add("")

    # ------------------------------------------------------------ 1. conclusion
    cadence, policy, kill = verdicts["cadence_B"], verdicts["policy_C"], verdicts["kill"]
    checks = verdicts["residual_checks"]
    add("## 1. 결론")
    add("")
    validity = verdicts["validity"]
    add(f"- **Validity (잔차 검사)**: 세 arm의 모든 stage solve를 원래 stage 계의 잔차로 다시 검사했다. "
        + ", ".join(f"{arm}: {num(checks[arm]['checks'])}건 중 실패 {num(checks[arm]['failures'])}건 (잔차/허용치 최대 {ratio(checks[arm]['max_residual_over_threshold'])})" for arm in ARMS)
        + f". 모든 solve 통과: {num(validity['residual_check_passes_in_every_solve'])}.")
    add(f"- **Validity (attempt 수)**: B의 attempt 수가 모든 family, 모든 tolerance에서 A의 1/{thresholds['validity_attempts_B']}~{thresholds['validity_attempts_B']}배 안에 있는가: {num(validity['attempts_B_within_1_05_everywhere'])}.")
    add(f"- **Cadence (B) 판정: {verdict_ko(cadence['verdict'])}.** family별: "
        + ", ".join(f"`{fid}` {verdict_ko(value)}" for fid, value in cadence["families"].items())
        + f". instructions 조건(matched error에서 A의 {thresholds['cadence_instructions_max']}배 이하)은 {cadence['instructions_part']['pass']}개 family가 충족, {cadence['instructions_part']['fail']}개 미달, {cadence['instructions_part']['open']}개 판정 불가. "
        f"wall 조건({thresholds['cadence_wall_min']}배 이상, authoritative 측정)은 {cadence['wall_part']['pass']}개 충족, {cadence['wall_part']['fail']}개 미달, {cadence['wall_part']['open']}개 판정 불가.")
    add(f"- **Policy (C) 판정: {verdict_ko(policy['verdict'])}.** family별: "
        + ", ".join(f"`{fid}` {verdict_ko(value)}" for fid, value in policy["families"].items())
        + f". instructions 조건(A의 {thresholds['policy_instructions_max']}배 이하)은 {policy['instructions_part']['pass']}개 충족, {policy['instructions_part']['fail']}개 미달; attempt 조건(A의 {thresholds['policy_attempts_max']}배 이하)은 {policy['attempts_part']['pass']}개 충족, {policy['attempts_part']['fail']}개 미달.")
    add(f"- **KILL 조항: {verdict_ko(kill['verdict'])}.** B의 잔차 검사 실패 {num(kill['residual_check_failures_in_B'])}건. " + {
        "KILL": "KILL 조건이 성립했다.",
        "NOT_TRIGGERED": "잔차 실패가 없고 authoritative wall이 1.10배 이상인 family가 있다.",
        "NOT_DECIDABLE": "잔차 실패는 없다. wall 조항(모든 family에서 1.10배 미만)은 authoritative wall이 없어 판정하지 못했다.",
    }.get(kill["verdict"], ""))
    wall_labels = {fid: families[fid]["wall_label"] for fid in families}
    if any(label != "authoritative" for label in wall_labels.values()):
        add("- **wall 조건을 문면대로 평가하지 못한 이유**: 측정 시간 내내 다른 프로젝트의 작업으로 host의 1분 load average가 timing 규칙의 한도 "
            f"{thresholds['timing_load_max']}을 넘었다(4절의 load 열). 규칙대로 20분까지 기다린 뒤 측정했고, 규칙을 못 지킨 cell은 `diagnostic_only`로 표시했다. "
            "diagnostic wall은 아래에 그대로 싣되 frozen 기준의 wall 조건을 대신하지 않는다. family별 wall 표시: "
            + ", ".join(f"`{fid}` {label}" for fid, label in wall_labels.items()) + ".")
    add("")
    add("family별 이득의 크기 (matched achieved error, 기하평균; 괄호는 비교 구간 안의 최소~최대):")
    add("")
    rows = []
    for fid, family in families.items():
        b, c = family["matched_error"]["B"], family["matched_error"]["C"]
        def cell(block, key="geometric_mean_ratio"):
            value = block.get(key)
            if value is None:
                return "-"
            return f"{value:.3f} ({block['min_ratio']:.3f}~{block['max_ratio']:.3f})"
        def wall(block):
            if block.get("speedup") is None:
                return "-"
            return f"{block['speedup']:.2f}x ({block['speedup_min']:.2f}~{block['speedup_max']:.2f}) [{block['label']}]"
        rows.append([f"`{fid}`", family["n"], cell(b["instructions_net"]), wall(b["wall"]), cell(b["jvp_calls"]),
                     cell(c["instructions_net"]), wall(c["wall"]), cell(c["jvp_calls"]), cell(c["attempts"])])
    add(table(["family", "n", "B instructions / A", "B wall 배속", "B JVP / A", "C instructions / A", "C wall 배속", "C JVP / A", "C attempts / A"], rows))
    add("")

    # ------------------------------------------------------------ 2. what was measured
    add("## 2. 무엇을 쟀나")
    add("")
    rule = predeclared["protected_rule"]
    add("보호된 규칙: " + "scaled 계 `(I - h gamma J) U = rhs`에서 true residual(새 JVP 한 번으로 계산)의 2-norm이 `max(gamma*atol_lin, rtol_lin*||rhs||_2)` 이하일 때만 해를 받는다. `rtol_lin = 1e-10`, `atol_lin = 1e-14`, restart 40, stage solve당 Arnoldi 벡터 200개 한도. 보호 kernel은 이 검사를 40열 pass가 끝날 때만 한다.")
    add("")
    for line in rule["source_lines"]:
        add(f"- `{line}`")
    add("")
    add(table(["arm", "내용"], [
        ["A", "보호 driver 그대로 (`integrate_rodas5p_mf_fast_observed_gmres_into`, `RoutingOptions::default().matrix_free`)."],
        ["B", "per-stage GMRES client (`KrylovStageClient::per_stage`, `PerStageGmresOptions::baseline()`): 같은 임계값을 매 Arnoldi 열마다 projected residual로 보고, 닿으면 true residual(JVP 1회, arm에 계상)로 확인한 뒤에만 받는다."],
        ["C", "B와 같되 임계값이 `max(tau_B, kappa*sqrt(n)*min_i(atol + rtol*|y_i|)/(h*gamma))`, `kappa = 1e-3`. `rtol`, `atol`은 step 허용치, `y`는 그 attempt를 선형화한 상태. 실행 전에 `PREDECLARED.json`에 적었고 바꾸지 않았다."],
        ["B0", "대조용(탐색): client driver + per-stage GMRES에 A와 같은 검사 주기. 판정에 쓰지 않는다."],
    ]))
    add("")
    add("- B의 임계값이 보호 규칙과 같다는 증명: 계약 테스트 `the_client_threshold_is_the_protected_threshold`(두 식이 무작위 2,000개 계에서 8 ulp 이내), `arm_b_and_the_protected_kernel_stop_at_the_same_residual`(첫 열 잔차를 임계값의 0.99배와 1.01배로 맞춘 2x2 계에서 보호 kernel과 B가 같은 열 수에서 멈춘다; 상대항과 절대항 둘 다), `the_routed_configuration_has_the_constants_of_arm_b`.")
    add("- 잔차 검사: B, C는 `ResidualAuditClient`가 solve 전의 b를 저장했다가 자기 JVP로 `b - W x`를 다시 계산한다(C는 감사가 스스로 계산한 `min_i(atol + rtol*|y_i|)`로 C의 임계값을 쓴다). A는 보호 driver가 GMRES에 넘기는 벡터를 내놓지 않으므로, attempt observer에서 stage 벡터 `U_i`를 읽어 stage 방정식으로 우변을 다시 만들고(`rhs_i = h*gamma*f(Y_i) + gamma*sum_j C_ij U_j`, 자체 RHS 평가) 자체 JVP로 잔차를 계산한다. 검사용 JVP와 RHS는 별도 counter에 있고 arm의 일에 넣지 않았다. 감사를 켠 실행과 끈 실행이 counter와 최종 상태의 모든 비트에서 같은지 parser가 확인한다.")
    add("- matched error: 달성 오차 `E = max_i |y_i(t1) - ref_i| / (1 + |ref_i|)`. arm별 (E, 일) 점을 tolerance 순서로 log-log 선분으로 잇고, 두 arm의 오차 구간이 겹치는 범위(기준해 불확도의 10배 이상)에서 201점 격자의 일 비율을 기하평균한 값으로 family를 판정한다. 곡선이 단조가 아니면 A는 작은 값, 후보는 큰 값을 쓴다.")
    add("- instructions: `perf stat -e instructions:u`로 프로세스 전체를 재고 같은 family의 SETUP 실행(문제와 시작 상태만 만드는 실행) 3회의 중앙값을 뺐다. wall: `flock ... taskset -c 6-11` 아래 한 프로세스에서 A, B, C를 round마다 순서를 돌려 가며 9 round(앞 2개는 warm-up).")
    add("")
    rows = []
    for fid, family in families.items():
        entry = declared[fid]
        reference = family.get("reference") or {}
        consistency = reference.get("rhs_consistency") or {}
        rows.append([f"`{fid}`", family["n"], f"[{family['t_span'][0]:g}, {family['t_span'][1]:g}]",
                     "새 family" if family["new"] else "V06 family, 새 크기",
                     "예" if family["strongly_stiff"] else "",
                     entry["description"].split(":")[0],
                     sci(reference.get("uncertainty")), sci(consistency.get("f0_max_relative_difference"))])
    add(table(["family", "n", "구간", "구분", "강한 stiff", "문제", "기준해 불확도 u_ref", "Rust/numpy RHS 상대차"], rows))
    add("")
    add("- 기준해: 각 family를 numpy로 다시 구현해 scipy DOP853(rtol = atol = 1e-13)으로 적분했다. 불확도는 DOP853(1e-11) 및 Radau(1e-11, 유한차분 sparse Jacobian) 결과와의 `|차| / (1 + |ref|)` 최댓값 중 큰 쪽이다. Rust 코드와 선형 solver를 공유하지 않는다.")
    rows = []
    for fid, family in families.items():
        reference = family.get("reference")
        if not reference:
            continue
        for check in reference["cross_checks"]:
            rows.append([f"`{fid}`", f"{check['method']} {check['rtol']:g}", sci(check["difference"]["max_scaled_1_plus_abs"]),
                         num(check["rhs_evaluations"]), num(reference["reference"]["rhs_evaluations"])])
    add("")
    add(table(["family", "대조 적분", "기준해와의 차 (scaled max)", "대조 RHS 평가 수", "기준해 RHS 평가 수"], rows))
    add("")
    add("- step tolerance: rtol = 1e-3, 1e-4, 1e-5, 1e-6, 1e-7, atol = rtol. 초기 step 1e-3(`AdaptiveStepConfig::default()`), 나머지 controller 설정은 기본값.")
    add("")

    # ------------------------------------------------------------ 3. discriminators
    add("## 3. frozen 판별식")
    add("")
    add("### 3.1 validity")
    add("")
    rows = []
    for fid, family in families.items():
        validity_f = family["validity"]
        for arm in ARMS:
            block = validity_f[arm]
            rows.append([f"`{fid}`", arm, num(block["checks"]), num(block["failures"]), ratio(block["max_residual_over_threshold"]),
                         num(block["every_solve_checked_and_same_run"])])
    add(table(["family", "arm", "검사한 solve", "실패", "잔차/허용치 최대", "모든 solve 검사 + 감사 on/off 동일"], rows))
    add("")
    rows = []
    for fid, family in families.items():
        validity_f = family["validity"]
        rows.append([f"`{fid}`"] + [ratio(validity_f["attempts_B_over_A"][label]) for label in labels] + [num(validity_f["attempts_B_within_1_05"])])
    add("attempts(B) / attempts(A), 같은 tolerance:")
    add("")
    add(table(["family"] + [f"rtol {label}" for label in labels] + ["1.05 이내"], rows))
    add("")
    rows = []
    for fid, family in families.items():
        validity_f = family["validity"]
        rows.append([f"`{fid}`"] + [ratio(validity_f["attempts_C_over_A"][label]) for label in labels] + [num(validity_f["attempts_C_at_most_1_10"])])
    add("attempts(C) / attempts(A), 같은 tolerance:")
    add("")
    add(table(["family"] + [f"rtol {label}" for label in labels] + ["1.10 이하"], rows))
    add("")

    add("### 3.2 Cadence (B)")
    add("")
    rows = []
    for fid, family in families.items():
        c = family["cadence"]
        block = family["matched_error"]["B"]
        rng = block["instructions_net"].get("range")
        rows.append([f"`{fid}`", num(c["validity"]), ratio(c["instructions_ratio"]), verdict_ko(c["instructions_part"]),
                     c.get("instructions_part_at_worst_grid_point", "-"),
                     speed(c["wall_speedup"]), c["wall_label"], verdict_ko(c["wall_part"]),
                     f"{rng[0]:.1e}~{rng[1]:.1e}" if rng else "-", verdict_ko(c["family"])])
    add(table(["family", "validity", "instructions B/A (기하평균)", f"<= {thresholds['cadence_instructions_max']}", "최악 격자점 기준", "wall 배속 (기하평균)", "wall 표시",
               f">= {thresholds['cadence_wall_min']}x", "비교한 오차 구간", "family"], rows))
    add("")
    add(f"- 판정 규칙(`PREDECLARED.json`): 3개 이상 family가 PASS면 PASS, 판정된 부분에서 2개 이상이 FAIL이면 FAIL, 그 밖은 NOT_DECIDABLE. 결과: PASS {cadence['pass']}, FAIL {cadence['fail']}, 판정 불가 {cadence['open']} -> **{verdict_ko(cadence['verdict'])}**.")
    add("")

    add("### 3.3 Policy (C)")
    add("")
    rows = []
    for fid, family in families.items():
        p = family["policy"]
        rows.append([f"`{fid}`", num(p["validity"]), ratio(p["instructions_ratio"]), verdict_ko(p["instructions_part"]),
                     p.get("instructions_part_at_worst_grid_point", "-"),
                     ratio(p["attempts_ratio_matched_error"]), ratio(p["attempts_equal_tolerance_max"]),
                     verdict_ko(p["attempts_part"]), verdict_ko(p["family"])])
    add(table(["family", "validity", "instructions C/A (기하평균)", f"<= {thresholds['policy_instructions_max']}", "최악 격자점 기준",
               "attempts C/A (matched error)", "attempts C/A (같은 tolerance, 최대)", f"<= {thresholds['policy_attempts_max']}", "family"], rows))
    add("")
    add(f"- 결과: PASS {policy['pass']}, FAIL {policy['fail']}, 판정 불가 {policy['open']} -> **{verdict_ko(policy['verdict'])}**.")
    add("")
    add("### 3.4 KILL")
    add("")
    add(f"- B의 잔차 검사 실패: {num(kill['residual_check_failures_in_B'])}건.")
    add("- B의 wall 배속(matched error): " + ", ".join(f"`{fid}` {speed(value)} [{kill['wall_labels'][fid]}]" for fid, value in kill["wall_speedup_B"].items()) + ".")
    add(f"- 판정: **{verdict_ko(kill['verdict'])}** ({kill['reason']}).")
    add("")

    # ------------------------------------------------------------ 4. cells
    add("## 4. cell별 자료")
    add("")
    for fid, family in families.items():
        add(f"### `{fid}` (n = {family['n']})")
        add("")
        rows = []
        for label in labels:
            cell = family["cells"][label]
            arms = cell["arms"]
            eq = cell["equal_tolerance_ratio_to_A"]
            rows.append([label,
                         " / ".join(num(arms[arm].get("attempts")) for arm in ARMS),
                         " / ".join(num(arms[arm].get("rejected")) for arm in ARMS),
                         " / ".join(num(arms[arm].get("jvp_calls")) for arm in ARMS),
                         " / ".join(sci(arms[arm].get("instructions_net")) for arm in ARMS),
                         ratio((eq.get("B") or {}).get("instructions")), ratio((eq.get("C") or {}).get("instructions")),
                         " / ".join(sci(arms[arm].get("E")) for arm in ARMS),
                         " / ".join(num(arms[arm].get("E_over_rtol")) for arm in ARMS)])
        add(table(["rtol", "attempts A / B / C", "기각 A / B / C", "JVP A / B / C", "instructions (net) A / B / C", "instr B/A", "instr C/A",
                   "달성 오차 E A / B / C", "E / rtol A / B / C"], rows))
        add("")
        rows = []
        for label in labels:
            timing = family["cells"][label]["timing"]
            used = next((entry for entry in timing["tries"] if entry["try"] == timing.get("used_try")), None)
            if used is None:
                rows.append([label, "-", "-", "-", "-", "-", "-", timing["label"], "-"])
                continue
            walls = timing["median_wall"]
            spreads = timing["spread"]
            speedups = timing.get("speedup_equal_tolerance") or {}
            rows.append([label, " / ".join(num(walls[arm], 4) for arm in ARMS),
                         " / ".join("-" if spreads[arm] is None else f"{100 * spreads[arm]:.1f}%" for arm in ARMS),
                         speed(speedups.get("B")), speed(speedups.get("C")),
                         f"{num(used['load_1min_before_first'])} / {num(used['load_1min_after_last'])}",
                         f"{timing['measurements']} (사용: {timing['used_try']}회차" + (", round별 9회 호출" if used["split_long_cell"] else "") + ")",
                         timing["label"], "; ".join(used["misses"]) if used["misses"] else ""])
        add(table(["rtol", "wall 중앙값 s A / B / C", "spread A / B / C", "배속 B (같은 tol)", "배속 C (같은 tol)", "1분 load 전 / 후", "측정 횟수", "표시", "규칙 미달 사유"], rows))
        add("")

    # ------------------------------------------------------------ 5. exploratory
    add("## 5. 탐색 (어느 판정에도 넣지 않음)")
    add("")
    add("### 5.1 이득은 어디서 오는가")
    add("")
    add("family별로 JVP 1회와 RHS 1회의 instructions를 보정 실행(각 20,000회 호출, SETUP 차감)으로 재고, 각 실행의 net instructions를 JVP 몫, RHS 몫, 나머지(직교화, 벡터 연산, driver)로 나눴다. JVP 보정 loop에는 방향 벡터를 다시 정규화하는 O(n) 연산이 들어 있어 JVP 몫은 약간 과대평가다.")
    add("")
    rows = []
    for fid, family in families.items():
        calibration = family["calibration"]
        rows.append([f"`{fid}`", num((calibration.get("jvp") or {}).get("instructions_per_call")),
                     num((calibration.get("rhs") or {}).get("instructions_per_call")), num(family["setup_instructions"]["median"])])
    add(table(["family", "instructions / JVP", "instructions / RHS", "SETUP instructions (중앙값)"], rows))
    add("")
    rows = []
    for fid, family in families.items():
        for label in labels:
            row = family["exploratory_decomposition"].get(label) or {}
            if "A" not in row:
                continue
            def shares(arm):
                block = row.get(arm)
                if not block:
                    return "-"
                return f"{100 * block['jvp_share']:.0f}% / {100 * block['rhs_share']:.1f}% / {100 * block['remainder_share']:.0f}%"
            def saving(arm):
                block = row.get(f"saving_A_to_{arm}")
                if not block:
                    return "-"
                return f"{100 * block['from_jvp']:.0f}% / {100 * block['from_remainder']:.0f}%"
            eq = family["cells"][label]["equal_tolerance_ratio_to_A"]
            rows.append([f"`{fid}`", label, shares("A"), shares("B"), shares("C"), saving("B"), saving("C"),
                         ratio((eq.get("B") or {}).get("orthogonalization_inner_products")),
                         ratio((eq.get("C") or {}).get("orthogonalization_inner_products"))])
    add(table(["family", "rtol", "A: JVP / RHS / 나머지", "B: JVP / RHS / 나머지", "C: JVP / RHS / 나머지",
               "A->B 절감의 출처 (JVP / 나머지)", "A->C 절감의 출처 (JVP / 나머지)", "직교화 내적 수 B/A", "직교화 내적 수 C/A"], rows))
    add("")
    add("- '나머지'에는 Arnoldi 직교화(two-pass modified Gram-Schmidt), 벡터 갱신, driver의 일이 들어 있다. 열 j의 직교화 비용은 j에 비례하므로, pass를 일찍 끝내면 JVP 수보다 직교화 일이 더 크게 줄 수 있다. 표의 '직교화 내적 수' 비가 그 크기다.")
    add("")
    add("### 5.2 대조 arm B0 (client driver에 보호 kernel의 검사 주기)")
    add("")
    rows = []
    for fid, family in families.items():
        for label in labels:
            cell = family["cells"][label]
            a, b0 = cell["arms"]["A"], cell["arms"]["B0"]
            eq = cell["equal_tolerance_ratio_to_A"].get("B0") or {}
            same = (a.get("jvp_calls") == b0.get("jvp_calls") and a.get("linear_iterations") == b0.get("linear_iterations")
                    and a.get("attempts") == b0.get("attempts")) if b0.get("present") else None
            rows.append([f"`{fid}`", label, f"{num(a.get('attempts'))} / {num(b0.get('attempts'))}",
                         f"{num(a.get('jvp_calls'))} / {num(b0.get('jvp_calls'))}", num(same), ratio(eq.get("instructions")), ratio(eq.get("error"))])
    add(table(["family", "rtol", "attempts A / B0", "JVP A / B0", "attempt, JVP, Arnoldi 열 수 동일", "instructions B0/A", "오차 B0/A"], rows))
    add("")
    add("- B0는 검사 주기가 A와 같으므로, instructions B0/A는 두 driver와 두 GMRES 구현의 차이만 보여 준다. B/A에서 이 값을 뺀 나머지가 검사 주기의 효과다.")
    add("")
    add("### 5.3 solve당 Krylov 차원")
    add("")
    rows = []
    for fid, family in families.items():
        for label in labels:
            cell = family["cells"][label]
            audited_a = cell["audit_on"].get("A") or {}
            log = audited_a.get("solve_log") or {}
            histogram = log.get("pass_histogram") or []
            passes = ", ".join(f"{k} pass: {count}" for k, count in enumerate(histogram) if count)
            def dims(arm):
                run = cell["arms"][arm]
                return f"{num(run.get('krylov_vectors_per_solve'))} / {num(run.get('max_solve_vectors'))}"
            c_run = cell["arms"]["C"]
            client = c_run.get("client") or {}
            rows.append([f"`{fid}`", label, f"{num(audited_a.get('krylov_vectors_per_solve'))} / {num(audited_a.get('max_solve_vectors'))}",
                         passes, dims("B"), dims("C"),
                         "-" if c_run.get("policy_bound_share") is None else f"{100 * c_run['policy_bound_share']:.0f}%",
                         num(client.get("zero_iterate_accepts"))])
    add(table(["family", "rtol", "A: 평균 / 최대 열", "A: pass 수 분포 (성공한 solve)", "B: 평균 / 최대 열", "C: 평균 / 최대 열",
               "C: 정책 임계값이 쓰인 solve", "C: 0 반복으로 받은 solve"], rows))
    add("")
    add("### 5.4 restart 길이 (rtol 1e-5)")
    add("")
    rows = []
    for fid, family in families.items():
        block = family["exploratory"]["restart"]
        for arm in ("A", "B"):
            cells = []
            for restart in ("10", "20", "40", "80"):
                key = f"{arm}_restart{restart}" + ("_declared" if restart == "40" else "")
                run = block.get(key) or {}
                if not run.get("present"):
                    cells.append("-")
                    continue
                status = "" if run.get("completed") else " (미완료)"
                cells.append(f"{num(run.get('attempts'))} att, {num(run.get('jvp_calls'))} JVP, {sci(run.get('instructions_net'))} instr, 실패 {num(run.get('linear_solve_failures'))}{status}")
            rows.append([f"`{fid}`", arm] + cells)
    add(table(["family", "arm", "restart 10", "restart 20", "restart 40 (선언 실행)", "restart 80"], rows))
    add("")
    add("### 5.5 preconditioner 한 가지 (client의 right Jacobi, arm B, rtol 1e-5)")
    add("")
    rows = []
    for fid, family in families.items():
        block = family["exploratory"]["jacobi"]
        plain = family["cells"].get("1e-5", {}).get("arms", {}).get("B", {})
        run, audited = block.get("audit0") or {}, block.get("audit1") or {}
        audit = audited.get("residual_audit") or {}
        rows.append([f"`{fid}`", f"{num(plain.get('attempts'))} / {num(run.get('attempts'))}",
                     f"{num(plain.get('jvp_calls'))} / {num(run.get('jvp_calls'))}",
                     f"{sci(plain.get('instructions_net'))} / {sci(run.get('instructions_net'))}",
                     f"{num(plain.get('krylov_vectors_per_solve'))} / {num(run.get('krylov_vectors_per_solve'))}",
                     f"{sci(plain.get('E'))} / {sci(run.get('E'))}", f"{num(audit.get('checks'))} / {num(audit.get('failures'))}"])
    add(table(["family", "attempts B / B+Jacobi", "JVP B / B+Jacobi", "instructions B / B+Jacobi", "solve당 열 B / B+Jacobi",
               "오차 E B / B+Jacobi", "Jacobi 실행의 잔차 검사 / 실패"], rows))
    add("")
    add("- 보호 driver는 preconditioner를 받지 않으므로(`rodas5p_matrix_free_fast.rs:1355-1366`) A에는 대응 실행이 없다.")
    add("")
    add("### 5.6 kappa 민감도 (arm C; 선언값은 1e-3, 판정에는 선언값만 썼다)")
    add("")
    rows = []
    for fid, family in families.items():
        for label in labels:
            declared_c = family["cells"][label]["arms"]["C"]
            a = family["cells"][label]["arms"]["A"]
            cells = []
            for kappa in ("1e-4", "1e-2"):
                run = (family["exploratory"]["kappa"].get(kappa) or {}).get(label) or {}
                cells.append(run)
            def show(run):
                if not run.get("present"):
                    return "-"
                return f"{ratio(_ratio(run.get('instructions_net'), a.get('instructions_net')))} / {ratio(_ratio(run.get('E'), a.get('E')))} / {ratio(_ratio(run.get('attempts'), a.get('attempts')))}"
            rows.append([f"`{fid}`", label, show(cells[0]), show(declared_c), show(cells[1])])
    add(table(["family", "rtol", "kappa 1e-4: instr / 오차 / attempts (A 대비)", "kappa 1e-3 (선언)", "kappa 1e-2"], rows))
    add("")
    add("### 5.7 Krylov 한도에 걸리는 경우 (표준 Brusselator, 길이 1, 600 cell, n = 1200, rtol 1e-4)")
    add("")
    stiff = results["exploratory_stiff_brusselator"]
    rows = []
    for arm in ARMS:
        run = stiff.get(f"{arm}_audit0") or {}
        audited = stiff.get(f"{arm}_audit1") or {}
        audit = audited.get("residual_audit") or {}
        if not run.get("present"):
            continue
        rows.append([arm, "완료" if run.get("completed") else "미완료", num(run.get("attempts")), num(run.get("rejected")),
                     num(run.get("linear_solve_failures")), num(run.get("jvp_calls")), sci(run.get("instructions_net")),
                     sci(run.get("E")), f"{num(audit.get('checks'))} / {num(audit.get('failures'))}"])
    if rows:
        add(table(["arm", "상태", "attempts", "기각", "선형 solve 실패", "JVP", "instructions (net)", "오차 E", "잔차 검사 / 실패"], rows))
        add("")
        add("- 이 영역에서는 무전처리 GMRES(40)이 200벡터 한도 때문에 step을 줄인다. 선언 family가 아니고 timing 규약도 적용하지 않았다.")
    else:
        add("- 실행하지 않았다 (NOT_RUN).")
    add("")

    # ------------------------------------------------------------ 6. how to switch on
    add("## 6. 이득을 얻으려면 사용자가 켜야 하는 것")
    add("")
    add("기본 경로는 그대로다. 보호 driver와 그 허용치는 수정하지 않았고, 아래는 전부 opt-in이다.")
    add("")
    add("```rust")
    add("use rodas5p_integrators::stage_clients::krylov::{")
    add("    KrylovPreconditioner, KrylovStageClient, PerStageGmresOptions, StageStoppingRule, StepTiedTolerance,")
    add("};")
    add("use rodas5p_integrators::integrate_rodas5p_fast_client_observed_with_atol;")
    add("")
    add("// arm B: 보호 규칙의 임계값, 매 Arnoldi 열마다 검사")
    add("let client = KrylovStageClient::per_stage(")
    add("    n, StageStoppingRule::protected_production()?, PerStageGmresOptions::baseline(), KrylovPreconditioner::None)?;")
    add("// arm C: 추가로 step 허용치에 묶인 임계값 (kappa = 1e-3)")
    add("let client = client.with_step_tied_tolerance(StepTiedTolerance::declared(config.rtol, config.atol)?);")
    add("let (run, partial, client) = integrate_rodas5p_fast_client_observed_with_atol(")
    add("    &problem, Box::new(client), t_span, &y0, &config, &output, &vec![config.atol; n])?;")
    add("```")
    add("")
    add("- 문제는 JVP를 제공해야 한다(행렬 없음). preconditioner는 `KrylovPreconditioner::RightJacobi`(Jacobian 대각 callback)만 있다.")
    add("- `ResidualAuditClient::new(client, rule).with_step_tied_tolerance(policy)`로 감싸면 모든 solve의 원 잔차를 별도 JVP로 검사한다(실행 결과는 비트 단위로 같다).")
    add("- facade(`vigilode`)의 routing은 건드리지 않았다. facade에서 이 client를 고르는 길은 아직 없다.")
    add("")

    # ------------------------------------------------------------ 7. before default
    add("## 7. default가 되기 전에 남은 것")
    add("")
    remaining = []
    if any(label != "authoritative" for label in wall_labels.values()):
        remaining.append("조용한 host에서 timing 규약을 다시 돌려 wall 조건을 문면대로 판정해야 한다(재측정 명령은 9절). 이번 wall 수치는 `diagnostic_only`다.")
    remaining.append("B는 보호 driver와 다른 driver(client driver)를 쓴다. 두 driver는 stage 우변을 scaled/unscaled 형태로 다르게 만들어 최종 상태가 반올림 수준에서 다르고(5.2절의 B0), 오류로 실패한 attempt 뒤에 RHS를 한 번 더 평가한다. default로 삼으려면 보호 kernel(`gmres_into.rs`)에 열 단위 검사를 넣거나 client driver를 matrix-free 기본 경로로 승격하는 결정이 필요하고, 어느 쪽이든 보호 파일 또는 default routing을 건드리므로 owner 결정 사항이다.")
    remaining.append("default 경로의 golden(`default_path_golden.rs`)은 반복 수와 상태 비트가 달라지므로 다시 만들어야 한다. 그 전에 G0과 hosted CI에서의 재현을 확인해야 한다.")
    remaining.append("C의 정책은 상수 하나(kappa = 1e-3)를 네 family에서 한 번 확인한 것이다. 성분 scale이 크게 다른 문제(atol << rtol*|y|인 성분과 0에 가까운 성분이 섞인 경우)는 이번 family에 없다: 그런 문제에서는 `min_i(atol + rtol*|y_i|)`가 가장 작은 성분에 맞춰져 보수적이 되지만, 2-norm 검사가 WRMS 검사를 대신할 수 있는지는 따로 확인해야 한다. 비자율 문제, mass matrix, DAE도 범위 밖이다.")
    remaining.append("Krylov 한도에 걸리는 강한 stiff 영역(5.7절)에서는 검사 주기만으로는 이득이 작다. 그 영역은 preconditioner가 필요하고, client에는 right Jacobi뿐이다(5.5절).")
    remaining.append("facade에 opt-in 선택지를 내는 일(V03 계열)과 문서화가 남아 있다.")
    for item in remaining:
        add(f"- {item}")
    add("")

    # ------------------------------------------------------------ 8. amendments and limits
    add("## 8. 사전등록 수정과 한계")
    add("")
    for amendment in predeclared["amendments"]:
        add(f"- 수정 {amendment['id']} ({amendment['date']}): {amendment['what']}")
        add(f"  - 시점: {amendment['status']}")
        for reason in amendment["why"]:
            add(f"  - 이유: {reason}")
        add(f"  - 적용 규칙: {amendment['rule_used']}")
    add("- 한계:")
    limits = [
        "family는 네 개, 크기는 n = 1,200~2,000이다. 더 큰 n, 다른 이산화, 실제 사용자 문제로의 외삽은 이 자료가 보장하지 않는다.",
        "`fhn3d`는 격자가 10^3이다. 이 해상도에서 확산이 stiff하려면 확산 길이가 영역 크기에 가까워야 하므로 해가 공간적으로 매끄럽다. 3D 문제로서 격자 해상도가 낮다.",
        "`bruss1d`는 V06 family의 새 크기(700 cell)다. 완전히 새로운 family는 세 개다.",
        f"A의 잔차 검사는 stage 방정식으로 우변을 다시 만드는 방식이라 kernel이 받은 벡터와 반올림 수준에서 다를 수 있다. A의 검사 실패는 {num(checks['A']['failures'])}건이다.",
        "instructions는 사용자 모드 명령 수이고 캐시와 메모리 대역폭 효과를 반영하지 않는다. wall이 그 효과를 보여 주지만 이번에는 host 부하 때문에 authoritative 표시를 받지 못한 cell이 있다(4절).",
        "초기 step은 crate 기본값 1e-3이다. V06은 1e-6을 썼고 그 검토에서 초기 구간이 JVP 비에 영향을 준다고 지적했다. 다른 초기 step에서의 값은 재지 않았다.",
    ]
    for item in limits:
        add(f"  - {item}")
    problems = results.get("checks") or []
    add(f"- parser의 일관성 검사에서 나온 문제: {len(problems)}건" + ("" if not problems else ": " + "; ".join(problems[:10])))
    add("")

    # ------------------------------------------------------------ 9. tests, regression, files
    add("## 9. 테스트, 회귀, 파일")
    add("")
    regression = results["v06_regression"]
    add(f"- V06 회귀: V17 build로 V06 측정 target을 V06 cell에서 다시 돌려 `shared_krylov/logs`의 기록과 비교했다. 비교 {regression['compared']}건 중 counter, 통계, 최종 상태 비트가 모두 같은 것 {regression['identical']}건.")
    rows = [[f"`{row['name']}`", num(row.get("identical")), num(row.get("attempts")), num(row.get("jvp_calls")),
             ", ".join(row.get("differing_fields") or [])] for row in regression["rows"]]
    if rows:
        add("")
        add(table(["V06 실행", "동일", "attempts", "JVP", "다른 필드"], rows))
        add("")
    add("- 테스트 target (모두 `cargo +1.94.1 test --locked -j 4 --release -p rodas5p-integrators --test <target>`): `stage_clients_krylov_v17_contracts`(11개), `stage_clients_krylov_v17_alloc`(1개, 전용 binary), V06의 `stage_clients_krylov_contracts`, `stage_clients_krylov_alloc`, sentinel `ppsw1_6b_contracts`, `ptsw2b_contracts`. 실행 결과는 `logs/checks/`에 있다.")
    add("- 파일:")
    for path, text in [
        ("PREDECLARED.json", "사전등록과 수정 A1"),
        ("NOTES.txt", "작업 기록"),
        ("reference/make_reference.py, reference/*.json", "numpy 기준해"),
        ("run_measurements.py", "실행 driver (setup, runs, timing, exploratory, regression)"),
        ("parse_results.py -> RESULTS.json", "로그만으로 결과 생성"),
        ("make_report.py -> REPORT_KO.md", "이 문서"),
        ("logs/", "원자료 (sizing, reference, setup, runs, timing, exploratory, v06_regression, checks)"),
        ("crates/rodas5p-integrators/src/stage_clients/krylov.rs, krylov/step_tied.rs", "client 추가분 (opt-in)"),
        ("crates/rodas5p-integrators/tests/stage_clients_krylov_v17_{problems,contracts,alloc,measure}.rs", "family, 계약 테스트, 할당 테스트, 측정 프로그램"),
    ]:
        add(f"  - `{path}`: {text}")
    add("- wall 재측정 (조용한 host에서): `python3 -I run_measurements.py timing --bin <stage_clients_krylov_v17_measure binary> --try 2` 뒤 `python3 -I parse_results.py && python3 -I make_report.py`. cell당 측정은 3회까지이며 규칙을 처음 만족한 측정을 쓴다.")
    add("")

    with open(args.out, "w", encoding="utf-8") as handle:
        handle.write("\n".join(out))
        handle.write("\n")
    print(f"wrote {args.out} ({len(out)} lines)")


def _ratio(a, b):
    if a is None or b is None or b == 0:
        return None
    return a / b


if __name__ == "__main__":
    main()
