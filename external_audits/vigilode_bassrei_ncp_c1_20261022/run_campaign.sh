#!/usr/bin/env bash
# BASS/rei NCP C1 work-precision campaign driver (runs on NCP, detached).
#
#   bash run_campaign.sh CAMP      (CAMP holds src/, SOURCE_LS_TREE.txt, IDENTITY_LOCAL.json)
#
# Hard cap: 120 min wall from this script's start, setup included.
#   setup   0-20   verify source, build runners; diffsol + Julia in parallel (setup hard cap 20 min)
#   parity  -25    RHS parity of every adapter vs the VigilODE kernel
#   ladder  -70    references first (alone), then work-precision ladders (cores 8-59, 16 Julia
#                  servers); VG-1/VG-3 allocation, rejection and cross-core identity in parallel (60-63)
#   abba    -105   accuracy-matched paired cost blocks (2 warm-up + 7)
#   measurement end 112 min: the watchdog TERMs this script; every registered child session
#                  (solvers, Julia servers, diffsol) is killed
#   final   112-120 analysis (clipped to the time left), hashes, OUT.tar.gz (tmp + rename), CAMPAIGN_DONE
# Every run's timeout is clipped to its phase deadline minus a reserve. Every row is fsync'ed.
# On abort (TERM/INT/watchdog) the trap kills the phase drivers and all registered children,
# then finalizes the partial data (verdict INVALID if incomplete). The script, and so every
# descendant, runs on cores 8-63 (cores 0-7 stay free); BLAS uses one thread everywhere.
#
# Dry-run overrides (local tests only; they make PLAN.json unofficial => verdict INVALID):
#   NCPC1_ALLOWED_CPUS (8-63), NCPC1_LADDER_CORES (8-59), NCPC1_VG_CORESET (60-63),
#   NCPC1_PARITY_CORESET (8), NCPC1_CASES / NCPC1_RUNGS (all), NCPC1_CAP_MIN (120).
set -uo pipefail
# The detached, non-login shell on NCP lacks the rustup bin dir (attempt 1,
# 20261022T063359Z, ended INVALID in setup with "cargo: command not found").
[ -d /root/.cargo/bin ] && export PATH=/root/.cargo/bin:$PATH
export RUSTUP_TOOLCHAIN=1.94.1
CAMP=$(readlink -f "$1")
SRC=$CAMP/src
H=$SRC/external_audits/vigilode_bassrei_ncp_c1_20261022/harness
mkdir -p "$CAMP/logs" "$CAMP/results" "$CAMP/tmp" "$CAMP/run/children"
LOG=$CAMP/logs
phase() { echo "[$(date -u +%H:%M:%S)] $*" | tee -a "$LOG/PHASES.log"; }
echo $$ > "$CAMP/campaign.pid"
date -u +%Y-%m-%dT%H:%M:%SZ > "$CAMP/CAMPAIGN_START_UTC.txt"
T0=$(date +%s)
CAP_S=$(( ${NCPC1_CAP_MIN:-120} * 60 ))
FINAL_RESERVE_S=480
MEAS_END_S=$(( CAP_S - FINAL_RESERVE_S ))           # 112 min
MONO0=$(python3 -s -c 'import time; print(time.monotonic())')
mono_at() {  # absolute monotonic deadline at min(minute $1, measurement end)
  python3 -s -c "print($MONO0 + min($1 * 60, $MEAS_END_S))"
}
left_s() { echo $(( CAP_S - ($(date +%s) - T0) )); }

taskset -cp "${NCPC1_ALLOWED_CPUS:-8-63}" $$ > "$LOG/affinity.txt" 2>&1 || phase "warning: taskset failed"
export NCPC1_PIN=1 PYTHONDONTWRITEBYTECODE=1 OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 \
  JULIA_NUM_THREADS=1 RAYON_NUM_THREADS=1 TMPDIR=$CAMP/tmp NCPC1_CHILD_REG=$CAMP/run/children

PHASE_PID=""; VG_PID=""; TELEM_PID=""; WATCHDOG_PID=""
kill_all() {
  for p in $PHASE_PID $VG_PID; do kill -TERM "$p" 2>/dev/null; done
  sleep 3
  for p in $PHASE_PID $VG_PID; do kill -KILL "$p" 2>/dev/null; done
  ( cd "$H" && python3 -s -c 'import common; print("killed", common.kill_registered())' ) >> "$LOG/kill_all.log" 2>&1
}

