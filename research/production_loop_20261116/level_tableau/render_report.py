#!/usr/bin/env python3
"""Loop L1 node V16 -- render REPORT_KO.md from RESULTS.json, LITERATURE.json and PREDECLARED.json.

    python3 -I render_report.py

Every number in the report is formatted from RESULTS.json (which parse_results.py builds from the raw logs).
"""
import json
import os
import subprocess

HERE = os.path.dirname(os.path.abspath(__file__))
R = json.load(open(os.path.join(HERE, "RESULTS.json")))
LIT = json.load(open(os.path.join(HERE, "LITERATURE.json")))
PRE = json.load(open(os.path.join(HERE, "PREDECLARED.json")))
out = []


def w(s=""):
    out.append(s)


def g(x, n=3):
    if x is None:
        return "n/a"
    if isinstance(x, bool):
        return "예" if x else "아니오"
    if isinstance(x, int):
        return str(x)
    if x == 0:
        return "0"
    if abs(x) >= 1e4 or abs(x) < 1e-3:
        return ("%." + str(n - 1) + "e") % x
    return ("%." + str(n) + "g") % x


def table(head, rows_):
    w("| " + " | ".join(head) + " |")
    w("|" + "---|" * len(head))
    for r in rows_:
        w("| " + " | ".join(str(c) for c in r) + " |")
    w()


def git(*args):
    try:
        return subprocess.run(["git", "-C", HERE] + list(args), capture_output=True, text=True).stdout.strip()
    except Exception:
        return "n/a"


VERDICT_KO = {
    "FOUND_PASS": "발견, 차수 확인, pay-off 스크린 통과",
    "FOUND_FAIL": "발견(대수 조건 충족), pay-off 스크린 미통과 (KILL 수준은 아님)",
    "FOUND_KILL": "발견(대수 조건 충족), 그러나 세 문제 모두에서 시도 횟수 비가 깊이 이득 이상: 동결된 KILL 조건에 해당",
    "FOUND_NOT_DECIDABLE": "발견(대수 조건 충족), 차수 또는 pay-off 판정 불가",
    "NOT_FOUND_WITHIN_BUDGET": "선언한 예산 안에서 찾지 못함 (불가능하다는 뜻이 아님)",
    "BLOCKED_TOOL_VALIDATION": "도구 검증 실패로 판정 불가",
}
NODE_KO = {"PASS": "PASS", "KILL": "KILL", "FAIL": "FAIL (통과한 레이아웃 없음, 모든 발견 레이아웃이 KILL 수준인 것은 아님)",
           "NOT_DECIDABLE": "판정 불가"}
LAYOUT_KO = {"depth5": "깊이 5 (8단, 레벨 1-2-2-2-1)", "depth6": "깊이 6 (10단, 레벨 1-2-2-2-2-1)"}
V = R["verdicts"]
C = R["candidates"]
TV = R["tool_validation"]
EX = R.get("exploratory", {})
FZ = R["frozen_thresholds"]

w("# V16 레벨 구조 Rosenbrock-Wanner 타블로 탐색 (루프 L1, 2026-11-17)")
w()
w("- 노드: `V16_LEVEL_TABLEAU_SEARCH` (사전등록 부록 A1). Python 프로토타입만 사용, Rust 코드와 보호 경로는 변경 없음.")
w("- 이 문서는 `render_report.py`가 `RESULTS.json`에서 생성한다. `RESULTS.json`은 `parse_results.py`가 `logs/*.jsonl`만 읽어 만든다.")
w("- 브랜치 `%s`, 생성 시점 HEAD `%s`." % (git("branch", "--show-current"), git("rev-parse", "--short", "HEAD")))
w("- 사전선언: `PREDECLARED.json` (첫 측정 전 커밋). 수정 기록 %d건: %s. 내용은 2절." % (
    len(PRE["amendments"]), ", ".join(a["id"] for a in PRE["amendments"])))
w()

# ------------------------------------------------------------------------------------------------ 1
w("## 1. 결론")
w()
w("도구 검증: 생성된 차수 조건 %d개(차수 5 이하)가 보호된 RODAS5P 계수에서 최대 %s (binary64), %s (60자리)로 소멸한다. 기준 %s: %s." % (
    TV["conditions"], g(TV["max_abs_residual_binary64"]), g(TV["max_abs_residual_mp60"]), g(TV["threshold"]),
    "통과" if TV["passed"] else "실패"))
w()
rows_ = []
for layout in ("depth5", "depth6"):
    v = V[layout]
    prim = v.get("primary_candidate")
    ver = C.get(prim, {}).get("verify", {}) if prim else {}
    ratios = v.get("attempt_ratios") or {}
    orders = v.get("observed_orders") or {}
    rows_.append([LAYOUT_KO[layout], VERDICT_KO.get(v["verdict"], v["verdict"]), prim or "-",
                  g(ver.get("gamma"), 6), g(ver.get("order5_max_abs_residual_binary64")) + " / " + g(ver.get("order5_max_abs_residual_exact")),
                  ", ".join(g(orders.get(p), 4) for p in ("BRUSS_ODE", "VDP_MU2", "PENDULUM_FORCED")),
                  ", ".join("%s %s" % (p, g(ratios.get(p), 4)) for p in ("ROBER", "VDPOL", "BURGERS")),
                  "통과 <= %s (3문제 중 2), KILL >= %s (3문제 모두)" % (g(v.get("pass_threshold")), g(v.get("depth_gain_kill_level")))])
table(["레이아웃", "판정", "1순위 후보", "gamma", "차수 5 잔차 (binary64 / 정확 산술)", "관측 차수 (3문제)",
       "시도 횟수 비 = 후보 / RODAS5P (정합 오차)", "동결 기준"], rows_)
