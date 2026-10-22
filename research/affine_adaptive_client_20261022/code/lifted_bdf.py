"""Opt-in SciPy 1.17.0 BDF adaptation; see vendor/SCIPY_LICENSE.txt.
Newton and step algorithms derive from vendor/scipy_bdf_1_17_0.py.
Only scale/norm routing, counters and transaction protection are changed.
"""
from pathlib import Path
import hashlib, json, ctypes
from collections import Counter
import numpy as np
import scipy
import scipy.integrate._ivp.bdf as _bdf
from scipy.integrate._ivp.bdf import BDF, change_D, NEWTON_MAXITER, MAX_ORDER, MIN_FACTOR, MAX_FACTOR

class NormFailure(RuntimeError): pass

class CNorm:
    def __init__(self, library):
        self.library = ctypes.CDLL(str(Path(library).resolve()))
        self.fn = self.library.bassrei_full33_error_norm
        ptr = ctypes.POINTER(ctypes.c_double)
        self.fn.argtypes = [ptr, ptr, ptr]
        self.fn.restype = ctypes.c_int
        self.calls = Counter()
        self.statuses = Counter()
    def __call__(self, error, scale, phase):
        e = np.ascontiguousarray(error, dtype=np.float64)
        s = np.ascontiguousarray(scale, dtype=np.float64)
        if e.shape != (30,) or s.shape != (33,):
            raise NormFailure('wrong C norm input shape')
        result = ctypes.c_double(-987.0)
        ptr = ctypes.POINTER(ctypes.c_double)
        status = self.fn(e.ctypes.data_as(ptr), s.ctypes.data_as(ptr), ctypes.byref(result))
        self.calls[phase] += 1
        self.statuses[str(status)] += 1
        if status != 0 or not np.isfinite(result.value) or result.value < 0:
            raise NormFailure(f'C norm failure status={status}, phase={phase}')
        return result.value

def solve_bdf_system(fun, t_new, y_predict, c, psi, LU, solve_lu, scale, tol, lifted_norm):
    """Solve the algebraic system resulting from BDF method."""
    d = 0
    y = y_predict.copy()
    dy_norm_old = None
    converged = False
    for k in range(NEWTON_MAXITER):
        f = fun(t_new, y)
        if not np.all(np.isfinite(f)):
            break

        dy = solve_lu(LU, c * f - psi - d)
        dy_norm = lifted_norm(dy, scale, "newton")

        if dy_norm_old is None:
            rate = None
        else:
            rate = dy_norm / dy_norm_old

        if (rate is not None and (rate >= 1 or
                rate ** (NEWTON_MAXITER - k) / (1 - rate) * dy_norm > tol)):
            break

        y += dy
        d += dy

        if (dy_norm == 0 or
                rate is not None and rate / (1 - rate) * dy_norm < tol):
            converged = True
            break

        dy_norm_old = dy_norm

    return converged, k + 1, y, d

