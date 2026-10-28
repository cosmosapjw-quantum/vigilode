"""Source-specific represented quartic center radius; no model/norm evaluation."""
from pathlib import Path
from fractions import Fraction as F
import hashlib,importlib.util,json,math,struct,sys
ROOT=Path(__file__).resolve().parents[1]
SOURCE_SHA='50f8d288abbf54339b7a6efe4fea5809f71d40c37532f50bf9c304d4587e3fa3'
EPS=F(1,2**53);ETA=F(1,2**1075);FMAX=F.from_float(sys.float_info.max);MIN_NORMAL=F(1,2**1022)
CLOCK_MAP='identity_x_to_native_time_declared_for_fixture'
class Refusal(ValueError):
    def __init__(self,code):self.code=code;super().__init__(code)
def rat(q):return {'numerator':str(q.numerator),'denominator':str(q.denominator)}
def readrat(q):return F(int(q['numerator']),int(q['denominator']))
def sha(payload):return hashlib.sha256(payload).hexdigest()
def canonical(obj):
    try:return sha(json.dumps(obj,sort_keys=True,separators=(',',':'),allow_nan=False).encode())
    except (TypeError,ValueError):raise Refusal('MALFORMED_INPUT')
def bits(value):return struct.unpack('>Q',struct.pack('>d',value))[0]
def frombits(value):return struct.unpack('>d',struct.pack('>Q',value))[0]
def decode(value):
    if isinstance(value,bool) or not isinstance(value,(str,float,int)):raise Refusal('MALFORMED_BINARY64')
    try:
        if isinstance(value,str):
            if not value.lower().startswith(('0x','-0x','+0x')):raise ValueError()
            f=float.fromhex(value)
        else:f=float(value)
    except (ValueError,OverflowError):raise Refusal('MALFORMED_BINARY64')
    if not math.isfinite(f):raise Refusal('NONFINITE_INPUT')
    return f,F.from_float(f)
def validate_source():
    if sha((ROOT/'theory/reference/dense_output_v2.rs').read_bytes())!=SOURCE_SHA:raise Refusal('SOURCE_BYTE_IDENTITY_MISMATCH')
def parse(inp):
    validate_source()
    if not isinstance(inp,dict):raise Refusal('MALFORMED_INPUT')
    if inp.get('source_sha256')!=SOURCE_SHA:raise Refusal('SOURCE_IDENTITY_MISMATCH')
    if inp.get('clock_mapping')!=CLOCK_MAP:raise Refusal('CLOCK_MAPPING_UNSUPPLIED_OR_MISMATCH')
    floats={};exact={}
    for key in ('x0','x1','h'):
        floats[key],exact[key]=decode(inp.get(key))
    if exact['h']<=0:raise Refusal('INVALID_H_METADATA')
    n=None
    for key in ('y0','y1','d0','d1','d2'):
        arr=inp.get(key)
        if not isinstance(arr,list) or len(arr)==0 or (n is not None and len(arr)!=n):raise Refusal('INVALID_COEFFICIENT_SHAPE')
        n=len(arr);pairs=[decode(x) for x in arr];floats[key]=[x[0] for x in pairs];exact[key]=[x[1] for x in pairs]
    delta=exact['x1']-exact['x0']
    if delta<=0:raise Refusal('NONPOSITIVE_SPAN')
    if delta<MIN_NORMAL:raise Refusal('SUBNORMAL_SPAN_UNSUPPORTED')
    if delta>FMAX:raise Refusal('NONFINITE_SPAN_UNSUPPORTED')
    represented=float(delta)
    if F.from_float(represented)!=delta:raise Refusal('INEXACT_SPAN_UNSUPPORTED')
    floats['delta']=represented;exact['delta']=delta
    return floats,exact,n

def coordinate_bound(y0,y1,d0,d1,d2,delta,*,eta=ETA):
    Y0,Y1,D0,D1,D2=map(abs,(y0,y1,d0,d1,d2));R=lambda b:EPS*b+eta;A=lambda b:b+R(b);Ec=EPS+eta
    Ba=D2;Ea=R(D2);Bb=A(D1+D2);Eb=Ea+R(D1+D2);Bd=Bb;Ed=Eb+R(Bb);Bq=A(D0+Bb);Eq=Ed+R(D0+Bb)
    El=Ec*Y0+R(Y0);Ek=Eq+Ec*(D0+D1+D2)+R(Bq);Bm=A(Y1+Bq);Em=Ek+R(Y1+Bq);En=Em+R(Bm);Eeval=El+En+R(Y0+Bm)
    guards=[('D1_PLUS_D2',D1+D2),('D0_PLUS_Bb',D0+Bb),('Y1_PLUS_Bq',Y1+Bq),('Y0_PLUS_Bm',Y0+Bm)]
    beta=2*EPS+eta/delta+eta;L=abs(y1-y0)+D0+D1+D2;rho=Eeval+L*beta;C=D0/4+4*D1/27+27*D2/256
    return {'rho':rho,'chord':C,'total':C+rho,'beta':beta,'L':L,'Eeval':Eeval,'guards':guards,'trace':{'Ba':Ba,'Ea':Ea,'Bb':Bb,'Eb':Eb,'Bd':Bd,'Ed':Ed,'Bq':Bq,'Eq':Eq,'El':El,'Ek':Ek,'Bm':Bm,'Em':Em,'En':En,'Eeval':Eeval}}

