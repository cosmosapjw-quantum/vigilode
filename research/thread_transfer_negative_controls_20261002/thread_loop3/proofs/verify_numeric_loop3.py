#!/usr/bin/env python3
"""High-precision probes with independently specified exact solutions.
The two-step no-go, long-time chart repair, matrix PR bound, and estimator
counterexample are separate tests. Runtime below is NOT a performance benchmark.
"""
from __future__ import annotations
import json,time,platform
from pathlib import Path
import sympy as s
import mpmath as mp
import numpy as np
from reference_rvj import AnalyticRVJ, slow_step, R5, phi5
ROOT=Path(__file__).resolve().parents[1]; OUT=ROOT/'results'; OUT.mkdir(exist_ok=True)
mp.mp.dps=100; started=time.perf_counter(); checks=[]
def text(x):return mp.nstr(x,40)
def check(name,condition,**data):
    ok=bool(condition); checks.append({'name':name,'passed':ok,**data})
    if not ok:
        (OUT/'numeric_failure.json').write_text(json.dumps(checks,indent=2,default=str))
        raise AssertionError(name)
def norm(x):return mp.sqrt(sum(abs(v)**2 for v in x))
x,y,k=s.symbols('x y k'); f=s.Matrix([x*x,(-k+2*x)*y+x*x])
raw=AnalyticRVJ((x,y),(k,),f)
no_go=[]
for N in [16,32,64,128,256]:
    h=mp.mpf(1)/N; kk=h**-6; st=mp.matrix([1,1/kk]); first=raw.step(st,h,(kk,)); second=raw.step(first,h,(kk,))
    exactx=1/(1-2*h); exacty=exactx*exactx/kk
    err=second[1]-exacty; ratio=err/h**3
    check(f'semilinear_no_go_is_actual_error_N{N}',err>0)
    # Exact scalar slow equation is unaffected by the fast mode in the full map.
    xx=slow_step(slow_step(mp.mpf(1),h),h)
    check(f'semilinear_slow_component_reference_N{N}',abs(second[0]-xx)<mp.mpf('1e-65'))
    no_go.append({'N':N,'h':text(h),'kappa':text(kk),'t_final':text(2*h),
                  'y_error':text(err),'y_error_over_h3':text(ratio),
                  'first_manifold_defect_over_h11':text((first[1]-first[0]**2/kk)/h**11)})
check('semilinear_no_go_approaches_four_thirds',abs(mp.mpf(no_go[-1]['y_error_over_h3'])-mp.mpf(4)/3)<mp.mpf('0.08'))
check('semilinear_no_go_asymptotic_refinement',abs(mp.mpf(no_go[-1]['y_error_over_h3'])-mp.mpf(4)/3)<abs(mp.mpf(no_go[0]['y_error_over_h3'])-mp.mpf(4)/3))
# Recompute at higher precision: the scientific counterexample is not roundoff.
with mp.workdps(140):
    h=mp.mpf(1)/128; kk=h**-6; st=mp.matrix([1,1/kk])
    for _ in range(2):st=raw.step(st,h,(kk,))
    high=st[1]-1/(kk*(1-2*h)**2)
check('no_go_100_vs_140_digit_agreement',abs(high-mp.mpf(no_go[3]['y_error']))<mp.mpf('1e-39'))

# Fixed problem limit: classical order five is not contradicted.
classical=[]
for N in [8,16,32,64]:
    h=mp.mpf(1)/(4*N); kk=mp.mpf(8); st=mp.matrix([1,1/kk])
    for _ in range(N):st=raw.step(st,h,(kk,))
    err=norm(st-mp.matrix([mp.mpf(4)/3,mp.mpf(16)/9/kk]))
    classical.append({'N':N,'h':text(h),'error':text(err)})
rates=[mp.log(mp.mpf(classical[j-1]['error'])/mp.mpf(classical[j]['error']),2) for j in range(1,len(classical))]
check('fixed_kappa_classical_order5',rates[-1]>mp.mpf('4.85'))

# Same compact-jet structure with exp instead of R5: a control, not an imported method claim.
a=-k+2*x; cj=[a]
for _ in range(4):cj.append(s.expand(x*x*s.diff(cj[-1],x)+a*cj[-1]))
cjf=s.lambdify((x,k),cj,'mpmath')
def exact_phi5(v):return (mp.exp(v)-sum(v**j/mp.factorial(j) for j in range(5)))/v**5

def exp_control_step(st,h,kk):
    xx,yy=st; ee=yy-xx*xx/kk; aa=-kk+2*xx
    ps=exact_phi5(2*h*xx); pf=exact_phi5(h*aa); c=cjf(xx,kk)
    un=xx+sum(h**j*xx**(j+1) for j in range(1,5))+120*h**5*ps*xx**6
    amp=1+sum(h**j*c[j-1]/mp.factorial(j) for j in range(1,5))+h**5*pf*c[4]+240*h**5*xx**6/kk*(ps-pf)
    yon=(xx*xx+sum((j+1)*h**j*xx**(j+2) for j in range(1,5))+720*h**5*pf*xx**7)/kk+240*h**5*xx**6*(xx+xx*xx/kk)/kk*(ps-pf)
    return mp.matrix([un,yon+amp*ee])
