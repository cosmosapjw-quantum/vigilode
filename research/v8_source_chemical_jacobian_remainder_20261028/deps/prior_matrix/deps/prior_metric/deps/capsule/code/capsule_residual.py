"""Stored-bit quartic intake and conditional finite-cover residual transport.

No physical model, native norm, or lift evaluator is defined or called here.
External bounds retain their supplied proof status and never issue a provider.
"""
from fractions import Fraction as F
from pathlib import Path
from functools import lru_cache
import hashlib, importlib.util, json, math, re, struct

ROOT=Path(__file__).resolve().parents[1]
BASS='cc602e209e7deeb511ab2a1f3b2268d55206c43c'
BINDING='8cb268e8de741e284cf494013f1e0748d27f03c5e77ba0c64413ae92af082d16'
SOURCE='50f8d288abbf54339b7a6efe4fea5809f71d40c37532f50bf9c304d4587e3fa3'
ENTRY='Rodas5pQuarticSegment::evaluate_center_at_into'
CLOCK={'variable':'x=ln(a/a_ref)','a_ref':'1/21','native_time':'x'}
ARITHMETIC=['RN ties-to-even','gradual underflow; FTZ/DAZ disabled','separate source-ordered binary64 operations; no FMA contraction/reassociation/excess precision']

class Refusal(ValueError):
    pass

def require(ok,code):
    if not ok:raise Refusal(code)

def ceiling():
    return {'actual_query_rounding_bound':None,'actual_ode_solution_enclosure':False,
            'whole_domain_certificate':False,'VerifiedModelProvider':None,'production':'HOLD',
            'globalphysicalcompletion':None,'L4':False,'actual_application':'BLOCKED_INPUT',
            'physical_calls':dict.fromkeys(('rhs','jacobian','ode','ncp','native_norm','lift','native_build'),0)}

@lru_cache(maxsize=1)
def dependencies():
    base=ROOT/'deps/predecessor'
    for item in json.loads((ROOT/'deps/SOURCE_IDENTITY.json').read_text()):
        require(hashlib.sha256((base/item['logical_path']).read_bytes()).hexdigest()==item['sha256'],'DEPENDENCY_BYTE_IDENTITY_MISMATCH')
    def load(name,path):
        spec=importlib.util.spec_from_file_location(name,path);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m
    corridor=load('capsule_reused_corridor',base/'code/native_norm_corridor.py')
    exact,_,_=corridor.dependencies();rowwise,dense,domain,_=exact.load_dependencies()
    dense.validate_source()
    return corridor,exact,rowwise,dense,domain

def rational(value):
    _,E,_,_,_=dependencies()
    return E.exact(value)

def bitvalue(value):
    require(isinstance(value,str)and re.fullmatch('[0-9a-fA-F]{16}',value)is not None,'U64_HEX_16_REQUIRED')
    f=struct.unpack('>d',bytes.fromhex(value))[0]
    require(math.isfinite(f),'NONFINITE_BITS')
    return value.lower(),F.from_float(f)

def parse_capsule(capsule):
    require(isinstance(capsule,dict),'CAPSULE_OBJECT_REQUIRED')
    require(capsule.get('bass_commit')==BASS and capsule.get('binding_id')==BINDING,'SOURCE_BINDING_MISMATCH')
    require(capsule.get('source_sha256')==SOURCE,'DENSE_SOURCE_MISMATCH')
    require(capsule.get('query_entrypoint')==ENTRY,'QUERY_ENTRYPOINT_MISMATCH')
    require(capsule.get('clock')==CLOCK,'CLOCK_IDENTITY_MISMATCH')
    require(capsule.get('arithmetic_assumptions')==ARITHMETIC,'ARITHMETIC_ASSUMPTIONS_REQUIRED')
    provenance=capsule.get('provenance')
    require(isinstance(provenance,dict)and provenance.get('kind')in('MANUFACTURED_TEST','OWNER_ACCEPTED_NATIVE'),'PROVENANCE_KIND_REQUIRED')
    if provenance['kind']=='OWNER_ACCEPTED_NATIVE':
        for key in ('accepted_record_locator','model_identity','config_identity','epoch_identity'):
            require(isinstance(provenance.get(key),str)and bool(provenance[key]),'OWNER_IDENTITY_REQUIRED:'+key)
    values={};bits={}
    for name in ('t_old','t_new','h','delta_center','delta_exact_lo','delta_exact_hi'):
        bits[name],values[name]=bitvalue(capsule.get(name))
    for name in ('y_old','y_new','d0','d1','d2'):
        arr=capsule.get(name);require(isinstance(arr,list)and len(arr)==30,'STATE_DIMENSION_30_REQUIRED')
        decoded=[bitvalue(x)for x in arr];bits[name]=[x[0]for x in decoded];values[name]=[x[1]for x in decoded]
    delta=values['t_new']-values['t_old'];_,_,row,dense,_=dependencies()
    require(dense.MIN_NORMAL<=delta<=dense.FMAX,'POSITIVE_NORMAL_DELTA_REQUIRED')
    require(all(values[name]==delta for name in ('delta_center','delta_exact_lo','delta_exact_hi')),'STORED_DELTA_MISMATCH')
    require(values['h']>0,'POSITIVE_H_REQUIRED')
    q={'x0':values['t_old'],'x1':values['t_new'],'h':values['h'],'delta':delta,
       **{a:values[b]for a,b in (('y0','y_old'),('y1','y_new'),('d0','d0'),('d1','d1'),('d2','d2'))}}
    bounds=[dense.coordinate_bound(*v,delta)for v in zip(*(q[k]for k in ('y0','y1','d0','d1','d2')))]
    require(all(m<=dense.FMAX for b in bounds for _,m in b['guards']),'SOURCE_RANGE_BOUND_UNAVAILABLE')
    return {'schema':'vigilode.parsed-capsule.v1','bits':bits,'q':q,'power':row.state_power(q),
            'rho':[b['rho']for b in bounds],'provenance':dict(provenance),
            'provenance_review':'MANUFACTURED'if provenance['kind']=='MANUFACTURED_TEST'else'UNRESOLVED_OWNER_RECORD',
            'query_entrypoint':ENTRY,'clock':CLOCK,'source_sha256':SOURCE,**ceiling()}

