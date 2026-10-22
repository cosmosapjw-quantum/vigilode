import json, sys
import numpy as np
from scipy.integrate import ode, solve_ivp

C, S = sys.argv[1], sys.argv[2]
ref = np.load(S + '/ref_mp.npy')
case = json.load(open(C + '/requests/HEAT_BASELINE_REQUEST.json'))['case']
y0 = np.array(case['initial_state'])
n = len(y0)
D = case['parameters']['diffusion'] * n * n
T = case['output_times']
J = np.zeros((n, n))
for i in range(n):
    J[i, i] -= 2 * D
    J[i, (i - 1) % n] += D
    J[i, (i + 1) % n] += D
den = 1e-9 + 1e-6 * np.abs(ref)
big = np.abs(ref) > 1e-3  # exclude near-zero components


def f(t, y):
    return D * (np.roll(y, 1) - 2 * y + np.roll(y, -1))


def run(mf, rtol, atol):
    r = ode(f, (lambda t, y: J) if mf == 21 else None)
    r.set_integrator('vode', method='bdf', with_jacobian=(mf in (21, 22)), rtol=rtol, atol=atol, nsteps=100000)
    r.set_initial_value(y0, 0.0)
    st = [y0.copy()]
    for t in T[1:]:
        st.append(r.integrate(t).copy())
        assert r.successful() and r.t == t
    iw = r._integrator.iwork
    return np.array(st), int(iw[10]), int(r._integrator.call_args[-1])


def metr(y):
    z = np.abs(y - ref) / den
    return float(z.max()), float(z[big].max()), float(np.abs(y - ref).max())


for mf in (20, 21, 22):
    raw = json.load(open(f'{C}/raw/B_VODE_MF{mf}/B-VODE_MF{mf}-c4{mf-20}-b00-X.json'))
    yr = np.array(raw['measurements'][0]['states'])
    y, nst, mfu = run(mf, 1e-6, 1e-9)
    yv, nstv, _ = run(mf, 1e-6, np.full(n, 1e-9))
    print(f'MF{mf} used {mfu}: local scalar-atol nst={nst} metric={metr(y)}; vector-atol nst={nstv} metric={metr(yv)};'
          f' raw metric={metr(yr)} raw-vs-local maxabs={np.abs(y-yr).max():.3e} bitwise={np.array_equal(y, yr)}')
    for rt in (1e-7, 1e-8, 1e-10):
        yy, ns, _ = run(mf, rt, rt * 1e-3)
        print(f'   rtol={rt:g}: nst={ns} metric(vs app budget)={metr(yy)}')
# per-time profile for MF21 to see whether error grows / is real
y21, _, _ = run(21, 1e-6, 1e-9)
z = np.abs(y21 - ref) / den
print('MF21 per-time max scaled', np.round(z.max(axis=1), 3).tolist())
y22, _, _ = run(22, 1e-6, 1e-9)
z = np.abs(y22 - ref) / den
print('MF22 per-time max scaled', np.round(z.max(axis=1), 3).tolist())
print('MF22 per-time max scaled |ref|>1e-3', np.round(np.where(big, z, 0).max(axis=1), 3).tolist())
# is MF22 error a broken-mode (non-sine) pattern? project error onto the sine mode
e = y22[8] - ref[8]
mode = ref[8] / np.linalg.norm(ref[8])
print('MF22 t idx 8: |err|', np.linalg.norm(e), 'mode-parallel', abs(e @ mode), 'orthogonal', np.linalg.norm(e - (e @ mode) * mode))
# Independent comparator using a different implementation (scipy BDF / Radau) at the same tolerances
for meth in ('BDF', 'Radau', 'LSODA'):
    sol = solve_ivp(f, (0, 1), y0, method=meth, t_eval=T, rtol=1e-6, atol=1e-9, jac=J)
    print(meth, 'nfev', sol.nfev, 'metric', metr(sol.y.T))