w("노드 판정 (동결 행의 규칙 적용): **%s**." % NODE_KO.get(V["node"], V["node"]))
w()
PSR = R["payoff_screen"].get("roles", {})
for arm in sorted(a for a, r in PSR.items() if r["role"] == "exploratory"):
    lay = PSR[arm]["layout"]
    ar = R["payoff_screen"]["arms"][arm]
    oc = R["order_check"].get(arm, {})
    cf = EX.get("methods", {}).get(arm, {}).get("code_form", {})
    w("탐색적 관찰 (판정에 쓰이지 않음): 개발 문제 순위에서 1순위가 아니었던 `%s` (%s, gamma %s)를 같은 스크린에 함께 돌렸다. 시도 횟수 비는 %s. 관측 차수는 %s. 코드형 계수 max|C| = %s. 이 후보는 사전선언된 선택 규칙이 고른 후보가 아니므로 레이아웃의 판정을 바꾸지 않는다. 단서로만 취급한다(12절)." % (
        arm, LAYOUT_KO[lay], g(C.get(arm, {}).get("verify", {}).get("gamma"), 4),
        ", ".join("%s %s" % (p, g(ar.get(p, {}).get("ratio"), 4)) for p in ("ROBER", "VDPOL", "BURGERS")),
        ", ".join("%s %s (%s)" % (p, g(oc.get(p, {}).get("observed_order"), 4), oc.get(p, {}).get("status", "n/a")) for p in ("BRUSS_ODE", "VDP_MU2", "PENDULUM_FORCED")),
        g(cf.get("max_abs_C"), 4)))
    w()
def _defect(name, k):
    v = (EX.get("methods", {}).get(name, {}).get("stability") or {}).get("taylor_defects_z1_to_z8")
    return v[k - 1] if v else None


G5 = TV["depth5_gamma"][:7]
G6L = next((x["gamma"] for x in EX.get("linear_bound_layout", []) if "linear order 6" in x.get("note", "")), None)
P5 = V["depth5"].get("primary_candidate")
P6 = V["depth6"].get("primary_candidate")
Z7_ROD = _defect("RODAS5P", 7)
Z6_D5 = _defect(P5, 6) if P5 else None
Z7_D6 = _defect(P6, 7) if P6 else None
w("구조적 설명 (탐색적 분석, 8절).")
w()
w("- 깊이 L의 강성 정확 타블로의 안정성 함수는 P(z)/(1 - gamma z)^L이고 gamma만으로 정해진다. RODAS5P는 극 7개로 선형 차수 6(z^7 결함 %s)을 갖지만, 깊이 5는 선형 차수 5(z^6 결함 %s)에 묶이고 깊이 6은 gamma = %s에서만 선형 차수 6이 되며 그때의 z^7 결함은 %s로 RODAS5P의 %s배다(8.1절)." % (
    g(Z7_ROD), g(Z6_D5), g(G6L, 5), g(Z7_D6), g(abs(Z7_D6 / Z7_ROD) if (Z7_D6 and Z7_ROD) else None, 3)))
w("- Prothero-Robinson 인자 W_2(z)를 항등적으로 0으로 만들면(RODAS5P가 가진 성질) 안정성 함수의 극이 하나 줄어든다(8.2b절의 항등식). 그래서 깊이 5에서는 불가능하고 깊이 6에서는 gamma가 깊이 5의 값 %s로 고정된다. 여기의 후보들은 이 조건을 정확히 만족하지 못하며 강성 Prothero-Robinson 문제에서 쌍별 차수가 1 근처로 떨어진다(8.3절)." % G5)
w("- 깊이 5에서는 임베디드 방법을 강성 정확하게 만들 수 없다(3절).")
w()
w("읽는 법.")
w()
w("- \"발견\"은 동결 행의 대수 조건 전부(차수 5의 17개 조건, 강성 정확성과 R(무한대)=0, A-안정성, 임베디드 차수 4, sum b_i gamma_i = 0, 계수 크기 100 이하)를 독립 검증 스크립트가 계수 파일에서 다시 확인했다는 뜻이다.")
w("- 시도 횟수 비는 같은 프로토타입, 같은 제어기 공식으로 적분했을 때 같은 달성 오차에서의 (수락+거부) 스텝 수의 비다. 1보다 크면 후보가 RODAS5P보다 스텝을 더 쓴다.")
w("- 깊이 이득은 체인만의 상한이다: 8/5 = 1.6, 8/6 = 1.33. 시도 횟수 비가 이 값 이상이면 두 프로세서를 써도 체인에서 얻는 것이 없다.")
w("- \"찾지 못함\"은 선언한 예산 안의 결과이며 불가능의 증명이 아니다. \"발견\"된 타블로는 보호 경로 밖의 새 방법이고, 라우팅과 Rust 이식은 이 노드의 범위가 아니다.")
w()

# ------------------------------------------------------------------------------------------------ 2
w("## 2. 절차와 사전선언 수정")
w()
w("1. `PREDECLARED.json`을 생성기, 탐색, 적분기의 어떤 실행보다 먼저 커밋했다. 동결 행이 정하지 않은 선택(레이아웃별 조건 목록, 미지수, 최적화기, 시작점 수와 시드, 시험 문제와 허용오차, 기준해, 정합 오차 보간 규칙, 제어기 공식, 계수 한계)을 담았다.")
w("2. 선언한 시드 범위 밖의 스모크 실행과 RODAS5P 단독 실행에서 드러난 문제 때문에 선언을 %d번 수정했다. 각 수정은 해당 실행 전에 커밋했고 동결 임계값은 건드리지 않았다. 리뷰어가 수정의 정당성을 판단해야 한다." % len(PRE["amendments"]))
w()
AMEND_KO = {
    "A-1": "탐색 최적화기의 내부만 바꿈: 1단계 함수 평가 150회, 경성 조건 다양체 위에서의 연마(시작점마다 60회, 상위 40개는 600회 추가), gamma 연속법의 풀이 방식, REG 가중치 1e-5, 깊이 6의 gamma 탐색 구간 [0.19, 0.33].",
    "A-2": "차수 검사의 반올림 가드를 1e-11에서 1e-12로 낮춤(후보가 생기기 전, RODAS5P 단독 실행을 근거로).",
    "A-3": "깊이 6에 정식화 D6L을 추가: gamma를 선형 차수 6이 되는 근 0.33414...(아래로 반올림)에 고정. 깊이 6의 후보 풀은 D6T 상위 8개 + D6L 상위 8개.",
}
for a in PRE["amendments"]:
    w("- **%s** (%s). 범위: %s." % (a["id"], a["date"], a["scope"]))
    if a["id"] in AMEND_KO:
        w("  - 요약: %s" % AMEND_KO[a["id"]])
    w("  - 근거: %s" % a["evidence"])
    for c in a["changes"]:
        w("  - 변경: %s" % c)
    w("  - 유지: %s" % a["unchanged"])
