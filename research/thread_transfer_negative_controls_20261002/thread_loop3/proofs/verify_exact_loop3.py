#!/usr/bin/env python3
"""Exact certificates for loop 3. A successful check can certify a failure case."""
from __future__ import annotations
import json, time, platform
from pathlib import Path
import sympy as s
from reference_rvj import AnalyticRVJ, P5,Q5,z
ROOT=Path(__file__).resolve().parents[1]; OUT=ROOT/'results'; OUT.mkdir(exist_ok=True)
checks=[]; started=time.perf_counter()
def check(name: str, condition: object, **data: object) -> None:
    ok=bool(condition); checks.append({'name':name,'passed':ok,**data})
    if not ok:
        (OUT/'exact_failure.json').write_text(json.dumps(checks,indent=2,default=str))
        raise AssertionError(name)
def zero(v):
    return all(s.cancel(e)==0 for e in v) if isinstance(v,s.MatrixBase) else s.cancel(v)==0
R=P5/Q5; ph=(z*z-5*z+12)/(1440*Q5); B=ph-s.Rational(1,120)
K=-(z*z-4*z+11)/(120*(z*z-6*z+60))
check('damped_defect_factorization',zero(B-(1-R)*K))
w=s.Symbol('w',real=True)
gap=s.expand(4*((w*w-60)**2+36*w*w)-((w*w-11)**2+16*w*w))
check('K_bound_halfplane_SOS',zero(gap-(3*(w*w-55)**2+5204)))
check('K_poles_open_right_halfplane',s.solve(z*z-6*z+60,z)==[3-s.sqrt(51)*s.I,3+s.sqrt(51)*s.I])
# R stability boundary certificate inherited from parent, explicitly rechecked.
r_gap=s.expand((Q5.subs(z,s.I*w)*Q5.subs(z,-s.I*w))-(P5.subs(z,s.I*w)*P5.subs(z,-s.I*w)))
check('R5_imaginary_boundary_gap',zero(r_gap-w**6/3600))
check('R5_Hurwitz_margin',s.Rational(3,5)*s.Rational(3,20)-s.Rational(1,60)==s.Rational(11,150))
# Abel identity does not require commuting step matrices.
Rs=[s.Matrix([[s.Rational(1,2),s.Rational(1,5)],[0,s.Rational(2,3)]]),
    s.Matrix([[s.Rational(1,3),0],[s.Rational(1,7),s.Rational(1,2)]]),
    s.Matrix([[s.Rational(2,5),-s.Rational(1,9)],[s.Rational(1,8),s.Rational(1,3)]])]
us=[s.Matrix([1,2]),s.Matrix([-2,3]),s.Matrix([4,-1])]
I=s.eye(2); product=[None]*4;product[3]=I
for j in range(2,-1,-1):product[j]=product[j+1]*Rs[j]
lhs=sum((product[j+1]*(I-Rs[j])*us[j] for j in range(3)),s.zeros(2,1))
rhs=us[-1]-product[0]*us[0]-sum((product[j]*(us[j]-us[j-1]) for j in range(1,3)),s.zeros(2,1))
check('ordered_Abel_identity',zero(lhs-rhs))
check('ordered_Abel_test_noncommuting',not zero(Rs[0]*Rs[1]-Rs[1]*Rs[0]))
# Two constant fast matrices; neither is normal; one is defective.
A1=s.Matrix([[-1,1,0],[0,-1,s.Rational(1,2)],[0,0,-2]])
A2=s.Matrix([[-1,6,0],[-6,-2,1],[0,-1,-3]])
for j,A in enumerate([A1,A2],1):
    neg=-(A+A.T)
    check(f'fast_matrix_{j}_dissipative_Sylvester',all(neg[:d,:d].det()>0 for d in range(1,4)))
    check(f'fast_matrix_{j}_nonnormal',not zero(A*A.T-A.T*A))
check('fast_matrix_1_defective',3-(A1+s.eye(3)).rank()==1 and zero(A1.charpoly(z).as_expr()-(z+1)**2*(z+2)))

