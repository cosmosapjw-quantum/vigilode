#!/usr/bin/env python3
"""Renders REPORT_KO.md of loop L1 nodes V19 and V21 from RESULTS.json and RESULTS_V21.json.

Every number in the report is read from the two result files; this script holds text only.
"""
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))


def load(name):
    path = os.path.join(HERE, name)
    if not os.path.exists(path):
        return None
    with open(path) as f:
        return json.load(f)


def f3(x):
    return "-" if x is None else f"{x:.3f}"


def f1(x):
    return "-" if x is None else f"{x:.1f}"


def sci(x):
    if x is None:
        return "-"
    if isinstance(x, str):
        return x
    return f"{x:.1e}"


def first_complete(measurements, n):
    cells = sorted((m for m in measurements if m["n"] == n and m["complete"]), key=lambda m: m["index"])
    return cells[0] if cells else None


def main():
    v19, v21 = load("RESULTS.json"), load("RESULTS_V21.json")
    out = []
    w = out.append
    w("# V19·V21: banded stage solve kernel 결과 (loop L1)")
    w("")
    w("이 문서는 `build_report.py`가 `RESULTS.json`과 `RESULTS_V21.json`에서 생성한다. 숫자는 모두 두 파일에서 읽는다.")
    w("")
    w("**정정 고지 (2026-11-23, 독립 리뷰 `REVIEW.json`)**. 이전 판의 2절은 틀렸다. (1) V19의 해석: '의존 사슬 가설이 반증됐고 kernel은 명령어 수에 묶여 있다'고 썼으나, 판정 측정에서 대입 순서만 바꾼 arm B는 같은 명령어 수로 cycle이 절반 가까이 줄었다. KILL 판정은 유효성 규칙에서 나온 것이고 가설의 반증이 아니다. (2) 2절의 표가 판정에 쓴 측정이 아니라 바쁜 host의 첫 측정이었고, 분할 arm이 더 느리다는 문장은 판정 측정과 맞지 않았다. (3) BD는 선언 밖 사후 arm이다. (4) 결과 파일에 없는 숫자를 본문에 적었던 곳을 지웠다. (5) stage b의 측정은 '전체 실행'이 아니라 시도 2-40의 시도당 명령어다. 동결 판정(KILL, PASS-I, PASS-C, PASS)은 모두 그대로이며 리뷰가 원자료에서 다시 계산해 일치를 확인했다.")
    w("")
    w("용어: **stage pattern** = RODAS5P 시도 1회의 선형대수(분해 1회 + solve 8회). **A0** = 보호 client `BandedStageLu`(Indexed kernel, facade의 banded 경로가 쓰는 kernel). **cycles** = 고정 core에서 잰 user-mode CPU cycle. **instructions** = user-mode 명령어 수(결정적, 호스트 부하와 무관).")
    w("")

    # ---------------------------------------------------------------- V21
    if v21:
        a1, a2, val = v21["stage_a1"], v21["stage_a2"], v21["validity"]
        w("## 1. V21: 같은 bit, 더 적은 명령어")
        w("")
        w("### 1.1 판정")
        w("")
        w("| 단계 | 내용 | 명령어 판정 (PASS-I) | cycle 판정 (PASS-C) |")
        w("|---|---|---|---|")
        w(f"| a1 | 좁은 band(lower = upper = 2)용 tight kernel TA, kernel 수준 | {a1['PASS_I']} | {a1['PASS_C']} (authoritative였다면: {a1.get('PASS_C_if_the_cells_were_authoritative', '-')}) |")
        w(f"| a2 | 넓은 band에서 보호 Slices kernel 대 Indexed kernel, 전체 실행 | {a2['PASS_I']} | {a2['PASS_C']} (authoritative였다면: {a2.get('PASS_C_if_the_cells_were_authoritative', '-')}) |")
        w("")
        w(f"기준: a1 PASS-I는 TA 명령어가 A0의 {a1['thresholds']['PASS_I']} 이하(n = 1e4, 1e5 모두), PASS-C는 cycle {a1['thresholds']['PASS_C']} 이하. a2 PASS-I는 Slices 명령어가 Indexed의 {a2['thresholds']['PASS_I']} 이하(넓은 band 3문제 모두), PASS-C는 cycle {a2['thresholds']['PASS_C']} 이하. cycle 판정은 호스트 1분 부하 8 이하이고 arm 내 편차 10% 이하일 때만 authoritative이고, 아니면 NOT_DECIDABLE(숫자는 diagnostic)이다.")
        w("")
        w(f"유효성: TA는 lower = upper = 2인 검증 사례 {val['cases_with_lower_upper_2']}건 중 {val['TA_bit_identical_cases']}건에서 보호 kernel과 bit 동일. TD(역대입에서 최신 미지수를 마지막에 빼고 나눗셈 유지)는 V2·V3 통과: {val['TD_valid']}.")
        w("")
        w("### 1.2 a1: 좁은 band, kernel 수준 (stage pattern당)")
        w("")
        for cell in sorted(v21["kernel_level_measurements"], key=lambda c: (c["n"], c["index"])):
            if not cell["complete"]:
                continue
            w(f"n = {cell['n']} (`{cell['file']}`, 부하 {cell['load_before']} → {cell['load_after']}, cycle authoritative: {cell['cycles_authoritative_for_TA']})")
            w("")
            w("| arm | 명령어 비 (A0 = 1) | cycle 비 (A0 = 1) | 행당 명령어: 분해 | 행당 명령어: solve 1회 | 행당 cycle: 분해 | 행당 cycle: solve 1회 | cycle 편차 |")
            w("|---|---|---|---|---|---|---|---|")
            for arm in ("A0", "A1", "TA", "TD", "TS", "BD", "B3"):
                e = cell["arms"].get(arm)
                if not e:
                    continue
                w(f"| {arm} | {f3(e.get('instructions_ratio_to_A0'))} | {f3(e.get('cycles_ratio_to_A0'))} | {f1(e['instructions_per_row_factorisation'])} | {f1(e['instructions_per_row_solve'])} | {f1(e['cycles_per_row_factorisation'])} | {f1(e['cycles_per_row_solve'])} | {f3(e['cycles_spread'])} |")
            w("")
        w("arm 설명: TA = 보호 kernel과 같은 연산을 같은 순서로, 행 offset 고정·값 carry로 작성(bit 동일). TD = TA에서 역대입 순서만 변경(최신 미지수 마지막), 나눗셈 유지. TS = 역수로 미리 scale한 U 행 사용(V19 동결 기준 V2 미달, 참고용). A1 = 보호 kernel 순서의 slice 구현(bit 동일). BD, B3 = V19의 탐색 arm.")
        w("")
        w("### 1.3 a2: 넓은 band, 보호 banded driver 전체 실행 (시도 1회당)")
        w("")
        w("| 문제 | n | band | 결과 동일(횟수·연산 수·상태 hash) | Slices 명령어 비 (Indexed = 1) | Slices cycle 비 | cycle 편차 Indexed / Slices | cycle 표시 |")
        w("|---|---|---|---|---|---|---|---|")
        for cell in sorted(v21["whole_run_measurements"], key=lambda c: (c["family"], c["size"], c["index"])):
            if not cell["complete"] or "slices" not in cell["kernels"]:
                continue
            sl, ix = cell["kernels"]["slices"], cell["kernels"]["indexed"]
            label = "authoritative" if cell["cycles_authoritative"] else "diagnostic"
            w(f"| {cell['family']} {cell['size']} | {cell['n']} | {cell['band']} | {cell['counts_operations_and_state_hash_equal_in_every_run']} | {f3(sl['instructions_ratio_to_indexed'])} | {f3(sl['cycles_ratio_to_indexed'])} | {f3(ix['cycles_spread'])} / {f3(sl['cycles_spread'])} | {label} |")
        w("")
        w("facade의 banded 경로(`crates/vigilode/src/solve.rs`)와 client kit의 기준 client(`BandedStageLu`)는 Indexed kernel을 쓴다. Slices kernel은 같은 보호 파일에 이미 있고 bit 동일로 문서화돼 있으며, 이 측정의 모든 실행에서 시도·수락·거부 횟수, 센 band 연산 수, 반환 상태 hash가 같았다(표의 '결과 동일' 열).")
        w("")

        sb = v21.get("stage_b")
        if sb and any(sb["per_cell"].values()):
            w("### 1.4 stage b: 선택형 client `BandedTightStageLu` (보호 banded driver와 bit 동일)")
            w("")
            w(f"명령어 측정 판정: **{sb['instruction_measure']}** (기준: client 실행의 시도당 명령어가 보호 banded driver의 {sb['thresholds']['PASS']} 이하, n = 1e4와 1e5 모두; {sb['thresholds']['KILL']} 초과면 KILL). 측정값은 시도 40회 실행과 1회 실행의 차이를 시도 수 차이로 나눈 것(시도 2-40의 시도당 명령어)이며 한 허용오차의 Brusselator에서 잰 것이다.")
            w("")
            w("| cell | n | 결과 동일(시도·수락·거부 횟수, 상태 hash) | 명령어 비 (보호 driver = 1) | cycle 비 | cycle 표시 |")
            w("|---|---|---|---|---|---|")
            for key, c in sb["per_cell"].items():
                if c:
                    w(f"| {key} | {c['n']} | {c['valid']} | {f3(c['instructions_ratio'])} | {f3(c['cycles_ratio'])} | {c['cycles_label']} |")
            w("")
            w("identity test(`crates/rodas5p-integrators/tests/stage_clients_banded_tight_identity.rs`)는 보호 banded driver와 client driver를 같은 문제에 돌려 출력 상태 bit, 시도·수락·거부 횟수, work counter 전체를 비교한다. 사용법: `integrate_rodas5p_fast_client_observed_with_atol(problem, Box::new(BandedTightStageLu::new(n, &band)?), ...)`. 기본 경로는 바뀌지 않는다.")
            w("")

    # ---------------------------------------------------------------- V19
    if v19:
        val, ver = v19["validity"], v19["verdicts"]
        w("## 2. V19: 의존 사슬 가설 (동결 판정)")
        w("")
        if ver:
            w(f"- stage a 판정: **{ver['stage_a']}** (cell이 authoritative였다면: {ver['stage_a_if_the_cells_were_authoritative']}).")
            w(f"- 둘째 질문(분할 유지 여부): {ver['second_question']} (authoritative였다면: {ver['second_question_if_authoritative']}).")
            th = ver["thresholds"]
            w(f"- 기준: 최선의 유효 arm이 A0의 {th['pass_ratio_to_A0']} 이하이고 A1의 {th['pass_ratio_to_A1']} 이하면 PASS, 모든 유효 arm이 A1의 {th['kill_ratio_to_A1']} 초과면 KILL (n = 1e4, 1e5 모두).")
        declared = ("B", "BV", "C2", "C4", "C8", "CV2", "CV4", "CV8")  # arms of PREDECLARED.json
        valid = [a for a in v19["valid_arms"] if a not in ("A0", "A1")]
        w(f"- 선언된 arm 중 유효(동결 기준 V2·V3): {', '.join(a for a in valid if a in declared)}. 선언 밖 arm 중 유효: {', '.join(a for a in valid if a not in declared)}. V1(A1이 A0와 bit 동일): {val['V1']['pass']} ({val['V1']['cases']}건).")
        w("")
        w("| arm | V2 (대상 계열, eta ≤ 1e-14) | V2 최악 eta | V3 (무음 실패 없음) | V3 오류 반환 수 | 유효 |")
        w("|---|---|---|---|---|---|")
        for arm in ("A0", "A1", "B", "BV", "C2", "C4", "C8", "CV2", "CV4", "CV8", "BD", "B2", "B3", "C16", "CV16"):
            e = val["arms"].get(arm)
            if not e:
                continue
            nerr = sum(len(x) for x in e["V3"]["error_returns"].values())
            w(f"| {arm} | {e['V2']['pass']} | {sci(e['V2']['worst_eta'])} | {e['V3']['pass']} | {nerr} | {e['valid']} |")
        w("")
        w("B 계열(역수 곱)과 C2, C4는 n = 1e5, 1/(h gamma) ≤ 1인 극단 사례에서 backward error가 동결 기준 1e-14를 넘어 V2에서 탈락한다. 기준은 바꾸지 않았다. B2, B3, C16, CV16은 사전등록에 탐색용으로 적은 arm이다. BD(나눗셈 유지)는 사전등록에 없고, 유효성 실행과 첫 timing 실행 사이에 추가한 사후 arm이다.")
        w("")
        by_file = {m["file"]: m for m in v19["measurements"]}
        deciding = {}
        for n in (10000, 100000):
            cell = by_file.get(ver["per_size"][str(n)]["measurement"]) if ver else None
            if not cell:
                continue
            deciding[n] = cell
            others = sorted(m["file"] for m in v19["measurements"] if m["n"] == n and m["file"] != cell["file"])
            w(f"n = {n}, 판정에 쓴 측정 (`{cell['file']}`, 부하 {cell['load_before']} → {cell['load_after']}, 부하 기준 충족: {cell['load_ok']}). 같은 크기의 다른 측정: {', '.join('`' + o + '`' for o in others) or '없음'}.")
            w("")
            w("| arm | cycle 비 (A0 = 1) | cycle 비 (A1 = 1) | 명령어 비 (A1 = 1) | solve cycle 비 (A0 = 1) | 분해 cycle 비 (A0 = 1) | 편차 |")
            w("|---|---|---|---|---|---|---|")
            base_i = cell["arms"]["A1"]["instructions_per_pattern"]
            for arm in ("A0", "A1", "B", "BV", "C2", "C4", "C8", "CV2", "CV4", "CV8", "BD", "B2", "B3", "C16", "CV16"):
                e = cell["arms"].get(arm)
                if not e:
                    continue
                w(f"| {arm} | {f3(e.get('ratio_to_A0'))} | {f3(e.get('ratio_to_A1'))} | {f3(e['instructions_per_pattern'] / base_i)} | {f3(e.get('solve_ratio_to_A0'))} | {f3(e.get('factorisation_ratio_to_A0'))} | {f3(e['spread'])} |")
            w("")
        if len(deciding) == 2:
            def rng(arms, key):
                values = [deciding[n]["arms"][a][key] for n in deciding for a in arms if a in deciding[n]["arms"]]
                return f"{min(values):.3f}-{max(values):.3f}"
            b_c = [deciding[n]["arms"]["B"]["ratio_to_A1"] for n in (10000, 100000)]
            b_i = [deciding[n]["arms"]["B"]["instructions_per_pattern"] / deciding[n]["arms"]["A1"]["instructions_per_pattern"] for n in (10000, 100000)]
            c8 = [ver["per_size"][str(n)]["ratio_to_A1"] for n in (10000, 100000)]
            w(f"해석 (독립 리뷰 뒤 정정). KILL은 유효성 규칙에서 나온다: 선언된 arm 중 V2·V3를 통과한 것은 C8과 CV8뿐이고, 둘 중 빠른 C8이 A1의 {c8[0]:.3f}(n = 1e4)과 {c8[1]:.3f}(n = 1e5)로 kill 수준 {ver['thresholds']['kill_ratio_to_A1']} 위다. "
              f"가설(대입이 의존 사슬에 묶여 있다)이 반증된 것은 아니다. 대입 순서만 바꾼 arm B는 stage pattern당 명령어가 A1의 {b_i[0]:.3f}배와 {b_i[1]:.3f}배인데 cycle은 A1의 {b_c[0]:.3f}과 {b_c[1]:.3f}이다. 즉 의존 사슬은 실제 비용이다. "
              "B는 backward error가 기준 1e-14를 넘어(위 표) 유효 arm이 아니었고, 그래서 판정에 들어가지 못했다. "
              f"분할 arm C2·C4·C8은 판정 측정에서 A1의 {rng(('C2', 'C4', 'C8'), 'ratio_to_A1')}로 A1보다 빠르지만 kill 수준에는 못 미친다. 검증을 붙인 CV arm은 A1의 {rng(('CV2', 'CV4', 'CV8'), 'ratio_to_A1')}로 더 느리다. "
              "명령어 수(V21의 tight loop)와 의존 사슬(대입 순서)은 solve 비용에서 따로 작용하는 두 요인이다.")
            w("")

    w("## 3. 의미와 남은 일")
    w("")
    w("- 이 결과는 kernel과 시도 단위의 비용이다. 방법(RODAS5P tableau, controller, 허용오차)과 결과 bit는 그대로다.")
    narrow = None
    if v21:
        for m in v21.get("whole_run_measurements", []):
            if m.get("family") == "bruss1d" and m.get("cycles_authoritative") and "slices" in m.get("kernels", {}):
                narrow = m["kernels"]["slices"]
    w("- facade의 banded 경로 kernel은 소유자 결정 OD-L1-9에 따라 node V22(`../banded_kernel_route/`)에서 띠 폭에 따라 고른다. 근거: Slices는 넓은 띠에서 이득이고(1.3절)"
      + (f", 띠 2에서는 명령어가 Indexed의 {narrow['instructions_ratio_to_indexed']:.3f}배인데 cycle은 {narrow['cycles_ratio_to_indexed']:.3f}배다(authoritative)." if narrow else ".")
      + " 보호 파일의 kernel 자체는 바꾸지 않았다.")
    w("- cycle 판정이 NOT_DECIDABLE인 칸은 조용한 호스트에서 같은 명령으로 다시 재면 된다(재측정은 cell당 2회까지, 기록은 모두 보존).")
    w("- 탐색 기록(판정 아님): 같은 코드를 `-C target-cpu=x86-64-v3`로 build하면 넓은 band 문제에서 Slices kernel의 명령어가 더 줄고 상태 hash는 같다. 수치는 `logs/xflag_v3_instructions.log`에 있다(결과 파일에는 없다).")
    w("- 다음 병목: tight kernel 뒤에는 stage 산술(보호 driver의 선형결합 loop)과 RHS가 남는다. 이 부분은 보호 driver 안에 있어 client로는 바꿀 수 없다.")
    w("- 후속 후보(판정 아님, 리뷰 자료): 대입 순서를 바꾸고 나눗셈을 유지한 tight kernel(TD)은 bit 동일은 아니지만 backward error 기준을 지키고 cycle이 더 적다(1.2절 표의 TD 행). bit 동일을 요구하지 않는 선택형 client로 다룰 수 있다.")
    w("")
    with open(os.path.join(HERE, "REPORT_KO.md"), "w", encoding="utf-8") as f:
        f.write("\n".join(out))
    print("REPORT_KO.md written,", len(out), "lines")


if __name__ == "__main__":
    main()
