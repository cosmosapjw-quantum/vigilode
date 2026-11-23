#!/usr/bin/env python3
"""Render REPORT_KO.md of loop node V08 from RESULTS.json and PREDECLARED.json.

Every number in the report is read from RESULTS.json (built by
build_results.py from the raw logs). Run with:  python3 -I build_report.py
"""

import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))


def load(name):
    with open(os.path.join(HERE, name), encoding="utf-8") as handle:
        return json.load(handle)


def ms(nanoseconds):
    return "-" if nanoseconds is None else f"{nanoseconds / 1.0e6:.2f}"


def us(nanoseconds):
    return "-" if nanoseconds is None else f"{nanoseconds / 1.0e3:.1f}"


def ratio(value):
    return "-" if value is None else f"{value:.2f}"


def percent(value):
    return "-" if value is None else f"{100.0 * value:.1f} %"


VERDICT_KO = {
    "PASS": "PASS",
    "FAIL": "FAIL",
    "KILL": "KILL",
    "NOT_DECIDABLE": "판정 불가",
    "NOT_RUN": "미실행",
}


def verdict(value):
    return VERDICT_KO.get(value, str(value))


def table(header, rows):
    lines = ["| " + " | ".join(header) + " |", "|" + "|".join("---" for _ in header) + "|"]
    lines += ["| " + " | ".join(str(cell) for cell in row) + " |" for row in rows]
    return "\n".join(lines)


def used_try(backend_entry):
    """The try whose numbers the tables show: the one the verdict used, else the last."""
    used = backend_entry["authoritative_ratios_used"]
    tries = backend_entry["tries"]
    if used:
        index = max(entry["try"] for entry in used.values()) - 1
        return tries[index]
    return tries[-1]


def wall_rows(results):
    rows = []
    wall = results["discriminators"]["wall"]
    for cell, entry in wall["cells"].items():
        for backend, data in entry["backends"].items():
            shown = used_try(data)
            arms = shown["arms"]
            seq = arms.get("seq", {})

            def cell_ratio(workers, key, flag, shown=shown):
                value = shown["ratios"].get(f"t{workers}")
                if value is None:
                    return "-"
                mark = "" if value[flag] else " (d)"
                return ratio(value[key]) + mark

            rows.append(
                [
                    f"`{cell}`" + (" (보조: 보호 kernel)" if backend == "banded_protected" else ""),
                    "예" if entry["qualifies"] else "아니오(통제)" if entry["class"] != "qualifying" else "아니오",
                    backend,
                    shown.get("colours") if shown.get("colours") is not None else f"n={entry['n']}",
                    us(shown["rhs_cost"]["median_ns"]) if shown.get("rhs_cost") else "-",
                    f"{shown['attempts']}/{shown['accepted_steps']}",
                    ms(seq.get("wall_median_ns")),
                    percent(seq.get("jacobian_share_of_wall")),
                    " / ".join(
                        cell_ratio(workers, "whole_solve_speed_up", "authoritative")
                        for workers in (1, 2, 4, 6)
                    ),
                    " / ".join(
                        cell_ratio(workers, "jacobian_phase_speed_up", "jacobian_phase_authoritative")
                        for workers in (1, 2, 4, 6)
                    ),
                    verdict(data["verdict"]) if backend == entry["primary_backend"] else f"({verdict(data['verdict'])})",
                ]
            )
    return rows


def tries_rows(results):
    rows = []
    for cell, entry in results["discriminators"]["wall"]["cells"].items():
        for backend, data in entry["backends"].items():
            for index, attempt in enumerate(data["tries"]):
                reasons = sorted(
                    {
                        reason
                        for value in attempt["ratios"].values()
                        for reason in value["why_not_authoritative"]
                    }
                )
                spreads = ", ".join(
                    f"{arm} {100.0 * summary['wall_spread']:.1f}"
                    for arm, summary in attempt["arms"].items()
                )
                ratios = " / ".join(
                    (
                        ratio(attempt["ratios"][f"t{workers}"]["whole_solve_speed_up"])
                        + ("" if attempt["ratios"][f"t{workers}"]["authoritative"] else " (d)")
                    )
                    if f"t{workers}" in attempt["ratios"]
                    else "-"
                    for workers in (2, 4, 6)
                )
                rows.append(
                    [
                        f"`{cell}`",
                        backend,
                        index + 1,
                        f"{attempt['load_1m_before']} / {attempt['load_1m_after']}",
                        ratios,
                        spreads,
                        "예" if attempt["same_final_state_in_every_run"] else "아니오",
                        "; ".join(reasons) if reasons else "모든 비율이 규칙 충족",
                        f"`{attempt['log']}`",
                    ]
                )
    return rows


