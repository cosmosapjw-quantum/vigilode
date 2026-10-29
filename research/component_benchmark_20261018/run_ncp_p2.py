"""Authorized P2 command driver; keeps the P1 ledger and all failed arms."""
import json, os, pathlib, subprocess, sys, time, shutil, hashlib
root=pathlib.Path('/root/VIGILODE_COMPONENT_20261018T134235Z/repo')
base=root.parent
os.chdir(root)
sys.path.insert(0,str(root/'tools'))
from component_benchmark.budget import reserve,close
from component_benchmark.runner import atomic_json,source_binding
from component_benchmark.protocol import digest
campaign=root/'research/component_benchmark_20261018'
p1=campaign/'ncp_execution_01'
run=campaign/'ncp_screening_01'
run.mkdir(exist_ok=False)
logs=run/'commands';logs.mkdir()
python=str(base/'env/python/bin/python')
os.environ['PYTHONPATH']=str(root/'tools')
os.environ['PATH']=str(base/'env/python/bin')+':'+os.environ['PATH']
for key in ('OMP_NUM_THREADS','OPENBLAS_NUM_THREADS','MKL_NUM_THREADS','BLIS_NUM_THREADS','RAYON_NUM_THREADS','JULIA_NUM_THREADS'):
    os.environ[key]='1'
ledger=campaign/'NCP_LEASE_LEDGER.json'
if not(p1/'P1_QUEUES_FINISHED.json').is_file():raise RuntimeError('P1 queues still active')
prior=json.loads(ledger.read_text());active=[x for x in prior['leases'] if 'end' not in x]
if len(active)!=1 or active[0]['phase']!='P1':raise RuntimeError('P1 lease identity mismatch')
close(ledger,active[0]['id'])
lease=reserve(ledger,'P2',20*3600-60,__import__('platform').node(),persistent=True)
atomic_json(run/'context.json',{'lease_id':lease,'ledger':str(ledger),'source_binding_sha256':digest(source_binding(root)),
    'exec_sha':subprocess.check_output(['git','rev-parse','HEAD']).decode().strip(),'pid':os.getpid(),'host':__import__('platform').node(),
    'driver_sha256':hashlib.sha256(pathlib.Path(__file__).read_bytes()).hexdigest()})
def command(label,argv,required=False):
    start=time.time()
    with (logs/(label+'.stdout')).open('wb') as out,(logs/(label+'.stderr')).open('wb') as err:
        p=subprocess.run(argv,stdout=out,stderr=err)
    atomic_json(logs/(label+'.json'),{'argv':argv,'cwd':str(root),'started_unix':start,'elapsed_seconds':time.time()-start,'exit_code':p.returncode,'source_binding_sha256':digest(source_binding(root))})
    print(label,p.returncode,flush=True)
    if required and p.returncode:raise RuntimeError(label+' failed')
    return p.returncode
gates=run/'prerequisites'
command('prerequisite_validation',[python,str(campaign/'prepare_p2.py'),'--root',str(root),'--p1',str(p1),'--output',str(gates)],True)
for label in ('screening','allocation'):
    target=run/label
    command(label+'_plan',[python,'tools/component_benchmark.py','plan','--config',str(gates/(label+'_config_gated.json')),'--output',str(target),'--phase','P2','--kind','screening'],True)
    command(label,[python,'tools/component_benchmark.py','run','--plan',str(target/'plan.json'),'--output',str(target),'--ledger',str(ledger),'--lease-id',lease,'--concurrency','56','--hpc'])
    for name in ('references.json','references.json.provenance.json'):
        shutil.copy2(p1/name,target/name)
    command(label+'_analysis',[python,'tools/component_benchmark.py','analyze','--output',str(target)])
atomic_json(run/'P2_QUEUES_FINISHED.json',{'time':time.time(),'lease_remains_active':True,'scope':'screening execution and fixed gates only; confirmation selection pending'})
