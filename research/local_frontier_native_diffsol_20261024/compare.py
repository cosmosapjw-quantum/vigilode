#!/usr/bin/env python3
"""Serial, endpoint matched native DVODE / diffsol / VigilODE comparison.

Python only starts processes and checks immutable outputs. Native callbacks and
the timed solve of either external solver never pass through Python.
"""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import statistics
import subprocess
import sys


def sha(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as f:
        for chunk in iter(lambda: f.read(1 << 20), b''):
            h.update(chunk)
    return h.hexdigest()


def write_new(path, obj):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('x') as f:
        json.dump(obj, f, indent=2, sort_keys=True, allow_nan=False)
        f.write('\n')


def git(repo, *args):
    return subprocess.check_output(['git', '-C', str(repo), *args], text=True).strip()


def numbers(values):
    return ' '.join(format(float(v), '.17g') for v in values)


def native_contract(d, rtol, atol, cap):
    case = d['case_id']
    p = d['parameters']
    if case == 1:
        params = [p['k1'], p['k2'], p['k3'], 0]
    elif case == 10:
        if p['B'] != [0, 0, 1] or p['E'] != [0, 0, 0] or p['c'] != 1:
            raise ValueError('unsupported gyro coefficients')
        params = [p['omega_c'], 0, 0, 0]
    elif case == 15:
        params = [p['omega0'], p['beta'], p['gamma'], 0]
    elif case == 16:
        params = [p['alpha'], p['omega0'], p['r0'], 0]
    else:
        raise ValueError('unsupported case')
    lines = [f"{case} {len(d['initial_state'])} {d['size']}",
             numbers([d['t_start'], d['t_end'], rtol, atol, cap]),
             numbers(params), numbers(d['initial_state']), str(len(d['rhs_probes']))]
    for probe in d['rhs_probes']:
        lines.append(numbers([probe['t'], *probe['state'], *probe['rhs']]))
    return '\n'.join(lines) + '\n'


def high_precision_reference(d):
    import mpmath as mp
    with mp.workdps(80):
        c, p = d['case_id'], d['parameters']
        x = lambda v: mp.mpf(str(v))
        t = x(d['t_end'])
        if c == 1:
            return None
        if c == 10:
            g = mp.sqrt(1+x(1)**2+x(.2)**2)
            w = x(p['omega_c'])
            phase = w*t/g
            result = [mp.sin(phase)/w, (mp.cos(phase)-1)/w, x(.2)*t/g,
                      mp.cos(phase), -mp.sin(phase), x(.2)]
        elif c == 15:
            w0, beta, gamma = x(p['omega0']), x(p['beta']), x(p['gamma'])
            w = w0*mp.exp(beta*t)
            phase = w0/beta*mp.expm1(beta*t)
            amp = mp.exp(-gamma*t)
            result = [amp*mp.cos(phase), amp*(-gamma*mp.cos(phase)-w*mp.sin(phase))]
        elif c == 16:
            alpha, w = x(p['alpha']), x(p['omega0'])
            if p['r0'] != 1 or alpha != x(.1):
                raise ValueError('unsupported inspiral reference')
            radius = (1-4*alpha*t)**x(.25)
            phase = w*(1-radius**x(2.5))/(x(2.5)*alpha)
            result = [radius*mp.cos(phase), radius*mp.sin(phase)]
        else:
            raise ValueError('unsupported reference case')
        return [float(v) for v in result]


def finite_real(value):
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        return False
    try:
        return math.isfinite(value)
    except OverflowError:
        return False


def assess(result, d, exact, ref_bound, cap, execution):
    answer = {'execution_status': execution, 'reference_kind': d['model_contract']['reference_kind'],
              'reference_uncertainty_bound': ref_bound,
              'error_limit': cap, 'actual_error': None, 'status': 'PROCESS_FAILURE'}
    if execution != 'COMPLETED' or not isinstance(result, dict):
        return answer
    if result.get('solver_success') is False or result.get('status') in ('SOLVER_FAILED', 'CONTRACT_FAILED'):
        answer['status'] = 'SOLVER_FAILURE'
        return answer
    if result.get('model_match', {}).get('status') != 'MATCHED_AT_DECLARED_PROBES' and result.get('method') in ('diffsol-bdf', 'native-dvode-adams', 'native-dvode-bdf'):
        answer['status'] = 'MODEL_MISMATCH'
        return answer
    state = result.get('final_state')
    t = result.get('final_time')
    if (not isinstance(state, list) or len(state) != len(d['initial_state']) or
            any(not finite_real(v) for v in state) or
            not finite_real(t) or abs(t-d['t_end']) > 1e-10*max(1, abs(d['t_end']))):
        answer['status'] = 'OUTPUT_CONTRACT_FAILURE'
        return answer
    if exact is None:
        answer['status'] = 'UNVALIDATED_REFERENCE'
        return answer
    err = max(abs(a-b) for a, b in zip(state, exact))
    answer['actual_error'] = err
    answer['status'] = 'PASS_SCOPED' if err + answer['reference_uncertainty_bound'] <= cap else 'ERROR_CAP_FAILURE'
    return answer


def main():
    a = argparse.ArgumentParser()
    for name in ('frozen', 'vigilode', 'native', 'diffsol', 'output', 'netlib-source-dir'):
        a.add_argument('--'+name, type=Path, required=True)
    a.add_argument('--size', choices=('smoke', 'standard'), default='smoke')
    a.add_argument('--cases', nargs='+', type=int, default=[1, 10, 15, 16])
    a.add_argument('--repeats', type=int, default=5)
    a.add_argument('--rtol', type=float, default=1e-6)
    a.add_argument('--atol', type=float, default=1e-9)
    a.add_argument('--error-limit', type=float, default=1e-4)
    a.add_argument('--timeout', type=float, default=90)
    a.add_argument('--budget', type=float, default=7200)
    a.add_argument('--memory-gib', type=float, default=1)
    a.add_argument('--max-steps', type=int, default=200000)
    a.add_argument('--allow-dirty-smoke', action='store_true')
    args = a.parse_args()
    if (args.repeats < 1 or not 1 <= args.max_steps <= 2000000 or len(set(args.cases)) != len(args.cases) or
            any(c not in (1, 10, 15, 16) for c in args.cases) or
            any(not math.isfinite(x) or x <= 0 for x in (args.rtol, args.atol, args.error_limit, args.timeout, args.budget, args.memory_gib))):
        raise ValueError('invalid limits or cases')
    followup = Path(__file__).resolve().parents[2]
    source = {'commit': git(followup, 'rev-parse', 'HEAD'),
              'tree': git(followup, 'rev-parse', 'HEAD^{tree}'),
              'status': git(followup, 'status', '--porcelain=v1')}
    if source['status'] and not (args.allow_dirty_smoke and args.size == 'smoke'):
        raise ValueError('follow-up source dirty')
    frozen = args.frozen.resolve()
    frozen_identity = {'commit': git(frozen, 'rev-parse', 'HEAD'),
                       'tree': git(frozen, 'rev-parse', 'HEAD^{tree}'),
                       'status': git(frozen, 'status', '--porcelain=v1')}
    if (frozen_identity['commit'] != 'aaf1f834d4c589e734ca7851a56c11bb44f7047a' or
            frozen_identity['tree'] != '03b12c682c688155e0f8b08c52508e04738acb93' or frozen_identity['status']):
        raise ValueError('frozen checkout identity mismatch')
    local = frozen/'research/local_frontier_20261024/local'
    sys.path.insert(0, str(local))
    from campaign import run_process, topology, utc
    from external_models import exact, verify
    import mpmath
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    cpu = topology()['physical_cpu_selection'][0]
    env = {n: '1' for n in ('RAYON_NUM_THREADS', 'OMP_NUM_THREADS', 'OPENBLAS_NUM_THREADS',
                           'MKL_NUM_THREADS', 'BLIS_NUM_THREADS', 'VECLIB_MAXIMUM_THREADS')}
    methods = ('dopri5', 'rodas5p-mf', 'switching', 'native-dvode-adams', 'native-dvode-bdf', 'diffsol-bdf')
    binaries = {'vigilode': args.vigilode.resolve(), 'native': args.native.resolve(), 'diffsol': args.diffsol.resolve()}
    for path in binaries.values():
        if not path.is_file(): raise FileNotFoundError(path)
    netlib = args.netlib_source_dir.resolve()
    netlib_sources = ('vode.f', 'dgefa.f', 'dgesl.f', 'dgbfa.f', 'dgbsl.f')
    for name in netlib_sources:
        if not (netlib/name).is_file(): raise FileNotFoundError(netlib/name)
    compiler = {n: subprocess.check_output([n, '--version'], text=True).splitlines()[0]
                for n in ('gfortran', 'g++', 'rustc')}
    native_links = subprocess.check_output(['ldd', str(binaries['native'])], text=True)
    identity = {'created_utc': utc(), 'followup_source': source, 'frozen_source': frozen_identity,
                'binary_sha256': {k: sha(v) for k, v in binaries.items()},
                'netlib_source_sha256': {n: sha(netlib/n) for n in netlib_sources},
                'native_driver_sha256': sha(followup/'research/local_frontier_native_diffsol_20261024/native/native_dvode.cpp'),
                'diffsol_source_sha256': sha(followup/'tools/benchmarks/diffsol/local_frontier_native/src/main.rs'),
                'diffsol_lock_sha256': sha(followup/'tools/benchmarks/diffsol/local_frontier_native/Cargo.lock'),
                'compiler_versions': compiler, 'native_ldd': native_links,
                'mpmath_version': mpmath.__version__,
                'external_model_sha256': sha(local/'external_models.py'),
                'campaign_sha256': sha(local/'campaign.py'),
                'runner_sha256': sha(__file__), 'cpu': cpu, 'topology': topology(),
                'environment': env, 'size': args.size, 'cases': args.cases,
                'methods': methods, 'repeats': args.repeats, 'warmup_per_method': 1,
                'rtol': args.rtol, 'atol': args.atol, 'error_limit': args.error_limit,
                'timeout': args.timeout, 'budget': args.budget, 'memory_gib': args.memory_gib,
                'max_steps': args.max_steps,
                'timing_authority': 'diagnostic_only',
                'timing_scope': 'internal solver elapsed reported per binary; process wall and /usr/bin/time resource are separate',
                'native_callback_path': 'compiled C++ -> Fortran DVODE, no Python in timed solve',
                'reference_uncertainty_policy': '80-digit analytic formula cross-checked against frozen descriptor and independent float formula; cross-check spread plus 1e-12 <= 10% cap; case 1 unvalidated'}
    write_new(output/'IDENTITY.json', identity)
    descriptors = {}
    for case in args.cases:
        descriptor_run = subprocess.run([str(binaries['vigilode']), '--describe', '--case', str(case), '--size', args.size],
                                        capture_output=True, text=True, timeout=30)
        record = {'command': [str(binaries['vigilode']), '--describe', '--case', str(case), '--size', args.size],
                  'exit_code': descriptor_run.returncode, 'stdout': descriptor_run.stdout, 'stderr': descriptor_run.stderr}
        write_new(output/f'descriptor_{case}_command.json', record)
        descriptor_run.check_returncode()
        d = json.loads(descriptor_run.stdout)
        match = verify(d)
        write_new(output/f'model_{case}.json', d)
        (output/f'native_{case}.txt').write_text(native_contract(d, args.rtol, args.atol, args.error_limit))
        ex = high_precision_reference(d)
        if ex is not None:
            float_ex = exact(d, d['t_end'])
            if d['exact_final'] is None or float_ex is None:
                raise ValueError('analytic reference absent')
            discrepancy = max(abs(x-y) for seq in (d['exact_final'], float_ex)
                              for x,y in zip(ex, seq))
            ref_bound = discrepancy + 1e-12
            if ref_bound > args.error_limit/10:
                raise ValueError('reference uncertainty exceeds 10% of cap')
        else:
            discrepancy = ref_bound = None
        write_new(output/f'reference_{case}.json', {'source':'80-digit mpmath analytic expression',
                  'reference_state':ex,'cross_check_spread':discrepancy,
                  'conservative_uncertainty_bound':ref_bound,
                  'descriptor_exact_final':d['exact_final']})
        descriptors[case] = (d, ex, ref_bound, match)
    import time
    began = time.monotonic()
    receipts = []
    pending = []
    for case in args.cases:
        d, ex, ref_bound, match = descriptors[case]
        for repeat in range(-1, args.repeats):
            order = methods[(repeat+1) % len(methods):] + methods[:(repeat+1) % len(methods)]
            for method in order:
                if time.monotonic()-began >= args.budget:
                    pending.append({'case': case, 'repeat': repeat, 'method': method, 'status': 'NOT_RUN_BUDGET'})
                    continue
                folder = output/'runs'/f'{case}_{args.size}_{repeat}_{method}'
                if method.startswith('native-dvode'):
                    mode = method.removeprefix('native-dvode-')
                    cmd = [str(binaries['native']), str(output/f'native_{case}.txt'), mode, str(args.max_steps)]
                elif method == 'diffsol-bdf':
                    cmd = [str(binaries['diffsol']), '--contract', str(output/f'model_{case}.json'),
                           '--rtol', str(args.rtol), '--atol', str(args.atol), '--error-limit', str(args.error_limit)]
                else:
                    cmd = [str(binaries['vigilode']), '--case', str(case), '--size', args.size,
                           '--method', method, '--parallel-layer', 'serial', '--threads', '1',
                           '--rtol', str(args.rtol), '--atol', str(args.atol), '--max-steps', str(args.max_steps), '--emit-state']
                raw = run_process(cmd, folder, args.timeout, args.memory_gib, [cpu], env)
                assessment = assess(raw['result'], d, ex, ref_bound, args.error_limit, raw['execution_status'])
                receipt = {'case': case, 'size': args.size, 'repeat': repeat, 'warmup': repeat == -1,
                           'method': method, 'raw_receipt': raw, 'assessment': assessment,
                           'descriptor_sha256': sha(output/f'model_{case}.json'),
                           'native_contract_sha256': sha(output/f'native_{case}.txt'),
                           'binary_sha256': identity['binary_sha256']['native' if method.startswith('native-dvode') else 'diffsol' if method == 'diffsol-bdf' else 'vigilode']}
                write_new(folder/'COMPARISON_RECEIPT.json', receipt)
                receipts.append(receipt)
                print(json.dumps({'case':case,'size':args.size,'repeat':repeat,'method':method,
                                  'execution':raw['execution_status'],'scientific':assessment['status'],
                                  'actual_error':assessment['actual_error']}),flush=True)
    groups = {}
    failures = []
    for r in receipts:
        if r['raw_receipt']['execution_status'] != 'COMPLETED' or r['assessment']['status'] != 'PASS_SCOPED':
            failures.append({'case': r['case'], 'repeat': r['repeat'], 'method': r['method'],
                             'warmup': r['warmup'], 'execution_status': r['raw_receipt']['execution_status'],
                             'scientific_status': r['assessment']['status'],
                             'receipt': f"runs/{r['case']}_{args.size}_{r['repeat']}_{r['method']}/COMPARISON_RECEIPT.json"})
    for case in args.cases:
        for method in methods:
            rows = [r for r in receipts if r['case']==case and r['method']==method and not r['warmup']]
            good = [r for r in rows if r['assessment']['status']=='PASS_SCOPED' and
                    r['raw_receipt']['result'] is not None and
                    isinstance(r['raw_receipt']['result'].get('elapsed_seconds'),(int,float))]
            values = [r['raw_receipt']['result']['elapsed_seconds'] for r in good]
            groups[f'{case}:{method}'] = {'attempts':len(rows),'accurate_timed_samples':len(good),
                                         'status':'ELIGIBLE' if len(good)==args.repeats else 'NOT_ELIGIBLE',
                                         'median_seconds':statistics.median(values) if values else None,
                                         'min_seconds':min(values) if values else None,
                                         'max_seconds':max(values) if values else None,
                                         'worst_actual_error':max((r['assessment']['actual_error'] for r in good),default=None)}
    summary = {'created_utc':utc(),'identity_sha256':sha(output/'IDENTITY.json'),'groups':groups,
               'pending':pending,'failures':failures,'attempt_count':len(receipts),
               'eligible_speed_ratios_vs_dopri5':{},
               'not_run':[{'method':'julia','status':'NOT_RUN','reason':'No pinned Julia Project/Manifest in this native-focused run'}],
               'scope':'Endpoint only; case 1 has no independent reference; no broad production qualification'}
    for case in args.cases:
        baseline = groups[f'{case}:dopri5']
        if baseline['status'] != 'ELIGIBLE': continue
        for method in methods:
            group = groups[f'{case}:{method}']
            if group['status'] == 'ELIGIBLE':
                summary['eligible_speed_ratios_vs_dopri5'][f'{case}:{method}'] = group['median_seconds']/baseline['median_seconds']
    write_new(output/'SUMMARY.json', summary)
    lines = ['# Native DVODE and diffsol local comparison', '',
             'Python orchestrates processes and checks outputs; native DVODE and diffsol timed solves use compiled callbacks.',
             'The source identities and raw outputs are in IDENTITY.json and runs/. Times are diagnostic only.', '',
             '| Case | Method | Accurate timed / attempted | Median s | Max error | Status |',
             '|---:|---|---:|---:|---:|---|']
    for case in args.cases:
        for method in methods:
            g = groups[f'{case}:{method}']
            lines.append(f"| {case} | {method} | {g['accurate_timed_samples']}/{g['attempts']} | {g['median_seconds']} | {g['worst_actual_error']} | {g['status']} |")
    (output/'REPORT.md').write_text('\n'.join(lines)+'\n')


if __name__ == '__main__':
    main()
