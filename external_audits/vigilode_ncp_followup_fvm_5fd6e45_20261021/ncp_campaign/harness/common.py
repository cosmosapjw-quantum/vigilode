"""Shared helpers for the ncpf-20261021 NCP follow-up campaign (heat n128).

Reference: independent Fourier solution of the periodic_heat semi-discrete ODE
    y_i' = D (y_{i-1} - 2 y_i + y_{i+1}),  D = diffusion * n^2  (float64 value),
evaluated in mpmath from the exact float64 IVP data. No solver output is used.
"""
import hashlib
import json
import math
import os
import subprocess
import threading
import time
from pathlib import Path


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, 'rb') as f:
        for chunk in iter(lambda: f.read(1 << 20), b''):
            h.update(chunk)
    return h.hexdigest()


def utc_now():
    return time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())


def append_jsonl(path, row):
    with open(path, 'a') as f:
        f.write(json.dumps(row, allow_nan=False, separators=(',', ':')) + '\n')
        f.flush()
        os.fsync(f.fileno())


def heat_reference_mp(case, dps=40):
    import mpmath as mp
    mp.mp.dps = dps
    y0 = [mp.mpf(x) for x in case['initial_state']]
    n = len(y0)
    D = mp.mpf(case['parameters']['diffusion']) * n * n
    c = [mp.cos(2 * mp.pi * m / n) for m in range(n)]
    s = [mp.sin(2 * mp.pi * m / n) for m in range(n)]
    ac = [mp.fsum(y0[j] * c[(j * k) % n] for j in range(n)) / n for k in range(n)]
    asn = [mp.fsum(y0[j] * s[(j * k) % n] for j in range(n)) / n for k in range(n)]
    lam = [-4 * D * mp.sin(mp.pi * k / n) ** 2 for k in range(n)]
    out = []
    for t in case['output_times']:
        tm = mp.mpf(t)
        e = [mp.exp(lam[k] * tm) for k in range(n)]
        bc = [e[k] * ac[k] for k in range(n)]
        bs = [e[k] * asn[k] for k in range(n)]
        row = [float(mp.fsum(bc[k] * c[(j * k) % n] + bs[k] * s[(j * k) % n] for k in range(n)))
               for j in range(n)]
        out.append(row)
    return out


def heat_reference_crosschecks(case, ref):
    """Compare mp reference with numpy FFT and the archived single-mode closed form."""
    import numpy as np
    y0 = np.array(case['initial_state'], dtype=float)
    n = len(y0)
    D = case['parameters']['diffusion'] * n * n
    k = np.arange(n)
    lam = -4.0 * D * np.sin(np.pi * k / n) ** 2
    yh = np.fft.fft(y0)
    d_fft = 0.0
    d_mode = 0.0
    for t, r in zip(case['output_times'], ref):
        f = np.real(np.fft.ifft(yh * np.exp(lam * t)))
        d_fft = max(d_fft, float(np.max(np.abs(f - np.array(r)))))
        q = math.exp(-4 * case['parameters']['diffusion'] * n * n * math.sin(math.pi / n) ** 2 * t)
        d_mode = max(d_mode, max(abs(x * q - y) for x, y in zip(case['initial_state'], r)))
    return {'max_abs_diff_numpy_fft': d_fft, 'max_abs_diff_single_mode_closed_form': d_mode}


