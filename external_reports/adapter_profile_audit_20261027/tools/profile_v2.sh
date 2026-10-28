#!/usr/bin/env bash
# Profile v2 (harness v2: solver call only). perf self is primary; perf
# inclusive uses frame-pointer unwinding and loses frames through code built
# without frame pointers (libm/std), so callgrind inclusive is the authority
# for any share-of-cost statement. BIN: measurement build with
# CARGO_PROFILE_MEASUREMENT_DEBUG=line-tables-only and
# RUSTFLAGS="-C force-frame-pointers=yes" (diagnostic build only).
set -uo pipefail
BIN=$1; OUT=$2
mkdir -p "$OUT"/{perf,callgrind,heaptrack}
export RAYON_NUM_THREADS=1
CELLS="rotation:adaptive:8000:3 rotation:evidence:8000:3 rotation:dopri:10000:3
decay:adaptive:250000:30 decay:dopri:400000:30
case5:adaptive:8000:3 case5:evidence:6000:3 case5:rodas:6000:3 case5:rodas-dense:50000:10 case5:rodas-fd:50000:10
stiff-forced:adaptive:60000:10 stiff-forced:evidence:60000:10 stiff-forced:rodas:150000:10
relaxation:adaptive:25000:5 relaxation:rodas:40000:5
transition-39:adaptive:300:1 transition-39:rodas:200:1 transition-79:adaptive:500:1 transition-79:rodas:200:1
gyro-tight:adaptive:700:1 gyro-tight:dopri:900:1 gyro-tight:rodas:12:1 gyro-tight:rodas-fd:400:1
gyro-default:adaptive:2500:3 gyro-default:dopri:3000:3
stale-return:adaptive:8000:3 stale-return:adaptive-sentinel:10000:3 stale-return:dopri:3000:3 stale-return:rodas:10000:3"
for cell in $CELLS; do
  IFS=: read -r case arm pn cn <<< "$cell"
  stem="${case}__${arm}"
  taskset -c 11 perf record -q -F 15000 --call-graph fp -o "$OUT/perf/$stem.data" -- \
    "$BIN" repeat-arm "$case" "$arm" "$pn" 2> "$OUT/perf/$stem.run.log" > /dev/null
  perf report -i "$OUT/perf/$stem.data" --stdio --no-children --sort symbol -g none --percent-limit 0.4 \
    2>/dev/null | grep -v '^#' | grep -v '^$' > "$OUT/perf/$stem.self.txt"
  perf report -i "$OUT/perf/$stem.data" --stdio --children --sort symbol -g none --percent-limit 1.5 \
    2>/dev/null | grep -v '^#' | grep -v '^$' > "$OUT/perf/$stem.inclusive.txt"
  taskset -c 11 valgrind --tool=callgrind --callgrind-out-file="$OUT/callgrind/$stem.out" \
    "$BIN" repeat-arm "$case" "$arm" "$cn" > /dev/null 2> "$OUT/callgrind/$stem.run.log"
  callgrind_annotate --inclusive=yes "$OUT/callgrind/$stem.out" > "$OUT/callgrind/$stem.inclusive.full.txt" 2>/dev/null
  head -80 "$OUT/callgrind/$stem.inclusive.full.txt" > "$OUT/callgrind/$stem.inclusive.txt"
  callgrind_annotate "$OUT/callgrind/$stem.out" > "$OUT/callgrind/$stem.self.full.txt" 2>/dev/null
  head -60 "$OUT/callgrind/$stem.self.full.txt" > "$OUT/callgrind/$stem.self.txt"
  echo "$stem done" >&2
done
# Heaptrack: steady-state allocation sites. Two repeat counts per cell allow
# differencing out one-time costs (coefficient parse, process start).
for cell in rotation:adaptive decay:adaptive stiff-forced:adaptive case5:adaptive transition-39:adaptive \
            stiff-forced:rodas case5:rodas case5:rodas-fd case5:rodas-dense; do
  IFS=: read -r case arm <<< "$cell"
  for n in 1 21; do
    stem="${case}__${arm}__n${n}"
    heaptrack -o "$OUT/heaptrack/$stem" "$BIN" repeat-arm "$case" "$arm" "$n" > /dev/null 2>&1
    heaptrack_print -f "$OUT/heaptrack/$stem.zst" --print-allocators --print-peaks 0 --print-leaks 0 \
      --print-temporary 0 -n 30 > "$OUT/heaptrack/$stem.sites.txt" 2>/dev/null
    heaptrack_print -f "$OUT/heaptrack/$stem.zst" 2>/dev/null | grep -E '^(calls to allocation functions|total runtime)' \
      > "$OUT/heaptrack/$stem.totals.txt"
  done
done