FINALIZED=0
finalize() {
  [ $FINALIZED -eq 1 ] && return
  FINALIZED=1
  trap '' TERM INT
  if [ -n "$WATCHDOG_PID" ]; then pkill -P "$WATCHDOG_PID" 2>/dev/null; kill "$WATCHDOG_PID" 2>/dev/null; fi
  phase "final: stopping drivers and registered children"
  kill_all
  [ -n "$TELEM_PID" ] && kill "$TELEM_PID" 2>/dev/null
  python3 -s "$H/verify_source.py" "$CAMP" --end > "$LOG/verify_source_end.out" 2>&1
  BUDGET=$(( $(left_s) - 90 )); [ $BUDGET -lt 30 ] && BUDGET=30
  phase "final: analysis (budget ${BUDGET}s)"
  NCPC1_RESULTS=$CAMP/results timeout "$BUDGET" python3 -s "$H/analyze.py" "$CAMP" > "$LOG/analyze.out" 2>&1 \
    || phase "final: analysis exit $?"
  date -u +%Y-%m-%dT%H:%M:%SZ > "$CAMP/CAMPAIGN_END_UTC.txt"
  ( cd "$CAMP" && find results raw logs REPORT.md CAMPAIGN_START_UTC.txt CAMPAIGN_END_UTC.txt IDENTITY_LOCAL.json \
      SOURCE_LS_TREE.txt -type f 2>/dev/null | sort | xargs -d '\n' sha256sum > OUTPUT_SHA256SUMS.txt
    tar czf OUT.tar.gz.tmp results raw logs REPORT.md OUTPUT_SHA256SUMS.txt CAMPAIGN_START_UTC.txt \
      CAMPAIGN_END_UTC.txt IDENTITY_LOCAL.json SOURCE_LS_TREE.txt 2>/dev/null
    mv OUT.tar.gz.tmp OUT.tar.gz && sha256sum OUT.tar.gz > OUT.tar.gz.sha256.tmp && mv OUT.tar.gz.sha256.tmp OUT.tar.gz.sha256 )
  phase "final: done ($(python3 -s -c "import json;print(json.load(open('$CAMP/results/VERDICT.json'))['verdict'])" 2>/dev/null)); elapsed $(( $(date +%s) - T0 ))s"
  touch "$CAMP/CAMPAIGN_DONE"
}
trap finalize EXIT
trap 'phase "signal: aborting"; exit 130' TERM INT

# watchdog: TERM this script at the measurement end (112 min); finalize then has <= 8 min
MAIN=$$
( sleep "$MEAS_END_S"
  if grep -q run_campaign.sh "/proc/$MAIN/cmdline" 2>/dev/null; then
    echo "[$(date -u +%H:%M:%S)] watchdog: measurement end" >> "$LOG/PHASES.log"; kill -TERM "$MAIN"
  fi ) &
WATCHDOG_PID=$!

python3 -s "$H/telemetry.py" "$CAMP" > /dev/null 2>&1 &
TELEM_PID=$!

run_phase() {  # run_phase NAME MINUTE_DEADLINE CMD...: background + wait, so signals act at once
  local name=$1 minute=$2; shift 2
  phase "$name"
  NCPC1_DEADLINE_MONO=$(mono_at "$minute") "$@" > "$LOG/$name.out" 2>&1 &
  PHASE_PID=$!
  wait $PHASE_PID; local rc=$?
  PHASE_PID=""
  phase "$name exit=$rc"
  return $rc
}

run_phase setup 20 timeout 1200 bash "$SRC/external_audits/vigilode_bassrei_ncp_c1_20261022/setup_ncp.sh" "$CAMP"
SETUP=$?
[ -f "$CAMP/julia.env" ] && source "$CAMP/julia.env"
if [ $SETUP -ne 0 ]; then phase "setup failed: no measurement phases (verdict INVALID)"; exit 1; fi

NCPC1_PARITY_CORE=${NCPC1_PARITY_CORESET:-8} run_phase parity 25 python3 -s "$H/parity.py" "$CAMP" ${NCPC1_CASES:+"$NCPC1_CASES"}

phase "vg (background, cores ${NCPC1_VG_CORESET:-60-63})"
NCPC1_VG_CORES=${NCPC1_VG_CORESET:-60-63} NCPC1_DEADLINE_MONO=$(mono_at 100) python3 -s "$H/vg.py" "$CAMP" \
  > "$LOG/vg.out" 2>&1 &
VG_PID=$!

NCPC1_CORES=${NCPC1_LADDER_CORES:-8-59} NCPC1_JULIA_SERVERS=16 run_phase ladder 70 \
  python3 -s "$H/ladder.py" "$CAMP" ${NCPC1_CASES:+--cases "$NCPC1_CASES"} ${NCPC1_RUNGS:+--rungs "$NCPC1_RUNGS"}

NCPC1_ABBA_CORES=${NCPC1_LADDER_CORES:-8-59} run_phase abba 105 \
  python3 -s "$H/abba.py" "$CAMP" --warmup 2 --confirm 7

wait $VG_PID 2>/dev/null; phase "vg exit=$?"
VG_PID=
exit 0
