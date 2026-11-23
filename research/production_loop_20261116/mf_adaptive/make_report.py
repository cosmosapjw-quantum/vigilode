#!/usr/bin/env python3
"""Loop node V24: writes REPORT_KO.md from RESULTS.json, CHOICE.json,
RECORDING_A.json, SENTINEL_REPIN.json and R_SUITES.json. Every number of the
report is read from those files; none is typed in here.

usage: python3 -I make_report.py [--out REPORT_KO.md]
"""
import argparse
import json
import os
import sys

sys.dont_write_bytecode = True
NODE = os.path.dirname(os.path.abspath(__file__))
LABELS = ["1e-3", "1e-4", "1e-5", "1e-6", "1e-7"]
FAMILIES = ["fhn3d", "cusp1d", "burgers2d", "bruss1d"]
FAMILY_TEXT = {
    "fhn3d": "fhn3d (3차원 FitzHugh-Nagumo, n = 2000)",
    "cusp1d": "cusp1d (CUSP 고리, n = 1200, 강한 경직)",
    "burgers2d": "burgers2d (2차원 점성 Burgers, n = 1568)",
    "bruss1d": "bruss1d (1차원 Brusselator 700칸, n = 1400)",
}


def load(name):
    path = os.path.join(NODE, name)
    if not os.path.exists(path):
        return None
    with open(path, encoding="utf-8") as handle:
        return json.load(handle)


def f3(value):
    return "-" if value is None else f"{value:.3f}"


def f2(value):
    return "-" if value is None else f"{value:.2f}"


def sci(value):
    return "-" if value is None else f"{value:.3e}"


def whole(value):
    return "-" if value is None else f"{int(value):,}"