w()
w("3. 후보 선택은 pay-off 문제(ROBER, VDPOL, BURGERS)를 보지 않고 했다: 정밀화된 후보를 개발 문제(HIRES, OREGO; U11에서 이미 소비된 설정이며 여기서는 개발용으로만 사용)에서 돌려 시도 횟수 비의 기하평균이 가장 낮은 후보를 레이아웃의 1순위로 정했다. 판정은 1순위 후보만으로 한다.")
w()

# ------------------------------------------------------------------------------------------------ 3
w("## 3. 도구 검증")
w()
w("- 생성기 `rowtrees.py`: 뿌리 나무를 차수 6까지 열거하고(차수별 %s개, 차수 6은 %d개), 가지가 하나인 꼭짓점에는 beta(대각 gamma 포함), 둘 이상인 꼭짓점에는 alpha를 쓰는 재귀로 Phi를 만든다. 조건은 `sum_j b_j Phi_j(t) = 1/gamma(t)`. 출처는 Hairer-Wanner II, IV.7절이며 기억에 의한 인용이다(책을 확인하지 않음). 공식은 다시 유도했고 RODAS5P에서 수치로 검증했다." % (
    ", ".join(str(x) for x in TV["tree_counts"]), 20))
w("- 동결 검증: 17개 조건의 RODAS5P 잔차 최대 %s (binary64). 기준 %s 이하이므로 통과." % (g(TV["max_abs_residual_binary64"]), g(TV["threshold"])))
w("- 생성기를 쓰지 않는 수치 교차 확인: 프로토타입 적분기에서 RODAS5P 자체의 고정 스텝 관측 차수 (6절 표의 RODAS5P 행).")
w("- 독립 검증기 `verify_candidate.py`는 17개 조건을 손으로 풀어 쓴 식으로 계산하며 생성기와 코드를 공유하지 않는다. 후보 형식으로 옮겨 쓴 RODAS5P에 적용한 자체 시험(스크래치, `NOTES.txt` 3번 항목)에서 생성기와 같은 잔차를 냈다.")
w("- 지수-1 확장(동결 검증 대상 아님): 두 색 나무 %d개를 생성했고 그중 %d개가 RODAS5P 주 가중치에서 1e-10 이하로 소멸한다. 소멸하지 않는 것: %s. 소멸하는 조건만 탐색의 연성 목표로 썼다." % (
    TV["index1_generated"], TV["index1_selected_main"],
    "; ".join("%s (차수 %d, 잔차 %s)" % (t["tree"], t["order"], g(t["residual_b"])) for t in TV["index1_not_vanishing_on_rodas5p"]) or "없음"))
w("- 깊이 5의 구조적 사실: gamma는 %s...로 고정된다. 이론 S9의 44-미지수 변형(임베디드 가중치 = beta의 7행)은 불가능하다: 그 임베디드 방법은 네 레벨만 쓰고 R_hat(무한대)=0이므로 차수 4에 `24 g^4 - 96 g^3 + 72 g^2 - 16 g + 1 = 0`이 필요한데, 고정된 gamma에서 이 값은 %s이다. 그래서 깊이 5에서는 임베디드 가중치를 자유 벡터로 두었고 임베디드 방법은 강성 정확이 아니다." % (
    TV["depth5_gamma"][:14], g(TV["depth5_embedded_obstruction_value"], 4)))
w()

# ------------------------------------------------------------------------------------------------ 4
w("## 4. 탐색")
w()
rows_ = []
for fid in sorted(R["search"]):
    e = R["search"][fid]
    sp = e.get("score_of_passing") or {}
    pb = e.get("phase_b", {})
    rows_.append([fid, LAYOUT_KO.get(e["layout"], e["layout"]), e["unknowns"], e["hard_conditions"], e["starts"],
                  "예" if e["complete"] else "아니오", e["passed_double"], e["gamma_out_of_window"],
                  g(sp.get("min")), g(sp.get("median")), pb.get("polished", "-"),
                  g((pb.get("score") or {}).get("min")), g((pb.get("score") or {}).get("median"))])
table(["정식화", "레이아웃", "미지수", "경성 조건", "시작점", "완료", "binary64 통과", "gamma 창 밖", "점수 최소 (A)",
       "점수 중앙 (A)", "B단계 수", "점수 최소 (B)", "점수 중앙 (B)"], rows_)
w("- 통과 기준(binary64): 사영 뒤 경성 조건 잔차 최대 1e-12 이하, gamma가 탐색 구간 안, 계수 크기 100 이하. 실패한 시작점도 로그에 남아 있다.")
w("- 점수 = sqrt(|D|^2 + |Dhat|^2 + |PR|^2 + 0.09 |PRhat|^2 + 0.01 |E6|^2): 지수-1 조건, Prothero-Robinson 항, 차수 6 오차 계수의 가중 노름. 낮을수록 좋다. FOUND 판정에는 쓰이지 않는다.")
cont = R["search"].get("D6T", {}).get("continuation")
if cont:
    w("- 깊이 6의 gamma 연속법(D6T, B단계 상위 5개에서 출발, gamma 고정 격자): %d회 풀이 중 %d회 통과. gamma별 최저 점수: %s." % (
        cont["solves"], cont["passed_double"], ", ".join("%s: %s" % (g(x["gamma"], 3), g(x["score"])) for x in cont["best_score_by_gamma"])))
