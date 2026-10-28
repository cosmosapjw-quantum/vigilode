#!/bin/sh
set -eu
adapter_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
export JULIA_DEPOT_PATH="${VIGILODE_JULIA_DEPOT:-/home/cosmosapjw/vigilode-nonstiff-adapters-run-tTYnLc/julia-development/julia-depot}"
export JULIA_NUM_THREADS=1
export OPENBLAS_NUM_THREADS=1
export VIGILODE_ADAPTER_SOURCE_COMMIT="${VIGILODE_ADAPTER_SOURCE_COMMIT:-UNCOMMITTED_DEVELOPMENT}"
exec "${VIGILODE_JULIA_EXECUTABLE:-/home/cosmosapjw/.julia/juliaup/julia-1.12.7+0.x64.linux.gnu/bin/julia}" --startup-file=no --history-file=no --threads=1 --project="$adapter_dir" "$adapter_dir/adapter.jl"
