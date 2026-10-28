from pathlib import Path
import sys,importlib.util,json,time,resource
sys.dont_write_bytecode=True
U=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('bounded_child',U.parents[1]/'bounded_child.py');runner=importlib.util.module_from_spec(spec);spec.loader.exec_module(runner)
config={'argv':[sys.executable,str(U/'worker.py')],'cwd':str(U),'timeout_seconds':30,'cpu_seconds':20,'address_space_bytes':512*1024**2,'output_prefix':str(U/'RUN')}
p=U/'RUN_INPUT.json';p.write_text(json.dumps(config,indent=2)+'\n')
start=time.monotonic();before=resource.getrusage(resource.RUSAGE_SELF);code=runner.run(p);after=resource.getrusage(resource.RUSAGE_SELF)
receipt=json.loads((U/'RUN.receipt.json').read_text());receipt['runner_parent_overhead']={'user_seconds':after.ru_utime-before.ru_utime,'system_seconds':after.ru_stime-before.ru_stime,'wall_minus_child_seconds':time.monotonic()-start-receipt['wall_seconds'],'host_global_overhead':'UNKNOWN_NOT_ZERO','scope':'science, reference algebra and saved SVG all in measured child'}
(U/'RUN_RECEIPT.json').write_text(json.dumps(receipt,indent=2)+'\n')
raise SystemExit(code)
