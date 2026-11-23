#!/usr/bin/env python3
"""One cell of the width scan of node V22 (PREDECLARED.json): whole runs of the protected banded
driver with the Indexed and the Slices kernel on one problem (family, size).

usage: measure.py <runner binary> <family> <size> <attempts> <index> [--kernels a,b] [--tag s]

Writes logs/drive_<tag><index>_<family><size>.jsonl next to this script. The runs, the rounds
(2 warm-ups + 7), the pinned core, the timing lock and the records are those of
../banded_solve/measure_drive.py; this script only sets where the log goes.
Tags: s, the series that decides; c, the small-n confirmation; x, exploratory (decides nothing).
"""
import importlib.util
import os

HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location(
    "measure_drive", os.path.join(HERE, "..", "banded_solve", "measure_drive.py"))
measure_drive = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(measure_drive)
measure_drive.HERE = HERE

if __name__ == "__main__":
    measure_drive.main()