class LiftedBDF(BDF):
    def __init__(self, fun, t0, y0, t_bound, *, affine, full_atol, lifted_norm, first_step, **kwargs):
        manifest=json.loads((Path(__file__).resolve().parents[1]/'SOURCE_MANIFEST.json').read_text())
        expected=manifest['files']['vendor/scipy_bdf_1_17_0.py']
        if scipy.__version__ != '1.17.0' or hashlib.sha256(Path(_bdf.__file__).read_bytes()).hexdigest()!=expected:
            raise RuntimeError('unsupported SciPy version/source; pinned 1.17.0 required')
        if first_step is None: raise ValueError('explicit full33-selected first_step required')
        self.affine=affine
        self.full_atol=np.asarray(full_atol,dtype=float).copy()
        if self.full_atol.shape!=(33,) or not np.isfinite(self.full_atol).all() or np.any(self.full_atol<=0):
            raise ValueError('finite positive full33 atol required')
        self.lifted_norm=lifted_norm
        self.accepted_steps=0
        self.lte_rejections=0
        self.newton_rejections=0
        super().__init__(fun,t0,y0,t_bound,first_step=first_step,**kwargs)
        if self.n != 30: raise ValueError('affine30 required')
    def _step_impl(self):
        # Restore external accepted state also if order adaptation fails after
        # upstream temporary updates; history D is always mutated privately.
        keys=('t','y','D','h_abs','J','LU','order','n_equal_steps')
        saved={k:getattr(self,k) for k in keys}
        try:
            result=self._step_impl_pinned()
        except NormFailure:
            for k,value in saved.items(): setattr(self,k,value)
            raise
        if result[0]: self.accepted_steps+=1
        return result
    def _step_impl_pinned(self):
        t = self.t
        D = self.D.copy()

        max_step = self.max_step
        min_step = 10 * np.abs(np.nextafter(t, self.direction * np.inf) - t)
        if self.h_abs > max_step:
            h_abs = max_step
            change_D(D, self.order, max_step / self.h_abs)
            self.n_equal_steps = 0
        elif self.h_abs < min_step:
            h_abs = min_step
            change_D(D, self.order, min_step / self.h_abs)
            self.n_equal_steps = 0
        else:
            h_abs = self.h_abs

        atol = self.atol
        rtol = self.rtol
        order = self.order

        alpha = self.alpha
        gamma = self.gamma
        error_const = self.error_const

        J = self.J
        LU = self.LU
        current_jac = self.jac is None

        step_accepted = False
        while not step_accepted:
            if h_abs < min_step:
                return False, self.TOO_SMALL_STEP

            h = h_abs * self.direction
            t_new = t + h

            if self.direction * (t_new - self.t_bound) > 0:
                t_new = self.t_bound
                change_D(D, order, np.abs(t_new - t) / h_abs)
                self.n_equal_steps = 0
                LU = None

            h = t_new - t
            h_abs = np.abs(h)

            y_predict = np.sum(D[:order + 1], axis=0)

            scale = self.full_atol + rtol * np.abs(self.affine.lift(y_predict))
            psi = np.dot(D[1: order + 1].T, gamma[1: order + 1]) / alpha[order]

            converged = False
            c = h / alpha[order]
            while not converged:
                if LU is None:
                    LU = self.lu(self.I - c * J)

                converged, n_iter, y_new, d = solve_bdf_system(
                    self.fun, t_new, y_predict, c, psi, LU, self.solve_lu,
                    scale, self.newton_tol, self.lifted_norm)

                if not converged:
                    if current_jac:
                        break
                    J = self.jac(t_new, y_predict)
                    LU = None
                    current_jac = True

            if not converged:
                self.newton_rejections += 1
                factor = 0.5
                h_abs *= factor
                change_D(D, order, factor)
                self.n_equal_steps = 0
                LU = None
                continue

            safety = 0.9 * (2 * NEWTON_MAXITER + 1) / (2 * NEWTON_MAXITER
                                                       + n_iter)

            scale = self.full_atol + rtol * np.abs(self.affine.lift(y_new))
            error = error_const[order] * d
            error_norm = self.lifted_norm(error, scale, "lte")

            if error_norm > 1:
                self.lte_rejections += 1
                factor = max(MIN_FACTOR,
                             safety * error_norm ** (-1 / (order + 1)))
                h_abs *= factor
                change_D(D, order, factor)
                self.n_equal_steps = 0
                # As we didn't have problems with convergence, we don't
                # reset LU here.
            else:
                step_accepted = True

        self.n_equal_steps += 1

        self.t = t_new
        self.y = y_new

        self.h_abs = h_abs
        self.J = J
        self.LU = LU

        # Update differences. The principal relation here is
        # D^{j + 1} y_n = D^{j} y_n - D^{j} y_{n - 1}. Keep in mind that D
        # contained difference for previous interpolating polynomial and
        # d = D^{k + 1} y_n. Thus this elegant code follows.
        D[order + 2] = d - D[order + 1]
        D[order + 1] = d
        for i in reversed(range(order + 1)):
            D[i] += D[i + 1]

        if self.n_equal_steps < order + 1:
            self.D = D
            return True, None

        if order > 1:
            error_m = error_const[order - 1] * D[order]
            error_m_norm = self.lifted_norm(error_m, scale, "order_minus")
        else:
            error_m_norm = np.inf

        if order < MAX_ORDER:
            error_p = error_const[order + 1] * D[order + 2]
            error_p_norm = self.lifted_norm(error_p, scale, "order_plus")
        else:
            error_p_norm = np.inf

        error_norms = np.array([error_m_norm, error_norm, error_p_norm])
        with np.errstate(divide='ignore'):
            factors = error_norms ** (-1 / np.arange(order, order + 3))

        delta_order = np.argmax(factors) - 1
        order += delta_order
        self.order = order

        factor = min(MAX_FACTOR, safety * np.max(factors))
        self.h_abs *= factor
        change_D(D, order, factor)
        self.n_equal_steps = 0
        self.LU = None

        self.D = D
        return True, None

def counted_native_system(fun, t_new, y_predict, c, psi, LU, solve_lu, scale, tol, native_norm):
    """Solve the algebraic system resulting from BDF method."""
    d = 0
    y = y_predict.copy()
    dy_norm_old = None
    converged = False
    for k in range(NEWTON_MAXITER):
        f = fun(t_new, y)
        if not np.all(np.isfinite(f)):
            break

        dy = solve_lu(LU, c * f - psi - d)
        dy_norm = native_norm(dy / scale, "newton")

        if dy_norm_old is None:
            rate = None
        else:
            rate = dy_norm / dy_norm_old

        if (rate is not None and (rate >= 1 or
                rate ** (NEWTON_MAXITER - k) / (1 - rate) * dy_norm > tol)):
            break

        y += dy
        d += dy

        if (dy_norm == 0 or
                rate is not None and rate / (1 - rate) * dy_norm < tol):
            converged = True
            break

        dy_norm_old = dy_norm

    return converged, k + 1, y, d

