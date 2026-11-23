#!/usr/bin/env python3
"""Renders REPORT_KO.md of loop L1 node V18_DENSE_LU_CLIENT from RESULTS.json.

Usage (from the worktree root):

    python3 -I research/production_loop_20261116/dense_lu_client/make_report.py

Every number in the report is read from RESULTS.json (built by parse_results.py from the raw
logs). The sentences around the numbers are fixed text; a sentence that depends on a result is
chosen by the verdict fields of RESULTS.json.
"""

import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
R = json.loads((HERE / "RESULTS.json").read_text())
V = R["verdicts"]
T = R["frozen_thresholds"]
SPANS = {cell: spec["t_span"] for cell, spec in json.loads((HERE / "PREDECLARED.json").read_text())["cells"].items()}


def f(value, digits=3):
    if value is None:
        return "-"
    if isinstance(value, bool):
        return "예" if value else "아니오"
    if isinstance(value, int):
        return str(value)
    return f"{value:.{digits}f}"


def e(value):
    return "-" if value is None else f"{value:.3e}"


def ms(ns):
    return "-" if ns is None else f"{ns / 1.0e6:.2f}"


def table(header, rows):
    out = ["| " + " | ".join(header) + " |", "|" + "|".join("---" for _ in header) + "|"]
    out += ["| " + " | ".join(str(c) for c in row) + " |" for row in rows]
    return "\n".join(out)


def verdict_word(value):
    return {
        "PASS": "PASS",
        "FAIL": "FAIL",
        "KILL": "KILL",
        "NOT_RUN": "NOT_RUN",
        "NOT_DECIDABLE": "NOT_DECIDABLE (타이밍 품질 규칙 미달)",
        "NOT_TRIGGERED": "해당 없음 (kill 조건 불충족)",
        "INCOMPLETE": "INCOMPLETE (로그 부족)",
    }.get(value, str(value))


lines = []
add = lines.append

add("# V18 조밀 LU 클라이언트: 인수분해 저장소 재사용과 선택형 스레드 인수분해")
add("")
add("- 노드: `V18_DENSE_LU_CLIENT` (사전등록 부록 A2), 루프 L1 wave 2")
add("- 이 문서는 `make_report.py`가 `RESULTS.json`에서 만든다. `RESULTS.json`은 `parse_results.py`가 `logs/`의 원시 출력만으로 만든다.")
add("- 사전 선언: `PREDECLARED.json` (첫 측정 실행 전에 커밋). 동결 임계값은 바꾸지 않았다.")
add("")

# ---------------------------------------------------------------------------------------------
add("## 1. 동결 판별식별 판정")
add("")
parts = V["reuse_parts"]
wall = R["reuse_wall"]
rows = [
    ["재사용 arm: 비트 동일성 (클라이언트 키트 집합 + faer 경로 n = 128, 512, 2048)", verdict_word(parts["identity"])],
    ["재사용 arm: 워밍업 후 시도당 힙 할당 0", verdict_word(parts["allocations"])],
    [f"재사용 arm: 워밍업 후 시도당 minor page fault {f(T['reuse_minor_faults_per_attempt_max'], 0)} 이하", verdict_word(parts["page_faults"])],
    [f"재사용 arm: n = 512 벽시계 {f(T['reuse_wall_pass'], 2)}배 이상, 세 힙 배치 변형 모두", verdict_word(parts["wall"])],
    ["재사용 arm PASS 기준 (위 네 항목 모두)", verdict_word(V["reuse_pass_criterion"])],
    [f"재사용 arm KILL 기준 (비트 동일성 불가, 또는 세 변형 모두 {f(T['reuse_wall_kill'], 2)}배 미만)", verdict_word(V["reuse_kill_criterion"])],
    ["스레드 arm, 계약 문구 그대로 (faer 인수분해 호출의 병렬 인자)", verdict_word(V["threaded_faer_par_argument"])],
    [f"스레드 arm, 클라이언트 측 분할 구현에 동결 임계값 적용: PASS 기준 (6 워커, n >= 512, {f(T['threaded_pass'], 1)}배 이상)", verdict_word(V["threaded_client_side_pass_criterion"])],
    [f"같은 구현: KILL 기준 (n = 2048, 6 워커, {f(T['threaded_kill'], 2)}배 미만)", verdict_word(V["threaded_client_side_kill_criterion"])],
]
add(table(["판별식", "판정"], rows))
add("")
add("스레드 arm에는 두 줄이 있다. faer 자체의 스레드 인수분해는 보호 대상 매니페스트를 바꾸지 않고는 쓸 수 없어서 NOT_RUN이다 (6절). "
    "매니페스트 변경이 필요 없는 클라이언트 측 구현은 만들어서 동결 임계값 그대로 재었다. "
    "둘째 줄을 부록 A2의 판정으로 볼지 탐색 결과로 볼지는 오케스트레이터가 정한다. 이 노드는 정하지 않는다.")