h=mp.mpf(1)/128;kk=h**-6;st=mp.matrix([1,1/kk])
for _ in range(2):st=exp_control_step(st,h,kk)
exp_err=st[1]-1/(kk*(1-2*h)**2)
check('exact_exponential_core_does_not_remove_no_go',abs(exp_err/h**3-mp.mpf(4)/3)<mp.mpf('0.12'))

# The Darboux ratio chart preserves the physical ODE, not a projected substitute.
chart=[]
for N in [8,16,32,64]:
    h=mp.mpf(1)/(4*N); kk=h**-6; xx=mp.mpf(1); ww=mp.mpf(0)
    for _ in range(N):
        xx=slow_step(xx,h); ww=R5(-kk*h)*ww
    yy=xx*xx*(ww+1/kk); ex=abs(xx-mp.mpf(4)/3); ey=abs(yy-mp.mpf(16)/9/kk)
    check(f'chart_preserves_w_zero_N{N}',ww==0)
    check(f'chart_regular_compact_domain_N{N}',1<=xx<=2)
    check(f'chart_physical_error_reconstruction_N{N}',ey<=4*ex/kk)
    chart.append({'N':N,'h':text(h),'kappa':text(kk),'x_error':text(ex),'y_error':text(ey)})
chart_rates=[mp.log(mp.mpf(chart[j-1]['x_error'])/mp.mpf(chart[j]['x_error']),2) for j in range(1,len(chart))]
check('chart_repair_uniform_slow_order5',chart_rates[-1]>mp.mpf('4.9'))
h=mp.mpf(1)/10;kk=mp.mpf(40);w0=mp.mpf(1)/10;w1=R5(-kk*h)*w0
check('chart_retains_nonzero_fast_initial_layer',0<w1<w0)

# Matrix PR: two nonnormal 3x3 blocks, one defective and one with complex fast modes.
A01=mp.matrix([[-1,1,0],[0,-1,mp.mpf(1)/2],[0,0,-2]])
A02=mp.matrix([[-1,6,0],[-6,-2,1],[0,-1,-3]])
def forcing(t,j):
    vals=[]
    for d in [5,6]:
        vals.append(mp.factorial(d)/mp.factorial(d-j)*t**(d-j) if j<=d else mp.mpf(0))
    vals.append([mp.sin,mp.cos,lambda x:-mp.sin(x),lambda x:-mp.cos(x)][j%4](t))
    return mp.matrix(vals)
def matrix_functions(A,h):
    Z=h*A; I=mp.eye(A.rows); Z2=Z*Z
    Q=I-mp.mpf(3)/5*Z+mp.mpf(3)/20*Z2-Z2*Z/60
    R=Q**-1*(I+mp.mpf(2)/5*Z+Z2/20)
    PH=Q**-1*(Z2-5*Z+12*I)/1440
    K=-(Z2-6*Z+60*I)**-1*(Z2-4*Z+11*I)/120
    return R,PH-I/120,K
matrix_pr=[]
for mi,A0 in enumerate([A01,A02],1):
    for lane in ['kappa1','kappa1000','kappa_hminus2']:
        run=[]
        for N in [8,16,32,64]:
            h=mp.mpf(1)/N; kk=mp.mpf(1) if lane=='kappa1' else (mp.mpf(1000) if lane=='kappa1000' else h**-2)
            A=kk*A0; RR,BB,KK=matrix_functions(A,h); ee=mp.zeros(3,1)
            for n in range(N):
                t=mp.mpf(n)/N
                rem=forcing(t+h,0)-sum((h**j/mp.factorial(j)*forcing(t,j) for j in range(6)),mp.zeros(3,1))
                ee=RR*ee+h**5*BB*forcing(t,5)-rem
            error=norm(ee); bound=h**5*((2*mp.mpf(841)+721)/60+mp.mpf(721)/720)
            check(f'matrix_PR_bound_m{mi}_{lane}_N{N}',error<=bound)
            Rnp=np.array(RR.tolist(),dtype=float); Knp=np.array(KK.tolist(),dtype=float)
            check(f'matrix_PR_contractivity_m{mi}_{lane}_N{N}',np.linalg.norm(Rnp,2)<=1+1e-12 and np.linalg.norm(Knp,2)<=1/60+1e-12)
            run.append({'N':N,'kappa':text(kk),'error':text(error),'bound':text(bound)})
        rates_m=[mp.log(mp.mpf(run[j-1]['error'])/mp.mpf(run[j]['error']),2) for j in range(1,len(run))]
        if lane=='kappa_hminus2':check(f'matrix_PR_uniform_refinement_m{mi}',rates_m[-1]>mp.mpf('4.75'))
        matrix_pr.append({'matrix':mi,'lane':lane,'data':run,'rates':[text(v) for v in rates_m]})
