"""Problem set for probe A1 (stage-target scaling). Transcribed from
stiff_benchmark.rs / rnext_common/mod.rs / problems.rs / the contract tests (read-only worktree a49f7e4);
hires/robertson/vdp/bruss are the hyp/probs.py transcriptions (copied, unchanged equations).
Each problem is a dict: f(t,y), J(t,y) (dense, for reference/diagnostics only), ft(t,y), y0, span,
ascale (atol = rtol*ascale), auto, exact (callable or None), and the explicit cost model:
  nnzJ  : nonzeros of J used by an analytic sparse JVP  -> F_jvp = 2*nnzJ flops (+2n for the shift W v)
  frhs  : flops of one RHS evaluation (transcendentals counted as 10)
Works with float64 or longdouble state vectors (dtype follows y)."""
import numpy as np


def _arr(lst, like):
    return np.array(lst, dtype=like.dtype)


def hires():
    def f(t, y):
        q = 280.0*y[5]*y[7]
        return _arr([-1.71*y[0] + 0.43*y[1] + 8.32*y[2] + 0.0007, 1.71*y[0] - 8.75*y[1],
                     -10.03*y[2] + 0.43*y[3] + 0.035*y[4], 8.32*y[1] + 1.71*y[2] - 1.12*y[3],
                     -1.745*y[4] + 0.43*y[5] + 0.43*y[6],
                     -q + 0.69*y[3] + 1.71*y[4] - 0.43*y[5] + 0.69*y[6], q - 1.81*y[6], -q + 1.81*y[6]], y)
    def J(t, y):
        j = np.zeros((8, 8), dtype=y.dtype)
        j[0, :3] = [-1.71, 0.43, 8.32]; j[1, :2] = [1.71, -8.75]
        j[2, 2:5] = [-10.03, 0.43, 0.035]; j[3, 1:4] = [8.32, 1.71, -1.12]
        j[4, 4:7] = [-1.745, 0.43, 0.43]
        j[5, 3:8] = [0.69, 1.71, -280*y[7] - 0.43, 0.69, -280*y[5]]
        j[6, 5:8] = [280*y[7], -1.81, 280*y[5]]; j[7, 5:8] = [-280*y[7], 1.81, -280*y[5]]
        return j
    y0 = np.zeros(8); y0[0] = 1.0; y0[7] = 0.0057
    return dict(name='hires', f=f, J=J, ft=None, y0=y0, span=(0.0, 321.8122), ascale=1e-4, auto=True,
                exact=None, nnzJ=25, frhs=45)


def robertson():
    f = lambda t, y: _arr([-0.04*y[0] + 1e4*y[1]*y[2], 0.04*y[0] - 1e4*y[1]*y[2] - 3e7*y[1]**2, 3e7*y[1]**2], y)
    J = lambda t, y: _arr([[-0.04, 1e4*y[2], 1e4*y[1]], [0.04, -1e4*y[2] - 6e7*y[1], -1e4*y[1]], [0, 6e7*y[1], 0]], y)
    return dict(name='robertson', f=f, J=J, ft=None, y0=np.array([1.0, 0, 0]), span=(0.0, 40.0), ascale=1e-4,
                auto=True, exact=None, nnzJ=7, frhs=16)


def vdp(mu=1000.0):
    f = lambda t, y: _arr([y[1], mu*(1 - y[0]**2)*y[1] - y[0]], y)
    J = lambda t, y: _arr([[0.0, 1.0], [-2*mu*y[0]*y[1] - 1, mu*(1 - y[0]**2)]], y)
    return dict(name='vdp', f=f, J=J, ft=None, y0=np.array([2.0, 0.0]), span=(0.0, 2000.0), ascale=1.0,
                auto=True, exact=None, nnzJ=3, frhs=7)


def bruss(cells=50):
    cc = (cells + 1.0)**2/50.0; n = 2*cells
    def f(t, y):
        u, v = y[0::2], y[1::2]
        one = np.ones(1, dtype=y.dtype)
        ul = np.concatenate([one, u[:-1]]); ur = np.concatenate([u[1:], one])
        vl = np.concatenate([3*one, v[:-1]]); vr = np.concatenate([v[1:], 3*one])
        out = np.empty(n, dtype=y.dtype)
        out[0::2] = 1 + u*u*v - 4*u + cc*(ul - 2*u + ur)
        out[1::2] = 3*u - u*u*v + cc*(vl - 2*v + vr)
        return out
    def J(t, y):
        j = np.zeros((n, n), dtype=y.dtype)
        for i in range(cells):
            u, v = y[2*i], y[2*i+1]; a, b = 2*i, 2*i+1
            j[a, a] = 2*u*v - 4 - 2*cc; j[a, b] = u*u; j[b, a] = 3 - 2*u*v; j[b, b] = -u*u - 2*cc
            if i > 0: j[a, a-2] = cc; j[b, b-2] = cc
            if i + 1 < cells: j[a, a+2] = cc; j[b, b+2] = cc
        return j
    x = (np.arange(cells) + 1.0)/(cells + 1.0)
    y0 = np.empty(n); y0[0::2] = 1 + np.sin(2*np.pi*x); y0[1::2] = 3.0
    return dict(name=f'bruss{cells}', f=f, J=J, ft=None, y0=y0, span=(0.0, 10.0), ascale=1.0, auto=True,
                exact=None, nnzJ=4*n - 4, frhs=10*n)