def verdict_text(value):
    return {"PASS": "성립(PASS)", "FAIL": "불성립(FAIL)", "NOT_RUN": "실행하지 않음(NOT_RUN)",
            "NOT_DECIDABLE": "판정 불가(NOT_DECIDABLE)", "reported only": "보고만 함"}.get(value, str(value))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", default=os.path.join(NODE, "REPORT_KO.md"))
    args = parser.parse_args()
    results = load("RESULTS.json")
    choice = load("CHOICE.json")
    recording = load("RECORDING_A.json")
    repin = load("SENTINEL_REPIN.json")
    suites = load("R_SUITES.json")
    checker = load("CHECKER_REPORT.json")
    disc = results["discriminators"]
    verdict = results["verdict"]
    arm = results["chosen_variant"]
    families = results["families"]
    out = []
    w = out.append

    w("# V24: 행렬 없는(matrix-free) 단계 풀이의 재시작 길이와 잔차 검사를 풀이에 맞추기")
    w("")
    w(f"- 노드: `V24_MF_ADAPTIVE_STAGE_SOLVE` (소유자 결정 OD-L1-11, 사전등록 부록 A6, `mf_adaptive/PREDECLARED.json`)")
    w(f"- 이 문서의 모든 숫자는 `RESULTS.json`, `CHOICE.json`, `RECORDING_A.json`, `R_SUITES.json`에서 스크립트(`make_report.py`)가 읽어 넣은 것입니다. 생성 시각: {results['generated']}")
    w(f"- 비교한 두 팔: **S** = 지금 출하된 정책 그대로, **N** = 선택된 적응형 변형 **{arm}**")
    w("")

    # ------------------------------------------------------------------ 1
    w("## 1. 결론")
    w("")
    six = {name: disc[name]["verdict"] for name in ("D0", "V", "A", "P", "S", "B")}
    not_pass = [name for name in ("D0", "V", "A", "P", "S", "B", "G", "R") if disc[name]["verdict"] != "PASS"]
    reasons = {
        "FAIL": "P, S, A, B 가운데 " + ", ".join(name for name in ("P", "S", "A", "B") if disc[name]["verdict"] == "FAIL") + "가 불성립이고 V는 온전함 (동결 규칙: \"P, S, A or B fails with V intact\")",
        "KILL": "V 위반 또는 두 계열에서 P가 0.85 초과 (동결 규칙의 KILL)",
        "PASS": "D0, V, A, P, S, B, G, R 모두 성립",
        "NOT_PASS": "PASS가 아닌 판별식: " + ", ".join(f"{name} {disc[name]['verdict']}" for name in not_pass),
    }
    w(f"**동결된 판정 규칙에 따른 결과: {verdict['verdict']}.** 사유: {reasons.get(verdict['verdict'], verdict.get('reason'))}.")
    w("")
    b = disc["B"]
    failing_b = []
    for family in FAMILIES:
        cells = families[family]["discriminators"]["B"]["cells"]
        for label in LABELS:
            if cells[label]["verdict"] == "FAIL":
                failing_b.append((family, label, cells[label]["S"], cells[label]["N"]))
    if verdict["verdict"] == "FAIL":
        w("- 적응형 정책은 **선택 사양(opt-in)으로만 남기고, matrix-free 경로의 기본값은 바꾸지 않았습니다.** 동결 규칙이 FAIL일 때 정한 그대로입니다. 5단계(기본값 전환, golden 기록 재고정)는 수행하지 않았습니다.")
        for family, label, s, n in failing_b:
            w(f"- 불성립한 판별식은 B 하나이고, 20칸 중 한 칸입니다: **{family}, rtol {label}에서 Krylov 예산(200열)을 다 쓰고 실패한 단계 풀이가 N {n}건, S {s}건.** B는 \"어느 칸에서도 N이 S보다 많으면 안 된다\"는 조건입니다.")
    w(f"- 성립한 것: " + ", ".join(f"{name} {verdict_text(value)}" for name, value in six.items() if value == "PASS") + ".")
    if suites and suites["verdict"] != "PASS":
        w(f"- R이 불성립인 까닭은 시험 하나입니다: {'; '.join(suites['summary']['failing_tests'])}. 이 센티널은 편집한 드라이버 파일의 해시를 고정하는데, 이 노드의 쓰기 허용 목록 밖에 있어 다시 고정할 수 없었습니다(10절). 그 밖의 시험 {whole(suites['summary']['tests_passed'])}개는 모두 통과했습니다.")
    explored = results.get("exploratory") or {}
    if "P1" in explored:
        w("- 탐색적 관측(판정에 쓰지 않음): 선택되지 않은 변형 P1은 결정 격자에서 예산 소진 풀이 수가 모든 칸에서 S와 같았고, 같은 선형계를 다시 푼 재현 분석에서도 출하 커널과 차이가 없었습니다. P2는 같은 선형계에서도 예산 경계의 계를 조금 더 놓쳤습니다(5절, 7절, 8-1절).")
    w("")
    w("| 판별식 | 동결 기준 | 측정값 | 결과 |")
    w("|---|---|---|---|")
    d0 = results["D0"]
    w(f"| D0 출하 정책 불변 | 편집 뒤의 S가 편집 전 드라이버와 비트 단위로 같음(끝 상태, 스텝 수, 모든 카운터) | 결정 격자 {d0['deciding_cells']['equal']}/{d0['deciding_cells']['compared']}칸, 개발 칸 {d0['development_cells']['equal']}/{d0['development_cells']['compared']}칸 동일 (진입점 3개 + 정책을 명시한 진입점) | {verdict_text(disc['D0']['verdict'])} |")
    v = disc["V"]
    w(f"| V 유효성 | N의 모든 단계 풀이에서 돌려준 해의 참 잔차가 출하 임계값 이하, 위반 0 | 결정 격자에서 완료된 풀이 {whole(v['deciding_grid']['completed_solves'])}건 전부 독립 감사, 위반 {v['violations']}건, 잔차/임계값 최댓값 {v['max_residual_over_threshold']:.6f}; 실패로 보고된 풀이 {whole(v['deciding_grid']['failed_solves'])}건 | {verdict_text(v['verdict'])} |")
    a = disc["A"]
    w(f"| A 정확도 | 모든 칸에서 N의 달성 오차 <= S의 3배 | 계열별 최대 오차비 " + ", ".join(f"{a['max_error_ratios'][family]:.3f}" for family in FAMILIES) + f" | {verdict_text(a['verdict'])} |")
    p = disc["P"]
    w(f"| P 명령어 수 | 정합 오차에서 계열별 N/S <= 0.50, 어느 칸도 1.00 초과 없음 | 계열별 " + ", ".join(f"{p['family_numbers'][family]:.3f}" for family in FAMILIES) + "; 칸 최댓값 " + ", ".join(f"{p['max_cell_numbers'][family]:.3f}" for family in FAMILIES) + f" | {verdict_text(p['verdict'])} |")
    s = disc["S"]
    w(f"| S 시도 횟수 | 계열별 기하평균 <= 1.05, 어느 칸도 1.15 초과 없음 | 기하평균 " + ", ".join(f"{s['geometric_means'][family]:.4f}" for family in FAMILIES) + "; 칸 최댓값 " + ", ".join(f"{s['max_ratios'][family]:.4f}" for family in FAMILIES) + f" | {verdict_text(s['verdict'])} |")
    w(f"| B 예산 소진 | 어느 칸에서도 N의 예산 소진 풀이 수 <= S | 계열 합계 S/N: " + ", ".join(f"{family} {b['budget_exhausted'][family]['S']}/{b['budget_exhausted'][family]['N']}" for family in FAMILIES) + f" | {verdict_text(b['verdict'])} |")
    t = disc["T"]
    w(f"| T 벽시계 시간 | 보고만 함(판정에 쓰지 않음) | 정합 오차에서 S/N 속도비: " + ", ".join(f"{family} {f2(t['wall_speedup_at_matched_error'].get(family))} ({t['labels'][family]})" for family in FAMILIES) + " | 보고만 함 |")
    w(f"| G golden 기록 | 기본값을 바꿀 때만 해당 | 기본값을 바꾸지 않았으므로 golden 파일은 한 바이트도 바뀌지 않음 | {verdict_text(disc['G']['verdict'])} |")
    if suites:
        rs = suites["summary"]
        w(f"| R 시험 묶음 | 센티널, stage-client, matrix-free 시험, facade 전체, clippy, rustfmt 통과 | 시험 {whole(rs['tests_passed'])}개 통과, {whole(rs['tests_failed'])}개 실패; 0으로 끝나지 않은 명령: {', '.join(rs['not_exit_0']) or '없음'} | {verdict_text(suites['verdict'])} |")
    else:
        w(f"| R 시험 묶음 | 센티널, stage-client, matrix-free 시험, facade 전체, clippy, rustfmt 통과 | - | {verdict_text(disc['R']['verdict'])} |")
    w("")

    # ------------------------------------------------------------------ 2
    w("## 2. 출하된 풀이가 하던 일, 적응형 풀이가 하는 일")
    w("")
    w("RODAS5P의 한 스텝은 같은 행렬 `I - h gamma J`에 대한 선형계를 8번 풉니다. matrix-free 경로는 행렬을 만들지 않고 재시작 GMRES로 풉니다(전처리 없음, 0에서 시작, 풀이당 Arnoldi 벡터 예산 200개).")
    w("")
    w("**출하된 정책 (S).** 한 주기가 항상 40열입니다. 주기 안에서는 잔차를 보지 않고, 40열을 다 만든 뒤에야 해를 만들어 참 잔차 `rhs - (I - h gamma J) U`를 한 번 계산해 임계값 `max(gamma * 1e-14, 1e-10 * ||rhs||)`과 비교합니다. 12열이면 충분했던 풀이도 40열을 다 만듭니다. 한 열의 직교화 비용은 그때까지 만든 열 수에 비례하므로, 필요 없는 뒤쪽 열일수록 비쌉니다.")
    w("")
    w("**적응형 정책 (N).** Arnoldi 열을 하나 만들 때마다 Givens 회전으로 얻는 투영 잔차 `|g_(j+1)|`를 같은 임계값과 비교합니다. 임계값 이하가 된 첫 열에서 주기를 끝내고, 그때 해를 만들어 **참 잔차를 새 연산자 곱 한 번으로 계산해 같은 임계값으로 다시 검사**합니다(출하 드라이버의 원래 검사 그대로). 통과하지 못하면 그 해에서 풀이를 계속합니다. 예산을 다 쓰면 출하 드라이버와 같은 오류로 실패하고, 드라이버는 지금처럼 그 시도를 기각하고 스텝을 줄입니다.")
    w("")
    w("- P1: 주기의 상한이 설정된 재시작 길이(40).")
    w("- P2: 풀이의 첫 주기 상한 10, 수렴하지 못한 주기마다 두 배(10, 20, 40, 40, ...), 최대 40.")
    w("")
    w("바뀌지 않은 것: 임계값(선형 풀이의 허용오차), Krylov 방법(2회 수정 Gram-Schmidt GMRES)과 전처리 없음, 예산 200, 스텝 제어기 상수, 잔차 회계(기본값 `RecomputeFinal`: 마지막 참 잔차를 한 번 더 계산하는 출하 동작을 N도 그대로 지불합니다).")
    w("")

    # ------------------------------------------------------------------ 3
    w("## 3. 네 계열 표")
    w("")
    w("오차는 V17의 기준 해(DOP853, rtol 1e-13)에 대한 `max_i |y_i - ref_i| / (1 + |ref_i|)`이고, 명령어 수는 `perf stat -e instructions:u`에서 같은 바이너리의 SETUP 실행 중앙값을 뺀 값입니다. \"정합 오차\" 열은 V17의 규칙 그대로(오차-일 곡선을 로그 보간, 201점 기하평균, S는 가장 작은 값, N은 가장 큰 값)입니다.")
    w("")
    w("| 계열 | 명령어 N/S (정합 오차, 판정값) | 같은 허용오차 기하평균 | 칸별 N/S (1e-3 ... 1e-7) | JVP 수 N/S | 시도 횟수 N/S | 오차 N/S 최댓값 | 예산 소진 S / N | 감사한 풀이 / 위반 |")
    w("|---|---|---|---|---|---|---|---|---|")
    for family in FAMILIES:
        entry = families[family]
        d = entry["discriminators"]
        comparison = entry["comparison"]
        cells = " ".join(f3(comparison["cells"][label].get("instructions_equal_tolerance")) for label in LABELS)
        w(f"| {FAMILY_TEXT[family]} | **{f3(d['P']['family_number'])}** | {f3(comparison['equal_tolerance']['instructions_geometric_mean'])} | {cells} | {f3(comparison['equal_tolerance']['jvp_geometric_mean'])} | {d['S']['geometric_mean']:.4f} | {d['A']['max_error_ratio']:.3f} | {d['B']['total_S']} / {d['B']['total_N']} | {whole(d['V']['totals']['independent_checks'])} / {d['V']['totals']['violations']} |")
    w("")
    w("읽는 법: 명령어 비 0.32는 같은 정확도에 드는 일이 출하 정책의 32%라는 뜻입니다. 허용오차가 빡빡할수록(스텝이 작아 선형계가 쉬워질수록) 이득이 큽니다. 출하 정책은 그런 쉬운 풀이에도 40열을 다 만들기 때문입니다.")
    w("")

    # ------------------------------------------------------------------ 4
    w("## 4. 칸별 표 (20칸)")
    w("")
    w("| 계열 | rtol | 시도 S / N | 열 수 S / N | 풀이당 열 S / N | 명령어 S | 명령어 N | N/S | 달성 오차 S | 달성 오차 N | 예산 소진 S / N | 투영 종료 뒤 참 잔차 불합격 (N) |")
    w("|---|---|---|---|---|---|---|---|---|---|---|---|")
    for family in FAMILIES:
        entry = families[family]
        for label in LABELS:
            cs, cn = entry["arms"]["S"][label], entry["arms"]["N"][label]
            row = entry["comparison"]["cells"][label]
            bcell = entry["discriminators"]["B"]["cells"][label]
            per_s = cs["linear_iterations"] / max(cs["linear_solves"], 1)
            per_n = cn["linear_iterations"] / max(cn["linear_solves"], 1)
            stat = cn.get("policy_statistics") or {}
            w(f"| {family} | {label} | {cs['attempts']} / {cn['attempts']} | {whole(cs['linear_iterations'])} / {whole(cn['linear_iterations'])} | {per_s:.1f} / {per_n:.1f} | {sci(cs.get('instructions_net'))} | {sci(cn.get('instructions_net'))} | {f3(row.get('instructions_equal_tolerance'))} | {sci(cs.get('E'))} | {sci(cn.get('E'))} | {bcell['S']} / {bcell['N']} | {stat.get('failed_confirmations', '-')} |")
    w("")
    w("\"풀이당 열\"에는 실패한 풀이가 쓴 열도 분자에 들어 있습니다(cusp1d).")
    w("")

    # ------------------------------------------------------------------ 5
    w("## 5. B가 불성립한 칸에서 무슨 일이 있었나")
    w("")
    for family, label, s_count, n_count in failing_b:
        cs, cn = families[family]["arms"]["S"][label], families[family]["arms"]["N"][label]
        w(f"- {family}, rtol {label}: S는 시도 {cs['attempts']}회(기각 {cs['rejected']}) 중 예산 소진 {s_count}건, N은 시도 {cn['attempts']}회(기각 {cn['rejected']}) 중 {n_count}건. 두 실행은 궤적이 조금 다릅니다(달성 오차 {sci(cs.get('E'))} 대 {sci(cn.get('E'))}).")
    cusp = families["cusp1d"]["discriminators"]["B"]["cells"]
    w("- cusp1d의 다섯 칸: " + ", ".join(f"rtol {label} S {cusp[label]['S']} / N {cusp[label]['N']}" for label in LABELS) + ". 다른 세 계열은 두 팔 모두 0건입니다.")
    w("- 예산을 소진한 풀이는 두 팔 모두 출하 드라이버의 방식 그대로 실패로 보고되어(같은 오류, 같은 기각 처리) V에는 걸리지 않습니다. 돌려준 해가 임계값을 넘은 경우는 없습니다.")
    replay = results.get("exploratory_budget_replay") or {}
    runs = replay.get("runs") or {}
    if runs:
        w("")
        w("**탐색적 재현 분석(판정에 쓰지 않음).** 같은 선형계에서 세 커널을 직접 비교하기 위해, 한 실행이 지나간 모든 단계 선형계(완료된 풀이와 시도가 실패한 풀이)를 관찰자에서 다시 만들어 출하 커널, P1, P2로 각각 한 번씩 더 풀었습니다. 재현 풀이는 자기 연산자와 카운터를 쓰고 실행을 바꾸지 않습니다.")
        w("")
        w("| 실행(궤적) | rtol | 선형계 수 | 예산 소진: 출하 / P1 / P2 | 출하는 풀고 P2는 소진 | 출하는 소진, P2는 풂 | 출하는 풀고 P1은 소진 | 출하는 소진, P1은 풂 | 세 커널이 모두 푼 계의 열 수 합: 출하 / P1 / P2 | 재현이 드라이버와 다른 계 |")
        w("|---|---|---|---|---|---|---|---|---|---|")
        ordered = sorted(runs, key=lambda name: (name.split("__")[2] != "S", name.split("__")[0], LABELS.index(name.split("__")[1])))
        for name in ordered:
            row = runs[name]
            if row.get("systems") is None:
                continue
            family, label, label_arm = name.split("__")
            w(f"| {family}, {label_arm}의 궤적 | {label} | {whole(row['systems'])} | {row['budget'][0]} / {row['budget'][1]} / {row['budget'][2]} | {row['shipped_solved_p2_budget']} | {row['shipped_budget_p2_solved']} | {row['shipped_solved_p1_budget']} | {row['shipped_budget_p1_solved']} | {whole(row['columns_where_all_solved'][0])} / {whole(row['columns_where_all_solved'][1])} / {whole(row['columns_where_all_solved'][2])} | {row['replay_differs_from_driver']} |")
        w("")
        differs = sum(row.get("replay_differs_from_driver") or 0 for row in runs.values())
        same_end = all(row.get("end_state_equals_the_instruction_run") for row in runs.values() if "end_state_equals_the_instruction_run" in row)
        w(f"재현의 자기 점검: 드라이버 자신의 커널로 다시 푼 결과가 드라이버의 기록(열 수, 실패 여부)과 다른 계는 모든 실행을 합쳐 {differs}개이고, 재현을 붙인 실행의 끝 상태는 명령어 측정 실행과 {'모두 같습니다' if same_end else '같지 않은 것이 있습니다'}.")
        w("")
        s_runs = [row for row in runs.values() if row.get("arm") == "S" and row.get("systems") is not None]
        if s_runs:
            lost = sum(row["shipped_solved_p2_budget"] for row in s_runs)
            won = sum(row["shipped_budget_p2_solved"] for row in s_runs)
            lost1 = sum(row["shipped_solved_p1_budget"] for row in s_runs)
            won1 = sum(row["shipped_budget_p1_solved"] for row in s_runs)
            total = sum(row["systems"] for row in s_runs)
            w(f"S의 궤적 위 선형계 {whole(total)}개를 합치면: 출하 커널은 풀었는데 P2가 예산을 소진한 계 **{lost}개**, 반대로 출하 커널은 소진했는데 P2가 푼 계 **{won}개**; P1은 각각 **{lost1}개**와 **{won1}개**입니다.")
            edge = []
            for row in runs.values():
                for item in row.get("disagreements_first_40") or []:
                    if item[3] >= 0 and item[5] == -1:
                        edge.append((item[3], item[4]))
            won_columns = []
            for row in runs.values():
                for item in row.get("disagreements_first_40") or []:
                    if item[3] == -1 and item[5] >= 0:
                        won_columns.append(item[5])

            def span(values):
                return f"모두 {min(values)}" if min(values) == max(values) else f"{min(values)}에서 {max(values)}"

            if edge:
                w(f"P2만 예산을 소진한 계(두 궤적 합쳐 기록된 {len(edge)}개)에서 출하 커널이 쓴 열 수는 {span([a for a, _ in edge])}, P1이 쓴 열 수는 {span([b for _, b in edge])}입니다(예산 200). 곧, 출하 정책이 예산의 끝에서 겨우 푸는 계에서, 첫 두 주기를 10열과 20열로 짧게 끊는 P2가 수렴을 놓칩니다."
                  + (f" 반대 방향(출하 커널은 소진, P2는 풂)의 {len(won_columns)}개 계에서 P2가 쓴 열 수는 {span(won_columns)}으로, 같은 경계입니다." if won_columns else ""))
            w("")
            w("해석(탐색적): B가 한 칸에서 어긋난 것은 두 실행의 궤적 차이만으로 생긴 것이 아닙니다. 같은 선형계에서도 P2는 예산 경계의 계를 출하 정책보다 조금 더 자주 놓칩니다. P1은 재현한 모든 계에서 출하 커널과 예산 소진 여부가 같았습니다. 이것은 사전등록되지 않은 사후 분석이므로 판정을 바꾸지 않습니다.")
            w("")
    w("")

    # ------------------------------------------------------------------ 6
    w("## 6. 변형 선택 (P1 대 P2)")
    w("")
    if choice:
        w(f"동결 규칙: 결정 격자 밖의 개발 문제 둘(V06의 bruss1d n = 1024, adr2d n = 1024, 다섯 허용오차)에서 정합 오차 명령어 비의 기하평균이 작은 변형, 2% 이내면 P1. 결정 격자를 돌리기 전에 `CHOICE.json`을 커밋했습니다.")
        w("")
        w("| 개발 문제 | P1 (S 대비) | P2 (S 대비) | P2 / P1 (직접) |")
        w("|---|---|---|---|")
        for family, entry in choice["problems"].items():
            direct = entry["P2_against_P1"]["matched_error"]["instructions_net"].get("geometric_mean_ratio")
            w(f"| {family} (n = {entry['n']}) | {f3(entry['R']['P1'])} | {f3(entry['R']['P2'])} | {f3(direct)} |")
        w(f"| 점수(두 문제의 기하평균) | {f3(choice['score']['P1'])} | {f3(choice['score']['P2'])} | 비 {f3(choice.get('score_ratio_P2_over_P1'))} |")
        w("")
        w(f"선택: **{choice['chosen']}** ({choice['reason']}). 개발 실행에서 V 위반: P1 {choice['V_violations_in_development_runs']['P1']}건, P2 {choice['V_violations_in_development_runs']['P2']}건. 개발 문제의 dev06_bruss1d는 결정 계열 bruss1d와 같은 방정식의 다른 크기(512칸 대 700칸)라는 점을 밝혀 둡니다.")
        repeat = results.get("development_repeat_with_the_final_binary") or {}
        if repeat.get("compared"):
            w(f"선택 뒤 식별자 문자열만 바꾼 최종 바이너리로 선택된 팔의 개발 칸을 다시 돌려 {repeat['equal']}/{repeat['compared']}개 실행이 모든 카운터와 끝 상태 비트까지 같음을 확인했습니다.")
    w("")

    # ------------------------------------------------------------------ 7
    exploratory = results.get("exploratory") or {}
    w("## 7. 탐색적 측정 (판정에 쓰지 않음)")
    w("")
    if exploratory:
        names = {"P1": "P1 (선택되지 않은 변형)", "P2R": "P2 + ReuseConfirmed (마지막 참 잔차를 다시 계산하지 않음)",
                 "P1R": "P1 + ReuseConfirmed", "SR": "출하 정책 + ReuseConfirmed"}
        w("결정 격자에서 S 대비. \"정합\"은 정합 오차 명령어 비, \"예산\"은 계열 전체의 예산 소진 풀이 수(같은 계열의 S는 3절 표 참조), \"감사\"는 독립 잔차 감사 위반 수입니다.")
        w("")
        w("| 팔 | " + " | ".join(f"{family}: 정합 / 같은 허용오차 / 시도 비 / 예산 / 감사" for family in FAMILIES) + " |")
        w("|---|" + "---|" * len(FAMILIES))
        for label_arm in sorted(exploratory):
            cells = []
            for family in FAMILIES:
                block = exploratory[label_arm]["families"].get(family)
                if not block:
                    cells.append("-")
                    continue
                budget = sum(((block["cells"][label].get("policy_statistics") or {}).get("budget_exhausted_solves") or 0) for label in LABELS)
                cells.append(f"{f3(block['instructions_matched_error'])} / {f3(block['instructions_equal_tolerance'])} / {block['attempts_equal_tolerance']:.4f} / {budget} / {block['audit_violations']}" if block.get("attempts_equal_tolerance") is not None else "-")
            w(f"| {names.get(label_arm, label_arm)} | " + " | ".join(cells) + " |")
        w("")
        if "P1" in exploratory:
            block = exploratory["P1"]["families"]
            w("선택되지 않은 P1을 결정 격자에 돌린 결과(탐색적): 정합 오차 명령어 비 " + ", ".join(f"{family} {f3(block[family]['instructions_matched_error'])}" for family in FAMILIES)
              + "; 칸별 같은 허용오차 비의 최댓값 " + ", ".join(f3(max(block[family]["per_cell_instructions_equal_tolerance"].values())) for family in FAMILIES)
              + "; 시도 횟수 비 " + ", ".join(f"{block[family]['attempts_equal_tolerance']:.4f}" for family in FAMILIES)
              + "; 오차 비 최댓값 " + ", ".join(f"{block[family]['max_error_ratio']:.3f}" for family in FAMILIES)
              + "; 감사 위반 " + ", ".join(str(block[family]["audit_violations"]) for family in FAMILIES) + ".")
            w("P1은 동결된 선택 규칙이 고른 변형이 아니고 이 숫자들은 선택과 결정 실행 뒤에 나온 것이므로, 이것으로 P1을 통과시킬 수는 없습니다. P1을 채택하려면 새 사전등록과 확인 실행이 필요합니다.")
            w("")
            rows = []
            for label in LABELS:
                block = exploratory["P1"]["families"]["cusp1d"]["cells"][label]
                rows.append(f"rtol {label}: {(block.get('policy_statistics') or {}).get('budget_exhausted_solves')}")
            w("P1의 cusp1d 칸별 예산 소진: " + ", ".join(rows) + f" (S: " + ", ".join(str(cusp[label]['S']) for label in LABELS) + ").")
            w("")
    d0x = d0.get("exploratory_instructions_after_over_before") or {}
    if d0x.get("geometric_mean"):
        w(f"추가한 분기의 비용: 편집 뒤 바이너리의 S 명령어 수는 편집 전 바이너리의 {d0x['min']:.5f}배에서 {d0x['max']:.5f}배(기하평균 {d0x['geometric_mean']:.5f})입니다. 두 바이너리가 다르므로 참고값입니다.")
        w("")

    # ------------------------------------------------------------------ 8
    w("## 8. 벽시계 시간 (T, 보고만 함)")
    w("")
    w("V17의 규약 그대로: 시간 측정 접두어(`flock ... taskset -c 6-11`), 한 칸에 9라운드(앞 2라운드는 워밍업, 7회 반복), 라운드마다 S와 N을 번갈아 실행, 중앙값. 부하 평균 8 이하이고 두 팔의 퍼짐이 10% 이하인 칸만 authoritative입니다. 한 번의 호출이 잠금을 2분 넘게 잡지 않도록 긴 칸은 라운드별로 나눴습니다.")
    w("")
    w("| 계열 | 칸 라벨 (1e-3 ... 1e-7) | 같은 허용오차 속도비 S/N (칸별) | 정합 오차 속도비 | 계열 라벨 |")
    w("|---|---|---|---|---|")
    for family in FAMILIES:
        tf = families[family]["T"]
        labels = " ".join((tf["cells"].get(label) or "-")[:4] for label in LABELS)
        speed = " ".join(f2(tf["speedup_equal_tolerance"].get(label)) for label in LABELS)
        w(f"| {family} | {labels} | {speed} | {f2(tf['wall'].get('speedup'))} | {tf['label']} |")
    w("")
    retried = []
    longest = 0.0
    for family in FAMILIES:
        for label in LABELS:
            cell = families[family]["timing_cells"][label]
            if cell.get("measurements", 0) > 1:
                retried.append(f"{family} {label} (1차 측정이 규칙을 놓친 이유: {'; '.join(cell['tries'][0]['misses'])}; 사용한 측정: {cell.get('used_try')}차)")
            for attempt in cell.get("tries", []):
                longest = max(longest, attempt.get("longest_process_seconds_under_the_lock") or 0.0)
    w("라벨 표기: auth = authoritative, diag = diagnostic_only, NOT_ = 실행하지 않음. diagnostic_only인 칸의 숫자는 판정 근거가 아닙니다. 이 노드의 결정 척도는 명령어 수입니다.")
    w("")
    w(f"다시 측정한 칸: {'; '.join(retried) if retried else '없음'}. 모든 측정은 로그로 남아 있습니다. 한 번의 호출이 시간 측정 잠금 아래에서 돈 가장 긴 시간은 {longest:.1f}초입니다.")
    w("")

    # ------------------------------------------------------------------ 8b
    w("## 8-1. 소유자가 정할 일")
    w("")
    w("이 노드가 정한 것은 없습니다. 동결 규칙이 정한 대로 기본값을 그대로 두었고, 아래는 근거가 가리키는 선택지입니다.")
    w("")
    w("1. **지금 상태 유지.** 적응형 정책은 선택 사양으로 남습니다. 병합하려면 10절의 h2 센티널 두 줄을 orchestrator가 다시 고정해야 합니다.")
    if "P1" in exploratory:
        block = exploratory["P1"]["families"]
        w("2. **P1을 새로 사전등록해 확인.** 탐색적 실행에서 P1은 예산 소진이 모든 칸에서 S와 같았고(재현 분석에서도 같은 계에서 차이 0), 정합 오차 명령어 비는 "
          + ", ".join(f3(block[family]["instructions_matched_error"]) for family in FAMILIES)
          + "이었습니다. 다만 cusp1d에서 오차 비 최댓값이 " + f"{block['cusp1d']['max_error_ratio']:.3f}" + "으로 A의 한계 3에 가깝습니다(cusp1d의 끝점 오차는 V17 검토에서도 잡음이 크다고 적혔습니다). 이 격자는 이미 본 자료이므로 확인에는 새 문제나 새 크기가 필요합니다.")
    w("3. **P2를 유지하되 B를 다시 정의.** 예를 들어 같은 선형계 위의 재현 비교로 정의할 수 있지만, 이번 재현 분석에서 P2는 그 기준으로도 출하 정책보다 예산 경계의 계를 더 놓쳤습니다(위 5절).")
    w("")

    # ------------------------------------------------------------------ 9
    w("## 9. matrix-free 경로 사용자에게 달라지는 것과 그대로인 것")
    w("")
    w("**그대로인 것 (기본 동작 전부).**")
    w("")
    w("- `Jacobian::jvp`만 준 문제의 기본 경로는 지금까지와 같은 풀이, 같은 비트, 같은 식별자 `rodas5p-mf-fast-transformed-v1`입니다. D0가 결정 격자 20칸과 개발 10칸에서, 그리고 작은 문제의 시험(`mf_adaptive_shipped_bits`)이 진입점 다섯 가지(직접 진입점 둘, 정책 진입점, 워크스페이스, 라우터)에서 이를 확인합니다.")
    w("- golden 기록, facade의 기록 형식, 다른 모든 드라이버와 경로, 허용오차와 제어기 상수, G0.")
    w("")
    w("**새로 생긴 것 (명시적으로 골라야만 쓰임).**")
    w("")
    w("- `rodas5p_integrators::MfStageSolvePolicy { Shipped, ColumnTested, ColumnTestedDoubling }`. `RoutingOptions::matrix_free_policy`(기본 `Shipped`), `Rodas5pMfFastWorkspace::set_stage_solve_policy`, `integrate_rodas5p_mf_fast_observed_with_stage_solve_policy`.")
    w("- 적응형 정책으로 돌린 실행은 자기 식별자를 보고합니다: 선택된 변형 P2(`ColumnTestedDoubling`)는 `rodas5p-mf-fast-transformed-v2`, 선택되지 않은 P1은 후보 식별자 `rodas5p-mf-fast-transformed-v2-candidate-p1`.")
    w("- 커널은 `rodas5p_krylov::StagedGmresWorkspace::solve_column_tested_into`입니다. `solve_gmres_into_*`와 기존 staged 풀이의 코드는 건드리지 않았고, 편집 전에 기록한 비트 시험(`gmres_staged_column_test_bits`)이 이를 지킵니다.")
    w("- facade(`vigilode`)에는 이 선택 사양을 고르는 통로를 만들지 않았습니다. 기본값을 바꾸지 않았으므로 `solve.rs`, `solution.rs`, golden 파일은 손대지 않았습니다.")
    w("")

    # ------------------------------------------------------------------ 10
    w("## 10. 쓰기 허용 목록 밖이라 이 노드가 고칠 수 없었던 것")
    w("")
    if repin:
        second = repin["second_sentinel_not_repinned"]
        w(f"1. **센티널 h2.** `crates/rodas5p-integrators/tests/ptsw2b_contracts.rs`의 `h2_protected_sources_are_byte_identical`도 `src/rodas5p_matrix_free_fast.rs`의 sha256을 고정합니다. 이 파일은 보호 파일이고 이 노드의 예외 목록에 없어서(예외는 `ppsw1_6b_contracts.rs`의 g3뿐) 다시 고정하지 못했습니다. 드라이버를 편집한 이상 h2는 이 브랜치에서 실패합니다. 필요한 변경은 두 줄입니다: h2의 고정값을 `{second['h2_pin_needed']}`로, 그리고 g3가 고정한 `tests/ptsw2b_contracts.rs`의 값을 `{second['g3_pin_of_tests_ptsw2b_contracts_rs_needed']}`로. (g3의 드라이버 고정값은 `{repin['pin_of_the_driver']['old']}`에서 `{repin['pin_of_the_driver']['new']}`로 바꿨고 `SENTINEL_REPIN.json`에 기록했습니다.)")
    w("2. **기본값 전환을 막는 facade 시험.** 판정이 PASS였더라도, 기본 JVP 경로의 식별자 v1 또는 출하 진입점과의 비트 동일성을 고정한 단언이 허용 목록 밖에 있습니다: `crates/vigilode/tests/capabilities_contracts.rs:116`, `crates/vigilode/tests/facade_contracts.rs:258-281, 929-971, 1038`. 실행 전에 `PROTOCOL.json`에 적어 둔 사항입니다. 이번에는 판정이 FAIL이라 전환 자체를 하지 않았습니다.")
    w("3. **재수출.** `crates/rodas5p-krylov/src/lib.rs`가 허용 목록에 없어 새 커널의 보고 타입(`ColumnTestedGmresReport`)과 모니터 타입을 이름으로 재수출하지 못했습니다. 커널은 이미 수출된 `StagedGmresWorkspace`의 메서드로 닿습니다.")
    w("4. **사전등록 문장의 계열 이름.** `PREDECLARED.json`은 결정 격자를 \"V17이 최종 실행한 네 계열(families_final)\"이라고 가리키면서 괄호 안에 ac3d, cusp1d, burgers2d, grayscott2d라고 적었습니다. families_final은 fhn3d, cusp1d, burgers2d, bruss1d이고, ac3d와 grayscott2d는 V17이 실행 전에 철회한 계열이라 기준 해도 생성기도 없습니다. 이 노드는 가리킨 대상(families_final)을 따랐고, 그 해석을 첫 실행 전에 `PROTOCOL.json`에 적었습니다. 검토자의 확인이 필요합니다.")
    w("")

    # ------------------------------------------------------------------ 11
    w("## 11. 시험 묶음 (R)")
    w("")
    if suites:
        w("| 명령 | 종료 코드 | 통과 / 실패 / 무시 | 실패한 시험 |")
        w("|---|---|---|---|")
        for row in suites["rows"]:
            if not row.get("present"):
                w(f"| {row['name']} | 실행하지 않음 | - | - |")
                continue
            failing = "; ".join(f"{(test['binary'] or '').split('/')[-1]}::{test['test']}" for test in row.get("failing_tests", [])) or "-"
            w(f"| {row['name']} | {row.get('exit_code')} | {row.get('passed', 0)} / {row.get('failed', 0)} / {row.get('ignored', 0)} | {failing} |")
        w("")
        head = {row.get("git_head") for row in suites["rows"] if row.get("present")}
        dirty = {row.get("git_dirty_tracked") for row in suites["rows"] if row.get("present")}
        w(f"실행한 트리: 커밋 {', '.join(sorted(str(value)[:8] for value in head))}; 추적 파일 중 커밋되지 않은 것의 수: {', '.join(str(value) for value in sorted(dirty))}.")
        w("")
        if checker:
            w(f"루프 검사기(`check_checkpoint.py`)를 그대로 돌리면 종료 코드 {checker['checker_unchanged_run']['exit_code']}이고 마지막 줄은 다음과 같습니다:")
            w("")
            w("```")
            w(checker["checker_unchanged_run"]["last_line_verbatim"])
            w("```")
            w("")
            w(f"검사기는 첫 불만에서 멈춥니다. 검사기의 `require`만 메모리에서 수집기로 바꿔(파일은 바꾸지 않음, `checker_report.py`) 끝까지 돌리면 불만은 모두 {checker['complaints']}줄입니다:")
            w("")
            w("```")
            for row in checker["every_complaint_with_require_collecting_in_memory"]:
                w(row["complaint"])
            w("```")
            w("")
            outside = checker["complaints_naming_a_path_outside_the_task"]
            w(f"이 가운데 이 노드의 쓰기 허용 목록 밖의 경로를 가리키는 줄: {len(outside)}개. 기준 커밋 이후 바뀐 경로 중 허용 목록 밖의 것: {len(checker['changed_paths_outside_the_write_list_of_the_task'])}개.")
            w("")
    w("## 12. 재현")
    w("")
    w("```")
    w("# 측정 (바이너리: cargo test -p rodas5p-integrators --release --test mf_adaptive_measure --no-run)")
    w("python3 -I research/production_loop_20261116/mf_adaptive/run_measurements.py {record_a|setup|d0|dev|runs|timing|exploratory|shadow} --bin <binary> ...")
    w("# 결과 파일")
    w("python3 -I research/production_loop_20261116/mf_adaptive/parse_results.py {recording|choice|results}")
    w("python3 -I research/production_loop_20261116/mf_adaptive/run_checks.py")
    w("python3 -I research/production_loop_20261116/mf_adaptive/make_report.py")
    w("```")
    w("")
    if recording:
        w(f"편집 전 기록(커밋 A): `RECORDING_A.json` ({recording['summary']['runs_present']}개 실행, {recording['summary']['cells']}칸; V17의 팔 A와 {recording['summary']['v17_arm_A_cells_equal']}/{recording['summary']['v17_arm_A_cells_compared']}칸 동일).")
    binaries = results.get("binaries") or {}
    parts = []
    if recording:
        parts.append("커밋 A(편집 전): " + ", ".join(f"{key[:12]} ({value}개 로그)" for key, value in recording["binaries"].items()))
    if choice:
        parts.append("개발 칸(두 변형이 모두 든 바이너리): " + ", ".join(f"{key[:12]} ({value}개 로그)" for key, value in choice["binaries"].items()))
    if binaries:
        parts.append("결정 격자, 시간 측정, 탐색 실행, 재현 분석: " + ", ".join(f"{key[:12]} ({value}개 로그)" for key, value in binaries.items()))
    w("측정 바이너리(sha256 앞 12자리): " + "; ".join(parts) + ". 재현 분석용 바이너리는 측정 프로그램에 재현 모드만 더한 것이고 드라이버와 커널은 같습니다.")
    w("")
    with open(args.out, "w", encoding="utf-8") as handle:
        handle.write("\n".join(out) + "\n")
    print(f"wrote {os.path.relpath(args.out, NODE)} ({len(out)} lines)")


if __name__ == "__main__":
    sys.exit(main())
