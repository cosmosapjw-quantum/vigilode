#!/usr/bin/env python3
"""Process-isolated Python/ctypes wrapper for Hairer's RADAU5 and RODAS.

The measured lane includes Python callbacks and ctypes dispatch.  The Fortran
integrator advances its own mesh; SOLOUT evaluates the requested grid with
CONTR5/CONTRO, never by restarting at output points.
"""
from __future__ import annotations

import argparse
import contextlib
import ctypes as C
import json
import math
import os
from pathlib import Path
import sys
import tempfile
import time

if __package__ in (None, ''):
    sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from tools.component_benchmark.models import jac, rhs
from tools.component_benchmark.protocol import strict_load, validate_case
from tools.component_benchmark.scipy_adapter import probe

INT = C.c_int
REAL = C.c_double
PI = C.POINTER(INT)
PR = C.POINTER(REAL)
FCN = C.CFUNCTYPE(None, PI, PR, PR, PR, PR, PI)
JAC = C.CFUNCTYPE(None, PI, PR, PR, PR, PI, PR, PI)
RAD_SOL = C.CFUNCTYPE(None, PI, PR, PR, PR, PR, PR, PI, PI, PR, PI, PI)
ROD_SOL = C.CFUNCTYPE(None, PI, PR, PR, PR, PR, PI, PI, PR, PI, PI)


def _blank(status: str, reason: str) -> dict:
    return {'status': status, 'reason': reason, 'times': [], 'states': [],
            'counters': None, 'allocations': None,
            'timing': {'warmup_seconds': None, 'timed_seconds': None,
                       'timing_authority': 'diagnostic_only'}}


@contextlib.contextmanager
def _capture_native_fds():
    """Keep Fortran unit-6 diagnostics out of the one-object JSON stdout."""
    libc = C.CDLL(None)
    libc.fflush(None)
    with tempfile.TemporaryFile(mode='w+b') as out, tempfile.TemporaryFile(mode='w+b') as err:
        saved_out, saved_err = os.dup(1), os.dup(2)
        captured = {'stdout': '', 'stderr': ''}
        try:
            os.dup2(out.fileno(), 1)
            os.dup2(err.fileno(), 2)
            yield captured
        finally:
            libc.fflush(None)
            os.dup2(saved_out, 1)
            os.dup2(saved_err, 2)
            os.close(saved_out)
            os.close(saved_err)
            out.seek(0)
            err.seek(0)
            captured['stdout'] = out.read().decode('utf-8', errors='replace')
            captured['stderr'] = err.read().decode('utf-8', errors='replace')


def _library(solver_id: str):
    names = {'hairer-radau5': ('libhairer_radau.so', 'radau5_', 'contr5_'),
             'hairer-rodas': ('libhairer_rodas.so', 'rodas_', 'contro_')}
    if solver_id not in names:
        raise ValueError('unknown Hairer solver_id')
    directory = os.environ.get('VIGILODE_HAIRER_LIBDIR')
    if not directory:
        raise FileNotFoundError('VIGILODE_HAIRER_LIBDIR is unset')
    library_path = Path(directory) / names[solver_id][0]
    lib = C.CDLL(str(library_path))
    entry = getattr(lib, names[solver_id][1])
    dense = getattr(lib, names[solver_id][2])
    dense.argtypes = (PI, PR, PR, PI)
    dense.restype = REAL
    return entry, dense, str(library_path)


