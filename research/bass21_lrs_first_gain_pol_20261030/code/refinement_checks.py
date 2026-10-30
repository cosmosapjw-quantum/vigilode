"""Measured saved-grid replay variation; not original BDF/global continuum convergence."""
from __future__ import annotations
import json,sys,csv,pathlib
import numpy as np
ROOT=pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'code'))
import lrs_polarization_replay as P

def main(archive,out):
    histories=P.load_owner_archive(archive)
    rows=[];max97=0.;max49=0.;qpair=[]
    for name,h in sorted(histories.items()):
        j2=h['photon_energy_moments'][:,2]
        results={}
        for stride in (1,2,4):
            ix=np.arange(0,193,stride)
            if ix[-1]!=192:raise RuntimeError('MUST_INCLUDE_OBSERVER_EPOCH')
            results[stride]=P.integrate_los(h['g'][ix],j2[ix],h['b'][ix],h['tau'][ix],h['x'][ix],all_epochs=False)[0]
        e97=float(np.max(abs(results[1]-results[2])));e49=float(np.max(abs(results[1]-results[4])))
        max97=max(max97,e97);max49=max(max49,e49)
        rows.append({'case':name,'Q193_equator':float(results[1][0]),'Q97_equator':float(results[2][0]),
         'Q49_equator':float(results[4][0]),'max_abs_193_97':e97,'max_abs_193_49':e49})
    p=pathlib.Path(out);p.mkdir(parents=True,exist_ok=True)
    with (p/'REFINEMENT_50_CASES.csv').open('w',newline='') as f:
        w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
    d={'schema':'bass21.saved_sampled_grid_pol_replay_checks.v1','cases':50,
       'max_abs_193_vs_97':max97,'max_abs_193_vs_49':max49,
       'claim':'Quadrature sampling sensitivity, not physical/BDF global convergence certificate'}
    (p/'REFINEMENT_SUMMARY.json').write_text(json.dumps(d,indent=2,sort_keys=True)+'\n')
    print(json.dumps(d,indent=2))

if __name__=='__main__':main(sys.argv[1],sys.argv[2])
