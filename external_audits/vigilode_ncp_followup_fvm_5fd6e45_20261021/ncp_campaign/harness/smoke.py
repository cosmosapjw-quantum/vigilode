"""Step 2: exact handoff commands for both heat requests, then full validation.

Also freezes the independent reference and the application budget used by all
later phases. Usage: python3 smoke.py CAMP
"""
import json
import os
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from common import (append_jsonl, child_env, heat_reference_crosschecks, heat_reference_mp,  # noqa: E402
                    run_child, sha256_file, vigilode_row)

CAMP = Path(sys.argv[1])
SRC = CAMP / 'src'
TARGET = Path(os.environ['CARGO_TARGET_DIR'])
BIN = TARGET / 'measurement/examples/component_benchmark_native'
REQ = {'BASE': 'research/ncp_followup_20261021/accuracy/HEAT_BASELINE_REQUEST.json',
       'STEN': 'research/ncp_followup_20261021/accuracy/HEAT_STENCIL_REQUEST.json'}

base_case = json.loads((SRC / REQ['BASE']).read_text())['case']
sten_case = json.loads((SRC / REQ['STEN']).read_text())['case']
same_ivp = {k: base_case[k] == sten_case[k] for k in
            ('model', 'parameters', 'initial_state', 't_span', 'output_times', 'rtol', 'atol')}
budget = {'rtol': base_case['rtol'], 'atol': base_case['atol'],
          'metric': 'max over all output times and components of |y-ref|/(atol_i+rtol*|ref|) <= 1',
          'source': REQ['BASE'] + ' case.rtol/atol (original application budget)',
          'internal_tolerances_equal_application_budget_for_all_arms': True,
          'same_ivp_fields_baseline_vs_stencil': same_ivp}
t0 = time.monotonic()
ref = heat_reference_mp(base_case)
ref_seconds = time.monotonic() - t0
cross = heat_reference_crosschecks(base_case, ref)
(CAMP / 'results/APPLICATION_BUDGET.json').write_text(json.dumps(budget, indent=1) + '\n')
(CAMP / 'results/REFERENCE.json').write_text(json.dumps(
    {'kind': 'independent analytic Fourier (mpmath dps=40, direct real DFT of float64 IVP)',
     'output_times': base_case['output_times'], 'states': ref, 'crosschecks': cross,
     'compute_seconds': ref_seconds}, allow_nan=False) + '\n')
print('reference', cross, round(ref_seconds, 2), flush=True)

os.makedirs(CAMP / 'raw/smoke', exist_ok=True)
rows = []
os.chdir(SRC)
for arm in ('BASE', 'STEN'):
    argv = [str(BIN), '--request', REQ[arm], '--measurement-mode', 'timing']
    info = run_child(argv, CAMP / f'raw/smoke/{arm}.stdout.json', CAMP / f'raw/smoke/{arm}.stderr.log',
                     env=child_env(), timeout=120)
    row = vigilode_row(info, budget, ref, base_case['output_times'])
    row.update(phase='P0_SMOKE', arm=arm, run_id=f'P0-{arm}', cwd=str(SRC),
               request_sha256=sha256_file(SRC / REQ[arm]), binary_sha256=sha256_file(BIN))
    append_jsonl(CAMP / 'results/SMOKE.jsonl', row)
    rows.append(row)
    acc = row.get('accuracy') or {}
    print(arm, 'exit', row['exit_code'], 'status', row.get('status'), 'valid', acc.get('valid'),
          'refusal', acc.get('refusal'), 'max_scaled', acc.get('max_scaled_error'),
          'pass', row.get('application_pass'), 'elapsed_ns', (row.get('timing') or {}).get('elapsed_ns'),
          'wall_ns', row['process_wall_ns'], 'extra_rhs', (row.get('derivative_work') or {}).get('extra_rhs_calls'),
          flush=True)
a = json.loads(Path(rows[0]['stdout_path']).read_text())
b = json.loads(Path(rows[1]['stdout_path']).read_text())
ident = {'times_equal': a.get('times') == b.get('times'),
         'states_bit_identical': json.dumps(a.get('states')) == json.dumps(b.get('states')),
         'counters_equal': a.get('counters') == b.get('counters')}
(CAMP / 'results/SMOKE_IDENTITY.json').write_text(json.dumps(ident, indent=1) + '\n')
print('identity', ident)
ok = all(r.get('success') for r in rows)
sys.exit(0 if ok else 1)
