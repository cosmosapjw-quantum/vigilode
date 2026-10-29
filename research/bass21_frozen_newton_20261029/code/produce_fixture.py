"""Reproducible single frozen-BASS-source certificate and source-cell scaling plot."""
from __future__ import annotations
import json,sys,math
from pathlib import Path
import numpy as np
from bass21_stage_box import build_native_box

HERE=Path(__file__).resolve().parents[1]
OUT=HERE/'results'
OUT.mkdir(exist_ok=True)

def make(rad_den,write_box=False):
    _,rate,b,T,UV=build_native_box(sigma=1.0,nb=1e6,rad_den=rad_den)
    j=np.array([[float(v) for v in row] for row in b.J0]); f=np.array([float(v) for v in b.F0]);
    x=np.linalg.solve(np.eye(len(f))-j,f)
    v=b.certify(x,1e-6,1e-6)
    if write_box:
        b.export_csv(x,OUT/'BASS21_ORIGINAL_N03_INTERVAL_INPUT.txt')
    actual=[]
    for k in range(24):
        rng=np.random.default_rng(100+k)
        z=[float(yi+di*__import__('fractions').Fraction(str(rng.uniform(-1,1)))) for yi,di in zip(b.y,b.d)]
        F,J=b.physical_sample(__import__('bass21_stage_box').vec(z)) # z may round outside by <ulps, checked
        J=np.array([[float(a) for a in row] for row in J]);F=np.array([float(a) for a in F])
        xe=np.linalg.solve(np.eye(len(x))-J,F)
        actual.append(max(abs(xi-ei)/float(pi) for xi,ei,pi in zip(x,xe,b.p)))
    return dict(rad_den=rad_den,min_realized_magnitude=min(float(v) for v in b.y if v>0),
                weighted_mu_upper=v['mu'],inverse_upper=v['inverse'],
                py_original_residual_bound=v['original_residual'],
                py_physical_error_bound=v['physical_stage_error'],
                max_sampled_physical_error=max(actual),
                sample_count=len(actual),accepted=v['accepted'],T_BASS=T,UV_BASS=UV,
                nonzero_rate_count=int(np.count_nonzero(rate['photo_s'])))

def main():
    rows=[make(i,write_box=i==256) for i in (256,512,1024,2048,4096)]
    for r in rows:
        if not r['accepted'] or r['max_sampled_physical_error']>r['py_physical_error_bound']:
            raise RuntimeError('SCOPE_CERTIFICATE_FAILED')
    (OUT/'BASS21_FROZEN_STAGE_RESULTS.json').write_text(json.dumps({'scope':'BASS21_SOURCE_FROZEN_PROPER_TIME_NEWTON_ONLY','rows':rows},indent=2)+'\n')
    print(json.dumps(rows,indent=2))
    import matplotlib
    matplotlib.use('Agg')
    from matplotlib import pyplot as plt
    f,ax=plt.subplots(figsize=(6.1,3.8))
    x=[1./r['rad_den'] for r in rows]
    ax.plot(x,[r['max_sampled_physical_error'] for r in rows],marker='o',label='Largest sampled stage error (not rigorous)')
    ax.plot(x,[r['py_physical_error_bound'] for r in rows],marker='s',label='Rigorous represented-rate box bound')
    ax.set(xlabel='Relative source-state box radius',ylabel='p-weighted stage error',title='Fixed BASS21 rates; one implicit-Euler Newton system')
    ax.legend(fontsize=8)
    ax.grid(True,alpha=0.3)
    f.tight_layout();f.savefig(OUT/'frozen_stage_box_bounds.png',dpi=175);plt.close(f)

if __name__=='__main__':main()