w()

# ------------------------------------------------------------------------------------------------ 5
w("## 5. 후보의 대수 검증 (계수 파일에서, 독립 스크립트)")
w()
rows_ = []
for cid in sorted(C):
    c = C[cid]
    v = c.get("verify", {})
    d = c.get("development", {})
    rows_.append([cid, g(v.get("gamma"), 6), g(c["score_double"]), g(v.get("order5_max_abs_residual_binary64")),
                  g(v.get("order5_max_abs_residual_exact")), g(v.get("embedded_order4_max_abs_residual_exact")),
                  g(v.get("embedded_order5_max_abs_residual")), g(v.get("W_sum_b_gamma_i_exact")),
                  g(v.get("R_infinity_exact")), g(v.get("A_stable")), g(v.get("max_abs_coefficient"), 4),
                  g(v.get("algebraic_pass")),
                  g((d.get("HIRES") or {}).get("ratio"), 4), g((d.get("OREGO") or {}).get("ratio"), 4),
                  g(c.get("development_geomean"), 4)])
table(["후보", "gamma", "점수", "차수5 잔차 b64", "차수5 잔차 정확", "임베디드 차수4 잔차", "임베디드 차수5 잔차(0이 아니어야 함)",
       "sum b gamma_i", "R(무한대)", "A-안정", "최대 계수", "대수 통과", "HIRES 비", "OREGO 비", "개발 기하평균"], rows_)
w("- 계수 파일: `candidates/<후보>.json`. 독립 계수는 유효숫자 40자리 십진 문자열이고, 종속 계수(묶인 행, b, bhat)는 그 문자열의 정확한 십진 합이므로 구조적 항등식(b = beta의 마지막 행 등)은 파일 안에서 정확히 성립한다.")
w("- 정확 산술 = 십진 문자열을 `fractions.Fraction`으로 읽은 유리수 계산. A-안정성 = 설계 안정성 함수의 E-다항식에 대한 Sturm 열(양의 근 0개) + 타블로 분자와 설계 분자의 차이 1e-25 이하 + 허수축 격자에서 |R(iy)| - 1 <= 1e-20.")
w("- `candidates/smoke_*.json`은 선언 범위 밖 시드의 파이프라인 시험용 파일이며 판정에 쓰이지 않는다.")
w()
for layout in ("depth5", "depth6"):
    s = R["selection"][layout]
    if s.get("primary"):
        w("- %s의 1순위: `%s` (%s). 개발 순위: %s." % (LAYOUT_KO[layout], s["primary"], s.get("rule"),
          ", ".join("%s %s" % (x["id"], g(x["development_geomean"], 4)) for x in s.get("ranking", [])) or "n/a"))
    else:
        w("- %s: 대수 조건을 통과한 후보 없음." % LAYOUT_KO[layout])
w()

# ------------------------------------------------------------------------------------------------ 6
w("## 6. 고정 스텝 관측 차수 (정확한 야코비안)")
w()
OC = R["order_check"]
rows_ = []
for arm in sorted(k for k in OC if k != "references"):
    for pid in ("BRUSS_ODE", "VDP_MU2", "PENDULUM_FORCED"):
        e = OC[arm].get(pid)
        if e:
            rows_.append([arm, pid, " ".join(g(x) for x in e["errors"]), " ".join(g(p["order"], 4) for p in e["pairs"]),
                          g(e["observed_order"], 4), e["status"]])
table(["방법", "문제", "오차 (스텝 수 10, 20, 40, 80, 160, 320)", "쌍별 차수 (오차 >= 1e-12인 쌍)", "관측 차수 (중앙값)", "판정 (>= 4.8)"], rows_)
w("- 기준해: mpmath.odefun 40자리. DOP853(rtol 1e-13)과의 차이: %s." % ", ".join(
    "%s %s" % (p, g(v["uncertainty_vs_dop853"])) for p, v in sorted(OC.get("references", {}).items())))
w("- RODAS5P 행은 프로토타입의 검증이자 생성기와 무관한 교차 확인이다.")
w()

# ------------------------------------------------------------------------------------------------ 7
w("## 7. pay-off 스크린 (정합 달성 오차에서의 시도 횟수 비)")
w()
PS = R["payoff_screen"]
rows_ = []
ROLE_KO = {"primary": "1순위 (판정에 사용)", "exploratory": "탐색적 (판정에 쓰이지 않음)"}
for arm in sorted(PS["arms"], key=lambda a: (PS["roles"][a]["layout"], PS["roles"][a]["role"] != "primary", a)):
    role = PS["roles"][arm]
    v = V[role["layout"]]
    for pid in ("ROBER", "VDPOL", "BURGERS"):
        m = PS["arms"][arm].get(pid)
        if m:
            rows_.append([LAYOUT_KO[role["layout"]], arm, ROLE_KO[role["role"]], pid, g(m.get("ratio"), 4),
                          g(m.get("ratio_min"), 4) + " ~ " + g(m.get("ratio_max"), 4),
                          m["usable_targets"], g(PS["references"][pid]["u_ref"]), m["failed_runs_new"],
                          g(v.get("pass_threshold")), g(v.get("depth_gain_kill_level"))])
table(["레이아웃", "후보", "역할", "문제", "비 (기하평균)", "목표별 최소 ~ 최대", "쓸 수 있는 목표 수", "기준해 불확도 u_ref", "실패 실행",
       "통과 임계", "KILL 수준"], rows_)
for layout in ("depth5", "depth6"):
    v = V[layout]
    if v.get("attempt_ratios"):
        w("- %s: 통과 임계 이하 %s문제, 깊이 이득 이상 %s문제 -> pay-off %s." % (
            LAYOUT_KO[layout], g(v.get("problems_at_or_below_pass_threshold")), g(v.get("problems_at_or_above_depth_gain")), v.get("payoff")))
