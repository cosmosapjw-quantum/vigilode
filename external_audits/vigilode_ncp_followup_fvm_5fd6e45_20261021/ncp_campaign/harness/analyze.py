"""Aggregate ncpf-20261021 campaign rows into summaries. Usage: python3 analyze.py CAMP_COPY_DIR"""
import glob
import hashlib
import json
import math
import statistics as st
import sys
from pathlib import Path

D = Path(sys.argv[1])
R = D / 'results'


def load(pattern):
    return [json.loads(l) for f in sorted(glob.glob(str(R / pattern))) for l in open(f)]


def rng(xs):
    xs = [x for x in xs if x is not None]
    if not xs:
        return None
    q = st.quantiles(xs, n=4) if len(xs) >= 2 else [xs[0]] * 3
    return {'n': len(xs), 'min': min(xs), 'q1': q[0], 'median': st.median(xs), 'q3': q[2], 'max': max(xs)}


def sign_test_p(k, n):
    # two-sided exact binomial p for k successes of n at p=1/2
    if n == 0:
        return None
    tail = sum(math.comb(n, i) for i in range(0, min(k, n - k) + 1)) / 2 ** n
    return min(1.0, 2 * tail)


def wilcoxon(diffs):
    try:
        from scipy.stats import wilcoxon as w
        return float(w(diffs).pvalue)
    except Exception:  # noqa: BLE001
        return None


def states_digest(path):
    doc = json.loads(Path(path).read_text())
    return hashlib.sha256(json.dumps([doc.get('times'), doc.get('states')]).encode()).hexdigest()


def localize(p):
    p = str(p)
    return D / p.split('/vigilode-ncpf-20261021T1031Z-5fd6e45/', 1)[1] if '/vigilode-ncpf-' in p else Path(p)


