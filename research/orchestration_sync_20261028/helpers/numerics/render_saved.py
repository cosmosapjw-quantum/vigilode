from pathlib import Path
import sys,json,importlib.util,time,resource
sys.dont_write_bytecode=True
BASE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('bounded_child',BASE.parent/'bounded_child.py');runner=importlib.util.module_from_spec(spec);spec.loader.exec_module(runner)
for name,stem in [('bass_crossblock','geometry_block'),('vigilode_interval_remainder','uniform_gamma')]:
    unit=BASE/name;config={'argv':['/usr/bin/inkscape',str(unit/(stem+'.svg')),'--export-type=png','--export-filename='+str(unit/(stem+'.png'))],'cwd':str(unit),'timeout_seconds':29,'cpu_seconds':19,'address_space_bytes':512*1024**2,'output_prefix':str(unit/'RENDER')}
    p=unit/'RENDER_INPUT.json';p.write_text(json.dumps(config,indent=2)+'\n')
    start=time.monotonic();before=resource.getrusage(resource.RUSAGE_SELF);rc=runner.run(p);after=resource.getrusage(resource.RUSAGE_SELF)
    rr=json.loads((unit/'RENDER.receipt.json').read_text());sr=json.loads((unit/'RUN_RECEIPT.json').read_text())
    aggregate={'unit':name,'science_receipt':'RUN_RECEIPT.json','render_receipt':'RENDER.receipt.json','sum_user_system_cpu_seconds':sr['user_seconds']+sr['system_seconds']+rr['user_seconds']+rr['system_seconds'],'sum_child_wall_seconds':sr['wall_seconds']+rr['wall_seconds'],'maximum_child_peak_rss_KiB':max(sr['maxrss_native'],rr['maxrss_native']),'caps':{'cpu_seconds':20,'wall_seconds':30,'address_space_bytes':512*1024**2},'render_parent_overhead':{'user_seconds':after.ru_utime-before.ru_utime,'system_seconds':after.ru_stime-before.ru_stime,'wall_minus_render_seconds':time.monotonic()-start-rr['wall_seconds']},'host_global_overhead':'UNKNOWN_NOT_ZERO','scientific_rerun':False}
    (unit/'AGGREGATE_RESOURCES.json').write_text(json.dumps(aggregate,indent=2)+'\n')
    if rc:raise SystemExit(rc)
    assert aggregate['sum_user_system_cpu_seconds']<20 and aggregate['sum_child_wall_seconds']<30