def query_membership(capsule,clock_bits,state_bits):
    parsed=parse_capsule(capsule);q=parsed['q'];clock_hex,x=bitvalue(clock_bits)
    require(q['x0']<=x<=q['x1'],'QUERY_OUTSIDE_SEGMENT')
    require(isinstance(state_bits,list)and len(state_bits)==30,'QUERY_DIMENSION_30_REQUIRED')
    decoded=[bitvalue(v)for v in state_bits];theta=(x-q['x0'])/q['delta']
    _,_,_,dense,_=dependencies();p=dense.polynomial(q,theta)
    errors=[abs(v[1]-a)for v,a in zip(decoded,p)]
    endpoint='y_old'if x==q['x0']else'y_new'if x==q['x1']else None
    endpoint_ok=endpoint is None or [v[0]for v in decoded]==parsed['bits'][endpoint]
    member=endpoint_ok and all(e<=r for e,r in zip(errors,parsed['rho']))
    return {'status':'PASS_SOURCE_QUERY_MEMBERSHIP'if member else'QUERY_MEMBERSHIP_NOT_ESTABLISHED',
            'clock_bits':clock_hex,'theta':theta,'component_errors':errors,'component_rho':parsed['rho'],
            'endpoint':endpoint,'endpoint_bits_preserved':endpoint_ok,'provenance_review':parsed['provenance_review'],**ceiling()}

SCOPES={'r_query':'KNOT_SAME_METRIC_QUERY','alpha':'KNOT_RESIDUAL','epsilon_eval':'KNOT_EVALUATION',
        'source_error':'KNOT_SOURCE_TRANSPORT','B1':'WHOLE_CELL','B2':'WHOLE_CELL',
        'Jop':'WHOLE_LINKING_PATH','Tx':'WHOLE_CLOCK_STATE_PATH','regularity':'WHOLE_CLOCK_STATE_PATH',
        'mu':'WHOLE_CORRIDOR','E0':'INITIAL_STATE','continuation':'WHOLE_CORRIDOR'}

def premise(record,name,metric,signed=False):
    require(isinstance(record,dict),'BOUND_RECORD_REQUIRED:'+name)
    require(record.get('fixed_norm_identity')==metric,'BOUND_METRIC_MISMATCH:'+name)
    require(record.get('bass_commit')==BASS and record.get('binding_id')==BINDING,'BOUND_SOURCE_MISMATCH:'+name)
    require(record.get('clock')==CLOCK,'BOUND_CLOCK_MISMATCH:'+name)
    require(record.get('scope')==SCOPES[name],'BOUND_SCOPE_MISMATCH:'+name)
    require(record.get('proof_status')in('DECLARED','VERIFIED'),'BOUND_PROOF_STATUS_REQUIRED:'+name)
    require(isinstance(record.get('evidence'),str)and bool(record['evidence']),'BOUND_EVIDENCE_REQUIRED:'+name)
    if name in ('regularity','continuation'):
        require(record.get('value')is True,'REGULARITY_PREMISE_REQUIRED:'+name);return True
    v=rational(record.get('value'));require(signed or v>=0,'NEGATIVE_BOUND:'+name)
    if name=='source_error'and v==0:
        require(record.get('same_source_parameters')is True,'ZERO_SOURCE_ERROR_REQUIRES_EXPLICIT_SAME_SOURCE')
    return v

