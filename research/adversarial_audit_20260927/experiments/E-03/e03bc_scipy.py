"""E-03b: solver-independent control (scipy Radau, two admissible policies) and
E-03c: accepted-endpoint vs interpolated-grid attribution (VigilODE dense_base + scipy)."""
import json, sys, time, math, os
import numpy as np, scipy
from scipy.integrate import solve_ivp
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import families
RUN='/home/cosmosapjw/.local/state/vigilode/adversarial-audit/20260927T121000Z'
OUT=f'{RUN}/exp/E-03/scipy'
man=json.load(open(f'{RUN}/exp/E-02/refdir/reference_manifest.json'))
arts={e['problem']['problem_id']:e['artifact_path'] for e in man['artifacts']}
bind={b['case_id']:b['problem_id'] for b in man['bindings']}
rows=json.load(open(f'{RUN}/exp/E-02/out/e0203_rows.json'))['rows']
def wrms_rows(Y, REF, absw=1e-10, relw=1e-8):
    W=absw+relw*np.abs(REF); return np.sqrt(np.mean(((Y-REF)/W)**2, axis=1))
def wrms_rows_case(Y, REF, atol, rtol): return wrms_rows(Y, REF, atol, rtol)
def run(rhs, jac, y0, span, rtol, atol, **kw):
    t0=time.perf_counter(); r=solve_ivp(rhs, span, y0, method='Radau', rtol=rtol, atol=atol, jac=jac, **kw)
    return r, time.perf_counter()-t0
