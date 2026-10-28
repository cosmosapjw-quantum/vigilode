#!/usr/bin/env python3
"""Recompute saved native endpoint errors; never infer unsaved trajectory values."""
import argparse
import hashlib
import json
import math
from pathlib import Path


def endpoint_metrics(case, state):
    if case['model'] != 'rotation' or len(state) != 2:
        raise ValueError('two-component rotation required')
    t0, t1 = case['t_span']
    x0, y0 = case['initial_state']
    phase = case['parameters']['omega'] * (t1 - t0)
    c, s = math.cos(phase), math.sin(phase)
    ref = [c*x0-s*y0, s*x0+c*y0]
    atol, rtol = case['atol'], case['rtol']
    if len(atol) != 2 or not all(math.isfinite(v) for v in [*state, *ref, *atol, rtol]):
        raise ValueError('finite state/reference/tolerances required')
    if rtol < 0 or any(v <= 0 for v in atol):
        raise ValueError('positive atol, nonnegative rtol required')
    errors = [abs(a-b) for a,b in zip(state,ref)]
    scaled = [e/(a+rtol*abs(r)) for e,a,r in zip(errors,atol,ref)]
    norm = math.hypot(*ref)
    # Dot/cross relative phase avoids subtracting two large unwrapped angles.
    drift = None if norm == 0 or math.hypot(*state) == 0 else math.atan2(ref[0]*state[1]-ref[1]*state[0], ref[0]*state[0]+ref[1]*state[1])
    return {'endpoint_reference':ref, 'endpoint_abs_error':max(errors),
            'endpoint_scaled_error':max(scaled),'endpoint_accuracy_pass':max(scaled)<=1,
            'endpoint_phase_error_rad':drift,
            'endpoint_amplitude_error':abs(math.hypot(*state)-norm)}


def analyze(bundle):
    path = bundle/'ROTATION_TOLERANCE_RESULTS_RECOVERED.jsonl'
    cases = json.loads((bundle/'harness/cases.json').read_text())
    base = next(c for c in cases if c['case_id']=='C-rotation-w10')
    records = []
    for raw in path.read_text().splitlines():
        r = json.loads(raw)
        if r.get('solver') != 'vigilode' or r.get('warmup',False):
            continue
        case = dict(base, rtol=r['rtol'], atol=r['atol'])
        metric = endpoint_metrics(case,r['states_final'])
        stored = r['accuracy']['endpoint_max_scaled_error']
        if not math.isclose(metric['endpoint_scaled_error'],stored,rel_tol=2e-12,abs_tol=1e-12):
            raise ValueError('saved endpoint metric disagrees with direct exact calculation')
        records.append(dict(rtol=r['rtol'],repetition=r['repetition'],**metric,
                            saved_full_sampled_grid_pass=r['accuracy']['accuracy_pass_scaled_le_1']))
    summary=[]
    for tol in sorted({r['rtol'] for r in records}, reverse=True):
        selected=[r for r in records if r['rtol']==tol]
        summary.append({'rtol':tol,'atol':base['atol'],'measurements':len(selected),
                        'endpoint_passes':sum(r['endpoint_accuracy_pass'] for r in selected),
                        'saved_sampled_grid_passes':sum(r['saved_full_sampled_grid_pass'] for r in selected),
                        'endpoint_scaled_error':selected[0]['endpoint_scaled_error'],
                        'endpoint_phase_error_rad':selected[0]['endpoint_phase_error_rad']})
    return {'schema_version':1,'source_bundle_commit':'7a63876a97fc17f640771ec06658269947049994',
            'input_sha256':hashlib.sha256(path.read_bytes()).hexdigest(),
            'input':str(path),'status':'implementation-verified',
            'scope':'Recomputed saved native endpoint only; saved full-grid flags separately identified.',
            'summary':summary,'records':records}


if __name__=='__main__':
    p=argparse.ArgumentParser()
    p.add_argument('--bundle',type=Path,required=True)
    p.add_argument('--out',type=Path,required=True)
    args=p.parse_args()
    args.out.write_text(json.dumps(analyze(args.bundle),indent=2,allow_nan=False)+'\n')
