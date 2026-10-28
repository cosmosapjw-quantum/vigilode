#!/usr/bin/env bash
# One-command start of the BASS/rei NCP C1 campaign from the local workstation.
#
#   bash external_audits/vigilode_bassrei_ncp_c1_20261022/launch_from_local.sh EXEC_COMMIT [--detach]
#
# Run inside a checkout that contains EXEC_COMMIT. Every exit prints its reason.
#   exit 4  EXEC not pushed (git branch -r --contains) or lacks the node's PREREGISTRATION.md
#   exit 5  host key mismatch / verification failure (nothing is sent)
#   exit 2  NCP unreachable (e.g. powered off)
#   exit 3  preflight gate failed (see preflight.txt): cpuset lacks 8-63, a live C1 run or heavy
#           foreign processes, MemAvailable < 48 GiB, /root free < 20 GiB + the Julia depot size
#   exit 6  upload/unpack/start failed
# Then: archive the source subset at EXEC + its ls-tree, upload to
# /root/vigilode-bassrei-ncp-c1-<UTC>/, start run_campaign.sh truly detached (setsid nohup,
# all fds redirected, affinity 8-63), confirm its PID. --detach returns right after; the
# default waits with short ssh polls (failures reported) up to 125 min, then fetch_results.sh.
# The key is used only via ssh -i; host-key verification stays on; BatchMode (no prompts).
set -uo pipefail
COMMIT=${1:?usage: launch_from_local.sh EXEC_COMMIT [--detach]}
DETACH=${2:-}
KEY=/home/cosmosapjw/.ssh/ncp_key
HOST=root@49.50.142.2
OPTS=(-i "$KEY" -o IdentitiesOnly=yes -o BatchMode=yes -o ConnectTimeout=20)
SSH=(ssh "${OPTS[@]}" -p 22 "$HOST")
SCP=(scp "${OPTS[@]}" -P 22)
MEM_REQ_KB=$(( 48 * 1024 * 1024 ))      # 16 Julia servers x 1.5 GiB + 36 workers x 0.4 GiB + margin
DISK_MARGIN_B=$(( 20 * 1024 * 1024 * 1024 ))
die() { echo "[launch] $2" >&2; exit "$1"; }
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(git -C "$HERE" rev-parse --show-toplevel) || die 4 "not inside a git checkout"
FULL=$(git -C "$REPO" rev-parse --verify "$COMMIT^{commit}" 2>/dev/null) || die 4 "unknown commit $COMMIT"
TREE=$(git -C "$REPO" rev-parse "$FULL^{tree}")
[ -n "$(git -C "$REPO" branch -r --contains "$FULL" 2>/dev/null)" ] || die 4 "EXEC $FULL is not on any remote branch (push it first)"
git -C "$REPO" cat-file -e "$FULL:research/bassrei_ncp_wp_c1_20261022/PREREGISTRATION.md" 2>/dev/null \
  || die 4 "EXEC $FULL lacks research/bassrei_ncp_wp_c1_20261022/PREREGISTRATION.md"
STAMP=$(date -u +%Y%m%dT%H%M%SZ)
CAMP=/root/vigilode-bassrei-ncp-c1-$STAMP
LOCAL=$HOME/vigilode-campaign/bassrei-ncp-c1-$STAMP
mkdir -p "$LOCAL"
PATHS=(Cargo.toml Cargo.lock rust-toolchain.toml .cargo/config.toml crates fixtures
  research/ncp_followup_20261021/BASS_REI_FVM_REFERENCE.py
  research/bassrei_p02_fixed_grid_20261021/reference
  research/bassrei_p03_block_lowrank_schur_20261022/reference
  research/bassrei_p04_source_exchange_stress_20261022/reference
  research/bassrei_p04_source_exchange_stress_20261022/source
  research/bassrei_ncp_wp_c1_20261022/PREREGISTRATION.md
  external_audits/vigilode_bassrei_ncp_c1_20261022
  external_audits/vigilode_ncp_intensive_120m_v1_20261021/harness/julia-env
  external_audits/vigilode_ncp_intensive_120m_v1_20261021/harness/diffsol-adapter/Cargo.lock)

