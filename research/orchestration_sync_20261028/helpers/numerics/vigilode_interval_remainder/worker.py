from pathlib import Path
import sys
sys.dont_write_bytecode=True
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from common import *
U=Path(__file__).resolve().parent
c=prepare(U);m=c.prior();bind=binding(c);weights=[F(1),F(1,100),F(1,10000)]
u=[F(1)+F(i+1,37) for i in range(33)]
met=m.metric(u);G,Gi=met['G'],met['Ginv']; coords=c.COORDINATES

def embed(q):
    a=m.zeros()
    for i,ri in enumerate(coords):
        for j,cj in enumerate(coords):a[ri][cj]=q[i][j]
    return a

def uniform(box,*,source_binding=bind,scope='UNIFORM_FIXED_METRIC_CHEMICAL_REMAINDER'):
    if scope!='UNIFORM_FIXED_METRIC_CHEMICAL_REMAINDER':raise ValueError('SCOPE_PROMOTION')
    c.binding_check(source_binding)
    if len(box)!=6:raise ValueError('SIX_RECOMBINATION_INTERVALS_REQUIRED')
    box=[interval(x,nonnegative=True) for x in box]
    center=[(a+b)/2 for a,b in box];radius=[(b-a)/2 for a,b in box]
    mc=c.polynomial([F(0)]*6+center,weights,PROV,source_binding);mr=c.polynomial([F(0)]*6+radius,weights,PROV,source_binding)
    qc=embed(mc['Q']);rq=embed([[abs(x) for x in row] for row in mr['Q']]); eb=[F(0)]*30
    for i,j in enumerate(coords):eb[j]=mc['electron_coefficients'][i]
    t=m.multiply(m.transpose(qc),m.multiply(G,qc));sc=m.generalized_upper(t,G,Gi,refinement=0)
    beta2=m.beta_squared(rq,G,Gi);sig2=m.quadratic(Gi,eb)
    sqrt=lambda x:m.prior().scalar_sqrt_enclosure(x,precision=40)['upper']
    qb=sqrt(sc['upper'])+sqrt(beta2);gamma=sqrt(sig2)*qb
    return {'intervals':box,'center':center,'radius':radius,'Qc30':qc,'RQ30':rq,'e30':eb,'s_c':sc,'beta_squared':beta2,'sigma_squared':sig2,'q_bound':qb,'Gamma':gamma,'Hessian_bilinear_factor':2*gamma,'fixed_norm_identity':met['fixed_norm_identity'],'upper_scales':u,'scope':scope}

box=[(F(i+1,10),F(i+3,5)) for i in range(6)];r=uniform(box)
tests={};diagnostics=[]
# Exact independent squared metric diagnostics at two opposite corners and interior.
for point,rates in [('lower',[a for a,b in box]),('upper',[b for a,b in box]),('interior',[(2*a+b)/3 for a,b in box])]:
    model=c.polynomial([F(0)]*6+rates,weights,PROV,bind)
    for scale in (F(1),F(-2,3)):
        d=[scale*F((-1)**i*(i+1),17) for i in range(6)];h=[F((-1)**(i+1),i+3) for i in range(6)]
        dr=[F(0)]*30;hr=[F(0)]*30;rr=[F(0)]*30;hh=[F(0)]*30
        for j,v,w,a,b in zip(coords,d,h,c.quadratic_remainder(model,d),c.hessian_action(model,d,h)):
            dr[j]=v;hr[j]=w;rr[j]=a;hh[j]=b
        lhs=m.quadratic(G,rr);nd=m.quadratic(G,dr);nh=m.quadratic(G,hr);hess=m.quadratic(G,hh)
        assert lhs<=r['Gamma']**2*nd**2
        assert hess<=4*r['Gamma']**2*nd*nh
        diagnostics.append({'rate_point':point,'rates':rates,'d':d,'h':h,'remainder_squared_norm':lhs,'d_squared_norm':nd,'hessian_squared_norm':hess,'h_squared_norm':nh})
tests['corner_and_interior_squared_metric_inequalities']=True
fixed=[(F(i+1,9),)*2 for i in range(6)];rf=uniform(fixed)
model=c.polynomial([F(0)]*6+[a for a,b in fixed],weights,PROV,bind)
up=c.remainder_bound(model,u,refinement=0,precision=40)
assert rf['beta_squared']==0 and rf['Gamma']**2>=rf['sigma_squared']*rf['s_c']['upper'] and up['Gamma']**2>=rf['sigma_squared']*rf['s_c']['upper']
tests['zero_width_valid_fixed_Q_specialization']=True
rz=uniform([(F(0),F(0))]*6);assert rz['Gamma']==0 and rz['q_bound']==0
tests['zero_rates_Gamma_zero']=True
rh=uniform([(F(0),F(2))]+[(F(0),F(0))]*5)
rc=uniform([(F(1),F(1))]+[(F(0),F(0))]*5)
d=[F(2,7)]+[F(0)]*5;model=c.polynomial([F(0)]*6+[F(2)]+[F(0)]*5,weights,PROV,bind)
re=c.quadratic_remainder(model,d);assert re==[-2*weights[0]*d[0]**2]+[0]*5
re2=c.quadratic_remainder(model,[3*x for x in d]);assert re2==[9*x for x in re]
# H block is independent in the inherited lift: exact norm oracle verifies it.
i=coords[0];gh=G[i][i]
assert all(G[i][j]==0 for j in range(30) if j!=i)
assert rh['center'][0]==1 and rh['radius'][0]==1 and rc['s_c']['upper']==1
assert F(4)>rc['Gamma']**2*gh
assert F(4)<=rh['Gamma']**2*gh
tests['H_only_oracle_scaling_and_radius_omission_failure']=True
omission={'box':(0,2),'center_rate':1,'edge_rate':2,'center_operator_squared':1,'edge_operator_squared':4,'operator_factor':2,'center_only_Gamma_squared':rc['Gamma']**2,'true_edge_H_remainder_ratio_squared':1/gh*4,'center_only_admission_fails_exactly':F(4)>rc['Gamma']**2*gh}
# Nonuniform scales and neutral lift must remain present, not replaced by an identity metric.
assert len(set(u))==33 and G!=m.identity() and G[24][25]!=0
assert m.multiply(G,Gi)==m.identity()
tests['nonuniform_scales_and_neutral_lift']=True
# Radius behavior is displayed without asserting monotonic algorithm output.
behavior=[]
for rad in (F(0),F(1,2),F(1)):
    b=[(F(1)-rad,F(1)+rad)]+[(F(0),F(0))]*5
    rr=rc if rad==0 else rh if rad==1 else uniform(b)
    behavior.append({'radius':rad,'Gamma':rr['Gamma'],'q_bound':rr['q_bound'],'sigma_squared':rr['sigma_squared'],'s_c_upper':rr['s_c']['upper'],'beta_squared':rr['beta_squared']})
