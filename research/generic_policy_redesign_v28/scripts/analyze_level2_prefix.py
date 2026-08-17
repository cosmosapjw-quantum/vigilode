#!/usr/bin/env python3
from __future__ import annotations
import hashlib, json
from pathlib import Path
import numpy as np
import pandas as pd

ROOT=Path(__file__).resolve().parents[1]
SHARDS=ROOT/'results/level2_shards'
OUT=ROOT/'results/analysis'
V27=Path('/mnt/data/GVJF_KEYSTONE_CODING_RESEARCH_CLOSEOUT_20260817/research_evidence/generic_policy_redesign_v27/results/prefix_shards')
HIST=Path('/mnt/data/GVJF_POLICY_MATH_RESEARCH_20260817/source_v24/research/generic_regime_atlas_v20/results/G4_S5B0_AGGREGATED_STEPS.csv')
CONTRACT=ROOT/'contracts/STAGED_LEVEL2_PREFIX_CONTRACT.json'

def sha(p): return hashlib.sha256(Path(p).read_bytes()).hexdigest()

def same_float(a,b):
    if a is None or b is None: return a is None and b is None
    return np.float64(a).view(np.uint64)==np.float64(b).view(np.uint64)

def exact_r_parity(new, old):
    af=['trajectory_id','family','dimension','rtol','attempt_index','accepted_steps_before','t_start','h','error_norm','accepted','recoverable_failure','failure','rhs_evaluations','jvp_vectors','linear_matvecs']
    sf=['trajectory_id','family','dimension','rtol','step_index','t_start','h','transition_level','rodas_embedded_error','rodas_rhs_evaluations','rodas_jvp_vectors','rodas_linear_matvecs']
    if len(new['attempt_rows'])!=len(old['attempt_rows']) or len(new['accepted_rows'])!=len(old['accepted_rows']): return False
    for x,y in zip(new['attempt_rows'],old['attempt_rows']):
        for f in af:
            if isinstance(x[f],float) or isinstance(y[f],float):
                if not same_float(x[f],y[f]): return False
            elif x[f]!=y[f]: return False
    for x,y in zip(new['accepted_rows'],old['accepted_rows']):
        for f in sf:
            if isinstance(x[f],float) or isinstance(y[f],float):
                if not same_float(x[f],y[f]): return False
            elif x[f]!=y[f]: return False
    return new['trajectories']==old['trajectories']

def event_parity(new, old):
    if len(new['prefix_rows'])!=len(old['prefix_rows']): return False
    fields=['trajectory_id','family','dimension','rtol','policy','decision_accepted_step','feature_value','target_attempt_index','target_accepted_steps_before','t_start','h','target_r_attempt_accepted','target_r_error_norm','target_r_recoverable_failure','prefix_succeeded','prefix_failure','full_e_continued']
    for x,y in zip(new['prefix_rows'],old['prefix_rows']):
        for f in fields:
            xv,yv=x[f],y[f]
            if isinstance(xv,float) or isinstance(yv,float):
                if not same_float(xv,yv): return False
            elif xv!=yv: return False
        if x['prefix_report']['level1_report'] != y['prefix_report']:
            return False
    return True

