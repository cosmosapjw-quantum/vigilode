"""Exact row-first quartic envelope certificates. No physical model/norm calls."""
from fractions import Fraction as F
from pathlib import Path
from math import comb
import hashlib,importlib.util,json
ROOT=Path(__file__).resolve().parents[1]
BINDING='8cb268e8de741e284cf494013f1e0748d27f03c5e77ba0c64413ae92af082d16'
class Refusal(ValueError):
    def __init__(self,code):self.code=code;super().__init__(code)
def rational(x):return {'numerator':str(x.numerator),'denominator':str(x.denominator)}
def exact(x):
    if not isinstance(x,dict) or set(x)!={'numerator','denominator'}:raise Refusal('MALFORMED_EXACT_RATIONAL')
    def integer(v):
        if isinstance(v,bool) or not isinstance(v,(str,int)):raise Refusal('MALFORMED_EXACT_RATIONAL')
        if isinstance(v,str) and (not v or v.strip()!=v or not v.lstrip('+-').isdigit()):raise Refusal('MALFORMED_EXACT_RATIONAL')
        try:return int(v)
        except ValueError:raise Refusal('MALFORMED_EXACT_RATIONAL')
    n,d=integer(x['numerator']),integer(x['denominator'])
    if d<=0:raise Refusal('INVALID_RATIONAL_DENOMINATOR')
    return F(n,d)
def sha(data):return hashlib.sha256(data).hexdigest()
def input_sha(inp):
    try:return sha(json.dumps(inp,sort_keys=True,separators=(',',':'),allow_nan=False).encode())
    except (TypeError,ValueError):return None

def load_dependencies():
    manifest=json.loads((ROOT/'inputs/REUSED_SOURCE_IDENTITY.json').read_text())
    for item in manifest['files']:
        if sha((ROOT/item['path']).read_bytes())!=item['sha256']:raise Refusal('REUSED_SOURCE_BYTE_IDENTITY_MISMATCH:'+item['path'])
    def load(name,path):
        spec=importlib.util.spec_from_file_location(name,path);module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module
    dense=load('unchanged_dense_dependency',ROOT/'inputs/reused_dense/code/dense_query_bound.py');domain=load('unchanged_domain_dependency',ROOT/'inputs/reused_domain/code/affine_tube_domain.py')
    return dense,domain,manifest

def validate_limits(max_depth,max_leaves,method):
    if isinstance(max_depth,bool) or not isinstance(max_depth,int) or not 0<=max_depth<=16:raise Refusal('INVALID_MAX_DEPTH')
    if isinstance(max_leaves,bool) or not isinstance(max_leaves,int) or not 1<=max_leaves<=2048:raise Refusal('INVALID_MAX_LEAVES')
    if not isinstance(method,str) or method not in ('hybrid','pure_bernstein'):raise Refusal('INVALID_METHOD')

def evaluate(power,t):
    out=F(0)
    for coefficient in reversed(power):out=out*t+coefficient
    return out

def power_to_bernstein(power):
    if len(power)!=5:raise Refusal('INVALID_QUARTIC_POWER_SHAPE')
    return [sum((F(comb(k,j),comb(4,j))*power[j] for j in range(k+1)),F(0)) for k in range(5)]

def split_midpoint(coefficients):
    if len(coefficients)!=5:raise Refusal('INVALID_BERNSTEIN_SHAPE')
    levels=[list(coefficients)]
    for _ in range(4):levels.append([(a+b)/2 for a,b in zip(levels[-1],levels[-1][1:])])
    return [level[0] for level in levels],[level[-1] for level in reversed(levels)]

