#!/bin/sh
# KS-N3 owner-host series (owner_host/KS_N3_PROTOCOL.json).
# Usage: run_ks_n3.sh <probe binary> <out dir>
# Writes env.json, then per process <tag>.meta.json, <tag>.perf.csv, <tag>.out.json and
# <tag>.exit, then end.json. Changes nothing else. No process waits for the load (protocol
# load_rule); the 1-minute load average is recorded before every process.
set -eu
PROBE=$1
OUT=$2
CORE=8
mkdir -p "$OUT"

cat > "$OUT/env.json" <<EOF
{"start": "$(date -Iseconds)", "head": "$(git rev-parse HEAD)", "kernel": "$(uname -r)",
 "rustc": "$(rustc +1.94.1 -V)", "perf": "$(perf --version)",
 "governor": "$(cat /sys/devices/system/cpu/cpu$CORE/cpufreq/scaling_governor 2>/dev/null || echo unknown)",
 "core": $CORE, "probe_sha256": "$(sha256sum "$PROBE" | cut -d' ' -f1)", "loadavg": "$(cat /proc/loadavg)"}
EOF

measure() {  # tag, then the probe arguments
    tag=$1
    shift
    printf '{"tag": "%s", "load1_before": %s, "start": "%s"}\n' \
        "$tag" "$(cut -d' ' -f1 /proc/loadavg)" "$(date -Iseconds)" > "$OUT/$tag.meta.json"
    status=0
    perf stat -x, -e instructions:u,cycles:u,task-clock -o "$OUT/$tag.perf.csv" -- \
        taskset -c "$CORE" "$PROBE" "$@" > "$OUT/$tag.out.json" 2> "$OUT/$tag.err" || status=$?
    echo "$status" > "$OUT/$tag.exit"
}

n_whole() {  # arm grid
    case "$1:$2" in
        shipped:32x64) echo 4 ;;
        shipped:*) echo 1 ;;
        *:32x64) echo 20 ;;
        *:64x128) echo 10 ;;
        *) echo 5 ;;
    esac
}

n_stage() {  # arm
    case "$1" in
        separable) echo 20 ;;
        *) echo 5 ;;
    esac
}

for round in 1 2 3; do
    if [ "$round" -eq 2 ]; then
        whole_arms="shipped separable-trusted separable-verified"
        stage_arms="gmres-identity separable gmres-longitude"
    else
        whole_arms="separable-verified separable-trusted shipped"
        stage_arms="gmres-longitude separable gmres-identity"
    fi
    for grid in 32x64 64x128 128x256; do
        nt=${grid%x*}
        np=${grid#*x}
        for arm in $whole_arms; do
            n=$(n_whole "$arm" "$grid")
            for solves in 1 $((1 + n)); do
                measure "r${round}_whole_${grid}_${arm}_s${solves}" whole "$arm" "$nt" "$np" "$solves"
            done
        done
    done
    for grid in 32x64 64x128 128x256; do
        nt=${grid%x*}
        np=${grid#*x}
        for h in 0.001 0.01 0.1; do
            for arm in $stage_arms; do
                n=$(n_stage "$arm")
                for solves in 1 $((1 + n)); do
                    measure "r${round}_stage_${grid}_h${h}_${arm}_s${solves}" stage "$arm" "$nt" "$np" "$h" "$solves"
                done
            done
        done
    done
done
echo "{\"end\": \"$(date -Iseconds)\", \"loadavg\": \"$(cat /proc/loadavg)\"}" > "$OUT/end.json"
