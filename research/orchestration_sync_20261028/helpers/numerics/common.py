from pathlib import Path
from fractions import Fraction as F
import hashlib, importlib.util, json
BASE=Path(__file__).resolve().parent
EXPECTED={'coupled.py':'09a3325c584b895f8c1de44d3b64a458ec7e27f8eec843c7e15aa246a6088d47','geometry.py':'b998b83f83da581cf51d08a734b34ecade5215f90ca1e72df43ce129c0fb38a0','compact_system.py':'3ebf56b5a8f1f392681365ec0769364329f293f944a0de4a5add441c8cbc1c49'}

def prepare(unit):
    files=[]
    for p in sorted((BASE/'reuse').rglob('*')):
        if p.is_file() and '__pycache__' not in str(p):
            files.append({'path':str(p.relative_to(BASE)), 'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()})
    for name,digest in EXPECTED.items():
        assert hashlib.sha256((BASE/'reuse/source_intake/source'/name).read_bytes()).hexdigest()==digest,'SOURCE_IDENTITY_MISMATCH'
    metadata=json.loads((BASE/'reuse/source_intake/SOURCE_IDENTITIES.json').read_text())
    for row in metadata:
        p=BASE/'reuse/source_intake/source'/row['name']
        if p.exists(): assert hashlib.sha256(p.read_bytes()).hexdigest()==row['sha256'],'SOURCE_MANIFEST_MISMATCH'
    # Verify complete inherited subtree against its supplied identity before import.
    ident=json.loads((BASE/'reuse/deps/PRIOR_MATRIX_IDENTITY.json').read_text())
    for row in ident['files']:
        p=BASE/'reuse/deps/prior_matrix'/row['path']
        assert hashlib.sha256(p.read_bytes()).hexdigest()==row['sha256'],'DEPENDENCY_IDENTITY_MISMATCH'
    (unit/'SOURCE_HASH_MANIFEST.json').write_text(json.dumps({'before_scientific_import':True,'files':files,'source_identities':metadata},indent=2)+'\n')
    spec=importlib.util.spec_from_file_location('unchanged_chemical',BASE/'reuse/code/chemical_polynomial.py')
    c=importlib.util.module_from_spec(spec);spec.loader.exec_module(c)
    return c

def dump(path,obj):
    def convert(x):
        if isinstance(x,F): return str(x)
        if isinstance(x,dict): return {k:convert(v) for k,v in x.items()}
        if isinstance(x,(list,tuple)): return [convert(v) for v in x]
        return x
    path.write_text(json.dumps(convert(obj),indent=2)+'\n')

def binding(c):
    return {'bass_commit':c.BASS,'binding_id':c.BINDING,'clock':c.prior().prior().source().CLOCK,'evidence':'Pinned unchanged source and dependency manifests; synthetic supplied diagnostic coefficients, not a physical corridor'}
PROV={'kind':'DIAGNOSTIC_SYNTHETIC_WEIGHTS','evidence':'Explicit rational positive diagnostic weights; no observed owner scales'}
CEILINGS={'query_rounding_bound':None,'actual_query_rounding_bound':None,'actual_model_enclosure_verified':False,'actual_ode_solution_enclosure':False,'whole_domain_certificate':False,'VerifiedModelProvider':None,'production':'HOLD','globalphysicalcompletion':None,'L4':False,'central_progress_delta':0}

def mul(a,b): return min(x*y for x in a for y in b),max(x*y for x in a for y in b)
def add(a,b):return a[0]+b[0],a[1]+b[1]
def neg(a):return -a[1],-a[0]
def interval(x,positive=False,nonnegative=False):
    if not isinstance(x,(tuple,list)) or len(x)!=2:raise ValueError('INTERVAL_SHAPE')
    try:a,b=map(F,x)
    except (ValueError,TypeError,OverflowError):raise ValueError('NONFINITE_OR_INVALID_INTERVAL')
    if a>b or (positive and a<=0) or (nonnegative and a<0):raise ValueError('INVALID_INTERVAL')
    return a,b

def reject(fn):
    try:fn()
    except (ValueError,AssertionError):return True
    raise AssertionError('INVALID_INPUT_NOT_REJECTED')
