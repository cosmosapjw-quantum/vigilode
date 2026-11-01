#!/usr/bin/env bash
set -euo pipefail
export OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1
python prepare_sources.py
python make_reference.py
python run_science.py
python -m unittest discover -s tests -v
python -O -m unittest discover -s tests -v
python ci_reference.py
# Native RODAS5P (from VigilODE repository root):
# cargo run --locked --release -p rodas5p-integrators --no-default-features --example rei_igm_five_clock_source > science/NATIVE_TIMEFORCED_PHYSICAL.json
# cd exploratory/rei_igm_timeforced_photoionization_20261101
# python verify_native.py ../../science/NATIVE_TIMEFORCED_PHYSICAL.json