def _one_run(case: dict, solver_id: str, entry, dense,
             check_dense_endpoints: bool = False) -> tuple[dict, float]:
    started = time.perf_counter()
    nval = len(case['initial_state'])
    n = INT(nval)
    t0, tf = map(float, case['t_span'])
    grid = list(map(float, case['output_times']))
    x = REAL(t0)
    xend = REAL(tf)
    h = REAL(1e-6)
    y = (REAL * nval)(*case['initial_state'])
    rtol = (REAL * nval)(*([float(case['rtol'])] * nval))
    atol = (REAL * nval)(*map(float, case['atol']))
    itol, ijac, mljac, mujac = INT(1), INT(1), INT(nval), INT(0)
    imas, mlmas, mumas, iout = INT(0), INT(0), INT(0), INT(1)
    ifcn, idfx = INT(1), INT(0)
    lwork = INT((4 if solver_id == 'hairer-radau5' else 2) * nval * nval
                + (12 if solver_id == 'hairer-radau5' else 14) * nval + 20)
    liwork = INT((3 if solver_id == 'hairer-radau5' else 1) * nval + 20)
    work = (REAL * lwork.value)()
    iwork = (INT * liwork.value)()
    rpar, ipar, idid = (REAL * 1)(), (INT * 1)(), INT(0)
    states: list[list[float]] = [list(map(float, case['initial_state']))]
    next_output = 1
    errors: list[str] = []
    previous_accepted = list(map(float, case['initial_state']))
    dense_endpoint_check = {'left_max_abs': 0.0, 'right_max_abs': 0.0,
                            'left_max_budget_ratio': 0.0,
                            'right_max_budget_ratio': 0.0,
                            'checked_intervals': 0}

    def callback_fcn(nptr, tptr, yptr, fptr, _rpar, _ipar):
        try:
            values = rhs(case, tptr[0], [yptr[i] for i in range(nval)])
            if len(values) != nval:
                raise ValueError('RHS dimension mismatch')
            for i, value in enumerate(values):
                fptr[i] = value
        except Exception as exc:
            errors.append(f'FCN {type(exc).__name__}: {exc}')
            for i in range(nval):
                fptr[i] = math.nan

    def callback_jac(nptr, tptr, yptr, dfyptr, ldptr, _rpar, _ipar):
        try:
            matrix = jac(case, tptr[0], [yptr[i] for i in range(nval)])
            ld = ldptr[0]
            if len(matrix) != nval or any(len(row) != nval for row in matrix):
                raise ValueError('Jacobian dimension mismatch')
            for j in range(nval):
                for i in range(nval):
                    dfyptr[i + j * ld] = matrix[i][j]
        except Exception as exc:
            errors.append(f'JAC {type(exc).__name__}: {exc}')
            for j in range(nval):
                for i in range(nval):
                    dfyptr[i + j * ldptr[0]] = math.nan

    def output(nrptr, xoldptr, xptr, yptr, contptr, lrcptr, irtrnptr):
        nonlocal next_output, previous_accepted
        if errors:
            irtrnptr[0] = -1
            return
        try:
            # NR=1 is the initial callback; the initial value is already saved.
            if nrptr[0] == 1:
                return
            old, new = xoldptr[0], xptr[0]
            if check_dense_endpoints:
                for side, tcheck, accepted in (
                    ('left', old, previous_accepted),
                    ('right', new, [yptr[i] for i in range(nval)]),
                ):
                    point = REAL(tcheck)
                    for i, expected in enumerate(accepted):
                        actual = dense(C.byref(INT(i + 1)), C.byref(point),
                                       contptr, lrcptr)
                        diff = abs(actual - expected)
                        budget = atol[i] + rtol[i] * abs(expected)
                        dense_endpoint_check[f'{side}_max_abs'] = max(
                            dense_endpoint_check[f'{side}_max_abs'], diff)
                        dense_endpoint_check[f'{side}_max_budget_ratio'] = max(
                            dense_endpoint_check[f'{side}_max_budget_ratio'],
                            diff / budget)
                dense_endpoint_check['checked_intervals'] += 1
                previous_accepted = [yptr[i] for i in range(nval)]
            fuzz = 8 * math.ulp(max(abs(old), abs(new), 1.0))
            while next_output < len(grid) and grid[next_output] <= new + fuzz:
                target = grid[next_output]
                if target < old - fuzz:
                    raise ArithmeticError(f'output {target} skipped by interval [{old},{new}]')
                if abs(target - new) <= fuzz:
                    row = [yptr[i] for i in range(nval)]
                else:
                    evaluation_time = REAL(target)
                    row = [dense(C.byref(INT(i + 1)), C.byref(evaluation_time),
                                 contptr, lrcptr) for i in range(nval)]
                if not all(math.isfinite(v) for v in row):
                    raise ArithmeticError(f'nonfinite dense output at {target}')
                states.append(row)
                next_output += 1
        except Exception as exc:
            errors.append(f'SOLOUT {type(exc).__name__}: {exc}')
            irtrnptr[0] = -1

    def rad_output(nr, old, now, state, cont, _werr, lrc, _ns, _rp, _ip, stop):
        output(nr, old, now, state, cont, lrc, stop)

    def rod_output(nr, old, now, state, cont, lrc, _ns, _rp, _ip, stop):
        output(nr, old, now, state, cont, lrc, stop)

    fcn, jcb = FCN(callback_fcn), JAC(callback_jac)
    sol = RAD_SOL(rad_output) if solver_id == 'hairer-radau5' else ROD_SOL(rod_output)
    # Fortran never calls these pointers with IMAS=0 and IDFX=0, but it still
    # expects non-null external arguments at the ABI boundary.
    dummy = C.cast(fcn, C.c_void_p)
    with _capture_native_fds() as diagnostic:
        if solver_id == 'hairer-radau5':
            entry(C.byref(n), fcn, C.byref(x), y, C.byref(xend), C.byref(h),
                  rtol, atol, C.byref(itol), jcb, C.byref(ijac),
                  C.byref(mljac), C.byref(mujac), dummy, C.byref(imas),
                  C.byref(mlmas), C.byref(mumas), sol, C.byref(iout), work,
                  C.byref(lwork), iwork, C.byref(liwork), rpar, ipar,
                  C.byref(idid))
        else:
            entry(C.byref(n), fcn, C.byref(ifcn), C.byref(x), y,
                  C.byref(xend), C.byref(h), rtol, atol, C.byref(itol),
                  jcb, C.byref(ijac), C.byref(mljac), C.byref(mujac),
                  dummy, C.byref(idfx), dummy, C.byref(imas),
                  C.byref(mlmas), C.byref(mumas), sol, C.byref(iout),
                  work, C.byref(lwork), iwork, C.byref(liwork), rpar,
                  ipar, C.byref(idid))
    elapsed = time.perf_counter() - started
    counters = {'rhs_evaluations': iwork[13], 'jacobian_evaluations': iwork[14],
                'computed_steps': iwork[15], 'accepted_steps': iwork[16],
                'rejected_steps': iwork[17], 'lu_factorizations': iwork[18],
                'linear_solves': iwork[19]}
    result = {'status': 'OK' if idid.value == 1 and next_output == len(grid)
              and not errors else 'NUMERICAL_FAILURE',
              'times': grid if next_output == len(grid) else grid[:next_output],
              'states': states, 'counters': counters, 'allocations': None,
              'fortran_idid': idid.value, 'fortran_diagnostics': diagnostic,
              'dense_endpoint_check': dense_endpoint_check if check_dense_endpoints else None}
    if result['status'] != 'OK':
        result['reason'] = errors[0] if errors else (
            f'Fortran IDID={idid.value}, endpoint={x.value}, '
            f'output_points={next_output}/{len(grid)}')
    return result, elapsed


