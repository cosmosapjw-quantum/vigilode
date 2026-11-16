#!/usr/bin/env python3
"""Build AMDAHL.json and REPORT_TABLES.md of loop L1 node V04 from RESULTS.json.

The formulas are those fixed in PREDECLARED.json (key "amdahl"); they are repeated in the output.
Every number is computed here from RESULTS.json, which build_results.py parses from the raw logs.
"""
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
LEVELS = (4, 5, 6)
WORKERS = (2, 4, 6)
LAGS = (2, 4)
BAND = (0.15, 0.92)
STEP_WALL_MIN_US = 500.0


def strict_load(path):
    def invalid(value):
        raise ValueError("nonfinite JSON constant: " + value)
    return json.loads(Path(path).read_text(), parse_constant=invalid)


def bound(saved):
    """Whole-step speed-up when the fraction `saved` of the step disappears."""
    return 1.0 / (1.0 - saved) if saved < 1.0 else None


def fractions_of(cell):
    """(basis, f) with f the fractions the bounds use, or (None, None)."""
    basis = cell.get("fraction_basis")
    matrix_free = cell["driver"] == "MatrixFree"
    if basis == "instructions":
        ph = cell["instruction_counts"]["phase_fractions"]
        return basis, {
            "f_J": ph.get("jacobian", 0.0), "f_LU": ph.get("factor", 0.0),
            "f_S": ph.get("solve", 0.0), "f_R": ph.get("rhs", 0.0),
            "f_remainder": ph["remainder"],
        }
    if basis == "wall":
        ph = cell["phase_fractions"]
        if matrix_free:
            return basis, {
                "f_J": 0.0, "f_LU": 0.0, "f_S": ph["solve_intervals"], "f_R": ph["rhs"],
                "f_remainder": ph["other"], "f_JVP": ph["jvp"],
                "f_S_stages_2_to_8": ph["solve_intervals_stages_2_to_8"],
                "f_JVP_stages_2_to_8": ph["jvp_stages_2_to_8"],
                "f_orthogonalisation_and_vector_work_in_solves":
                    ph["krylov_orthogonalisation_and_vector_work_in_solves"],
            }
        return basis, {
            "f_J": ph["jacobian"], "f_LU": ph["factor"], "f_S": ph["solve"], "f_R": ph["rhs"],
            "f_remainder": ph["remainder"],
        }
    return None, None


def regime(cell):
    basis, f = fractions_of(cell)
    if basis is None:
        return None
    chain = f["f_R"] + f["f_S"]
    jlu = f["f_J"] + f["f_LU"]
    wall = cell.get("wall")
    step_us = (wall["plain_ns_per_accepted_step"]["median"] / 1000.0
               if wall and wall.get("plain_ns_per_accepted_step") else None)
    attempt_us = wall["plain_ns_per_attempt"]["median"] / 1000.0 if wall else None
    r = jlu / chain if chain > 0 else None
    in_band = r is not None and BAND[0] <= r <= BAND[1]
    slow_enough = step_us is not None and step_us >= STEP_WALL_MIN_US
    out = {
        "cell": cell["cell"],
        "n": cell["n"],
        "structure": cell["structure"],
        "problem": cell["problem"],
        "jacobian_source": cell["jacobian_source"],
        "rhs_multiplier": cell["rhs_multiplier"],
        "lu_mode": cell.get("lu_mode"),
        "admissibility": cell["admissibility"],
        "reduced_protocol": cell["reduced_protocol"],
        "fraction_basis": basis,
        "fractions": dict(f, f_chain=chain),
        "plain_wall_per_accepted_step_us": step_us,
        "plain_wall_per_attempt_us": attempt_us,
        "a_level_structured_tableau": {f"L{L}": bound((1.0 - L / 8.0) * chain) for L in LEVELS},
        "b_speculative_jacobian_lu": {
            "r": r,
            "saving": min(jlu, chain / 4.0),
            "bound": bound(min(jlu, chain / 4.0)),
            "bound_jacobian_only": bound(min(f["f_J"], chain / 4.0)),
            "r_in_0.15_0.92": in_band,
            "plain_wall_per_accepted_step_ge_500us": slow_enough,
            "gates_V07_stage_b": in_band and slow_enough,
        },
        "c_parallel_jacobian_assembly": {f"P{p}": bound(f["f_J"] * (1.0 - 1.0 / p)) for p in WORKERS},
        "d_lagged_lu": {f"k{k}": bound(f["f_LU"] * (1.0 - 1.0 / k)) for k in LAGS},
        "e_parallel_dense_lu": (
            {f"P{p}": bound(f["f_LU"] * (1.0 - 1.0 / p)) for p in WORKERS}
            if cell["structure"] == "dense" else None),
        "f_shared_krylov_basis": None,
        "g_parallel_rhs": {f"P{p}": bound(f["f_R"] * (1.0 - 1.0 / p)) for p in WORKERS},
        "g_parallel_rhs_exploratory": None,
    }
    if cell["driver"] == "MatrixFree" and basis == "wall":
        out["f_shared_krylov_basis"] = {
            "later_stage_solve_fraction": f["f_S_stages_2_to_8"],
            "later_stage_jvp_fraction": f["f_JVP_stages_2_to_8"],
            "bound": bound(f["f_S_stages_2_to_8"]),
            "bound_jvp_part_only": bound(f["f_JVP_stages_2_to_8"]),
        }
        out["g_parallel_rhs_exploratory"] = {
            "what": "right-hand side and JVP callbacks in parallel",
            **{f"P{p}": bound((f["f_R"] + f["f_JVP"]) * (1.0 - 1.0 / p)) for p in WORKERS}}
    elif cell["jacobian_source"] == "fd":
        out["g_parallel_rhs_exploratory"] = {
            "what": "right-hand sides of the stages and of the finite-difference Jacobian in parallel",
            **{f"P{p}": bound((f["f_R"] + f["f_J"]) * (1.0 - 1.0 / p)) for p in WORKERS}}
    return out


