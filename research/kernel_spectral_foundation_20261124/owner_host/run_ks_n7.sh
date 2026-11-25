#!/bin/sh
# KS-N7 owner-host timing cell (owner_host/KS_N7_PROTOCOL.json).
# Usage: run_ks_n7.sh <spatial_separable test binary> <out dir>
# Writes env.json, then per process <tag>.meta.json, <tag>.perf.csv, <tag>.out.txt and <tag>.exit,
# then end.json. No process waits for the load; the load is recorded before every process.
set -eu
BIN=$1
OUT=$2
CORE=8
mkdir -p "$OUT"
cat > "$OUT/env.json" <<EOF
{"start": "$(date -Iseconds)", "head": "$(git rev-parse HEAD)", "kernel": "$(uname -r)",
 "rustc": "$(rustc +1.94.1 -V)", "perf": "$(perf --version)",
 "governor": "$(cat /sys/devices/system/cpu/cpu$CORE/cpufreq/scaling_governor 2>/dev/null || echo unknown)",
 "core": $CORE, "binary_sha256": "$(sha256sum "$BIN" | cut -d' ' -f1)", "loadavg": "$(cat /proc/loadavg)"}
EOF
measure() {  # tag arm grid solves
    printf '{"tag": "%s", "load1_before": %s, "start": "%s"}\n' \
        "$1" "$(cut -d' ' -f1 /proc/loadavg)" "$(date -Iseconds)" > "$OUT/$1.meta.json"
    status=0
    KS_N7_ARM=$2 KS_N7_GRID=$3 KS_N7_SCOPE=prepared KS_N7_SOLVES=$4 \
        perf stat -x, -e instructions:u,cycles:u,task-clock -o "$OUT/$1.perf.csv" -- \
        taskset -c "$CORE" "$BIN" measure_box_transform_one_arm --exact --ignored --nocapture \
        > "$OUT/$1.out.txt" 2>&1 || status=$?
    echo "$status" > "$OUT/$1.exit"
}
for round in 1 2 3; do
    if [ "$round" -eq 2 ]; then arms="paired complex"; else arms="complex paired"; fi
    for cell in 256x256:1000 128x128x64:50; do
        grid=${cell%%:*}
        n=${cell#*:}
        for arm in $arms; do
            for solves in 1 $((1 + n)); do
                measure "r${round}_${grid}_${arm}_s${solves}" "$arm" "$grid" "$solves"
            done
        done
    done
done
echo "{\"end\": \"$(date -Iseconds)\", \"loadavg\": \"$(cat /proc/loadavg)\"}" > "$OUT/end.json"
