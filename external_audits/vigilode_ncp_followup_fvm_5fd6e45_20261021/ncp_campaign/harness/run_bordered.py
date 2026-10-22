"""Manufactured bordered band probe (NOT an ODE benchmark): residual/storage first, timing diagnostic.

Usage: taskset -c CORE python3 run_bordered.py CAMP CORE WARMUPS RUNS
Rows -> results/BORDERED.jsonl
"""
import json
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from common import append_jsonl, child_env, run_child  # noqa: E402

CAMP = Path(sys.argv[1])
CORE, WARM, RUNS = int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4])
BIN = CAMP / 'bin/bordered_probe'
raw = CAMP / 'raw/C_bordered'
os.makedirs(raw, exist_ok=True)
L = U = 1
M = 2
for label, extra in (('default', []), ('n1600', ['--n', '1600']), ('n4096', ['--n', '4096'])):
    for k in range(WARM + RUNS):
        run_id = f'C-{label}-c{CORE:02d}-r{k:02d}'
        info = run_child([str(BIN)] + extra, raw / f'{run_id}.json', raw / f'{run_id}.stderr.log',
                         env=child_env(), timeout=120)
        probes = []
        for line in Path(info['stdout_path']).read_text().splitlines():
            if line.strip():
                p = json.loads(line)
                n = p['n']
                expected_slots = n * (L + U + 1) + 2 * n * M + M * M + n
                p['expected_f64_slots_formula'] = expected_slots
                p['storage_matches_formula'] = p['allocated_f64_slots'] == expected_slots
                p['residual_ok_1e-11'] = p['residual_inf'] <= 1e-11
                p['error_ok_1e-11'] = p['max_solution_error'] <= 1e-11
                probes.append(p)
        row = dict(info, phase='C', label=label, core=CORE, warmup=k < WARM, run_id=run_id, probes=probes,
                   success=(info['exit_code'] == 0 and len(probes) > 0 and all(
                       p['storage_matches_formula'] and p['residual_ok_1e-11'] and p['error_ok_1e-11']
                       for p in probes)),
                   scope='manufactured bordered band linear solve; not a BASS ODE benchmark')
        append_jsonl(CAMP / 'results/BORDERED.jsonl', row)
print(json.dumps({'phase': 'C', 'status': 'WORKER_COMPLETED'}))
