import warnings; warnings.filterwarnings("ignore")
"""Matched-accuracy scoring for PROBE B5.
Rules: (R) regression frontier: per seed, OLS of log10(work) on log10(endpoint err) over the whole ladder,
evaluated at E; (Rw) windowed local fit (points with |log err - log E| <= 1, >= 4 points);
(C) harness cheapest-run: per seed, min work among runs with err <= E; (Cp) cheapest-run pooled over seeds.
Ratios arm/ref per seed; reported as median [min, max] over seeds. 'x' = E outside the measured error range
of either arm for that seed (extrapolated); 's' = cheapest-run saturated (E >= max err of both arms)."""
import json, sys, math
import numpy as np
from collections import defaultdict

def load(path, key=('problem', 'arm', 'seed')):
    D = defaultdict(list)
    for line in open(path):
        d = json.loads(line)
        D[tuple(d[k] for k in key)].append(d)
    for k in D: D[k].sort(key=lambda r: -r['rtol'])
    return D

def fit(rows, w):
    e = np.log10([r['err'] for r in rows]); y = np.log10([r[w] for r in rows])
    ok = np.isfinite(e) & np.isfinite(y)
    return np.polyfit(e[ok], y[ok], 1), (10 ** e[ok].min(), 10 ** e[ok].max())

def front(rows, w, E):
    (b, a), (lo, hi) = fit(rows, w)
    return 10 ** (a + b * math.log10(E)), not (lo <= E <= hi)

def wfront(rows, w, E, half=1.0):
    pts = [(math.log10(r['err']), math.log10(r[w])) for r in rows if r['err'] > 0 and abs(math.log10(r['err']) - math.log10(E)) <= half]
    if len(pts) < 4: return None
    e, y = np.array(pts).T
    if e.max() - e.min() < 0.3: return None
    b, a = np.polyfit(e, y, 1)
    return 10 ** (a + b * math.log10(E))

def cheapest(rows, w, E):
    ok = [r[w] for r in rows if r['err'] <= E]
    return min(ok) if ok else None

def summ(v):
    v = [x for x in v if x is not None and np.isfinite(x)]
    if not v: return None
    return (float(np.median(v)), float(min(v)), float(max(v)))

def fmt(s, flag=''):
    if s is None: return '   n/a          '
    return f"{s[0]:.3f}[{s[1]:.2f},{s[2]:.2f}]{flag:1s}"

def score(D, prob, arm, ref, w, Es, seeds, extra=()):
    out = {}
    for E in Es:
        R = []; Rw = []; Cc = []; xflag = False; sflag = False
        for s in seeds:
            ka = (prob,) + tuple(extra) + (arm, s); kr = (prob,) + tuple(extra) + (ref, s)
            if ka not in D or kr not in D: continue
            ra, rr = D[ka], D[kr]
            fa, xa = front(ra, w, E); fr, xr = front(rr, w, E); R.append(fa / fr); xflag |= (xa or xr)
            wa, wr = wfront(ra, w, E), wfront(rr, w, E)
            Rw.append(wa / wr if (wa and wr) else None)
            ca, cr = cheapest(ra, w, E), cheapest(rr, w, E)
            Cc.append(ca / cr if (ca and cr) else None)
            if E >= max(r['err'] for r in ra) and E >= max(r['err'] for r in rr): sflag = True
        pa = [r for s in seeds for r in D.get((prob,) + tuple(extra) + (arm, s), [])]
        pr = [r for s in seeds for r in D.get((prob,) + tuple(extra) + (ref, s), [])]
        ca, cr = cheapest(pa, w, E), cheapest(pr, w, E)
        out[E] = dict(R=summ(R), Rw=summ(Rw), C=summ(Cc), Cp=(ca / cr if ca and cr else None), x=xflag, s=sflag)
    return out

def rejfrac(D, prob, arm, seeds, extra=()):
    fr = []; ra = at = 0
    for s in seeds:
        rows = D.get((prob,) + tuple(extra) + (arm, s), [])
        a = sum(r['att'] for r in rows); j = sum(r['rej'] for r in rows)
        if a: fr.append(j / a); ra += j; at += a
    return (ra / at if at else None, min(fr) if fr else None, max(fr) if fr else None)
