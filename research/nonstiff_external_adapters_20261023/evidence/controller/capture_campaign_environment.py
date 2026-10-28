from pathlib import Path
import os,platform,json,time,resource,importlib.metadata,hashlib
r=Path(__file__).resolve().parent
cg=Path('/sys/fs/cgroup')/Path('/proc/self/cgroup').read_text().strip().split('::')[-1].lstrip('/');chain=[]
while True:
 d={'path':str(cg),'values':{}}
 for name in ['cpu.max','memory.max','memory.high','cpuset.cpus.effective','cpu.weight']:
  p=cg/name
  if p.is_file():d['values'][name]=p.read_text().strip()
 chain.append(d)
 if cg==Path('/sys/fs/cgroup'):break
 cg=cg.parent
j=Path('/home/cosmosapjw/.julia/juliaup/julia-1.12.7+0.x64.linux.gnu')
d={'captured_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'timing_authority':'diagnostic_only','platform':platform.platform(),'cpu_model':next(x.split(':',1)[1].strip() for x in Path('/proc/cpuinfo').read_text().splitlines() if x.startswith('model name')),'affinity':sorted(os.sched_getaffinity(0)),'logical_cpus':os.cpu_count(),'loadavg':os.getloadavg(),'memory_rlimit_as':resource.getrlimit(resource.RLIMIT_AS),'proc_meminfo':Path('/proc/meminfo').read_text(),'actual_cgroup_ancestry':chain,'python_version':platform.python_version(),'python_packages':{name:importlib.metadata.version(name) for name in ['numpy','matplotlib','scipy']},'julia_threads':1,'julia_openblas_threads':1,'diffsol_parallelism':'Pinned package/faer defaults; no affinity or thread override by adapter; not asserted equivalent to native WorkCounters','vode_baseline_thread_policy':'Original baseline retained; Python callback overhead included; original NumPy OpenBLAS24 observed','task_runtime_paths':{p:Path(p).exists() for p in ['/workspace/scratch/0dc1e721cf82/task_runtime/installed/bin','/workspace/scratch/0dc1e721cf82/task_runtime/cargo']},'julia_runtime_files':{str(p.relative_to(j)):hashlib.sha256(p.read_bytes()).hexdigest() for p in [j/'bin/julia',j/'lib/julia/sys.so',j/'lib/libjulia.so.1.12.7'] if p.is_file()}}
(r/'CAMPAIGN_ENVIRONMENT.json').write_text(json.dumps(d,indent=2)+'\n');print(d['cpu_model'],d['loadavg'],d['python_packages'])