w("- 규칙: 허용오차 사다리 rtol = 10^(-k/2), k = 6..18, atol = a_scale * rtol. 각 방법의 (오차, 시도) 점에서 파레토 전선을 만들고 ln-ln 선형 보간으로 목표 오차 10^(-m/2)에서의 시도 횟수를 구한다. 두 방법의 전선 범위 안에 있고 100 u_ref 이상인 목표만 쓴다. 문제의 비는 쓸 수 있는 목표들의 기하평균이며 목표가 3개 미만이면 판정 불가.")
w("- 기준해: scipy Radau (rtol 1e-13). u_ref는 Radau(rtol 1e-11)과의 차이와 RODAS5P 프로토타입(rtol 1e-11)과의 차이 중 큰 값.")
w("- 벽시계 시간은 재지 않았다. 판정에 쓰인 수는 모두 결정적 계수기(스텝 시도 횟수)다.")
w()
for prim in sorted(PS["arms"], key=lambda a: (PS["roles"][a]["layout"], PS["roles"][a]["role"] != "primary", a)):
    layout = PS["roles"][prim]["layout"]
    arms = PS["arms"].get(prim)
    if not arms:
        continue
    w("### %s, 후보 `%s` (%s): 허용오차별 원자료" % (LAYOUT_KO[layout], prim, ROLE_KO[PS["roles"][prim]["role"]]))
    w()
    for pid in ("ROBER", "VDPOL", "BURGERS"):
        m = arms.get(pid)
        if not m:
            continue
        rows_ = []
        ref = {r["k"]: r for r in m["runs_rodas5p"]}
        for r in m["runs_new"]:
            q = ref.get(r["k"], {})
            rows_.append([g(r["rtol"]), "%s+%s" % (q.get("accepted"), q.get("rejected")), g(q.get("error")),
                          "%s+%s" % (r["accepted"], r["rejected"]), g(r.get("error")) if r["finished"] else "미완료"])
        w("%s:" % pid)
        w()
        table(["rtol", "RODAS5P 수락+거부", "RODAS5P 오차", "후보 수락+거부", "후보 오차"], rows_)

# ------------------------------------------------------------------------------------------------ 8
w("## 8. 탐색적 분석 (판정에 쓰이지 않음)")
w()
w("### 8.1 안정성 함수가 정하는 선형 오차")
w()
w("깊이 L의 강성 정확 타블로에서 N = beta - gamma I는 N^L = 0이므로 R(z) = P(z)/(1 - gamma z)^L이고 deg P <= L - 1이다. 차수 조건이 P를 gamma만의 함수로 고정한다. 따라서 선형 문제에서의 오차는 나머지 계수와 무관하게 (L, gamma)로 정해진다.")
w()
lb = EX.get("linear_bound_layout", [])
eps = EX.get("eps", [])
if lb:
    table(["깊이", "gamma", "비고"] + ["eps=%s" % g(e) for e in eps],
          [[x["depth"], g(x["gamma"], 5), x["note"]] + [g(v, 3) for v in x["step_ratio_vs_rodas5p"]] for x in lb])
    w("표의 수는 스칼라 선형 문제 y' = lambda y (lambda < 0)에서 스텝당 오차 |R(z) - exp(z)|가 eps를 처음 넘는 |z|의 비, 곧 RODAS5P 대비 필요한 스텝 수의 비다(모형). 깊이 이득 1.6(깊이 5), 1.33(깊이 6)과 비교한다.")
    w()
st = {k: v.get("stability") for k, v in EX.get("methods", {}).items() if v.get("stability")}
if st:
    table(["방법", "gamma", "z^6 결함", "z^7 결함", "R_hat(무한대)"],
          [[k, g(v["gamma"], 6), g(v["taylor_defects_z1_to_z8"][5]), g(v["taylor_defects_z1_to_z8"][6]), g(v["R_hat_infinity"], 3)]
           for k, v in sorted(st.items())])
    w("RODAS5P는 극이 7개인 R로 선형 차수 6을 갖는다. 깊이 5는 선형 차수 5에 묶이고, 깊이 6은 gamma = %s에서만 선형 차수 6이 되며 그때도 z^7 결함이 RODAS5P보다 훨씬 크다(위 표). 스칼라 선형 모형에서는 이 차이가 깊이 이득을 넘는다. 실제 문제에서는 비선형 오차 항과 섞이므로 7절의 비는 이 모형값보다 작다." % g(G6L, 5))
    w()
w("### 8.2 선도 오차 항")
w()
et = {k: v for k, v in EX.get("methods", {}).items() if v.get("error_terms")}
if et:
    table(["방법", "|E6| (차수 6 오차 계수 노름)", "RODAS5P 대비", "|ehat5| (추정기 계수 노름)", "RODAS5P 대비", "지수-1 잔차 노름 (주)", "지수-1 잔차 노름 (임베디드)",
           "PR 항 노름 q=2 / 3 / 4", "c_i 범위", "코드형 max|A|, max|C|"],
          [[k, g(v["error_terms"]["e6_norm"]), g(v.get("relative_to_rodas5p", {}).get("e6_norm"), 3), g(v["error_terms"]["ehat5_norm"]),
            g(v.get("relative_to_rodas5p", {}).get("ehat5_norm"), 3), g(v["error_terms"]["index1_main_norm"]),
            g(v["error_terms"]["index1_embedded_norm"]),
            " / ".join(g(v["error_terms"]["pr_main_by_q"][q]) for q in ("2", "3", "4")),
            "[%s, %s]" % (g(v["error_terms"]["c_min"], 3), g(v["error_terms"]["c_max"], 3)),
            "%s, %s" % (g(v.get("code_form", {}).get("max_abs_A"), 4), g(v.get("code_form", {}).get("max_abs_C"), 4))]
           for k, v in sorted(et.items())])
    w("- E6는 비강성 점근 영역의 주 방법 오차 계수, ehat5는 임베디드 방법의 차수 5 계수(오차 추정의 크기)다. RODAS5P의 지수-1 잔차와 PR q=2, 3 항은 설계상 0이다.")
    w("- c_i가 [0, 1]을 벗어나는 단은 스텝 구간 밖에서 우변을 평가한다. 선언한 목적함수는 이를 제어하지 않았다.")
    w()