def scalar_certificate(power,*,strict=False,max_depth=12,max_leaves=256,method='pure_bernstein',cheap_lower=None):
    """Per-row left-first dyadic search; negative coefficients are inconclusive."""
    validate_limits(max_depth,max_leaves,method)
    if not isinstance(strict,bool):raise Refusal('INVALID_STRICTNESS')
    root=power_to_bernstein(power);stack=[(F(0),F(1),root,0)];terminal=[];splits=0;evaluations=0;witness=None
    def leaf(lo,hi,b,depth,status,reason,lower):
        return {'interval':[rational(lo),rational(hi)],'depth':depth,'bernstein':[rational(x) for x in b],'bernstein_lower':rational(min(b)),'bernstein_upper':rational(max(b)),'lower_used':rational(lower),'status':status,'reason':reason}
    while stack:
        lo,hi,b,depth=stack.pop();lower=min(b)
        if method=='hybrid' and cheap_lower is not None:lower=max(lower,cheap_lower)
        if lower>0 if strict else lower>=0:
            terminal.append(leaf(lo,hi,b,depth,'CERTIFIED','POSITIVE_BOUND' if strict else 'NONNEGATIVE_BOUND',lower));continue
        for t in (lo,(lo+hi)/2,hi):
            value=evaluate(power,t);evaluations+=1
            if value<=0 if strict else value<0:
                witness={'theta':rational(t),'row_value_after_support':rational(value),'strict':strict};break
        if witness is not None:
            terminal.append(leaf(lo,hi,b,depth,'VIOLATION_WITNESS','EXACT_RATIONAL_POINT',lower))
            for a,z,c,dep in stack:terminal.append(leaf(a,z,c,dep,'NOT_NEEDED_AFTER_WITNESS','NOT_EXPLORED_NO_PROOF_CLAIM',min(c)))
            stack=[];break
        if depth>=max_depth:
            terminal.append(leaf(lo,hi,b,depth,'UNRESOLVED','DEPTH_LIMIT',lower));continue
        frontier=len(terminal)+len(stack)+1
        if frontier+1>max_leaves:
            terminal.append(leaf(lo,hi,b,depth,'UNRESOLVED','LEAF_LIMIT',lower));continue
        left,right=split_midpoint(b);mid=(lo+hi)/2;splits+=1;stack.append((mid,hi,right,depth+1));stack.append((lo,mid,left,depth+1))
    terminal.sort(key=lambda x:exact(x['interval'][0]));intervals=[(exact(x['interval'][0]),exact(x['interval'][1])) for x in terminal]
    partition=bool(intervals) and intervals[0][0]==0 and intervals[-1][1]==1 and all(a[1]==b[0] for a,b in zip(intervals,intervals[1:]))
    if witness is not None:status='VIOLATION_WITNESS'
    elif all(x['status']=='CERTIFIED' for x in terminal):status='CERTIFIED'
    else:status='UNRESOLVED_AT_LIMIT'
    return {'status':status,'strict':strict,'method':method,'strategy':'LEFT_FIRST_DYADIC_MIDPOINT','power':[rational(x) for x in power],'bernstein_root':[rational(x) for x in root],'root_lower':rational(min(root)),'certified_lower':rational(min(exact(x['lower_used']) for x in terminal)) if status=='CERTIFIED' else None,'terminal_intervals':terminal,'terminal_partition_covers_unit_interval':partition,'full_proof_coverage':status=='CERTIFIED' and partition,'early_witness_does_not_claim_full_proof_coverage':witness is not None,'witness':witness,'work':{'terminal_leaves':len(terminal),'splits':splits,'exact_witness_evaluations':evaluations,'max_depth_reached':max(x['depth'] for x in terminal)},'limits':{'max_depth':max_depth,'max_leaves':max_leaves,'scope':'PER_ROW'}}

def state_power(q):
    return [[a,b-a+c,d-c,e-d,-e] for a,b,c,d,e in zip(*(q[k] for k in ('y0','y1','d0','d1','d2')))]

def row_data(c,terms,q,rho):
    coefficients=[[q['x0'],q['delta'],F(0),F(0),F(0)]]+state_power(q)
    H=[sum((F(coef)*coefficients[i][k] for i,coef in terms.items()),F(0)) for k in range(5)];H[0]+=c
    sigma=sum((abs(F(coef))*rho[i] for i,coef in terms.items()),F(0));Fpower=list(H);Fpower[0]-=sigma
    sums=[sum((F(coef)*q[key][i-1] for i,coef in terms.items() if i>0),F(0)) for key in ('d0','d1','d2')]
    kappa=abs(sums[0])/4+4*abs(sums[1])/27+27*abs(sums[2])/256
    component=sum((abs(F(coef))*(abs(q['d0'][i-1])/4+4*abs(q['d1'][i-1])/27+27*abs(q['d2'][i-1])/256) for i,coef in terms.items() if i>0),F(0))
    endpoint_min=min(evaluate(H,F(0)),evaluate(H,F(1)));row_lower=endpoint_min-kappa-sigma;component_lower=endpoint_min-component-sigma
    return H,Fpower,sigma,row_lower,component_lower,kappa,component,sums

