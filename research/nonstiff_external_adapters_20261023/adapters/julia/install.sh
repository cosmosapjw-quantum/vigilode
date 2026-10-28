#!/bin/sh
# Explicit environment preparation; never invoked by the benchmark launcher.
set -eu
adapter_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
: "${VIGILODE_JULIA_DEPOT:?Set a dedicated writable Julia depot path}"
export JULIA_DEPOT_PATH="$VIGILODE_JULIA_DEPOT"
export JULIA_PKG_PRECOMPILE_AUTO=0
export JULIA_NUM_PRECOMPILE_TASKS="${JULIA_NUM_PRECOMPILE_TASKS:-4}"
exec "${VIGILODE_JULIA_EXECUTABLE:-/home/cosmosapjw/.julia/juliaup/julia-1.12.7+0.x64.linux.gnu/bin/julia}" --startup-file=no --history-file=no --project="$adapter_dir" -e 'using Pkg; VERSION == v"1.12.7" || error("Julia 1.12.7 required"); Pkg.instantiate(); Pkg.precompile()'