# One full, independent augmented-matrix RVJ evaluation, not only the recurrence.
t,y1,y2,y3=s.symbols('t y1 y2 y3'); AA=s.Matrix([[-80,80,0],[0,-80,40],[0,0,-160]])
phivec=s.Matrix([t**5,t**6,s.sin(t)]); fy=AA*(s.Matrix([y1,y2,y3])-phivec)+s.diff(phivec,t)
pr=AnalyticRVJ((t,y1,y2,y3),(),s.Matrix([1,*fy])); hh=mp.mpf(1)/32; tt=mp.mpf(1)/3
estate=mp.matrix([mp.mpf(1)/13,-mp.mpf(1)/17,mp.mpf(1)/19]); st=mp.matrix([tt,*list(forcing(tt,0)+estate)])
full=pr.step(st,hh); RR,BB,KK=matrix_functions(mp.matrix(AA.tolist()),hh)
rem=forcing(tt+hh,0)-sum((hh**j/mp.factorial(j)*forcing(tt,j) for j in range(6)),mp.zeros(3,1))
target=RR*estate+hh**5*BB*forcing(tt,5)-rem
check('matrix_PR_full_VoC_step_matches_recurrence',norm(mp.matrix(list(full)[1:])-forcing(tt+hh,0)-target)<mp.mpf('1e-75'))

# Embedded estimator can be arbitrarily blind, even on the proved constant-rate PR class.
la=s.Symbol('lam'); sf=s.Matrix([1,la*(y-t**5)+5*t**4]); p5=AnalyticRVJ((t,y),(la,),sf,5); p4=AnalyticRVJ((t,y),(la,),sf,4)
estimators=[];hh=mp.mpf(1)/10
for zz in [-10,-100,-10000,-1000000]:
    zz=mp.mpf(zz); ll=zz/hh; main=p5.step([0,0],hh,(ll,)); low=p4.step([0,0],hh,(ll,))
    trueerr=abs(main[1]-hh**5); eta=abs(main[1]-low[1]); ratio=eta/trueerr
    expected=120*phi5(zz)/(1-120*phi5(zz))
    cert=hh**5*abs(120*phi5(zz)-1)
    check(f'embedded_quintic_effectivity_z{int(zz)}',abs(ratio-expected)<mp.mpf('1e-75'))
    check(f'quintic_defect_certificate_z{int(zz)}',abs(cert-trueerr)<mp.mpf('1e-75'))
    estimators.append({'z':text(zz),'true_local_error':text(trueerr),'embedded_estimate':text(eta),'effectivity':text(ratio),'certified_defect':text(cert)})
check('embedded_effectivity_below_one_in_100000',mp.mpf(estimators[-1]['effectivity'])<mp.mpf('1e-5'))
# In a generic method this certificate requires a bounded sixth derivative; it is NOT claimed generally cheap.

result={'scope':'dense analytic research references and recurrence checks; no AD/Krylov performance benchmark',
'environment':{'python':platform.python_version(),'mpmath':mp.__version__,'sympy':s.__version__,'numpy':np.__version__,'mp_dps':mp.mp.dps},
'passed':sum(c['passed'] for c in checks),'total':len(checks),'elapsed_seconds':time.perf_counter()-started,'checks':checks,
'semilinear_two_step_no_go':no_go,'fixed_problem_classical':{'data':classical,'rates':[text(v) for v in rates]},
'exponential_core_control':{'h':text(mp.mpf(1)/128),'y_error_over_h3':text(exp_err/(mp.mpf(1)/128)**3)},
'chart_repair':{'data':chart,'slow_rates':[text(v) for v in chart_rates],'nonzero_fast_mode_after_one_step':text(w1)},
'matrix_PR':matrix_pr,'embedded_estimator_counterexample':estimators,
'not_claimed':['general nonlinear stiff convergence','uniform accuracy throughout arbitrary unresolved initial layers','production AD','Krylov iteration or speedup','RODAS5P/BDF/Radau wall-clock ranking']}
(OUT/'numeric_loop3.json').write_text(json.dumps(result,indent=2,ensure_ascii=False)+'\n')
print(json.dumps({'passed':result['passed'],'total':result['total'],'seconds':result['elapsed_seconds'],
'no_go_ratios':[v['y_error_over_h3'] for v in no_go],'chart_rates':result['chart_repair']['slow_rates'],
'classical_rates':result['fixed_problem_classical']['rates'],'exp_control':result['exponential_core_control'],'effectivity_last':estimators[-1]['effectivity']},indent=2))
