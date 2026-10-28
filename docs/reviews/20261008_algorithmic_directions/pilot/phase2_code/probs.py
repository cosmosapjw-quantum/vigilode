import numpy as np
def bruss(cells):
    cc = (cells + 1.0)**2/50.0; n = 2*cells
    def f(t, y):
        u, v = y[0::2], y[1::2]
        ul = np.r_[1.0, u[:-1]]; ur = np.r_[u[1:], 1.0]; vl = np.r_[3.0, v[:-1]]; vr = np.r_[v[1:], 3.0]
        out = np.empty(n)
        out[0::2] = 1 + u*u*v - 4*u + cc*(ul - 2*u + ur)
        out[1::2] = 3*u - u*u*v + cc*(vl - 2*v + vr)
        return out
    def J(t, y):
        j = np.zeros((n, n))
        for i in range(cells):
            u, v = y[2*i], y[2*i+1]; a, b = 2*i, 2*i+1
            j[a, a] = 2*u*v - 4 - 2*cc; j[a, b] = u*u; j[b, a] = 3 - 2*u*v; j[b, b] = -u*u - 2*cc
            if i > 0: j[a, a-2] = cc; j[b, b-2] = cc
            if i + 1 < cells: j[a, a+2] = cc; j[b, b+2] = cc
        return j
    x = (np.arange(cells) + 1.0)/(cells + 1.0)
    y0 = np.empty(n); y0[0::2] = 1 + np.sin(2*np.pi*x); y0[1::2] = 3.0
    return f, J, (lambda t, y: np.zeros(n)), y0, (0.0, 10.0), 1.0
def hires():
    def f(t, y):
        q = 280.0*y[5]*y[7]
        return np.array([-1.71*y[0] + 0.43*y[1] + 8.32*y[2] + 0.0007, 1.71*y[0] - 8.75*y[1],
                         -10.03*y[2] + 0.43*y[3] + 0.035*y[4], 8.32*y[1] + 1.71*y[2] - 1.12*y[3],
                         -1.745*y[4] + 0.43*y[5] + 0.43*y[6],
                         -q + 0.69*y[3] + 1.71*y[4] - 0.43*y[5] + 0.69*y[6], q - 1.81*y[6], -q + 1.81*y[6]])
    def J(t, y):
        j = np.zeros((8, 8))
        j[0, :3] = [-1.71, 0.43, 8.32]; j[1, :2] = [1.71, -8.75]
        j[2, 2:5] = [-10.03, 0.43, 0.035]; j[3, 1:4] = [8.32, 1.71, -1.12]
        j[4, 4:7] = [-1.745, 0.43, 0.43]
        j[5, 3:8] = [0.69, 1.71, -280*y[7] - 0.43, 0.69, -280*y[5]]
        j[6, 5:8] = [280*y[7], -1.81, 280*y[5]]; j[7, 5:8] = [-280*y[7], 1.81, -280*y[5]]
        return j
    y0 = np.zeros(8); y0[0] = 1.0; y0[7] = 0.0057
    return f, J, (lambda t, y: np.zeros(8)), y0, (0.0, 321.8122), 1e-4
def vdp(mu=1000.0):
    f = lambda t, y: np.array([y[1], mu*(1 - y[0]**2)*y[1] - y[0]])
    J = lambda t, y: np.array([[0.0, 1.0], [-2*mu*y[0]*y[1] - 1, mu*(1 - y[0]**2)]])
    return f, J, (lambda t, y: np.zeros(2)), np.array([2.0, 0.0]), (0.0, 2000.0), 1.0
def robertson():
    f = lambda t, y: np.array([-0.04*y[0] + 1e4*y[1]*y[2], 0.04*y[0] - 1e4*y[1]*y[2] - 3e7*y[1]**2, 3e7*y[1]**2])
    J = lambda t, y: np.array([[-0.04, 1e4*y[2], 1e4*y[1]], [0.04, -1e4*y[2] - 6e7*y[1], -1e4*y[1]], [0, 6e7*y[1], 0]])
    return f, J, (lambda t, y: np.zeros(3)), np.array([1.0, 0, 0]), (0.0, 40.0), 1e-4
def heat_pr(n=64, lam_max=1e4, adv=0.0):
    """Forced stiff vector PR: y' = A(y - phi) + phi', A = scaled 1-D Dirichlet Laplacian (+ upwind
    advection if adv>0, nonnormal); exact y = phi. Spectrum of A in [-lam_max, ~-lam_max*pi^2/(4(n+1)^2)]."""
    dx = 1.0/(n + 1); x = np.arange(1, n + 1)*dx
    L = (np.diag(-2*np.ones(n)) + np.diag(np.ones(n-1), 1) + np.diag(np.ones(n-1), -1))
    A = (lam_max/4.0)*L
    if adv:
        A = A + adv*(np.diag(-np.ones(n)) + np.diag(np.ones(n-1), -1))   # upwind, nonnormal
    A = A - 1.0*np.eye(n)
    phi = lambda t: np.sin(np.pi*x)*np.sin(t) + 0.35*np.sin(2*np.pi*x)*np.cos(t/2)
    dphi = lambda t: np.sin(np.pi*x)*np.cos(t) - 0.175*np.sin(2*np.pi*x)*np.sin(t/2)
    d2phi = lambda t: -np.sin(np.pi*x)*np.sin(t) - 0.0875*np.sin(2*np.pi*x)*np.cos(t/2)
    f = lambda t, y: A @ (y - phi(t)) + dphi(t)
    J = lambda t, y: A
    ft = lambda t, y: -A @ dphi(t) + d2phi(t)
    return f, J, ft, phi(0.0), (0.0, 2.0), 1.0, phi