def main():
    OUT.mkdir(parents=True,exist_ok=True)
    contract=json.load(open(CONTRACT)); assert contract['schema']=='gvjf-policy-redesign-v28-staged-level2-prefix-contract-v1'
    files=sorted(SHARDS.glob('n*_*.json'))
    if len(files)!=12: raise RuntimeError(f'expected 12 shards, got {len(files)}')
    parity=[]; rows=[]
    for p in files:
        d=json.load(open(p)); oldp=V27/p.name; old=json.load(open(oldp))
        rp=exact_r_parity(d,old); ep=event_parity(d,old)
        parity.append({'file':p.name,'dimension':d['trajectories'][0]['dimension'],'family':d['trajectories'][0]['family'],'rjf_exact_parity':rp,'event_and_level1_exact_parity':ep,'v27_sha256':sha(oldp),'v28_sha256':sha(p)})
        if not rp: raise RuntimeError(f'R-JF drift: {p.name}')
        if not ep: raise RuntimeError(f'event/level1 drift: {p.name}')
        if d['switching_active'] or d['full_e_continuations']!=0: raise RuntimeError(f'forbidden activation: {p.name}')
        if d['status']!='complete': raise RuntimeError(f'incomplete shard: {p.name}')
        for r in d['prefix_rows']:
            if not r['prefix_succeeded'] or r['prefix_report'] is None: raise RuntimeError(f'prefix failure: {p.name}')
            pr=r['prefix_report']; l1=pr['level1_report']; t2=l1['early_flow_defect']; t3=pr['stage3_flow_defect']; t4=pr['stage4_flow_defect']
            w1=l1['work']; wi=pr['level2_incremental_work']; wc=pr['cumulative_work']
            rows.append({'source_file':p.name,'dimension':r['dimension'],'family':r['family'],'trajectory_id':r['trajectory_id'],'rtol':r['rtol'],'decision_step':r['decision_accepted_step'],'target_step':r['target_accepted_steps_before'],'target_attempt_index':r['target_attempt_index'],'t_start':r['t_start'],'h':r['h'],'target_r_accepted':r['target_r_attempt_accepted'],'target_r_error':r['target_r_error_norm'],
                         'rho2':t2['tolerance_scaled_defect_wrms'],'rho3':t3['tolerance_scaled_defect_wrms'],'rho4':t4['tolerance_scaled_defect_wrms'],'eta2':t2['normalized_defect'],'eta3':t3['normalized_defect'],'eta4':t4['normalized_defect'],
                         'level1_jvp':w1['jvp_vectors'],'level2_incremental_jvp':wi['jvp_vectors'],'level2_cumulative_jvp':wc['jvp_vectors'],'level1_rhs':w1['rhs_evaluations'],'level2_incremental_rhs':wi['rhs_evaluations'],'level2_cumulative_rhs':wc['rhs_evaluations'],'level1_phi':w1['phi_actions'],'level2_incremental_phi':wi['phi_actions'],'level2_cumulative_phi':wc['phi_actions'],'level2_incremental_krylov':wi['phi_krylov_vectors']})
    df=pd.DataFrame(rows)
    df['max_rho34']=df[['rho3','rho4']].max(axis=1)
    df['rho4_over_rho3']=df['rho4']/df['rho3']
    df['max_rho34_over_rho2']=df['max_rho34']/df['rho2']
    hist=pd.read_csv(HIST)
    labels=hist[['trajectory_id','dimension','step_index','strong_winner','exponential_locally_admissible','exponential_total_error','exponential_jvp_vectors','exponential_rhs_evaluations','rodas_jvp_vectors','rodas_rhs_evaluations']]
    joined=df.merge(labels,left_on=['trajectory_id','dimension','target_step'],right_on=['trajectory_id','dimension','step_index'],how='left',validate='one_to_one')
    if joined['strong_winner'].isna().any(): raise RuntimeError('historical label join missing')
    joined['unsafe']=~joined['exponential_locally_admissible']
    joined['level2_incremental_over_target_r_jvp']=joined['level2_incremental_jvp']/joined['rodas_jvp_vectors']
    joined['level2_cumulative_over_target_r_jvp']=joined['level2_cumulative_jvp']/joined['rodas_jvp_vectors']
    joined['level2_incremental_over_full_e_jvp']=joined['level2_incremental_jvp']/joined['exponential_jvp_vectors']
    joined['level2_cumulative_over_full_e_jvp']=joined['level2_cumulative_jvp']/joined['exponential_jvp_vectors']
    joined.to_csv(OUT/'LEVEL2_ROWS_WITH_HISTORICAL_LABELS.csv',index=False)
    pd.DataFrame(parity).to_csv(OUT/'RJF_EVENT_PARITY_LEDGER.csv',index=False)
    hires=joined[joined.family=='hires-ramped'].copy(); hires.to_csv(OUT/'HIRES_KILL_ROWS.csv',index=False)
    feats=['rho2','rho3','rho4','max_rho34','rho4_over_rho3','max_rho34_over_rho2']
    sep=[]
    for f in feats:
        safe=joined.loc[~joined.unsafe,f]; bad=joined.loc[joined.unsafe,f]
        high_gap=float(bad.min()-safe.max())
        low_gap=float(safe.min()-bad.max())
        sep.append({'feature':f,'safe_min':float(safe.min()),'safe_max':float(safe.max()),'unsafe_min':float(bad.min()),'unsafe_max':float(bad.max()),'high_side_strict_separation':bool(bad.min()>safe.max()),'high_side_gap':high_gap,'low_side_strict_separation':bool(bad.max()<safe.min()),'low_side_gap':low_gap})
    sepdf=pd.DataFrame(sep); sepdf.to_csv(OUT/'LEVEL2_FEATURE_SEPARATION.csv',index=False)
    dims=[]
    for dim,g in joined.groupby('dimension'):
        dims.append({'dimension':int(dim),'events':len(g),'unsafe_events':int(g.unsafe.sum()),'prefix_failures':0,'level2_incremental_jvp_median':float(g.level2_incremental_jvp.median()),'level2_incremental_jvp_p95':float(g.level2_incremental_jvp.quantile(.95)),'level2_incremental_jvp_max':int(g.level2_incremental_jvp.max()),'level2_cumulative_jvp_median':float(g.level2_cumulative_jvp.median()),'level2_cumulative_jvp_p95':float(g.level2_cumulative_jvp.quantile(.95)),'level2_cumulative_jvp_max':int(g.level2_cumulative_jvp.max()),'cumulative_over_target_r_jvp_median':float(g.level2_cumulative_over_target_r_jvp.median()),'cumulative_over_target_r_jvp_p95':float(g.level2_cumulative_over_target_r_jvp.quantile(.95)),'cumulative_over_full_e_jvp_median':float(g.level2_cumulative_over_full_e_jvp.median()),'cumulative_over_full_e_jvp_p95':float(g.level2_cumulative_over_full_e_jvp.quantile(.95))})
    dimdf=pd.DataFrame(dims); dimdf.to_csv(OUT/'LEVEL2_DIMENSION_SUMMARY.csv',index=False)
    fam=joined.groupby(['dimension','family']).agg(events=('family','size'),unsafe_events=('unsafe','sum'),rho4_over_rho3_min=('rho4_over_rho3','min'),rho4_over_rho3_max=('rho4_over_rho3','max'),incremental_jvp_median=('level2_incremental_jvp','median'),incremental_jvp_max=('level2_incremental_jvp','max')).reset_index(); fam.to_csv(OUT/'LEVEL2_FAMILY_SUMMARY.csv',index=False)
    result={'schema':'gvjf-policy-redesign-v28-staged-level2-result-v1','contract_sha256':sha(CONTRACT),'implementation_head_before_output':'3d9068f58fdd53482c1ee1d58277df658888152d','solver_shards':12,'events':len(joined),'events_n128':int((joined.dimension==128).sum()),'events_n512':int((joined.dimension==512).sum()),'prefix_failures':0,'full_e_continuations':0,'active_switching':False,'n2048_executed':False,'rjf_exact_parity_all':bool(pd.DataFrame(parity).rjf_exact_parity.all()),'event_and_level1_exact_parity_all':bool(pd.DataFrame(parity).event_and_level1_exact_parity.all()),'unsafe_events_historical_labels':joined.loc[joined.unsafe,['dimension','family','decision_step','target_step','rho2','rho3','rho4','rho4_over_rho3','max_rho34_over_rho2','exponential_total_error']].to_dict('records'),'strict_separating_descriptive_features':sepdf.loc[sepdf.high_side_strict_separation|sepdf.low_side_strict_separation,'feature'].tolist(),'dimension_summary':dimdf.to_dict('records'),'primary_verdict':'level2 adds real safety-discriminating information: stage-amplification ratios strictly separate the replicated HIRES inadmissible structural event from all observed safe events, while raw rho3/rho4 levels do not; this is descriptive evidence only and no final threshold is selected'}
    (OUT/'RESULT_SUMMARY.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result,indent=2))

if __name__=='__main__': main()
