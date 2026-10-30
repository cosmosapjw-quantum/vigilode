#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
python code/verify_hashes.py static
python code/restore_inputs.py
python -m unittest discover -s tests -p 'test_full_history.py' -v
python code/full_history_replay.py >/dev/null
python code/clock_quadrature.py >/dev/null
python code/plot_science.py >/dev/null
python code/verify_hashes.py results
printf 'VIGILODE_BASS21_FULL25_PASS tests=20/20 original=50/50 histories=9650 paired_rows=4825 new_ivps=0\n'
