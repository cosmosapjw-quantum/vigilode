#!/usr/bin/env python3
"""Host telemetry every 10 s into results/TELEMETRY.jsonl until killed."""
import json
import subprocess
import sys
import time
from pathlib import Path

out = Path(sys.argv[1]) / "results" / "TELEMETRY.jsonl"


def cpu():
    v = list(map(int, open("/proc/stat").readline().split()[1:]))
    return sum(v), v[3] + (v[4] if len(v) > 4 else 0)


prev = cpu()
while True:
    time.sleep(10)
    cur = cpu()
    mem = {}
    for x in open("/proc/meminfo"):
        k = x.split(":")[0]
        if k in ("MemTotal", "MemAvailable", "SwapFree"):
            mem[k] = int(x.split()[1]) * 1024
    top = subprocess.run(["ps", "-eo", "pid,pcpu,rss,comm", "--sort=-pcpu"], capture_output=True,
                         text=True).stdout.splitlines()[1:13]
    row = {"utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), "monotonic": time.monotonic(),
           "load": open("/proc/loadavg").read().split()[:3],
           "busy": 1 - (cur[1] - prev[1]) / max(cur[0] - prev[0], 1), "mem": mem, "top": top}
    prev = cur
    with open(out, "a") as f:
        f.write(json.dumps(row) + "\n")