# ------------------------------------------------------------------------------------------------
# Markdown tables
# ------------------------------------------------------------------------------------------------

def pct(value):
    return "-" if value is None else f"{100.0 * value:.1f}"


def num(value, digits=2):
    return "-" if value is None else f"{value:.{digits}f}"


def us(value_ns):
    if value_ns is None:
        return "-"
    value = value_ns / 1000.0
    return f"{value:.3g}" if value < 100 else f"{value:.0f}"


def short_label(cell):
    text = {"authoritative": "AUTH", "diagnostic_only": "diag", "not_measured": "not run"}[
        cell["admissibility"]]
    if cell.get("reduced_protocol"):
        text += " (reduced)"
    return text


def table(header, rows):
    lines = ["| " + " | ".join(header) + " |", "|" + "|".join(["---"] * len(header)) + "|"]
    lines += ["| " + " | ".join(str(v) for v in row) + " |" for row in rows]
    return "\n".join(lines)


def tables(results, regimes):
    cells = results["cells"]
    by_id = {r["cell"]: r for r in regimes}
    parts = ["<!-- generated by build_amdahl.py from RESULTS.json and AMDAHL.json; do not edit -->"]

    direct = [c for c in cells if c["driver"] != "MatrixFree" and "wall" in c]
    parts.append("\n## T1. Wall anatomy, direct drivers (per attempt, medians over the timed repetitions)\n")
    parts.append(table(
        ["cell", "n", "LU", "attempts", "accepted", "plain us/attempt", "instr us/attempt",
         "J %", "LU %", "solve %", "RHS %", "rest %", "phase sum", "instr/plain", "identical",
         "load before/after", "label"],
        [[c["cell"], c["n"], c.get("lu_mode", "-"), c["run"]["attempts"], c["run"]["accepted_steps"],
          us(c["wall"]["plain_ns_per_attempt"]["median"]),
          us(c["wall"]["instrumented_ns_per_attempt"]["median"]),
          pct(c["phase_fractions"]["jacobian"]), pct(c["phase_fractions"]["factor"]),
          pct(c["phase_fractions"]["solve"]), pct(c["phase_fractions"]["rhs"]),
          pct(c["phase_fractions"]["remainder"]), num(c["sum_check"]["median"], 3),
          num(c["wall"]["instrumented_over_plain"]["ratio_of_medians"], 3),
          "yes" if c["identity"]["all_identical"] and c["identity"]["all_runs_same_integration"] else "NO",
          f"{c['load_average']['runner_before'][0]:.1f}/{c['load_average']['runner_after'][0]:.1f}",
          short_label(c)] for c in direct]))

    mf = [c for c in cells if c["driver"] == "MatrixFree" and "wall" in c]
    parts.append("\n## T2. Wall anatomy, matrix-free driver (per attempt, medians)\n")
    parts.append(table(
        ["cell", "n", "attempts", "accepted", "plain us/attempt", "RHS %", "JVP %",
         "solve intervals %", "of which orthogonalisation + vector work %", "other %",
         "sum (RHS+JVP)", "sum (RHS+intervals)", "instr/plain", "identical", "JVP/attempt",
         "Krylov iterations/solve", "label", "label with intervals"],
        [[c["cell"], c["n"], c["run"]["attempts"], c["run"]["accepted_steps"],
          us(c["wall"]["plain_ns_per_attempt"]["median"]),
          pct(c["phase_fractions"]["rhs"]), pct(c["phase_fractions"]["jvp"]),
          pct(c["phase_fractions"]["solve_intervals"]),
          pct(c["phase_fractions"]["krylov_orthogonalisation_and_vector_work_in_solves"]),
          pct(c["phase_fractions"]["other"]), num(c["sum_check"]["median"], 3),
          num(c["sum_check_with_solve_intervals"]["median"], 3),
          num(c["wall"]["instrumented_over_plain"]["ratio_of_medians"], 3),
          "yes" if c["identity"]["all_identical"] and c["identity"]["all_runs_same_integration"] else "NO",
          num(c["counts"]["per_attempt"]["jvp_calls"], 1),
          num(c["counts"]["totals"]["krylov_iterations"] / max(1, c["counts"]["totals"]["stage_solves"]), 1),
          short_label(c), c.get("admissibility_with_solve_intervals_extension", "-")] for c in mf]))

    parts.append("\n## T3. Phase time per attempt in microseconds (median / minimum)\n")
    rows = []
    for c in direct:
        p = c["phases_ns_per_attempt"]
        rows.append([c["cell"]] + [f"{us(p[k]['median'])} / {us(p[k]['min'])}"
                                   for k in ("jacobian", "factor", "solve", "rhs", "remainder")]
                    + ["-"])
    for c in mf:
        p = c["phases_ns_per_attempt"]
        rows.append([c["cell"], "-", "-",
                     f"{us(p['solve_intervals']['median'])} / {us(p['solve_intervals']['min'])}",
                     f"{us(p['rhs']['median'])} / {us(p['rhs']['min'])}",
                     f"{us(p['other']['median'])} / {us(p['other']['min'])}",
                     f"{us(p['jvp']['median'])} / {us(p['jvp']['min'])}"])
    parts.append(table(["cell", "Jacobian", "factor", "solves", "RHS", "rest", "JVP (inside solves)"], rows))

    parts.append("\n## T4. Calls per attempt and per accepted step\n")
    rows = []
    for c in direct + mf:
        a, s = c["counts"]["per_attempt"], c["counts"]["per_accepted_step"]
        keys = ["jacobian_builds", "factorizations", "stage_solves", "rhs_calls", "fd_rhs_calls"]
        rows.append([c["cell"]] + [f"{num(a.get(k), 2)} / {num(s.get(k) if s else None, 2)}" for k in keys]
                    + [f"{num(a.get('jvp_calls'), 1)} / {num(s.get('jvp_calls') if s else None, 1)}"
                       if "jvp_calls" in a else "-",
                       f"{num(a.get('krylov_iterations'), 1)} / {num(s.get('krylov_iterations') if s else None, 1)}"
                       if "krylov_iterations" in a else "-",
                       num(a.get("preconditioner_applications"), 1)
                       if "preconditioner_applications" in a else "-"])
    parts.append(table(["cell", "Jacobian builds", "factorisations", "stage solves", "RHS calls",
                        "RHS calls inside FD", "JVP calls", "Krylov iterations",
                        "preconditioner applications"], rows))

    counted = [c for c in cells if "instruction_counts" in c and "phase_fractions" in c["instruction_counts"]]
    parts.append("\n## T5. Instruction counts (callgrind, inclusive Ir of the marker symbols)\n")
    rows = []
    for c in counted:
        i = c["instruction_counts"]
        f = i["phase_fractions"]
        rows.append([c["cell"], c["n"], i["role"], i["attempts"], f"{i['region_ir_per_attempt']:.0f}",
                     pct(f.get("jacobian")), pct(f.get("factor")), pct(f.get("solve")),
                     pct(f.get("rhs")), pct(f.get("jvp")), pct(f["remainder"]),
                     num(i["marked_over_plain"], 4),
                     "yes" if i["marked_run_identical_to_plain_run"] else "NO"])
    parts.append(table(["cell", "n", "role", "attempts", "Ir/attempt", "J %", "LU %", "solve %",
                        "RHS %", "JVP %", "rest %", "marked/plain Ir", "identical"], rows))

    parts.append("\n## T6. Upper bounds of the whole-step speed-up per lever (fractions of T1, T2, T5)\n")
    rows = []
    for r in regimes:
        b = r["b_speculative_jacobian_lu"]
        e = r["e_parallel_dense_lu"]
        f = r["f_shared_krylov_basis"]
        rows.append([
            r["cell"], "Ir" if r["fraction_basis"] == "instructions" else "wall",
            {"authoritative": "AUTH", "diagnostic_only": "diag", "not_measured": "-"}[r["admissibility"]],
            pct(r["fractions"]["f_chain"]),
            " / ".join(num(r["a_level_structured_tableau"][f"L{L}"]) for L in LEVELS),
            num(b["r"], 3), num(b["bound"]), num(b["bound_jacobian_only"]),
            " / ".join(num(r["c_parallel_jacobian_assembly"][f"P{p}"]) for p in WORKERS),
            " / ".join(num(r["d_lagged_lu"][f"k{k}"]) for k in LAGS),
            " / ".join(num(e[f"P{p}"]) for p in WORKERS) if e else "-",
            num(f["bound"]) if f else "-",
            " / ".join(num(r["g_parallel_rhs"][f"P{p}"]) for p in WORKERS),
        ])
    parts.append(table(
        ["cell", "basis", "label", "chain %", "(a) L=4/5/6", "r=(J+LU)/chain", "(b) J+LU",
         "(b) J only", "(c) P=2/4/6", "(d) k=2/4", "(e) P=2/4/6", "(f)", "(g) P=2/4/6"], rows))

    parts.append("\n## T7. Regimes that open V07 stage b: r in [0.15, 0.92] and plain wall per accepted step >= 500 us\n")
    gate = [r for r in regimes if r["b_speculative_jacobian_lu"]["gates_V07_stage_b"]]
    parts.append(table(
        ["cell", "n", "r", "plain us/accepted step", "(b) bound", "(b) J only", "basis", "label"],
        [[r["cell"], r["n"], num(r["b_speculative_jacobian_lu"]["r"], 3),
          us(r["plain_wall_per_accepted_step_us"] * 1000.0),
          num(r["b_speculative_jacobian_lu"]["bound"]),
          num(r["b_speculative_jacobian_lu"]["bound_jacobian_only"]),
          r["fraction_basis"],
          r["admissibility"] + (" (reduced_protocol)" if r["reduced_protocol"] else "")]
         for r in gate]) if gate else "(none)")

    near = [r for r in regimes if not r["b_speculative_jacobian_lu"]["gates_V07_stage_b"]
            and r["b_speculative_jacobian_lu"]["r_in_0.15_0.92"]]
    parts.append("\n## T8. Regimes with r in [0.15, 0.92] whose accepted step is shorter than 500 us (not in the gate list)\n")
    parts.append(table(
        ["cell", "r", "r in band", "plain us/accepted step", ">= 500 us", "label"],
        [[r["cell"], num(r["b_speculative_jacobian_lu"]["r"], 3),
          "yes" if r["b_speculative_jacobian_lu"]["r_in_0.15_0.92"] else "no",
          us(r["plain_wall_per_accepted_step_us"] * 1000.0)
          if r["plain_wall_per_accepted_step_us"] is not None else "-",
          "yes" if r["b_speculative_jacobian_lu"]["plain_wall_per_accepted_step_ge_500us"] else "no",
          r["admissibility"]] for r in near]) if near else "(none)")

    skipped = [c for c in cells if "wall" not in c]
    parts.append("\n## T9. Cells without a wall measurement\n")
    parts.append(table(
        ["cell", "protocol", "reason", "pilot attempts", "pilot accepted", "pilot rejected",
         "pilot s/attempt"],
        [[c["cell"], c["protocol"], c["sizing"].get("reason", "; ".join(c.get("measurement_notes", []))),
          c["sizing"].get("pilot_attempts", "-"), c["sizing"].get("pilot_accepted", "-"),
          c["sizing"].get("pilot_rejected", "-"),
          num(c["sizing"].get("pilot_seconds_per_attempt"), 4)] for c in skipped]) if skipped else "(none)")
    _ = by_id
    return "\n".join(parts) + "\n"


