from pathlib import Path
import gzip,json
P=Path(__file__).parent
PRIOR=P.parent/'m2_dae_native_stage_adjoint_20261127'
ENGINE=P.parent/'m2_dae_engine_adjoint_20261127'
def read(p,m):
    if not p.exists():p=Path(str(p)+'.gz')
    s=gzip.decompress(p.read_bytes()).decode() if p.suffix=='.gz' else p.read_text()
    return [json.loads(l.split(m+' ',1)[1])for l in s.splitlines()if l.startswith(m+' {')or l.startswith('test ')and ' ... '+m+' {'in l]
def compare(old,new,m,count,ignore=()):
    a,b=read(old,m),read(new,m);assert len(a)==len(b)==count,(m,len(a),len(b))
    eq=lambda x,y:{k:v for k,v in x.items()if k not in ignore}=={k:v for k,v in y.items()if k not in ignore}
    assert all(eq(x,y)for x,y in zip(a,b)),m
    return {'count':count,'logical':count,'exact':sum(x==y for x,y in zip(a,b)),'excluded_keys':list(ignore),'changed':[{'case':x.get('case'),'candidate':x.get('candidate'),'before':{k:x.get(k)for k in ignore},'after':{k:y.get(k)for k in ignore}}for x,y in zip(a,b)if x!=y]}
out={}
for m,count,ignore in [('NATIVE_STAGE',36,('allocation','heap')),('NATIVE_PACKET',37,()),('NATIVE_STATE',36,()),('NATIVE_ODE',18,()),('NATIVE_REFUSAL',1,())]:
    out[m]=compare(PRIOR/'portfolio.log',P/'prior_stage.log',m,count,ignore)
for m,count,ignore in [('ADJOINT_MODEL',4,()),('ADJOINT_TAYLOR',12,()),('ADJOINT_ODE',16,('median_ns','samples_ns')),('ADJOINT_FD',4,()),('ADJOINT_REFUSAL',1,())]:
    out[m]=compare(ENGINE/'adjoint.log',P/'prior_adjoint.log',m,count,ignore)
out['ENGINE_EPOCH']=compare(ENGINE/'engine.log',P/'prior_engine.log','ENGINE_EPOCH',28,())
out['ODE_COMPARE']=compare(ENGINE/'ode.log',P/'prior_ode.log','ODE_COMPARE',15,())
(P/'REPLAY_COMPARE.json').write_text(json.dumps(out,indent=2,sort_keys=True)+'\n')
print('REPLAY',sum(v['logical']for v in out.values()),sum(v['exact']for v in out.values()),[(k,v['logical'],v['exact'])for k,v in out.items()])
