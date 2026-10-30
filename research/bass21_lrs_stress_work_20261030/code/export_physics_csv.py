#!/usr/bin/env python3
"""Export model-derived BASS21 radiation stress physical tables, preserving exact NPZ."""
from pathlib import Path
import csv,numpy as np,argparse

def write(path,fields,rows):
 with open(path,'w',newline='',encoding='utf8') as f:
  w=csv.writer(f,lineterminator='\n');w.writerow(fields);w.writerows(rows)

def export(folder):
 p=Path(folder)
 with np.load(p/'STRESS_50x193x6_SAME_OWNER.npz',allow_pickle=False) as f:
  a={name:f[name] for name in f.files}  # decompress each array once, not per CSV cell
  names=a['cases'];x=a['x'];l=a['l_even'];J=a['deltaJ_even'];pair=a['pairs'];dJ=a['paired_deltaJ_even'];
  f0=p/'FULL50_EPOCH_MULTIPOLE_ROWS.csv';f1=p/'MATCHED25_EPOCH_MULTIPOLE_ROWS.csv';f2=p/'FULL50_EPOCH_STRESS.csv'
  write(f0,['case','epoch_index','x_log_a','ell','delta_Jell_a4_normalized'],((str(name),j,format(float(xj),'.17g'),int(ell),format(float(J[i,j,k]),'.17g')) for i,name in enumerate(names) for j,xj in enumerate(x) for k,ell in enumerate(l)))
  write(f1,['pair','epoch_index','x_log_a','ell','matched_delta_delta_Jell'],((str(name),j,format(float(xj),'.17g'),int(ell),format(float(dJ[i,j,k]),'.17g')) for i,name in enumerate(pair) for j,xj in enumerate(x) for k,ell in enumerate(l)))
  write(f2,['case','epoch_index','x_log_a','delta_rho','delta_pressure_perp','delta_pressure_parallel','delta_rho_anisotropic_work','delta_rho_work_defect','deltaQ_B_projection','delta_J2_shear_source','delta_J2_Thomson_collision'],((str(name),j,format(float(xj),'.17g'),format(float(J[i,j,0]),'.17g'),format(float(a['deltaP_perp'][i,j]),'.17g'),format(float(a['deltaP_parallel'][i,j]),'.17g'),format(float(a['deltaJ0_geometric_work'][i,j]),'.17g'),format(float(a['energy_balance_gap'][i,j]),'.17g'),format(float(a['BQ'][i,j]),'.17g'),format(float(a['quadrupole_shear'][i,j]),'.17g'),format(float(a['quadrupole_collision'][i,j]),'.17g')) for i,name in enumerate(names) for j,xj in enumerate(x)))
 print('WROTE_CSV_ROWS',len(names)*len(x)*len(l),len(pair)*len(x)*len(l),len(names)*len(x))
if __name__=='__main__':
 ap=argparse.ArgumentParser();ap.add_argument('--results',type=Path,required=True);export(ap.parse_args().results)
