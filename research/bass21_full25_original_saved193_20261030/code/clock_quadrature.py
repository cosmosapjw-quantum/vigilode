"""Retrospective saved-clock quadrature diagnostics; no trajectory evolution or IVPs."""
from __future__ import annotations
import csv,json,pathlib,collections
import numpy as np
from scipy.integrate import cumulative_simpson,cumulative_trapezoid
ROOT=pathlib.Path(__file__).resolve().parents[1]
def main():
 grouped=collections.defaultdict(list)
 with (ROOT/'results'/'ALL_50_NATIVE_193_CHEMISTRY.csv').open() as f:
  for row in csv.DictReader(f):grouped[row['case']].append(row)
 out=[]
 for c,rs in sorted(grouped.items()):
  rs.sort(key=lambda r:int(r['epoch']))
  x=np.array([float(r['x']) for r in rs]);g=np.array([float(r['gamma_T_per_H']) for r in rs]);tau=np.array([float(r['tau_T_saved']) for r in rs])
  quad=cumulative_simpson(g,x=x,initial=0);trap=cumulative_trapezoid(g,x=x,initial=0)
  d={'case':c,'samples':len(rs),'tau_stored':float(tau[-1]),'tau_simpson':float(quad[-1]),'tau_trapezoid':float(trap[-1]),
     'final_simpson_minus_saved':float(quad[-1]-tau[-1]),'max_simpson_minus_saved':float(np.max(np.abs(quad-tau))),
     'final_trapezoid_minus_saved':float(trap[-1]-tau[-1]),'max_trapezoid_minus_saved':float(np.max(np.abs(trap-tau))),
     'max_saved_tau_step_decrease':float(np.min(np.diff(tau))),
     'minimum_gamma':float(g.min()),'maximum_gamma':float(g.max())}
  out.append(d)
 with (ROOT/'results'/'CLOCK_QUADRATURE_50_CASES.csv').open('w',newline='') as f:
  w=csv.DictWriter(f,fieldnames=list(out[0]));w.writeheader();w.writerows(out)
 summary={'schema':'vigilode.bass21.bdf_vs_sampled_quadrature_observation.v1',
   'n_cases':len(out),'max_over_cases_simpson_vs_saved':max(r['max_simpson_minus_saved'] for r in out),
   'max_over_cases_trap_vs_saved':max(r['max_trapezoid_minus_saved'] for r in out),
   'min_simpson_final_minus_saved':min(r['final_simpson_minus_saved'] for r in out),
   'max_simpson_final_minus_saved':max(r['final_simpson_minus_saved'] for r in out),
   'all_saved_tau_monotone':all(r['max_saved_tau_step_decrease']>=0 for r in out),
   'all_gamma_nonnegative':all(r['minimum_gamma']>=0 for r in out),
   'classification':'QUADRATURE_VS_STORED_BDF_SAMPLED_GRID; no new IVPs; not a solver-convergence certificate'}
 (ROOT/'results'/'CLOCK_QUADRATURE_SUMMARY.json').write_text(json.dumps(summary,indent=2,sort_keys=True)+'\n')
 print(json.dumps(summary,indent=2,sort_keys=True))
if __name__=='__main__':main()
