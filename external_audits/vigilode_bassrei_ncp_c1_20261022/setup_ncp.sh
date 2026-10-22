#!/usr/bin/env bash
# Setup phase of the BASS/rei NCP C1 campaign (runs on NCP; <= 15 min).
#
#   bash setup_ncp.sh CAMP
#
# 1. Verify the unpacked source subset against the commit's ls-tree (IDENTITY.json).
# 2. Build the three VigilODE runners (measurement profile, --locked --offline,
#    Rust 1.94.1) into CAMP/target; copy them to CAMP/bin.
# 3. In parallel: the diffsol adapter (standalone crate in CAMP/diffsol-adapter,
#    lock = checkout lock + archived diffsol lock; 8-minute cap, else HOLD) and the
#    Julia environment (the existing depot is COPIED into CAMP/julia-depot, never
#    stacked; 12-minute cap on loading OrdinaryDiffEq/Sundials, else HOLD).
# Campaign outputs go to CAMP only (cargo additionally updates its own home cache, ~/.cargo).
# All work inherits run_campaign.sh's affinity (cores 8-63); -j values fit 56 cores.
set -uo pipefail
trap 'kill $(jobs -p) 2>/dev/null; pkill -TERM -P $$ 2>/dev/null; exit 143' TERM INT
CAMP=$(readlink -f "$1")
SRC=$CAMP/src
H=$SRC/external_audits/vigilode_bassrei_ncp_c1_20261022/harness
LOG=$CAMP/logs
mkdir -p "$LOG" "$CAMP/bin" "$CAMP/results"
export RUSTUP_TOOLCHAIN=1.94.1
phase() { echo "[$(date -u +%H:%M:%S)] $*" | tee -a "$LOG/PHASES.log"; }

phase "setup: verify source"
python3 -s "$H/verify_source.py" "$CAMP" > "$LOG/verify_source.out" 2>&1
VERIFY=$?
if [ $VERIFY -ne 0 ]; then
  phase "setup: SOURCE MISMATCH (see results/IDENTITY.json); the verdict will be INVALID (G7)"
fi

# --- diffsol (background) -------------------------------------------------
(
  set -uo pipefail
  rm -rf "$CAMP/diffsol-adapter"
  cp -a "$H/diffsol-adapter" "$CAMP/diffsol-adapter"
  ARCH_LOCK=$SRC/external_audits/vigilode_ncp_intensive_120m_v1_20261021/harness/diffsol-adapter/Cargo.lock
  cd "$CAMP/diffsol-adapter"
  END=$(( $(date +%s) + 480 ))
  OK=1
  # lock seeds, in order: merged (checkout + archived diffsol), archived only; no unpinned resolution
  for seed in merged archived; do
    case $seed in
      merged) python3 -s "$H/merge_locks.py" "$SRC/Cargo.lock" "$ARCH_LOCK" > Cargo.lock ;;
      archived) cp "$ARCH_LOCK" Cargo.lock ;;
    esac
    cp Cargo.lock "$CAMP/results/DIFFSOL_LOCK_SEED_$seed.lock"
    LEFT=$(( END - $(date +%s) ))
    [ $LEFT -le 10 ] && break
    echo "== seed $seed (${LEFT}s left)" >> "$LOG/diffsol_build.log"
    if NCPC1_DIFFSOL_VERSION=0.17.1 CARGO_TARGET_DIR=$CAMP/target-diffsol timeout $LEFT \
         cargo build --release --offline --locked -j 16 >> "$LOG/diffsol_build.log" 2>&1 \
       || NCPC1_DIFFSOL_VERSION=0.17.1 CARGO_TARGET_DIR=$CAMP/target-diffsol timeout $LEFT \
         cargo build --release --offline -j 16 >> "$LOG/diffsol_build.log" 2>&1; then
      OK=0; echo "$seed" > "$CAMP/results/DIFFSOL_LOCK_SEED.txt"; break
    fi
  done
  if [ $OK -eq 0 ]; then
    cp "$CAMP/target-diffsol/release/vigilode_bassrei_diffsol" "$CAMP/bin/"
    cp Cargo.lock "$CAMP/results/DIFFSOL_Cargo.lock"     # the full resolved lock, for provenance
    sha256sum Cargo.lock > "$CAMP/results/DIFFSOL_Cargo.lock.sha256"
    echo '{"status": "OK"}' > "$CAMP/results/DIFFSOL_SETUP.json"
  else
    rc=1
    python3 -s - "$rc" "$LOG/diffsol_build.log" > "$CAMP/results/DIFFSOL_SETUP.json" <<'PY'
import json, sys
tail = open(sys.argv[2], errors="replace").read()[-3000:]
print(json.dumps({"status": "HOLD", "exit_code": int(sys.argv[1]), "reason": "offline build failed or exceeded 8 min",
                  "log_tail": tail}))
PY
  fi
) &
DIFFSOL_PID=$!