add("")

# ---------------------------------------------------------------------------------------------
add("## 2. 무엇을 만들었나, 사용자가 무엇을 켜야 하나")
add("")
add("- 코드: `crates/rodas5p-integrators/src/stage_clients/dense_lu_reuse.rs`, `dense_lu_reuse/threaded.rs`. 기본 경로는 건드리지 않았다. 아무것도 기본으로 선택되지 않는다.")
add("- 재사용 arm을 쓰려면: `DenseLuReuseStageClient::for_protected_run(&problem, t0, &y0)?` (또는 `new(n, DenseLuMode::Faer)`)로 클라이언트를 만들어 "
    "`integrate_rodas5p_fast_client_observed_with_atol`에 넘긴다. 보호 조밀 드라이버와 같은 정책으로 LU 모드를 고르고, faer 모드에서는 인수분해가 클라이언트 소유 저장소에 쓴다.")
add("- 스레드 arm을 쓰려면: 위 클라이언트에 `.with_factor_workers(k)?` (k = 2 이상)를 붙인다. 풀은 클라이언트가 소유하고, 전역 설정은 건드리지 않는다. k = 1은 재사용 arm과 같다.")
add("- 보호 경로가 한 번의 인수분해에서 하는 일 (`rodas5p_core::LuFactorization::new` -> faer 0.24.4): 열 우선 행렬 새로 할당, 그 사본 할당, 치환 배열 2개, 스크래치 1개, `lu_in_place`(순차), U용 세 번째 행렬 할당과 삼각 복사. "
    "재사용 arm은 같은 `lu_in_place`를 같은 인자로, 같은 배치의 자기 소유 `Mat`에 대해 부른다. W는 열 우선 행렬에 바로 조립하고 L/U 분리는 하지 않는다 (faer의 삼각 풀이는 반대쪽 삼각을 읽지 않는다).")
add("")

