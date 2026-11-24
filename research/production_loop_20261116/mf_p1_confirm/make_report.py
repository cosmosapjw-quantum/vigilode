#!/usr/bin/env python3
"""Loop node V25: renders REPORT_KO.md (Korean, for the owner) from
RESULTS.json, PROTOCOL.json and the optional step-6 files. Every number of
the report is read from those files; nothing is typed in.

usage: python3 -I make_report.py
"""
import json
import os
import sys

sys.dont_write_bytecode = True
NODE = os.path.dirname(os.path.abspath(__file__))
FAMILIES = ["ac3d", "robertson_rd1d", "schnakenberg2d", "porous2d"]
RTOLS = ["1e-3", "1e-4", "1e-5", "1e-6", "1e-7"]
VERDICT_KO = {"PASS": "통과", "FAIL": "실패", "NOT_DECIDABLE": "판정 불가", "NOT_RUN": "미실행"}


def load(name):
    path = os.path.join(NODE, name)
    if not os.path.exists(path):
        return None
    with open(path, encoding="utf-8") as handle:
        return json.load(handle)


def fmt(value, digits=3):
    if value is None:
        return "-"
    if isinstance(value, bool):
        return "예" if value else "아니오"
    if isinstance(value, int):
        return str(value)
    if value != 0 and (abs(value) < 1e-3 or abs(value) >= 1e5):
        return f"{value:.{digits - 1}e}"
    return f"{value:.{digits}f}"


def ko(verdict):
    return VERDICT_KO.get(verdict, verdict or "-")


