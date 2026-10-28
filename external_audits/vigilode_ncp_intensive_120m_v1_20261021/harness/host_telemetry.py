import json, time, os, pathlib, subprocess
job=pathlib.Path('/dev/shm/vigilode_ncp_intensive_120m_v1_20261021T062109Z')
out=job/'HOST_TELEMETRY.jsonl'
def readcpu():
    s=open('/proc/stat').readline().split(); v=list(map(int,s[1:])); idle=v[3]+(v[4] if len(v)>4 else 0); return sum(v),idle
def sample():
    total,idle=readcpu(); load=list(map(float,open('/proc/loadavg').read().split()[:3])); mem={}
    for x in open('/proc/meminfo'):
        if x.split(':')[0] in ('MemTotal','MemAvailable','MemFree','Cached','SwapFree'):
            k,v=x.split(':',1); mem[k]=int(v.strip().split()[0])*1024
    try: psi=open('/proc/pressure/memory').read().strip()
    except OSError: psi=None
    mhz=[]
    for x in open('/proc/cpuinfo'):
        if x.startswith('cpu MHz'): mhz.append(float(x.split(':')[1]))
    proc=[]
    for x in subprocess.run(['ps','-eo','pid,ppid,rss,pcpu,comm'],capture_output=True,text=True).stdout.splitlines()[1:]:
        if 'run_campaign.py' in x or 'component_benchmark_native' in x or 'python3' in x:
            proc.append(x.strip())
    return {'utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'monotonic':time.monotonic(),'load_1_5_15':load,'cpu_busy_ratio_since_prior':None if not hasattr(sample,'prev') else 1-(idle-sample.prev[1])/max(total-sample.prev[0],1),'memory_bytes':mem,'memory_pressure':psi,'reported_cpu_mhz_min_median_max':[min(mhz),sorted(mhz)[len(mhz)//2],max(mhz)] if mhz else None,'processes':proc}
with open(out,'a',buffering=1) as f:
    while True:
        x=sample(); f.write(json.dumps(x,separators=(',',':'))+'\n'); sample.prev=readcpu(); time.sleep(10)