fams=sys.argv[1:] or list(families.FAMILIES)
for fam in fams:
    rt,span=families.FAMILIES[fam]; rhs,jac,y0,_=rt(96)
    frows=[r for r in rows if r['family']==fam]; pid=bind[frows[0]['case_id']]
    art=json.load(open(f"{RUN}/exp/E-02/refdir/{arts[pid]}")); grid=np.array(art['requested_times']); REF=np.array(art['states'])
    ref_unc=art['convergence']['reference_uncertainty_wrms']
    rep={'family':fam,'problem_id':pid,'scipy':scipy.__version__,'numpy':np.__version__,'t_span':span,'reference_uncertainty_wrms':ref_unc,'tight':{},'rtol_rows':[]}
    print(f'== {fam}', flush=True)
    # tight references with dense output (L2 = canonical level, L1 = one level coarser for interpolant uncertainty)
    L2,w2=run(rhs,jac,y0,span,1e-12,1e-14,dense_output=True,t_eval=grid); L1,w1=run(rhs,jac,y0,span,1e-10,1e-12,dense_output=True,t_eval=grid)
    assert L2.success and L1.success, (L2.message, L1.message)
    Y2=L2.y.T; Y1=L1.y.T
    rep['tight']={'L2':{'wall':w2,'nfev':int(L2.nfev),'njev':int(L2.njev),'nlu':int(L2.nlu),'steps':int(len(L2.t_events) if False else len(L2.sol.ts)-1),
        'max_grid_wrms_vs_artifact':float(wrms_rows(Y2,REF).max()),'bitwise_equal_grid_points':int(sum(np.array_equal(Y2[i],REF[i]) for i in range(len(grid))))},
        'L1':{'wall':w1,'nfev':int(L1.nfev),'steps':int(len(L1.sol.ts)-1),'max_grid_wrms_vs_L2':float(wrms_rows(Y1,Y2,).max()),'artifact_d1':art['convergence']['d1_max_grid_wrms']}}
    print('  tight', json.dumps(rep['tight']), flush=True)
    h0=(span[1]-span[0])/100.0; hgrid=h0
    for r in frows:
        rtol=r['rtol']; atol=r['atol']; rr={'case_id':r['case_id'],'rtol':rtol,'atol':atol,'arms':{},'pairs':[],'vigilode_attribution':{}, 'scipy_attribution':{}}
        arms={}
        specs={'A_h0_dense':dict(first_step=h0,dense_output=True),'B_h0x0.7_dense':dict(first_step=0.7*h0,dense_output=True),
               'C_h0_teval':dict(first_step=h0,t_eval=grid),'D_h0_maxstep_grid_dense':dict(first_step=h0,max_step=hgrid,dense_output=True)}
        for name,kw in specs.items():
            res,w=run(rhs,jac,y0,span,rtol,atol,**kw)
            if not res.success: arms[name]={'status':'failure','message':res.message}; continue
            Y = res.sol(grid).T if 'dense_output' in kw else res.y.T
            wt=wrms_rows(Y,REF); wc=wrms_rows_case(Y,REF,atol,rtol)
            a={'status':'success','wall':w,'nfev':int(res.nfev),'njev':int(res.njev),'nlu':int(res.nlu),'steps':int(len(res.t)-1) if 'dense_output' not in kw else int(len(res.sol.ts)-1),
               'max_grid_wrms_tight':float(wt.max()),'endpoint_wrms_tight':float(wt[-1]),'max_grid_wrms_casetol':float(wc.max()),'endpoint_wrms_casetol':float(wc[-1]),
               'reference_admissible': not (ref_unc>0.1*wt.max()),'grid_wrms_tight':wt.tolist()}
            if 'dense_output' in kw:
                ts=res.sol.ts; Ya=res.sol(ts).T; R2=L2.sol(ts).T; R1=L1.sol(ts).T
                ea=wrms_rows(Ya,R2); unc=wrms_rows(R1,R2)
                interior=np.array([not any(abs(g-t)<=64*np.finfo(float).eps*max(abs(g),abs(t),1.0) for t in ts) for g in grid])
                a['accepted_endpoints']={'count':int(len(ts)),'max_wrms_tight':float(ea.max()),'median_wrms_tight':float(np.median(ea)),'ref_interp_uncertainty_max':float(unc.max()),
                    'grid_interior_count':int(interior.sum()),'grid_interior_max_wrms_tight':float(wt[interior].max()) if interior.any() else None,'grid_interior_median_wrms_tight':float(np.median(wt[interior])) if interior.any() else None,
                    'ratio_grid_interior_max_over_accepted_max':float(wt[interior].max()/ea.max()) if interior.any() and ea.max()>0 else None}
            arms[name]=a; a['_Y']=Y
            print(f"  {r['case_id'][-14:]} {name:24s} steps={a['steps']:5d} maxT={a['max_grid_wrms_tight']:.4g} maxCase={a['max_grid_wrms_casetol']:.3g}", flush=True)
        for L,Rn in [('A_h0_dense','B_h0x0.7_dense'),('A_h0_dense','C_h0_teval'),('A_h0_dense','D_h0_maxstep_grid_dense')]:
            if arms.get(L,{}).get('status')!='success' or arms.get(Rn,{}).get('status')!='success': continue
            gap=float(wrms_rows(arms[L]['_Y'],arms[Rn]['_Y']).max()); dm=arms[L]['max_grid_wrms_tight']
            rr['pairs'].append({'left':L,'right':Rn,'gap_wrms_tight':gap,'denominator_max_grid_wrms':dm,'ratio':gap/dm if dm>0 else None,'violates_0p1_rule':bool(gap>0.1*dm),
                'both_reference_admissible':bool(arms[L]['reference_admissible'] and arms[Rn]['reference_admissible']),
                'bitwise_identical_grid_states':bool(np.array_equal(arms[L]['_Y'],arms[Rn]['_Y']))})
        for a in arms.values(): a.pop('_Y',None)
        rr['arms']=arms
        # E-03c VigilODE attribution: exact accepted-step states from the union-schedule run vs tight dense reference
        va={x['policy']['name']:x for x in r['arms']}
        u=va.get('dense_base_union_accepted'); d=va['dense_base']
        if u and u['status']=='success' and u['extra_times']:
            ts=np.array(u['extra_times']); Ya=np.array(u['extra_states']); R2=L2.sol(ts).T; R1=L1.sol(ts).T
            ea=wrms_rows(Ya,R2); unc=wrms_rows(R1,R2)
            Yg=np.array(d['grid_states']); wt=wrms_rows(Yg,REF)
            acc_all=np.array(u['extra_times']+[span[1]])
            interior=np.array([not any(abs(g-t)<=64*np.finfo(float).eps*max(abs(g),abs(t),1.0) for t in acc_all) for g in grid])
            # grid points coinciding with accepted endpoints (exact states at theta==1) -- includes t_f
            coinc=~interior; coinc[0]=False
            rr['vigilode_attribution']={'accepted_endpoint_count_interior':int(len(ts)),'accepted_max_wrms_tight':float(ea.max()),'accepted_median_wrms_tight':float(np.median(ea)),
                'endpoint_tf_wrms_tight':float(wt[-1]),'ref_interp_uncertainty_max':float(unc.max()),'ref_interp_uncertainty_median':float(np.median(unc)),
                'grid_interior_count':int(interior.sum()),'grid_interior_max_wrms_tight':float(wt[interior].max()),'grid_interior_median_wrms_tight':float(np.median(wt[interior])),
                'grid_coincident_count':int(coinc.sum()),'grid_coincident_max_wrms_tight':float(wt[coinc].max()) if coinc.any() else None,
                'ratio_grid_interior_max_over_accepted_max':float(wt[interior].max()/max(ea.max(),wt[-1])),
                'ratio_grid_interior_max_over_accepted_all_max':float(wt[interior].max()/max(ea.max(),wt[-1])),
                'union_same_step_sequence': u['diagnostics']['accepted_step_sizes']==d['diagnostics']['accepted_step_sizes'],
                'accepted_wrms_tight':ea.tolist(),'accepted_times':ts.tolist()}
            print(f"  {r['case_id'][-14:]} VIGILODE accepted max={ea.max():.4g} (unc {unc.max():.2g}) tf={wt[-1]:.4g} | grid-interior max={wt[interior].max():.4g} ratio={rr['vigilode_attribution']['ratio_grid_interior_max_over_accepted_max']:.2f}", flush=True)
        rep['rtol_rows'].append(rr)
        json.dump(rep,open(f'{OUT}/{fam}.json','w'),indent=1)
    json.dump(rep,open(f'{OUT}/{fam}.json','w'),indent=1)
print('DONE', flush=True)