summary = {}
# ---------------- heat paired (A0, A) ----------------
pairs = load('PAIRS_*.jsonl')
heat = {}
for ph in sorted({r['phase'] for r in pairs}):
    P = [r for r in pairs if r['phase'] == ph]
    arms = {}
    for arm in ('BASE', 'STEN'):
        rr = [r for r in P if r['arm'] == arm]
        conf = [r for r in rr if not r['warmup']]
        digests = {states_digest(localize(r['stdout_path'])) for r in rr}
        arms[arm] = {
            'runs_total': len(rr), 'runs_confirmation': len(conf),
            'exit0': sum(r['exit_code'] == 0 for r in rr), 'status_OK': sum(r.get('status') == 'OK' for r in rr),
            'valid_full_trajectory': sum(bool(r.get('accuracy') and r['accuracy']['valid']) for r in rr),
            'application_pass': sum(r['application_pass'] for r in rr),
            'max_scaled_error_max': max(r['accuracy']['max_scaled_error'] for r in rr),
            'endpoint_scaled_error_max': max(r['accuracy']['endpoint_scaled_error'] for r in rr),
            'max_abs_error_max': max(r['accuracy']['max_abs_error'] for r in rr),
            'distinct_times_states_digests': len(digests), 'times_states_digest': sorted(digests),
            'counters_distinct': len({json.dumps(r['counters'], sort_keys=True) for r in rr}),
            'counters': rr[0]['counters'],
            'derivative_extra_rhs_calls': sorted({r['derivative_work']['extra_rhs_calls'] for r in rr}),
            'analytic_stencil_jacobian_builds': sorted({r['derivative_work']['analytic_stencil_jacobian_builds']
                                                        for r in rr}),
            'derivative_policy': sorted({json.dumps(r.get('derivative_policy')) for r in rr}),
            'warmed_elapsed_ns': rng([r['timing']['elapsed_ns'] for r in conf]),
            'in_process_warmup_s': rng([r['timing']['warmup_seconds'] for r in conf]),
            'cold_process_wall_ns': rng([r['process_wall_ns'] for r in conf]),
            'child_cpu_s': rng([r['child_user_s'] + r['child_sys_s'] for r in conf]),
            'child_maxrss_kb': rng([r['child_maxrss_kb'] for r in conf]),
            'cores': sorted({r['core'] for r in rr}),
        }
        c = rr[0]['counters']
        arms[arm]['rhs_total_including_derivative'] = c['rhs_calls'] + arms[arm]['derivative_extra_rhs_calls'][0]
    blocks = {}
    for r in P:
        if not r['warmup']:
            blocks.setdefault(r['block_id'], {})[r['arm']] = r
    ratios = {'warmed_elapsed': [], 'cold_process_wall': [], 'child_cpu': []}
    diffs = {'warmed_elapsed': [], 'cold_process_wall': []}
    order_split = {'BS': [], 'SB': []}
    block_ids = []
    for bid, b in sorted(blocks.items()):
        if 'BASE' in b and 'STEN' in b:
            block_ids.append(bid)
            eb, es = b['BASE']['timing']['elapsed_ns'], b['STEN']['timing']['elapsed_ns']
            wb, ws = b['BASE']['process_wall_ns'], b['STEN']['process_wall_ns']
            ratios['warmed_elapsed'].append(es / eb)
            ratios['cold_process_wall'].append(ws / wb)
            ratios['child_cpu'].append((b['STEN']['child_user_s'] + b['STEN']['child_sys_s'])
                                       / max(1e-9, b['BASE']['child_user_s'] + b['BASE']['child_sys_s']))
            diffs['warmed_elapsed'].append(es - eb)
            diffs['cold_process_wall'].append(ws - wb)
            order_split[b['BASE']['order']].append(es / eb)
    paired = {'blocks': len(block_ids), 'block_ids_first_last': [block_ids[0], block_ids[-1]] if block_ids else None,
              'ratio_STEN_over_BASE': {k: rng(v) for k, v in ratios.items()},
              'STEN_faster_blocks': {k: sum(x < 0 for x in v) for k, v in diffs.items()},
              'sign_test_p_two_sided': {k: sign_test_p(sum(x < 0 for x in v), len(v)) for k, v in diffs.items()},
              'wilcoxon_p_two_sided': {k: wilcoxon(v) for k, v in diffs.items()},
              'warmed_ratio_by_order': {k: rng(v) for k, v in order_split.items()},
              'timing_authority': 'diagnostic_only'}
    heat[ph] = {'arms': arms, 'paired': paired,
                'utc_range': [min(r['started_utc'] for r in P), max(r['started_utc'] for r in P)],
                'loadavg1_range': [min(r['loadavg_at_start'][0] for r in P), max(r['loadavg_at_start'][0] for r in P)]}
summary['heat_paired'] = heat

# ---------------- comparators (B) ----------------
comp = {}
for f in sorted(glob.glob(str(R / 'COMP_*.jsonl'))):
    rows = [json.loads(l) for l in open(f)]
    arm = rows[0]['arm']
    X = [r for r in rows if r['role'] == 'X']
    Xc = [r for r in X if not r['warmup']]
    S = [r for r in rows if r['role'] == 'STEN']
    Sc = [r for r in S if not r['warmup']]
    reps = [m for r in X for m in (r.get('measured_reps') or [])]
    blocks = {}
    for r in rows:
        if not r['warmup'] and r.get('block_id', '').count('-b'):
            blocks.setdefault(r['block_id'], {})[r['role']] = r
    pr = {'inprocess': [], 'process_wall': []}
    for b in blocks.values():
        if 'X' in b and 'STEN' in b and b['X'].get('inprocess_median_ns'):
            pr['inprocess'].append(b['X']['inprocess_median_ns'] / b['STEN']['timing']['elapsed_ns'])
            pr['process_wall'].append(b['X']['process_wall_ns'] / b['STEN']['process_wall_ns'])
    comp[arm] = {
        'core': rows[0]['core'], 'x_processes': len(X), 'x_confirmation_processes': len(Xc),
        'x_exit0': sum(r['exit_code'] == 0 for r in X), 'x_status_OK': sum(r.get('status') == 'OK' for r in X),
        'x_valid_all_reps': sum(bool(r.get('success')) for r in X),
        'x_application_pass_all_reps': sum(bool(r.get('application_pass')) for r in X),
        'measured_reps_total': len(reps),
        'reps_application_pass': sum(bool(m['accuracy'].get('application_pass')) for m in reps),
        'max_scaled_error': rng([m['accuracy'].get('max_scaled_error') for m in reps]),
        'endpoint_scaled_error': rng([m['accuracy'].get('endpoint_scaled_error') for m in reps]),
        'times_supplied_by_solver': sorted({m['accuracy'].get('times_supplied_by_solver') for m in reps}),
        'counters_distinct': len({json.dumps(m.get('counters'), sort_keys=True) for m in reps}),
        'counters': reps[0].get('counters') if reps else None,
        'inprocess_elapsed_ns_all_reps': rng([m['elapsed_ns'] for r in Xc for m in r['measured_reps']]),
        'cold_process_wall_ns': rng([r['process_wall_ns'] for r in Xc]),
        'child_cpu_s': rng([r['child_user_s'] + r['child_sys_s'] for r in Xc]),
        'child_maxrss_kb': rng([r['child_maxrss_kb'] for r in Xc]),
        'meta': X[0].get('meta'),
        'sten_in_blocks': {'runs': len(S), 'application_pass': sum(r['application_pass'] for r in S),
                           'warmed_elapsed_ns': rng([r['timing']['elapsed_ns'] for r in Sc]),
                           'cold_process_wall_ns': rng([r['process_wall_ns'] for r in Sc])},
        'paired_blocks': len(pr['inprocess']),
        'ratio_X_over_STEN': {k: rng(v) for k, v in pr.items()},
        'timing_scope_note': 'X in-process = solver-specific timer (see meta.timing_scope); STEN = adapter warmed solve incl. setup/output; cross-solver ratios are diagnostic_only',
        'utc_range': [min(r['started_utc'] for r in rows), max(r['started_utc'] for r in rows)],
    }