def main():
    results = strict_load(HERE / "RESULTS.json")
    pre = strict_load(HERE / "PREDECLARED.json")
    regimes = [r for r in (regime(c) for c in results["cells"]) if r is not None]
    gate = [r for r in regimes if r["b_speculative_jacobian_lu"]["gates_V07_stage_b"]]

    def best(key, sub, only_authoritative):
        rows = []
        for r in regimes:
            if only_authoritative and r["admissibility"] != "authoritative":
                continue
            block = r[key]
            if block is None or block.get(sub) is None:
                continue
            rows.append((block[sub], r["cell"]))
        rows.sort(reverse=True)
        return [{"cell": cell, "bound": value} for value, cell in rows[:5]]

    levers = {
        "a_level_structured_tableau_L5": ("a_level_structured_tableau", "L5"),
        "b_speculative_jacobian_lu": ("b_speculative_jacobian_lu", "bound"),
        "b_speculative_jacobian_only": ("b_speculative_jacobian_lu", "bound_jacobian_only"),
        "c_parallel_jacobian_assembly_P6": ("c_parallel_jacobian_assembly", "P6"),
        "d_lagged_lu_k4": ("d_lagged_lu", "k4"),
        "e_parallel_dense_lu_P6": ("e_parallel_dense_lu", "P6"),
        "f_shared_krylov_basis": ("f_shared_krylov_basis", "bound"),
        "g_parallel_rhs_P6": ("g_parallel_rhs", "P6"),
    }
    doc = {
        "schema": "vigilode.loop_l1.v04_cost_anatomy.amdahl.v1",
        "node": "V04_STAGE_COST_ANATOMY",
        "generated_by": "build_amdahl.py from RESULTS.json",
        "formulas": pre["amdahl"],
        "reading": "every bound is an upper bound of the whole-step speed-up under zero overhead, perfect scaling and an unchanged step count; a bound computed from a diagnostic_only row is diagnostic",
        "v07_stage_b_gate": {
            "condition": "0.15 <= (f_J + f_LU) / f_chain <= 0.92 and plain wall per accepted step >= 500 microseconds",
            "regimes": [{
                "cell": r["cell"], "n": r["n"], "r": r["b_speculative_jacobian_lu"]["r"],
                "plain_wall_per_accepted_step_us": r["plain_wall_per_accepted_step_us"],
                "bound": r["b_speculative_jacobian_lu"]["bound"],
                "bound_jacobian_only": r["b_speculative_jacobian_lu"]["bound_jacobian_only"],
                "fraction_basis": r["fraction_basis"],
                "admissibility": r["admissibility"], "reduced_protocol": r["reduced_protocol"],
            } for r in gate],
            "regimes_on_authoritative_rows": [r["cell"] for r in gate
                                              if r["admissibility"] == "authoritative"],
        },
        "largest_bounds_per_lever": {
            name: {"authoritative_rows": best(key, sub, True), "all_rows": best(key, sub, False)}
            for name, (key, sub) in levers.items()},
        "regimes": regimes,
    }
    text = json.dumps(doc, indent=1, ensure_ascii=False, allow_nan=False) + "\n"
    (HERE / "AMDAHL.json").write_text(text)
    rendered = tables(results, regimes)
    (HERE / "REPORT_TABLES.md").write_text(rendered)
    # The report carries the same tables between two markers; the prose around them is
    # hand-written and is not touched.
    report = HERE / "REPORT_KO.md"
    begin, end = "<!-- BEGIN GENERATED TABLES -->", "<!-- END GENERATED TABLES -->"
    if report.exists():
        body = report.read_text()
        if begin in body and end in body:
            head, rest = body.split(begin, 1)
            _, tail = rest.split(end, 1)
            report.write_text(head + begin + "\n" + rendered + end + tail)
            print("updated the generated tables of REPORT_KO.md")
    print("wrote AMDAHL.json and REPORT_TABLES.md; V07 stage b gate list:",
          [r["cell"] for r in gate])


if __name__ == "__main__":
    sys.exit(main())
