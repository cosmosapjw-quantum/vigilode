#!/usr/bin/env python3
"""Render REPORT_KO.md of loop L1 node V03 part A from RECEIPT.json.

Every number in the report is read from RECEIPT.json, which build_receipt.py parsed from the
raw logs. The API example is copied from the documentation of `MethodChoice` in
crates/vigilode/src/selection/frontend.rs (a doctest of the facade suite), so it is code
that was compiled and run.

Usage (from the worktree root):
    python3 -I research/production_loop_20261116/unified_frontend/build_report.py
"""

import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]


def strict_number(text):
    raise ValueError(f"non-finite JSON constant {text}")


def doc_example():
    """The doctest of `MethodChoice`, without the hidden lines."""
    source = (ROOT / "crates/vigilode/src/selection/frontend.rs").read_text(encoding="utf-8")
    lines = source.splitlines()
    end = lines.index("pub enum MethodChoice<'a> {")
    fences = [i for i in range(end) if lines[i] == "/// ```"]
    start, stop = fences[-2], fences[-1]
    body = []
    for line in lines[start + 1 : stop]:
        text = line[4:] if line.startswith("/// ") else ""
        if text.startswith("# "):
            continue
        body.append(text)
    return "\n".join(body)


def yes(flag):
    return "예" if flag else "아니오"


def fmt(value):
    if isinstance(value, float):
        return f"{value:.4g}"
    return str(value)


QUALIFICATION = {
    "production": "production",
    "scoped-calibration": "scoped calibration (production 아님)",
    "implemented-unqualified": "구현됨, 미자격",
    "reference-implementation-only": "참조 구현 전용",
}

FORMS = {
    "dense": "Dense",
    "banded": "Banded",
    "jvp": "Jvp",
    "sparse-jvp-banded": "SparseJvp(band)",
    "sparse-jvp-general": "SparseJvp(일반)",
    "finite-difference": "FD",
}


def method_rows(catalog):
    rows = []
    for entry in catalog:
        d = entry["descriptor"]
        available = entry["availability"]["state"] == "available"
        variants = d["variants"]
        features = [
            name
            for name, key in (
                ("mass", "supports_mass"),
                ("event", "supports_events"),
                ("dense output", "supports_dense_output"),
            )
            if any(v[key] for v in variants)
        ]
        forms = ", ".join(FORMS[f] for f in entry["jacobian_forms"]) or "불필요 (RHS만)"
        rows.append(
            "| `{id}` | {avail} | {forms} | {features} | {band} | {vatol} | {stages}/{order} | "
            "{qual} | {auto} |".format(
                id=d["method_id"],
                avail="가능" if available else "불가",
                forms=forms if available else "-",
                features=", ".join(features) or "없음",
                band=yes(entry["declared_band"]),
                vatol=yes(entry["vector_atol"]),
                stages=d["stages"],
                order=d["order"],
                qual=QUALIFICATION[d["qualification"]],
                auto=yes(entry["auto_rankable"]) if available else "-",
            )
        )
    return rows


