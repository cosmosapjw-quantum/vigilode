"""Paired VigilODE heat blocks on one pinned core: BASE (basis-JVP) vs STEN (analytic stencil).

Usage: taskset -c CORE python3 run_pairs.py CAMP PHASE CORE WARMUP_BLOCKS CONFIRM_BLOCKS
Block k order: AB if k even else BA.  Rows -> results/PAIRS_<PHASE>_c<CORE>.jsonl
"""
import json
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from common import append_jsonl, child_env, run_child, vigilode_row  # noqa: E402

CAMP = Path(sys.argv[1])
PHASE, CORE = sys.argv[2], int(sys.argv[3])
WARM, CONF = int(sys.argv[4]), int(sys.argv[5])
BIN = CAMP / 'bin/component_benchmark_native'
REQ = {'BASE': CAMP / 'requests/HEAT_BASELINE_REQUEST.json',
       'STEN': CAMP / 'requests/HEAT_STENCIL_REQUEST.json'}
budget = json.loads((CAMP / 'results/APPLICATION_BUDGET.json').read_text())
refdoc = json.loads((CAMP / 'results/REFERENCE.json').read_text())
ref, output_times = refdoc['states'], refdoc['output_times']
raw = CAMP / 'raw' / PHASE
os.makedirs(raw, exist_ok=True)
out = CAMP / 'results' / f'PAIRS_{PHASE}_c{CORE:02d}.jsonl'
affinity = sorted(os.sched_getaffinity(0))
env = child_env()
for k in range(WARM + CONF):
    block_id = f'{PHASE}-c{CORE:02d}-b{k:02d}'
    order = ('BASE', 'STEN') if k % 2 == 0 else ('STEN', 'BASE')
    for pos, arm in enumerate(order):
        run_id = f'{block_id}-{arm}'
        argv = [str(BIN), '--request', str(REQ[arm]), '--measurement-mode', 'timing']
        info = run_child(argv, raw / f'{run_id}.json', raw / f'{run_id}.stderr.log', env=env, timeout=120)
        row = vigilode_row(info, budget, ref, output_times)
        row.update(phase=PHASE, core=CORE, affinity=affinity, block_id=block_id, block_index=k,
                   warmup=k < WARM, arm=arm, order=''.join(a[0] for a in order), position=pos,
                   run_id=run_id)
        append_jsonl(out, row)
print(json.dumps({'phase': PHASE, 'core': CORE, 'blocks': WARM + CONF, 'status': 'WORKER_COMPLETED'}))