# ---------------------------------------------------------------------------------------------
add("## 3. 측정 1: 비트 동일성")
add("")
identity = R["identity"]
s = identity["summary"]
add(table(
    ["출처", "행 수", "비트 동일", "비고"],
    [
        ["테스트: 클라이언트 키트 집합 (Robertson, HIRES, Brusselator n = 64, 200, 평균장 n = 200; rtol 3개)", s["client_kit_rows"], s["client_kit_rows_identical"], f"비교한 수락 상태 {s['client_kit_accepted_states_compared']}개"],
        ["테스트: faer 경로, 재사용 arm", s["faer_path_test_rows_reuse"], s["faer_path_test_rows_reuse_identical"], f"n = {s['faer_path_test_sizes']}"],
        ["러너: 재사용 arm", s["runner_rows_reuse"], s["runner_rows_reuse_identical"], f"n = {s['runner_sizes']}, 수락 상태 {s['runner_accepted_states_compared_reuse']}개"],
        ["러너: 키트 faer 클라이언트", s["runner_rows_kit"], s["runner_rows_kit_identical"], ""],
        ["러너: 스레드 arm (2, 4, 6 워커; 보고만)", s["runner_rows_threaded"], s["runner_rows_threaded_identical"], f"끝 상태 최대 차이 {e(s['threaded_largest_end_state_difference'])}"],
    ],
))
add("")
tb = identity["test_binary"]
tb2 = identity["test_binary_n2048"]
add(f"- 테스트 바이너리: `{tb.get('log')}` {'; '.join(tb.get('result_lines', [])) or tb.get('status')}; `{tb2.get('log')}` {'; '.join(tb2.get('result_lines', [])) or tb2.get('status')}.")
add("- 비교한 양: 시도 수, 수락/기각 수, 야코비안 재사용 수, `WorkCounters`의 모든 필드, 수락 상태의 개수와 지문(t와 y의 비트), 끝 상태 지문. 테스트에서는 출력 시각과 상태 전부도 비교한다.")
add("- 커널 수준: `factors_permutation_and_solves_equal_the_protected_kernel` 테스트가 행 교환이 일어나는 행렬에서 결합 인자, 치환, 풀이를 `LuFactorization`과 비트 단위로 비교한다 "
    f"(상태: {tb.get('tests', {}).get('factors_permutation_and_solves_equal_the_protected_kernel', '-')}).")
add(f"- 행 교환이 실제로 일어난 재사용 행(러너): {s['runner_reuse_rows_with_row_interchanges']}개, 마지막 인수분해에서 옮겨진 행의 최대 수 {s['largest_rows_moved_by_last_factorization']}. 평균장 Brusselator는 행 교환이 없고 skewnet은 있다.")
add(f"- 스레드 인수분해의 인자가 순차 인자와 비트 동일한가 (테스트, n = 65..512, 2/4/6 워커): {s['threaded_kernel_rows_equal_sequential_bits']} / {s['threaded_kernel_rows']} 행.")
if identity["first_differing_reuse_row"]:
    row = identity["first_differing_reuse_row"]
    add(f"- 처음 달라진 재사용 행: `{row['log']}`: {row['first_difference']}")
n2048 = [r for r in identity["runner_rows"] if r.get("status") == "RUN" and r["n"] == 2048 and r["arm"] == "reuse"]
if n2048:
    add("")
    add("n = 2048 (러너, 재사용 arm):")
    add("")
    add(table(
        ["셀", "rtol", "완주", "시도", "수락 상태", "비트 동일", "옮겨진 행"],
        [[r["cell"], f"{r['rtol']:.0e}", f(r["complete"]), r["attempts_protected"], r["accepted_states_compared"], f(r["bit_identical"]), r["rows_moved_by_last_factorization"]] for r in n2048],
    ))
    add("")
    add("평균장 n = 2048은 시도 40회에서 멈춘 실행을 마지막 수락 상태에서 비교한다 (사전 선언).")
add("")

# ---------------------------------------------------------------------------------------------
add("## 4. 측정 2: 힙 할당과 page fault")
add("")
alloc = R["allocations"]
add(f"할당 (계수 할당자를 쓰는 전용 테스트 바이너리 `{alloc['test_binary'].get('log')}`, n = 128, 512; 판정 {verdict_word(alloc['verdict'])}):")
add("")
add(table(
    ["arm", "행", "센 스텝", "센 스텝의 할당 합", "시도당 할당", "스텝당 최대", "시도당 바이트 (n별)"],
    [
        [arm, item["rows"], item["counted_steps"], item["counted_allocations_total"],
         f"{item['allocations_per_attempt_min']}..{item['allocations_per_attempt_max']}", item["allocations_per_step_max"],
         ", ".join(f"n={n}: {v}" for n, v in item["bytes_per_attempt_by_n"].items())]
        for arm, item in alloc["summary_by_arm"].items()
    ],
))
add("")
add("스레드 arm의 0이 아닌 값은 rayon 풀의 작업 큐 블록이다 (인수분해 수십 회에 한 번).")
add("")
faults = R["page_faults"]
add(f"Minor page fault (러너 `faults` 모드, 프로세스당 arm 하나, 3회 연속 실행, 워밍업 = 처음 2 수락 스텝; 판정 {verdict_word(faults['verdict'])}). "
    "값은 수락 스텝당이다. 수락 스텝 하나는 시도 1회와 그 기각 시도를 담으므로 시도당 값의 상한이다.")