def solve(request: dict) -> dict:
    case, solver_id = request.get('case'), request.get('solver_id')
    if not isinstance(case, dict) or validate_case(case):
        return _blank('ENVIRONMENT_FAILURE', 'invalid CaseSpec')
    if solver_id not in ('hairer-radau5', 'hairer-rodas'):
        return _blank('UNSUPPORTED_CAPABILITY', 'unknown Hairer solver_id')
    if request.get('measurement_mode', 'timing') != 'timing':
        return _blank('UNSUPPORTED_CAPABILITY', 'Hairer wrapper has timing mode only')
    if request.get('solver_config') not in (None, {}):
        return _blank('UNSUPPORTED_CAPABILITY', 'Hairer wrapper has no solver_config overrides')
    if case.get('mass_matrix') is not None:
        return _blank('UNSUPPORTED_CAPABILITY', 'Hairer wrapper supports explicit-mass ODE only')
    if len(case['initial_state']) > 512:
        return _blank('UNSUPPORTED_CAPABILITY', 'dimension exceeds dense Jacobian memory policy')
    try:
        entry, dense, library_path = _library(solver_id)
        warmup, warmup_seconds = _one_run(case, solver_id, entry, dense,
                                         check_dense_endpoints=True)
        if warmup['status'] != 'OK':
            warmup['reason'] = 'warmup: ' + warmup.get('reason', 'unknown failure')
            return warmup
        result, timed_seconds = _one_run(case, solver_id, entry, dense)
        result['timing'] = {'warmup_seconds': max(warmup_seconds, sys.float_info.min),
                            'timed_seconds': max(timed_seconds, sys.float_info.min),
                            'timing_authority': 'diagnostic_only'}
        result.update({'measurement_mode': 'timing',
                       'timing_scope': 'warmed_solver_setup_integration_output_certificate',
                       'solver_id': solver_id, 'library_path': library_path,
                       'wrapper_lane': 'Python ctypes callbacks wrapping Hairer Fortran',
                       'derivative_mode': 'analytic_model_jacobian',
                       'dense_endpoint_check': warmup['dense_endpoint_check'],
                       'warmup_fortran_diagnostics': warmup['fortran_diagnostics']})
        return result
    except (OSError, FileNotFoundError) as exc:
        return _blank('ENVIRONMENT_FAILURE', str(exc))
    except Exception as exc:
        return _blank('NUMERICAL_FAILURE', f'{type(exc).__name__}: {exc}')


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument('--request', type=Path, required=True)
    parser.add_argument('--probe', action='store_true')
    args = parser.parse_args()
    try:
        request = strict_load(args.request.read_text())
        result = probe(request) if args.probe else solve(request)
    except Exception as exc:
        result = _blank('ENVIRONMENT_FAILURE', f'{type(exc).__name__}: {exc}')
    print(json.dumps(result, allow_nan=False, separators=(',', ':')))


if __name__ == '__main__':
    main()