tests['radius_behavior_no_monotonic_claim']=all(x['Gamma']>=0 for x in behavior)
tests['invalid_inputs_rejected']=all([reject(lambda:uniform([(2,1)]+box[1:])),reject(lambda:uniform([(-1,1)]+box[1:])),reject(lambda:uniform(box,source_binding={**bind,'bass_commit':'wrong'})),reject(lambda:uniform(box,scope='FULL_J_PSD')),reject(lambda:uniform(box[:-1])),reject(lambda:uniform([(0,float('nan'))]+box[1:]))])
# For any same-norm J, carrier C=J+delta I has ||J-C||=delta. This separate
# exact mismatch premise contributes delta*rho, which Gamma alone does not cover.
delta=F(3,2);rho=F(1,4);transfer=r['Gamma']*rho*rho+delta*rho
assert transfer-r['Gamma']*rho*rho==F(3,8)>0
v=[F(0)]*30;v[i]=F(2,7)
assert m.quadratic(G,[delta*x for x in v])==delta**2*m.quadratic(G,v)
tests['separate_carrier_mismatch_term']=True
assert all(tests.values()),tests
result={'unit':'VIGILODE-UNIFORM-RATE-REMAINDER','status':'PASS_UNIFORM_SUPPLIED_RATE_BOX_FIXED_METRIC','binding':bind,'weights':weights,'weight_provenance':PROV,'uniform_bound':r,'controls':tests,'scientific_control_groups':len(tests),'diagnostic_count':len(diagnostics),'diagnostics':diagnostics,'fixed_width_Gamma':rf['Gamma'],'upstream_fixed_width_Gamma':up['Gamma'],'radius_omission_adversary':omission,'radius_behavior':behavior,'carrier_mismatch_control':{'carrier':'C=J+delta I, same physical metric','separately_valid_deltaJ':delta,'rho':rho,'Gamma_rho_squared_plus_deltaJ_rho':transfer},'proof':'For each frozen rate in independent box, ||Q(k)||_G <= sqrt(s_c)+sqrt(beta_squared); e dual norm gives ||R(d)||_G <= Gamma ||d||_G^2 and polarization gives Hessian bound 2 Gamma.','scope_limit':'Uniform across fixed rate choices only; no bound on rate changes between z and z+d, source clock or geometry variation. Gamma alone does not assert uncertain Jacobian PSD.','inherited_metric':'Unchanged upstream metric(u33), full physical lift and inverse retained','missing_premises':['Physical rate-box corridor binding','Prepared owner scales and capsule','Clock/geometry variation','Callback rounding'],**CEILINGS}
dump(U/'RESULTS.json',result);dump(U/'DIAGNOSTIC_INPUTS.json',{'box':box,'weights':weights,'u33':u,'binding':bind,'points':diagnostics})
# Display only the three bounded radius diagnostics, no monotonic claim.
vals=[float(x['Gamma']) for x in behavior];maxv=max(vals) or 1
svg=['<svg xmlns="http://www.w3.org/2000/svg" width="840" height="510" role="img"><title>Uniform Gamma at three supplied H-rate interval radii</title><rect width="100%" height="100%" fill="white"/><g font-family="sans-serif" fill="#172635"><text x="35" y="35" font-size="20">Uniform Gamma: three supplied H-only rate boxes</text><text x="35" y="60">Fixed center 1; radii 0, 1/2, 1. Other five rates zero; unchanged physical metric.</text><path d="M100 100V380H780" fill="none" stroke="#172635"/>']
for j,(item,value) in enumerate(zip(behavior,vals)):
    x=200+j*230;y=380-240*value/maxv
    svg.append(f'<rect x="{x-35}" y="{y}" width="70" height="{380-y}" fill="#1768a2"/><text x="{x}" y="{y-10}" text-anchor="middle">{value:.6g}</text><text x="{x}" y="405" text-anchor="middle">radius {item["radius"]}</text>')
svg.append('<text x="35" y="445">Three exact computed bounds; no general monotonicity claim. Display values are approximate.</text><text x="35" y="475">Source: unchanged metric / Q polynomial; RESULTS.json stores exact rational outward bounds.</text></g></svg>')
(U/'uniform_gamma.svg').write_text(''.join(svg)+'\n')
print('PASS',len(tests),'control groups;',len(diagnostics),'exact corner/interior diagnostics')
