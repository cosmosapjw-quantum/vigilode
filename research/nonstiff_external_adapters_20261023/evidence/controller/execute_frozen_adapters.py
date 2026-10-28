from pathlib import Path
import subprocess,os,json,time,hashlib,sys
root=Path(__file__).resolve().parent
source=Path('/home/cosmosapjw/vigilode-nonstiff-adapters-source-7htbCC')
study=source/'research/nonstiff_external_adapters_20261023'
commit=sys.argv[1]
assert subprocess.check_output(['git','-C',str(source),'rev-parse','HEAD'],text=True).strip()==commit
assert not subprocess.check_output(['git','-C',str(source),'status','--porcelain=v1'],text=True).strip()
final=root/'frozen-execution';final.mkdir(exist_ok=False)
env={**os.environ,'VIGILODE_ADAPTER_SOURCE_COMMIT':commit,'CARGO_TARGET_DIR':str(final/'target'),'CARGO_BUILD_JOBS':'2','CARGO_PROFILE_DEV_DEBUG':'0','CARGO_PROFILE_TEST_DEBUG':'0','VIGILODE_JULIA_DEPOT':str(root/'julia-development/julia-depot'),'VIGILODE_JULIA_EXECUTABLE':'/home/cosmosapjw/.julia/juliaup/julia-1.12.7+0.x64.linux.gnu/bin/julia'}
receipts=[]
def run(label,args):
 before=time.monotonic(); print(label,flush=True)
 with (final/(label+'.stdout.log')).open('w') as out,(final/(label+'.stderr.log')).open('w') as err:
  p=subprocess.run([str(x) for x in args],cwd=source,env=env,stdout=out,stderr=err)
 r={'label':label,'command':[str(x) for x in args],'cwd':str(source),'exit_code':p.returncode,'elapsed_seconds':time.monotonic()-before,'adapter_source_commit':commit,'stdout':label+'.stdout.log','stderr':label+'.stderr.log'}
 for k in ['stdout','stderr']:r[k+'_sha256']=hashlib.sha256((final/r[k]).read_bytes()).hexdigest()
 receipts.append(r);(final/'COMMANDS.json').write_text(json.dumps(receipts,indent=2)+'\n');print(json.dumps(r),flush=True)
 if p.returncode:sys.exit(p.returncode)
run('rustc-version',['rustc','-Vv']);run('cargo-version',['cargo','--version'])
manifest=study/'adapters/diffsol/Cargo.toml'
run('cargo-metadata',['cargo','metadata','--locked','--offline','--format-version','1','--manifest-path',manifest])
run('build-measurement',['cargo','build','--locked','--offline','--profile','measurement','--manifest-path',manifest])
binary=final/'target/measurement/vigilode-nonstiff-diffsol-adapter'
(final/'BINARY_IDENTITY.json').write_text(json.dumps({'adapter_source_commit':commit,'adapter_source_tree':subprocess.check_output(['git','-C',str(source),'rev-parse','HEAD^{tree}'],text=True).strip(),'diffsol_binary':str(binary),'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'julia_launcher_sha256':hashlib.sha256((study/'adapters/julia/run.sh').read_bytes()).hexdigest(),'environment':{k:env[k] for k in ['VIGILODE_ADAPTER_SOURCE_COMMIT','CARGO_TARGET_DIR','CARGO_BUILD_JOBS','CARGO_PROFILE_DEV_DEBUG','CARGO_PROFILE_TEST_DEBUG','VIGILODE_JULIA_DEPOT','VIGILODE_JULIA_EXECUTABLE']}},indent=2)+'\n')
run('diffsol-protocol',['python3',study/'adapters/diffsol/test_protocol.py','--binary',binary,'--analyzer',source/'research/nonstiff_explicit_20261023/analyze_bench.py','--out',final/'diffsol-protocol'])
run('adapter-preflight',['python3',study/'verify_adapters.py','--diffsol-adapter',binary,'--julia-adapter',study/'adapters/julia/run.sh','--adapter-source-commit',commit,'--preflight-out',final/'preflight'])
print('FINAL_PREFLIGHT_PASS_CAMPAIGN_NOT_STARTED',flush=True)
