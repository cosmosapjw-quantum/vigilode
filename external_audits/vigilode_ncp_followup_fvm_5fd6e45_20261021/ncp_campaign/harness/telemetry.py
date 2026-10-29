"""1 s host telemetry until STOP file exists. Usage: python3 telemetry.py CAMP"""
import json
import sys
import time
from pathlib import Path

CAMP = Path(sys.argv[1])
stop = CAMP / 'results/TELEMETRY.STOP'
out = CAMP / 'results/TELEMETRY.jsonl'


def cpu_times():
    rows = {}
    for line in open('/proc/stat'):
        if line.startswith('cpu'):
            f = line.split()
            v = list(map(int, f[1:9]))
            rows[f[0]] = (sum(v), v[3] + v[4])  # total, idle+iowait
    return rows


prev = cpu_times()
while not stop.exists():
    time.sleep(1.0)
    cur = cpu_times()
    busy = {k: round(1 - (cur[k][1] - prev[k][1]) / max(1, cur[k][0] - prev[k][0]), 4) for k in cur}
    prev = cur
    mem = {l.split(':')[0]: int(l.split()[1]) for l in open('/proc/meminfo')
           if l.startswith(('MemAvailable', 'MemFree', 'Shmem:'))}
    row = {'utc': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()), 'monotonic_ns': time.monotonic_ns(),
           'loadavg': open('/proc/loadavg').read().split()[:4], 'busy_total': busy.pop('cpu'),
           'busy_per_cpu': [busy[f'cpu{i}'] for i in range(len(busy))], 'meminfo_kb': mem}
    with open(out, 'a') as f:
        f.write(json.dumps(row, separators=(',', ':')) + '\n')
