import json, os, pathlib, subprocess, sys, time, shutil, hashlib

root=pathlib.Path('/root/VIGILODE_COMPONENT_20261018T134235Z/repo')
base=root.parent
os.chdir(root)
sys.path.insert(0,str(root/'tools'))
from component_benchmark.budget import reserve, close
from component_benchmark.runner import atomic_json,source_binding
from component_benchmark.protocol import digest
campaign=root/'research/component_benchmark_20261018'
run=campaign/'ncp_execution_01'
run.mkdir(exist_ok=False)
logs=run/'commands';logs.mkdir()
python=str(base/'env/python/bin/python')
os.environ['PYTHONPATH']=str(root/'tools')
os.environ['PATH']=str(base/'env/python/bin')+':'+os.environ['PATH']
for key in ('OMP_NUM_THREADS','OPENBLAS_NUM_THREADS','MKL_NUM_THREADS','BLIS_NUM_THREADS','RAYON_NUM_THREADS','JULIA_NUM_THREADS'):
    os.environ[key]='1'
ledger=campaign/'NCP_LEASE_LEDGER.json'
prior=json.loads(ledger.read_text())
active=[x for x in prior['leases'] if 'end' not in x]
if len(active)!=1 or active[0]['phase']!='P0':raise RuntimeError('P0 transfer identity mismatch')
close(ledger,active[0]['id'])
lease=reserve(ledger,'P1',12*3600-60,__import__('platform').node(),persistent=True)
atomic_json(run/'context.json',{'lease_id':lease,'ledger':str(ledger),'source_binding_sha256':digest(source_binding(root)),
    'exec_sha':subprocess.check_output(['git','rev-parse','HEAD']).decode().strip(),'pid':os.getpid(),'host':__import__('platform').node()})

def command(label,argv,required=False):
    start=time.time()
    with (logs/(label+'.stdout')).open('wb') as out,(logs/(label+'.stderr')).open('wb') as err:
        p=subprocess.run(argv,stdout=out,stderr=err)
    atomic_json(logs/(label+'.json'),{'argv':argv,'cwd':str(root),'started_unix':start,'elapsed_seconds':time.time()-start,'exit_code':p.returncode,'source_binding_sha256':digest(source_binding(root))})
    print(label,p.returncode,flush=True)
    if required and p.returncode:raise RuntimeError(label+' failed')
    return p.returncode

command('host_topology',['lscpu','-J'])
command('baseline_identity',[python,'tools/validate_all_dag_completion.py'],True)
frozen=run/'frozen'
command('freeze',[python,'tools/component_benchmark/campaign.py','freeze','--root',str(root),'--env-root',str(base/'env'),'--output',str(frozen)],True)
art=json.loads((campaign/'evidence/component_build/portable_artifacts.json').read_text())['artifacts']
entries=[]
for item in art:
    if item.get('argv')!=['--test-threads=1']:continue
    path=base/'assets/binaries'/pathlib.Path(item['executable']).name
    entries.append({key:item[key] for key in ('id','sha256','source_paths','source_hashes','argv')}|{'executable':str(path)})
manifest=run/'component_manifest.json'
atomic_json(manifest,{'schema':'vigilode-component-contract-execution-v1','exec_sha':subprocess.check_output(['git','rev-parse','HEAD']).decode().strip(),'source_root':str(root),'entries':entries})
command('component_contracts',[python,'-m','component_benchmark.component_contracts','--manifest',str(manifest),'--output',str(run/'components'),'--ledger',str(ledger),'--lease-id',lease,'--concurrency','56','--hpc'])
refs=run/'references'
command('reference_plan',[python,'tools/component_benchmark.py','plan','--config',str(frozen/'reference_config.json'),'--output',str(refs),'--phase','P1','--kind','reference'],True)
command('references',[python,'tools/component_benchmark.py','run','--plan',str(refs/'plan.json'),'--output',str(refs),'--ledger',str(ledger),'--lease-id',lease,'--concurrency','56','--hpc'])
command('collect_references',[python,'tools/component_benchmark/campaign.py','collect-references','--run',str(refs),'--output',str(run/'references.json')])
command('model_probes',[python,'tools/component_benchmark/probe_campaign.py','--config',str(frozen/'screening_config.json'),'--output',str(run/'probes'),'--ledger',str(ledger),'--lease-id',lease,'--concurrency','56','--hpc','--root',str(root)])
atomic_json(run/'P1_QUEUES_FINISHED.json',{'time':time.time(),'lease_remains_active':True,'scope':'raw execution only; per-case gates pending'})
