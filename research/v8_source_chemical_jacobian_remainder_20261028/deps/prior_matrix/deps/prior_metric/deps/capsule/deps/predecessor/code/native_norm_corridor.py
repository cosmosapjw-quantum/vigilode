"""Derived exact dual support/corridor algebra; no native norm or lift evaluator."""
from fractions import Fraction as F
from pathlib import Path
from math import isqrt
import argparse, hashlib, importlib.util, json

ROOT = Path(__file__).resolve().parents[1]
BINDING = '8cb268e8de741e284cf494013f1e0748d27f03c5e77ba0c64413ae92af082d16'
BASS = 'cc602e209e7deeb511ab2a1f3b2268d55206c43c'
# Exact fixed source coordinate correspondence, not a lift implementation.
BLOCKS = ((23, (24,), (23,)), (25, (26,27), (24,25)), (28, (29,30,31), (26,27,28)))
INDEPENDENT = tuple((i,i) for i in range(23)) + ((29,32),)

class Refusal(ValueError):
    def __init__(self, code):
        self.code=code
        super().__init__(code)

def sha(b):
    return hashlib.sha256(b).hexdigest()

def canonical(value):
    try:
        return sha(json.dumps(value,sort_keys=True,separators=(',',':'),allow_nan=False).encode())
    except (ValueError,TypeError):
        raise Refusal('INPUT_NOT_JSON_SERIALIZABLE')

def dependencies():
    identity=json.loads((ROOT/'inputs/REUSED_EXACT_SIGN_IDENTITY.json').read_text())
    for row in identity['files']:
        if sha((ROOT/row['path']).read_bytes())!=row['sha256']:
            raise Refusal('REUSED_SOURCE_IDENTITY_MISMATCH:'+row['path'])
    spec=importlib.util.spec_from_file_location('preserved_exact_sign',ROOT/'inputs/reused_exact_sign/code/exact_row_sign.py')
    exact=importlib.util.module_from_spec(spec)
    spec.loader.exec_module(exact)
    norm=json.loads((ROOT/'inputs/native_norm/NATIVE_NORM_BINDING.json').read_text())
    if norm['reduced_dimension']!=30 or norm['physical_dimension']!=33:
        raise Refusal('NATIVE_NORM_DIMENSION_MISMATCH')
    # The immutable binding file supplies L/configuration. Its hash is verified
    # above; no copy of its norm or lift evaluation algorithm is defined here.
    return exact,identity,norm

def vector(value,n,E,name):
    if not isinstance(value,list) or len(value)!=n:
        raise Refusal('INVALID_'+name+'_SHAPE')
    return [E.exact(x) for x in value]

def polynomial(value,E,name):
    if not isinstance(value,list) or not value or len(value)>5:
        raise Refusal('INVALID_'+name+'_POLYNOMIAL')
    return E.trim([E.exact(x) for x in value])

def precision_check(value):
    if isinstance(value,bool) or not isinstance(value,int) or not 0<=value<=128:
        raise Refusal('INVALID_SQRT_PRECISION')

def step_limit(value):
    if isinstance(value,bool) or not isinstance(value,int) or not 0<=value<=256:
        raise Refusal('INVALID_MAX_ISOLATION_STEPS')

def encoded(values,E):
    return [E.rat(x) for x in values]

def fixed_norm_id(u,E):
    return canonical({'native_norm_blob':'95efe19477aba8efe378a1f5e083c09b2cdffb69','upper_scales':encoded(u,E),'reduced':30,'physical':33})

