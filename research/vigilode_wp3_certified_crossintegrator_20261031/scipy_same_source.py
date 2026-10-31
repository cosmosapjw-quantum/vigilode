#!/usr/bin/env python3
"""Research-only original cooperative cubic IVPs, scipy independent integrator results.
NOT RODAS5P. Source is immutable WP1 binary64 and WP2 continuous defect certified reference.
"""
import csv, hashlib, json, sys, time, struct
from fractions import Fraction
from pathlib import Path
import numpy as np
from scipy.integrate import solve_ivp

ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'sources'))
import DEFECT_CERTIFIER as cert
METHODS=('RK45','DOP853','Radau','BDF','LSODA')
RTOLS=(1e-4,1e-6,1e-8,1e-10)


def reference(cohort, source, steps, filename):
    data=(ROOT/'sources'/filename).read_bytes()
    width=8*source['n'];expected=(steps*64+1)*width
    assert len(data)==expected,(cohort,len(data),expected)
    raw=[struct.unpack('>'+('d'*source['n']),data[j*width:(j+1)*width]) for j in range(0,steps*64+1,64)]
    return np.asarray(raw,dtype=np.float64)


def make_rhs(src):
    k=np.asarray([float(x) for x in src['k']],dtype=np.float64)
    g=np.asarray([float(x) for x in src['g']],dtype=np.float64)
    c=np.asarray([[float(x) for x in row] for row in src['c']],dtype=np.float64)
    C=np.sum(c,axis=1)
    def fun(t,y):
        return -k*y-g*y*y*y+c @ y-C*y
    def jac(t,y):
        return c-np.diag(k+3*g*y*y+C)
    return fun,jac


def exact_error(y,ref):
    # Exact dyadic comparison of binary64 outputs to certified frozen reference states.
    worst=Fraction(0)
    for a,b in zip(y.flat,ref.flat):
        worst=max(worst,abs(Fraction.from_float(float(a))-Fraction.from_float(float(b))))
    return worst


