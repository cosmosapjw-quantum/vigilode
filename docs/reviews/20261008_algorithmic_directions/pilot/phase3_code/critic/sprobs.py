"""Critic stress problems for the recommended matrix-free stack (EXPLORATORY; not ledger authority).
Same dict interface as probe/stack/tprobs.py: f, J (dense), ft, y0, span, ascale, auto, exact, nnzJ, frhs,
plus 'ref' (endpoint reference when no exact solution) and 'kind' (error-metric family).
Cost model as tprobs: F_jvp = 2 nnzJ + 2n, frhs counted with transcendentals = 10 flops."""
import json, math, os
import numpy as np
import scipy.linalg as sl

HERE = os.path.dirname(os.path.abspath(__file__))
REFDIR = os.path.join(HERE, 'refs')


def _phi_slow(n, c=1.0, off=0.0):
    p = 0.17*np.arange(n)
    phi = lambda t: np.sin(c*t + p) + off
    dphi = lambda t: c*np.cos(c*t + p)
    ddphi = lambda t: -c*c*np.sin(c*t + p)
    return phi, dphi, ddphi


def _linear_forced(A, phi, dphi, ddphi, y0, span, name, ascale, exact_end, nnzJ, frhs, **extra):
    """y' = A (y - phi) + phi'  (exact solution phi + e^{tA}(y0 - phi0))."""
    f = lambda t, y: A @ (y - phi(t)) + dphi(t)
    J = lambda t, y: A
    ft = lambda t, y: -(A @ dphi(t)) + ddphi(t)
    d = dict(name=name, f=f, J=J, ft=ft, y0=y0, span=span, ascale=ascale, auto=False, exact=exact_end,
             nnzJ=nnzJ, frhs=frhs, kind='synthetic')
    d.update(extra)
    return d


def stosc(omega=1e3, sigma=50.0, nb=64, excite=0.0, T=2.0):
    """Stiff-oscillatory forced blocks [[-s, -w_k], [w_k, -s]], w_k = omega (0.1 + 0.9 k/(nb-1)); smooth forcing.
    excite > 0 starts off the slow manifold (a damped carrier that must be resolved)."""
    n = 2*nb
    A = np.zeros((n, n)); wk = omega*(0.1 + 0.9*np.arange(nb)/(nb - 1))
    for k in range(nb):
        a, b = 2*k, 2*k + 1
        A[a, a] = -sigma; A[a, b] = -wk[k]; A[b, a] = wk[k]; A[b, b] = -sigma
    phi, dphi, ddphi = _phi_slow(n)
    e = excite*np.ones(n); y0 = phi(0.0) + e

    def exact(t):
        out = phi(t).copy()
        for k in range(nb):
            a, b = 2*k, 2*k + 1
            c, s = math.cos(wk[k]*t), math.sin(wk[k]*t); dmp = math.exp(-sigma*t)
            out[a] += dmp*(c*e[a] - s*e[b]); out[b] += dmp*(s*e[a] + c*e[b])
        return out
    return _linear_forced(A, phi, dphi, ddphi, y0, (0.0, T), f'stosc-w{omega:g}-s{sigma:g}-x{excite:g}', 1.0, exact,
                          nnzJ=2*n, frhs=4*n + 20*n)


def oscpr(omega=1e3, n=64, lam_lo=1e2, lam_hi=1e6, periods=8.0):
    """Vector forced-oscillator Prothero-Robinson (L-0079-type carrier): y_i' = lam_i (y_i - g_i) + g_i',
    g_i = sin(omega t + p_i), lam_i log-spaced in [-lam_hi, -lam_lo]; exact g. Span = 2 pi periods/omega."""
    k = np.arange(n)
    lam = -np.exp(np.log(lam_lo) + (np.log(lam_hi) - np.log(lam_lo))*k/(n - 1))
    p = 2*np.pi*((k*0.618033988749895) % 1.0)
    g = lambda t: np.sin(omega*t + p)
    dg = lambda t: omega*np.cos(omega*t + p)
    f = lambda t, y: lam*(y - g(t)) + dg(t)
    J = lambda t, y: np.diag(lam)
    ft = lambda t, y: -lam*dg(t) - omega*omega*np.sin(omega*t + p)
    T = 2*np.pi*periods/omega
    return dict(name=f'oscpr-w{omega:g}', f=f, J=J, ft=ft, y0=g(0.0), span=(0.0, T), ascale=1.0, auto=False,
                exact=g, nnzJ=n, frhs=25*n, kind='synthetic')


