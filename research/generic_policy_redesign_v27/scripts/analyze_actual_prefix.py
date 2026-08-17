#!/usr/bin/env python3
from __future__ import annotations
import hashlib, json, math
from pathlib import Path
import matplotlib.pyplot as plt
import numpy as np
import pandas as pd

ROOT=Path(__file__).resolve().parents[1]
SHARDS=ROOT/'results/prefix_shards'
OUT=ROOT/'results/analysis'
PLOTS=ROOT/'plots'
V26=ROOT.parent/'generic_policy_redesign_v26/results/attempt_shards'
HIST=Path('/mnt/data/GVJF_POLICY_MATH_RESEARCH_20260817/source_v24/research/generic_regime_atlas_v20/results/G4_S5B0_AGGREGATED_STEPS.csv')
CONTRACT=ROOT/'contracts/ACTUAL_LEVEL1_PREFIX_CONTRACT.json'

def sha(p): return hashlib.sha256(Path(p).read_bytes()).hexdigest()

def load_jsons(pattern):
    return [(p,json.load(open(p))) for p in sorted(SHARDS.glob(pattern))]

def exact_r_parity(prefix_payload, baseline_payload):
    a=prefix_payload['attempt_rows']; b=baseline_payload['attempt_rows']
    if len(a)!=len(b): raise RuntimeError('attempt count drift')
    fields=['trajectory_id','family','dimension','rtol','attempt_index','accepted_steps_before','t_start','h','error_norm','accepted','recoverable_failure','failure','rhs_evaluations','jvp_vectors','linear_matvecs']
    for i,(x,y) in enumerate(zip(a,b)):
        for f in fields:
            xv=x[f]; yv=y[f]
            if isinstance(xv,float) or isinstance(yv,float):
                if xv is None or yv is None:
                    ok=xv is None and yv is None
                else:
                    ok=np.float64(xv).view(np.uint64)==np.float64(yv).view(np.uint64)
            else: ok=xv==yv
            if not ok: raise RuntimeError(f'attempt drift i={i} field={f}: {xv} != {yv}')
    a=prefix_payload['accepted_rows']; b=baseline_payload['accepted_rows']
    if len(a)!=len(b): raise RuntimeError('accepted count drift')
    fields=['trajectory_id','family','dimension','rtol','step_index','t_start','h','transition_level','rodas_embedded_error','rodas_rhs_evaluations','rodas_jvp_vectors','rodas_linear_matvecs']
    for i,(x,y) in enumerate(zip(a,b)):
        for f in fields:
            xv=x[f]; yv=y[f]
            if isinstance(xv,float) or isinstance(yv,float): ok=np.float64(xv).view(np.uint64)==np.float64(yv).view(np.uint64)
            else: ok=xv==yv
            if not ok: raise RuntimeError(f'accepted drift i={i} field={f}')
    return True

def baseline_path(dim,family):
    short={'robertson-ramped':'robertson','hires-ramped':'hires','van-der-pol-ramped':'van-der-pol','rotating-nonnormal':'rotating-nonnormal','nonautonomous-stiff-forcing':'nonautonomous-forcing','semilinear-advection-diffusion-ramped':'semilinear'}[family]
    return V26/f'n{dim}_{short}.json'