# True semilinear counterexample, local g-derivatives independent of k.
x,y,h,k,e=s.symbols('x y h k e')
f=s.Matrix([x*x,(-k+2*x)*y+x*x]); model=AnalyticRVJ((x,y),(k,),f)
lin=s.diag(0,-k); gg=(f-lin*s.Matrix([x,y])).applyfunc(s.expand)
check('semilinear_g_independent_of_stiffness',not any(v.has(k) for v in gg))
check('semilinear_higher_nonlinearity_derivatives_zero',all(s.diff(v,x,3)==0 and s.diff(v,y,3)==0 for v in gg))
C=k*y-x*x; D=k*x*x
check('Darboux_C_cofactor',zero((s.Matrix([C]).jacobian([x,y])*f)[0]-(-k+2*x)*C))
check('Darboux_D_cofactor',zero((s.Matrix([D]).jacobian([x,y])*f)[0]-2*x*D))
gauge=y/x**2-1/k
check('gauge_exact_constant_fast_equation',zero((s.Matrix([gauge]).jacobian([x,y])*f)[0]+k*gauge))
ww=s.Symbol('w'); reconstruct=s.Matrix([x,x*x*(ww+1/k)])
check('chart_jacobian',reconstruct.jacobian([x,ww]).det()==x*x)
check('chart_inverse_composition',zero(gauge.subs(y,reconstruct[1])-ww))
# A bounded-degree cofactor search with explicit normalization.
a0,a1,a2,a3=s.symbols('a0 a1 a2 a3')
ans=a0+a1*x+a2*y+a3*x*x
conditions=s.Poly(s.expand((s.Matrix([ans]).jacobian([x,y])*f)[0]-(-k+2*x)*ans),x,y).coeffs()+[a2-k]
sol=s.solve(conditions,[a0,a1,a2,a3],dict=True)
check('normalized_C_degree2_search',sol==[{a0:0,a1:0,a2:k,a3:-1}])
ansd=a0+a1*x+a2*y+a3*x*x
cond=s.Poly(s.expand((s.Matrix([ansd]).jacobian([x,y])*f)[0]-2*x*ansd),x,y).coeffs()+[a3-k]
sold=s.solve(cond,[a0,a1,a2,a3],dict=True)
check('normalized_D_degree2_search',sold==[{a0:0,a1:0,a2:0,a3:k}])
# Scalar error equation C0=y-x^2/k has c_j determined from its exact flow.
a=-k+2*x; cs=[a]
for _ in range(4):cs.append(s.expand(x*x*s.diff(cs[-1],x)+a*cs[-1]))
for j in range(1,6):
    check(f'semilinear_flow_jet_{j}',zero(model.jets[j-1][1]-s.factorial(j+1)*x**(j+2)/k-cs[j-1]*(y-x*x/k)))
