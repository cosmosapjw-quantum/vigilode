#!/usr/bin/env python3
"""Read-only re-analysis plus independent decimal-tableau convergence probes.
No new calibration verdict, production route, freeze, holdout, or timing claim.
Python >=3.10; NumPy, mpmath. Existing inputs are never modified.
"""
from __future__ import annotations
import argparse,csv,hashlib,json,math,platform
from pathlib import Path
import numpy as np
import mpmath as mp
ROOT=Path(__file__).resolve().parents[1]
BUNDLE=ROOT/'research/scientific_validity_v2_20260829/external_reaudit_bundle'

def load(p):
    return json.loads(p.read_text(),parse_constant=lambda s:(_ for _ in ()).throw(ValueError(s)))
def dump(p,obj):p.write_text(json.dumps(obj,indent=2,allow_nan=False)+'\n')
def csvout(p,rows):
    with p.open('w',newline='') as f:
        w=csv.DictWriter(f,list(rows[0]));w.writeheader();w.writerows(rows)
def interval(error,u,budget=None):
    if any(type(v) not in (int,float) or not math.isfinite(v) or v<0 for v in [error,u]+([] if budget is None else [budget])):raise ValueError('finite nonnegative inputs required')
    lower=max(error-u,0.);upper=error+u
    if not math.isfinite(upper):raise ValueError('overflow')
    verdict='budget-not-specified' if budget is None else 'within-budget' if upper<=budget else 'outside-budget' if lower>budget else 'reference-unresolved'
    return dict(lower=lower,upper=upper,budget=budget,verdict=verdict)

def reanalyse(out):
    rows=[]
    for record in load(BUNDLE/'rust/calibration_all_cases_compact.json')['records']:
        a=record['artifact'];s=a['spec'];c=a['clipped'];d=a['dense']
        ec=c['metrics']['max_grid_wrms'];ed=d['metrics']['max_grid_wrms'];gap=a['output_policy_discrepancy_wrms']
        jc=c['counters']['jvp_vectors'];jd=d['counters']['jvp_vectors']
        rows.append(dict(case_id=s['id'],family=s['family'],dimension=s['dimension'],rtol=s['rtol'],atol=s['atol'],
          clipped_error_wrms=ec,dense_error_wrms=ed,trajectory_discrepancy_wrms=gap,scalar_error_gap=abs(ec-ed),
          discrepancy_over_dense=gap/ed,clipped_jvp=jc,dense_jvp=jd,jvp_relative_gap=abs(jc-jd)/jd,
          legacy_status=a['row']['status'],accuracy_verdict=interval(ed,a['reference_uncertainty_wrms'])['verdict']))
    assert len(rows)==54 and len({(r['family'],r['dimension'],r['rtol']) for r in rows})==54
    assert sorted({r['dimension'] for r in rows})==[96,384,1536]
    assert all(r['legacy_status']=='output-policy-dominated' for r in rows)
    csvout(out/'campaign_54_reanalysis.csv',rows)
    response=[]
    for family,n in sorted({(r['family'],r['dimension']) for r in rows}):
        subset=sorted([r for r in rows if r['family']==family and r['dimension']==n],key=lambda r:-r['rtol'])
        for c,f in zip(subset,subset[1:]):
            ratio=c['dense_error_wrms']/f['dense_error_wrms']
            response.append(dict(family=family,dimension=n,rtol_coarse=c['rtol'],rtol_fine=f['rtol'],error_ratio=ratio,tolerance_response_exponent=math.log(ratio)/math.log(c['rtol']/f['rtol']),method_order='NOT_INFERRED_FROM_RTOL'))
    csvout(out/'tolerance_response.csv',response)
    result=dict(cases=54,dimensions=[96,384,1536],historical_statuses_unchanged=True,
      proposed_jvp_5pct_rejections=sum(r['jvp_relative_gap']>.05 for r in rows),
      jvp_gap_range=[min(r['jvp_relative_gap'] for r in rows),max(r['jvp_relative_gap'] for r in rows)],
      discrepancy_over_dense_range=[min(r['discrepancy_over_dense'] for r in rows),max(r['discrepancy_over_dense'] for r in rows)],
      accuracy_budget='NOT_SPECIFIED_NO_NEW_PASS',holdout_opened=False,freeze_created=False)
    dump(out/'campaign_summary.json',result)
    return result