echo "[launch] preflight"
"${SSH[@]}" 'set +e
echo "CPUSET=$(cat /sys/fs/cgroup/cpuset.cpus.effective 2>/dev/null || grep Cpus_allowed_list /proc/self/status | cut -f2)"
echo "NPROC=$(nproc)"
echo "MEMAVAIL_KB=$(awk "/MemAvailable/ {print \$2}" /proc/meminfo)"
echo "DISK_AVAIL_B=$(df -B1 --output=avail /root | tail -1 | tr -dc 0-9)"
echo "LOAD1=$(cut -d" " -f1 /proc/loadavg)"
echo "LIVE_C1=$(pgrep -fc "[r]un_campaign.sh" || true)"
echo "HEAVY=$(top -b -n 2 -d 1 2>/dev/null | awk "/^top -/ {n++} n==2 && \$1 ~ /^[0-9]+\$/ && \$9+0 >= 25 && \$12 != \"top\" {printf \"%s:%s:%s;\", \$1, \$12, \$9}")"
for d in /root/vigilode-julia-c1-20261022/depot /root/VIGILODE_COMPONENT_20261018T134235Z/env/julia-depot /root/VIGILODE_COMPONENT_20261018T134235Z/env/depot \
         /root/VIGILODE_COMPONENT_20261018T134235Z/julia-depot /dev/shm/vigilode_ncp_intensive_120m_v1_20261021T062109Z/julia-depot /root/.julia; do
  if [ -d "$d/packages/OrdinaryDiffEq" ] && [ -d "$d/packages/Sundials" ]; then
    echo "DEPOT=$d"; echo "DEPOT_BYTES=$(timeout 120 du -sb "$d" | cut -f1)"; break
  fi
done
ls -d /root/vigilode-bassrei-ncp-c1-* 2>/dev/null | sed "s/^/EARLIER=/" || true
echo PREFLIGHT_END' > "$LOCAL/preflight.txt" 2> "$LOCAL/ssh_preflight.err"
RC=$?
if [ $RC -ne 0 ] || ! grep -q PREFLIGHT_END "$LOCAL/preflight.txt"; then
  if grep -qE "REMOTE HOST IDENTIFICATION HAS CHANGED|Host key verification failed" "$LOCAL/ssh_preflight.err"; then
    die 5 "HOST KEY MISMATCH or verification failure for $HOST: not launching (see $LOCAL/ssh_preflight.err)"
  fi
  die 2 "NCP unreachable or preflight failed (ssh exit $RC): $(tail -2 "$LOCAL/ssh_preflight.err" | tr '\n' ' ')"
fi
val() { grep "^$1=" "$LOCAL/preflight.txt" | head -1 | cut -d= -f2-; }
REASONS=()
python3 - "$(val CPUSET)" <<'PY' || REASONS+=("cpuset $(val CPUSET) does not contain 8-63")
import sys
s = set()
for part in sys.argv[1].split(","):
    part = part.strip()
    if not part:
        continue
    a, _, b = part.partition("-")
    s |= set(range(int(a), int(b or a) + 1))
