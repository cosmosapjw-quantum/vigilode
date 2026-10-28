"""Exact rational, signed-mu affine barriers, with inherited capsule row algebra.

External premise statuses are data. This module never evaluates a model, native
norm or lift and never promotes a manufactured capsule to owner provenance.
"""
from fractions import Fraction as F
from pathlib import Path
from functools import lru_cache
import hashlib, importlib.util, json
ROOT=Path(__file__).resolve().parents[1]

@lru_cache(maxsize=1)
def source():
    path=ROOT/'deps/capsule/code/capsule_residual.py'
    identity=json.loads((ROOT/'deps/CAPSULE_COPY_IDENTITY.json').read_text())
    for row in identity['files']:
        if hashlib.sha256((ROOT/'deps/capsule'/row['path']).read_bytes()).hexdigest()!=row['sha256']:
            raise ValueError('CAPSULE_DEPENDENCY_BYTE_IDENTITY_MISMATCH:'+row['path'])
    spec=importlib.util.spec_from_file_location('piecewise_preserved_capsule',path)
    c=importlib.util.module_from_spec(spec);spec.loader.exec_module(c)
    return c

def require(ok,code):
    if not ok:raise ValueError(code)

def exact(x):return source().rational(x)

def limits(max_subcells,refinement,max_isolation_steps):
    require(isinstance(max_subcells,int)and not isinstance(max_subcells,bool)and max_subcells>0,'POSITIVE_INTEGER_SUBCELL_CAP_REQUIRED')
    require(isinstance(refinement,int)and not isinstance(refinement,bool)and refinement>0 and refinement&(refinement-1)==0,'POWER_OF_TWO_REFINEMENT_REQUIRED')
    source().dependencies()[0].step_limit(max_isolation_steps)

def scalar_piece(d,mu,eta,r,*,fixed_norm_identity='SCALAR_EXACT_MANUFACTURED_METRIC',max_isolation_steps=128):
    C,E,_,_,_=source().dependencies();C.step_limit(max_isolation_steps)
    d,m,e,r=map(exact,(d,mu,eta,r))
    require(d>0,'POSITIVE_PHYSICAL_WIDTH_REQUIRED');require(e>=0 and r>=0,'NONNEGATIVE_ETA_RADIUS_REQUIRED')
    a=d*m;require(a<1,'SUBDIVISION_REQUIRED');b=d*(m*r+e);s=max(b,b/(1-a))
    B0=s-b;B1=(1-a)*s-b
    cert=C.barrier_core([r,s],d,m,e,r,E,limit=max_isolation_steps,norm_identity=fixed_norm_identity,
         provenance={'statement':'Exact signed-mu affine supersolution in fixed incoming-radius affine class','fixed_norm_identity':fixed_norm_identity})
    return {'status':'PASS_EXACT_AFFINE_PIECE'if cert['status']=='PASS_CONDITIONAL_BARRIER'else'INCONCLUSIVE',
            'd':d,'mu':m,'eta':e,'incoming':r,'a':a,'b':b,'slope':s,'outgoing':r+s,'B0':B0,'B1':B1,
            'radius_power':[r,s],'barrier':cert,**source().ceiling()}

