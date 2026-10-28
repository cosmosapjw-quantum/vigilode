#!/usr/bin/env bash
# Profile representative cells: perf (frame-pointer call graphs, self and
# inclusive by symbol), callgrind (instruction counts) and heaptrack
# (allocation sites). BIN is a measurement build with
# CARGO_PROFILE_MEASUREMENT_DEBUG=line-tables-only and
# RUSTFLAGS="-C force-frame-pointers=yes" (diagnostic build only).
set -euo pipefail
BIN=$1; OUT=$2
mkdir -p "$OUT"/{perf,callgrind,heaptrack}
export RAYON_NUM_THREADS=1
# case arm perf_n callgrind_n
CELLS="rotation:adaptive:8000:3 rotation:dopri:10000:3 decay:adaptive:200000:30 decay:dopri:400000:30
case5:adaptive:8000:3 case5:rodas:6000:3 case5:rodas-dense:50000:10 stiff-forced:adaptive:50000:10
stiff-forced:rodas:150000:10 relaxation:adaptive:25000:5 transition-39:adaptive:300:1 transition-39:rodas:200:1
gyro-tight:adaptive:600:1 gyro-tight:dopri:900:1 gyro-tight:rodas:12:1 stale-return:adaptive-sentinel:10000:3
stale-return:adaptive:8000:3"
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
  callgrind_annotate --inclusive=yes "$OUT/callgrind/$stem.out" > "$OUT/callgrind/$stem.inclusive.full.txt" 2>/dev/null || true
  head -80 "$OUT/callgrind/$stem.inclusive.full.txt" > "$OUT/callgrind/$stem.inclusive.txt"
  callgrind_annotate "$OUT/callgrind/$stem.out" > "$OUT/callgrind/$stem.self.full.txt" 2>/dev/null || true
  head -60 "$OUT/callgrind/$stem.self.full.txt" > "$OUT/callgrind/$stem.self.txt"
  echo "$stem done" >&2
done
for cell in rotation:adaptive:200 case5:adaptive:200 stiff-forced:adaptive:500 decay:adaptive:2000 transition-39:adaptive:5; do
  IFS=: read -r case arm n <<< "$cell"
  stem="${case}__${arm}"
  heaptrack -o "$OUT/heaptrack/$stem" "$BIN" repeat-arm "$case" "$arm" "$n" > /dev/null 2>&1 || true
  heaptrack_print -f "$OUT/heaptrack/$stem.zst" --print-allocators --print-peaks 0 --print-leaks 0 \
    --print-temporary 0 -n 25 2>/dev/null > "$OUT/heaptrack/$stem.sites.txt" || true
done
