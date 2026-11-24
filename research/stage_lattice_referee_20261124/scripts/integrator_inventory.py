#!/usr/bin/env python3
"""Machine-readable inventory of the integrator-side modules and their reachability from the
`vigilode` facade (node SL-C1, input of the package consolidation plan).

For every module of rodas5p-integrators, rodas5p-krylov, rodas5p-core, vigilode-coupled and
vigilode-hybrid the script records: lines, public item count, the identifiers the facade
names through an explicit `<crate>::` path, and a status:

  default          reached by the facade's default routes (protected drivers)
  facade-opt-in    reachable from the facade only through an explicit opt-in entry point
  integrator-only  library code that the facade does not name
  research-gate    research harness code (gates, receipts, corpora, audits) by file name
  negative         recorded negative verdict; kept for evidence, never routed

The statuses of the named modules come from STATUS below (sources: research/LEDGER.jsonl
rows cited there, this run's tests); every other module is classified by reachability and
name. Usage: integrator_inventory.py <repo root> <output json>
"""
from __future__ import annotations

import collections
import json
import pathlib
import re
import sys

CRATES = {
    "rodas5p_integrators": "rodas5p-integrators",
    "rodas5p_krylov": "rodas5p-krylov",
    "rodas5p_core": "rodas5p-core",
    "vigilode_coupled": "vigilode-coupled",
    "vigilode_hybrid": "vigilode-hybrid",
}
ITEM = re.compile(
    r"^\s*pub\s+(?:async\s+)?(?:unsafe\s+)?(fn|struct|enum|trait|const|static|type)\s+([A-Za-z_]\w*)",
    re.M,
)
RESEARCH = re.compile(
    r"^(g[1-4]_|audit2_|a1_two_arm|unified_gates|unified_screen|native_gates|scientific_|v38d_|"
    r"comparator_fidelity|homotopy_experiments|candidates|policy_redesign_v25|common_w_gate|"
    r"banded_exact_tests|n01_|rhs_telemetry|controller_telemetry|problems)"
)
# (crate, module) -> (status, evidence)
STATUS = {
    ("rodas5p_integrators", "rodas5p_fast.rs"): ("default", "protected dense/banded fast drivers"),
    ("rodas5p_integrators", "rodas5p_fast_dense.rs"): ("default", "dense-output fast drivers"),
    ("rodas5p_integrators", "rodas5p_matrix_free_fast.rs"): ("default", "JVP route; P1/P2 opt-in (L-0318, L-0335)"),
    ("rodas5p_integrators", "sequential.rs"): ("default", "mass matrix / Enforce route"),
    ("rodas5p_integrators", "routing.rs"): ("default", "router of Rodas5p::solve"),
    ("rodas5p_integrators", "stage_clients/banded_tight.rs"): ("facade-opt-in", "BandedStagePolicy::Tight (this run); V21 same bits"),
    ("rodas5p_integrators", "stage_clients/banded_two_way.rs"): ("facade-opt-in", "BandedStagePolicy::TwoWay (this run)"),
    ("rodas5p_integrators", "stage_clients/krylov.rs"): ("integrator-only", "per-stage GMRES client PASS L-0318"),
    ("rodas5p_integrators", "stage_clients/assembly.rs"): ("integrator-only", "parallel FD assembly; strict rule FAIL L-0331"),
    ("rodas5p_integrators", "stage_clients/dense_lu_reuse.rs"): ("integrator-only", "same bits; wall NOT_DECIDABLE"),
    ("rodas5p_integrators", "stage_clients/speculative.rs"): ("negative", "V07 L-0316"),
    ("rodas5p_integrators", "bdf.rs"): ("facade-opt-in", "registry; RHS-only FD + Newton"),
    ("rodas5p_integrators", "radau.rs"): ("facade-opt-in", "registry; RHS-only FD + Newton"),
    ("rodas5p_integrators", "dopri5.rs"): ("facade-opt-in", "Dopri5, SwitchingSolver, Dopri5Ensemble"),
    ("rodas5p_integrators", "dopri5_stiffness.rs"): ("facade-opt-in", "switching secant estimate"),
    ("rodas5p_integrators", "exponential.rs"): ("facade-opt-in", "registry PEXPRB54s4"),
    ("rodas5p_integrators", "adaptive_exponential.rs"): ("facade-opt-in", "registry PEXPRB54s4"),
    ("rodas5p_integrators", "rodas5p_lagged.rs"): ("facade-opt-in", "registry; lagged LU killed on HIRES"),
    ("rodas5p_integrators", "ppsw1.rs"): ("facade-opt-in", "PeerOde; G0 holdout HOLD"),
    ("rodas5p_integrators", "quotient_flow.rs"): ("facade-opt-in", "registry Lawson"),
    ("rodas5p_integrators", "affine_event_certificate.rs"): ("facade-opt-in", "Solution::certified_affine_event_windows"),
    ("rodas5p_integrators", "source_owned_graph_schur.rs"): ("facade-opt-in", "structured feature; C5 2.24x slower (negative perf)"),
    ("rodas5p_integrators", "transactional_q1_q2.rs"): ("negative", "R7 4.75x slower"),
    ("rodas5p_integrators", "causal_majorant.rs"): ("integrator-only", "causal majorant; no solve path"),
    ("rodas5p_integrators", "homotopy.rs"): ("integrator-only", "R5 continuation 2.66-6.94x slower than LU"),
    ("rodas5p_integrators", "nonlinear_homotopy.rs"): ("integrator-only", "homotopy family"),
    ("rodas5p_integrators", "fixed_point_continuation.rs"): ("integrator-only", "PORTED_BLOCKED (P14)"),
    ("rodas5p_integrators", "actual_k_homotopy.rs"): ("integrator-only", "actual-k comparison"),
    ("rodas5p_integrators", "ptsw2b.rs"): ("integrator-only", "PTSW2B"),
    ("rodas5p_integrators", "sabr.rs"): ("integrator-only", "SABR"),
    ("rodas5p_integrators", "rodas5p_fast_small_batch.rs"): ("negative", "lane batch 0.93"),
    ("rodas5p_integrators", "conditioned_polynomial.rs"): ("integrator-only", "Chebyshev/Leja actions"),
}