class CountedBDF(BDF):
    """Ordinary owner BDF decisions; additive counters only."""
    def __init__(self,*args,**kwargs):
        self.accepted_steps=0;self.lte_rejections=0;self.newton_rejections=0;self.norm_calls=Counter()
        super().__init__(*args,**kwargs)
    def native_norm(self,error,phase):
        self.norm_calls[phase]+=1
        return _bdf.norm(error)
    def _step_impl(self):
        result=self._step_impl_native()
        if result[0]:self.accepted_steps+=1
        return result
    def _step_impl_native(self):
        t = self.t
        D = self.D

        max_step = self.max_step
        min_step = 10 * np.abs(np.nextafter(t, self.direction * np.inf) - t)
        if self.h_abs > max_step:
            h_abs = max_step
            change_D(D, self.order, max_step / self.h_abs)
            self.n_equal_steps = 0
        elif self.h_abs < min_step:
            h_abs = min_step
            change_D(D, self.order, min_step / self.h_abs)
            self.n_equal_steps = 0
        else:
            h_abs = self.h_abs

        atol = self.atol
        rtol = self.rtol
        order = self.order

        alpha = self.alpha
        gamma = self.gamma
        error_const = self.error_const

        J = self.J
        LU = self.LU
        current_jac = self.jac is None

        step_accepted = False
        while not step_accepted:
            if h_abs < min_step:
                return False, self.TOO_SMALL_STEP

            h = h_abs * self.direction
            t_new = t + h

            if self.direction * (t_new - self.t_bound) > 0:
                t_new = self.t_bound
                change_D(D, order, np.abs(t_new - t) / h_abs)
                self.n_equal_steps = 0
                LU = None

            h = t_new - t
            h_abs = np.abs(h)

            y_predict = np.sum(D[:order + 1], axis=0)

            scale = atol + rtol * np.abs(y_predict)
            psi = np.dot(D[1: order + 1].T, gamma[1: order + 1]) / alpha[order]

            converged = False
            c = h / alpha[order]
            while not converged:
                if LU is None:
                    LU = self.lu(self.I - c * J)

                converged, n_iter, y_new, d = counted_native_system(
                    self.fun, t_new, y_predict, c, psi, LU, self.solve_lu,
                    scale, self.newton_tol, self.native_norm)

                if not converged:
                    if current_jac:
                        break
                    J = self.jac(t_new, y_predict)
                    LU = None
                    current_jac = True

            if not converged:
                self.newton_rejections += 1
                factor = 0.5
                h_abs *= factor
                change_D(D, order, factor)
                self.n_equal_steps = 0
                LU = None
                continue

            safety = 0.9 * (2 * NEWTON_MAXITER + 1) / (2 * NEWTON_MAXITER
                                                       + n_iter)

            scale = atol + rtol * np.abs(y_new)
            error = error_const[order] * d
            error_norm = self.native_norm(error / scale, "lte")

            if error_norm > 1:
                self.lte_rejections += 1
                factor = max(MIN_FACTOR,
                             safety * error_norm ** (-1 / (order + 1)))
                h_abs *= factor
                change_D(D, order, factor)
                self.n_equal_steps = 0
                # As we didn't have problems with convergence, we don't
                # reset LU here.
            else:
                step_accepted = True

        self.n_equal_steps += 1

        self.t = t_new
        self.y = y_new

        self.h_abs = h_abs
        self.J = J
        self.LU = LU

        # Update differences. The principal relation here is
        # D^{j + 1} y_n = D^{j} y_n - D^{j} y_{n - 1}. Keep in mind that D
        # contained difference for previous interpolating polynomial and
        # d = D^{k + 1} y_n. Thus this elegant code follows.
        D[order + 2] = d - D[order + 1]
        D[order + 1] = d
        for i in reversed(range(order + 1)):
            D[i] += D[i + 1]

        if self.n_equal_steps < order + 1:
            return True, None

        if order > 1:
            error_m = error_const[order - 1] * D[order]
            error_m_norm = self.native_norm(error_m / scale, "order_minus")
        else:
            error_m_norm = np.inf

        if order < MAX_ORDER:
            error_p = error_const[order + 1] * D[order + 2]
            error_p_norm = self.native_norm(error_p / scale, "order_plus")
        else:
            error_p_norm = np.inf

        error_norms = np.array([error_m_norm, error_norm, error_p_norm])
        with np.errstate(divide='ignore'):
            factors = error_norms ** (-1 / np.arange(order, order + 3))

        delta_order = np.argmax(factors) - 1
        order += delta_order
        self.order = order

        factor = min(MAX_FACTOR, safety * np.max(factors))
        self.h_abs *= factor
        change_D(D, order, factor)
        self.n_equal_steps = 0
        self.LU = None

        return True, None