def reconstruct_six(out):
    rows=[]
    for record in load(BUNDLE/'rust/selected_raw_n96_rtol1e-8.json')['records']:
        a=record['artifact'];s=a['spec'];ref=load(BUNDLE/'reference/selected_raw'/(a['reference']['problem_id']+'.json'))
        y=np.array(ref['states']);c=np.array(a['clipped']['states']);d=np.array(a['dense']['states'])
        assert c.shape==d.shape==y.shape
        assert np.array_equal(a['clipped']['output_times'],ref['requested_times'])
        assert np.array_equal(a['dense']['output_times'],ref['requested_times'])
        scale=1e-10+1e-8*np.abs(y)
        metric=lambda z:float(np.max(np.sqrt(np.mean((z/scale)**2,axis=1))))
        ec,ed,gap=metric(c-y),metric(d-y),metric(c-d)
        expected=[a['clipped']['metrics']['max_grid_wrms'],a['dense']['metrics']['max_grid_wrms'],a['output_policy_discrepancy_wrms']]
        for x,v in zip([ec,ed,gap],expected):
            # Same input/state/norm; only a different length-n reduction order.
            assert abs(x-v)<=128*np.finfo(float).eps*max(abs(x),abs(v),np.finfo(float).tiny)
        rows.append(dict(case_id=s['id'],clipped_error_wrms=ec,dense_error_wrms=ed,direct_gap_wrms=gap,metric_check='PASS'))
    assert len(rows)==6
    csvout(out/'selected_six_metric_reconstruction.csv',rows)
    return dict(full_state_cases=6,status='PASS',other_48_states='not_in_repository; compact metrics only')

class ScalarRodas:
    def __init__(self,snapshot):
        mp.mp.dps=70;self.gamma=mp.mpf(snapshot['gamma'])
        matrix=lambda rows:mp.matrix([[mp.mpf(x) for x in row] for row in rows])
        A=matrix(snapshot['A']);C=matrix(snapshot['C']);H=matrix(snapshot['H'])
        self.G=(mp.eye(8)/self.gamma-C)**-1;self.alpha=A*self.G;self.D=H*self.G
        self.b=self.G.T*mp.matrix([mp.mpf(v) for v in snapshot['b_code']])
        self.c=[mp.mpf(v) for v in snapshot['c']];self.gr=[sum(self.G[i,j] for j in range(8)) for i in range(8)]
    def step(self,t,y,h,kind):
        if kind=='exponential':rhs=lambda t,v:-v;jac=-mp.mpf(1);ft=mp.mpf(0)
        else:
            lam=mp.mpf(2 if kind=='manufactured_nonlinear' else 1000);nu=mp.mpf('.25');g=mp.exp(-t)
            rhs=lambda s,v:-lam*(v-mp.exp(-s))-mp.exp(-s)+nu*(v-mp.exp(-s))**2
            jac=-lam+2*nu*(y-g);ft=(1-lam)*g+2*nu*(y-g)*g
        w=1-h*self.gamma*jac;k=[]
        for i in range(8):
            yi=y+sum(self.alpha[i,j]*k[j] for j in range(i));gi=sum(self.G[i,j]*k[j] for j in range(i))
            k.append((h*rhs(t+self.c[i]*h,yi)+h*jac*gi+h*h*self.gr[i]*ft)/w)
        return y+sum(self.b[j]*k[j] for j in range(8)),k
    def sample(self,y,yn,k,theta):
        d=[sum(self.D[i,j]*k[j] for j in range(8)) for i in range(3)]
        return (1-theta)*y+theta*(yn+(1-theta)*(d[0]+theta*(d[1]+theta*d[2])))