add("")
add(table(
    ["셀", "경로 길이", "arm", "실행 수", "스텝당 minor fault (최소..최대)"],
    [[r["cell"], r["argv0_length"], r["arm"], r["runs"], f"{f(r['minor_faults_per_accepted_step_min'], 2)}..{f(r['minor_faults_per_accepted_step_max'], 2)}"] for r in faults["summary"]],
))
add("")
add(f"재사용 arm의 최악값: {f(faults['reuse_worst_minor_faults_per_accepted_step'], 3)} (실행 {faults['reuse_runs']}개).")
add("")

# ---------------------------------------------------------------------------------------------
add("## 5. 측정 3: 재사용 arm의 벽시계, n = 512, 세 힙 배치 변형")
add("")
add("프로토콜: 코어 6-11 고정 (타이밍 락), arm 교대, 워밍업 2 + 반복 7, 실행 전후 load average 기록. "
    f"권위 조건: 전후 1분 load {f(R['timing_quality_rule']['load_limit'], 0)} 이하, arm별 (최대-최소)/중앙값 {f(R['timing_quality_rule']['spread_limit'], 2)} 이하. "
    "속도비 = 보호 드라이버 중앙값 / 재사용 arm 중앙값.")
add("")
rows = []
for cell, item in wall["cells"].items():
    for variant in item["variants"]:
        used = variant["invocation"]["used"]
        if used is None:
            rows.append([cell, variant["argv0_length_requested"], variant["status"], "-", "-", "-", "-", "-", "-", "-", "-"])
            continue
        p, r = used["arms"].get("protected"), used["arms"].get("reuse")
        rows.append([
            cell, used["argv0_length"], variant["label"], f"{len(variant['invocation']['tries'])}회 중 {used['try']}번째",
            ms(p["wall_median_ns"]), ms(r["wall_median_ns"]), f(variant["speedup_protected_over_reuse"], 3),
            f"{f(p['spread'], 3)} / {f(r['spread'], 3)}",
            f"{f(p['steady_minor_faults_per_accepted_step'], 1)} / {f(r['steady_minor_faults_per_accepted_step'], 1)}",
            f"{used['loadavg_before'][0]} / {used['loadavg_after'][0]}", p["attempts"],
        ])
add(table(["셀", "경로 길이", "라벨", "쓴 시도", "보호 ms", "재사용 ms", "속도비", "spread 보호/재사용", "스텝당 fault 보호/재사용", "load 전/후", "시도 수"], rows))
add("")
add(f"- 셀별 판정: " + ", ".join(f"{cell}: {verdict_word(v)}" for cell, v in V["reuse_wall_per_cell"].items()) + f". 합친 판정: {verdict_word(parts['wall'])}.")
add("- 모든 시도(재측정 포함)는 `RESULTS.json`의 `reuse_wall.cells.*.variants[].invocation.tries`에 있다.")
add("")