def residual_transfer(capsule,partition,cells,fixed_norm_identity):
    parsed=parse_capsule(capsule);q=parsed['q']
    require(isinstance(fixed_norm_identity,str)and bool(fixed_norm_identity),'FIXED_NORM_IDENTITY_REQUIRED')
    require(isinstance(partition,list)and len(partition)>=2,'POSITIVE_PARTITION_REQUIRED')
    t=[rational(v)for v in partition]
    require(t[0]==q['x0']and t[-1]==q['x1'],'PARTITION_ENDPOINT_COVERAGE_REQUIRED')
    require(all(a<b for a,b in zip(t,t[1:])),'PARTITION_MUST_STRICTLY_INCREASE')
    require(isinstance(cells,list)and len(cells)==len(t)-1,'ONE_CELL_PER_INTERVAL_REQUIRED')
    out=[]
    for j,(left,right,c)in enumerate(zip(t,t[1:],cells)):
        require(isinstance(c,dict),'CELL_OBJECT_REQUIRED')
        # Explicit endpoint binding also refuses gaps or overlaps in cell records.
        require(rational(c.get('left'))==left and rational(c.get('right'))==right,'CELL_PARTITION_MISMATCH')
        _,xi=bitvalue(c.get('knot_bits'));require(left<=xi<=right,'KNOT_OUTSIDE_CELL')
        membership=query_membership(capsule,c['knot_bits'],c.get('state_bits'))
        require(membership['status']=='PASS_SOURCE_QUERY_MEMBERSHIP','CELL_QUERY_MEMBERSHIP_FAILED')
        records=c.get('bounds');require(isinstance(records,dict),'CELL_BOUNDS_REQUIRED')
        names=('r_query','alpha','epsilon_eval','source_error','B1','B2','Jop','Tx','regularity')
        b={n:premise(records.get(n),n,fixed_norm_identity)for n in names}
        k=max(xi-left,right-xi);query=b['Jop']*b['r_query']
        mesh=(b['B2']+b['Jop']*b['B1']+b['Tx'])*k
        eta=b['alpha']+b['epsilon_eval']+b['source_error']+query+mesh
        out.append({'cell':j,'left':left,'right':right,'knot':xi,'kappa':k,'eta':eta,
                    'query_transport':query,'mesh_transport':mesh,'premises':records,'query_membership':membership})
    return {'schema':'vigilode.finite-cell-residual-transfer.v1','status':'PASS_CONDITIONAL_ALL_REAL_RESIDUAL_FORMULA',
            'eta':max(c['eta']for c in out),'cells':out,'partition':t,'all_real_cover':True,
            'Delta':q['delta'],'native_h':q['h'],'fixed_norm_identity':fixed_norm_identity,
            'proof_validation':'EXTERNAL_PREMISES_RETAIN_SUPPLIED_STATUS; NO_AUTOMATIC_ADMISSION',
            'provenance_review':parsed['provenance_review'],**ceiling()}

def join_barrier(transfer,radius,mu,E0,*,continuation,max_isolation_steps=128):
    require(transfer.get('status')=='PASS_CONDITIONAL_ALL_REAL_RESIDUAL_FORMULA','TRANSFER_REQUIRED')
    metric=transfer['fixed_norm_identity'];m=premise(mu,'mu',metric,signed=True);e=premise(E0,'E0',metric)
    premise(continuation,'continuation',metric)
    C,E,_,_,_=dependencies();C.step_limit(max_isolation_steps)
    R=C.polynomial(radius,E,'RADIUS')
    result=C.barrier_core(R,transfer['Delta'],m,transfer['eta'],e,E,limit=max_isolation_steps,
                         norm_identity=metric,provenance={'statement':'Conditional finite-cell residual transfer with supplied same-metric premises',
                         'fixed_norm_identity':metric,'mu':mu,'E0':E0,'continuation':continuation})
    return {'schema':'vigilode.capsule-conditional-barrier.v1','barrier':result,
            'transfer_eta':transfer['eta'],'domain_corridor':'SEPARATE_EXISTING_CORRIDOR_OBLIGATION',**ceiling()}

def jsonable(value):
    if isinstance(value,F):return {'numerator':str(value.numerator),'denominator':str(value.denominator)}
    if isinstance(value,dict):return {k:jsonable(v)for k,v in value.items()}
    if isinstance(value,(tuple,list)):return [jsonable(v)for v in value]
    return value
