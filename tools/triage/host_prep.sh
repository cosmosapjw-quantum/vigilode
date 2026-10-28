#!/bin/bash
# Record the measurement host of a component-triage campaign: HOST.json.
#
# Usage: tools/triage/host_prep.sh [--apply] [--output HOST.json] [--force] [--help]
#
# Records: CPU model, cpufreq governor per CPU, intel_pstate no_turbo / cpufreq boost, SMT state,
# isolcpus (kernel command line and /sys/devices/system/cpu/isolated), load average, kernel, `rustc -V`,
# and the affinity mask of this shell (taskset -p $$, else /proc/self/status Cpus_allowed_list).
#
# --apply tries to set every governor to `performance` (needs root). A failure is reported and recorded in
# HOST.json (`apply.status = "failed"`) and the script exits 1: it never continues silently.
#
# Consumers (triage_check TIMING inputs) treat wall-clock data as diagnostic when `wall_clock_admissible`
# is false: any governor other than `performance`, or an affinity mask that is not a strict subset of the
# online CPUs (unpinned).
set -euo pipefail

apply=0
output="HOST.json"
force=0
while [ $# -gt 0 ]; do
  case "$1" in
    --apply) apply=1 ;;
    --output) output="${2:?--output needs a path}"; shift ;;
    --force) force=1 ;;
    -h|--help) sed -n '2,17p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
  shift
done
if [ -e "$output" ] && [ "$force" -ne 1 ]; then
  echo "immutable output exists: $output (pass --force to overwrite)" >&2
  exit 2
fi

apply_status="not_requested"
apply_detail=""
if [ "$apply" -eq 1 ]; then
  apply_status="ok"
  shopt -s nullglob
  govs=(/sys/devices/system/cpu/cpu[0-9]*/cpufreq/scaling_governor)
  if [ ${#govs[@]} -eq 0 ]; then
    apply_status="failed"
    apply_detail="no cpufreq scaling_governor files (no cpufreq driver or virtualised host)"
  else
    for g in "${govs[@]}"; do
      if ! echo performance > "$g" 2>/dev/null; then
        apply_status="failed"
        apply_detail="cannot write $g (needs root)"
        break
      fi
    done
  fi
  echo "apply governor=performance: $apply_status ${apply_detail}" >&2
fi

export HP_APPLY_STATUS="$apply_status" HP_APPLY_DETAIL="$apply_detail" HP_OUTPUT="$output" HP_SHELL_PID="$$"
export HP_RUSTC="$(rustc -V 2>/dev/null || echo unavailable)"
export HP_TASKSET="$(taskset -p $$ 2>/dev/null || echo unavailable)"
python3 - <<'PY'
import glob, json, os, platform, re

def read(path):
    try:
        with open(path) as handle:
            return handle.read().strip()
    except OSError:
        return None

def cpu_list(text):
    out = set()
    for part in (text or "").split(","):
        part = part.strip()
        if not part:
            continue
        if "-" in part:
            a, b = part.split("-")
            out.update(range(int(a), int(b) + 1))
        else:
            out.add(int(part))
    return sorted(out)

model = None
for line in (read("/proc/cpuinfo") or "").splitlines():
    if line.startswith("model name"):
        model = line.split(":", 1)[1].strip()
        break
governors = {}
for path in sorted(glob.glob("/sys/devices/system/cpu/cpu[0-9]*/cpufreq/scaling_governor"),
                   key=lambda p: int(re.search(r"cpu(\d+)", p).group(1))):
    governors[re.search(r"(cpu\d+)", path).group(1)] = read(path)
online = cpu_list(read("/sys/devices/system/cpu/online"))
status = read(f"/proc/{os.environ['HP_SHELL_PID']}/status") or ""
allowed = next((l.split(":", 1)[1].strip() for l in status.splitlines() if l.startswith("Cpus_allowed_list")), None)
cmdline = read("/proc/cmdline") or ""
isol = re.search(r"isolcpus=(\S+)", cmdline)
pinned = allowed is not None and online and set(cpu_list(allowed)) < set(online)
all_performance = bool(governors) and all(g == "performance" for g in governors.values())
doc = {
    "schema": "vigilode-triage-host-v1",
    "cpu_model": model,
    "governors": governors,
    "all_governors_performance": all_performance,
    "no_turbo": read("/sys/devices/system/cpu/intel_pstate/no_turbo"),
    "cpufreq_boost": read("/sys/devices/system/cpu/cpufreq/boost"),
    "smt_control": read("/sys/devices/system/cpu/smt/control"),
    "smt_active": read("/sys/devices/system/cpu/smt/active"),
    "isolcpus_cmdline": isol.group(1) if isol else None,
    "isolated": read("/sys/devices/system/cpu/isolated"),
    "online": read("/sys/devices/system/cpu/online"),
    "loadavg": read("/proc/loadavg"),
    "kernel": platform.release(),
    "kernel_version": platform.version(),
    "rustc": os.environ["HP_RUSTC"],
    "taskset": os.environ["HP_TASKSET"],
    "cpus_allowed_list": allowed,
    "pinned": bool(pinned),
    "apply": {"status": os.environ["HP_APPLY_STATUS"], "detail": os.environ["HP_APPLY_DETAIL"] or None},
    "wall_clock_admissible": bool(all_performance and pinned),
    "wall_clock_rule": "admissible only if every governor is performance and the affinity mask is a strict subset "
                       "of the online CPUs; otherwise wall-clock data are diagnostic",
}
text = json.dumps(doc, indent=1, sort_keys=True) + "\n"
with open(os.environ["HP_OUTPUT"], "w") as handle:
    handle.write(text)
print(text, end="")
PY
if [ "$apply_status" = "failed" ]; then
  echo "error: --apply failed: $apply_detail (recorded in $output)" >&2
  exit 1
fi