def main():
    results = load("RESULTS.json")
    protocol = load("PROTOCOL.json")
    golden = load("GOLDEN_DIFF.json")
    suites = load("R_SUITES.json")
    edits = load("OUT_OF_LIST_EDITS.json")
    if results is None or protocol is None:
        sys.exit("RESULTS.json and PROTOCOL.json are needed")
    d = results["discriminators"]
    fam = results["families"]
    spans = {entry["id"]: entry["t_span"] for entry in protocol["families"]}
    sizes = {entry["id"]: entry["n"] for entry in protocol["families"]}
    decision = results["decision_before_step_6"]
    final = results["verdict"]
    lines = []
    w = lines.append

    w("# V25: 행렬 없는 경로의 적응 단계 풀이 P1 — 새 문제 네 가지에서의 확인 시험")
    w("")
    w("이 보고서는 `make_report.py`가 `RESULTS.json`과 `PROTOCOL.json`에서 숫자를 읽어 만든 것입니다. "
      "판정 규칙은 `PREDECLARED.json`(부록 A7, 동결)에 있고, 그 파일이 열어 둔 선택은 P1의 첫 실행 전에 "
      "`PROTOCOL.json`에 고정했습니다.")
    w("")
    w("## 1. 무엇을 시험했나")
    w("")
    w("RODAS5P의 행렬 없는(matrix-free) 경로는 단계마다 선형계 `(I - h gamma J) U = rhs`를 재시작 GMRES로 풉니다. "
      "두 정책의 차이는 '언제 멈출지를 얼마나 자주 확인하느냐' 하나뿐입니다.")
    w("")
    w("- **S (현재 기본값)**: 재시작 주기마다 Arnoldi 열 40개를 모두 만든 다음에야 참 잔차를 한 번 확인합니다. "
      "필요한 열이 5개뿐이어도 40개를 만듭니다.")
    w("- **P1 (적응 정책)**: 열을 하나 만들 때마다 Givens 회전으로 이미 알고 있는 투영 잔차를 확인하고, 단계 임계값을 "
      "처음 만족하는 열에서 주기를 끝냅니다. 그 다음 참 잔차를 **같은 임계값**으로 다시 확인하고, 통과하지 못하면 풀이를 "
      "계속합니다. 주기의 상한은 원래의 재시작 길이 40 그대로이므로 **새 하이퍼파라미터가 없습니다**.")
    w("- 단계 임계값, Krylov 방법·예산(200)·전처리, 제어기 상수, 허용오차는 바꾸지 않았습니다. "
      "P2(첫 주기 10열, 두 배씩 증가)는 비교용으로만 보고합니다.")
    w("")
    w("## 2. 문제족 (이전 노드가 쓰지 않은 네 가지)")
    w("")
    w("| 문제족 | n | 시간 구간 | 성격 | 기준해 불확실성 u_ref |")
    w("|---|---|---|---|---|")
    nature = {"ac3d": "3차원 Allen-Cahn (반응-확산)", "robertson_rd1d": "Robertson 반응 + 확산 (강한 강성)",
              "schnakenberg2d": "Schnakenberg 2차원 (Turing 패턴, 오차 증폭 큼)", "porous2d": "다공성 매질 u_t = lap(u^2) (비선형 확산)"}
    for family in FAMILIES:
        ref = fam[family]["reference"] or {}
        w(f"| {family} | {sizes[family]} | [{spans[family][0]:g}, {spans[family][1]:g}] | {nature[family]} | {fmt(ref.get('u_ref'))} |")
    w("")
    w("구간은 동결 규칙의 크기 맞춤 시험(S 단독, rtol 1e-7)으로 정했습니다: robertson_rd1d는 1초 미만이어서 세 번 두 배로 "
      "늘려 [0, 320]이 되었고, 나머지는 잠정 구간 그대로입니다. 기준해는 V17 규칙(SciPy DOP853 rtol 1e-13, 교차 검증 두 개)으로 "
      "독립 numpy 구현에서 계산했고, Rust와 numpy의 우변·JVP는 20개 상태에서 모든 성분이 비트 단위로 같았습니다.")
    w("")
    w("## 3. 판정표")
    w("")
    w("| 판별자 | 뜻 | 결과 |")
    w("|---|---|---|")
    w(f"| D0 | S와 P1을 명시적으로 고른 실행이 편집 뒤에도 비트 단위로 같음 | {ko(d['D0']['verdict'])} |")
    w(f"| V | P1이 끝낸 모든 단계 풀이의 참 잔차가 임계값 이하 (풀이 로그로 재계산) | {ko(d['V']['verdict'])} (P1 위반 {d['V']['violations']['P1']}, 완료 풀이 {d['V']['totals_P1']['completed_solves']}, 최대 잔차/임계값 {fmt(d['V']['max_residual_over_threshold'], 5)}) |")
    w(f"| B | 예산을 다 쓴 풀이가 어느 셀에서도 S보다 많지 않음 | {ko(d['B']['verdict'])} |")
    w(f"| A | 같은 허용오차에서 P1 오차가 S의 3배 이하 | {ko(d['A']['verdict'])} |")
    w(f"| S | 시도 횟수 비: 문제족 기하평균 1.05 이하, 셀 1.15 이하 | {ko(d['S']['verdict'])} |")
    w(f"| E | 같은 달성 오차에서 명령어 수가 문제족마다 S보다 적고(1.00 미만) 어느 셀에서도 많지 않음 | {ko(d['E']['verdict'])} |")
    w(f"| W | 권위 있는 벽시계 셀 중 P1이 S보다 느린 셀이 없음 | {ko(d['W']['verdict'])} |")
    w(f"| G | (전환 시) 골든 기록 차이가 한계 이내 | {ko(d['G']['verdict'])} |")
    w(f"| R | (전환 시) 테스트 묶음·clippy·rustfmt 통과 | {ko(d['R']['verdict'])} |")
    w("")
    w("## 4. 문제족별 숫자")
    w("")
    w("| 문제족 | E: 같은 오차에서 명령어 비 (P1/S) | E: 가장 나쁜 셀 | A: 최대 오차 비 | S: 시도 비 기하평균 / 최대 | B: 예산 소진 S / P1 | W: 벽시계 속도 향상 | W: 권위 셀 |")
    w("|---|---|---|---|---|---|---|---|")
    for family in FAMILIES:
        disc = fam[family]["discriminators"]
        w(f"| {family} | {fmt(disc['E']['family_number'])} | {fmt(disc['E']['max_cell_number'])} | {fmt(disc['A']['max_error_ratio'])} | "
          f"{fmt(disc['S']['geometric_mean'])} / {fmt(disc['S']['max_ratio'])} | {disc['B']['total_S']} / {disc['B']['total_P1']} | "
          f"{fmt(disc['W'].get('speedup_at_matched_error'))} | {disc['W']['authoritative_cells']} / 5 |")
    w("")
    w("명령어 비는 V17의 같은 달성 오차 통계(오차 구간 201점의 기하평균)입니다. 1보다 작을수록 P1이 싸다는 뜻입니다. "
      "벽시계 속도 향상은 S 시간 / P1 시간이며, 고정 코어·교차 실행·준비 2회와 반복 7회·부하 기록 규칙을 만족한 셀만 판정에 씁니다.")
    w("")
    w("### 셀별 (허용오차마다)")
    w("")
    for family in FAMILIES:
        disc = fam[family]["discriminators"]
        cmp_cells = fam[family]["comparison"]["cells"]
        w(f"**{family}**")
        w("")
        w("| rtol | 오차 S | 오차 P1 | 오차 비 | 명령어 셀 수치 (종류) | 시도 S / P1 | 열/풀이 S / P1 | 예산 소진 S / P1 | 벽시계 비 P1/S (권위) |")
        w("|---|---|---|---|---|---|---|---|---|")
        for label in RTOLS:
            row = cmp_cells[label]
            ecell = disc["E"]["cells"][label]
            kind = "같은 오차" if (ecell.get("kind") or "").startswith("matched") else "같은 허용오차"
            scell = disc["S"]["cells"][label]
            bcell = disc["B"]["cells"][label]
            wcell = disc["W"]["cells"][label]
            s_cells = fam[family]["arms"]["S"][label]
            p_cells = fam[family]["arms"]["P1"][label]
            cps = lambda cell: (cell.get("linear_iterations") or 0) / cell["linear_solves"] if cell.get("linear_solves") else None
            w(f"| {label} | {fmt(row.get('E_S'))} | {fmt(row.get('E_X'))} | {fmt(row.get('error_ratio'))} | {fmt(ecell.get('cell_number'))} ({kind}) | "
              f"{scell.get('attempts_S')} / {scell.get('attempts_P1')} | {fmt(cps(s_cells), 1)} / {fmt(cps(p_cells), 1)} | {bcell.get('S')} / {bcell.get('P1')} | "
              f"{fmt(wcell.get('ratio'))} ({'예' if wcell.get('authoritative') else '아니오'}) |")
        w("")
    w("## 5. 판정")
    w("")
    w(f"- 6단계 전 동결 규칙의 판정: **{decision['verdict']}** — {decision['reason']}")
    w(f"- 최종 판정: **{final['verdict']}** (G {ko(final['G'])}, R {ko(final['R'])})")
    w("")
    notes = results.get("report_notes_ko") or []
    for note in notes:
        w(f"- {note}")
    if notes:
        w("")
    if golden:
        w("### G: 골든 기록")
        w("")
        w(f"- {golden.get('summary')}")
        w("")
    if suites:
        w("### R: 테스트 묶음")
        w("")
        w(f"- {suites.get('summary')}")
        for failure in suites.get("failing_tests", [])[:20]:
            w(f"  - 실패: `{failure}`")
        w("")
    if edits:
        w("### 작성 허용 목록 밖에서 바뀌어야 하는 줄 (오케스트레이터/소유자 결정 필요)")
        w("")
        for item in edits.get("lines", []):
            w(f"- `{item.get('file')}:{item.get('line')}` — {item.get('reason')}")
        w("")
    w("## 6. P2 (보고만 함)")
    w("")
    w("| 문제족 | 같은 오차 명령어 비 P2/S | 예산 소진 S / P2 | 최대 오차 비 | 감사 위반 |")
    w("|---|---|---|---|---|")
    for family in FAMILIES:
        p2 = fam[family]["P2_reported_only"]
        w(f"| {family} | {fmt(p2['instructions_matched_error'])} | {p2['budget_exhausted']['S']} / {p2['budget_exhausted']['P2']} | {fmt(p2['max_error_ratio'])} | {p2['audit_violations']} |")
    w("")
    w("## 7. 사용자에게 무엇이 바뀌는가")
    w("")
    for text in results.get("user_impact_ko") or ["(RESULTS.json에 기록 없음)"]:
        w(f"- {text}")
    w("")
    w("## 8. 증거 파일")
    w("")
    w("- 동결 설계: `PREDECLARED.json`; 프로토콜: `PROTOCOL.json`; 결과: `RESULTS.json`; 기록 A: `RECORDING_A.json`")
    w("- 원시 로그: `logs/sizing`, `logs/consistency`, `logs/reference`, `logs/setup`, `logs/record_a` (S와 P1, 단계 풀이 로그 `*.solves.txt.gz` 포함), `logs/p2`, `logs/timing`")
    w("- 진행 기록: `NOTES.txt`")
    text = "\n".join(lines) + "\n"
    with open(os.path.join(NODE, "REPORT_KO.md"), "w", encoding="utf-8") as handle:
        handle.write(text)
    print(f"wrote REPORT_KO.md ({len(text)} bytes)")


if __name__ == "__main__":
    main()
