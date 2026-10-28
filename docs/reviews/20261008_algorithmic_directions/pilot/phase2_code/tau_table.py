import json, numpy as np
d = json.load(open('/home/user/wt-speed/fixtures/rodas5p_coefficients_snapshot.json'))
g = float(d['gamma']); A = np.array([[float(x) for x in r] for r in d['A']])
C = np.array([[float(x) for x in r] for r in d['C']]); bc = np.array([float(x) for x in d['b_code']]); s = 8
def T(z):
    L = (1 - g*z)*np.eye(s) - g*(z*np.tril(A,-1) + np.tril(C,-1)); Li = np.linalg.inv(L)
    return bc @ Li, Li[7, :]
ys = np.concatenate([-np.logspace(-3, 5, 1500)[::-1], [0], np.logspace(-3, 5, 1500)])
AS = np.r_[np.linspace(0, 4.4, 45)]
TYt = []; TEt = []
for a in AS:
    by = np.zeros(s); be = np.zeros(s)
    for y in ys:
        ty, te = T(a + 1j*y); by = np.maximum(by, abs(ty)); be = np.maximum(be, abs(te))
    TYt.append(by); TEt.append(be)
np.savez('tau_table.npz', a=AS, ty=np.array(TYt), te=np.array(TEt))
for k in range(0, 45, 5):
    print(f"a=h*mu={AS[k]:.2f}: sum sup|T_y|={np.sum(TYt[k]):.1f} sum sup|T_e|={np.sum(TEt[k]):.1f}")