# --- Julia (background) ---------------------------------------------------
(
  set -uo pipefail
  JUL=${NCPC1_JULIA_BIN:-/root/VIGILODE_COMPONENT_20261018T134235Z/env/julia-1.12.7+0.x64.linux.gnu/bin/julia}
  SRC_DEPOT=""
  for d in ${NCPC1_JULIA_DEPOT_SRC:-} \
           /root/vigilode-julia-c1-20261022/depot \
           /root/VIGILODE_COMPONENT_20261018T134235Z/env/julia-depot \
           /root/VIGILODE_COMPONENT_20261018T134235Z/env/depot \
           /root/VIGILODE_COMPONENT_20261018T134235Z/julia-depot \
           /dev/shm/vigilode_ncp_intensive_120m_v1_20261021T062109Z/julia-depot \
           /root/.julia; do
    if [ -d "$d/packages/OrdinaryDiffEq" ] && [ -d "$d/packages/Sundials" ]; then SRC_DEPOT=$d; break; fi
  done
  if [ ! -x "$JUL" ] || [ -z "$SRC_DEPOT" ]; then
    echo "{\"status\": \"HOLD\", \"reason\": \"julia binary or a depot with OrdinaryDiffEq+Sundials not found\", \"julia\": \"$JUL\"}" \
      > "$CAMP/results/JULIA_SETUP.json"
    exit 0
  fi
  DEPOT_BYTES=$(timeout 300 du -sb "$SRC_DEPOT" 2>/dev/null | cut -f1)
  FREE_BYTES=$(df -B1 --output=avail "$CAMP" | tail -1 | tr -dc 0-9)
  if [ -z "$DEPOT_BYTES" ] || [ $(( FREE_BYTES - DEPOT_BYTES )) -lt $(( 20 * 1024 * 1024 * 1024 )) ]; then
    echo "{\"status\": \"HOLD\", \"reason\": \"depot copy would leave < 20 GB free\", \"depot_bytes\": \"${DEPOT_BYTES:-unknown}\", \"free_bytes\": $FREE_BYTES}" \
      > "$CAMP/results/JULIA_SETUP.json"
    exit 0
  fi
  mkdir -p "$CAMP/julia-env"
  cp "$SRC/external_audits/vigilode_ncp_intensive_120m_v1_20261021/harness/julia-env/Project.toml" \
     "$SRC/external_audits/vigilode_ncp_intensive_120m_v1_20261021/harness/julia-env/Manifest.toml" "$CAMP/julia-env/"
  T0=$(date +%s)
  if ! timeout 600 cp -a "$SRC_DEPOT" "$CAMP/julia-depot"; then   # a private copy: the source depot is only read
    echo "{\"status\": \"HOLD\", \"reason\": \"depot copy failed or exceeded 10 min\", \"source_depot\": \"$SRC_DEPOT\"}" \
      > "$CAMP/results/JULIA_SETUP.json"
    exit 0
  fi
  T1=$(date +%s)
  if JULIA_DEPOT_PATH=$CAMP/julia-depot JULIA_PKG_OFFLINE=true JULIA_NUM_PRECOMPILE_TASKS=12 timeout 720 \
       "$JUL" --startup-file=no --project="$CAMP/julia-env" \
       -e 'using OrdinaryDiffEq, OrdinaryDiffEqBDF, SciMLBase, Sundials; println("JULIA_OK ", VERSION)' \
       > "$LOG/julia_setup.log" 2>&1; then
    T2=$(date +%s)
    cat > "$CAMP/julia.env" <<EOT
export NCPC1_JULIA=$JUL
export NCPC1_JULIA_PROJECT=$CAMP/julia-env
export NCPC1_JULIA_DEPOT=$CAMP/julia-depot
EOT
    echo "{\"status\": \"OK\", \"source_depot\": \"$SRC_DEPOT\", \"depot_bytes\": $DEPOT_BYTES, \"copy_seconds\": $((T1-T0)), \"load_seconds\": $((T2-T1)), \"timing_authority\": \"diagnostic_only\"}" \
      > "$CAMP/results/JULIA_SETUP.json"
  else
    echo "{\"status\": \"HOLD\", \"reason\": \"package load/precompile failed or exceeded 12 min\", \"source_depot\": \"$SRC_DEPOT\"}" \
      > "$CAMP/results/JULIA_SETUP.json"
  fi
) &
JULIA_PID=$!

# --- runners (foreground) ---------------------------------------------------
phase "setup: build runners"
( cd "$SRC" && CARGO_TARGET_DIR=$CAMP/target cargo build --profile measurement -p rodas5p-integrators \
    --example lrs_photon_fv_run --example lrs_photon_block_run --example lrs_photon_chem_run \
    --locked --offline -j 24 ) > "$LOG/runner_build.log" 2>&1
BUILD=$?
for b in lrs_photon_fv_run lrs_photon_block_run lrs_photon_chem_run; do
  [ -x "$CAMP/target/measurement/examples/$b" ] && cp "$CAMP/target/measurement/examples/$b" "$CAMP/bin/"
done
phase "setup: runner build exit=$BUILD"
wait $DIFFSOL_PID; phase "setup: diffsol $(python3 -s -c "import json;print(json.load(open('$CAMP/results/DIFFSOL_SETUP.json'))['status'])" 2>/dev/null)"
wait $JULIA_PID;   phase "setup: julia $(python3 -s -c "import json;print(json.load(open('$CAMP/results/JULIA_SETUP.json'))['status'])" 2>/dev/null)"
python3 -s "$H/verify_source.py" "$CAMP" > "$LOG/verify_source_with_bins.out" 2>&1   # records binary hashes
echo "{\"source_verify_exit\": $VERIFY, \"runner_build_exit\": $BUILD}" > "$CAMP/results/SETUP.json"
[ $BUILD -eq 0 ]