def pr_forced(lam=-1.0e4, mu=0.0):
    """prothero_robinson_problem(-1e4, 0, 0) on [0, 2], atol_scale 1 (rnext_common cases)."""
    def f(t, y):
        d = y[0] - np.sin(t)
        return _arr([lam*d + np.cos(t) + mu*d*d], y)
    J = lambda t, y: _arr([[lam + 2*mu*(y[0] - np.sin(t))]], y)
    def ft(t, y):
        d = y[0] - np.sin(t)
        return _arr([-lam*np.cos(t) - np.sin(t) - 2*mu*d*np.cos(t)], y)
    return dict(name='pr', f=f, J=J, ft=ft, y0=np.array([0.0]), span=(0.0, 2.0), ascale=1.0, auto=False,
                exact=lambda t: np.array([np.sin(t)]), nnzJ=1, frhs=25)


def quad4():
    """quadratic-4 (rnext_common quadratic()): y_i' = a_i y_i + q_i y_i^2 on [0, 0.5], exact solution."""
    n = 4
    a = -1.0 - np.arange(n); q = -0.05*(1 + np.arange(n) % 3); z0 = 1.0 + 0.1*np.arange(n)
    f = lambda t, y: a*y + q*y*y
    J = lambda t, y: np.diag(a + 2*q*y)
    def exact(t):
        e = np.exp(a*t)
        return a*z0*e/(a - q*z0*(e - 1.0))
    # Rust JVP is a dense row loop over the 4x4 A plus the diagonal term: 2*16 + 3*4 flops
    return dict(name='quad4', f=f, J=J, ft=None, y0=z0.copy(), span=(0.0, 0.5), ascale=1.0, auto=True,
                exact=exact, nnzJ=22, frhs=4*n)


def diag_pr(n=128, lam_max=1.0e6):
    """inner_forcing_fixed_step_ladder_contracts.rs diagonal Prothero-Robinson (E-04 p1pr, Q = I)."""
    k = np.arange(n)
    lam = -np.exp(np.log(lam_max)*k/(n - 1))
    phase = 2*np.pi*((k*0.618033988749895) % 1.0)
    f = lambda t, y: lam*(y - np.sin(t + phase)) + np.cos(t + phase)
    J = lambda t, y: np.diag(lam)
    ft = lambda t, y: -lam*np.cos(t + phase) - np.sin(t + phase)
    exact = lambda t: np.sin(t + phase)
    return dict(name=f'diagpr{n}', f=f, J=J, ft=ft, y0=exact(0.0), span=(0.0, 1.0), ascale=1e-2, auto=False,
                exact=exact, nnzJ=n, frhs=25*n)


def semilin(n=64, D=0.05, a=0.5, r=-1.0, nl=0.5):
    """semilinear_advection_diffusion_problem(n, D, a, r, nl, 0) (problems.rs:700): exact phi = e^-t sin(pi i dx)."""
    dx = 1.0/(n + 1)
    Aop = (np.diag(np.full(n, -2*D/dx**2 + r - a/dx)) + np.diag(np.full(n - 1, D/dx**2 + a/dx), -1)
           + np.diag(np.full(n - 1, D/dx**2), 1))
    shape = np.sin(np.pi*np.arange(1, n + 1)*dx)
    phi = lambda t: np.exp(-t)*shape
    def f(t, y):
        d = y - phi(t)
        return Aop @ d - phi(t) + nl*d**3
    J = lambda t, y: Aop + np.diag(3*nl*(y - phi(t))**2)
    def ft(t, y):
        d = y - phi(t)
        dphi = -phi(t)
        return -(Aop @ dphi) - dphi - 3*nl*d**2*dphi
    return dict(name=f'semilin{n}', f=f, J=J, ft=ft, y0=phi(0.0), span=(0.0, 1.0), ascale=1e-2, auto=False,
                exact=phi, nnzJ=3*n - 2, frhs=12*n)


PROBLEMS = dict(hires=hires, robertson=robertson, vdp=vdp, bruss50=lambda: bruss(50), pr=pr_forced, quad4=quad4,
                bruss160=lambda: bruss(160))
