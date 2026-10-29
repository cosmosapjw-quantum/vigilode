"""Independent spot checks used in the adversarial review (not a full rerun)."""
import sys, json, decimal
from pathlib import Path
import numpy as np

base = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(base / 'src'))
from spherical_mesh import EntropyGrid, power_bregman
from fisher_demo import build_problem
from remap_budget_demo import hellinger, poisson_divergence

rng = np.random.default_rng(1731)
checks = {}
f = np.exp(rng.normal(size=(8, 16))*2)
g = EntropyGrid(f)
root = g.root
children = g.children(root)
meanloss = sum(c.area*(-c.mean**.75) for c in children)-root.area*(-root.mean**.75)
checks['jensen_telescope_abs'] = abs(root.entropy-sum(c.entropy for c in children)-meanloss)
res = g.evaluate(children, {'independent_random_kernel': rng.normal(size=f.shape)})
checks['random_observable'] = res['observables']['independent_random_kernel']
D = decimal.Decimal
decimal.getcontext().prec = 65
rows = []
for eps in ['1e-8', '-1e-8', '1e-12', '-1e-12']:
    x = float(1+float(eps))
    xd, a = D.from_float(x), D(3)/D(4)
    expected = D(1)+a*(xd-1)-xd**a
    got = float(power_bregman(np.array([x]), 1.)[0])
    rows.append({'input': x, 'relative_error': float(abs(D.from_float(got)-expected)/expected)})
checks['stable_scalar_decimal'] = rows
p = build_problem(L=3, z=8., anisotropy=.6)
n = len(p['H'])
P = np.zeros((n, n))
for i, b in enumerate(p['blocks']):
    P[2*i:2*i+2, 2*i:2*i+2] = b
v, Q = np.linalg.eigh(P)
sqrtP = (Q*np.sqrt(v))@Q.T
vals = np.linalg.eigvalsh(sqrtP@p['H']@sqrtP)
checks['independent_fisher_eigen_interval'] = [float(vals[0]), float(vals[-1])]
checks['fisher_weight_interval'] = [p['lower'], p['upper']]
checks['fisher_spectrum_vs_centered_score_max_abs'] = float(np.max(np.abs(vals-np.linalg.eigvalsh(p['B']))))
p0, q0 = np.array([0., .2, 2., 3.]), np.array([1., .3, 1., 2.])
checks['hellinger_squared'] = hellinger(p0, q0)**2
checks['poisson_kl'] = poisson_divergence(p0, q0)
checks['passed'] = (checks['jensen_telescope_abs'] < 1e-12
    and checks['random_observable']['certified']
    and max(x['relative_error'] for x in rows) < 1e-12
    and checks['fisher_spectrum_vs_centered_score_max_abs'] < 1e-8
    and checks['hellinger_squared'] <= checks['poisson_kl'])
assert checks['passed']
(base / 'results/independent_numeric_spotchecks.json').write_text(json.dumps(checks, indent=2)+'\n')
print(json.dumps(checks, indent=2))