def outward(q):
    if q<0:raise Refusal('NEGATIVE_EXPORT_RADIUS')
    if q>FMAX:raise Refusal('FINITE_RADIUS_EXPORT_UNAVAILABLE')
    f=float(q)
    if F.from_float(f)<q:f=math.nextafter(f,math.inf)
    if not math.isfinite(f) or F.from_float(f)<q:raise Refusal('FINITE_RADIUS_EXPORT_UNAVAILABLE')
    return f

def ceiling():
    return {'actual_binding_query_rounding_bound':None,'whole_domain_certificate':False,'provider_constructed':False,'solution_enclosure':False,'actual_BASS_segment_supplied':False,'physical_calls':{'rhs':0,'jacobian':0,'ode':0,'ncp':0,'native_norm':0},'production':'HOLD','global_physical_completion':None,'L4':False,'canonical':'27/88_UNCHANGED'}

def certify(inp):
    f,q,n=parse(inp);coords=[]
    for i,values in enumerate(zip(*(q[k] for k in ('y0','y1','d0','d1','d2')))):
        bound=coordinate_bound(*values,q['delta'])
        for name,magnitude in bound['guards']:
            if magnitude>FMAX:raise Refusal('SUFFICIENT_UNIFORM_RANGE_BOUND_UNAVAILABLE:'+str(i)+':'+name)
        coords.append({'coordinate':i,'rho':rat(bound['rho']),'chord_radius':rat(bound['chord']),'total_radius':rat(bound['total']),'lipschitz':rat(bound['L']),'evaluation_error':rat(bound['Eeval']),'recurrence':{k:rat(v) for k,v in bound['trace'].items()},'range_guards':[{'guard':name,'magnitude':rat(mag),'passed':mag<=FMAX} for name,mag in bound['guards']]})
    tube=None
    if n==30:
        radii=[outward(readrat(c['total_radius'])) for c in coords]
        if any(max(abs(a),abs(b))+F.from_float(r)>FMAX for a,b,r in zip(q['y0'],q['y1'],radii)):raise Refusal('FINITE_TUBE_ENVELOPE_EXPORT_UNAVAILABLE')
        tube={'binding_id':'8cb268e8de741e284cf494013f1e0748d27f03c5e77ba0c64413ae92af082d16','u':[f['x0'].hex()]+[v.hex() for v in f['y0']],'v':[f['x1'].hex()]+[v.hex() for v in f['y1']],'r':[0.0.hex()]+[r.hex() for r in radii]}
    float_bits={k:([f'{bits(x):016x}' for x in v] if isinstance(v,list) else f'{bits(v):016x}') for k,v in f.items()}
    return {'schema':'vigilode.native-quartic-source-family-certificate.v1','source_sha256':SOURCE_SHA,'input_sha256':canonical(inp),'dimension':n,'scope':'FIXED_REPRESENTED_COEFFICIENT_FAMILY_CONDITIONAL_ON_DECLARED_IEEE_TRACE','source_family_uniform_bound':'EXACT_COMPONENTWISE_RHO_UNDER_DECLARED_ASSUMPTIONS','uniform_query_domain':'All finite binary64 clocks in represented closed endpoint interval','arithmetic_assumptions':['RN ties-to-even','gradual underflow; FTZ/DAZ disabled','separate source-ordered binary64 operations; no FMA contraction/reassociation/excess precision'],'synthetic_fixture':inp.get('synthetic_fixture') is True,'clock_mapping':CLOCK_MAP,'delta':rat(q['delta']),'normalization_beta':rat(2*EPS+ETA/q['delta']+ETA),'epsilon':rat(EPS),'eta':rat(ETA),'coordinates':coords,'represented_input_bits':float_bits,'tube':tube,'tube_geometry_only':True,'export_radius_exact_domination':None if tube is None else all(F.from_float(float.fromhex(r))>=readrat(c['total_radius']) for r,c in zip(tube['r'][1:],coords)),'exact_endpoint_pointwise_error_bound':rat(F(0)),'range_refusal_meaning':'Sufficient family-bound unavailable; no assertion of native impossibility.','actual_domain_certificate_joined':False,**ceiling()}

def polynomial(exact,theta):
    return [(1-theta)*a+theta*b+theta*(1-theta)*(c+theta*d+theta*theta*e) for a,b,c,d,e in zip(*(exact[k] for k in ('y0','y1','d0','d1','d2')))]
def chord(exact,theta):return [(1-theta)*a+theta*b for a,b in zip(exact['y0'],exact['y1'])]
def query_parameter(inp,clock):
    f,q,n=parse(inp);x,xx=decode(clock)
    if not q['x0']<=xx<=q['x1']:raise Refusal('QUERY_OUTSIDE_REPRESENTED_SEGMENT')
    return x,xx,(xx-q['x0'])/q['delta'],f,q,n

def load_membership():
    dep=ROOT/'inputs/reused_membership';identity=json.loads((ROOT/'inputs/REUSED_MEMBERSHIP_IDENTITY.json').read_text())
    for name,h in identity['source_sha256'].items():
        if sha((dep/name).read_bytes())!=h:raise Refusal('REUSED_MEMBERSHIP_BYTE_IDENTITY_MISMATCH')
    code=dep/'code';sys.path.insert(0,str(code))
    try:
        spec=importlib.util.spec_from_file_location('unchanged_membership_dependency',code/'float_query_membership.py');module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
    finally:sys.path.remove(str(code))
    return module