def actual_flags():
    return {'query_rounding_bound':None,'actual_BASS_segment_supplied':False,'whole_domain_certificate':False,'VerifiedModelProvider':None,'actual_ode_solution_enclosure':False,'globalphysicalcompletion':None,'L4':False,'production':'HOLD','canonical':'27/88_UNCHANGED','actual_query_failure':False,'physical_calls':{'rhs':0,'jacobian':0,'ode':0,'ncp':0,'native_norm':0}}

def _certificate(inp):
    dense,domain,manifest=load_dependencies()
    if not isinstance(inp,dict):raise Refusal('MALFORMED_INPUT')
    if inp.get('binding_id')!=BINDING:raise Refusal('BINDING_MISMATCH')
    if inp.get('synthetic_fixture') is not True or 'actual_owner_segment' not in inp or inp['actual_owner_segment'] is not None:raise Refusal('SYNTHETIC_PROVENANCE_REQUIRED')
    max_depth=inp.get('max_depth',12);max_leaves=inp.get('max_leaves',256);method=inp.get('method','hybrid');validate_limits(max_depth,max_leaves,method)
    floats,q,n=dense.parse(inp)
    if n!=30:raise Refusal('STATE_DIMENSION_MUST_BE_30')
    binding=domain.source_binding();inherited=[];guard_records=[]
    for i,values in enumerate(zip(*(q[k] for k in ('y0','y1','d0','d1','d2')))):
        bound=dense.coordinate_bound(*values,q['delta'])
        for name,magnitude in bound['guards']:
            if magnitude>dense.FMAX:raise Refusal('INHERITED_SUFFICIENT_RANGE_BOUND_UNAVAILABLE:'+str(i)+':'+name)
        inherited.append(bound['rho']);guard_records.append({'coordinate':i,'rho':rational(bound['rho']),'guards':[{'name':name,'magnitude':rational(mag),'passed':mag<=dense.FMAX} for name,mag in bound['guards']]})
    if 'rho_allowance' in inp:
        supplied=inp['rho_allowance']
        if not isinstance(supplied,list) or len(supplied)!=30:raise Refusal('INVALID_RHO_ALLOWANCE_SHAPE')
        state_rho=[exact(x) for x in supplied]
        if any(x<0 for x in state_rho):raise Refusal('NEGATIVE_RHO_ALLOWANCE')
        if any(a<b for a,b in zip(state_rho,inherited)):raise Refusal('RHO_ALLOWANCE_BELOW_INHERITED_BOUND')
    else:state_rho=inherited
    rho=[F(0)]+state_rho
    def certify_row(name,c,terms,strict,category):
        H,Fpower,sigma,cheap,old,kappa,component,sums=row_data(c,terms,q,rho);cert=scalar_certificate(Fpower,strict=strict,max_depth=max_depth,max_leaves=max_leaves,method=method,cheap_lower=cheap)
        out={'name':name,'category':category,'strict':strict,'constant':rational(c),'terms':{str(i):rational(F(v)) for i,v in terms.items()},'power_H':[rational(x) for x in H],'power_H_minus_sigma':[rational(x) for x in Fpower],'sigma':rational(sigma),'row_signed_dense_sums':[rational(x) for x in sums],'row_chord_kappa':rational(kappa),'component_chord_kappa':rational(component),'row_chord_lower':rational(cheap),'component_box_lower':rational(old),'row_chord_dominates_component_box':cheap>=old,'certificate':cert}
        if cert['witness'] is not None:
            t=exact(cert['witness']['theta']);base=[q['x0']+q['delta']*t]+dense.polynomial(q,t);error=[-((F(1) if terms.get(i,0)>0 else F(-1)) if terms.get(i,0)!=0 else F(0))*rho[i] for i in range(31)];point=[a+b for a,b in zip(base,error)];value=c+sum((F(coef)*point[i] for i,coef in terms.items()),F(0))
            out['declared_envelope_witness']={'theta':rational(t),'base_point':[rational(x) for x in base],'error_offset':[rational(x) for x in error],'point':[rational(x) for x in point],'exact_row_value':rational(value),'scope':'DECLARED_REAL_POLYNOMIAL_PLUS_INDEPENDENT_ERROR_BOX_ONLY','actual_query_failure':False,'endpoint_copy_has_zero_actual_rounding_error':t in (0,1),'error_box_membership_checked':all(abs(e)<=r for e,r in zip(error,rho)),'row_equals_polynomial_minus_support':value==evaluate(Fpower,t)}
        return out
    physical=[certify_row(name,c,terms,strict,'PHYSICAL_AFFINE') for name,c,terms,strict in domain.rows(binding)]
    finite=[certify_row('finite_'+kind+'_w'+str(i),dense.FMAX,{i:sign},False,'FINITE_RANGE') for i in range(31) for kind,sign in (('plus',1),('minus',-1))]
    def group(rows):
        statuses=[r['certificate']['status'] for r in rows]
        return 'DECLARED_ENVELOPE_COUNTEREXAMPLE' if 'VIOLATION_WITNESS' in statuses else 'UNRESOLVED' if 'UNRESOLVED_AT_LIMIT' in statuses else 'PASS'
    ps,fs=group(physical),group(finite);status='DECLARED_ENVELOPE_COUNTEREXAMPLE' if 'DECLARED_ENVELOPE_COUNTEREXAMPLE' in (ps,fs) else 'UNRESOLVED' if 'UNRESOLVED' in (ps,fs) else 'PASS';allrows=physical+finite
    return {'schema':'vigilode.rowwise-bernstein-domain-certificate.v1','unit':'V8_ROWWISE_BERNSTEIN_DOMAIN_20261027','input_sha256':input_sha(inp),'source_binding':binding,'source_dependencies':manifest,'synthetic_fixture':True,'actual_owner_segment':None,'status':status,'method':method,'rho_inherited':[rational(x) for x in inherited],'rho_used':[rational(x) for x in rho],'rho_clock':rational(F(0)),'rho_allowance_domination':all(a>=b for a,b in zip(state_rho,inherited)),'inherited_source_range_guards':guard_records,'physical19':{'count':len(physical),'status':ps,'rows':physical},'finite62':{'count':len(finite),'status':fs,'rows':finite},'source_family_uniform_declared_envelope_domain_inclusion':status=='PASS','proof_assumptions':['Inherited fixed source binary64 RN/gradual-underflow/no reassociation/no FMA trace','Exact represented coefficients and exact positive normal Delta; positive h provenance only','Same represented clock; rho_x=0','Synthetic declared family only; actual accepted capsule not supplied'],'work':{'total_terminal_leaves':sum(r['certificate']['work']['terminal_leaves'] for r in allrows),'total_splits':sum(r['certificate']['work']['splits'] for r in allrows),'total_exact_witness_evaluations':sum(r['certificate']['work']['exact_witness_evaluations'] for r in allrows),'max_depth_per_row':max_depth,'max_leaves_per_row':max_leaves,'limit_scope':'PER_ROW','row_count':len(allrows)},**actual_flags()}

def certificate(inp):
    try:return _certificate(inp)
    except Refusal as exc:return {'schema':'vigilode.rowwise-bernstein-refusal.v1','status':'REFUSED','refusal':exc.code,'input_sha256':input_sha(inp),'refusal_scope':'Invalid declared input or inherited sufficient bound unavailable; no native impossibility claim.',**actual_flags()}
    except ValueError as exc:
        if hasattr(exc,'code'):return {'schema':'vigilode.rowwise-bernstein-refusal.v1','status':'REFUSED','refusal':'INHERITED_'+exc.code,'input_sha256':input_sha(inp),'refusal_scope':'Inherited typed refusal; no impossibility claim.',**actual_flags()}
        raise
