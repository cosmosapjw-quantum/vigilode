from pathlib import Path
import gzip,json
P=Path(__file__).parent;PRIOR=P.parent/'m2_dae_nonlinear_adjoint_dopri_20261127';AFFINE=P.parent/'m2_dae_engine_adjoint_20261127'
def read(p,m):
    if not p.exists():p=Path(str(p)+'.gz')
    s=gzip.decompress(p.read_bytes()).decode() if p.suffix=='.gz' else p.read_text()
    return [json.loads(l.split(m+' ',1)[1])for l in s.splitlines()if l.startswith(m+' {')or l.startswith('test ')and ' ... '+m+' {'in l]
out={}
def compare(old,new,m,count,ignore=()):
    a,b=read(old,m),read(new,m);assert len(a)==len(b)==count,(m,len(a),len(b))
    key=lambda x:{k:v for k,v in x.items()if k not in ignore}
    assert all(key(x)==key(y)for x,y in zip(a,b)),m
    out[m]={'count':count,'logical':count,'exact':sum(x==y for x,y in zip(a,b)),'excluded_keys':list(ignore)}
for m,count,ignore in [('NL_ADJOINT',20,('median_ns','samples_ns')),('NL_NUMERIC',10,()),('NL_PRIMAL',5,())]:compare(PRIOR/'adjoint.log',P/'prior_nonlinear.log',m,count,ignore)
for m,count,ignore in [('DOPRI_KERNEL',16,('median_ns','samples_ns','fixed_median_ns','fixed_samples_ns')),('DOPRI_EDGE',1,())]:compare(PRIOR/'dopri.log',P/'prior_dopri.log',m,count,ignore)
compare(PRIOR/'split.log',P/'prior_stage.log','SPLIT_STAGE',30)
for m,count,ignore in [('ADJOINT_MODEL',4,()),('ADJOINT_TAYLOR',12,()),('ADJOINT_ODE',16,('median_ns','samples_ns')),('ADJOINT_FD',4,()),('ADJOINT_REFUSAL',1,())]:compare(AFFINE/'adjoint.log',P/'prior_affine.log',m,count,ignore)
(P/'REPLAY_COMPARE.json').write_text(json.dumps(out,indent=2,sort_keys=True)+'\n')
print('REPLAY',sum(r['count']for r in out.values()),sum(r['exact']for r in out.values()),[(k,v['logical'],v['exact'])for k,v in out.items()])