def main():
    results = load("RESULTS.json")
    predeclared = load("PREDECLARED.json")
    disc = results["discriminators"]
    identity, calls, closed, wall = (
        disc["identity"],
        disc["rhs_calls"],
        disc["fail_closed"],
        disc["wall"],
    )
    out = []
    add = out.append

    add("# V08 Jacobian 조립: 정확한 thread 병렬·색칠 유한차분 조립 (opt-in stage client)")
    add("")
    add(
        "- node: `V08_JACOBIAN_ASSEMBLY` (loop L1, wave 2). 이 문서는 `build_report.py`가 "
        "`RESULTS.json`에서 만든다. 숫자는 모두 실행 로그(`logs/`)에서 `build_results.py`가 읽은 값이다."
    )
    add(
        f"- 사전 선언: `PREDECLARED.json`, commit `{results['predeclared_commit']}` "
        "(어떤 측정보다 먼저 commit). 결과 생성 시점 HEAD: "
        f"`{results['head_at_build']}`."
    )
    add(
        "- code: `crates/rodas5p-integrators/src/stage_clients/assembly.rs`, `assembly/engine.rs`, "
        "`assembly/pattern.rs`. test: `tests/stage_clients_assembly_{identity,alloc,measure,problems}.rs`."
    )
    add("")

    add("## 1. 판정 요약")
    add("")
    qualifying = wall["qualifying_cells"]
    add(
        table(
            ["동결 판별식", "판정", "근거"],
            [
                [
                    "Jacobian과 궤적이 순차 조립과 bit 동일",
                    verdict(identity["verdict"]),
                    f"선언 문제 5개의 {identity['runs_compared']}개 run(arm seq, t1, t2, t4, t6)이 "
                    "보호 driver + 기준 유한차분 run과 모든 항목에서 같다 (3.1절)",
                ],
                [
                    "색칠 경로의 RHS 호출 = 색 수 + 1",
                    verdict(calls["verdict"]),
                    f"client run {calls['client_runs_checked']}개 전부에서 조립 호출 = build 수 × (색 수 + 1) "
                    f"(dense 경로는 n + 1). 위반 {len(calls['failures'])}건 (3.2절)",
                ],
                [
                    "틀린 pattern은 닫힌 채 실패",
                    verdict(closed["verdict"]),
                    f"음성 통제 {sum(1 for c in closed.get('controls', []) if c['must_fail'])}개 모두 "
                    "`InvalidInput` + typed error, 수락 step 0 (3.3절)",
                ],
                [
                    "wall ≥ 1.5x (n ≥ 64, RHS ≥ 50 µs인 FD 문제); KILL < 1.1x",
                    verdict(wall["node_verdict"]),
                    "자격 문제별: "
                    + ", ".join(f"`{cell}` {verdict(value)}" for cell, value in qualifying.items())
                    + ". 사전 선언한 엄격한 규칙(자격 문제 전부 PASS여야 node PASS)에 따른 값 (3.4절)",
                ],
            ],
        )
    )
    add("")
    add(
        "판정 규칙은 `PREDECLARED.json`의 `verdict_rules` 그대로다. wall 판정은 전체 풀이 시간"
        "(client 생성 + driver 호출)으로 하고, Jacobian phase 시간은 옆에 적기만 한다."
    )
    add("")
    undecided = [
        cell
        for cell, entry in wall["cells"].items()
        if entry["qualifies"] and entry["verdict"] == "NOT_DECIDABLE"
    ]
    if undecided:
        add(
            "**wall 판정 불가의 이유**: "
            + ", ".join(f"`{cell}`" for cell in undecided)
            + "에서 품질 규칙(1분 load ≤ 8, arm 내 퍼짐 ≤ 10 %)을 충족한 측정이 P = 2·4·6 전부에 대해서는 "
            "나오지 않았다. 측정하는 동안 같은 group의 다른 agent들이 build·timing을 했고 다른 project의 "
            "job도 돌았다. 선언한 유한 대기(20분) 뒤 측정했고, cell마다 측정은 "
            + "·".join(
                str(len(wall["cells"][cell]["backends"][wall["cells"][cell]["primary_backend"]]["tries"]))
                for cell in undecided
            )
            + "회다(규칙상 첫 측정 + 재측정 두 번까지, 모든 측정을 남김). 기준은 움직이지 않았다. 아래 "
            "진단값은 try마다 일관되지만 판정에 쓰지 않는다."
        )
        add("")
        rows = []
        for cell, entry in wall["cells"].items():
            if not entry["qualifies"]:
                continue
            data = entry["backends"][entry["primary_backend"]]
            for workers in (2, 4, 6):
                values = [
                    attempt["ratios"][f"t{workers}"]
                    for attempt in data["tries"]
                    if f"t{workers}" in attempt["ratios"]
                ]
                rows.append(
                    [
                        f"`{cell}`",
                        workers,
                        ", ".join(
                            ratio(value["whole_solve_speed_up"])
                            + ("" if value["authoritative"] else " (d)")
                            for value in values
                        ),
                        ", ".join(ratio(value["jacobian_phase_speed_up"]) for value in values),
                    ]
                )
        add(
            table(
                ["자격 문제", "P", "전체 풀이 속도비 (try 순서)", "Jacobian phase 속도비 (try 순서)"],
                rows,
            )
        )
        add("")

    add("## 2. 만든 것")
    add("")
    add(
        "`AssemblyStageClient`는 선형화마다 문제의 RHS만으로 Jacobian을 만들고, 그 값을 kit의 기존 "
        "client(바꾸지 않음)에 넘긴다. 문제의 Jacobian·JVP callback은 부르지 않는다. 기본 경로에는 "
        "아무 영향이 없다(opt-in)."
    )
    add("")
    add(
        "- **산술**: facade의 유한차분 `crates/vigilode/src/fd.rs`와 같다. 증분은 "
        "`(y_j + sqrt(eps) max(|y_j|, 1)) - y_j`(`fd.rs:443-455`), 원소는 "
        "`(f(t, y + shift)[i] - f(t, y)[i]) / delta_j`(`fd.rs:499`, `fd.rs:611`)."
    )
    add(
        "- **dense 경로**: 열마다 RHS 1회 + 기준 평가 1회 = build당 n + 1회. 순차 mode는 "
        "`fd.rs:476-509`의 loop를 문장 단위로 옮겼다."
    )
    add(
        "- **색칠 경로**: 선언한 sparsity pattern의 열을 Curtis–Powell–Reid(자연 순서 first-fit)로 묶고 "
        "묶음마다 RHS 1회 = build당 색 수 + 1회. loop는 `fd.rs:581-622`. 꽉 찬 band에서는 facade와 같은 "
        "묶음(`j mod (lower + upper + 1)`)이 된다."
    )
    add(
        "- **실행**: 순차(호출 thread, pool 없음) 또는 thread(client가 소유한 rayon pool, 정확히 P개 thread, "
        "전역 pool 미사용). thread mode는 순도 계약을 명시적으로 수락해야 켜진다(5절)."
    )
    add(
        "- **선형대수 back end**: `DenseLuStageClient`(보호 dense driver의 LU), `BandedStageLu`(보호 banded "
        "kernel, Indexed), `BandedTightStageLu`(V21, 보호 kernel과 bit 동일). 조립한 값은 back end의 "
        "배치(dense row-major 또는 band)로 한 번 복사된다."
    )
    add("")
    add(
        "이 crate에는 상태 Jacobian의 유한차분이 없다(`OdeProblem::new`는 Jacobian이나 JVP를 요구한다). "
        "보호 driver의 유한차분 경로는 facade(`vigilode`)가 Jacobian callback으로 끼워 넣는 `fd.rs`이고, "
        "integrators crate는 facade에 의존할 수 없으므로 산술을 옮겨 적었다. 옮겨 적은 것이 실제 facade "
        "code와 같은지는 6절(facade 대조)에서 `vigilode::Rodas5p::solve`로 확인했다."
    )
    add("")

    add("## 3. 판별식별 결과")
    add("")
    add("### 3.1 Jacobian과 궤적의 bit 동일성")
    add("")
    add(
        "기준은 보호 driver(dense: `integrate_rodas5p_fast_observed_with_atol`, banded: "
        "`integrate_rodas5p_fast_banded_observed_with_atol`, kernel Indexed)에 `fd.rs`를 두 번째로 옮겨 적은 "
        "순차 유한차분을 Jacobian callback으로 준 run이다. 비교 항목: " + ", ".join(
            f"`{key}`" for key in identity["rule"].split(": ", 1)[1].split(", ")
        )
        + ". `state_fingerprint`는 모든 선형화 시점의 (t, y) bit, `jacobian_fingerprint`는 조립한 "
        "Jacobian의 모든 저장 원소 bit를 이어서 hash한 값이다."
    )
    add("")
    rows = []
    for cell, backends in identity["cells"].items():
        for backend, group in backends.items():
            equal = sum(1 for arm in group["arms"].values() if arm["equal_to_protected_run"])
            rows.append(
                [
                    f"`{cell}`",
                    backend,
                    group["protected_driver"],
                    f"{group['attempts']} / {group['accepted_steps']} / {group['rejected_steps']}",
                    group["builds"],
                    f"{group['t_end']:.6g}",
                    f"`{group['jacobian_fingerprint']}`",
                    f"{equal} / {len(group['arms'])}",
                ]
            )
    add(
        table(
            [
                "문제",
                "back end",
                "기준 driver",
                "시도 / 수락 / 거절",
                "Jacobian build",
                "t_end",
                "Jacobian fingerprint (기준)",
                "기준과 같은 arm",
            ],
            rows,
        )
    )
    add("")
    add(
        "crate test(`stage_clients_assembly_identity.rs`, 13개)는 작은 문제에서 같은 것을 더 넓게 본다: "
        "모든 mode(dense·색칠, 순차·thread 1–4개, dense·band 저장)의 Jacobian이 기준과 원소 단위로 bit 동일, "
        "rtol 1e-4·1e-6·1e-8에서 보호 dense driver(in-place LU와 faer LU)·보호 banded driver와 "
        "상태·통계·counter까지 동일."
    )
    add("")
    add(
        "색칠 경로의 Jacobian이 dense 경로와 bit 동일하려면 RHS의 성분 i가 선언한 열의 값만 읽어야 한다"
        "(pattern = 읽기 의존성). 수학적으로 0이지만 부동소수점에서 상쇄되지 않는 항은 의존성으로 선언해야 "
        "하고, 선언하지 않으면 3.3절의 검사가 거부한다."
    )
    add("")

    add("### 3.2 RHS 호출 수")
    add("")
    rows = []
    for cell, backends in identity["cells"].items():
        for backend, group in backends.items():
            for arm_name in ("seq", "t6"):
                arm = group["arms"].get(arm_name)
                if arm is None:
                    continue
                rows.append(
                    [
                        f"`{cell}`",
                        backend,
                        arm_name,
                        arm["colours"] if arm["colours"] is not None else "-",
                        arm["rhs_per_build"],
                        arm["builds"],
                        arm["assembly_rhs_calls"],
                        arm["verification_rhs_calls"],
                        group["stage_rhs_calls"],
                        arm["rhs_calls"],
                        arm["rhs_calls_counted_by_the_callback"],
                    ]
                )
    add(
        table(
            [
                "문제",
                "back end",
                "arm",
                "색 수",
                "build당 RHS",
                "build",
                "조립 RHS",
                "검사 RHS",
                "stage RHS (기준 run)",
                "run counter",
                "callback이 센 값",
            ],
            rows,
        )
    )
    add("")
    add(
        f"검사한 client run {calls['client_runs_checked']}개(그중 {calls['of_which_with_callback_count']}개는 "
        "RHS callback 안의 counter로도 확인) 전부에서 조립 RHS = build × (색 수 + 1), dense 경로는 "
        "build × (n + 1)이다. 검사 RHS는 첫 선형화 한 번뿐이다: 검사 열 8개, thread mode는 기준 평가 1회 추가. "
        "순차 dense 경로는 검사가 없다. run counter = stage RHS + 조립 RHS + 검사 RHS."
    )
    if calls["failures"]:
        add("")
        add("위반:")
        for failure in calls["failures"]:
            add(f"- {failure}")
    add("")

    add("### 3.3 틀린 pattern과 순수하지 않은 RHS: 닫힌 채 실패")
    add("")
    add("검사는 두 겹이다.")
    add("")
    add(
        "1. **색칠 build마다(추가 RHS 없음)**: 어떤 색의 열도 선언하지 않은 행은 그 색의 평가에서 기준 평가와 "
        "bit가 같아야 한다. 다르면 선언하지 않은 의존성이다."
    )
    add(
        "2. **첫 선형화, 그리고 요청할 때**: 검사 열(기본 8개, 고르게 배치)을 호출 thread에서 한 열씩 평가한다. "
        "색칠 경로는 선언한 원소가 조립한 원소와 bit 동일해야 하고 선언하지 않은 행은 변하지 않아야 한다. "
        "thread mode는 기준 평가도 호출 thread에서 다시 해서 pool이 낸 값과 비교한다."
    )
    add("")
    add(
        "실패하면 `AssemblyError`(typed)가 report handle에 남고 driver에는 `CoreError::InvalidInput`이 간다 "
        "(치명적 종류: run이 `Err`로 끝나고 재시도하지 않는다)."
    )
    add("")
    rows = []
    for control in closed.get("controls", []):
        rows.append(
            [
                control["control"],
                f"`{control['problem']}`",
                control["arm"],
                control["first_use_check"],
                "실패해야 함" if control["must_fail"] else "통과해도 됨",
                f"{control['outcome']} {control['driver_error_kind'] or ''}".strip(),
                f"`{control['typed_error']}`" if control["typed_error"] else "-",
                control["accepted_steps"] if control["accepted_steps"] is not None else 0,
                f"`{control['direct_full_check']}`",
            ]
        )
    add(
        table(
            [
                "통제",
                "문제",
                "arm",
                "첫 검사",
                "기대",
                "driver 결과",
                "typed error",
                "수락 step",
                "직접 전체 검사 (`verify`, 모든 열)",
            ],
            rows,
        )
    )
    add("")
    add(
        "**검사가 보지 못하는 것**(숨기지 않는다): (a) 검사한 모든 상태에서 유한차분이 정확히 0인 미선언 원소. "
        "(b) 검사 열을 표본으로 뽑을 때, 표본에 들지 않은 열에서만 같은 색의 선언 원소에 섞여 들어가는 미선언 "
        "원소. 위 표의 반례 행이 (b)다: 검사 열을 끄면 섞인 원소는 build마다의 검사만으로는 드러나지 않는다. "
        "`CheckColumns::All`(RHS n회)이 (b)를 닫는다. 체계적인 오류(모든 행에서 같은 결합을 빠뜨림)는 표본 "
        "8열로 잡힌다."
    )
    add("")

    add("### 3.4 wall")
    add("")
    add(
        "각 cell은 한 process에서 arm을 돌아가며(round r은 arm r mod A부터) warm-up 2회 + 측정 7회. "
        "timing lock + `taskset -c 6-11`. run 길이는 선언대로 48 시도 또는 구간 끝. 속도비 = seq arm 중앙값 "
        "/ tP arm 중앙값. `(d)`는 품질 규칙(1분 load ≤ 8, arm 내 (max − min)/중앙값 ≤ 10 %)을 못 맞춰 "
        "`diagnostic_only`인 값이다. 표의 숫자는 판정에 쓴 try(없으면 마지막 try)의 값이다."
    )
    add("")
    add(
        table(
            [
                "문제",
                "자격",
                "back end",
                "색 수",
                "RHS 비용 µs",
                "시도/수락",
                "seq wall ms",
                "seq의 Jacobian 비중",
                "전체 풀이 속도비 P=1/2/4/6",
                "Jacobian phase 속도비 P=1/2/4/6",
                "cell 판정",
            ],
            wall_rows(results),
        )
    )
    add("")
    add(f"node 판정(엄격한 규칙): **{verdict(wall['node_verdict'])}**.")
    not_qualifying = wall["declared_qualifying_not_qualifying"]
    if not_qualifying:
        add(
            "자격으로 선언했지만 RHS 비용이 50 µs 미만이라 판정에서 빠진 문제: "
            + ", ".join(f"`{cell}`" for cell in not_qualifying)
            + " (바꾸거나 키우지 않았다)."
        )
    add("")
    rows = []
    for cell, entry in wall["cells"].items():
        for backend, data in entry["backends"].items():
            shown = used_try(data)
            seq = shown["arms"].get("seq", {})
            share = seq.get("jacobian_share_of_wall")
            rows.append(
                [
                    f"`{cell}`",
                    backend,
                    ms(seq.get("linearize_median_ns")),
                    ms(seq.get("factor_median_ns")),
                    ms(seq.get("solve_median_ns")),
                    ms(seq.get("construct_median_ns")),
                    percent(share),
                    percent(seq.get("rhs_share_of_jacobian_phase_estimate")),
                    percent(seq.get("rhs_share_of_wall_estimate")),
                    " / ".join(
                        ratio(
                            shown["ratios"].get(f"t{workers}", {}).get(
                                "amdahl_bound_from_the_measured_jacobian_share"
                            )
                        )
                        for workers in (2, 4, 6)
                    ),
                    ratio(shown.get("client_seq_over_protected_driver_wall")),
                ]
            )
    add(
        table(
            [
                "문제",
                "back end",
                "seq Jacobian ms",
                "seq LU ms",
                "seq stage solve ms",
                "client 생성 ms",
                "Jacobian 비중",
                "Jacobian phase 중 RHS (추정)",
                "wall 중 조립 RHS (추정)",
                "Amdahl 상한 P=2/4/6",
                "seq client / 보호 driver wall",
            ],
            rows,
        )
    )
    add("")
    add(
        "Amdahl 상한은 측정한 seq arm의 Jacobian 비중 f로 계산한 `1 / (1 - f (1 - 1/P))`다. "
        "'Jacobian phase 중 RHS'는 build 수 × build당 RHS 수 × 보정한 RHS 비용을 seq arm의 Jacobian phase "
        "시간으로 나눈 추정값이다: 나머지는 원소 계산·검사·back end로의 복사이며, RHS만큼 잘 병렬화되지 "
        "않는다. 마지막 열은 순차 client의 wall을 같은 유한차분 callback을 단 보호 driver의 wall로 나눈 "
        "값(진단)이다."
    )
    add("")
    add("측정 이력(모든 try를 남긴다):")
    add("")
    add(
        table(
            [
                "문제",
                "back end",
                "try",
                "1분 load 전/후",
                "전체 속도비 P=2/4/6",
                "arm별 wall 퍼짐 %",
                "모든 run의 끝 상태 동일",
                "권위 없는 이유",
                "log",
            ],
            tries_rows(results),
        )
    )
    add("")
    add(
        "**정합 오차**: 한 cell의 모든 arm은 같은 수락 상태를 bit 단위로 낸다(timing run마다 끝 상태 "
        "fingerprint를 확인, 위 표). 따라서 달성 오차는 arm마다 같은 수이고, 속도비는 정확히 같은 오차에서의 "
        "비교다. 달성 오차 수준 자체는 7절에 적었다."
    )
    add("")

    add("## 4. thread 병렬이 이득인 영역과 손해인 영역")
    add("")
    add(
        "병렬화되는 것은 Jacobian 조립의 RHS뿐이다. 8개 stage의 RHS와 LU, stage solve는 순차로 남는다. "
        "그래서 전체 풀이의 이득은 Jacobian 비중 f가 정한다(3.4절 둘째 표)."
    )
    add("")
    add(
        "- **이득**: dense 유한차분(build당 n + 1 RHS, stage는 8 RHS)에서 RHS가 비쌀 때. f가 1에 가깝다."
    )
    add(
        "- **조건부**: 색칠 경로. build당 RHS가 색 수 + 1뿐이므로 f ≤ (색 수 + 1)/(색 수 + 9)이고, 선형대수 "
        "시간이 더해지면 더 작다. 색 수가 많고(넓은 band) LU가 싸야 이득이 난다. back end의 kernel 속도가 "
        "f를 바꾼다(보조 cell 참조)."
    )
    add(
        "- **이득 없음**: 좁은 band의 고전적 MOL(색 5개). 구조상 Jacobian이 step의 작은 부분이다. V04의 "
        "측정(FD band에서 Jacobian phase 3.6–9.3 %)과 같은 결론이다."
    )
    add(
        "- **손해**: RHS가 싼 문제(통제 cell). pool에 일을 넘기고 깨우는 비용이 RHS보다 크다. P = 1 열은 "
        "병렬 이득 없이 넘기는 비용만 보여 준다."
    )
    add("")
    add(
        "기준 평가도 pool의 작업 하나다. 작업 수(색 수 + 1, dense는 n + 1)가 P보다 적으면 남는 worker는 "
        "할 일이 없다. 원소 계산과 행별 모으기도 pool에서 하지만, 이 부분과 back end로의 복사는 memory "
        "대역폭에 묶여 RHS만큼 빨라지지 않는다. 그래서 Jacobian phase의 속도비는 P보다 작고, RHS가 "
        "Jacobian phase에서 차지하는 몫(3.4절 둘째 표)이 작을수록 더 작다."
    )
    add("")
    shown_cells = []
    for cell, entry in wall["cells"].items():
        primary = entry.get("primary_backend")
        if not primary:
            continue
        shown = used_try(entry["backends"][primary])
        seq = shown["arms"].get("seq", {})
        best = max(
            (
                value["whole_solve_speed_up"]
                for arm, value in shown["ratios"].items()
                if arm != "t1"
            ),
            default=None,
        )
        shown_cells.append(
            f"`{cell}`: 조립 RHS가 wall의 {percent(seq.get('rhs_share_of_wall_estimate'))}"
            f", P = 2·4·6 중 가장 큰 전체 속도비 {ratio(best)}"
        )
    add("측정값으로 본 순서: " + "; ".join(shown_cells) + ".")
    add("")

    add("## 5. 순도 계약 (thread mode를 켜는 사람이 수락하는 것)")
    add("")
    add(
        "thread build에서는 RHS가 여러 thread에서 동시에, 서로 다른 이동된 상태로, 정해지지 않은 순서로 "
        "불린다. callback 형이 요구하는 `Send + Sync`는 memory 안전만 보장한다. "
        "`RhsPurity::pure_and_reentrant()`를 부르는 사람은 다음을 보증한다."
    )
    add("")
    add(
        "1. 결과는 `(t, y)`만의 함수다: 호출 사이에 넘어가는 상태가 없다(직전 호출을 key로 하는 cache, "
        "결과에 들어가는 counter, 난수, 적응 table)."
    )
    add("2. 동시 호출이 서로에게 영향을 주지 않는다: 공유 scratch가 없거나, 있어도 결과를 바꿀 수 없다.")
    add(
        "3. 결과가 실행 thread에 의존하지 않는다: thread-local 상태가 없고, 모든 thread의 부동소수점 "
        "환경(반올림 mode, flush-to-zero)이 같다."
    )
    add("")
    add(
        "계약을 어기면 Jacobian이 scheduling에 의존한다. client는 계약을 증명할 수 없다. 대신 첫 선형화에서 "
        "기준 평가와 검사 열을 호출 thread에서 다시 계산해 pool의 결과와 bit 비교하고, 다르면 "
        "`AssemblyError::NotReproducible`로 run을 끝낸다(3.3절 통제 3). 이 검사는 표본이다: 통과가 순도의 "
        "증명은 아니다. 순차 mode에는 이 계약이 필요 없다(보호 경로와 같은 호출 순서)."
    )
    add("")
    add(
        "실패 시의 차이 하나: RHS가 `Err`를 내거나 원소가 넘치면 thread build도 순차 build가 만났을 첫 "
        "실패(작업 순서, 기준 평가 먼저)를 그대로 돌려주므로 driver의 반응과 궤적은 같다. 다만 그때까지 "
        "build의 모든 RHS가 이미 호출되었으므로 실패한 build의 `rhs_calls`는 순차보다 크다."
    )
    add("")

    add("## 6. facade 대조와 facade의 색칠 경로 (별도 측정)")
    add("")
    facade = results["facade"]
    if "rows" in facade:
        identity_rows = [row for row in facade["rows"] if row["measurement"] == "facade_identity"]
        rows = []
        for row in identity_rows:
            for arm in row["client_arms"]:
                rows.append(
                    [
                        row["case"],
                        f"`{row['facade']['entry_point']}`",
                        f"{row['facade']['attempts']} / {row['facade']['accepted_steps']}",
                        row["facade"]["fd_rhs_calls"],
                        f"{arm['arm']} + {row['client_backend']}",
                        arm["assembly_rhs_calls"],
                        arm["final_components_with_different_bits"],
                        "예" if arm["same_steps_as_the_facade"] else "아니오",
                    ]
                )
        add(
            "**인용 확인**: `vigilode::Rodas5p::solve`(`Jacobian::FiniteDifference`, vector atol)와, 같은 설정으로 "
            "client driver + 조립 client를 돌린 결과. facade의 `fd.rs` code가 실제로 실행된 쪽과 비교한다."
        )
        add("")
        add(
            table(
                [
                    "경우",
                    "facade가 부른 entry",
                    "시도 / 수락",
                    "facade FD RHS",
                    "client",
                    "client 조립 RHS",
                    "끝 상태에서 bit가 다른 성분",
                    "step 수 동일",
                ],
                rows,
            )
        )
        add("")
        coloured = next(
            (row for row in facade["rows"] if row["measurement"] == "facade_coloured_route"), None
        )
        if coloured:
            solve = coloured["solve_with_sparse_jvp"]
            recovered = coloured["recovered_values_against_the_analytic_jacobian"]
            analytic = coloured["final_state_against_the_analytic_band_jacobian"]
            grouped = coloured["final_state_against_the_facade_grouped_finite_differences"]
            add(
                "**facade의 색칠 경로**(`Jacobian::sparse_jvp` + `crates/vigilode/src/coloring.rs`). 무효 처리된 "
                "ledger 행 L-0214(simulation에서 나온 수)를 실행한 수로 대신한다. V08의 동결 판별식은 아니다."
            )
            add("")
            add(f"- 문제: {coloured['problem']}. pattern은 해석 Jacobian의 실제 0 아닌 원소 {coloured['pattern']['nnz']}개, "
                f"band ({coloured['pattern']['bandwidth'][0]}, {coloured['pattern']['bandwidth'][1]}).")
            add(
                f"- 색 수: **{coloured['pattern']['colours']}** (꽉 찬 band로 묶으면 "
                f"{coloured['pattern']['groups_of_the_full_band_for_comparison']}). colouring 유효: "
                f"{'예' if coloured['pattern']['colouring_is_valid'] else '아니오'}."
            )
            add(
                f"- Jacobian build당 JVP 호출: **{solve['jvp_calls_per_jacobian_build']:.1f}** "
                f"(callback이 센 {solve['jvp_calls_counted_by_the_callback']}회 / build "
                f"{solve['jacobian_builds_charged_by_the_driver']}회; 시도 {solve['attempts']}, 수락 "
                f"{solve['accepted_steps']}, entry `{solve['entry_point']}`)."
            )
            add(
                f"- 복원한 값 대 해석 dense Jacobian: y0에서 {recovered['at_y0']['entries']}개 원소 중 bit가 다른 것 "
                f"{recovered['at_y0']['entries_with_different_bits']}개(최대 차 "
                f"{recovered['at_y0']['largest_absolute_difference']:.3g}), 끝 상태에서 "
                f"{recovered['at_the_final_state']['entries_with_different_bits']}개(최대 차 "
                f"{recovered['at_the_final_state']['largest_absolute_difference']:.3g})."
            )
            add(
                f"- 끝 상태 대 해석 band Jacobian(`Jacobian::banded`) 풀이: bit가 다른 성분 "
                f"{analytic['components_with_different_bits']}개, 최대 차 "
                f"{analytic['largest_absolute_difference']:.3g}."
            )
            add(
                f"- 참고: 같은 band의 facade 유한차분은 build당 RHS {grouped['fd_rhs_calls_per_build']:.1f}회이고, "
                f"끝 상태는 색칠 JVP 풀이와 {grouped['components_with_different_bits']}개 성분에서 다르다"
                f"(최대 차 {grouped['largest_absolute_difference']:.3g}; 유한차분 Jacobian이므로 당연하다)."
            )
            add(
                f"- **발견**: facade `Stats`의 `counters.jvp_calls`는 "
                f"{solve['jvp_calls_in_stats_counters']}, `driver_counters.jvp_calls`는 "
                f"{solve['jvp_calls_in_stats_driver_counters']}이다. band 채우기 안에서 부른 JVP "
                f"{solve['jvp_calls_counted_by_the_callback']}회가 facade의 work 기록에 잡히지 않는다."
            )
    else:
        add(f"미실행: {facade.get('reason')}")
    add("")

    add("## 7. 그 밖의 측정")
    add("")
    tests = results["tests"]
    if "allocation" in tests:
        allocation = tests["allocation"]
        add(
            f"- **할당**(`stage_clients_assembly_alloc.rs`, 별도 binary): 순차 mode run "
            f"{allocation['sequential_runs']}개, 정상 구간 step {allocation['sequential_steady_steps_measured']}개에서 "
            f"할당 합계 **{allocation['sequential_steady_allocations_total']}** "
            f"({verdict(allocation['verdict'])}). 대상: "
            + "; ".join(allocation["sequential_clients"])
            + ". thread mode는 보고만 한다: "
            + "; ".join(
                f"{row['client']} rtol {row['rtol']:g}: step {row['steady_steps_measured']}개에 "
                f"{row['steady_allocations_total']}회"
                for row in allocation["threaded_runs_reported"]
            )
            + "."
        )
    else:
        add(f"- 할당: 미실행 ({tests.get('reason')}).")
    perf = results["instructions"]["runs"]
    if perf:
        add(
            "- **instructions:u**(진단 전용, arm당 풀이 1회, process 전체): "
            + "; ".join(
                f"`{run['cell']}` {run['arm']}: "
                + (
                    f"{run['instructions_user_all_threads'] / 1.0e9:.3f} G"
                    if run["instructions_user_all_threads"]
                    else "읽지 못함"
                )
                for run in perf
            )
            + ". thread는 instruction을 줄이지 않는다(조금 늘린다). 이 node의 판별식이 아니다."
        )
    else:
        add("- instructions:u: 미실행.")
    reference = results["reference_error"]
    if "rows" in reference and reference["rows"]:
        for row in reference["rows"]:
            if row["status"] == "computed":
                add(
                    f"- **달성 오차** `{row['cell']}` (t = {row['t_end']:.6g}): 독립 기준(SciPy LSODA, rtol 1e-12, "
                    f"atol 1e-14, 해석 Jacobian, NumPy RHS) 대비 최대 절대 오차 "
                    f"{row['achieved_error_max_norm']:.3g}, 허용오차 단위로 최대 "
                    f"{row['achieved_error_in_tolerance_units_max']:.3g}(rms "
                    f"{row['achieved_error_in_tolerance_units_rms']:.3g}); 기준의 불확도(rtol 1e-10 풀이와의 "
                    f"최대 차) {row['reference_uncertainty_max_norm']:.3g}. 탐색적 기록이며 판별식이 아니다."
                )
            else:
                add(
                    f"- **달성 오차** `{row['cell']}`: 계산하지 못함. 장애: {row['obstacle']}."
                )
    else:
        add("- 달성 오차: 미실행.")
    add("")

    add("## 8. 선언과 다르게 한 것, 경과")
    add("")
    add(
        "- timing의 arm 순환은 진단용 `prot` arm(보호 driver + 기준 유한차분 callback)을 포함한 6개 arm으로 "
        "돌렸다(round r은 arm r mod 6부터). `PREDECLARED.json`의 문장은 5개 arm(r mod 5)이다. `prot`는 "
        "판정에 쓰이지 않는다. 보조 cell(보호 banded kernel)은 선언대로 seq와 t6만 쟀다."
    )
    add(
        "- 계약 test의 허용오차: 처음에는 모든 문제에 atol = 1e-2 rtol을 썼다. 선언은 Brusselator에 "
        "atol = rtol(b1 case scale 1)이다. test를 선언대로 고쳤고 전부 통과한다. 측정 program은 처음부터 "
        "선언값(rtol 1e-6, atol 1e-8)이다."
    )
    add(
        "- 기준 풀이는 이 host의 `python3 -I`가 찾는 SciPy 1.11.4 / NumPy 1.26.4로 계산했다. banded 문제의 "
        "첫 시도는 LSODA의 banded Jacobian 배열 크기(lband 행 padding) 때문에 실패했고, script를 고쳐 다시 "
        "돌렸다(`logs/reference_error.log` 머리말)."
    )
    add(
        "- 대기와 측정 순서: 유한 대기 21:57–22:17(1분 간격, load 12–28, 8 이하가 된 적 없음) 뒤 try 1. "
        "try 2는 다른 작업(instruction 계수, 기준 풀이, 검사)을 마친 뒤 load가 8 근처일 때 쟀다. "
        "측정 이력 표가 모든 try를 보여 준다."
    )
    add("")
    add("## 9. 재현")
    add("")
    add("```bash")
    add("cd research/production_loop_20261116/jacobian_assembly")
    add("./run_measurements.sh build && ./run_measurements.sh failclosed")
    add("./run_measurements.sh identity <cell>          # 5개 cell")
    add("./run_measurements.sh timing <cell> <try>      # timing lock, core 6-11")
    add("./run_measurements.sh runner-build && ./run_measurements.sh facade")
    add("python3 -I build_results.py && python3 -I build_report.py")
    add("```")
    add("")
    checks = results["checks"]
    if "rows" in checks and checks["rows"]:
        add(
            table(
                ["검사", "종료 code", "결과"],
                [[row["name"], row["exit_code"], row["result"]] for row in checks["rows"]],
            )
        )
        add("")

    add("## 10. 열린 문제")
    add("")
    add(
        "- 색칠 경로의 전체 풀이 이득은 문제 구조(색 수, 선형대수 비용)에 달려 있다. 넓은 band에서도 "
        "back end kernel에 따라 판정이 갈린다."
    )
    add(
        "- 검사 열은 표본이다. 표본 밖 열의 섞인 원소와, 검사 시점에 유한차분이 0인 원소는 보지 못한다."
    )
    add(
        "- dense back end의 값 복사(n²)와 thread mode의 작업별 상태 복사는 RHS가 싼 문제에서 손해다."
    )
    add(
        "- facade `Jacobian::sparse_jvp`의 band 경로는 JVP 호출을 `Stats`에 기록하지 않는다(6절)."
    )
    add(
        "- 색 수가 적고 n이 큰 문제(`bruss1d-n131072`)에서는 이 client의 Jacobian phase 대부분이 RHS가 아니라 "
        "원소 계산·복사다(3.4절 둘째 표). thread로는 줄지 않는다. 원소 table 없이 band를 직접 채우고 "
        "복사를 없애는 순차 경로가 더 낫다(만들지 않았다)."
    )
    add(
        "- 묶음은 자연 순서 first-fit이라 최소가 아니다(순환 삼중대각 n = 11에서 5색, 최소는 4). client "
        "생성(색칠과 table)은 n = 4096, 49색에서 16–18 ms로, 짧은 측정 풀이의 4–7 %다(선언한 wall에 포함)."
    )
    if wall["node_verdict"] == "NOT_DECIDABLE":
        most = max(
            len(data["tries"])
            for entry in wall["cells"].values()
            for name, data in entry["backends"].items()
            if name == entry["primary_backend"]
        )
        left = max(0, 3 - most)
        add(
            "- wall 판별식은 host가 조용할 때 같은 명령(`run_measurements.sh timing <cell> <try>`)으로 "
            f"다시 재야 판정된다. 규칙이 허용하는 측정은 cell당 3회이고 지금까지 {most}회 했다"
            + (f"(남은 재측정 {left}회)." if left else "(남은 재측정 없음).")
        )
    add("- client는 opt-in으로만 둔다. 기본 routing과 보호 source는 바꾸지 않았다.")
    add("")

    with open(os.path.join(HERE, "REPORT_KO.md"), "w", encoding="utf-8") as handle:
        handle.write("\n".join(out))
    print("REPORT_KO.md written,", len(out), "lines")
    return 0


if __name__ == "__main__":
    sys.exit(main())
