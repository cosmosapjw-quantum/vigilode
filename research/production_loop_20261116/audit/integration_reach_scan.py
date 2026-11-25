#!/usr/bin/env python3
"""Static reach scan: which research components does the facade crate reference?

Written for the integration-status record INTEGRATION_STATUS_20261124_KO.md.
Run from the repository root:

    python3 -I research/production_loop_20261116/audit/integration_reach_scan.py

Part 1 counts, for distinctive identifiers of each research component, the
non-comment lines of crates/vigilode/src/**/*.rs that contain them. A count of
0 means the facade source never names the component.

Part 2 is a coarse module-level scan of crates/rodas5p-integrators: for each
module declared in its lib.rs it lists the public item names of the module
that also occur in the facade source. Generic names (validate, dimension,
as_str, ...) make this part noisy: a hit is a lead, not a proof of use; a
module with no hit is not named by the facade.

Static only: no build, no dynamic trace. Re-exports through glob `pub use`
are covered because the scan matches item names, not paths.
"""

import glob
import os
import re
import sys

ROOT = os.getcwd()
FACADE = os.path.join(ROOT, "crates/vigilode/src")
INTEGRATORS = os.path.join(ROOT, "crates/rodas5p-integrators/src")

IDENTIFIERS = [
    # stage clients and policies of the loop (V17, V18, V21, V08, V24)
    "stage_clients", "BandedTightStageLu", "AssemblyStageClient",
    "DenseLuReuseStageClient", "MfStageSolvePolicy",
    # homotopy / Newton-free root finding lines
    "homotopy", "Homotopy", "fixed_point_continuation", "nonlinear_homotopy",
    "actual_k_homotopy",
    # causality / transactional lines
    "causal_majorant", "upper_causal_solve", "transactional", "Transactional",
    "transaction::",
    # other integrator research lanes
    "packet", "sabr", "Sabr", "small_batch", "SmallBatch", "ptsw2b", "Ptsw2b",
    "source_owned_graph_schur", "stage_batch", "collective",
    # parallel execution
    "ParallelExecution", "rayon",
    # switching inputs
    "dopri5_stability_limited", "HybridRouteAutomaton", "nonnormal",
    "SecantPreconditioner", "Leja",
    # coupled crate
    "vigilode_coupled",
]


def facade_lines():
    out = []
    for path in sorted(glob.glob(os.path.join(FACADE, "**/*.rs"), recursive=True)):
        rel = os.path.relpath(path, ROOT)
        with open(path) as handle:
            for number, line in enumerate(handle, 1):
                if line.strip().startswith("//"):
                    continue
                out.append((rel, number, line))
    return out


def part1(lines):
    print("## Part 1: distinctive identifiers in non-comment facade lines")
    for ident in IDENTIFIERS:
        hits = [(rel, number) for rel, number, line in lines if ident in line]
        files = sorted({rel for rel, _ in hits})
        print(f"{ident:28s} {len(hits):4d}  {' '.join(files)}")


def module_files(module):
    files = []
    single = os.path.join(INTEGRATORS, module + ".rs")
    if os.path.exists(single):
        files.append(single)
    directory = os.path.join(INTEGRATORS, module)
    if os.path.isdir(directory):
        files += sorted(glob.glob(os.path.join(directory, "**/*.rs"), recursive=True))
    return files


def part2(lines):
    print()
    print("## Part 2: integrator modules and facade name hits (noisy, see docstring)")
    with open(os.path.join(INTEGRATORS, "lib.rs")) as handle:
        lib = handle.read()
    modules = re.findall(r"^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+([a-z0-9_]+)\s*;", lib, re.M)
    code = "\n".join(line for _, _, line in lines)
    rows = []
    for module in modules:
        files = module_files(module)
        source = "".join(open(path).read() for path in files)
        names = set(re.findall(
            r"pub\s+(?:const\s+)?(?:fn|struct|enum|trait|type|const|static)\s+([A-Za-z_][A-Za-z0-9_]*)",
            source))
        names = {name for name in names if len(name) > 4}
        hits = sorted(name for name in names if re.search(r"\b" + re.escape(name) + r"\b", code))
        path_hit = bool(re.search(r"\b" + module + r"::", code))
        rows.append((len(hits), module, source.count("\n"), path_hit, hits))
    rows.sort(key=lambda row: (row[0], row[1]))
    for count, module, size, path_hit, hits in rows:
        flag = "Y" if path_hit else "-"
        print(f"{module:40s} lines={size:6d} path={flag} hits={count:3d} {','.join(hits[:6])}")
    print(f"\n{len(modules)} modules declared in rodas5p-integrators/src/lib.rs")


def main():
    if not os.path.isdir(FACADE):
        sys.exit("run from the repository root")
    lines = facade_lines()
    part1(lines)
    part2(lines)


if __name__ == "__main__":
    main()
