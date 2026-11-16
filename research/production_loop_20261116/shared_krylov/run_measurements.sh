#!/bin/bash
# Loop node V06: executes the runs of PREDECLARED.json, one process per run,
# each under `perf stat -e instructions:u`, and keeps every raw output under
# logs/. Nothing is judged here; parse_results.py reads the logs.
#
# usage: run_measurements.sh <group> [family ...]
#   group  stage_a      the declared runs of the stage-a families
#          exploratory  C60, Ckeep and the stiff Brusselator (not counted)
#   family restricts the group to the named families (default: all of it)
#
# One log per run: logs/<name>.log (V06_META lines, the program's
# V06_RUN_JSON and V06_STATE_JSON lines, anything it wrote to stderr) and
# logs/<name>.perf (perf stat CSV). A run that exists is not repeated;
# delete nothing: rerun under a new name instead.
set -u

NODE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$NODE/../../.." && pwd)"
LOGS="$NODE/logs"
export CARGO_TARGET_DIR=/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w2-v06
CPUS="0-9,12-21"
TIME_LIMIT_SECONDS=10800
mkdir -p "$LOGS"

group="${1:?group: stage_a or exploratory}"
shift
families=("$@")

cd "$ROOT" || exit 2
build_log="$LOGS/build_${group}.log"
taskset -c "$CPUS" cargo +1.94.1 test --locked -j 4 --release -p rodas5p-integrators \
    --test stage_clients_krylov_measure --no-run --message-format=json 2>"$build_log.stderr" |
    python3 -I -c '
import json, sys
for line in sys.stdin:
    try:
        message = json.loads(line)
    except ValueError:
        continue
    if message.get("reason") == "compiler-artifact" and message.get("executable") \
            and message["target"]["name"] == "stage_clients_krylov_measure":
        print(message["executable"])
' >"$build_log"
BIN="$(tail -n 1 "$build_log")"
if [ ! -x "$BIN" ]; then
    echo "no measurement binary; see $build_log.stderr" >&2
    exit 2
fi

{
    echo "V06_ENV group=$group date=$(date -Is)"
    echo "V06_ENV git_head=$(git -C "$ROOT" rev-parse HEAD)"
    echo "V06_ENV git_dirty_tracked=$(git -C "$ROOT" status --porcelain --untracked-files=no | wc -l)"
    echo "V06_ENV predeclared_sha256=$(sha256sum "$NODE/PREDECLARED.json" | cut -d' ' -f1)"
    echo "V06_ENV binary=$BIN"
    echo "V06_ENV binary_sha256=$(sha256sum "$BIN" | cut -d' ' -f1)"
    echo "V06_ENV rustc=$(rustc +1.94.1 --version)"
    echo "V06_ENV perf=$(perf --version)"
    echo "V06_ENV kernel=$(uname -r)"
    echo "V06_ENV cpu=$(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | sed 's/^ //')"
    echo "V06_ENV taskset=$CPUS"
    echo "V06_ENV loadavg=$(cut -d' ' -f1-3 /proc/loadavg)"
} >"$LOGS/environment_${group}_$(date +%Y%m%dT%H%M%S).log"

# run_one <name> <family> <rtol> <arm> <precond> <audit> [max_dim] [keep]
run_one() {
    local name="$1" family="$2" rtol="$3" arm="$4" precond="$5" audit="$6"
    local max_dim="${7:-200}" keep="${8:-0}"
    local log="$LOGS/$name.log"
    if [ -e "$log" ]; then
        echo "skip $name (log exists)"
        return
    fi
    echo "run  $name  $(date +%H:%M:%S)"
    {
        echo "V06_META name=$name family=$family rtol=$rtol arm=$arm precond=$precond audit=$audit max_dim=$max_dim keep=$keep"
        echo "V06_META start=$(date -Is) loadavg_before=$(cut -d' ' -f1-3 /proc/loadavg)"
        V06_LABEL="$name" V06_FAMILY="$family" V06_RTOL="$rtol" V06_ARM="$arm" \
            V06_PRECOND="$precond" V06_AUDIT="$audit" V06_MAX_DIM="$max_dim" V06_KEEP="$keep" \
            timeout "$TIME_LIMIT_SECONDS" taskset -c "$CPUS" \
            perf stat -x, -e instructions:u -o "$LOGS/$name.perf" -- \
            "$BIN" --ignored --nocapture --exact v06_run
        local status=$?
        echo "V06_META exit_status=$status"
        echo "V06_META end=$(date -Is) loadavg_after=$(cut -d' ' -f1-3 /proc/loadavg)"
    } >"$log" 2>&1
}

selected() {
    [ "${#families[@]}" -eq 0 ] && return 0
    local family
    for family in "${families[@]}"; do
        [ "$family" = "$1" ] && return 0
    done
    return 1
}

case "$group" in
stage_a)
    for family in adr2d grayscott2d bruss1d; do
        selected "$family" || continue
        for rtol in 1e-4 1e-6; do
            p="a_${family}_${rtol}"
            run_one "${p}_SETUP" "$family" "$rtol" SETUP none 0
            run_one "${p}_A_none" "$family" "$rtol" A none 0
            run_one "${p}_B0_none" "$family" "$rtol" B0 none 0
            run_one "${p}_B_none_audit1" "$family" "$rtol" B none 1
            run_one "${p}_B_none_audit0" "$family" "$rtol" B none 0
            run_one "${p}_C_none_audit1" "$family" "$rtol" C none 1
            run_one "${p}_C_none_audit0" "$family" "$rtol" C none 0
            run_one "${p}_B_jacobi_audit1" "$family" "$rtol" B jacobi 1
            run_one "${p}_C_jacobi_audit1" "$family" "$rtol" C jacobi 1
        done
    done
    ;;
exploratory)
    for family in adr2d grayscott2d bruss1d; do
        selected "$family" || continue
        for rtol in 1e-4 1e-6; do
            p="x_${family}_${rtol}"
            run_one "${p}_C60_none_audit1" "$family" "$rtol" C none 1 60 0
            run_one "${p}_Ckeep_none_audit1" "$family" "$rtol" C none 1 200 1
        done
    done
    if selected bruss1d_stiff; then
        p="x_bruss1d_stiff_1e-4"
        run_one "${p}_SETUP" bruss1d_stiff 1e-4 SETUP none 0
        run_one "${p}_A_none" bruss1d_stiff 1e-4 A none 0
        run_one "${p}_B_none_audit1" bruss1d_stiff 1e-4 B none 1
        run_one "${p}_C60_none_audit1" bruss1d_stiff 1e-4 C none 1 60 0
    fi
    ;;
*)
    echo "unknown group $group" >&2
    exit 2
    ;;
esac
echo "done $group $(date -Is)"