# ---------------------------------------------------------------------------------------------
add("## 6. 측정 4: 스레드 arm")
add("")
fp = R["threaded"]["faer_par_argument"]
add("### 6.1 faer의 병렬 인자: NOT_RUN")
add("")
add(f"- 작업공간 매니페스트(`Cargo.toml` 24행)는 faer 0.24.4를 기능 {fp['faer_features_in_the_build']}로 빌드한다 (`logs/{fp['feature_log']}`). `rayon` 기능이 없다.")
add(f"- 이 빌드에서 `faer::Par`에는 `Seq`뿐이다. 컴파일 프로브: `{fp['compile_probe_error']}` (종료 코드 {fp['compile_probe_exit_code']}, `logs/{fp['compile_probe_log']}`).")
add("- 켜려면 보호 대상인 `Cargo.toml`과 `Cargo.lock`(faer의 의존성에 rayon, spindle 추가)을 바꿔야 한다. 동결 행의 NOT_RUN 조건에 해당한다.")
add("")
add("### 6.2 클라이언트 측 분할 구현")
add("")
add("faer의 재귀를 클라이언트 안에서 돌린다: 64열 이하 패널은 faer의 순차 `lu_in_place`, 그 위 수준의 삼각 풀이, 행렬 곱, 행 교환을 워커 수만큼의 서로 겹치지 않는 블록으로 나눠 각 블록에서 faer의 순차 커널을 부른다 (`rayon::join`, 클라이언트 소유 풀). "
    "상수(패널 64열, 조각당 최소 2e6 flop, 교환 65536개)는 타이밍 전에 정했고 조정하지 않았다.")
add("")
client = R["threaded"]["client_side_arm"]
for cell, size in client["sizes"].items():
    add(f"**{cell}** (적분 구간 {SPANS[cell]}):")
    add("")
    rows = []
    for inv in size["invocations"]:
        used = inv["used"]
        if used is None:
            rows.append([f"{inv['rtol']:.0e}", inv["status"], "-", "-", "-", "-", "-", "-", "-"])
            continue
        arms = used["arms"]
        p = arms["protected"]
        cells_ms = " / ".join(ms(arms[a]["wall_median_ns"]) if a in arms else "-" for a in ("protected", "reuse", "threaded2", "threaded4", "threaded6"))
        rows.append([
            f"{inv['rtol']:.0e}", inv["label"], f"{len(inv['tries'])}회 중 {used['try']}번째", p["attempts"], e(p.get("error_max_abs")),
            cells_ms, f(used["worst_spread"], 3), f"{used['loadavg_before'][0]} / {used['loadavg_after'][0]}",
            f(p["wall_median_ms_per_attempt"], 2),
        ])
    add(table(["rtol", "라벨", "쓴 시도", "시도 수", "보호 arm 달성 오차", "중앙값 ms: 보호 / 재사용 / 2 / 4 / 6 워커", "최악 spread", "load 전/후", "보호 ms/시도"], rows))
    add("")
    rows = []
    for arm in ("threaded2", "threaded4", "threaded6", "reuse"):
        item = size["matched_error"].get(arm)
        if not item:
            continue
        for point in item["points"]:
            rows.append([
                arm, f"{point['rtol']:.0e}", f(point["speedup_at_matched_error"], 3), f(point["speedup_at_equal_tolerance"], 3),
                f"{point['attempts_arm']} / {point['attempts_protected']}", f(point["error_ratio"], 6),
                f(point["bit_identical_to_protected"]), point["label"],
            ])
    if rows:
        add(table(["arm", "rtol", "같은 달성 오차에서 속도비", "같은 허용오차에서 속도비", "시도 arm/보호", "오차비", "궤적 비트 동일", "라벨"], rows))
        add("")
ps = client["per_size"]
add("- 6 워커 판독: " + "; ".join(
    f"{cell}: 상태 {item['state']}, 속도비 최소 {f(item['speedup_min_6_workers'], 3)} / 최대 {f(item['speedup_max_6_workers'], 3)}, PASS 임계 충족 {f(item['meets_pass_thresholds'])}"
    for cell, item in ps.items()))
add(f"- PASS 기준: {verdict_word(client['pass_criterion'])}. KILL 기준: {verdict_word(client['kill_criterion'])}.")
add("- 기준 해: skewnet의 정확해 phi(t_end)를 파서가 mpmath 40자리로 다시 계산해 끝 상태(비트 패턴으로 기록)와 비교한다. "
    + "; ".join(f"{cell}: 러너 값과의 최대 차이 {e(ref['runner_value_max_abs_difference'])}" for cell, ref in R["references"].items()) + ". 기준 해 불확도 상한 1e-12 (사전 선언).")
