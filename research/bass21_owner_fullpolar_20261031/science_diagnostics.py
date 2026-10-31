"""Reproducible source-frozen physical diagnostics, no new IVP or closure."""
from __future__ import annotations
import csv,json,math
from collections import defaultdict
from pathlib import Path
import numpy as np
import matplotlib
matplotlib.use('Agg')
matplotlib.rcParams['svg.hashsalt']='VIGILODE_OWNER_FROZEN_FULLSTOKES_20261031'
import matplotlib.pyplot as plt
ROOT=Path(__file__).resolve().parent
source=ROOT/'results/OWNER_FULLPOLAR_COLLISION_193000.csv'
R=ROOT/'results';F=ROOT/'figures'

def run():
    frame=defaultdict(lambda:dict(count=0,maxU=0.,meanU=0.,maxQ=0.,maxV=0.,extreme=None))
    epoch=defaultdict(float)
    case=defaultdict(float)
    with source.open(newline='') as f:
      for row in csv.DictReader(f):
        name=row['frame'];i=int(row['epoch_index']);u=float(row['collision_U']);q=float(row['collision_Q']);v=float(row['collision_V'])
        fdat=frame[name];fdat['count']+=1;fdat['meanU']+=abs(u)
        if abs(u)>fdat['maxU']:
            fdat['maxU']=abs(u);fdat['extreme']={k:row[k] for k in ['case','epoch_index','direction','collision_U','collision_Q']}
        fdat['maxQ']=max(fdat['maxQ'],abs(q));fdat['maxV']=max(fdat['maxV'],abs(v))
        epoch[(name,i)]=max(epoch[(name,i)],abs(u))
        case[(name,row['case'])]=max(case[(name,row['case'])],abs(u))
    for d in frame.values():d['meanU']/=d['count']
    assert sum(d['count'] for d in frame.values())==193000
    assert max(frame['rest']['maxU'],frame['z005']['maxU'])<1e-12
    assert frame['x005']['maxU']>5e-4 and frame['tilt45']['maxU']>5e-4
    assert max(d['maxV'] for d in frame.values())==0
    R.mkdir(exist_ok=True);F.mkdir(exist_ok=True)
    with (R/'SOURCE_FROZEN_U_FRAME_DIAGNOSTICS.csv').open('w',newline='') as f:
        w=csv.writer(f);w.writerow(['frame','rows','max_abs_CU','mean_abs_CU','max_abs_CQ','max_abs_CV','case_at_maxU','epoch_at_maxU','direction_at_maxU','signed_CU_at_max'])
        for name,d in frame.items():
            e=d['extreme'];w.writerow([name,d['count'],d['maxU'],d['meanU'],d['maxQ'],d['maxV'],e['case'],e['epoch_index'],e['direction'],e['collision_U']])
    with (R/'SOURCE_FROZEN_U_EPOCH_MAX.csv').open('w',newline='') as f:
        w=csv.writer(f);w.writerow(['frame','epoch_index','max_abs_CU'])
        for k in sorted(epoch):w.writerow([k[0],k[1],epoch[k]])
    with (R/'SOURCE_FROZEN_U_CASE_MAX.csv').open('w',newline='') as f:
        w=csv.writer(f);w.writerow(['frame','case','max_abs_CU'])
        for k in sorted(case):w.writerow([k[0],k[1],case[k]])
    fig,ax=plt.subplots(figsize=(8.3,4.9))
    for name in ['rest','x005','tilt45','z005']:
        ax.plot(np.arange(193),[epoch.get((name,i),0.) for i in range(193)],label=name)
    ax.set(xlabel='Original BASS saved epoch index',ylabel=r'Max $|C_U|$ over 50 cases x 5 directions',title='Source-frozen Thomson collision, controlled electron velocities')
    ax.grid(alpha=.28);ax.legend();fig.tight_layout()
    fig.savefig(F/'source_frozen_max_U_by_epoch.png',dpi=170)
    fig.savefig(F/'source_frozen_max_U_by_epoch.svg',metadata={'Date':None})
    plt.close(fig)
    out={'status':'NUMERICALLY_CHECKED_SAVED_SOURCE_NOT_NEW_IVP','source_csv':source.name,
         'total_rows':sum(d['count'] for d in frame.values()),'frames':frame,'owner_ivps':0,'VIGILODE_DAG':'7/23'}
    (R/'FRAME_DIAGNOSTICS_SUMMARY.json').write_text(json.dumps(out,indent=2,sort_keys=True)+'\n')
    print(json.dumps(out,indent=2,sort_keys=True))
    return out
if __name__=='__main__':run()