def construct_chain(partition,etas,mus,E0,*,fixed_norm_identity='SCALAR_EXACT_MANUFACTURED_METRIC',max_subcells=4096,refinement=1,max_isolation_steps=128):
    limits(max_subcells,refinement,max_isolation_steps)
    require(isinstance(fixed_norm_identity,str)and bool(fixed_norm_identity),'FIXED_NORM_IDENTITY_REQUIRED')
    require(isinstance(partition,list)and len(partition)>=2,'POSITIVE_PARTITION_REQUIRED')
    t=[exact(x)for x in partition];require(all(a<b for a,b in zip(t,t[1:])),'PARTITION_MUST_STRICTLY_INCREASE')
    require(isinstance(etas,list)and isinstance(mus,list)and len(etas)==len(mus)==len(t)-1,'ONE_ETA_MU_PER_CELL_REQUIRED')
    es=[exact(v)for v in etas];ms=[exact(v)for v in mus];r=exact(E0)
    require(r>=0 and all(e>=0 for e in es),'NONNEGATIVE_ETA_RADIUS_REQUIRED')
    # No subcell arrays or scalar certificates exist until every count is known.
    plan=[];total=0
    for l,h,m in zip(t,t[1:],ms):
        target=2*(h-l)*m;n=1
        if m>0 and target>1:
            ceil=(target.numerator+target.denominator-1)//target.denominator
            n=1<<(ceil-1).bit_length()
        n*=refinement;total+=n;plan.append(n)
    if total>max_subcells:
        return {'status':'INCONCLUSIVE_RESOURCE_LIMIT','required_subcells':total,'max_subcells':max_subcells,'planned_per_cell':plan,'subcell_certificates_computed':0,**source().ceiling()}
    out=[]
    for j,(l,h,e,m,n)in enumerate(zip(t,t[1:],es,ms,plan)):
        d=(h-l)/n
        for k in range(n):
            piece=scalar_piece(d,m,e,r,fixed_norm_identity=fixed_norm_identity,max_isolation_steps=max_isolation_steps)
            piece.update(cell=j,subcell=k,left=l+k*d,right=l+(k+1)*d)
            out.append(piece);r=piece['outgoing']
    return {'schema':'vigilode.exact-piecewise-barrier.v1','status':'PASS_CONDITIONAL_PIECEWISE_BARRIER'if all(p['status']=='PASS_EXACT_AFFINE_PIECE'for p in out)else'INCONCLUSIVE',
            'partition':t,'planned_per_cell':plan,'required_subcells':total,'pieces':out,'initial':exact(E0),'outgoing':r,
            'fixed_norm_identity':fixed_norm_identity,'physical_clock':'actual right-left; native h unused',**source().ceiling()}

def interval_record(record,left,right,*,outer=False,gamma=None):
    require(exact(record.get('left'))==left and exact(record.get('right'))==right,'BOUND_INTERVAL_MISMATCH')
    if outer:
        require(record.get('corridor')=='R_PLUS_GAMMA','OUTER_CORRIDOR_SCOPE_REQUIRED')
        require(exact(record.get('gamma'))==gamma,'OUTER_GAMMA_MISMATCH')

def enlarged_row(c,terms,strict,name,parsed,piece,u,gamma,*,precision=32,max_isolation_steps=128):
    """Original source row F, composed exactly on this physical subcell."""
    C,E,row,_,_=source().dependencies();q=parsed['q'];rho=[F(0)]+parsed['rho']
    H,Fpower,sigma,*_=row.row_data(c,terms,q,rho)
    left=(piece['left']-q['x0'])/q['delta'];right=(piece['right']-q['x0'])/q['delta']
    # Inspected signature: substitute(p,a,b)=p(a+(b-a)t), not p(a+b*t).
    composed=E.substitute(Fpower,left,right)
    support=C.support_core([F(terms.get(i+1,0))for i in range(30)],u,E,precision=precision)
    K=E._trusted_exact(support['Kbar']);outer=[piece['incoming']+gamma,piece['slope']]
    G=C.add(composed,[-K*x for x in outer]);cert=C.sign(G,E,strict,max_isolation_steps)
    return {'name':name,'strict':strict,'constant':c,'terms':terms,'H_power':H,'F_power':Fpower,'box_support':sigma,
            'theta_interval':[left,right],'F_subcell_power':composed,'outer_radius_power':outer,'G_power':E.trim(G),'dual_support':support,
            'sufficient_sign':cert,'status':'PASS_CONDITIONAL_CORRIDOR'if cert['status']=='PASS'else'INCONCLUSIVE',
            'negative_G_is_not_physical_failure':True}