w("### 8.2b Prothero-Robinson 조건과 극의 수 (구조적 사실)")
w()
w("N = beta - gamma I, u_2 = beta^2 1 - c^2/2라 하면 강성 정확 레벨 타블로에서 항등식 `b^T N^(L-1) u_2 = gamma^2 * b^T N^(L-1) 1`이 성립한다(N^(L-1)은 레벨 1에서 레벨 L로 가는 경로만 남기고 (u_2)_1 = gamma^2이기 때문). Prothero-Robinson 국소 오차 인자 W_2(z) = sum_k z^(k+1) b^T beta^k u_2가 항등적으로 0이려면 좌변이 0이어야 하고, 그러면 mu_(L-1) = b^T N^(L-1) 1 = 0, 곧 R(z) = P(z)/(1 - gamma z)^(L-1)로 극이 하나 줄어든다. 유도는 `NOTES.txt` 5번 항목, 아래는 수치 확인이다.")
w()
pid_rows = [[k, v["pr_pole_identity"]["levels"], g(v["pr_pole_identity"]["mu_last"]), g(v["pr_pole_identity"]["bT_N_u2"]),
             g(v["pr_pole_identity"]["gamma_squared_times_mu_last"]), g(v["pr_pole_identity"]["identity_defect"])]
            for k, v in sorted(EX.get("methods", {}).items()) if v.get("pr_pole_identity")]
if pid_rows:
    table(["방법", "레벨 수 L", "mu_(L-1)", "b^T N^(L-1) u_2", "gamma^2 mu_(L-1)", "항등식 잔차"], pid_rows)
w("- RODAS5P는 8단이지만 mu_7 = 0(설계)이라 극이 7개이고 W_2 = W_3 = 0이다(`logs/validation.jsonl`의 pr_terms). 같은 기제다.")
w("- 두 프로세서, 차수 5, R(무한대)=0에서의 귀결: 깊이 5에서 W_2 = 0을 정확히 요구하면 극이 4개가 되어 불가능하다(이론 S9의 L = 4 불가). 깊이 6에서는 극이 5개가 되어 gamma가 깊이 5의 값(%s)으로 고정되고 선형 차수 5(z^6 결함 %s)에 묶인다. 주 방법과 임베디드 방법 모두에 정확한 W_2 = 0을 주려면 레벨이 7개 이상 필요하다(깊이 이득 8/7). 이 문단은 유도와 위 수치 확인에 근거하며 기계 증명은 아니다." % (G5, g(Z6_D5)))
hp = R.get("exploratory_hard_pr_smoke")
if hp:
    w("- 스모크 시험(선언 범위 밖 시드, `%s`): gamma = 0.33414에서 PR 조건(q = 2, 3)을 경성으로 넣은 변형은 %d개 시작점 중 %d개만 실현 가능했다(경성 잔차 %s). 위 항등식과 일치하며 이 변형은 선언 실행 전에 버렸다." % (
        hp["log"], hp["starts"], hp["passed_double"], ", ".join(g(x) for x in hp["max_hard"])))
w()
w("### 8.3 강성 Prothero-Robinson 문제의 고정 스텝 차수")
w()
PRX = R.get("prothero_robinson_exploratory", {})
rows_ = []
for arm in sorted(PRX):
    for lam in sorted(PRX[arm], key=lambda s_: -float(s_)):
        e = PRX[arm][lam]
        rows_.append([arm, lam, " ".join(g(x) for x in e["errors"]), " ".join(g(x, 3) for x in e["pair_orders"])])
if rows_:
    table(["방법", "lambda", "오차 (스텝 수 10..320)", "쌍별 차수"], rows_)
    w("y' = lambda (y - phi(t)) + phi'(t), phi = sin t + 2, t in [0, 1]. 오차가 1e-14 근처면 반올림 수준이다.")
    w()
w("### 8.4 두 프로세서 스케줄")
w()
for layout in ("depth5", "depth6"):
    prim = V[layout].get("primary_candidate")
    sc = EX.get("methods", {}).get(prim, {}).get("schedule") if prim else None
    if sc:
        w("- `%s`: %d단, %d라운드. 라운드별 (프로세서 1, 프로세서 2): %s. 한 라운드의 두 단은 서로 독립이고 같은 LU를 쓴다. 체인 깊이 이득 8/%d = %s, 총 풀이 수는 RODAS5P의 %s배." % (
            prim, sc["stages"], sc["depth"], ", ".join("(%s, %s)" % (x["processor_1"], x["processor_2"] if x["processor_2"] else "-") for x in sc["schedule"]),
            sc["depth"], g(sc["depth_gain_vs_8"], 3), g(sc["total_solves_vs_8"], 3)))
w()
w("### 8.5 스텝 전체 속도 모형")
w()
ms = EX.get("modelled_step_speedup")
if ms:
    w("모형: `%s`. f_chain은 `cost_anatomy/AMDAHL.json`의 영역별 체인 비중(%d개 영역), 표시는 `REVIEW_OVERLAY.json`. 오버헤드 0, 완전한 2-프로세서 확장, 단당 비용 동일을 가정한 상한이며 측정이 아니다." % (ms["formula"], ms["regimes"]))
    w()
    rows_ = []
    for arm in sorted(ms["arms"], key=lambda a: (ms["arms"][a]["layout"], ms["arms"][a]["role"] != "primary", a)):
        ma = ms["arms"][arm]
        for pid, e in ma["problems"].items():
            qa = e.get("overlay_authoritative_regimes") or {}
            qall = e.get("all_regimes") or {}
            rows_.append([arm, ROLE_KO[ma["role"]], pid, g(e["attempt_ratio"], 4), g(e["chain_only_bound"], 3),
                          "%s / %s / %s" % (g(qa.get("min"), 3), g(qa.get("median"), 3), g(qa.get("max"), 3)),
                          "%s / %s" % (e["regimes_above_1_authoritative"], qa.get("count")),
                          "%s / %s / %s" % (g(qall.get("min"), 3), g(qall.get("median"), 3), g(qall.get("max"), 3)),
                          "%s / %s" % (e["regimes_above_1_all"], qall.get("count"))])
    table(["후보", "역할", "문제", "시도 비", "체인만의 상한 (8/L)/비", "권위 영역: 최소 / 중앙 / 최대", "1 초과 영역 수 (권위)",
           "전체 영역: 최소 / 중앙 / 최대", "1 초과 영역 수 (전체)"], rows_)
    w("1보다 작은 값은 두 프로세서를 써도 RODAS5P 순차 스텝보다 느리다는 뜻이다.")
    w()