def flatten_prefix(payload,path):
    rows=[]
    attempt_wall={int(a['attempt_index']):a['wall_seconds'] for a in payload['attempt_rows']}
    attempt_jvp={int(a['attempt_index']):a['jvp_vectors'] for a in payload['attempt_rows']}
    for r in payload['prefix_rows']:
        pr=r.get('prefix_report'); tel=pr.get('early_flow_defect') if pr else None; w=pr.get('work') if pr else {}
        rows.append({
            'source_file':path.name,'dimension':r['dimension'],'family':r['family'],'trajectory_id':r['trajectory_id'],'rtol':r['rtol'],'policy':r['policy'],'decision_step':r['decision_accepted_step'],'feature_value':r['feature_value'],'target_attempt_index':r['target_attempt_index'],'target_step':r['target_accepted_steps_before'],'t_start':r['t_start'],'h':r['h'],'target_r_accepted':r['target_r_attempt_accepted'],'target_r_error_norm':r['target_r_error_norm'],'target_r_recoverable_failure':r['target_r_recoverable_failure'],'prefix_success':r['prefix_succeeded'],'prefix_failure':r['prefix_failure'],'prefix_wall_debug':r['prefix_wall_seconds'],'full_e_continued':r['full_e_continued'],
            'rho':None if tel is None else tel['tolerance_scaled_defect_wrms'],'eta':None if tel is None else tel['normalized_defect'],'prefix_rhs':w.get('rhs_evaluations'),'prefix_jvp':w.get('jvp_vectors'),'prefix_phi':w.get('phi_actions'),'prefix_krylov':w.get('phi_krylov_vectors'),'prefix_orth_ip':w.get('orthogonalization_inner_products'),'prefix_projected_exp':w.get('phi_projected_exponentials'),'prefix_jacobian_builds':w.get('jacobian_builds'),'prefix_direct_factorizations':w.get('direct_factorizations'),'prefix_newton_iterations':w.get('nonlinear_iterations'),
            'target_r_wall_debug':attempt_wall[int(r['target_attempt_index'])],'target_r_jvp':attempt_jvp[int(r['target_attempt_index'])],
        })
    return rows