def probe(out):
    method=ScalarRodas(load(ROOT/'fixtures/rodas5p_coefficients_snapshot.json'))
    local=[];glob=[];split=[];theta=mp.mpf('.37');t0=mp.mpf('.3')
    for kind in ('exponential','manufactured_nonlinear','stiff_manufactured'):
        for n in (4,8,16,32,64,128):
            h=mp.mpf(1)/n;y=mp.exp(-t0);yn,k=method.step(t0,y,h,kind);exact=mp.exp(-(t0+theta*h))
            local.append(dict(problem=kind,n=n,h=float(h),theta=float(theta),endpoint_error=float(abs(yn-mp.exp(-(t0+h)))),dense_error=float(abs(method.sample(y,yn,k,theta)-exact)),linear_mutant_error=float(abs((1-theta)*y+theta*yn-exact))))
            y=mp.mpf(1);dm=lm=im=nm=identity=mp.mpf(0)
            for j in range(n):
                t=j*h;yn,k=method.step(t,y,h,kind);exact=mp.exp(-(t+theta*h));v=method.sample(y,yn,k,theta)
                if kind=='exponential':flow=y*mp.exp(-theta*h)
                else:
                    lam=mp.mpf(2 if kind=='manufactured_nonlinear' else 1000);dev=y-mp.exp(-t);decay=mp.exp(-lam*theta*h)
                    flow=exact+dev*decay/(1-mp.mpf('.25')*dev*(1-decay)/lam)
                interp=v-flow;inherited=flow-exact
                identity=max(identity,abs((v-exact)-interp-inherited));im=max(im,abs(interp));nm=max(nm,abs(inherited))
                dm=max(dm,abs(v-exact));lm=max(lm,abs((1-theta)*y+theta*yn-exact));y=yn
            glob.append(dict(problem=kind,n=n,h=float(h),theta=float(theta),endpoint_error=float(abs(y-mp.exp(-1))),dense_error=float(dm),linear_mutant_error=float(lm)))
            split.append(dict(problem=kind,n=n,interpolation_defect=float(im),propagated_node_error=float(nm),total_dense_error=float(dm),signed_decomposition_residual=float(identity)))
    slopes=[]
    for scope,rows in [('local_exact_start',local),('global_interval',glob)]:
        for kind in ('exponential','manufactured_nonlinear','stiff_manufactured'):
            subset=[r for r in rows if r['problem']==kind]
            for a,b in zip(subset,subset[1:]):
                slopes.append(dict(scope=scope,problem=kind,n_from=a['n'],n_to=b['n'],**{field+'_slope':math.log2(a[field+'_error']/b[field+'_error']) for field in ['endpoint','dense','linear_mutant']}))
    for name,rows in [('local_exact_start',local),('global_interval',glob),('refinement_slopes',slopes),('exact_flow_decomposition',split)]:csvout(out/(name+'.csv'),rows)
    result=dict(precision_digits=70,polynomial_degree=4,fourth_derivative='-24*d2',expected_nonstiff_local={'endpoint':6,'dense':5,'linear_mutant':2},expected_nonstiff_global={'endpoint':5,'dense':5,'linear_mutant':2},stiff_order='not uniform; retained as stress data',roundoff='rounded input coefficients retained; do not interpret every tiny-h slope as asymptotic order')
    dump(out/'probe_summary.json',result);return result

def main():
    ap=argparse.ArgumentParser();ap.add_argument('--output',type=Path,required=True);ap.add_argument('--skip-probe',action='store_true');args=ap.parse_args()
    out=args.output.resolve()
    if out.exists():raise SystemExit('Choose a fresh output directory; do not overwrite evidence')
    out.mkdir(parents=True)
    inputs=[ROOT/'fixtures/rodas5p_coefficients_snapshot.json',BUNDLE/'rust/calibration_all_cases_compact.json',BUNDLE/'rust/selected_raw_n96_rtol1e-8.json']
    hashes=lambda:{str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest() for p in inputs}
    before=hashes();result=dict(status='EXPLORATORY_NONAUTHORITATIVE',python=platform.python_version(),campaign=reanalyse(out),selected_slice=reconstruct_six(out))
    if not args.skip_probe:result['probe']=probe(out)
    assert hashes()==before,'immutable input changed'
    result.update(inputs_preserved=before,freeze_created=False,holdout_opened=False)
    dump(out/'run_summary.json',result);print(json.dumps(result,indent=2))
if __name__=='__main__':main()
