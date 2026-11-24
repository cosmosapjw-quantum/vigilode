#!/bin/sh
# KS-N1 owner-host series (owner_host/KS_N1_PROTOCOL.json).
# Usage: run_ks_n1.sh <probe binary> <banded_kernel_entry_identity test binary> <out dir>
# Writes env.json, then per process <tag>.meta.json, <tag>.perf.csv and <tag>.out.json, then the
# three diagnostic runs diag_<k>.perf.csv and diag_<k>.out.txt. Changes nothing else.
set -eu
PROBE=$1
TESTBIN=$2
OUT=$3
CORE=3
mkdir -p "$OUT"

load1() { cut -d' ' -f1 /proc/loadavg; }

# Wait until the 1-minute load average is <= 3.0 (every 5 s, at most 600 s); print it.
quiet() {
    waited=0
    while :; do
        l=$(load1)
        if awk -v l="$l" 'BEGIN { exit !(l <= 3.0) }'; then
            echo "$l $waited"
            return
        fi
        if [ "$waited" -ge 600 ]; then
            echo "$l $waited"
            return
        fi
        sleep 5
        waited=$((waited + 5))
    done
}

cat > "$OUT/env.json" <<EOF
{"start": "$(date -Iseconds)", "head": "$(git rev-parse HEAD)", "kernel": "$(uname -r)",
 "rustc": "$(rustc +1.94.1 -V)", "perf": "$(perf --version)",
 "governor": "$(cat /sys/devices/system/cpu/cpu$CORE/cpufreq/scaling_governor 2>/dev/null || echo unknown)",
 "core": $CORE, "probe_sha256": "$(sha256sum "$PROBE" | cut -d' ' -f1)",
 "test_binary_sha256": "$(sha256sum "$TESTBIN" | cut -d' ' -f1)", "loadavg": "$(cat /proc/loadavg)"}
EOF

measure() {  # tag arm rows solves
    q=$(quiet)
    printf '{"tag": "%s", "load1_before": %s, "waited_s": %s, "start": "%s"}\n' \
        "$1" "${q% *}" "${q#* }" "$(date -Iseconds)" > "$OUT/$1.meta.json"
    perf stat -x, -e instructions:u,cycles:u,task-clock -o "$OUT/$1.perf.csv" -- \
        taskset -c "$CORE" "$PROBE" "$2" "$3" "$4" > "$OUT/$1.out.json"
}

for round in 1 2 3; do
    if [ "$round" -eq 2 ]; then arms="fixed2 indexed"; else arms="indexed fixed2"; fi
    for cell in n1e4:5000:100 n1e5:50000:12; do
        id=${cell%%:*}
        rest=${cell#*:}
        rows=${rest%%:*}
        n=${rest#*:}
        for arm in $arms; do
            for solves in 1 $((1 + n)); do
                measure "r${round}_${id}_${arm}_s${solves}" "$arm" "$rows" "$solves"
            done
        done
    done
done

for k in 1 2 3; do
    q=$(quiet)
    printf '{"tag": "diag_%s", "load1_before": %s, "waited_s": %s, "start": "%s"}\n' \
        "$k" "${q% *}" "${q#* }" "$(date -Iseconds)" > "$OUT/diag_$k.meta.json"
    status=0
    perf stat -x, -e instructions:u,cycles:u,task-clock -o "$OUT/diag_$k.perf.csv" -- \
        taskset -c "$CORE" "$TESTBIN" measure_fixed_width_against_indexed_whole_solves \
        --exact --ignored --nocapture > "$OUT/diag_$k.out.txt" 2>&1 || status=$?
    echo "exit $status" >> "$OUT/diag_$k.out.txt"
done
echo "{\"end\": \"$(date -Iseconds)\", \"loadavg\": \"$(cat /proc/loadavg)\"}" > "$OUT/end.json"
