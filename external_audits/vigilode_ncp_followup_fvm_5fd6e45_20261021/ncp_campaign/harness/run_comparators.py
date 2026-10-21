"""Paired comparator blocks on one pinned core: VigilODE STEN process vs comparator process X.

Usage: taskset -c CORE python3 run_comparators.py CAMP ARM CORE WARMUP_BLOCKS CONFIRM_BLOCKS
ARM in VODE_MF20 VODE_MF21 VODE_MF22 JULIA_RODAS5P DIFFSOL_BDF.
Block k order: STEN first if k even else X first. JULIA gets one unpaired preflight first.
Rows -> results/COMP_<ARM>.jsonl
"""
import json
import os
import statistics
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from common import append_jsonl, child_env, evaluate, run_child, vigilode_row  # noqa: E402

CAMP = Path(sys.argv[1])
ARM, CORE = sys.argv[2], int(sys.argv[3])
WARM, CONF = int(sys.argv[4]), int(sys.argv[5])
JUL = '/root/VIGILODE_COMPONENT_20261018T134235Z/env/julia-1.12.7+0.x64.linux.gnu/bin/julia'
OLD_DEPOT = '/dev/shm/vigilode_ncp_intensive_120m_v1_20261021T062109Z/julia-depot'
BIN = CAMP / 'bin/component_benchmark_native'
STEN_REQ = CAMP / 'requests/HEAT_STENCIL_REQUEST.json'
BASE_REQ = CAMP / 'requests/HEAT_BASELINE_REQUEST.json'
budget = json.loads((CAMP / 'results/APPLICATION_BUDGET.json').read_text())
refdoc = json.loads((CAMP / 'results/REFERENCE.json').read_text())
ref, output_times = refdoc['states'], refdoc['output_times']
raw = CAMP / 'raw' / f'B_{ARM}'
os.makedirs(raw, exist_ok=True)
out = CAMP / 'results' / f'COMP_{ARM}.jsonl'
env = child_env()
xenv = dict(env)
if ARM.startswith('VODE_'):
    xargv = ['python3', str(CAMP / 'harness/vode_worker.py'), ARM[-2:], str(BASE_REQ), '2', '7']
    timeout = 600
elif ARM == 'JULIA_RODAS5P':
    xargv = [JUL, '--project=' + str(CAMP / 'harness/julia-env'), str(CAMP / 'harness/julia_solver_v3.jl'),
             str(CAMP / 'requests/julia-heat-Rodas5P.json')]
    xenv['JULIA_DEPOT_PATH'] = str(CAMP / 'julia-depot-overlay') + ':' + OLD_DEPOT
    timeout = 1800
elif ARM == 'DIFFSOL_BDF':
    xargv = [str(CAMP / 'bin/diffsol-adapter-v2'), str(CAMP / 'requests/diffsol-heat-BDF.json')]
    timeout = 120
else:
    raise SystemExit('unknown arm ' + ARM)


def x_row(info):
    row = dict(info)
    try:
        doc = json.loads(Path(info['stdout_path']).read_text())
    except Exception as e:  # noqa: BLE001
        row.update(status='OUTPUT_PARSE_FAILURE', parse_error=repr(e), success=False, application_pass=False)
        return row
    row['status'] = doc.get('status')
    meas = doc.get('measurements') or []
    reps = []
    for m in meas:
        times = m.get('times')
        supplied = times is not None
        if not supplied and ARM == 'DIFFSOL_BDF':
            times = output_times  # solve_dense(&times) evaluates at the requested grid; flagged below
        acc = evaluate(times, m.get('states'), output_times, ref, budget['rtol'], budget['atol'],
                       times_supplied=supplied)
        el = m.get('elapsed_ns')
        if el is None and m.get('elapsed_seconds') is not None:
            el = int(round(m['elapsed_seconds'] * 1e9))
        reps.append({'repetition': m.get('repetition'), 'elapsed_ns': el, 'accuracy': acc,
                     'counters': m.get('counters'), 'retcode': m.get('retcode') or m.get('stop_reason'),
                     'setup_seconds': m.get('setup_seconds')})
    row['measured_reps'] = reps
    row['meta'] = {k: v for k, v in doc.items() if k not in ('measurements', 'warmup')}
    row['success'] = (info['exit_code'] == 0 and doc.get('status') == 'OK' and len(reps) == 7
                      and all(r['accuracy']['valid'] for r in reps))
    row['application_pass'] = bool(row['success'] and all(r['accuracy']['application_pass'] for r in reps))
    els = [r['elapsed_ns'] for r in reps if r['elapsed_ns'] is not None]
    row['inprocess_median_ns'] = statistics.median(els) if els else None
    row['inprocess_min_ns'] = min(els) if els else None
    row['max_scaled_error'] = max((r['accuracy'].get('max_scaled_error', float('inf')) for r in reps),
                                  default=None)
    return row


if ARM == 'JULIA_RODAS5P':
    info = run_child(xargv, raw / 'preflight.json', raw / 'preflight.stderr.log', env=xenv, timeout=timeout)
    row = x_row(info)
    row.update(phase='B', arm=ARM, role='X', block_id=f'B-{ARM}-preflight', warmup=True, preflight=True,
               core=CORE, run_id=f'B-{ARM}-preflight')
    append_jsonl(out, row)
for k in range(WARM + CONF):
    block_id = f'B-{ARM}-c{CORE:02d}-b{k:02d}'
    order = ('STEN', 'X') if k % 2 == 0 else ('X', 'STEN')
    for pos, role in enumerate(order):
        run_id = f'{block_id}-{role}'
        if role == 'STEN':
            argv = [str(BIN), '--request', str(STEN_REQ), '--measurement-mode', 'timing']
            info = run_child(argv, raw / f'{run_id}.json', raw / f'{run_id}.stderr.log', env=env, timeout=120)
            row = vigilode_row(info, budget, ref, output_times)
        else:
            info = run_child(xargv, raw / f'{run_id}.json', raw / f'{run_id}.stderr.log', env=xenv,
                             timeout=timeout)
            row = x_row(info)
        row.update(phase='B', arm=ARM, role=role, core=CORE, block_id=block_id, block_index=k,
                   warmup=k < WARM, order=''.join(r[0] for r in order), position=pos, run_id=run_id,
                   affinity=sorted(os.sched_getaffinity(0)))
        append_jsonl(out, row)
print(json.dumps({'arm': ARM, 'core': CORE, 'blocks': WARM + CONF, 'status': 'WORKER_COMPLETED'}))