def main():
    receipt = json.loads(
        (HERE / "RECEIPT.json").read_text(encoding="utf-8"), parse_constant=strict_number
    )
    d = receipt["discriminators"]
    d1, d1b = d["D1_decision_equality"], d["D1_phase_B"]
    d2, d3, d4 = (
        d["D2_adapter_bitwise_equality"],
        d["D3_refusal_before_any_callback"],
        d["D4_default_route_golden"],
    )
    suite = receipt["full_suite"]
    lints = receipt["lints"]
    peer = receipt["peer_defects"]
    catalog = receipt["method_catalog"]
    phase = receipt["phase_reached"]
    out = []
    w = out.append

    w("# V03 통합 frontend (part A) 보고")
    w("")
    w(
        "이 문서는 `build_report.py`가 `RECEIPT.json`에서 만든 것이다. 숫자는 모두 "
        "`logs/*.log`(실행된 명령의 원 출력)에서 `build_receipt.py`가 파싱했다. 손으로 적은 숫자는 없다."
    )
    w("")
    w(f"- base: `{receipt['base_commit']}`")
    w(f"- 측정에 쓰인 코드 commit: `{receipt['code_commit_of_the_runs']}`")
    w("- 사전등록: `PREDECLARED.json` (첫 실행 전에 commit `00d32764`)")
    w("- wall time 주장은 없다. 로그의 경과 시간은 `diagnostic_only`다.")
    w("")

    w("## 1. 결론")
    w("")
    w(f"- 도달 단계: **Phase {phase}**.")
    if phase == "B":
        w(
            "  `GeneralOde::select`의 본문이 method catalog에서 유도한 선택"
            "(`MethodRegistry::general_selection`)으로 바뀌었고, 이전 본문은 "
            "`GeneralOde::select_reference`로 남아 테스트 oracle이 된다. 기본 라우팅의 결정은 "
            "한 건도 달라지지 않았다."
        )
    w("- 동결 판별 기준(PREREGISTRATION.md V03 행)의 결과:")
    w("")
    w("| 기준 | 측정 | 판정 |")
    w("|---|---|---|")
    m = d1["measured"]
    w(
        f"| D1 `route(General)` = `GeneralOde::select`, 전수 격자, callback 0 | "
        f"{m['equal_to_select']} / {m['entries']} 일치, golden 파일과 {m['equal_to_golden_file']} / "
        f"{m['golden_file_entries']} 일치, callback {m['callback_entries']}회 | {d1['verdict']} |"
    )
    m = d2["measured"]
    w(
        f"| D2 adapter가 직접 호출과 bit 동일 | adapter {m['adapters_equal']} / {m['adapters']}, "
        f"사례 {m['cases_bitwise_equal']} / {m['cases']} | {d2['verdict']} |"
    )
    m = d3["measured"]
    w(
        f"| D3 거부는 어떤 callback보다 먼저 | `route` {m['route_calls']}회 "
        f"(계획 {m['route_plans']}, 거부 {m['route_refusals']}) callback {m['route_callback_entries']}회; "
        f"거부된 `solve` {m['refused_solve_calls']}회 callback {m['solve_callback_entries']}회 | {d3['verdict']} |"
    )
    m = d4["measured"]
    w(
        f"| D4 기본 경로 golden 불변 | 두 번 연속 실행 모두 exit {m['run1']['exit_code']}/{m['run2']['exit_code']}, "
        f"golden 파일 {len(m['golden_files_identical_bytes'])}개 byte 동일, base 대비 diff "
        f"{len(m['git_diff_of_golden_and_oracle_against_base'])}줄 | {d4['verdict']} |"
    )
    w("")
    m = d1b["measured"]["extension_grid"]
    w(
        f"- Phase B 조건: catalog 유도 선택과 이전 본문이 확장 격자 {m['select_equal_to_reference']} / "
        f"{m['entries']}건, golden 격자 {d1b['measured']['golden_grid_equal_to_reference_body']}건에서 "
        f"일치하고 callback은 {m['callback_entries']}회다. 판정 {d1b['verdict']}."
    )
    fixed = sum(1 for r in peer["reproductions"] if r["base_sources"] == "FAILED" and r["after_fix"] == "ok")
    w(
        f"- PeerOde 결함(COD-02, COD-04) 재현 테스트 {len(peer['reproductions'])}건: base 소스에서 "
        f"{sum(1 for r in peer['reproductions'] if r['base_sources'] == 'FAILED')}건 실패, 수정 후 {fixed}건 통과."
    )
    w(
        f"- facade 전체 suite: {suite['passed']} passed, {suite['failed']} failed, {suite['ignored']} ignored "
        f"(테스트 바이너리 {suite['binaries']}개, exit {suite['exit_code']})."
    )
    if suite["failed_tests"]:
        w(f"  실패한 테스트: {', '.join('`' + t + '`' for t in suite['failed_tests'])}")
    w(
        f"- clippy exit {lints['clippy_exit_code']}, fmt exit {lints['fmt_check_exit_code']}, "
        f"rustdoc exit {lints['doc_exit_code']}, loop checker exit {lints['loop_checker_exit_code']}."
    )
    edits = receipt["edits_of_existing_tests"]
    w(
        "- 기존 테스트(`unified_method_capabilities.rs`, `unified_method_sessions.rs`) 수정: "
        + ("없음. 두 파일은 그대로 통과한다." if not edits else ", ".join(e["path"] for e in edits))
    )
    outside = receipt["changed_paths_outside_the_task_allowlist"]
    w(
        "- 허용 경로 밖의 변경: "
        + ("없음." if not outside else ", ".join(outside))
        + " 보호 경로의 base 대비 diff: "
        + ("없음." if not receipt["protected_paths_diff_against_base"] else "있음(아래 참조).")
    )
    w("")

    w("## 2. API")
    w("")
    w(
        "진입점은 하나다. `route`가 선언된 사실만 읽어 계획 또는 타입이 있는 거부를 돌려주고"
        "(사용자 callback 0회), `solve`가 계획을 실행하고, `view()`가 방법과 무관한 결과를 빌려준다. "
        "아래 예는 `MethodChoice` 문서의 doctest이며 facade suite에서 컴파일되고 실행된다."
    )
    w("")
    w("```rust")
    w(doc_example())
    w("```")
    w("")
    w("| 항목 | 내용 |")
    w("|---|---|")
    w("| `MethodChoice::General(policy)` | 오늘의 기본 결정. `GeneralOde::select`와 같다 |")
    w("| `MethodChoice::Explicit(id)` | catalog의 방법 하나를 id로 지정 |")
    w("| `MethodChoice::Auto(policy)` | opt-in 순위 결정. 기본값이 아니다 |")
    w("| `MethodRegistry::route` / `route_with` | 순수 함수. `MethodRoutePlan` 또는 `MethodRouteRefusal` |")
    w("| `MethodRegistry::solve` | 계획 실행. 입력 오류는 callback 전에 `MethodSolveError` |")
    w(
        "| `SolutionView` | `times()`, `states()`, `output_grid()`, `work()`, `route()`/`route_id()`"
        "(실행 기록에서 읽은 경로), `failure()`(타입이 있는 사유), `at(t)` |"
    )
    w("| `MethodRegistry::catalog` / `entry` | 방법 표(아래) |")
    w("| `MethodParams` | PPSW1 옵션, Lawson의 선언된 선형 flow |")
    w("")
    w(
        "기존 frontend(`Rodas5p`, `Dopri5`, `GeneralOde`, `AutomaticOde`, `SwitchingSolver`, "
        "`ProductionSolver`, `PeerOde`, `LawsonDopri5`, `ExplicitSession`, `Dopri5Ensemble`)는 "
        "그대로 동작한다. `lib.rs`는 수정하지 않았다."
    )
    w("")

    w("## 3. 방법 표")
    w("")
    w(
        "`MethodRegistry::catalog()`의 내용이다(테스트 `catalog_extends_the_registry_and_states_what_is_unavailable`가 "
        "출력). 앞의 6개는 U06 `descriptors()`와 같고, 뒤의 3개가 이번에 facade에서 도달 가능해진 방법이다."
    )
    w("")
    w(
        "| id | 실행 | Jacobian 형태 | 지원 기능 | band 선언 | vector atol | 단계/차수 | qualification | Auto 순위 대상 |"
    )
    w("|---|---|---|---|---|---|---|---|---|")
    out.extend(method_rows(catalog))
    w("")
    rodas = next(e for e in catalog if e["descriptor"]["method_id"] == "rodas5p")
    w(
        "RODAS5P 항목의 정확한 경로 variant(`MethodEntry::route_variants`). U06 descriptor에는 권한 variant "
        "3개만 있고 banded와 sequential이 없었다(COD-03). 계획의 variant 이름과 general 선택의 matrix-free 권한, "
        "dense 차원 상한 판정이 이 표를 읽는다."
    )
    w("")
    w("| variant | n x n 행렬 저장 | matrix-free | mass | event | dense output |")
    w("|---|---|---|---|---|---|")
    for v in rodas["route_variants"]:
        w(
            f"| `{v['label']}` | {yes(v['dense_matrix'])} | {yes(v['matrix_free'])} | "
            f"{yes(v['supports_mass'])} | {yes(v['supports_events'])} | {yes(v['supports_dense_output'])} |"
        )
    w("")
    w("adapter가 bit 동일해야 하는 직접 호출과 D2 결과:")
    w("")
    w("| 방법 | 직접 호출 | 사례 | bit 동일 | 판정 |")
    w("|---|---|---|---|---|")
    direct = {e["descriptor"]["method_id"]: e["direct_call"] for e in catalog}
    for a in receipt["adapters"]:
        call = f"`{direct.get(a['method'], '')}`"
        if a["test"].startswith("d2_dopri5"):
            call += " 및 `vigilode::GeneralOde::solve(problem, span, y0, options, policy)`"
        w(f"| `{a['method']}` | {call} | {a['cases']} | {a['cases_bitwise_equal']} | {a['verdict']} |")
    w("")
    w(
        "비교한 양: 모든 출력 시각과 상태의 bit, 성공 여부, 수락/거절 step 수, attempts, driver work "
        "counter 전체. RODAS5P는 facade 경로 6개(dense-fast, banded-fast, matrix-free, fast-dense, "
        "fast-banded-dense, sequential-dense)를 모두 포함한다."
    )
    w("")

    w("## 4. Auto가 하는 것과 하지 않는 것")
    w("")
    ranking = receipt["auto"]["ranking"]
    fallback = receipt["auto"]["fallback"]
    w("하는 것:")
    w("")
    w(
        "- 후보는 adapter가 있고, 선언된 사실과 호출자가 허용한 `CapabilitySet` 아래에서 admission을 "
        "통과한 catalog 항목이다."
    )
    w(
        "- 순서: (1) regime 적합성(stiff로 선언하면 stiff용 방법만, 그 외에는 explicit 방법이 먼저), "
        "(2) qualification, (3) catalog 순서. 계획에 catalog 항목마다 한 행씩 순위 또는 거부 사유가 들어 있다."
    )
    w(
        f"- 예(Robertson, stiff, 전체 허용, 미자격 포함): {' > '.join('`' + i + '`' for i in ranking['stiff_full_permission'])}."
    )
    w(
        f"- 같은 문제에 기존 `GeneralOdePolicy`의 권한만 주면: {' > '.join('`' + i + '`' for i in ranking['stiff_general_policy'])} "
        "(PPSW1 starter는 Newton 권한이 없어 거부, 미자격 방법은 제외)."
    )
    w(
        "- 실측 증거는 호출자가 넘긴 `CostEvidence`만 쓴다. 정책이 `use_measured_evidence`를 켰을 때만, "
        "같은 등급 안에서만 순서를 바꾸며, 그 증거를 얻는 데 든 callback 단위는 결과에 기록된다 "
        f"(예: {' > '.join('`' + i + '`' for i in ranking['with_measured_evidence'])}, "
        f"기록된 callback 단위 {ranking['evidence_callback_units']})."
    )
    w(
        "- fallback은 정책이 켰을 때만 한다. 실패한 시도 다음 순위의 방법을 `t0`에서 다시 실행하고, "
        "모든 시도를 결과에 남긴다. 실패한 시도의 work는 합계에 포함된다."
    )
    w("")
    w("fallback 예(Robertson, regime unknown, 시도 예산 500):")
    w("")
    w("| 시도 | 경로 | 성공 | 사유 | prefix 끝 | RHS 호출 | 수락 step |")
    w("|---|---|---|---|---|---|---|")
    for index, attempt in enumerate(fallback["attempts"], 1):
        work = attempt["work"]
        w(
            f"| {index} | `{attempt['route']}` | {yes(attempt['success'])} | {attempt['failure'] or '-'} | "
            f"{fmt(attempt['prefix_end'])} | {work['rhs_calls']} | {work['accepted_steps']} |"
        )
    total = fallback["total_work"]
    w(f"| 합계 | | | | | {total['rhs_calls']} | {total['accepted_steps']} |")
    w("")
    w("하지 않는 것:")
    w("")
    w("- 기본값이 아니다. `GeneralOde`, `Rodas5p` 등 어떤 frontend도 Auto를 부르지 않는다(OD-L1-3).")
    w("- `route`는 아무것도 측정하지 않는다. stiffness 탐지나 timing pilot이 없다.")
    w("- 호출자가 허용하지 않은 capability를 쓰는 방법으로 넘어가지 않는다.")
    w(
        f"  (미분 권한이 없으면 fallback 후보가 없어 시도는 {fallback['without_permission_attempts']}번으로 끝난다.)"
    )
    w("- 실패한 시도의 상태를 이어 쓰지 않는다(방법 전환이 아니라 `t0` 재시작).")
    w("- Lawson처럼 문제의 우변 의미를 바꾸는 방법은 순위에 넣지 않는다(명시 선택만).")
    w("- Auto가 더 빠르거나 정확하다는 주장은 이 노드에 없다.")
    w("")

    w("## 5. 사용할 수 없는 것과 이유")
    w("")
    for entry in catalog:
        if entry["availability"]["state"] != "available":
            w(
                f"- `{entry['descriptor']['method_id']}`: adapter 없음. catalog에 적힌 사유: "
                f"\"{entry['availability']['reason']}\". 즉 matrix-free common-W backend는 "
                "`ppsw1::Session`의 stage backend일 뿐이고, 끝에서 끝까지 도는 driver와 matrix-free starter가 없다."
            )
    w(
        "- BDF, Radau: `Jacobian::Dense`와 `Jacobian::Jvp`만 내린다. FD, banded, sparse-JVP, band 선언, "
        "mass, event, dense output, vector atol은 타입이 있는 거부다."
    )
    w("- PEXPRB54s4: `Jacobian::Jvp`인 autonomous 문제만. 그 외는 거부.")
    w("- lagged RODAS5P: `Jacobian::Dense`만.")
    w("- PPSW1: `Jacobian::Dense` 또는 `Jacobian::Jvp`, band 선언 없음, scalar atol.")
    w("- Lawson: 선언된 flow(`MethodParams::lawson_flow`)가 없으면 거부.")
    w(
        "- 이번 후보 목록에 없어 여전히 facade에서 닿지 않는 것: small/batch RODAS5P, SABR, homotopy, "
        "transactional q1/q2, PTSW2B, `vigilode-coupled`."
    )
    w("- 다른 방법으로 조용히 대체하는 adapter는 없다.")
    w("")

    w("## 6. 라우팅 권위: Phase A와 Phase B")
    w("")
    w(
        "- Phase A: `route(General)`은 `GeneralOde::select`를 호출한다. golden oracle의 결정 격자"
        f"({d1['measured']['entries']}건)를 같은 순서로 다시 열거해 `select`, golden 파일과 비교했다."
    )
    w(
        "- Phase B: COD-03이 요구한 요청 확장을 `MethodFacts`로 넣었다(차원, provider 형태, band 선언, "
        "autonomy, 시간 미분, mass, event, dense output, interior-error 모드, atol 종류). catalog의 RODAS5P "
        "항목에는 facade의 driver 계열 6개를 경로 variant로 넣었다(banded-fast, sequential-dense 포함, 3절 표). "
        "general 선택의 receipt(`implicit_route`)는 이전과 같이 facade의 `Rodas5p::capabilities`가 답한 경로다."
    )
    m = d1b["measured"]["extension_grid"]
    w(
        f"- 확장 격자는 golden 격자에 sparse-JVP provider 두 형태를 더한 {m['entries']}건이다. 두 구현이 "
        f"읽는 모든 술어의 곱이므로 전수 비교다. 결과 종류 {m['distinct_outcomes']}가지."
    )
    debug = d1b["measured"]["debug_profile_run"]
    w(
        f"- debug profile 실행(debug assertion 켜짐): exit {debug['exit_code']}, {debug['passed']} passed. "
        "catalog 유도 경로가 reference 본문으로 물러난 적이 없다."
    )
    w("")
    w("COD-03의 13개 probe 사례(불일치 9건 포함)를 새 경로로 다시 실행한 결과:")
    w("")
    w("| 사례 | `select` | `route(General)` 일치 | 같은 권한의 Auto |")
    w("|---|---|---|---|")
    for row in receipt["cod03_audit_cases"]:
        w(
            f"| {row['case']} | `{row['select']}` | {yes(row['route_general_equal'])} | "
            f"{('`' + row['auto_under_the_same_policy'] + '`') if row['auto_under_the_same_policy'] else '거부'} |"
        )
    w("")
    w(
        "mass, event, dense output 세 경우는 general surface 자체의 제한이다. 방법(RODAS5P)은 지원하므로 "
        "Auto와 Explicit에서는 admit된다. 나머지 거부는 양쪽이 같다."
    )
    w("")
    w(
        f"D3 격자에서 `route(Explicit(\"rodas5p\"))`의 계획은 "
        f"{d3['measured']['rodas5p_plans_equal_to_the_route_that_runs'][0]} / "
        f"{d3['measured']['rodas5p_plans_equal_to_the_route_that_runs'][1]}건이 실행되는 driver 계열과 같고, "
        "그 variant 행은 요청한 mass, event, dense output을 지원한다. 거부는 `Rodas5p::capabilities`의 거부와 같다."
    )
    w("")
    w(
        "남긴 것: `MethodRequest`, `MethodRegistry::descriptors()`, `query()`(U06 표)는 바꾸지 않았다. "
        "`MethodRequest`의 struct literal을 허용 경로 밖의 `tests/geometry_mol_contracts.rs`가 쓰고, "
        "U06 테스트가 6개 항목을 고정하기 때문이다. `query()`는 provider 형태를 모르는 거친 권한 표로 남고, "
        "라우팅은 `route`가 한다."
    )
    w("")

    w("## 7. PeerOde 결함 수정 (COD-02, COD-04)")
    w("")
    w(
        "`ppsw1.rs`는 보호 파일이라 건드리지 않았다. 수정은 `selection/peer.rs`(adapter)에만 있다. "
        "각 결함은 감사 probe에서 만든 테스트로 먼저 실패를 확인했다(`logs/failing_first_peer_defects.log`)."
    )
    w("")
    w("| 재현 테스트 | base 소스 | 수정 후 |")
    w("|---|---|---|")
    for r in peer["reproductions"]:
        w(f"| `{r['test']}` | {r['base_sources']} | {r['after_fix']} |")
    w("")
    w("| 사례 | base: 성공, h0, 오차(허용오차 단위) | 수정 후: 성공, h0, 오차(허용오차 단위), Peer step |")
    w("|---|---|---|")
    for key, label in (("cod02_p3", "y' = 20 cos 20t"), ("cod02_p3b", "y' = 200 cos 200t")):
        b, a = peer["forcing_cases"][key]["base_sources"], peer["forcing_cases"][key]["after_fix"]
        w(
            f"| {label} | {yes(b['success'])}, {fmt(b['h0'])}, {fmt(b['error_in_tolerance_units'])} | "
            f"{yes(a['success'])}, {fmt(a['h0'])}, {fmt(a['error_in_tolerance_units'])}, "
            f"{a['accepted_peer_steps']} |"
        )
    w("")
    w("수정 내용:")
    w("")
    w(
        "- starter 구간 오차 제어: step doubling. `h0`의 starter 상태를 `h0/2` starter 두 번과 비초기 node "
        "5개에서 비교하고(`ppsw1::error_norm`, 호출자 허용오차), 추정이 1 이하일 때만 쓴다. 아니면 `h0`를 "
        "반으로 줄인다(최대 40회, 그 뒤 타입이 있는 실패). 추정이지 상계가 아니다."
    )
    w("- `h0`가 유한한 양수가 아니면 callback 전에 `InvalidInput`.")
    w("- admission이 provider 형태를 읽는다. JVP provider는 JVP 권한이 있어야 하고, band 선언은 admission에서 거부된다.")
    w(
        "- 실패 사유는 `PeerFailure`(타입)이고, callback 오류는 종류(recoverable/fatal), callback, 메시지를 "
        "그대로 가진다. `failure` 문자열은 그 표시 형식으로 남겼다(기존 호출자 호환)."
    )
    w(
        "- Jacobian/JVP의 recoverable 오류는 그 사유로 멈춘다(선형화 점이 step 크기와 무관해 반감으로 고칠 수 없다)."
    )
    w("- 끝점은 `t_end`에 정확히 착지시키고, `success`는 착지 여부다.")
    w("")
    rows = peer["u07_trajectory_regression_exploratory"]
    unchanged = sum(1 for r in rows if r["trajectory_unchanged"])
    w(
        f"탐색적 회귀 확인: U07 계약 문제 4개 설정 중 {unchanged}개에서 수락 궤적(시각과 상태의 FNV-1a digest)이 "
        "수정 전후 bit 동일하다. starter 제어가 첫 `h0`를 받아들였기 때문이다. startup work는 제어용 "
        f"starter 2회만큼 늘었다(Jacobian build {rows[0]['startup_jacobian_builds'][0]} -> "
        f"{rows[0]['startup_jacobian_builds'][1]}, RHS {rows[0]['startup_rhs_calls'][0]} -> "
        f"{rows[0]['startup_rhs_calls'][1]}). 동결 기준이 아니다."
    )
    w("")

    w("## 8. 실행한 명령")
    w("")
    w("| 로그 | exit | passed | failed | ignored |")
    w("|---|---|---|---|---|")
    for command in receipt["commands"]:
        tests = command["tests"]
        counted = tests["binaries"] > 0
        w(
            f"| `{command['log']}` | {command['exit_code']} | {tests['passed'] if counted else '-'} | "
            f"{tests['failed'] if counted else '-'} | {tests['ignored'] if counted else '-'} |"
        )
    w("")
    w(
        "`logs/failing_first_peer_defects.log`의 exit 101은 의도한 결과다(수정 전 소스에서 재현 테스트가 실패). "
        "각 로그의 첫 줄들에 명령, commit, 시각, load average가 있다."
    )
    w("")

    w("## 9. 발견한 것과 한계")
    w("")
    w(
        "- U06 registry의 `radau-iia3`는 2단계/3차라고 적혀 있었다. `radau.rs`의 기본 설정은 3단계 Radau IIA "
        "5차(내장 3차 추정)다. descriptor를 코드에 맞게 고쳤고 테스트가 코드 값과 비교한다."
    )
    w(
        "- BDF, Radau, PEXPRB, lagged driver는 실패했을 때 마지막 수락 상태를 내주지 않는다. 그래서 view의 "
        "prefix는 도달한 출력 표본(끝점 일정이면 `t0`)뿐이다. fatal callback 오류에서는 driver가 기록 없이 "
        "오류만 돌려주므로, adapter가 타입이 있는 사유와 callback 경계에서 센 호출 수를 남긴다. 더 나은 "
        "prefix는 `rodas5p-integrators` 수정이 필요하다(이 노드 범위 밖)."
    )
    corner = receipt["executed_route_vs_capability_query"]
    w(
        "- 새로 확인한 facade 결함(고치지 않음, `capabilities.rs`와 `solve.rs`의 RODAS5P 경로는 이 노드 범위 밖): "
        "일반(비 band) sparse-JVP provider에서 `Rodas5p::capabilities`는 항상 `MatrixFree`라고 답하지만, "
        "`Rodas5p::solve`는 dense output이면 fast dense driver를, vector atol이면 dense fast driver를 실행하고 "
        "Jacobian을 JVP 열 n개로 조립한다. 실행 결과:"
    )
    w("")
    w("  | 요청 | capability query | 계획한 경로 | 실행된 경로 | entry point | Jacobian build | JVP 호출 |")
    w("  |---|---|---|---|---|---|---|")
    for row in corner["requests"]:
        w(
            f"  | {row['request']} | {row['capability_query_route']} | `{row['planned_route']}` | "
            f"`{row['executed_route']}` | `{row['entry_point']}` | {row['jacobian_builds']} | {row['jvp_calls']} |"
        )
    w("")
    w(
        "  registry의 계획은 실제로 실행되는 driver와 그 dense-matrix 필요를 적는다(view는 실행 기록에서 경로를 "
        "다시 읽어 확인한다). 그래서 Auto는 dense-matrix 권한 없이 이 driver로 가지 않는다. 격자에서 capability "
        f"query가 실행되지 않는 driver를 답한 요청은 {d3['measured']['rodas5p_capability_query_names_another_driver_than_runs']}건이다."
    )
    g = corner["general_surface_vector_atol"]
    w(
        f"- 같은 결함의 기본 경로 쪽 귀결(바꾸지 않음, 기본 라우팅 불변): 일반 sparse-JVP + vector atol에서 "
        f"`GeneralOde::select`는 `{g['select_implicit_route']}`를 고르고 dense 차원 상한을 적용하지 않는다"
        f"(예: n = {g['dimension']}, 상한 {g['max_dense_dimension']}). 실제로는 `{g['planned_route_of_route_general']}`가 "
        "실행되어 n x n 행렬을 만든다. 이전 본문도 같은 결정을 한다. 소유자 결정이 필요하다."
    )
    w(
        "- dense provider에 band를 선언하면 보호 router가 `(t0, y0)`에서 callback으로 band를 검증한다. 그 거부"
        "(`SolveError::StructureRefused`)는 라우팅 거부가 아니라 직접 호출과 같은 solve 단계 오류다."
    )
    w(
        "- `GeneralOde::select`는 이제 호출마다 catalog를 만든다(작은 heap 할당). 수치 workspace 할당이나 "
        "callback은 없다. hot loop에서 불리지 않는다."
    )
    w("- Auto와 fallback에는 성능 측정이 없다. 이 노드의 증거는 결정 동등성과 bit 동등성뿐이다.")
    w("- 출력 격자를 호출자가 지정하는 기능은 넣지 않았다(직접 호출과의 bit 동등성을 유지하기 위해 각 방법의 고유 격자를 그대로 보고).")
    w("")

    w("## 10. 재현")
    w("")
    w("```sh")
    w("sh research/production_loop_20261116/unified_frontend/run_checks.sh \\")
    w("    contracts existing golden1 golden2 suite clippy fmt doc checker diff")
    w("python3 -I research/production_loop_20261116/unified_frontend/build_receipt.py")
    w("python3 -I research/production_loop_20261116/unified_frontend/build_report.py")
    w("```")
    w("")
    (HERE / "REPORT_KO.md").write_text("\n".join(out), encoding="utf-8")
    print(f"REPORT_KO.md: {len(out)} lines")
    return 0


if __name__ == "__main__":
    sys.exit(main())