_E05 = {}


def e05(s=1.0, T=2.0, c=1.0, excite=0.1):
    """E-05 operator A = Q(D+N)Q^T (n = 256, same construction/seed as e05_krylov_stress.rs), forced to a smooth
    phi = sin(c t + p) + 0.5 with an initial excitation excite*b (b = E-05's unit RHS); exact endpoint via expm."""
    import e05mat
    if s not in _E05:
        M, b = e05mat.e05_mats(scales=(0.0, 1.0, 10.0, 100.0))
        for kk, v in M.items(): _E05[kk] = (v[0], b)
    A, b = _E05[s]; n = A.shape[0]
    phi, dphi, ddphi = _phi_slow(n, c=c, off=0.5)
    e = excite*b; y0 = phi(0.0) + e
    cache = {}

    def exact(t):
        if t not in cache:
            cache[t] = phi(t) + sl.expm(t*A) @ e
        return cache[t]
    return _linear_forced(A, phi, dphi, ddphi, y0, (0.0, T), f'e05-s{s:g}-T{T:g}', 1e-2, exact,
                          nnzJ=n*n, frhs=2*n*n + 22*n)


def vigb(k=10, nb=32, lam_lo=1.0, lam_hi=1e4, T=2.0):
    """Block-diagonal VIG-A02 family: blocks lam_j [[-2, 2^k], [2^-k, -2]] (eigenvalues -lam_j, -3 lam_j), smooth
    forcing, exact solution phi (start on phi)."""
    n = 2*nb; A = np.zeros((n, n))
    lam = np.exp(np.log(lam_lo) + (np.log(lam_hi) - np.log(lam_lo))*np.arange(nb)/max(nb - 1, 1))
    for j in range(nb):
        a, b = 2*j, 2*j + 1
        A[a, a] = -2*lam[j]; A[a, b] = lam[j]*2.0**k; A[b, a] = lam[j]*2.0**(-k); A[b, b] = -2*lam[j]
    phi, dphi, ddphi = _phi_slow(n, off=0.5)
    return _linear_forced(A, phi, dphi, ddphi, phi(0.0), (0.0, T), f'vigb-k{k}-nb{nb}', 1e-2, lambda t: phi(t),
                          nnzJ=2*n, frhs=4*n + 22*n)


def heatsw(n=128, D=1.0, kappa=1.0, T=1.5, switches=(0.137, 0.4142, 0.7183, 1.1, 1.3)):
    """1-D heat + decay with an undeclared on/off source s(t) q(x) (jumps at the switch times); exact solution
    from the eigen-decomposition, piecewise in time. y0 = 0."""
    dx = 1.0/(n + 1); x = np.arange(1, n + 1)*dx
    L = (np.diag(np.full(n, -2.0)) + np.diag(np.ones(n - 1), 1) + np.diag(np.ones(n - 1), -1))*D/dx**2 - kappa*np.eye(n)
    q = np.sin(np.pi*x) + (x > 0.5)
    sw = np.array(switches)
    s_of = lambda t: 1.0 if (np.searchsorted(sw, t, side='right') % 2 == 0) else 0.0
    f = lambda t, y: L @ y + s_of(t)*q
    Jm = lambda t, y: L
    ft = lambda t, y: np.zeros(n)
    lamv, V = np.linalg.eigh(L)

    def exact(t):
        cuts = [0.0] + [c for c in sw if c < t] + [t]
        z = np.zeros(n); qz = V.T @ q
        for a, b in zip(cuts[:-1], cuts[1:]):
            dt = b - a; sval = s_of(0.5*(a + b))
            ex = np.exp(lamv*dt)
            z = ex*z + sval*(ex - 1.0)/lamv*qz
        return V @ z
    return dict(name=f'heatsw-{n}', f=f, J=Jm, ft=ft, y0=np.zeros(n), span=(0.0, T), ascale=1e-2, auto=False,
                exact=exact, nnzJ=3*n - 2, frhs=2*(3*n - 2) + 2*n, kind='synthetic')