add("")

# ---------------------------------------------------------------------------------------------
add("## 7. 탐색 측정 (판정 없음)")
add("")
ex = R["exploratory"]
rows = []
for item in ex["timing"]:
    used = item["invocation"]["used"]
    if used is None:
        rows.append([item["base"], item["status"], "-", "-", "-"])
        continue
    rows.append([
        item["base"], item["label"],
        ", ".join(f"{arm}: {f(v, 3)}" for arm, v in item["speedup_over_protected_at_equal_tolerance"].items()),
        ms(used["arms"]["protected"]["wall_median_ns"]), f"{used['loadavg_before'][0]} / {used['loadavg_after'][0]}",
    ])
if rows:
    add("같은 허용오차에서 보호 드라이버 대비 속도비 (기준 해 없음):")
    add("")
    add(table(["실행", "라벨", "속도비", "보호 중앙값 ms", "load 전/후"], rows))
    add("")
rows = []
for item in ex["instructions"]:
    if item.get("status") != "RUN":
        continue
    for arm, data in item["arms"].items():
        rows.append([item["cell"], arm, data["attempts_total"], f"{data['instructions_user_per_attempt']:.4e}", f(data.get("instructions_ratio_to_protected"), 4), f(data["minor_faults_per_attempt"], 1)])
if rows:
    add("사용자 모드 명령 수 (`perf stat -e instructions:u`, 설정 비용 차감, 결정적 보조 지표). 커널이 page fault에서 하는 일은 들어 있지 않다:")
    add("")
    add(table(["셀", "arm", "시도 합", "시도당 instructions:u", "보호 대비 비", "시도당 minor fault (perf)"], rows))
    add("")

# ---------------------------------------------------------------------------------------------
add("## 8. 한계와 열린 항목")
add("")
add("- 스레드 arm의 둘째 줄(6.2)이 부록 A2의 판정인지는 오케스트레이터의 결정 사항이다.")
add("- 벽시계 판정은 타이밍 품질 규칙을 만족한 시도만 쓴다. 라벨이 diagnostic_only인 칸의 수치는 판정에 쓰지 않았다.")
add("- 재사용 arm과 보호 드라이버의 동일성은 이 빌드(faer 0.24.4, `rayon` 기능 꺼짐, 전역 병렬도 `Seq`)에서 시험한 성질이다. faer 버전이나 기능이 바뀌면 다시 시험해야 한다.")
add("- 스레드 arm의 풀은 실행마다 클라이언트와 함께 만들어지고 버려진다 (잰 구간 안). 풀이(solve)는 순차다.")
failed = R["sources"]["invocations_with_non_zero_exit"]
add(f"- 종료 코드가 0이 아닌 호출: {len(failed)}개" + (": " + ", ".join(f"`{x['log']}` ({x['exit_code']})" for x in failed) if failed else "") + ".")
add(f"- 타이밍 전 대기에 쓴 시간: {f(R['sources']['wait_budget']['waited_s'], 0)}초 (노드 한도 1200초).")
add("")
add("## 9. 재현")
add("")
add("```")
add("export CARGO_TARGET_DIR=/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w2-v18")
add("python3 -I research/production_loop_20261116/dense_lu_client/run_measurements.py build links reference identity tests faults")
add("python3 -I research/production_loop_20261116/dense_lu_client/run_measurements.py wall wp extra perf")
add("python3 -I research/production_loop_20261116/dense_lu_client/parse_results.py")
add("python3 -I research/production_loop_20261116/dense_lu_client/make_report.py")
add("```")
add("")
add(f"러너 바이너리 sha256: {', '.join(R['sources']['runner_sha256'])}. 원시 로그 {len(R['sources']['logs'])}개의 sha256은 `RESULTS.json`의 `sources.logs`에 있다.")

(HERE / "REPORT_KO.md").write_text("\n".join(lines) + "\n")
print(f"REPORT_KO.md: {len(lines)} lines")