def capsule_chain(capsule,partition,cells,mu_records,E0_record,upper_scales,gamma,continuation,*,max_subcells=4096,refinement=1,sqrt_precision=32,max_isolation_steps=128):
    c=source();C,E,_,dense,domain=c.dependencies();limits(max_subcells,refinement,max_isolation_steps);C.precision_check(sqrt_precision)
    u=C.vector(upper_scales,33,E,'SCALES');require(all(v>0 for v in u),'PHYSICAL_UPPER_SCALES_MUST_BE_POSITIVE')
    g=exact(gamma);require(g>0,'POSITIVE_OUTER_GAMMA_REQUIRED');metric=C.fixed_norm_id(u,E)
    transfer=c.residual_transfer(capsule,partition,cells,metric);parsed=c.parse_capsule(capsule);t=transfer['partition']
    require(isinstance(mu_records,list)and len(mu_records)==len(t)-1,'ONE_MU_RECORD_PER_CELL_REQUIRED')
    mus=[]
    for left,right,record in zip(t,t[1:],mu_records):
        mus.append(c.premise(record,'mu',metric,signed=True));interval_record(record,left,right,outer=True,gamma=g)
    r=c.premise(E0_record,'E0',metric);require(exact(E0_record.get('at'))==t[0],'INITIAL_ENDPOINT_MISMATCH')
    c.premise(continuation,'continuation',metric);interval_record(continuation,t[0],t[-1],outer=True,gamma=g)
    chain=construct_chain(t,[v['eta']for v in transfer['cells']],mus,r,fixed_norm_identity=metric,max_subcells=max_subcells,refinement=refinement,max_isolation_steps=max_isolation_steps)
    if chain['status']=='INCONCLUSIVE_RESOURCE_LIMIT':return {'transfer':transfer,'chain':chain,'status':chain['status'],**c.ceiling()}
    binding=domain.source_binding();physical=domain.rows(binding)
    finite=[('finite_'+kind+'_w'+str(i),dense.FMAX,{i:s},False)for i in range(31)for kind,s in(('plus',1),('minus',-1))]
    rows=[]
    for p in chain['pieces']:
        def group(data):
            rs=[enlarged_row(co,terms,strict,name,parsed,p,u,g,precision=sqrt_precision,max_isolation_steps=max_isolation_steps)for name,co,terms,strict in data]
            return {'count':len(rs),'rows':rs,'status':'PASS_CONDITIONAL_CORRIDOR'if all(r['status']=='PASS_CONDITIONAL_CORRIDOR'for r in rs)else'INCONCLUSIVE'}
        rows.append({'cell':p['cell'],'subcell':p['subcell'],'left':p['left'],'right':p['right'],'physical19':group(physical),'finite62':group(finite)})
    joined=chain['status']=='PASS_CONDITIONAL_PIECEWISE_BARRIER'and all(r[k]['status']=='PASS_CONDITIONAL_CORRIDOR'for r in rows for k in('physical19','finite62'))
    return {'schema':'vigilode.capsule-piecewise-enlarged-join.v1','status':'PASS_CONDITIONAL_PIECEWISE_DOMAIN_JOIN'if joined else'INCONCLUSIVE',
            'transfer':transfer,'chain':chain,'rows_by_subcell':rows,'gamma':g,'upper_scales':u,'fixed_norm_identity':metric,'mu_records':mu_records,
            'E0_record':E0_record,'continuation':continuation,'provenance_review':parsed['provenance_review'],
            'proof_validation':'SUPPLIED_DECLARED_OR_VERIFIED_STATUS_RETAINED; NO_AUTOMATIC_ADMISSION',
            'conditional_actual_solution_assumptions':['same fixed upper metric and clock and source','all-real eta and signed mu on independently justified R+gamma corridor',
                'positive actual scales bounded above by frozen upper_scales','f C1 on open neighbourhood','required compact interior domain and continuation','supplied initial/inherited radius'],**c.ceiling()}
