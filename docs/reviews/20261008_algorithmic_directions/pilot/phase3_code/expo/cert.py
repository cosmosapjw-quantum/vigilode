"""PROBE B3 certification audit (EXPLORATORY). Closed-loop E runs with every phi action audited:
  arm Ec  : 2-norm KIOPS (E2) + JAK defect-integral bound per output + actual error vs dense augmented expm + mu_2(J) per step
            + the declared-structure log-norm bound mu_struct (O(n), matrix-free given the declared structure)
  arm Esc : WRMS-scaled KIOPS (E) + actual errors + mu_2(D J D^-1) per step (is the scaled-norm certificate applicable?)
Families: rot-96, forc-96 (mu_2 <= 0 structural), semi-96 and hires-96 (mu_2 > 0: EstimateOnly; bound measured anyway).
Violation: actual > bound (1 + 1e-9); 'above floor' additionally requires actual > 64 eps ||exact output|| (rounding floor:
the bound has no rounding term)."""
import os, sys, json, math
for _v in ("OPENBLAS_NUM_THREADS", "OMP_NUM_THREADS"): os.environ[_v] = "1"
HERE = os.path.dirname(os.path.abspath(__file__)); sys.path.insert(0, HERE); sys.path.insert(0, os.path.join(HERE, 'rodas'))
import numpy as np
import corpus_v2 as cv, run_e, pexprb as PX

EPS = np.finfo(float).eps


def mu_struct_rot(n):
    blocks = n // 2
    sb = cv.diversity_array(max(blocks, 1))[:blocks]
    m = 2 * blocks
    s_pad = cv.diversity_array(n)[np.arange(m, n)] if m < n else np.zeros(0)

    def mu(t, y):
        r, _ = cv.smooth_ramp(t, 0.50, 0.08)
        bs = 20.0 + 480.0 * r; eta = 0.1 + 0.8 * r
        lam = (-1.35 + math.sqrt(0.65 ** 2 + eta ** 2)) / 2.0          # lambda_max(sym([[-1, eta],[0, -0.35]]))
        mul = float(np.max(bs * sb * lam))                                # max over blocks of mu_2(R^T A_k R)
        if len(s_pad): mul = max(mul, float(np.max(-bs * s_pad)))
        rn, _ = cv.smooth_ramp(t, 0.60, 0.06)
        return mul + float(np.max(2.0 * 40.0 * rn * y))                  # + max diag of 2 nl y
    return mu


def mu_struct_forc(p):
    J = p['J']
    return lambda t, y: float(J(t, y).diagonal().max())                 # diagonal J: mu_2 = max diag (one JVP with 1)


def summarize(rows, mulog):
    acts = [r for r in rows]
    out = dict(n_actions=len(acts))
    mus = np.array([m[1] for m in mulog]); out['steps'] = len(mus)
    out['frac_steps_mu_le_0'] = float(np.mean(mus <= 0)) if len(mus) else None
    out['mu_max'] = float(mus.max()) if len(mus) else None
    if mulog and mulog[0][3] is not None:
        ms = np.array([m[3] for m in mulog]); out['mu_struct_max'] = float(ms.max()); out['frac_struct_le_0'] = float(np.mean(ms <= 0))
        out['struct_ge_true'] = bool(np.all(ms >= np.array([m[2] for m in mulog]) - 1e-9 * np.abs(ms)))
    B = [r for r in acts if r['bound'] is not None and r['mu'] is not None and r['mu'] <= 0]
    out['n_cert_actions'] = len(B)
    if B:
        act = np.array([r['actual'] for r in B]); bnd = np.array([r['bound'] for r in B]); nrm = np.array([r['norm'] for r in B])
        viol = act > bnd * (1 + 1e-9)
        floor = 64 * EPS * np.maximum(nrm, 1e-300)
        out['violations_raw'] = int(viol.sum())
        out['violations_above_floor'] = int((viol & (act > floor)).sum())
        sel = act > floor
        rat = bnd[sel] / act[sel]
        out['n_above_floor'] = int(sel.sum())
        if len(rat):
            out['ratio_quantiles'] = {q: float(np.quantile(rat, q)) for q in (0.0, 0.1, 0.5, 0.9, 0.99, 1.0)}
            out['frac_ratio_gt_1e4'] = float(np.mean(rat > 1e4))
            out['frac_ratio_gt_1e2'] = float(np.mean(rat > 1e2))
        out['bound_over_tol_median'] = float(np.median(bnd / np.array([r['tol'] for r in B])))
        # worst violations
        if viol.any():
            iv = np.argsort(-(act / np.maximum(bnd, 1e-300)) * viol)[:5]
            out['worst'] = [dict(actual=float(act[i]), bound=float(bnd[i]), norm=float(nrm[i]), call=B[i]['call'], tau=B[i]['tau'])
                            for i in iv if viol[i]]
    # estimate-only effectivity (all actions): est vs actual for the last output of each call
    E = [r for r in acts if r['k'] == 0]
    return out


def main():
    jobs = []
    for pn in ('rot-96', 'forc-96'):
        for rt in (1e-4, 1e-6, 1e-8):
            jobs.append((pn, 'Ec', rt))
        jobs.append((pn, 'Esc', 1e-6))
    for pn in ('semi-96', 'hires-96', 'bruss1d-50'):
        jobs.append((pn, 'Ec', 1e-6))
    res = {}
    for pn, arm, rt in jobs:
        p = dict(run_e.problem(pn))
        if pn.startswith('rot'): p['mu_struct'] = mu_struct_rot(p['n'])
        if pn.startswith('forc'): p['mu_struct'] = mu_struct_forc(p)
        a = PX.make_arm(**run_e.EARMS[arm])
        r = PX.integrate(p, rt, a)
        s = summarize(r.get('cert_rows', []), r.get('mu_log', []))
        s.update(att=r['att'], rej=r['rej'], err=float(p['err'](r['y'])), flops=r['flops']['total'], cert_flops=r['flops']['cert'],
                 cert_overhead=r['flops']['cert'] / r['flops']['total'])
        res[f'{pn} {arm} {rt:g}'] = s
        print(pn, arm, rt, json.dumps(s, default=float), flush=True)
        json.dump(res, open(os.path.join(HERE, 'res/cert.json'), 'w'), indent=1, default=float)


if __name__ == '__main__':
    main()
