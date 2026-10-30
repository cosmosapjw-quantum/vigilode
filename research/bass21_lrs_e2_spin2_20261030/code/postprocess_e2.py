#!/usr/bin/env python3
"""Derive inspectable E2 shear/collision diagnostics from actual full NPZ.

No independent IVP or external source, only deterministic source-preserving reduction.
"""
import argparse, pathlib, json, csv, numpy as np

def execute(npz_path,out):
    out=pathlib.Path(out);out.mkdir(parents=True,exist_ok=True)
    with np.load(npz_path,allow_pickle=False) as a:
        x=a['x'];E=a['E2_candidate'];c=a['spin2_c_even'];S=a['E2_shear_rate'];C=a['E2_collision_rate'];F=a['j2_polarized_collision_feedback']; names=a['cases'];pairs=a['paired_E2']
        stats={'median_abs_c4_over_c2_s04_at_last':float(np.median(np.abs(c[25:,-1,1])/np.maximum(np.abs(c[25:,-1,0]),1.e-20))),
         'all_25_paired_final_e2_positive':bool(np.all(pairs[:,-1]>0)),
         'final_E2_sign_counts':{'positive':int(np.sum(E[:,-1]>0)),'negative':int(np.sum(E[:,-1]<0))},
         'mean_final_shear_rate':float(S[:,-1].mean()),'mean_final_collision_rate':float(C[:,-1].mean()),
         'mean_accumulated_shear_rate':float(np.trapezoid(S,x=x).mean()),'mean_accumulated_collision_rate':float(np.trapezoid(C,x=x).mean()),
         'mean_final_e2_reconstructed_from_rate':float(np.trapezoid(S+C,x=x).mean()),
         'max_abs_E2_work_residual_from_trapezoid':float(np.max(np.abs(E[:,-1]-np.trapezoid(S+C,x=x))))}
        (out/'PHYSICS_INTERPRETATION.json').write_text(json.dumps(stats,indent=2,sort_keys=True,allow_nan=False)+'\n')
        rows=[]
        for i,name in enumerate(names):
            rows.append({'case':str(name),'E2_final':repr(float(E[i,-1])),'c2_final':repr(float(c[i,-1,0])),'c4_final':repr(float(c[i,-1,1])),
             'E2_shear_rate_final':repr(float(S[i,-1])),'E2_collision_rate_final':repr(float(C[i,-1])),
             'j2_polarized_collision_feedback_final':repr(float(F[i,-1])),
             'E2_shear_integral':repr(float(np.trapezoid(S[i],x=x))),
             'E2_collision_integral':repr(float(np.trapezoid(C[i],x=x)))})
        with (out/'ENDPOINT_50_SHEAR_COLLISION.csv').open('w',newline='') as f:
            wr=csv.DictWriter(f,fieldnames=rows[0].keys(),lineterminator='\n');wr.writeheader();wr.writerows(rows)
        return stats
if __name__=='__main__':
    ap=argparse.ArgumentParser();ap.add_argument('--npz',required=True);ap.add_argument('--out',required=True);a=ap.parse_args()
    print(json.dumps(execute(a.npz,a.out),indent=2))