sys.exit(0 if set(range(8, 64)) <= s else 1)
PY
[ "$(val MEMAVAIL_KB)" -ge "$MEM_REQ_KB" ] 2>/dev/null || REASONS+=("MemAvailable $(val MEMAVAIL_KB) kB < $MEM_REQ_KB kB (48 GiB)")
NEED_B=$(( DISK_MARGIN_B + $(val DEPOT_BYTES || echo 0) ))
[ "$(val DISK_AVAIL_B)" -ge "$NEED_B" ] 2>/dev/null || REASONS+=("/root free $(val DISK_AVAIL_B) B < 20 GiB + depot $(val DEPOT_BYTES) B")
[ "$(val LIVE_C1)" = "0" ] || REASONS+=("a C1 run_campaign.sh is live on NCP ($(val LIVE_C1))")
[ -z "$(val HEAVY)" ] || REASONS+=("heavy foreign processes (pid:cmd:%cpu): $(val HEAVY)")
awk -v l="$(val LOAD1)" 'BEGIN{exit !(l < 4)}' || REASONS+=("1-min load $(val LOAD1) >= 4")
if [ ${#REASONS[@]} -gt 0 ]; then
  printf '[launch] preflight refused:\n' >&2; printf '  - %s\n' "${REASONS[@]}" >&2
  printf '%s\n' "${REASONS[@]}" > "$LOCAL/preflight_refused.txt"
  exit 3
fi
echo "[launch] preflight ok: $(tr '\n' ' ' < "$LOCAL/preflight.txt" | cut -c1-300)"

echo "[launch] archive $FULL"
git -C "$REPO" archive --format=tar.gz -o "$LOCAL/src.tar.gz" "$FULL" -- "${PATHS[@]}" || die 6 "git archive failed"
git -C "$REPO" ls-tree -r "$FULL" -- "${PATHS[@]}" > "$LOCAL/SOURCE_LS_TREE.txt"
python3 -s - "$FULL" "$TREE" "$STAMP" "$CAMP" "${PATHS[@]}" > "$LOCAL/IDENTITY_LOCAL.json" <<'PY'
import json, sys
c, t, s, camp, *paths = sys.argv[1:]
print(json.dumps({"commit": c, "tree": t, "stamp": s, "ncp_dir": camp, "paths": paths}, indent=1))
PY

echo "[launch] upload to $CAMP"
"${SSH[@]}" "mkdir -p $CAMP/src" || die 6 "mkdir failed on NCP"
"${SCP[@]}" "$LOCAL/src.tar.gz" "$LOCAL/SOURCE_LS_TREE.txt" "$LOCAL/IDENTITY_LOCAL.json" "$HOST:$CAMP/" \
  || die 6 "upload failed"
START=$("${SSH[@]}" "cd $CAMP && tar xzf src.tar.gz -C src && sha256sum src.tar.gz > src.tar.gz.sha256 || { echo UNPACK_FAILED; exit 1; }
setsid nohup taskset -c 8-63 bash src/external_audits/vigilode_bassrei_ncp_c1_20261022/run_campaign.sh $CAMP > $CAMP/campaign.log 2>&1 < /dev/null &
sleep 3
if [ -f $CAMP/campaign.pid ] && kill -0 \$(cat $CAMP/campaign.pid) 2>/dev/null; then echo STARTED \$(cat $CAMP/campaign.pid); else echo START_FAILED; fi")
echo "$START" > "$LOCAL/start.txt"
case "$START" in STARTED*) ;; *) die 6 "start failed on NCP: $START (see $CAMP/campaign.log)";; esac
echo "$CAMP" > "$LOCAL/NCP_DIR.txt"
echo "[launch] campaign started: $CAMP ($START); local receipts: $LOCAL"
echo "[launch] later: bash $HERE/fetch_results.sh $CAMP $LOCAL"
[ "$DETACH" = "--detach" ] && exit 0

for i in $(seq 1 125); do
  sleep 60
  "${SSH[@]}" "test -f $CAMP/CAMPAIGN_DONE" 2>> "$LOCAL/poll.err"
  rc=$?
  if [ $rc -eq 0 ]; then echo "[launch] campaign done"; break; fi
  if [ $rc -eq 255 ]; then echo "[launch] poll $i: ssh failure (see $LOCAL/poll.err); continuing" >&2; continue; fi
  [ $((i % 10)) -eq 0 ] && "${SSH[@]}" "tail -1 $CAMP/logs/PHASES.log" 2>> "$LOCAL/poll.err"
done
bash "$HERE/fetch_results.sh" "$CAMP" "$LOCAL"