def brusw(cells=50, amp=0.5, switches=(1.37, 2.71, 4.14, 6.28, 8.0)):
    """Bruss-1D-50 (harness equations) plus an undeclared on/off source amp*sin(pi x) in the u equation."""
    import tprobs
    p = tprobs.bruss(cells)
    x = (np.arange(cells) + 1.0)/(cells + 1.0)
    q = np.zeros(2*cells); q[0::2] = amp*np.sin(np.pi*x)
    sw = np.array(switches)
    s_of = lambda t: 1.0 if (np.searchsorted(sw, t, side='right') % 2 == 0) else 0.0
    f0 = p['f']
    p = dict(p); p['f'] = lambda t, y: f0(t, y) + s_of(t)*q
    p['name'] = f'brusw{cells}'; p['auto'] = False; p['ft'] = None; p['kind'] = 'bench'
    p['switches'] = list(switches)
    return p


def long_robertson(T=4e10):
    import tprobs
    p = dict(tprobs.robertson()); p['span'] = (0.0, T); p['name'] = 'robL'; p['kind'] = 'bench'
    return p


def long_vdp(T=2e4):
    import tprobs
    p = dict(tprobs.vdp()); p['span'] = (0.0, T); p['name'] = 'vdpL'; p['kind'] = 'bench'
    return p


def long_bruss(T=100.0, cells=50):
    import tprobs
    p = dict(tprobs.bruss(cells)); p['span'] = (0.0, T); p['name'] = f'brussL{cells}'; p['kind'] = 'bench'
    return p


def bench(name):
    import tprobs
    p = dict(tprobs.PROBLEMS[name]()); p['kind'] = 'bench'
    return p


PROBLEMS = {
    # stiff-oscillatory, on the slow manifold (stage-solve stress: complex spectrum, restarted GMRES)
    'stosc1e2': lambda: stosc(1e2, 50.0), 'stosc1e3': lambda: stosc(1e3, 50.0), 'stosc1e4': lambda: stosc(1e4, 50.0),
    # stiff-oscillatory with an excited damped carrier (resolution regime)
    'stoscx1e2': lambda: stosc(1e2, 200.0, excite=0.5, T=0.5), 'stoscx1e3': lambda: stosc(1e3, 200.0, excite=0.5, T=0.5),
    # forced oscillator (carrier resolved by steps), omega 1e2..1e4
    'oscpr1e2': lambda: oscpr(1e2), 'oscpr1e3': lambda: oscpr(1e3), 'oscpr1e4': lambda: oscpr(1e4),
    # strongly nonnormal (E-05 operator)
    'e05s0': lambda: e05(0.0), 'e05s1': lambda: e05(1.0), 'e05s10': lambda: e05(10.0, T=0.1, c=20.0),
    # VIG-A02 2x2 family, block diagonal
    'vig1b10': lambda: vigb(10, nb=1, lam_lo=1e3, lam_hi=1e3), 'vig1b20': lambda: vigb(20, nb=1, lam_lo=1e3, lam_hi=1e3),
    'vigb0': lambda: vigb(0), 'vigb10': lambda: vigb(10), 'vigb20': lambda: vigb(20), 'vigb46': lambda: vigb(46),
    # discontinuous forcing
    'heatsw': lambda: heatsw(), 'brusw': lambda: brusw(),
    # long-time
    'robL': lambda: long_robertson(), 'vdpL': lambda: long_vdp(), 'brussL': lambda: long_bruss(),
    # benchmark problems (misdeclared T, seeds, FD)
    'hires': lambda: bench('hires'), 'robertson': lambda: bench('robertson'), 'vdp': lambda: bench('vdp'),
    'bruss50': lambda: bench('bruss50'),
}


def problem(name):
    p = PROBLEMS[name]()
    p['pname'] = name
    return p


def reference(name, p=None):
    """(endpoint reference, uncertainty) -- exact where available, else refs/<name>.json (Radau)."""
    p = p or problem(name)
    if p.get('exact') is not None:
        return np.array(p['exact'](p['span'][1]), dtype=float), 0.0
    fn = os.path.join(REFDIR, f'{name}.json')
    if os.path.exists(fn):
        d = json.load(open(fn)); return np.array(d['y']), d['unc']
    if name in ('hires', 'robertson', 'vdp', 'bruss50'):
        d = json.load(open(os.path.join(HERE, 'spot', f'ref_{name}.json')))
        LD = np.longdouble
        return np.array([float(LD(v)) for v in d['1e-15']['y_ld']]), d['uncertainty_componentwise']
    raise FileNotFoundError(fn)