# ------------------------------------------------------------------------------------------------ 9
w("## 9. 문헌 확인 (1차 출처, 신규성 주장 없음)")
w()
w("방법: %s" % LIT["method"])
w()
for e in LIT["entries"]:
    w("- %s. 접근: %s. 분류: %s." % (e["citation"], e["access"], e["class"]))
    for f in e["facts"]:
        w("  - %s" % f)
w()
for s in LIT["reading"]:
    w("- 읽기: %s" % s)
w()

# ----------------------------------------------------------------------------------------------- 10
w("## 10. RODAS5P와 비교해 얻는 것과 잃는 것")
w()
for layout in ("depth5", "depth6"):
    v = V[layout]
    prim = v.get("primary_candidate")
    if not prim:
        w("- %s: 후보 없음." % LAYOUT_KO[layout])
        continue
    m = EX.get("methods", {}).get(prim, {})
    rel = m.get("relative_to_rodas5p", {})
    stb = m.get("stability", {})
    ratios = v.get("attempt_ratios") or {}
    w("- %s, `%s`." % (LAYOUT_KO[layout], prim))
    w("  - 얻는 것: 단 풀이의 임계 경로가 8에서 %d로 줄어든다(두 프로세서, 같은 LU). 차수 6 오차 계수 노름은 RODAS5P의 %s배." % (DEPTH_ := {"depth5": 5, "depth6": 6}[layout], g(rel.get("e6_norm"), 3)))
    w("  - 잃는 것: 선형 오차(z^6 결함 %s, z^7 결함 %s; RODAS5P는 z^7에서 %s), 그리고 같은 오차에서의 시도 횟수 비 %s." % (
        g((stb.get("taylor_defects_z1_to_z8") or [None] * 8)[5]), g((stb.get("taylor_defects_z1_to_z8") or [None] * 8)[6]),
        g(((EX.get("methods", {}).get("RODAS5P", {}).get("stability") or {}).get("taylor_defects_z1_to_z8") or [None] * 8)[6]),
        ", ".join("%s %s" % (p, g(ratios.get(p), 4)) for p in ("ROBER", "VDPOL", "BURGERS"))))
    if layout == "depth5":
        w("  - 깊이 5에서는 임베디드 방법이 강성 정확일 수 없다(R_hat(무한대) = %s로 정규화). gamma가 고정이라 안정성 함수를 조정할 자유도가 없다." % g(stb.get("R_hat_infinity"), 3))
    else:
        w("  - 깊이 6은 단이 10개라 총 작업량이 1.25배다. 두 번째 프로세서가 없으면 그만큼 느리다.")
for arm in sorted(a for a, r in PSR.items() if r["role"] == "exploratory"):
    m = EX.get("methods", {}).get(arm, {})
    rel = m.get("relative_to_rodas5p", {})
    stb = m.get("stability", {})
    ar = R["payoff_screen"]["arms"][arm]
    w("- 탐색적 후보 `%s` (판정에 쓰이지 않음): 차수 6 오차 계수 노름은 RODAS5P의 %s배, z^6 결함 %s, 시도 횟수 비 %s." % (
        arm, g(rel.get("e6_norm"), 3), g((stb.get("taylor_defects_z1_to_z8") or [None] * 8)[5]),
        ", ".join("%s %s" % (p, g(ar.get(p, {}).get("ratio"), 4)) for p in ("ROBER", "VDPOL", "BURGERS"))))
w()

# ----------------------------------------------------------------------------------------------- 11
w("## 11. 이런 방법의 자격 검증에 필요한 것")
w()
w("이 노드의 스크린은 Python 프로토타입의 스텝 수 비교다. 채택 여부를 다루려면 적어도 다음이 필요하다.")
w()
w("1. 조건 집합의 완결: 지수-1 조건(주, 임베디드)과 Prothero-Robinson 조건을 연성 목표가 아니라 정확히 만족시키고, RODAS5P가 가진 조밀 출력 계수(H 행렬)와 그 차수 조건을 새로 구성한다. 여기의 후보에는 조밀 출력이 없다.")
w("2. 선형 오차와 Prothero-Robinson 조건의 해결 또는 수용: 8.1절과 8.2b절의 한계는 레이아웃의 성질이다. RODAS5P와 같은 극 수(7개)와 정확한 W_2 = 0을 함께 가지려면 레벨이 8개 필요하므로 깊이 이득이 없다. 레벨 7개(깊이 이득 8/7, 12단)로는 극 6개에 정확한 W_2 = 0, 또는 극 7개에 부정확한 W_2 중 하나만 가능하다.")
w("3. Rust 구현과 비트 수준 결정성: 레벨 안의 두 단을 두 스레드에서 풀 때 결과가 스레드 수와 무관해야 하고, 단당 비용이 작은 문제에서는 동기화 비용이 이득을 지운다(V04의 체인 비중과 단당 시간 참고).")
w("4. 정합 달성 오차에서의 벽시계 측정: 고정 코어, 교차 배치, 2회 예열 + 7회 반복, 부하 기록. 이 보고서의 속도 수치는 모형이다.")
w("5. 새 문제 집합에서의 확인(이미 소비된 HIRES, Oregonator, Brusselator N=32는 holdout으로 쓸 수 없다), 외부 비교자(CVODE, Radau, Julia Rodas5P), 부정확한 야코비안과 지연 야코비안에서의 거동(W-차수), 스텝 제어기와의 상호작용(추정기 보정).")
w("6. 보호 경로와의 분리: 새 타블로는 새 방법 id를 갖고 기본 라우팅에 들어가지 않는다. G0와 허용오차, 제어기 상수는 그대로다.")
w()

