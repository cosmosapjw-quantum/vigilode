#!/usr/bin/env bash
# User-authorized online install (2026-10-22) of the pinned NCP C1 Julia environment into a fresh,
# private, persistent depot. Single JULIA_DEPOT_PATH entry; cores 0-7 only; low priority.
set -uo pipefail
B=/root/vigilode-julia-c1-20261022
JUL=/root/VIGILODE_COMPONENT_20261018T134235Z/env/julia-1.12.7+0.x64.linux.gnu/bin/julia
export JULIA_DEPOT_PATH=$B/depot
export JULIA_NUM_PRECOMPILE_TASKS=6
export JULIA_PKG_PRECOMPILE_AUTO=0
cd $B
date -u +%FT%TZ > $B/INSTALL_START_UTC.txt
sha256sum env/Project.toml env/Manifest.toml > $B/ENV_SHA256SUMS.txt
nice -n 10 taskset -c 0-7 "$JUL" --startup-file=no --project=$B/env -e '
using Pkg
Pkg.instantiate()
Pkg.precompile()
using OrdinaryDiffEq, OrdinaryDiffEqBDF, SciMLBase, Sundials, SparseArrays
println("JULIA_INSTALL_OK ", VERSION)
Pkg.status()
' > $B/install.log 2>&1
echo "exit=$?" >> $B/install.log
date -u +%FT%TZ > $B/INSTALL_END_UTC.txt
touch $B/INSTALL_DONE
