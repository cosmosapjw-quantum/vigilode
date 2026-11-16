#!/usr/bin/env python3
"""Loop node V06: render REPORT_KO.md from RESULTS.json.

Every number in the report is read from RESULTS.json (built by
parse_results.py from the raw logs); only the prose is written here.

usage: python3 -I make_report.py
"""
import json
from pathlib import Path

NODE = Path(__file__).resolve().parent


def num(value, digits=3):
    if value is None:
        return "-"
    if isinstance(value, bool):
        return "예" if value else "아니오"
    if isinstance(value, int):
        return f"{value:,}"
    return f"{value:.{digits}f}"


def sci(value, digits=2):
    return "-" if value is None else f"{value:.{digits}e}"


def yes(value):
    return "-" if value is None else ("예" if value else "아니오")


def table(header, rows):
    lines = ["| " + " | ".join(header) + " |", "|" + "|".join("---" for _ in header) + "|"]
    lines += ["| " + " | ".join(str(cell) for cell in row) + " |" for row in rows]
    return "\n".join(lines)


def tol(value):
    return f"{value:.0e}".replace("e-0", "e-").replace("e+0", "e")


def main():
    results = json.loads((NODE / "RESULTS.json").read_text())
    predeclared = json.loads((NODE / "PREDECLARED.json").read_text())
    stage = results["stage_a"]
    verdict = stage["verdict"]
    cells = stage["cells"]
    explore = results["exploratory"]
    runs = results["runs"]
    env = next(
        (value for key, value in sorted(results["environment"].items()) if key.startswith("environment_stage_a")), {}
    )

    declared_runs = [run for name, run in runs.items() if name.startswith("a_")]
    exploratory_runs = [run for name, run in runs.items() if name.startswith("x_")]
    audited = [run for run in runs.values() if "residual_audit" in run]
    audit_checks = sum(run["residual_audit"]["checks"] for run in audited)
    audit_failures = sum(run["residual_audit"]["failures"] for run in audited)
    audit_max = max((run["residual_audit"]["max_ratio"] for run in audited), default=None)
    declared_audited = [run for run in declared_runs if "residual_audit" in run]
    declared_checks = sum(run["residual_audit"]["checks"] for run in declared_audited)
    declared_failures = sum(run["residual_audit"]["failures"] for run in declared_audited)
    not_ok = [run["name"] for run in runs.values() if run["status"] not in ("OK", "SETUP")]
    instruction_ratios = [
        cell["B_vs_C_none"]["instructions_user_audit_off"]["candidate_over_baseline"]
        for cell in cells
        if cell["B_vs_C_none"].get("instructions_user_audit_off")
    ]
    krylov_only = [
        cell["B_vs_C_none"].get("krylov_vector_ratio_baseline_over_candidate_exploratory") for cell in cells
    ]
    krylov_only_families = {}
    for cell in cells:
        krylov_only_families.setdefault(cell["family"], []).append(
            cell["B_vs_C_none"].get("krylov_vector_ratio_baseline_over_candidate_exploratory")
        )
    krylov_only_passing = sum(
        1 for values in krylov_only_families.values() if all(v is not None and v >= 1.5 for v in values)
    )

    later_stage_shares = []
    for cell in cells:
        b_stages = cell["B_vs_C_none"]["baseline"].get("krylov_vectors_per_attempt_by_stage") or []
        c_stages = cell["B_vs_C_none"]["candidate"].get("krylov_vectors_per_attempt_by_stage") or []
        later_stage_shares += [c / b for b, c in list(zip(b_stages, c_stages))[1:] if b]
    jacobi_cost = []
    for cell in cells:
        plain = cell["B_vs_C_none"]["baseline"].get("jvp")
        jacobi = cell["B_vs_C_jacobi_secondary"]["baseline"].get("jvp")
        if plain and jacobi:
            jacobi_cost.append(jacobi / plain)
    adr = [cell for cell in cells if cell["family"] == "adr2d"]

    out = []
    w = out.append
    w("# V06 공유 Krylov basis: 실측 보고")
    w("")
    w("- node `V06_SHARED_KRYLOV_BASIS`, loop L1 wave 2, 2026-11-17 (KST)")
    w(f"- 사전등록: `PREDECLARED.json` (sha256 `{env.get('predeclared_sha256', '-')}`), 첫 측정 전에 commit")
    w(f"- stage a 측정 commit `{env.get('git_head', '-')}` (추적 파일 변경 {env.get('git_dirty_tracked', '-')}건), "
      f"binary sha256 `{env.get('binary_sha256', '-')}`")
    w(f"- 환경: {env.get('cpu', '-')}, {env.get('rustc', '-')}, {env.get('perf', '-')}, taskset {env.get('taskset', '-')}")
    w("- 이 문서의 수치는 전부 `make_report.py`가 `RESULTS.json`에서 읽어 채웠다. `RESULTS.json`은 "
      "`parse_results.py`가 `logs/`의 실행 출력만으로 만든다.")
    exploratory_env = {
        key: value for key, value in results["environment"].items() if key.startswith("environment_exploratory")
    }
    if exploratory_env:
        heads = sorted({value.get("git_head", "-") for value in exploratory_env.values()})
        binaries = sorted({value.get("binary_sha256", "-") for value in exploratory_env.values()})
        dirty = sorted({value.get("git_dirty_tracked", "-") for value in exploratory_env.values()})
        w(f"- 탐색 실행: commit {', '.join('`' + head + '`' for head in heads)}, binary sha256 "
          f"{', '.join('`' + binary + '`' for binary in binaries)}. 일부 그룹 시작 시점에 추적 파일 변경이 "
          f"{'/'.join(dirty)}건 있었다(`parse_results.py`와 계약 테스트 파일로, 측정 binary에 들어가지 않는다; binary 해시가 그룹 간에 같다).")
    w("")

    w("## 1. 결론")
    w("")
    w(f"- **Stage a: {verdict['verdict']}.** frozen 기준(JVP 비 ≥ 1.5, step 비 ≤ 1.05)을 두 tolerance 모두에서 만족한 family는 "
      f"3개 중 {verdict['families_passing']}개다. 필요한 수는 {verdict['families_needed']}개.")
    if "median_family_level_jvp_ratio" in verdict:
        w(f"- family별 JVP 비(두 tolerance 중 작은 값)의 중앙값은 {num(verdict['median_family_level_jvp_ratio'])}. "
          f"1.2 미만이면 v2를 취소한다는 조항은 "
          f"{'발동했다' if verdict['v2_cancelled_by_median_rule'] else '발동하지 않았다'}.")
    w("- stage a를 통과하지 못했으므로 **v2 kernel(`shared_basis_v2.rs`)은 만들지 않았다.** "
      f"최종 판별식은 **{results['final']['verdict']}**이다.")
    w(f"- 원 잔차 감사: 선언 실행 {num(declared_checks)}건의 stage solve 중 실패 {num(declared_failures)}건 "
      f"(탐색 실행 포함 {num(audit_checks)}건 중 {num(audit_failures)}건). "
      f"잔차/허용치의 최댓값은 {num(audit_max, 6)}.")
    if instruction_ratios:
        w(f"- instructions: 병합 kernel을 쓰는 C는 B의 {num(min(instruction_ratios), 0)}~{num(max(instruction_ratios), 0)}배다. "
          "병합 kernel이 basis 벡터를 하나 더할 때마다 n×k 행렬을 처음부터 다시 분해하기 때문이다.")
    w("- 판정에 쓰지 않는 탐색 결과 가운데 결정에 중요한 것은 8.3절(선형 허용치)과 8.4절(병합 hook)이다. "
      "그 자료에서 JVP 수를 좌우하는 것은 basis 공유가 아니라 선형 stopping test다.")
    w(f"- 사전등록 수정: {len(predeclared.get('amendments', []))}건.")
    w("")

    w("## 2. 무엇을 쟀나")
    w("")
    w("세 arm이 같은 선형 stopping test를 쓴다. 보호된 matrix-free driver의 규칙이다: scaled 계 "
      "`(I - h gamma J) U = rhs`에서 **true residual**(새 JVP 한 번으로 계산)의 2-norm이 "
      "`max(gamma*atol, rtol*||rhs||_2)` 이하일 때만 해를 받는다. 값은 routed production 경로의 "
      "`rtol = 1e-10`, `atol = 1e-14`, restart 40, stage solve당 Arnoldi 벡터 200개 한도.")
    w("")
    for line in predeclared["stopping_test"]["protected_source_lines"]:
        w(f"- `{line}`")
    w(f"- `{predeclared['stopping_test']['parameters_source']}`")
    w("")
    w("| arm | 내용 |")
    w("|---|---|")
    w("| A | 보호 driver 그대로 (`integrate_rodas5p_mf_fast_observed_gmres_into`). 잔차 검사는 40열 restart pass가 끝날 때만 한다. |")
    w("| B0 | 대조용. client driver + per-stage GMRES, A와 같은 검사 주기. client 경로가 A를 재현하는지 본다. |")
    w("| B | per-stage GMRES client (frozen 기준의 기준선). 매 Arnoldi 열마다 projected residual을 보고, 기준에 닿으면 true residual로 확인한다. |")
    w("| C | shared-basis client. attempt마다 basis 하나(최대 200개)를 `rodas5p_krylov::shared_basis::SharedBasis`로 8개 solve가 공유한다. kernel이 수렴을 보고하면 true residual로 확인하고, 못 끝낸 solve는 B의 GMRES로 처음부터 푼다(병합 hook과 같은 fallback). |")
    w("")
    w("- 확인용 true residual JVP(solve당 1회)는 B와 C 모두 arm의 JVP에 넣었다. 감사용 JVP는 별도 counter에 있고 arm 합계에 넣지 않았다.")
    w("- JVP 비 = JVP(B)/JVP(C), step 비 = attempts(C)/attempts(B). family는 두 tolerance 모두 통과해야 통과다 (`PREDECLARED.json`).")
    w("- 병합 kernel의 수렴 판정식은 `h_gamma*||r||_w <= theta` 하나뿐이라, client가 균일 weight "
      "`sqrt(n)/(h_gamma*tau)`와 `theta = 1`로 보호 규칙의 임계값을 넣었다. kernel 파일은 수정하지 않았다.")
    w("")
    rows = []
    for family in predeclared["families_stage_a"]:
        reference = results["references"].get(family["id"], {})
        rows.append(
            [
                f"`{family['id']}`",
                family["n"],
                f"[{family['t_span'][0]:g}, {family['t_span'][1]:g}]",
                family["description"].split(":")[0],
                sci((reference.get("uncertainty") or {}).get("max_abs")),
                num((reference.get("method") or {}).get("nfev")),
            ]
        )
    w(table(["family", "n", "구간", "문제", "기준해 불확도 (max abs)", "DOP853 RHS 평가 수"], rows))
    w("")
    w("- 기준해: 각 family를 numpy로 다시 구현해 scipy DOP853(rtol 1e-13)으로 적분했다. 불확도는 scipy Radau(rtol 1e-10) 결과와의 "
      "성분별 최대 차이다. Rust 코드와 선형 solver를 공유하지 않는다.")
    w(f"- 최종 시험용으로 선언한 네 번째 family `{predeclared['family_final_only']['id']}`는 기준해만 만들어 두었고 실행하지 않았다"
      "(최종 시험이 NOT_RUN).")
    w(f"- 실행 수: 선언 실행 {len(declared_runs)}개, 탐색 실행 {len(exploratory_runs)}개. 실행 하나가 프로세스 하나이고 "
      "`perf stat -e instructions:u` 아래에서 돌았다. counter는 결정적이므로 반복은 1회, wall time은 `diagnostic_only`.")
    if not_ok:
        w(f"- 완료하지 못한 실행: {', '.join('`' + name + '`' for name in not_ok)} (8절 참조).")
    else:
        w("- 모든 실행이 끝까지 적분했다.")
    w("")

    w("## 3. frozen 판별식: stage a")
    w("")
    rows = []
    for cell in cells:
        primary = cell["B_vs_C_none"]
        b, c = primary["baseline"], primary["candidate"]
        rows.append(
            [
                f"`{cell['family']}`",
                tol(cell["rtol"]),
                num(b.get("attempts")),
                num(c.get("attempts")),
                num(b.get("jvp")),
                num(c.get("jvp")),
                f"**{num(cell['jvp_ratio'])}**",
                num(cell["step_ratio"]),
                yes(primary["residual_audit_clean"]["baseline"]) + " / " + yes(primary["residual_audit_clean"]["candidate"]),
                "통과" if cell["passes"] else "미달",
            ]
        )
    w(table(["family", "rtol", "attempts B", "attempts C", "JVP B", "JVP C", "JVP 비", "step 비", "감사 통과 B / C", "판정"], rows))
    w("")
    rows = []
    for row in verdict["families"]:
        rows.append(
            [
                f"`{row['family']}`",
                num(row["jvp_ratio_by_rtol"]["1e-4"]),
                num(row["jvp_ratio_by_rtol"]["1e-6"]),
                num(row["family_level_jvp_ratio_min_over_rtol"]),
                "통과" if row["passes"] else "미달",
            ]
        )
    w(table(["family", "JVP 비 (1e-4)", "JVP 비 (1e-6)", "family 값 (작은 쪽)", "판정"], rows))
    w("")
    w(f"- 통과 family {verdict['families_passing']}개 < {verdict['families_needed']}개: **{verdict['verdict']}**.")
    if "median_family_level_jvp_ratio" in verdict:
        w(f"- 중앙값 {num(verdict['median_family_level_jvp_ratio'])} (6개 cell의 중앙값은 "
          f"{num(verdict.get('median_of_the_six_cells_reported_only'))}, 참고용).")
    same_steps = all(cell["step_ratio"] == 1.0 for cell in cells)
    error_ratios = [cell["B_vs_C_none"].get("error_ratio_candidate_over_baseline") for cell in cells]
    if same_steps and all(value is not None for value in error_ratios):
        w(f"- 여섯 cell 모두 step 비는 정확히 1이고, C와 B의 달성 오차 비는 {min(error_ratios):.6f}~{max(error_ratios):.6f}이다. "
          "두 arm은 같은 궤적을 따른다.")
    else:
        w("- step 비와 달성 오차 비는 위 표와 4절 참조.")
    w("")
    w("### 계수 방식에 대한 민감도 (판정에 쓰지 않음)")
    w("")
    w("사전등록한 JVP에는 solve당 1회의 확인용 true residual이 들어 있다(두 arm 모두). 이를 빼고 Krylov 벡터만 세면:")
    w("")
    rows = [
        [f"`{cell['family']}`", tol(cell["rtol"]), num(cell["jvp_ratio"]), num(value)]
        for cell, value in zip(cells, krylov_only)
    ]
    w(table(["family", "rtol", "JVP 비 (사전등록)", "Krylov 벡터 비 (탐색)"], rows))
    w("")
    w(f"- Krylov 벡터만 세면 1.5 이상을 두 tolerance에서 넘는 family가 {krylov_only_passing}개가 된다. "
      f"`adr2d`는 사전등록 계수로 {', '.join(num(cell['jvp_ratio']) for cell in adr)}이고 "
      f"Krylov 벡터만 세면 "
      f"{', '.join(num(cell['B_vs_C_none'].get('krylov_vector_ratio_baseline_over_candidate_exploratory')) for cell in adr)}이다. "
      "판정은 사전등록대로 FAIL이지만, `adr2d`의 통과 여부는 이 계수 선택에 달려 있다는 점을 밝힌다.")
    w("- 어느 계수로도 최종 판별식의 JVP 조건(B의 0.5배 이하, 즉 비 2.0 이상)에 닿는 family는 없다. v2는 벡터 하나의 비용을 "
      "줄이는 것이지 벡터 수를 줄이지 않으므로(기각 후 재시도 제외, 8.2절), stage a를 통과로 읽더라도 최종 PASS는 이 자료로는 기대할 수 없다. "
      "이것은 측정이 아니라 추론이다.")
    thetas = [1.0 / value for value in krylov_only if value]
    if thetas:
        w(f"- 최종 판별식의 instructions 조건(B의 0.8배 이하)도 같은 방향이다. V05의 비용 모형(`theory/THEORY.md` S7)에서 basis 하나의 "
          "직교화 작업은 basis 여덟 개의 `8*theta^2`배이고, `theta`는 C와 B의 Krylov 벡터 수의 비다. 측정된 `theta`는 "
          f"{min(thetas):.3f}~{max(thetas):.3f}로 모두 `1/sqrt(8) = 0.354`보다 크므로 모형상 직교화 작업은 "
          f"{8 * min(thetas) ** 2:.1f}~{8 * max(thetas) ** 2:.1f}배가 된다. 이 family들은 JVP가 stencil 한 번이라 싸다. "
          "모형에 측정값을 넣은 추론이며 v2의 instructions를 잰 것이 아니다.")
    w("")

    w("## 4. B는 A의 공정한 대역인가")
    w("")
    rows = []
    for cell in cells:
        a, b0 = cell["arm_A"], cell["arm_B0"]
        b = cell["B_vs_C_none"]["baseline"]
        c = cell["B_vs_C_none"]["candidate"]
        stand = cell["client_as_stand_in_for_A"]
        identical = all((stand.get("B0_over_A") or {}).get("counters_identical", {"x": False}).values())
        rows.append(
            [
                f"`{cell['family']}`",
                tol(cell["rtol"]),
                f"{num(a.get('attempts'))} / {num(b0.get('attempts'))} / {num(b.get('attempts'))}",
                f"{num(a.get('jvp'))} / {num(b0.get('jvp'))} / {num(b.get('jvp'))}",
                yes(identical),
                sci((stand.get("B0_over_A") or {}).get("final_state_max_abs_difference"), 1),
                num((stand.get("B_over_A") or {}).get("jvp")),
                " / ".join(
                    num((arm.get("error") or {}).get("max_tolerance_units"), 4) for arm in (a, b, c)
                ),
            ]
        )
    w(table(
        ["family", "rtol", "attempts A / B0 / B", "JVP A / B0 / B", "B0 counter = A", "상태 차 B0-A", "JVP B/A", "달성 오차 A / B / C (허용치 단위)"],
        rows,
    ))
    w("")
    b0_identical = sum(
        1
        for cell in cells
        if all(((cell["client_as_stand_in_for_A"].get("B0_over_A") or {}).get("counters_identical") or {"x": False}).values())
    )
    w(f"- B0(같은 검사 주기)는 여섯 cell 중 {b0_identical}개에서 A와 JVP 수, Arnoldi 반복 수, matvec 수, 진단 matvec 수, RHS 호출 수가 "
      "모두 같다. 최종 상태는 반올림 수준으로만 다르다(두 driver가 stage 우변을 scaled/unscaled 형태로 다르게 만든다).")
    b_attempts_equal = sum(
        1 for cell in cells if (cell["client_as_stand_in_for_A"].get("B_over_A") or {}).get("attempts") == 1.0
    )
    b_error = [
        (cell["client_as_stand_in_for_A"].get("B_over_A") or {}).get("error_ratio") for cell in cells
    ]
    if all(value is not None for value in b_error):
        w(f"- B는 여섯 cell 중 {b_attempts_equal}개에서 A와 attempt 수가 같고, 달성 오차 비 B/A는 "
          f"{min(b_error):.6f}~{max(b_error):.6f}이다. JVP가 적은 것은 pass 중간에 멈출 수 있어서다. "
          "A를 기준선으로 쓰면 C의 이득에 이 효과가 섞이므로 frozen 비교의 기준선은 B다.")
    w("- 달성 오차는 기준해 대비 `max_i |y_i - ref_i| / (atol + rtol |ref_i|)`.")
    w("")

    w("## 5. Krylov 차원, basis 메모리, instructions")
    w("")
    rows = []
    for cell in cells:
        primary = cell["B_vs_C_none"]
        b, c = primary["baseline"], primary["candidate"]
        instructions = primary.get("instructions_user_audit_off") or {}
        rows.append(
            [
                f"`{cell['family']}`",
                tol(cell["rtol"]),
                f"{num(b.get('krylov_vectors_per_solve_mean'), 1)} / {num(b.get('krylov_vectors_per_solve_max'))}",
                f"{num(c.get('basis_dim_mean_at_attempt_end'), 1)} / {num(c.get('basis_dim_max'))}",
                num(c.get("fallback_solves")),
                f"{num((b.get('gmres_storage_bytes') or 0) / 1e6, 2)} / {num((c.get('shared_storage_bytes') or 0) / 1e6, 2)}",
                sci(instructions.get("baseline")),
                sci(instructions.get("candidate")),
                num(instructions.get("candidate_over_baseline"), 0),
            ]
        )
    w(table(
        ["family", "rtol", "B: solve당 벡터 평균 / 최대", "C: attempt 끝 basis 차원 평균 / 최대", "C fallback solve", "저장 MB (B / C)", "instructions B", "instructions C", "C/B"],
        rows,
    ))
    w("")
    w("- B의 저장은 GMRES basis 41개 벡터와 작업 벡터, C의 저장은 병합 kernel의 Q, Z, 분해용 행렬(각 n×200)이다. C는 fallback용 B 저장도 함께 갖는다.")
    w("- instructions는 감사를 끈 실행의 프로세스 전체 값이다. 문제 구성만 하는 SETUP 실행을 빼도 비는 같다(`RESULTS.json`).")
    w("- `bruss1d`에서는 basis가 한도 200에 닿아 fallback이 생겼다(8.2절의 C300 참조).")
    deterministic = sum(
        1
        for cell in cells
        if cell["B_vs_C_none"].get("audit_on_equals_audit_off", {}).get("baseline") is True
        and cell["B_vs_C_none"].get("audit_on_equals_audit_off", {}).get("candidate") is True
    )
    w(f"- 감사를 켠 실행과 끈 실행은 여섯 cell 중 {deterministic}개에서 B, C 모두 counter, 통계, 최종 상태가 비트 단위로 같다 "
      "(같은 arm을 두 번 돌린 것이기도 하다).")
    allocation = (results.get("verification") or {}).get("allocation")
    if allocation:
        w(f"- 할당 검사(`stage_clients_krylov_alloc`, 단독 binary): client 구성 {len(allocation['clients'])}가지, 실행 {allocation['runs']}개, "
          f"warm-up 뒤 accepted step {num(allocation['steady_steps_measured'])}개에서 step당 할당 최대 "
          f"{allocation['steady_allocations_max_per_step']}회(합계 {allocation['steady_allocations_total']}). "
          f"fallback이 일어난 실행 {allocation['runs_with_fallback_solves']}개 포함. 작은 개발용 문제(n = 64~128)에서 잰 것이다.")
    w("")

    w("## 6. 이득이 작은 이유: stage별 분해")
    w("")
    w("attempt당 새 Krylov 벡터 수를 solve 순서(stage 1~8)별로 평균한 값이다.")
    w("")
    rows = []
    for cell in cells:
        primary = cell["B_vs_C_none"]
        for label, arm in (("B", primary["baseline"]), ("C", primary["candidate"])):
            stages = arm.get("krylov_vectors_per_attempt_by_stage") or []
            rows.append([f"`{cell['family']}`", tol(cell["rtol"]), label] + [num(value, 1) for value in stages])
    w(table(["family", "rtol", "arm"] + [f"s{i}" for i in range(1, 9)], rows))
    w("")
    w("- stage 1은 C도 B와 같은 수의 벡터가 필요하다(basis가 비어 있다).")
    if later_stage_shares:
        w(f"- stage 2~8에서 C가 새로 만드는 벡터는 B의 {num(100 * min(later_stage_shares), 0)}~"
          f"{num(100 * max(later_stage_shares), 0)}%다(위 표의 stage별 C/B). stage 우변에는 `f(y + sum a_ij U_j)`의 비선형 증분이 들어 있고, "
          "이것은 앞 stage들의 Krylov 공간 밖에 있다. 허용치가 1e-10이라 공간 밖 성분이 작아도 거의 처음부터 줄여야 한다.")
    w("- 우변이 앞 우변과 그 해의 선형결합이면(선형 문제의 다음 stage가 그렇다) 공유가 잘 된다: 계약 테스트 "
      "`single_solves_meet_the_rule_and_the_shared_basis_is_reused`에서 그런 우변은 새 벡터가 첫 solve의 1/4 이하다. "
      "선언한 세 family는 모두 비선형이다. n >= 1e3의 선형 문제는 재지 않았다.")
    w("")

    w("## 7. 보조 측정: 같은 preconditioner (right Jacobi)")
    w("")
    rows = []
    for cell in cells:
        secondary = cell["B_vs_C_jacobi_secondary"]
        b, c = secondary["baseline"], secondary["candidate"]
        rows.append(
            [
                f"`{cell['family']}`",
                tol(cell["rtol"]),
                num(b.get("jvp")),
                num(c.get("jvp")),
                num(secondary.get("jvp_ratio_baseline_over_candidate")),
                num(secondary.get("step_ratio_candidate_over_baseline")),
                yes(secondary["residual_audit_clean"]["baseline"]) + " / " + yes(secondary["residual_audit_clean"]["candidate"]),
            ]
        )
    w(table(["family", "rtol", "JVP B", "JVP C", "JVP 비", "step 비", "감사 통과 B / C"], rows))
    w("")
    w("- B와 C에 같은 right Jacobi(`diag(W)`)를 걸었다. A는 preconditioner를 받지 않으므로 A 대조는 없다. "
      "코드의 `JacobiPreconditioner`는 dense 행렬에서만 만들 수 있어서, client가 문제의 해석적 Jacobian 대각으로 같은 대각 scaling을 한다.")
    if jacobi_cost:
        w(f"- 이 family들에서 Jacobi는 B를 돕지 못한다: B의 JVP가 무전처리 대비 {num(min(jacobi_cost))}~{num(max(jacobi_cost))}배다. "
          "공유 이득도 무전처리보다 작다. right preconditioning에서는 stage 해 `U_j = D^-1 u_j`가 `W D^-1`의 Krylov 공간 밖으로 "
          "나가기 때문이다(계약 테스트 `single_solves_meet_the_rule_and_the_shared_basis_is_reused`).")
    w("")

    w("## 8. 탐색 실행 (어느 판정에도 넣지 않음)")
    w("")
    w("### 8.1 나중 binary로 선언 실행 재현")
    w("")
    for row in explore.get("declared_runs_repeated_with_the_later_binary", []):
        w(f"- `{row['declared_run']}` 대 `{row['repeated_run']}`: counter, 통계, 최종 상태 일치 = "
          f"{yes(row['identical_counters_statistics_and_state'])}")
    w("- 탐색 옵션을 넣은 뒤의 binary가 선언 arm의 동작을 바꾸지 않았음을 보인다.")
    w("")

    w("### 8.2 basis 크기와 기각 후 유지")
    w("")
    rows = []
    for row in explore.get("shared_basis_variants", []):
        for tag, label in (("C60", "C60 (병합 기본값 60)"), ("C300", "C300 (한도 300)"), ("Ckeep", "Ckeep (기각 후 유지)")):
            entry = row.get(tag)
            if entry is None:
                continue
            against = entry.get("against_B") or {}
            rows.append(
                [
                    f"`{row['family']}`",
                    tol(row["rtol"]),
                    label,
                    entry.get("status", "-"),
                    num(entry.get("jvp")),
                    num(against.get("jvp_ratio_baseline_over_candidate")),
                    num(against.get("step_ratio_candidate_over_baseline")),
                    num(entry.get("basis_dim_max")),
                    num(entry.get("fallback_solves")),
                    num(entry.get("basis_kept")),
                    num(entry.get("residual_audit_failures")),
                ]
            )
    w(table(["family", "rtol", "변형", "상태", "JVP", "JVP 비 (B 대비)", "step 비", "basis 최대", "fallback", "유지 횟수", "감사 실패"], rows))
    w("")
    def variant_ratios(tag, family=None):
        return [
            (row[tag].get("against_B") or {}).get("jvp_ratio_baseline_over_candidate")
            for row in explore.get("shared_basis_variants", [])
            if tag in row and (family is None or row["family"] == family)
        ]

    c60 = [value for value in variant_ratios("C60") if value is not None]
    c300 = [value for value in variant_ratios("C300", "bruss1d") if value is not None]
    keep_gain = [
        row["Ckeep"].get("jvp_ratio_declared_C_over_this")
        for row in explore.get("shared_basis_variants", [])
        if "Ckeep" in row and row["Ckeep"].get("basis_kept")
    ]
    if c60:
        w(f"- C60: 병합 hook의 기본 크기 60으로는 보호 stopping test 아래에서 basis가 금방 차서 fallback이 많다. "
          f"JVP 비는 {min(c60):.3f}~{max(c60):.3f}로, 이득이 거의 없거나 손해다.")
    if c300:
        w(f"- C300: 한도를 300으로 올려도 `bruss1d`의 JVP 비는 {', '.join(f'{value:.3f}' for value in c300)}로 1.5에 못 미친다. "
          "선언한 한도 200이 `bruss1d`의 판정을 바꾼 것은 아니다.")
    if keep_gain:
        w("- Ckeep: local-error 기각 뒤의 재시도에서 Jacobian은 그대로이고 shift만 바뀌므로 Q와 Z = JQ를 그대로 쓴다(재사용에 JVP 0). "
          f"기각이 있는 `bruss1d`에서 선언한 C보다 JVP가 "
          f"{', '.join(f'{100 * (1 - 1 / value):.1f}%' for value in keep_gain)} 적다. "
          "기각이 없는 `grayscott2d`, `adr2d`에서는 C와 같다.")
    w("")

    w("### 8.3 선형 허용치를 바꾸면 (세 arm에 같은 값)")
    w("")
    w("보호 규칙의 `rtol = 1e-10`만 바꾸고 나머지는 그대로 둔 실행이다. 마지막 행(1e-10)은 선언 실행이다.")
    w("")
    rows = []
    for block in explore.get("linear_tolerance_sweep", []):
        for row in sorted(block["by_linear_rtol"], key=lambda item: -item["linear_rtol"]):
            a, b, c = row.get("A") or {}, row.get("B") or {}, row.get("C") or {}
            compare = row.get("B_vs_C") or {}
            rows.append(
                [
                    f"`{block['family']}`",
                    tol(block["rtol"]),
                    tol(row["linear_rtol"]),
                    f"{num(a.get('attempts'))} / {num(b.get('attempts'))} / {num(c.get('attempts'))}",
                    num(a.get("jvp")),
                    num(b.get("jvp")),
                    num(c.get("jvp")),
                    num(compare.get("jvp_ratio_baseline_over_candidate")),
                    num(b.get("krylov_vectors_per_solve_mean"), 1),
                    " / ".join(num(arm.get("error_max_tolerance_units"), 3) for arm in (a, b, c)),
                ]
            )
    w(table(
        ["family", "ODE rtol", "선형 rtol", "attempts A / B / C", "JVP A", "JVP B", "JVP C", "JVP 비 B/C", "B: solve당 벡터", "달성 오차 A / B / C"],
        rows,
    ))
    w("")
    summary = {}
    for block in explore.get("linear_tolerance_sweep", []):
        for row in block["by_linear_rtol"]:
            entry = summary.setdefault(row["linear_rtol"], {"b_jvp": [], "b_err": [], "ratio": [], "err": [], "step": []})
            against = row.get("B_at_this_tolerance_against_B_at_1e-10") or {}
            compare = row.get("B_vs_C") or {}
            if against.get("jvp_ratio_baseline_over_candidate"):
                entry["b_jvp"].append(1.0 / against["jvp_ratio_baseline_over_candidate"])
            if against.get("error_ratio_candidate_over_baseline") is not None:
                entry["b_err"].append(against["error_ratio_candidate_over_baseline"])
            for key, name in (
                ("ratio", "jvp_ratio_baseline_over_candidate"),
                ("err", "error_ratio_candidate_over_baseline"),
                ("step", "step_ratio_candidate_over_baseline"),
            ):
                if compare.get(name) is not None:
                    entry[key].append(compare[name])

    def span(values, digits=2):
        return "-" if not values else f"{min(values):.{digits}f} ~ {max(values):.{digits}f}"

    rows = [
        [
            tol(linear),
            span(entry["b_jvp"]) if linear != 1e-10 else "1 (기준)",
            span(entry["b_err"]) if linear != 1e-10 else "1 (기준)",
            span(entry["ratio"]),
            span(entry["err"]),
            span(entry["step"]),
        ]
        for linear, entry in sorted(summary.items(), reverse=True)
    ]
    w("여섯 cell의 범위로 요약하면:")
    w("")
    w(table(
        ["선형 rtol", "JVP B / JVP B(1e-10)", "오차 B / 오차 B(1e-10)", "JVP 비 B/C", "오차 C / 오차 B", "step 비 C/B"],
        rows,
    ))
    w("")
    w("- B는 선형 허용치를 1e-8, 1e-6으로 풀어도 달성 오차가 거의 그대로이고 JVP는 표의 비율만큼 준다. "
      "1e-4에서는 오차와 attempt 수가 변하기 시작한다. A는 pass를 끝까지 돌기 때문에 허용치를 풀어도 JVP가 거의 줄지 않는다.")
    w("- 공유의 JVP 비는 선형 허용치를 1e-10에서 1e-6으로 풀수록 커진다. 그러나 1e-6부터는 **같은 잔차 허용치인데도 C의 달성 오차가 "
      "B보다 나빠지는 cell이 생기고**(표의 오차 C/B), 1e-4에서는 attempt도 늘어 JVP 비가 다시 작아지는 cell이 있다. "
      "공유 basis에서는 여덟 stage의 잔차가 모두 같은 부분공간에 직교하므로 stage 오차가 서로 상관되어 더해지는 것으로 보인다"
      "(확인하지 않은 설명이다). 즉 느슨한 허용치에서의 큰 JVP 비는 matched error가 아니다.")
    w("- 이 절은 탐색이며 frozen 기준(보호 규칙 1e-10)을 대신하지 않는다.")
    w("")

    w("### 8.4 병합 hook을 그 자체의 stopping test로 돌리면")
    w("")
    w("보호 driver에 `set_shared_basis(Some(max_dim 60, theta 1e-3))`을 켠 실행이다. hook은 driver의 GMRES 규칙이 아니라 "
      "`h*gamma*||r||_WRMS <= 1e-3`(step controller의 weight)으로 stage 해를 받는다.")
    w("")
    rows = []
    for row in explore.get("merged_hook_with_its_own_stopping_test", []):
        hook, a = row["hook_as_merged"] or {}, row["A_as_shipped"] or {}
        rows.append(
            [
                f"`{row['family']}`",
                tol(row["rtol"]),
                f"{num(a.get('attempts'))} / {num(hook.get('attempts'))}",
                num(a.get("jvp")),
                num(hook.get("jvp")),
                num((row.get("A_vs_hook") or {}).get("jvp_ratio_baseline_over_candidate"), 1),
                num((row.get("B_vs_hook") or {}).get("jvp_ratio_baseline_over_candidate"), 1),
                f"{num(a.get('error_max_tolerance_units'), 3)} / {num(hook.get('error_max_tolerance_units'), 3)}",
            ]
        )
    w(table(["family", "rtol", "attempts A / hook", "JVP A", "JVP hook", "JVP A/hook", "JVP B/hook", "달성 오차 A / hook"], rows))
    w("")
    w("- hook은 A보다 JVP가 훨씬 적고 달성 오차는 비슷하다. 이 차이의 대부분은 공유가 아니라 **느슨한 stopping test**에서 온다: "
      "같은 계에서 hook이 받는 잔차는 보호 임계값의 수만 배다(계약 테스트 "
      "`the_merged_hook_accepts_residuals_far_above_the_protected_threshold`, 실패하도록 쓴 "
      "`merged_hook_solution_meets_the_protected_threshold`의 출력은 `logs/merged_hook_failing_test.log`).")
    w("- 따라서 hook을 켠 실행과 끈 실행의 JVP 차이는 공유의 효과를 재지 못한다. 병합 계열의 계약 테스트 "
      "`shared_basis_on_heat_uses_fewer_jvps_at_matched_steps`가 하는 비교가 이것이다. "
      "공유의 효과를 보려면 같은 규칙의 per-stage GMRES와 비교해야 하며, 8.3절이 2-norm 상대 규칙으로 그 비교를 보여 준다. "
      "hook의 WRMS 규칙을 쓰는 per-stage GMRES는 만들지 않았다.")
    w("")

    stiff = explore.get("bruss1d_stiff_1e-4")
    w("### 8.5 강하게 stiff한 경우 (표준 Brusselator, 길이 1, 512 cell)")
    w("")
    if stiff:
        rows = []
        for tag, label in (("A_none", "A"), ("B_none_audit1", "B"), ("C60_none_audit1", "C60")):
            entry = stiff.get(tag)
            if entry is None:
                continue
            rows.append(
                [
                    label,
                    entry.get("status", "-"),
                    num(entry.get("attempts")),
                    num(entry.get("rejected")),
                    num(entry.get("linear_solve_failures")),
                    num(entry.get("jvp")),
                    num(entry.get("error_max_tolerance_units"), 3),
                ]
            )
        w(table(["arm", "상태", "attempts", "기각", "선형 solve 실패", "JVP", "달성 오차"], rows))
        w("")
        compare = stiff.get("B_vs_C60") or {}
        w(f"- JVP 비 B/C60 = {num(compare.get('jvp_ratio_baseline_over_candidate'))}, "
          f"step 비 = {num(compare.get('step_ratio_candidate_over_baseline'))}.")
        w("- 이 영역에서는 출하 경로(무전처리 GMRES(40), 200벡터, 1e-10)가 Krylov 한도 때문에 step을 줄인다. "
          "병합 kernel은 벡터당 O(n k^2)이라 수백 차원 basis로는 시간이 너무 걸려 기본 크기 60만 쟀다.")
    else:
        w("- 실행하지 않았다.")
    w("")

    w("## 9. 한계")
    w("")
    nfev = {
        family: num(((results["references"].get(family) or {}).get("method") or {}).get("nfev"))
        for family in ("grayscott2d", "adr2d", "bruss1d")
    }
    w("- stage a family 셋은 사전에 shift 수 `h*gamma*rho(J)`가 1~10 정도가 되게 잡았다. 실제로 약하게 stiff하다: explicit DOP853이 "
      f"rtol 1e-13에서 RHS {nfev['grayscott2d']}번(`grayscott2d`), {nfev['adr2d']}번(`adr2d`), {nfev['bruss1d']}번(`bruss1d`)에 끝난다. "
      "강하게 stiff한 문제에 대해서는 8.5절의 한 cell 말고는 말할 수 없다.")
    few = {
        family: [num(cell["B_vs_C_none"]["baseline"].get("attempts")) for cell in cells if cell["family"] == family]
        for family in ("grayscott2d", "adr2d")
    }
    w(f"- `grayscott2d`(attempt {', '.join(few['grayscott2d'])})와 `adr2d`({', '.join(few['adr2d'])})는 attempt 수가 적고, "
      "초기 step 1e-6에서 최대 5배씩 올라가는 구간이 그 안에 들어 있다. 비는 결정적이지만 적은 step의 합이다.")
    w("- JVP에는 확인용 true residual이 들어 있다(3절의 민감도 표).")
    w("- instructions와 메모리는 병합 kernel(벡터당 재분해, n×k 행렬 3개)의 값이다. v2가 있었다면 달랐을 것이다. v2는 만들지 않았다.")
    w("- preconditioner는 Jacobi 하나만 쟀고, 이 family들에서는 효과가 없다. 강한 preconditioner 아래의 공유는 재지 않았다.")
    w("- wall time은 공유 호스트에서 고정 core 없이 한 번 잰 값이라 `diagnostic_only`다.")
    w("- step 사이 basis 유지는 구현하지도 재지도 않았다(10절은 측정에서 나온 추론이다).")
    w("")

    w("## 10. basis를 step 사이에도 유지해야 하는가")
    w("")
    w("권고: **아니오.** 기각 뒤 재시도에서만 유지할 가치가 있다.")
    w("")
    if later_stage_shares:
        w("- 같은 attempt 안, 즉 Jacobian과 shift가 모두 같고 Z = JQ가 정확한 상황에서도 다음 stage는 per-stage GMRES가 쓰는 벡터의 "
          f"{num(100 * min(later_stage_shares), 0)}~{num(100 * max(later_stage_shares), 0)}%를 다시 만든다(6절). "
          "step이 바뀌면 우변은 그보다 더 달라진다.")
    w("- step이 바뀌면 Jacobian이 바뀌므로 저장한 Z = JQ는 못 쓴다. 벡터 k개를 넘기려면 JVP k번으로 Z를 다시 만들어야 하고, "
      "stage 1이 처음부터 푸는 데 드는 벡터 수(5절의 B 평균)가 넘길 basis 크기(5절의 C 평균)보다 훨씬 작다. "
      "Z를 갱신하지 않으면 다른 Jacobian으로 푸는 것이 되어 방법이 바뀐다(차수 조건은 V05 S4).")
    w("- 기각 뒤 재시도는 Jacobian이 같고 shift만 다르므로 Q, Z를 JVP 없이 그대로 쓸 수 있다. 효과는 기각 비율만큼이다(8.2절 Ckeep).")
    w("- JVP를 줄이려는 것이 목적이면 8.3절과 8.4절이 가리키는 쪽, 즉 선형 stopping test를 ODE 허용치에 맞추는 것이 공유보다 훨씬 크다. "
      "이것은 보호 driver의 허용치를 바꾸는 일이라 이 node의 범위 밖이며, 별도 사전등록이 필요하다.")
    w("")

    w("## 11. 파일과 재현")
    w("")
    w("- client: `crates/rodas5p-integrators/src/stage_clients/krylov.rs` "
      "(`KrylovStageClient::per_stage`, `KrylovStageClient::shared_basis`, `ResidualAuditClient`, `StageStoppingRule`)")
    w("- 테스트: `tests/stage_clients_krylov_contracts.rs`, `tests/stage_clients_krylov_alloc.rs` (단독 binary), "
      "문제 정의와 arm 실행기 `tests/stage_clients_krylov_problems.rs`, 측정 프로그램 `tests/stage_clients_krylov_measure.rs`")
    w("- 증거: `PREDECLARED.json`, `reference/` (기준해와 생성 스크립트), `logs/` (실행별 `.log`, `.perf`, 환경 기록), "
      "`run_measurements.sh`, `parse_results.py`, `RESULTS.json`, `make_report.py`")
    w("")
    w("검증 명령(결과는 node의 구조화 출력에 있다):")
    w("")
    w("```")
    w("cargo +1.94.1 test --locked -j 4 --release -p rodas5p-integrators \\")
    w("  --test stage_clients_krylov_contracts --test stage_clients_krylov_alloc --test stage_clients_krylov_measure")
    w("cargo +1.94.1 test --locked -j 4 --release -p rodas5p-integrators --test ppsw1_6b_contracts --test ptsw2b_contracts")
    w("cargo +1.94.1 clippy --locked -j 4 --release -p rodas5p-integrators --lib --test stage_clients_krylov_problems \\")
    w("  --test stage_clients_krylov_contracts --test stage_clients_krylov_alloc --test stage_clients_krylov_measure --no-deps -- -D warnings")
    w("python3 -I research/production_loop_20261116/check_checkpoint.py")
    w("```")
    w("")
    w("측정 재현:")
    w("")
    w("```")
    w("cd research/production_loop_20261116/shared_krylov")
    w("python3 -I reference/make_reference.py --family bruss1d --out reference/bruss1d.json")
    w("./run_measurements.sh stage_a          # 이미 있는 log는 건너뛴다")
    w("./run_measurements.sh exploratory      # exploratory_tolerance, exploratory_hook, exploratory_capacity, exploratory_recheck")
    w("python3 -I parse_results.py && python3 -I make_report.py")
    w("```")
    w("")
    (NODE / "REPORT_KO.md").write_text("\n".join(out) + "\n")
    print(f"wrote REPORT_KO.md ({len(out)} lines)")


if __name__ == "__main__":
    main()