def main() -> int:
    root = pathlib.Path(sys.argv[1])
    facade = root / "crates/vigilode/src"
    code = "\n".join(
        "\n".join(l for l in p.read_text().split("\n") if not l.lstrip().startswith("//"))
        for p in facade.rglob("*.rs")
    )
    named = collections.defaultdict(set)
    segments = collections.defaultdict(set)
    for ck in CRATES:
        for m in re.finditer(ck + r"::((?:\w+::)*)(\{[^;]*?\}|\w+)", code, re.S):
            for s in filter(None, m.group(1).split("::")):
                segments[ck].add(s)
            for ident in re.findall(r"[A-Za-z_]\w*", m.group(2)):
                named[ck].add(ident)
    modules = []
    for ck, cdir in CRATES.items():
        src = root / "crates" / cdir / "src"
        for p in sorted(src.rglob("*.rs")):
            text = p.read_text()
            body = text.split("#[cfg(test)]")[0]
            items = sorted({m.group(2) for m in ITEM.finditer(body)})
            mod = str(p.relative_to(src))
            stem = [s for s in mod[:-3].split("/") if s not in ("mod", "lib")]
            hits = sorted(i for i in items if i in named[ck] and i not in ("new",))
            path_named = any(s in segments[ck] or s in named[ck] for s in stem)
            reachable = bool(hits) or path_named
            if (ck, mod) in STATUS:
                status, evidence = STATUS[(ck, mod)]
            elif RESEARCH.match(pathlib.Path(mod).name):
                status, evidence = "research-gate", "classified by file name"
            elif reachable:
                status, evidence = "facade-opt-in", "named by the facade source"
            else:
                status, evidence = "integrator-only", "not named by the facade source"
            modules.append({
                "crate": CRATES[ck], "module": mod, "lines": text.count("\n"),
                "public_items": len(items), "facade_named_items": hits[:12],
                "status": status, "evidence": evidence,
            })
    summary = collections.Counter((m["crate"], m["status"]) for m in modules)
    lines = collections.Counter()
    for m in modules:
        lines[(m["crate"], m["status"])] += m["lines"]
    out = {
        "schema": "vigilode.referee2.integrator-inventory.v1",
        "statuses": ["default", "facade-opt-in", "integrator-only", "research-gate", "negative"],
        "summary": [
            {"crate": c, "status": s, "modules": n, "lines": lines[(c, s)]}
            for (c, s), n in sorted(summary.items())
        ],
        "modules": modules,
    }
    pathlib.Path(sys.argv[2]).write_text(json.dumps(out, indent=1) + "\n")
    for row in out["summary"]:
        print(f"{row['crate']:22s} {row['status']:16s} modules={row['modules']:3d} lines={row['lines']:6d}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