# ----------------------------------------------------------------------------------------------- 12
w("## 12. 한계와 미해결")
w()
w("- 사전선언을 %d번 수정했다(2절). 수정은 최적화기 내부, 차수 검사의 반올림 가드, 깊이 6의 추가 정식화에 한정되지만, 스모크 실행을 본 뒤의 수정이다." % len(PRE["amendments"]))
w("- 탐색은 선언한 목적함수(가중치 고정)와 예산 안의 결과다. 다른 가중치, c_i 범위 제약, 더 긴 연마, 다른 임베디드 설계는 시험하지 않았다. 후보의 연성 조건(지수-1, PR)은 정확히 0이 아니다(8.2절).")
w("- 계수 한계 100은 k-형 계수(gamma, alpha, gamma_ij, b, bhat)에 적용했다. 코드형 A, C의 크기는 8.2절에 참고로 적었다(보호된 RODAS5P의 max|C|도 100을 넘는다).")
w("- pay-off 문제 3개와 개발 문제 2개는 작은 계다. 프로토타입은 조밀 LU를 쓰고, 출력 시점 처리와 예측형 제어기 종류는 재현하지 않았다.")
w("- 기준해의 독립성: Radau(scipy)와 프로토타입은 다른 코드이지만 u_ref의 한 성분은 RODAS5P 프로토타입 자신과의 차이다.")
w("- 문헌 확인은 짧았고 Voss, Chen-Liu의 본문은 읽지 못했다.")
fallbacks = [f for f in ("D5G", "D6F") if f in R["search"]]
w("- 깊이 5의 임베디드 방법 정규화(R_hat(무한대) = 0.2)와 c_s = 1, 깊이 6의 c_9 = 1은 동결 행에 없는 설계 선택이다. 대체 정식화(D5G, D6F)는 주 정식화가 후보를 내지 못할 때만 실행하기로 선언했고, %s." % (
    "실행 기록이 4절에 있다: " + ", ".join(fallbacks) if fallbacks else "두 레이아웃 모두 주 정식화에서 후보가 나왔으므로 실행하지 않았다"))
w("- 선택 규칙의 민감도: 개발 문제(HIRES, OREGO)의 순위와 pay-off 문제의 순위가 다르다. 깊이 6에서 개발 2위였던 후보가 pay-off 문제 두 개에서 1순위 후보보다 낫다(7절의 탐색적 행). 이 차이는 문제 의존성이 크다는 뜻이며, 그 후보를 사후에 1순위로 바꾸지 않았다. 확인하려면 새 문제에서 사전등록된 확증 실행이 필요하다.")
w("- 탐색적 후보 `D6T_r07`은 선언한 규칙으로 BRUSS_ODE의 관측 차수가 4.8에 못 미치고(6절: 쌍별 차수가 불규칙하고, 가드 아래로 빠진 마지막 쌍은 표에 없다), 코드형 계수가 크다(8.2절). gamma = 0.19는 A-안정 창의 아래 끝(0.1839)에 가깝고 임베디드 방법은 허수축에서 |R_hat| > 1인 구간이 있다(5절 로그의 embedded_grid_max_abs_R_minus_1).")
w("- 프로토타입의 제어기는 추정기 차수 5를 두 방법에 똑같이 쓴다. 후보의 추정기 크기(8.2절의 ehat5)는 RODAS5P와 다르므로 같은 허용오차에서의 달성 오차가 다르다. 비교는 달성 오차를 맞춰서 했다.")
w()

# ----------------------------------------------------------------------------------------------- 13
w("## 13. 재현")
w()
w("```")
w("cd research/production_loop_20261116/level_tableau     # 모든 명령은 taskset -c 0-5,12-17, OMP_NUM_THREADS=1")
w("python3 -I run_validation.py > logs/validation.jsonl")
w("python3 -I search.py D6T --starts 2000 --workers 4 > logs/search_D6T.jsonl      # D5F, D6L도 같은 방식")
w("python3 -I search.py D6T --polish-top logs/search_D6T.jsonl > logs/polish_D6T.jsonl")
w("python3 -I search.py D6T --continuation logs/polish_D6T.jsonl > logs/continuation_D6T.jsonl")
w("python3 -I refine.py D6T logs/polish_D6T.jsonl logs/continuation_D6T.jsonl > logs/refine_D6T.jsonl")
w("python3 -I verify_candidate.py candidates/D6T_r0*.json > logs/verify_D6T.jsonl")
w("python3 -I run_payoff.py --label dev --problems HIRES,OREGO --arms RODAS5P candidates/... > logs/payoff_dev_....jsonl")
w("python3 -I run_order.py candidates/<1순위>.json > logs/order_<1순위>.jsonl")
w("python3 -I run_payoff.py --label screen --problems ROBER,VDPOL,BURGERS --arms RODAS5P candidates/<1순위>.json > logs/payoff_screen_....jsonl")
w("python3 -I run_exploratory.py RODAS5P candidates/<1순위>.json > logs/exploratory.jsonl")
w("python3 -I parse_results.py && python3 -I render_report.py")
w("```")
w()
w("입력 로그의 SHA-256은 `RESULTS.json`의 `inputs_sha256`에 있다.")

with open(os.path.join(HERE, "REPORT_KO.md"), "w") as fh:
    fh.write("\n".join(out) + "\n")
print("REPORT_KO.md written, %d lines" % len(out))