summary['comparators'] = comp

# ---------------- bordered (C) ----------------
bord = {}
for r in load('BORDERED.jsonl'):
    for p in r['probes']:
        e = bord.setdefault(str(p['n']), {'runs': 0, 'success_runs': 0, 'residual_inf_max': 0, 'max_solution_error_max': 0,
                                          'allocated_f64_slots': set(), 'expected_slots': p['expected_f64_slots_formula'],
                                          'factor_ns': [], 'apply_ns_per_call': [], 'exit_codes': set()})
        e['runs'] += 1
        e['success_runs'] += int(r['success'])
        e['residual_inf_max'] = max(e['residual_inf_max'], p['residual_inf'])
        e['max_solution_error_max'] = max(e['max_solution_error_max'], p['max_solution_error'])
        e['allocated_f64_slots'].add(p['allocated_f64_slots'])
        e['exit_codes'].add(r['exit_code'])
        if not r['warmup']:
            e['factor_ns'].append(p['factor_ns_diagnostic'])
            e['apply_ns_per_call'].append(p['apply_total_ns_diagnostic'] / p['apply_repetitions'])
for e in bord.values():
    e['allocated_f64_slots'] = sorted(e['allocated_f64_slots'])
    e['exit_codes'] = sorted(e['exit_codes'])
    e['storage_matches_formula'] = e['allocated_f64_slots'] == [e['expected_slots']]
    e['factor_ns'] = rng(e['factor_ns'])
    e['apply_ns_per_call'] = rng(e['apply_ns_per_call'])
summary['bordered_probe'] = bord

# ---------------- telemetry ----------------
tel = [json.loads(l) for l in open(R / 'TELEMETRY.jsonl')] if (R / 'TELEMETRY.jsonl').exists() else []
if tel:
    summary['telemetry'] = {'samples': len(tel), 'utc_range': [tel[0]['utc'], tel[-1]['utc']],
                            'loadavg1': rng([float(t['loadavg'][0]) for t in tel]),
                            'busy_total': rng([t['busy_total'] for t in tel]),
                            'busy_reserved_cpus_0_7_max': max(max(t['busy_per_cpu'][:8]) for t in tel),
                            'mem_available_kb': rng([t['meminfo_kb']['MemAvailable'] for t in tel])}
(R / 'ANALYSIS_SUMMARY.json').write_text(json.dumps(summary, indent=1, default=list) + '\n')
print(json.dumps(summary, indent=1, default=list)[:20000])