phis=ph.subs(z,2*h*x); phif=ph.subs(z,h*a)
U=x+sum(h**j*x**(j+1) for j in range(1,5))+120*h**5*phis*x**6
Amp=1+sum(h**j*cs[j-1]/s.factorial(j) for j in range(1,5))+h**5*phif*cs[4]+240*h**5*x**6/k*(phis-phif)
yon=(x*x+sum((j+1)*h**j*x**(j+2) for j in range(1,5))+720*h**5*phif*x**7)/k+240*h**5*x**6*(x+x*x/k)/k*(phis-phif)
Defect=s.cancel(yon-U*U/k)
Ascaled=s.cancel(h**8*Amp.subs(k,h**-6))
Dscaled=s.cancel(Defect.subs(k,h**-6)/h**11)
check('weighted_boundary_amplification',zero(Ascaled.subs(h,0)+x*x/3))
check('weighted_boundary_manifold_defect',zero(Dscaled.subs(h,0)+4*x**7))
check('weighted_boundary_regular_denominators',s.denom(Ascaled).subs(h,0)!=0 and s.denom(Dscaled).subs(h,0)!=0)
check('two_step_leading_error_four_thirds',Ascaled.subs({h:0,x:1})*Dscaled.subs({h:0,x:1})==s.Rational(4,3))
check('slow_U_consistency',U.subs(h,0)==x)
# Verify the claimed map against an independent full matrix expression at an exact rational point.
pt={x:s.Rational(2,3),y:s.Rational(1,7),k:s.Integer(20),h:s.Rational(1,40)}
J=model.jac.subs(pt); Z=pt[h]*J; eye=s.eye(2)
QZ=eye-s.Rational(3,5)*Z+s.Rational(3,20)*Z**2-Z**3/60
N5=(Z**2-5*Z+12*eye)/1440
full=s.Matrix([pt[x],pt[y]])+sum((pt[h]**j*model.jets[j-1].subs(pt)/s.factorial(j) for j in range(1,5)),s.zeros(2,1))+pt[h]**5*QZ.inv()*N5*model.jets[4].subs(pt)
check('full_matrix_map_matches_slow_update',zero(full[0]-U.subs(pt)))
check('full_matrix_map_matches_error_recurrence',zero(full[1]-full[0]**2/pt[k]-((Amp*(y-x*x/k)+Defect).subs(pt))))
# Variable leading fast rate is a separate, less restrictive counterexample.
at=-k*(1+x); ct=[at]
for _ in range(4):ct.append(s.expand(s.diff(ct[-1],x)+at*ct[-1]))
AM=1+sum(h**j*ct[j-1]/s.factorial(j) for j in range(1,5))+h**5*ph.subs(z,h*at)*ct[4]
check('variable_fast_rate_amplification_growth',zero(s.limit(AM/k**3,k,s.oo)-h**4*(1+x)**2/6))
# Stiff embedded estimator blindness on the well-prepared quintic PR problem at t=0.
eta_ratio=s.factor(120*ph/(1-120*ph))
check('quintic_estimator_ratio',zero(eta_ratio+5*(z*z-5*z+12)/(z*(z*z-4*z+11))))
check('quintic_estimator_blind_stiff_limit',s.limit(eta_ratio,z,-s.oo)==0)
check('quintic_estimator_leading_ratio',s.limit((-z)*eta_ratio,z,-s.oo)==5)
# Certificate is the exact local error for a degree-five exact trajectory.
check('quintic_defect_certificate_exact',zero(120*B-(120*ph-1)))
# Noncommuting frame change has a connection, not just similarity.
t=s.Symbol('t',real=True); S=s.Matrix([[s.cos(t),-s.sin(t)],[s.sin(t),s.cos(t)]])
conn=s.simplify(S.T*s.diff(S,t))
check('rotating_frame_connection_nonzero',conn==s.Matrix([[0,-1],[1,0]]))
# Basic matrix-PR exact recurrence at a rational point, with two coupled fast modes.
t,y1,y2=s.symbols('t y1 y2'); A=s.Matrix([[-60,40],[0,-60]])
phi=s.Matrix([t**5,t**6]); fy=A*(s.Matrix([y1,y2])-phi)+s.diff(phi,t)
pr=AnalyticRVJ((t,y1,y2),(),s.Matrix([1,*fy])); p={t:s.Rational(1,5),y1:s.Rational(2,7),y2:-s.Rational(1,9)}; hh=s.Rational(1,20)
J=pr.jac.subs(p); Z=hh*J; I3=s.eye(3)
Qm=I3-s.Rational(3,5)*Z+s.Rational(3,20)*Z**2-Z**3/60; Nm=(Z**2-5*Z+12*I3)/1440
upd=s.Matrix([p[t],p[y1],p[y2]])+sum((hh**j*pr.jets[j-1].subs(p)/s.factorial(j) for j in range(1,5)),s.zeros(3,1))+hh**5*Qm.inv()*Nm*pr.jets[4].subs(p)
ZA=hh*A; QA=s.eye(2)-s.Rational(3,5)*ZA+s.Rational(3,20)*ZA**2-ZA**3/60; PA=s.eye(2)+s.Rational(2,5)*ZA+ZA**2/20
BA=QA.inv()*(ZA**2-5*ZA+12*s.eye(2))/1440-s.eye(2)/120
err0=s.Matrix([p[y1],p[y2]])-phi.subs(t,p[t]); rem=phi.subs(t,p[t]+hh)-sum((hh**j*s.diff(phi,t,j).subs(t,p[t])/s.factorial(j) for j in range(6)),s.zeros(2,1))
target=QA.inv()*PA*err0+hh**5*BA*s.diff(phi,t,5).subs(t,p[t])-rem
check('vector_PR_full_augmented_matrix_recurrence',zero(s.Matrix(upd[1:])-phi.subs(t,p[t]+hh)-target))

result={'scope':'exact research certificates, including no-go results; not a production solver',
'environment':{'python':platform.python_version(),'sympy':s.__version__},'passed':sum(c['passed'] for c in checks),'total':len(checks),
'elapsed_seconds':time.perf_counter()-started,'checks':checks,
'vector_PR':{'K':str(K),'K_halfplane_bound':'1/60','SOS':str(gap),'fixed_step_bound':'||e_N||_H <= ||e0||_H + h^5*((2 M5+T M6)/60+T M6/720)'},
'semilinear_counterexample':{'F':[str(v) for v in f],'g':[str(v) for v in gg],'initial_state':['1','1/k'],'k':'h^(-6)',
'A_scaled_boundary':str(Ascaled.subs(h,0)),'D_scaled_boundary':str(Dscaled.subs(h,0)),'two_step_error_over_h3_limit':'4/3',
'scaled_amplification':str(Ascaled),'scaled_defect':str(Dscaled),'domain':'0<=t<=1/4; k>=1; local smooth cutoff may globalize g'},
'chart':{'w':str(gauge),'reconstruction':str(reconstruct),'jacobian':'x^2','open_conditions':['x!=0','k!=0'],'C_solution':str(sol),'D_solution':str(sold)},
'embedded_effectivity':str(eta_ratio)}
(OUT/'exact_loop3.json').write_text(json.dumps(result,indent=2,ensure_ascii=False)+'\n')
print(json.dumps({'passed':result['passed'],'total':result['total'],'seconds':result['elapsed_seconds']},indent=2))
