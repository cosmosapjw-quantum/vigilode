import json, numpy as np, statistics, hashlib
RUN='/home/cosmosapjw/.local/state/vigilode/adversarial-audit/20260927T121000Z'
new=json.load(open(f'{RUN}/exp/E-02/out/e0203_rows.json'))
def wrms_tight(y, ref):  # tight basis, matches Rust ReferenceWrmsBasis (abs 1e-10, rel 1e-8, anchored on reference)
    w=1e-10+1e-8*np.abs(ref); return np.sqrt(np.mean(((y-ref)/w)**2))
def wrms_case(y, ref, atol, rtol):
    w=atol+rtol*np.abs(ref); return np.sqrt(np.mean(((y-ref)/w)**2))
out=[]; 
for r in new['rows']:
    art=json.load(open(f"{RUN}/exp/E-02/refdir/artifacts/{r['case_id'].split('-rtol-')[0].replace('-n96','-n96')}-v2.json")) if False else None
    arms={x['policy']['name']:x for x in r['arms']}
    # reference states via manifest binding: artifact file by problem_id
    row={'case_id':r['case_id'],'family':r['family'],'rtol':r['rtol'],'atol':r['atol'],'ref_unc':r['reference_uncertainty_wrms'],'arms':{},'pairs':[]}
    out.append((r,row,arms))
# map case -> artifact
man=json.load(open(f'{RUN}/exp/E-02/refdir/reference_manifest.json'))
bind={b['case_id']:b['problem_id'] for b in man['bindings']}
arts={}
for e in man['artifacts']:
    arts[e['problem']['problem_id']]=e['artifact_path']
res_rows=[]
for r,row,arms in out:
    pid=bind[r['case_id']]; a=json.load(open(f"{RUN}/exp/E-02/refdir/{arts[pid]}"))
    ref=np.array(a['states']); tg=np.array(a['requested_times'])
    assert np.array_equal(tg, np.array(arms['dense_base']['grid_times']))
    for name,arm in arms.items():
        Y=np.array(arm['grid_states'])
        wt=np.array([wrms_tight(Y[i],ref[i]) for i in range(len(tg))])
        wc=np.array([wrms_case(Y[i],ref[i],r['atol'],r['rtol']) for i in range(len(tg))])
        assert abs(wt.max()-arm['metrics']['max_grid_wrms'])<=1e-9*wt.max()
        row['arms'][name]={'steps':arm['internal_steps'],'max_grid_wrms_tight':float(wt.max()),'endpoint_wrms_tight':float(wt[-1]),
            'argmax_grid_index':int(wt.argmax()),'max_grid_wrms_casetol':float(wc.max()),'endpoint_wrms_casetol':float(wc[-1]),
            'solver_max_accepted_norm':arm['max_accepted_error_norm'],
            'reference_admissible': not (r['reference_uncertainty_wrms']>0.1*wt.max()),
            'grid_wrms_tight':wt.tolist()}
    for L,R,D in [('dense_base','dense_h0x0.7','dense_base'),('dense_base','dense_maxstep_grid','dense_base'),('dense_h0x0.7','dense_maxstep_grid','dense_h0x0.7'),('clipped_base','clipped_h0x0.7','clipped_base'),('clipped_base','dense_base','dense_base')]:
        YL=np.array(arms[L]['grid_states']); YR=np.array(arms[R]['grid_states'])
        gap=max(wrms_tight(YL[i],YR[i]) if False else np.sqrt(np.mean(((YL[i]-YR[i])/(1e-10+1e-8*np.abs(ref[i])))**2)) for i in range(len(tg)))
        # cross-check with Rust pair if present
        rp=[p for p in r['pairs'] if p['left']==L and p['right']==R]
        if rp: assert abs(rp[0]['gap_wrms']-gap)<=1e-9*max(gap,1e-300), (rp[0]['gap_wrms'],gap)
        dm=row['arms'][D]['max_grid_wrms_tight']
        row['pairs'].append({'left':L,'right':R,'denominator':D,'gap_wrms_tight':float(gap),'denominator_max_grid_wrms':dm,'ratio':float(gap/dm),
            'violates_0p1_rule': bool(gap>0.1*dm),'both_reference_admissible': bool(row['arms'][L]['reference_admissible'] and row['arms'][R]['reference_admissible']),
            'both_solver_norm_le_1': bool(row['arms'][L]['solver_max_accepted_norm']<=1 and row['arms'][R]['solver_max_accepted_norm']<=1),
            'both_casetol_max_le_1': bool(row['arms'][L]['max_grid_wrms_casetol']<=1 and row['arms'][R]['max_grid_wrms_casetol']<=1)})
    res_rows.append(row)
