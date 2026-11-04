#!/usr/bin/env python3
"""
Unified Cross-Solver Benchmark Report Generator
Integrates results across VigilODE, DVODE, Diffsol, Julia SciML, and Hairer References.
Enforces strict provenance, accurate full-run allocation reporting, and fail-closed validation.
"""

import json
import sys
import os
import math
import hashlib

PROBLEMS = ["robertson", "hires", "van-der-pol-mu1000", "brusselator-1d-n100"]
RTOLS = [1e-4, 1e-6, 1e-8]

ARMS_DENSE = [
    ("VigilODE (Ours)", "vigilode", "rust"),
    ("Julia Rodas5P", "julia-rodas5p", "julia"),
    ("Netlib DVODE (MF=21)", "dvode-mf21", "rust"),
    ("Diffsol BDF (Dense)", "diffsol-bdf-dense", "rust"),
    ("Julia FBDF", "julia-fbdf", "julia"),
    ("SUNDIALS CVODE", "cvode-bdf-dense", "rust"),
    ("Julia CVODE_BDF", "julia-cvode-bdf", "julia"),
    ("Hairer RODAS", "rodas", "rust"),
    ("Hairer RADAU5", "radau5", "rust"),
]

ARMS_BANDED = [
    ("VigilODE (Banded)", "vigilode-banded", "rust"),
    ("VigilODE (Dense)", "vigilode", "rust"),
    ("Netlib DVODE (MF=24 Banded)", "dvode-mf24", "rust"),
    ("Diffsol BDF (Sparse)", "diffsol-bdf-sparse", "rust"),
    ("Diffsol BDF (Dense)", "diffsol-bdf-dense", "rust"),
    ("SUNDIALS CVODE (Band)", "cvode-bdf-band", "rust"),
    ("Julia FBDF", "julia-fbdf", "julia"),
    ("Julia Rodas5P", "julia-rodas5p", "julia"),
    ("Julia CVODE_BDF", "julia-cvode-bdf", "julia"),
    ("Hairer RODAS-Band", "rodas-band", "rust"),
    ("Hairer RADAU5-Band", "radau5-band", "rust"),
]

def sha256_file(filepath: str) -> str:
    h = hashlib.sha256()
    with open(filepath, "rb") as f:
        while chunk := f.read(65536):
            h.update(chunk)
    return h.hexdigest()

def format_num(val, fmt="{:.3e}") -> str:
    if val is None:
        return "—"
    if isinstance(val, (int, float)):
        return fmt.format(val)
    return str(val)

def format_rtol_heading(rtol: float) -> str:
    """BENCH-002: Parse numerical exponent reliably without relying on string formatting."""
    exp = -round(math.log10(rtol))
    return f"$rtol = 10^{{-{exp}}}$ (`rtol={rtol}`)"

def format_bytes(n_bytes: int) -> str:
    if n_bytes >= 1024 * 1024:
        return f"{n_bytes / (1024 * 1024):.1f} MB"
    elif n_bytes >= 1024:
        return f"{n_bytes / 1024:.1f} KB"
    else:
        return f"{n_bytes} B"

def format_allocations(alloc_data) -> str:
    """BENCH-001: Report exact measured heap_calls and heap_bytes over the full-run window."""
    if not alloc_data:
        return "—"
    calls = alloc_data.get("heap_calls", 0)
    n_bytes = alloc_data.get("heap_bytes", 0)
    if calls == 0:
        return "0 calls (0 B)"
    return f"{calls} calls ({format_bytes(n_bytes)})"

def check_host_compatibility(rust_prov: dict, jl_prov: dict) -> dict:
    """BENCH-003: Check if both datasets were measured on the same execution host."""
    rust_cpu = rust_prov.get("host_cpu", "unknown")
    jl_cpu = jl_prov.get("host_cpu", "unknown")
    rust_cpus = rust_prov.get("host_logical_cpus", 0)
    jl_cpus = jl_prov.get("host_logical_cpus", 0)

    same_host = (rust_cpu == jl_cpu) and (rust_cpus == jl_cpus) and (rust_cpu != "unknown")
    if same_host:
        verdict = "diagnostic_only (verified same physical host)"
    else:
        verdict = f"diagnostic_only (different_hosts: Rust on [{rust_cpu}, {rust_cpus} CPUs], Julia on [{jl_cpu}, {jl_cpus} CPUs])"

    return {
        "same_host": same_host,
        "rust_host": f"{rust_cpu} ({rust_cpus} CPUs)",
        "jl_host": f"{jl_cpu} ({jl_cpus} CPUs)",
        "timing_authority_verdict": verdict,
    }

def load_and_validate_data(wp_path: str, julia_path: str, enforce_completeness: bool = True):
    """BENCH-004: Fail-closed loader asserting key uniqueness and grid completeness."""
    with open(wp_path, "r") as f:
        wp_data = json.load(f)
    with open(julia_path, "r") as f:
        julia_raw = json.load(f)

    # Handle both v2 manifest dictionary or v1 flat list for Julia results
    if isinstance(julia_raw, dict) and "results" in julia_raw:
        jl_rows_raw = julia_raw["results"]
        jl_prov = julia_raw.get("provenance", {})
    else:
        jl_rows_raw = julia_raw
        jl_prov = {}

    rust_rows = {}
    for r in wp_data.get("rows", []):
        key = (r["problem"], r["arm"], float(r["rtol"]))
        if key in rust_rows:
            raise ValueError(f"Duplicate entry in Rust dataset for key: {key}")
        rust_rows[key] = r

    jl_rows = {}
    for r in jl_rows_raw:
        key = (r["problem"], r["algorithm"], float(r["rtol"]))
        if key in jl_rows:
            raise ValueError(f"Duplicate entry in Julia dataset for key: {key}")
        jl_rows[key] = r

    if enforce_completeness:
        for prob in PROBLEMS:
            target_arms = ARMS_BANDED if prob == "brusselator-1d-n100" else ARMS_DENSE
            for rtol in RTOLS:
                for label, arm_id, source in target_arms:
                    k = (prob, arm_id, rtol)
                    if source == "rust" and k not in rust_rows:
                        raise ValueError(f"Missing expected cell in Rust data: {k}")
                    elif source == "julia" and k not in jl_rows:
                        raise ValueError(f"Missing expected cell in Julia data: {k}")

    rust_prov = wp_data.get("provenance", {})
    host_comp = check_host_compatibility(rust_prov, jl_prov)

    return {
        "wp_data": wp_data,
        "rust_rows": rust_rows,
        "rust_prov": rust_prov,
        "jl_rows": jl_rows,
        "jl_prov": jl_prov,
        "host_comp": host_comp,
        "wp_sha256": sha256_file(wp_path),
        "jl_sha256": sha256_file(julia_path),
    }