def run():
    assert hashlib.sha256((ROOT/'sources/SWEEP.log').read_bytes()).hexdigest()==cert.EXPECTED_SWEEP_SHA256
    assert hashlib.sha256((ROOT/'sources/WP_SUMMARY.json').read_bytes()).hexdigest()==cert.EXPECTED_WP1_SUMMARY_SHA256
    traces,_=cert.parse_log(ROOT/'sources/SWEEP.log')
    rawcert=json.loads((ROOT/'sources/CERTIFICATE.json').read_text())
    results=[]
    source_witness={}
    for cohort in cert.COHORTS:
        baseline=traces[(cohort,2,False)]
        source=cert.parse_source(baseline)
        n=len(baseline['points'])-1
        assert n in (64,128)
        ys=reference(cohort,source,n,cohort+'_micro64_reference_state_bits.bin')
        initial=ys[0]; assert all(Fraction.from_float(float(v))==s for v,s in zip(initial,source['y0']))
        t_eval=np.array([cert.bits_to_f(p['time']) for p in baseline['points']],dtype=float)
        assert np.all(np.diff(t_eval)>0) and t_eval[0]==0 and t_eval[-1]==.1
        R=next(x for x in rawcert['micro_cohorts'][cohort] if x['reference_micro']==64)
        digest=hashlib.sha256((ROOT/'sources'/R['reference_states_name']).read_bytes()).hexdigest()
        assert digest==R['reference_states_sha256']
        eps=Fraction.from_float(float(R['total_reference_plus_clock_delta_up']))
        fun,jac=make_rhs(source)
        J_ind=np.empty((source['n'],source['n']),float)
        for i in range(source['n']):
            for j in range(source['n']):
                J_ind[i,j]=float(source['c'][i][j]) if i!=j else (-float(source['k'][i])-3*float(source['g'][i])*initial[i]**2-sum(map(float,source['c'][i])))
        jac_gap=float(np.linalg.norm(jac(0,initial)-J_ind,ord=np.inf))
        assert jac_gap<5e-14
        source_witness[cohort]={'source_bits_sha256':hashlib.sha256(json.dumps(baseline['source_bits']).encode()).hexdigest(),'steps':n,'dimension':source['n'],'time_last':float(t_eval[-1]),'jacobian_identity_gap':jac_gap,'wp2_error_enclosure':float(eps),'reference_bits_sha256':digest}
        for method in METHODS:
            for rtol in RTOLS:
                atol=rtol*1e-2
                out=None; durations=[]
                for repeat in range(3):
                    start=time.perf_counter_ns()
                    opts={'jac':jac} if method in ('Radau','BDF','LSODA') else {}
                    sol=solve_ivp(fun,(float(t_eval[0]),float(t_eval[-1])),initial,method=method,t_eval=t_eval,rtol=rtol,atol=atol,**opts)
                    dur=time.perf_counter_ns()-start
                    assert sol.success and len(sol.t)==len(t_eval) and np.array_equal(sol.t,t_eval),(cohort,method,rtol,sol.message)
                    assert np.isfinite(sol.y).all()
                    assert sol.y.min()>=-1e-11,'source positivity violation'
                    durations.append(dur)
                    if repeat==0:out=sol
                assert out is not None
                vals=np.asarray(out.y.T)
                observed=exact_error(vals,ys)
                lower=max(Fraction(0),observed-eps)
                upper=observed+eps
                results.append({'cohort':cohort,'integrator':method,'rtol':rtol,'atol':atol,'steps_observed':n,'time_end':float(t_eval[-1]),
                    'observed_peak_error':float(observed),'rigorous_peak_error_lower':float(lower),
                    'rigorous_peak_error_upper':float(upper),'relative_uncertainty':float(2*eps/observed) if observed else None,
                    'nfev':int(out.nfev),'njev':int(out.njev or 0),'nlu':int(out.nlu or 0),'wall_ns_median':int(np.median(durations)),
                    'wall_ns_raw':durations,'source_initial_same':bool(np.array_equal(vals[0],initial)),
                    'configured_source_initial_exact':bool(all(Fraction.from_float(float(x))==y for x,y in zip(initial,source['y0']))),
                    'returned_initial_ulp_max':int(np.max(np.abs(vals[0].view(np.int64)-initial.view(np.int64)))),
                    'source_positive':bool(np.min(vals)>=-1e-11),'validation':'ORIGINAL_SAME_CUBIC_SOURCE_CERTIFIED_REFERENCE_NONRODAS'})
        print('COHORT',cohort,'runs',len([r for r in results if r['cohort']==cohort]),'referenceδ',float(eps),flush=True)
    target=ROOT/'results';target.mkdir(exist_ok=True)
    with (target/'SCIPY_SAME_SOURCE.json').open('w') as fh:
        json.dump({'schema':'vigilode.wp3.scipy_original_cubic.v1','input_head':'cf690ad5ec723052e9eb6f71335f7cb3b43607a6',
                   'frozen_wp1_sha256':cert.EXPECTED_SWEEP_SHA256,'source_witnesses':source_witness,
                   'scope':'new SciPy executables only; exact dyadic achieved error intervals vs fixed WP2 micro64 reference; NOT native RODAS8; CPU wall time not compared to GitHub native runner',
                   'results':results},fh,indent=2)
    with (target/'SCIPY_SAME_SOURCE.csv').open('w',newline='') as fh:
        fields=[f for f in results[0] if f!='wall_ns_raw'];wr=csv.DictWriter(fh,fields);wr.writeheader();wr.writerows([{k:v for k,v in row.items() if k in fields} for row in results])
    print('RESULTS',len(results),'records',sum(r['rigorous_peak_error_upper']<1e-6 for r in results),'upper error under 1e-6',flush=True)
    return results
if __name__=='__main__':run()