def tally(L,R,key='both_reference_admissible'):
    ps=[p for row in res_rows for p in row['pairs'] if p['left']==L and p['right']==R]
    bc=[p for p in ps if p[key]]
    return {'pairs':len(ps),'both_correct':len(bc),'violations_among_both_correct':sum(p['violates_0p1_rule'] for p in bc),
            'fraction_violating':(sum(p['violates_0p1_rule'] for p in bc)/len(bc) if bc else None),
            'median_ratio':statistics.median(p['ratio'] for p in ps),'min_ratio':min(p['ratio'] for p in ps),'max_ratio':max(p['ratio'] for p in ps)}
summary={'h0_pair_refadmissible':tally('dense_base','dense_h0x0.7'),'maxstep_pair_refadmissible':tally('dense_base','dense_maxstep_grid'),
 'h0_pair_casetol_correct':tally('dense_base','dense_h0x0.7','both_casetol_max_le_1'),'maxstep_pair_casetol_correct':tally('dense_base','dense_maxstep_grid','both_casetol_max_le_1'),
 'clipped_h0_pair':tally('clipped_base','clipped_h0x0.7'),'campaign_pair':tally('clipped_base','dense_base'),
 'arms_reference_admissible':sum(a['reference_admissible'] for row in res_rows for a in row['arms'].values()),'arms_total':sum(len(row['arms']) for row in res_rows),
 'dense_base_casetol_max_le_1':sum(row['arms']['dense_base']['max_grid_wrms_casetol']<=1 for row in res_rows),
 'clipped_base_casetol_max_le_1':sum(row['arms']['clipped_base']['max_grid_wrms_casetol']<=1 for row in res_rows),
 'dense_base_endpoint_vs_max_ratio_median':statistics.median(row['arms']['dense_base']['max_grid_wrms_tight']/row['arms']['dense_base']['endpoint_wrms_tight'] for row in res_rows),
 'dense_base_max_over_endpoint_gt5':sum(row['arms']['dense_base']['max_grid_wrms_tight']/row['arms']['dense_base']['endpoint_wrms_tight']>5 for row in res_rows)}
json.dump({'summary':summary,'rows':res_rows},open(f'{RUN}/exp/E-03/e03a_rows.json','w'),indent=1)
print(json.dumps(summary,indent=1))
print("case | rtol | dense_base steps/maxT/maxCase/solv | h0x0.7 gap ratio | maxstep gap ratio | endpoint/max(dense)")
for row in res_rows:
    d=row['arms']['dense_base']; p1=row['pairs'][0]; p2=row['pairs'][1]
    print(f"{row['case_id'][:52]:52s} {row['rtol']:.0e} steps={d['steps']:4d} maxT={d['max_grid_wrms_tight']:9.3g} maxCase={d['max_grid_wrms_casetol']:8.3g} solv={d['solver_max_accepted_norm']:.2f} | r_h0={p1['ratio']:7.3f} adm={p1['both_reference_admissible']} | r_ms={p2['ratio']:7.3f} adm={p2['both_reference_admissible']} | max/end={d['max_grid_wrms_tight']/d['endpoint_wrms_tight']:8.2f} argmax={d['argmax_grid_index']}")