def main():
    OUT.mkdir(parents=True,exist_ok=True); PLOTS.mkdir(parents=True,exist_ok=True)
    contract=json.load(open(CONTRACT)); assert contract['schema']=='gvjf-policy-redesign-v27-actual-level1-prefix-contract-v1'
    allrows=[]; parity=[]; trajcost=[]
    k3=load_jsons('*_k3.json')
    if len(k3)!=12: raise RuntimeError(f'expected 12 k3 shards, found {len(k3)}')
    for path,d in k3:
        tr=d['trajectories'][0]; base_path=baseline_path(tr['dimension'],tr['family']); base=json.load(open(base_path)); exact_r_parity(d,base)
        parity.append({'file':path.name,'dimension':tr['dimension'],'family':tr['family'],'exact_r_parity':True,'baseline_sha256':sha(base_path)})
        allrows += flatten_prefix(d,path)
        pref=d['prefix_rows']; at=d['attempt_rows']
        pj=sum(r['prefix_report']['work']['jvp_vectors'] for r in pref if r['prefix_report']); prhs=sum(r['prefix_report']['work']['rhs_evaluations'] for r in pref if r['prefix_report']); pw=sum(r['prefix_wall_seconds'] for r in pref); rj=sum(a['jvp_vectors'] for a in at); rrhs=sum(a['rhs_evaluations'] for a in at); rw=sum(a['wall_seconds'] for a in at)
        trajcost.append({'dimension':tr['dimension'],'family':tr['family'],'events':len(pref),'prefix_jvp':pj,'r_jvp':rj,'prefix_jvp_over_r_jvp':pj/rj,'prefix_rhs':prhs,'r_rhs':rrhs,'prefix_rhs_over_r_rhs':prhs/rrhs,'debug_prefix_wall':pw,'debug_r_wall':rw,'debug_prefix_over_r_wall':pw/rw})
    # k1 comparator only for the two known nonautonomous causal counterexample trajectories.
    for path,d in load_jsons('*_frozen-k1.json'):
        tr=d['trajectories'][0]; base_path=baseline_path(tr['dimension'],tr['family']); exact_r_parity(d,json.load(open(base_path))); allrows += flatten_prefix(d,path)
    rows=pd.DataFrame(allrows); k3rows=rows[rows.policy=='k3-development'].copy(); k1=rows[rows.policy=='frozen-k1-comparator'].copy()
    if not k3rows.prefix_success.all(): raise RuntimeError('k3 prefix failure')
    if not (~k3rows.full_e_continued).all(): raise RuntimeError('full E continuation appeared')
    if not k3rows.target_r_accepted.all(): raise RuntimeError('k3 event targeted rejected R attempt')
    if int(k3rows.prefix_jacobian_builds.fillna(0).sum())!=0 or int(k3rows.prefix_direct_factorizations.fillna(0).sum())!=0: raise RuntimeError('explicit factorization appeared')

    hist=pd.read_csv(HIST)
    labels=hist[['trajectory_id','dimension','step_index','strong_winner','exponential_locally_admissible','exponential_total_error','exponential_jvp_vectors','exponential_rhs_evaluations','rodas_wall_median','exponential_wall_median','exponential_prefix_wall_median']]
    joined=k3rows.merge(labels,left_on=['trajectory_id','dimension','target_step'],right_on=['trajectory_id','dimension','step_index'],how='left',validate='one_to_one')
    if joined.strong_winner.isna().any(): raise RuntimeError('historical label join missing')
    joined['historical_e_strong']=(joined.strong_winner=='E-K')
    joined['debug_prefix_over_target_r_wall']=joined.prefix_wall_debug/joined.target_r_wall_debug
    joined['prefix_jvp_over_target_r_jvp']=joined.prefix_jvp/joined.target_r_jvp
    joined['prefix_jvp_over_full_e_jvp']=joined.prefix_jvp/joined.exponential_jvp_vectors
    joined['prefix_rhs_over_full_e_rhs']=joined.prefix_rhs/joined.exponential_rhs_evaluations
    tc=pd.DataFrame(trajcost); parity_df=pd.DataFrame(parity)
    rows.to_csv(OUT/'ACTUAL_PREFIX_ROWS_ALL.csv',index=False); joined.to_csv(OUT/'K3_PREFIX_WITH_HISTORICAL_E_LABELS.csv',index=False); tc.to_csv(OUT/'K3_TRAJECTORY_COST_LEDGER.csv',index=False); parity_df.to_csv(OUT/'RJF_PARITY_LEDGER.csv',index=False)

    dimrows=[]
    for dim,g in joined.groupby('dimension'):
        t=tc[tc.dimension==dim]
        dimrows.append({'dimension':int(dim),'k3_events':len(g),'prefix_successes':int(g.prefix_success.sum()),'rejected_first_targets':int((~g.target_r_accepted).sum()),'historical_e_strong_fraction':float(g.historical_e_strong.mean()),'historical_e_admissible_fraction':float(g.exponential_locally_admissible.mean()),'rho_median':float(g.rho.median()),'rho_min':float(g.rho.min()),'rho_max':float(g.rho.max()),'prefix_jvp_median':float(g.prefix_jvp.median()),'prefix_jvp_p95':float(g.prefix_jvp.quantile(.95)),'prefix_jvp_max':float(g.prefix_jvp.max()),'prefix_over_target_r_jvp_median':float(g.prefix_jvp_over_target_r_jvp.median()),'prefix_over_target_r_jvp_p95':float(g.prefix_jvp_over_target_r_jvp.quantile(.95)),'prefix_over_full_e_jvp_median':float(g.prefix_jvp_over_full_e_jvp.median()),'prefix_over_full_e_jvp_p95':float(g.prefix_jvp_over_full_e_jvp.quantile(.95)),'trajectory_aggregate_prefix_over_r_jvp':float(t.prefix_jvp.sum()/t.r_jvp.sum()),'trajectory_aggregate_prefix_over_r_rhs':float(t.prefix_rhs.sum()/t.r_rhs.sum()),'debug_trajectory_aggregate_prefix_over_r_wall':float(t.debug_prefix_wall.sum()/t.debug_r_wall.sum())})
    dimdf=pd.DataFrame(dimrows); dimdf.to_csv(OUT/'K3_DIMENSION_SUMMARY.csv',index=False)
    fam=joined.groupby(['dimension','family']).agg(events=('family','size'),historical_e_strong_fraction=('historical_e_strong','mean'),historical_e_admissible_fraction=('exponential_locally_admissible','mean'),rho_median=('rho','median'),rho_max=('rho','max'),prefix_jvp_median=('prefix_jvp','median')).reset_index(); fam.to_csv(OUT/'K3_FAMILY_SUMMARY.csv',index=False)

    # Frozen-k1 counterexample rows at decision step 10.
    k1counter=k1[k1.decision_step==10].copy(); k1counter.to_csv(OUT/'FROZEN_K1_REJECTED_PROPOSAL_COUNTEREXAMPLES.csv',index=False)

    # Plot 1: actual level-1 rho vs historical full-E error at the same accepted target.
    fig,ax=plt.subplots(figsize=(7.2,4.8))
    for dim,g in joined.groupby('dimension'):
        ax.scatter(g.rho,g.exponential_total_error,label=f'N={dim}',alpha=.75)
    ax.axhline(1.0,linestyle='--',linewidth=1.2); ax.set_xscale('log'); ax.set_yscale('log'); ax.set_xlabel('Actual level-1 tolerance-scaled rho'); ax.set_ylabel('Historical full-E local error norm'); ax.set_title('Level-1 rho is informative but not a full-step safety certificate'); ax.grid(True,which='both',alpha=.25); ax.legend(); fig.tight_layout(); fig.savefig(PLOTS/'LEVEL1_RHO_VS_HISTORICAL_FULL_E_ERROR.png',dpi=180); plt.close(fig)

    # Plot 2: deterministic JVP burden relative to R and historical full E.
    fig,ax=plt.subplots(figsize=(7.2,4.8)); x=np.arange(2); width=.34
    vals=[]
    for dim in [128,512]:
        g=joined[joined.dimension==dim]; vals.append((g.prefix_jvp_over_target_r_jvp.median(),g.prefix_jvp_over_full_e_jvp.median()))
    ax.bar(x-width/2,[v[0] for v in vals],width,label='prefix / target R JVP'); ax.bar(x+width/2,[v[1] for v in vals],width,label='prefix / historical full-E JVP'); ax.set_xticks(x,['N=128','N=512']); ax.set_ylabel('Median JVP-vector ratio'); ax.set_title('Actual U2/D2 prefix has nonzero but bounded deterministic vector work'); ax.grid(True,axis='y',alpha=.25); ax.legend(); fig.tight_layout(); fig.savefig(PLOTS/'LEVEL1_PREFIX_JVP_COST.png',dpi=180); plt.close(fig)

    # Plot 3: family historical E precision of raw k3 events.
    fig,ax=plt.subplots(figsize=(8.0,4.8)); piv=fam.pivot(index='family',columns='dimension',values='historical_e_strong_fraction'); piv.plot(kind='bar',ax=ax); ax.set_ylim(0,1.05); ax.set_ylabel('Historical E-strong fraction'); ax.set_title('k=3 removes rejected-next targets but still has one HIRES E-inadmissible event'); ax.grid(True,axis='y',alpha=.25); fig.tight_layout(); fig.savefig(PLOTS/'K3_FAMILY_E_STRONG_PRECISION.png',dpi=180); plt.close(fig)

    # Plot 4: k1 rejected-proposal rho counterexample.
    fig,ax=plt.subplots(figsize=(6.8,4.5));
    if len(k1counter): ax.bar([f"N={int(n)}" for n in k1counter.dimension],k1counter.rho)
    ax.set_ylabel('Actual level-1 rho at first rejected proposal'); ax.set_title('Frozen k=1 causality miss carries a very large actual early defect'); ax.grid(True,axis='y',alpha=.25); fig.tight_layout(); fig.savefig(PLOTS/'K1_REJECTED_PROPOSAL_RHO.png',dpi=180); plt.close(fig)

    unsafe=joined[~joined.exponential_locally_admissible]
    result={'schema':'gvjf-policy-redesign-v27-actual-prefix-result-v1','contract_sha256':sha(CONTRACT),'new_solver_shards':14,'k3_full_family_shards':12,'frozen_k1_counterexample_shards':2,'full_e_continuations':0,'active_switching':False,'n2048_executed':False,'all_k3_prefixes_succeeded':bool(k3rows.prefix_success.all()),'all_k3_first_r_targets_accepted':bool(k3rows.target_r_accepted.all()),'rjf_exact_parity_all_12_k3_shards':bool(parity_df.exact_r_parity.all()),'dimension_summary':dimdf.to_dict('records'),'historical_k3_inadmissible_events':unsafe[['dimension','family','decision_step','target_step','rho','exponential_total_error']].to_dict('records'),'frozen_k1_rejected_proposal_counterexamples':k1counter[['dimension','family','decision_step','h','rho','target_r_error_norm']].to_dict('records'),'primary_verdict':'actual level-1 prefix mechanism survives; k3 fixes rejected-next causality but does not by itself certify full-E admissibility; proceed to staged level-2 safety research, not active switching'}
    (OUT/'RESULT_SUMMARY.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result,indent=2))
if __name__=='__main__': main()