def support_core(a,u,E,*,precision=32,upper=None):
    """Derived Sherman-Morrison blocks for q=a^T A^-1 a, no norm evaluation."""
    precision_check(precision)
    if any(x<=0 for x in u):
        raise Refusal('PHYSICAL_UPPER_SCALES_MUST_BE_POSITIVE')
    q=F(0);w=[F(0)]*30;parts=[]
    for reduced,physical in INDEPENDENT:
        w[reduced]=u[physical]**2*a[reduced]
        value=a[reduced]*w[reduced]
        q+=value
        parts.append({'kind':'INDEPENDENT','reduced':reduced,'physical':physical,'q':E.rat(value)})
    identities=[]
    for neutral,charged,reduced in BLOCKS:
        c=[u[i]**2 for i in charged];cN=u[neutral]**2
        total=sum((ci*a[j] for ci,j in zip(c,reduced)),F(0));denominator=cN+sum(c,F(0))
        value=sum((ci*a[j]**2 for ci,j in zip(c,reduced)),F(0))-total**2/denominator
        for ci,j in zip(c,reduced):w[j]=ci*(a[j]-total/denominator)
        q+=value
        sum_w=sum((w[j] for j in reduced),F(0))
        identity=[w[j]/ci+sum_w/cN==a[j] for ci,j in zip(c,reduced)]
        identities.extend(identity)
        parts.append({'kind':'CHEMISTRY_BLOCK','neutral':neutral,'charged':list(charged),'reduced':list(reduced),
                      'q':E.rat(value),'denominator':E.rat(denominator),'weighted_row_sum':E.rat(total),
                      'A_times_w_equals_a_by_block_identity':identity})
    if q<0 or q!=sum((x*y for x,y in zip(a,w)),F(0)) or not all(identities):
        raise Refusal('INTERNAL_DUAL_IDENTITY_FAILURE')
    squared=33*q
    denominator=2**precision
    floor_int=isqrt((squared.numerator*denominator**2)//squared.denominator)
    lower=F(floor_int,denominator);dyadic=lower if lower**2==squared else lower+F(1,denominator)
    if upper is not None:
        if upper<0 or upper**2<squared:
            raise Refusal('SUPPLIED_SUPPORT_UPPER_NOT_VERIFIED')
        selected=upper;choice='SUPPLIED_VERIFIED_RATIONAL'
    else:selected=dyadic;choice='UPWARD_DYADIC'
    return {'status':'PASS_EXACT_DUAL_SUPPORT','a':encoded(a,E),'upper_scales':encoded(u,E),
            'fixed_norm_identity':fixed_norm_id(u,E),'q':E.rat(q),'K_squared':E.rat(squared),'w':encoded(w,E),
            'q_equals_a_dot_w':True,'A_times_w_equals_a':True,'block_records':parts,
            'Kbar':E.rat(selected),'Kbar_squared_dominates_K_squared':selected**2>=squared,
            'upper_method':choice,'dyadic_sqrt':{'precision':precision,'lower':E.rat(lower),'upper':E.rat(dyadic),
                'lower_square_le':lower**2<=squared,'upper_square_ge':dyadic**2>=squared},
            'scope':'EXACT_FROZEN_UPPER_METRIC_SUPPORT; actual positive scales s<=u imply K_s<=K_u',
            'native_norm_evaluations':0}

def ceiling():
    return {'actual_query_failure':False,'query_rounding_bound':None,'actual_owner_segment':None,
            'whole_domain_certificate':False,'VerifiedModelProvider':None,'actual_ode_solution_enclosure':False,
            'production':'HOLD','globalphysicalcompletion':None,'L4':False,'central_last_confirmed':'28/88','this_unit_delta':0,
            'native_unknown_budgets':'UNKNOWN_PRESERVED','physical_calls':{'rhs':0,'jacobian':0,'ode':0,'ncp':0,'native_norm':0,'lift':0}}

def refusal(fn):
    def wrapped(*args,**kwargs):
        try:return fn(*args,**kwargs)
        except ValueError as exc:
            if not hasattr(exc,'code'):raise
            return {'schema':'vigilode.native-norm-corridor.refusal.v1','status':'REFUSED','refusal':exc.code,**ceiling()}
    return wrapped

@refusal
def support_certificate(a,upper_scales,*,precision=32,Kbar=None):
    E,identity,_=dependencies()
    av=vector(a,30,E,'ROW');u=vector(upper_scales,33,E,'SCALES')
    result=support_core(av,u,E,precision=precision,upper=E.exact(Kbar) if Kbar is not None else None)
    return {'schema':'vigilode.native-norm-dual-support.v1',**result,'source_dependencies':identity,**ceiling()}

def sign(p,E,strict,limit):
    return E._scalar_certificate_core(E.trim(p),strict=strict,max_isolation_steps=limit)

def add(p,q):
    n=max(len(p),len(q));return [(p[i]if i<len(p)else F(0))+(q[i]if i<len(q)else F(0))for i in range(n)]

def barrier_core(R,Delta,mu,eta,E0,E,*,limit,norm_identity,provenance):
    if Delta<=0:raise Refusal('NONPOSITIVE_CLOCK_SPAN')
    if eta<0 or E0<0:raise Refusal('NEGATIVE_BARRIER_ETA_OR_INITIAL')
    if not isinstance(norm_identity,str) or not norm_identity:
        raise Refusal('FIXED_NORM_IDENTITY_REQUIRED')
    if not isinstance(provenance,dict) or not provenance.get('statement'):
        raise Refusal('BARRIER_PROVENANCE_REQUIRED')
    if provenance.get('fixed_norm_identity')!=norm_identity:
        raise Refusal('BARRIER_FIXED_NORM_PREMISE_MISMATCH')
    B=add(E.derivative(R),[-Delta*mu*x for x in R]);B[0]-=Delta*eta
    rp=sign(R,E,False,limit);bp=sign(B,E,False,limit);initial=E.evaluate(R,F(0))>=E0
    passed=rp['status']=='PASS' and bp['status']=='PASS' and initial
    return {'status':'PASS_CONDITIONAL_BARRIER'if passed else'INCONCLUSIVE','radius_power':encoded(R,E),
            'barrier_power':encoded(E.trim(B),E),'radius_sign':rp,'barrier_sign':bp,'initial_dominates_E0':initial,
            'Delta':E.rat(Delta),'mu':E.rat(mu),'eta':E.rat(eta),'E0':E.rat(E0),
            'fixed_norm_identity':norm_identity,'provenance':provenance,
            'clock_derivative':'d/dx=(1/Delta)d/dtheta; nativeh is not substituted',
            'conditional_on_external_initial_residual_lognorm_clock_source_and_continuation_premises':True}

@refusal
def barrier_certificate(radius,Delta,mu,eta,E0,*,norm_identity,provenance,max_isolation_steps=128):
    E,_,_=dependencies();step_limit(max_isolation_steps)
    return {'schema':'vigilode.comparison-barrier.v1',**barrier_core(polynomial(radius,E,'RADIUS'),E.exact(Delta),E.exact(mu),E.exact(eta),E.exact(E0),E,limit=max_isolation_steps,norm_identity=norm_identity,provenance=provenance),**ceiling()}

@refusal
def constructive_affine_radius(Delta,mu,eta,R0):
    E,_,_=dependencies();D,m,e,r=map(E.exact,(Delta,mu,eta,R0))
    if D<=0 or m<0 or e<0 or r<0 or D*m>=1:
        raise Refusal('AFFINE_BARRIER_CONSTRUCTION_PREMISES_FAIL')
    slope=D*(m*r+e)/(1-D*m)
    return {'schema':'vigilode.affine-barrier-construction.v1','status':'PASS_EXACT_ALGEBRA',
            'radius_power':encoded([r,slope],E),'barrier_power':encoded([D*m*slope,-D*m*slope],E),
            'slope':E.rat(slope),'requires_same_fixed_norm_external_mu_eta_E0_premises':True,**ceiling()}

def direction_core(R,Fpower,rho,terms,c,query,powers,E,support):
    """Verify an exact algebraic direction identity, never evaluate native norm."""
    if not isinstance(query,dict):raise Refusal('INVALID_DIRECTION_WITNESS')
    t=E.exact(query.get('theta'));lam=E.exact(query.get('lambda'))
    if not 0<=t<=1 or lam<0:raise Refusal('INVALID_DIRECTION_PARAMETER')
    w=[E._trusted_exact(x)for x in support['w']];qval=E._trusted_exact(support['q'])
    e=[-lam*x for x in w];radius=E.evaluate(R,t)
    norm_squared=lam*lam*qval/33;decrement=lam*qval
    corner=[-((1 if terms.get(i,0)>0 else -1)if terms.get(i,0)else 0)*rho[i]for i in range(31)]
    base=[E.evaluate(p,t)for p in powers];point=[base[0]+corner[0]]+[base[i]+corner[i]+e[i-1]for i in range(1,31)]
    row_value=c+sum((F(v)*point[i]for i,v in terms.items()),F(0));expected=E.evaluate(Fpower,t)-decrement
    membership=radius>=0 and norm_squared<=radius**2
    corner_ok=all(abs(x)<=r for x,r in zip(corner,rho))
    violation=row_value<=0 if query['strict']else row_value<0
    return {'status':'VERIFIED_DECLARED_FIXED_UPPER_METRIC_COUNTEREXAMPLE'if membership and corner_ok and violation and row_value==expected else'WITNESS_NOT_ESTABLISHED',
            'theta':E.rat(t),'lambda':E.rat(lam),'w':encoded(w,E),'error_direction':encoded(e,E),
            'error_box_corner':encoded(corner,E),'error_box_membership':corner_ok,
            'point':encoded(point,E),'radius':E.rat(radius),'derived_norm_squared':E.rat(norm_squared),
            'derived_norm_squared_identity':'lambda^2*q/33; no norm evaluator called',
            'norm_ball_membership':membership,'row_decrement':E.rat(decrement),'row_value':E.rat(row_value),
            'row_equals_F_minus_lambda_q':row_value==expected,'strict':query['strict'],
            'scope':'DECLARED_FIXED_UPPER_METRIC_CORRIDOR_ONLY; unknown actuals<=u may define a smaller ball',
            'actual_query_failure':False,'native_norm_evaluations':0}

@refusal
def corridor_certificate(inp):
    if not isinstance(inp,dict):raise Refusal('MALFORMED_INPUT')
    input_hash=canonical(inp)
    if inp.get('binding_id')!=BINDING or inp.get('bass_commit')!=BASS:
        raise Refusal('SOURCE_OR_BINDING_MISMATCH')
    if inp.get('synthetic_fixture')is not True or inp.get('actual_owner_segment','MISSING')is not None:
        raise Refusal('SYNTHETIC_PROVENANCE_REQUIRED')
    E,identity,norm=dependencies();old,dense,domain,_=E.load_dependencies()
    _,q,n=dense.parse(inp)
    if n!=30:raise Refusal('STATE_DIMENSION_MUST_BE_30')
    u=vector(inp.get('upper_scales'),33,E,'SCALES')
    if any(x<=0 for x in u):raise Refusal('PHYSICAL_UPPER_SCALES_MUST_BE_POSITIVE')
    prem=inp.get('scales_provenance')
    if not isinstance(prem,dict)or not prem.get('statement')or prem.get('frozen_over_segment')is not True or prem.get('actual_positive_scales_bounded_by_upper')is not True:
        raise Refusal('FIXED_UPPER_SCALE_PREMISE_REQUIRED')
    precision=inp.get('sqrt_precision',32);precision_check(precision)
    limit=inp.get('max_isolation_steps',128);step_limit(limit)
    R=polynomial(inp.get('radius_power'),E,'RADIUS');rp=sign(R,E,False,limit)
    inherited=[];guards=[]
    for i,values in enumerate(zip(*(q[k]for k in ('y0','y1','d0','d1','d2')))):
        b=dense.coordinate_bound(*values,q['delta'])
        if any(m>dense.FMAX for _,m in b['guards']):raise Refusal('SOURCE_RANGE_BOUND_UNAVAILABLE:'+str(i))
        inherited.append(b['rho']);guards.append({'coordinate':i,'guards':[{'name':name,'passed':m<=dense.FMAX,'magnitude':E.rat(m)}for name,m in b['guards']]})
    if 'rho_allowance'in inp:
        state_rho=vector(inp['rho_allowance'],30,E,'RHO')
        if any(x<0 for x in state_rho):raise Refusal('NEGATIVE_RHO_ALLOWANCE')
        if any(a<b for a,b in zip(state_rho,inherited)):raise Refusal('RHO_BELOW_SOURCE_RN_BOUND')
    else:state_rho=inherited
    rho=[F(0)]+state_rho;binding=domain.source_binding()
    physical=domain.rows(binding);finite=[('finite_'+kind+'_w'+str(i),dense.FMAX,{i:s},False)for i in range(31)for kind,s in(('plus',1),('minus',-1))]
    names={x[0]for x in physical+finite};uppers=inp.get('support_uppers',{});witnesses=inp.get('direction_witnesses',{})
    if not isinstance(uppers,dict)or not set(uppers)<=names:raise Refusal('UNKNOWN_SUPPORT_UPPER_ROW')
    if not isinstance(witnesses,dict)or not set(witnesses)<=names:raise Refusal('UNKNOWN_DIRECTION_WITNESS_ROW')
    powers=[[q['x0'],q['delta'],F(0),F(0),F(0)]]+old.state_power(q)
    def row(name,c,terms,strict):
        H,Fpower,sigma,*_=old.row_data(c,terms,q,rho);a=[F(terms.get(i+1,0))for i in range(30)]
        sc=support_core(a,u,E,precision=precision,upper=E.exact(uppers[name])if name in uppers else None);K=E._trusted_exact(sc['Kbar'])
        G=add(Fpower,[-K*x for x in R]);certificate=sign(G,E,strict,limit)
        out={'name':name,'strict':strict,'constant':E.rat(c),'terms':{str(i):E.rat(F(v))for i,v in terms.items()},
             'H_power':encoded(H,E),'box_support':E.rat(sigma),'F_power':encoded(Fpower,E),'G_power':encoded(E.trim(G),E),
             'dual_support':sc,'sufficient_sign':certificate,'status':'PASS_CONDITIONAL_CORRIDOR'if certificate['status']=='PASS'and rp['status']=='PASS'else'INCONCLUSIVE',
             'negative_G_is_not_physical_failure':True}
        if name in witnesses:
            if not isinstance(witnesses[name],dict):raise Refusal('INVALID_DIRECTION_WITNESS')
            request=dict(witnesses[name]);request['strict']=strict
            w=direction_core(R,Fpower,rho,terms,c,request,powers,E,sc);out['direction_witness']=w
            if rp['status']=='PASS'and w['status']=='VERIFIED_DECLARED_FIXED_UPPER_METRIC_COUNTEREXAMPLE':out['status']='DECLARED_FAMILY_COUNTEREXAMPLE'
        return out
    ph=[row(*data)for data in physical];fi=[row(*data)for data in finite]
    def group(rows):
        states=[x['status']for x in rows]
        if'DECLARED_FAMILY_COUNTEREXAMPLE'in states:return'DECLARED_FAMILY_COUNTEREXAMPLE'
        return'PASS_CONDITIONAL_CORRIDOR'if all(x=='PASS_CONDITIONAL_CORRIDOR'for x in states)else'INCONCLUSIVE'
    ps,fs=group(ph),group(fi);status='DECLARED_FAMILY_COUNTEREXAMPLE'if'DECLARED_FAMILY_COUNTEREXAMPLE'in(ps,fs)else'PASS_CONDITIONAL_CORRIDOR'if ps==fs=='PASS_CONDITIONAL_CORRIDOR'else'INCONCLUSIVE'
    barrier=None
    if'barrier'in inp:
        b=inp['barrier']
        if not isinstance(b,dict):raise Refusal('INVALID_BARRIER_INPUT')
        barrier=barrier_core(R,q['delta'],E.exact(b.get('mu')),E.exact(b.get('eta')),E.exact(b.get('E0')),E,limit=limit,norm_identity=fixed_norm_id(u,E),provenance=b.get('provenance'))
    return {'schema':'vigilode.native-norm-corridor.v1','unit':'V8_NATIVE_NORM_CORRIDOR_20261028','status':status,
            'input_sha256':input_hash,'source_dependencies':identity,'native_constructor_data_source':'inputs/native_norm/NATIVE_NORM_BINDING.json',
            'fixed_source_binding':binding,'fixed_norm_identity':fixed_norm_id(u,E),'upper_scales':encoded(u,E),'scale_premises':prem,
            'radius_power':encoded(R,E),'radius_sign':rp,'rho_used':encoded(rho,E),'rho_inherited':encoded(inherited,E),'rho_clock':E.rat(F(0)),
            'source_range_guards':guards,'physical19':{'count':len(ph),'status':ps,'rows':ph},'finite62':{'count':len(fi),'status':fs,'rows':fi},
            'barrier':barrier,'combined_corridor_barrier_compatibility':None if barrier is None else status=='PASS_CONDITIONAL_CORRIDOR'and barrier['status']=='PASS_CONDITIONAL_BARRIER',
            'scope':'SYNTHETIC_DECLARED_FIXED_UPPER_METRIC_PLUS_SEPARATE_QUERY_BOX; not nativefloating acceptance or actualODE enclosure',**ceiling()}

def main():
    ap=argparse.ArgumentParser();ap.add_argument('input');ap.add_argument('--output');args=ap.parse_args()
    try:result=corridor_certificate(json.loads(Path(args.input).read_text()))
    except(OSError,json.JSONDecodeError):result={'status':'REFUSED','refusal':'INPUT_READ_ERROR',**ceiling()}
    text=json.dumps(result,indent=2)+'\n'
    if args.output:Path(args.output).write_text(text)
    else:print(text,end='')
    return 2 if result['status']=='REFUSED'else 0

if __name__=='__main__':
    raise SystemExit(main())