def evaluate(times, states, output_times, ref, rtol_app, atol_app, times_supplied=True):
    """Full-trajectory application-budget evaluation; refuses malformed output."""
    res = {'valid': False, 'refusal': None, 'times_supplied_by_solver': times_supplied}
    if not isinstance(states, list) or len(states) == 0:
        res['refusal'] = 'empty_states'
        return res
    if times is None:
        res['refusal'] = 'missing_times'
        return res
    if len(times) != len(output_times):
        res['refusal'] = f'time_count {len(times)} != {len(output_times)}'
        return res
    if any(float(a) != float(b) for a, b in zip(times, output_times)):
        res['refusal'] = 'times_not_exactly_requested'
        return res
    if len(states) != len(output_times):
        res['refusal'] = f'state_count {len(states)} != {len(output_times)}'
        return res
    n = len(ref[0])
    for row in states:
        if not isinstance(row, list) or len(row) != n:
            res['refusal'] = 'state_shape'
            return res
        for x in row:
            if not isinstance(x, (int, float)) or not math.isfinite(x):
                res['refusal'] = 'nonfinite_or_nonnumeric'
                return res
    mse = mae = 0.0
    arg = None
    end_se = end_ae = 0.0
    last = len(states) - 1
    for ti, (row, rr) in enumerate(zip(states, ref)):
        for j, (y, r) in enumerate(zip(row, rr)):
            d = abs(y - r)
            z = d / (atol_app[j] + rtol_app * abs(r))
            if z > mse:
                mse, arg = z, (ti, j)
            mae = max(mae, d)
            if ti == last:
                end_se = max(end_se, z)
                end_ae = max(end_ae, d)
    res.update(valid=True, max_scaled_error=mse, argmax_time_index_component=arg, max_abs_error=mae,
               endpoint_scaled_error=end_se, endpoint_abs_error=end_ae,
               application_pass=mse <= 1.0)
    return res


def run_child(argv, stdout_path, stderr_path, env=None, timeout=None):
    """Run argv; return exact wall (monotonic ns) and child rusage via wait4."""
    la = os.getloadavg()
    started_utc = utc_now()
    with open(stdout_path, 'wb') as fo, open(stderr_path, 'wb') as fe:
        t0 = time.monotonic_ns()
        p = subprocess.Popen(argv, stdout=fo, stderr=fe, env=env)
        killed = {'v': False}

        def _kill():
            killed['v'] = True
            try:
                p.kill()
            except ProcessLookupError:
                pass
        timer = threading.Timer(timeout, _kill) if timeout else None
        if timer:
            timer.start()
        _, status, ru = os.wait4(p.pid, 0)
        t1 = time.monotonic_ns()
        if timer:
            timer.cancel()
        p.returncode = os.waitstatus_to_exitcode(status)
    return {'argv': argv, 'exit_code': p.returncode, 'timed_out': killed['v'],
            'process_wall_ns': t1 - t0, 'monotonic_start_ns': t0, 'monotonic_end_ns': t1,
            'started_utc': started_utc, 'child_user_s': ru.ru_utime, 'child_sys_s': ru.ru_stime,
            'child_maxrss_kb': ru.ru_maxrss, 'loadavg_at_start': la,
            'stdout_path': str(stdout_path), 'stdout_sha256': sha256_file(stdout_path),
            'stdout_bytes': os.path.getsize(stdout_path), 'stderr_path': str(stderr_path)}


def child_env():
    env = dict(os.environ)
    env.update(OPENBLAS_NUM_THREADS='1', OMP_NUM_THREADS='1', MKL_NUM_THREADS='1',
               RAYON_NUM_THREADS='1', JULIA_NUM_THREADS='1', PYTHONDONTWRITEBYTECODE='1')
    return env


def vigilode_row(info, budget, ref, output_times):
    """Parse adapter stdout; status OK alone is not success."""
    row = dict(info)
    try:
        out = json.loads(Path(info['stdout_path']).read_text())
    except Exception as e:  # noqa: BLE001
        row.update(status='OUTPUT_PARSE_FAILURE', parse_error=repr(e))
        return row
    row['status'] = out.get('status')
    row['reason'] = out.get('reason')
    row['timing'] = out.get('timing')
    row['counters'] = out.get('counters')
    row['solver_details'] = out.get('solver_details')
    row['derivative_work'] = out.get('derivative_work')
    row['derivative_policy'] = out.get('derivative_policy')
    row['measurement_mode'] = out.get('measurement_mode')
    if out.get('status') == 'OK':
        row['accuracy'] = evaluate(out.get('times'), out.get('states'), output_times, ref,
                                   budget['rtol'], budget['atol'])
    else:
        row['accuracy'] = None
    row['success'] = (info['exit_code'] == 0 and out.get('status') == 'OK'
                      and bool(row['accuracy']) and row['accuracy'].get('valid') is True)
    row['application_pass'] = bool(row['success'] and row['accuracy']['application_pass'])
    return row
