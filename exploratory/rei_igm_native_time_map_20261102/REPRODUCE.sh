#!/usr/bin/env bash
# Opt-in, NONAUTHORITATIVE original F05 46,080-cell Rust point-map reproducer.
# Usage: REPRODUCE.sh OUT_DIRECTORY [ORIGINAL_OWNER_INCREMENT_CSV_GZ]
# Does not mutate canonical REI state, mainline, original source, or producer.
set -euo pipefail
if [ "${1:-}" = "" ]; then
  echo "USAGE: REPRODUCE.sh OUT_DIRECTORY [OWNER_ORIGINAL_INCREMENT_CSV_GZ]" >&2; exit 2
fi
OUT="$(mkdir -p -- "$1" && realpath "$1")"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../" && pwd)"
RESEARCH="$ROOT/exploratory/rei_igm_native_time_map_20261102"
PARENT="$OUT/ORIGINAL_T160_PARENT5.raw"
ORACLE="$OUT/ORIGINAL_T160_NONPHOTO_E1.raw"
PHOTO="$OUT/ORIGINAL_T160_PHOTON_R1.raw"
CLOCK="$RESEARCH/ORIGINAL_F05_T160_SOURCE_CLOCK.json"
RESULT="$OUT/NATIVE_FULL26_TIME_MAP_RESULT.json"
RAW="$OUT/ORIGINAL_F05_T160_RUST_POINT_FULL_HALFHALF.raw"
gzip -dc "$ROOT/exploratory/rei_igm_native_photo_thermal_20261102/ORIGINAL_REI_F05_T160_PARENT5_THERMAL.bin.gz" > "$PARENT"
gzip -dc "$ROOT/exploratory/rei_igm_native_nonphoto_20261102/ORIGINAL_F05_T160_NONPHOTO_EVENT_ORACLE.bin.gz" > "$ORACLE"
gzip -dc "$ROOT/exploratory/rei_igm_native_rank3_20261101/ORIGINAL_REI_F05_T160_PHOTON_SOURCE.bin.gz" > "$PHOTO"
printf '%s\n' \
  "eeb4aa50c56ed5437c26b1c7087d0f68884db3db331d4ef435657b0bceb22064  $PARENT" \
  "28ea21ea9e5b4210b3e8971313af6aa6a8c4a07fad10c984bfd29424967ebf0b  $ORACLE" \
  "a4a37417fc392a7d2710c808dc19580b6373091550fd5f7ecdae4c0511784469  $PHOTO" \
  "0bd888ce706ee110cea4ba23a1e871817cf1f054b85ebe5837934fd19db0492e  $CLOCK" \
  | sha256sum --check --
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$OUT/target}"
cargo test --manifest-path "$ROOT/Cargo.toml" --offline --locked -p rodas5p-integrators \
  --example rei_original46k_native_time_map > "$OUT/RUST_UNIT_TEST.log" 2>&1
cargo run --manifest-path "$ROOT/Cargo.toml" --offline --locked --release -p rodas5p-integrators \
  --example rei_original46k_native_time_map -- "$PARENT" "$ORACLE" "$PHOTO" "$CLOCK" "$RAW" \
  > "$RESULT" 2> "$OUT/RUST_POINT_RUN_STDERR.log"
python3 "$RESEARCH/independent_raw_time_map_audit.py" --raw "$RAW" \
 --parent "$PARENT" --clock "$CLOCK" --receipt "$RESULT" \
 --out "$OUT/INDEPENDENT_RUST_46080_AUDIT.json"
if [ -n "${2:-}" ]; then
  python3 "$RESEARCH/compare_owner_original_increment.py" \
   --native "$RAW" --parent "$PARENT" --owner "$2" \
   --out "$OUT/RUST_POINT_VS_OWNER_INTERVAL_AUDIT.json"
fi
sha256sum "$RESULT" "$RAW" "$OUT/INDEPENDENT_RUST_46080_AUDIT.json"
echo "POINT-MAP COMPLETE: official_tick161=NOT_ACCEPTED production=HOLD"
