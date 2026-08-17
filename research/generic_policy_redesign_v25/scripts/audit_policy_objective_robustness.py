#!/usr/bin/env python3
from __future__ import annotations
import ast, json, sys
from pathlib import Path
import matplotlib.pyplot as plt
import numpy as np
import pandas as pd

ROOT=Path(__file__).resolve().parents[1]
CAL=ROOT/'results/calibration128_analysis/CALIBRATION_CANDIDATES.csv'
CROSS=ROOT/'results/crossdim512_analysis/ALL_108_FROZEN_THRESHOLD_CANDIDATES.csv'
OUT=ROOT/'results/crossdim512_analysis'
PLOTS=ROOT/'plots'


def main():
    c128=pd.read_csv(CAL)
    c512=pd.read_csv(CROSS)
    cols512=['candidate_key','opportunity_retention','probe_precision','probe_count','aggregate_prefix_overhead','min_family_precision','min_family_opportunity']
    m=c128.merge(c512[cols512],on='candidate_key',suffixes=('_128','_512'))
    m['min_opportunity']=m[['opportunity_retention_128','opportunity_retention_512']].min(axis=1)
    m['min_pooled_precision']=m[['probe_precision_128','probe_precision_512']].min(axis=1)
    m['max_prefix_overhead']=m[['aggregate_prefix_overhead_128','aggregate_prefix_overhead_512']].max(axis=1)
    # Development diagnostic only: it is NOT a replacement for the frozen v2.5 policy.
    safe_first=m[(m.min_pooled_precision>=1.0-1e-15)&(m.min_opportunity>=0.75)].copy()
    if safe_first.empty:
        safety_choice=None
    else:
        safety_choice=safe_first.sort_values(
            ['min_opportunity','max_prefix_overhead','candidate_key'],
            ascending=[False,True,True],kind='mergesort').iloc[0]
    best_opp=float(m.min_opportunity.max())
    epsilon=0.01
    epsilon_pool=m[m.min_opportunity>=best_opp-epsilon].copy()
    eps_choice=epsilon_pool.sort_values(
        ['min_pooled_precision','min_family_precision','max_prefix_overhead','candidate_key'],
        ascending=[False,False,True,True],kind='mergesort').iloc[0]
    frozen_key=json.load(open(ROOT/'contracts/FROZEN_POLICY_CALIBRATION128.json'))
    frozen_name=f"{frozen_key['feature']}|q={float(frozen_key['quantile']):.2f}|k={int(frozen_key['persistence_k'])}|delta={float(frozen_key['prefix_budget_delta']):.2f}"
    frozen=m[m.candidate_key==frozen_name].iloc[0]
    m.to_csv(OUT/'CROSSDIM_108_COMBINED_ROBUSTNESS.csv',index=False)

    fig,ax=plt.subplots(figsize=(7.5,5.0))
    for k,g in m.groupby('k'):
        ax.scatter(g.min_opportunity,g.min_pooled_precision,s=38,alpha=.7,label=f'k={int(k)}')
    ax.scatter([frozen.min_opportunity],[frozen.min_pooled_precision],marker='*',s=200,label='v2.5 frozen')
    if safety_choice is not None:
        ax.scatter([safety_choice.min_opportunity],[safety_choice.min_pooled_precision],marker='D',s=100,label='safety-first diagnostic')
    ax.set_xlabel('min opportunity retention across N=128,512')
    ax.set_ylabel('min pooled probe precision across N=128,512')
    ax.set_title('Objective sensitivity: persistence trades tiny opportunity for large precision')
    ax.set_xlim(0,1.02); ax.set_ylim(.45,1.02); ax.grid(True,alpha=.25); ax.legend()
    fig.tight_layout(); fig.savefig(PLOTS/'POLICY_OBJECTIVE_ROBUSTNESS.png',dpi=180); plt.close(fig)

    summary={
      'schema':'gvjf-policy-redesign-v25-objective-robustness-audit-v1',
      'status':'diagnostic-only-no-retuning',
      'frozen_policy':{k:(float(v) if isinstance(v,(np.floating,float)) else int(v) if isinstance(v,(np.integer,int)) else v) for k,v in frozen.to_dict().items()},
      'epsilon_absolute_opportunity':epsilon,
      'epsilon_pool_count':int(len(epsilon_pool)),
      'epsilon_precision_choice':{k:(float(v) if isinstance(v,(np.floating,float)) else int(v) if isinstance(v,(np.integer,int)) else v) for k,v in eps_choice.to_dict().items()},
      'safety_first_constraint':{'min_pooled_precision':1.0,'min_opportunity':0.75},
      'safety_first_candidate_count':int(len(safe_first)),
      'safety_first_choice':None if safety_choice is None else {k:(float(v) if isinstance(v,(np.floating,float)) else int(v) if isinstance(v,(np.integer,int)) else v) for k,v in safety_choice.to_dict().items()},
      'interpretation':'The v2.5 frozen policy is not changed. This audit motivates a new v2.6 contract before any N=2048 sealed holdout.'
    }
    (OUT/'OBJECTIVE_ROBUSTNESS_AUDIT.json').write_text(json.dumps(summary,indent=2)+'\n')
    print(json.dumps(summary,indent=2))

if __name__=='__main__': main()