def generate_report(wp_path: str, julia_path: str, out_markdown_path: str, enforce_completeness: bool = True):
    validated = load_and_validate_data(wp_path, julia_path, enforce_completeness=enforce_completeness)
    rust_rows = validated["rust_rows"]
    jl_rows = validated["jl_rows"]
    host_comp = validated["host_comp"]

    lines = []
    lines.append("# VigilODE Comprehensive Cross-Solver Benchmark Report")
    lines.append("")
    lines.append("## Provenance and Execution Manifest")
    lines.append("")
    lines.append(f"- **Timing Authority**: `{host_comp['timing_authority_verdict']}`")
    lines.append(f"- **Rust Dataset**: `{os.path.basename(wp_path)}` (SHA-256: `{validated['wp_sha256']}`)")
    lines.append(f"- **Rust Execution Host**: `{host_comp['rust_host']}`")
    lines.append(f"- **Julia Dataset**: `{os.path.basename(julia_path)}` (SHA-256: `{validated['jl_sha256']}`)")
    lines.append(f"- **Julia Execution Host**: `{host_comp['jl_host']}`")
    lines.append(f"- **Same-Host Speed Authority**: `{'PASS' if host_comp['same_host'] else 'HOLD (cross-host speed comparison inadmissible)'}`")
    lines.append("- **Error Metric**: Max WRMS error on common 101-point uniform grid against reference solution ($rtol=10^{-12}$).")
    lines.append("- **Allocation Scope**: Full-run window (`setup + integration + grid states + teardown`), measured via C `malloc`/`calloc` interposition hook.")
    lines.append("")

    for prob in PROBLEMS:
        lines.append(f"## Problem: `{prob}`")
        lines.append("")
        target_arms = ARMS_BANDED if prob == "brusselator-1d-n100" else ARMS_DENSE

        for rtol in RTOLS:
            heading = format_rtol_heading(rtol)
            lines.append(f"### Tolerance: {heading}")
            lines.append("")
            lines.append("| Solver Arm | Status | Error (WRMS) | Wall Time (ms) | RHS f-evals | Jac evals | Steps (acc/rej) | Full-Run Allocations |")
            lines.append("| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |")

            for label, arm_id, source in target_arms:
                k = (prob, arm_id, rtol)
                if source == "rust":
                    entry = rust_rows[k]
                    status_kind = entry.get("status", {}).get("kind", "unknown")
                    status = "✅ Success" if status_kind == "success" else f"❌ {entry.get('status', {}).get('message', 'Failed')}"
                    err = format_num(entry.get("error_common"))
                    t_ms = f"{entry.get('wall_ms_median', 0.0):.3f}" if entry.get("wall_ms_median") is not None else "—"
                    rhs = str(entry.get("callbacks", {}).get("rhs", "—"))
                    jac = str(entry.get("callbacks", {}).get("jac", "—"))
                    w = entry.get("work", {})
                    acc = w.get("accepted_steps", "—")
                    rej = w.get("rejected_steps", "—")
                    steps = f"{acc}/{rej}" if acc != "—" else "—"
                    alloc_str = format_allocations(entry.get("allocations"))
                else: # julia
                    entry = jl_rows[k]
                    status = "✅ Success" if entry.get("status") == "Success" else f"❌ {entry.get('status')}"
                    err = format_num(entry.get("error_common"))
                    t_ms = f"{entry.get('time_median_ms', 0.0):.3f}" if entry.get("time_median_ms") is not None else "—"
                    rhs = str(entry.get("nf", "—"))
                    jac = str(entry.get("njacs", "—"))
                    acc = entry.get("naccept", "—")
                    rej = entry.get("nreject", "—")
                    steps = f"{acc}/{rej}" if acc != "—" else "—"
                    alloc_str = "GC managed"

                lines.append(f"| **{label}** | {status} | {err} | {t_ms} | {rhs} | {jac} | {steps} | {alloc_str} |")

            lines.append("")

    content = "\n".join(lines)
    os.makedirs(os.path.dirname(out_markdown_path), exist_ok=True)
    with open(out_markdown_path, "w") as f:
        f.write(content)
    print(f"Generated unified report: {out_markdown_path}")

if __name__ == "__main__":
    wp_path = sys.argv[1] if len(sys.argv) > 1 else "research/bench_wave2_20261021/results_v1.json"
    jl_path = sys.argv[2] if len(sys.argv) > 2 else "bench/out/julia_bench_results.json"
    out_path = sys.argv[3] if len(sys.argv) > 3 else "bench/out/UNIFIED_BENCHMARK_REPORT.md"
    generate_report(wp_path, jl_path, out_path, enforce_completeness=True)
