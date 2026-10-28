#!/usr/bin/env bash
# Copy the campaign results back and verify them (safe to repeat).
#
#   bash fetch_results.sh NCP_CAMP_DIR LOCAL_DIR
#
# Complete campaign (CAMPAIGN_DONE present): fetch OUT.tar.gz (written as a temp file and
# renamed by finalize, so it is complete) and verify every sha256.
# Campaign process alive (campaign.pid + kill -0 + /proc cmdline): report and exit 4.
# Dead without CAMPAIGN_DONE (crash, reboot): pack the partial results remotely into
# OUT_PARTIAL.tar.gz (tmp + rename; OUT.tar.gz is never overwritten), after verify_source --end
# and analysis (verdict INVALID if incomplete), and fetch that.
# ssh transport failures (exit 255) are reported as such (exit 2), never read as "not running".
set -uo pipefail
CAMP=${1:?ncp dir}; LOCAL=${2:?local dir}
KEY=/home/cosmosapjw/.ssh/ncp_key; HOST=root@49.50.142.2
OPTS=(-i "$KEY" -o IdentitiesOnly=yes -o BatchMode=yes -o ConnectTimeout=20)
SSH=(ssh "${OPTS[@]}" -p 22 "$HOST")
SCP=(scp "${OPTS[@]}" -P 22)
die() { echo "[fetch] $2" >&2; exit "$1"; }
mkdir -p "$LOCAL/ncp"
STATE=$("${SSH[@]}" "if [ -f $CAMP/CAMPAIGN_DONE ]; then echo DONE;
  elif [ -f $CAMP/campaign.pid ] && kill -0 \$(cat $CAMP/campaign.pid) 2>/dev/null \
       && grep -q run_campaign.sh /proc/\$(cat $CAMP/campaign.pid)/cmdline 2>/dev/null; then echo RUNNING;
  elif [ -d $CAMP ]; then echo DEAD; else echo MISSING; fi" 2> "$LOCAL/fetch_ssh.err")
RC=$?
[ $RC -eq 255 ] && die 2 "ssh failure: $(tail -2 "$LOCAL/fetch_ssh.err" | tr '\n' ' ')"
case "$STATE" in
  RUNNING) die 4 "campaign still running (pid $("${SSH[@]}" "cat $CAMP/campaign.pid")); try later" ;;
  MISSING) die 3 "no campaign directory $CAMP on NCP" ;;
  DONE) TAR=OUT.tar.gz ;;
  DEAD)
    echo "[fetch] campaign process gone without CAMPAIGN_DONE: packing partial results remotely"
    "${SSH[@]}" "cd $CAMP && H=src/external_audits/vigilode_bassrei_ncp_c1_20261022/harness && \
      ( cd \$H && NCPC1_CHILD_REG=$CAMP/run/children python3 -s -c 'import common; common.kill_registered()' ); \
      python3 -s \$H/verify_source.py $CAMP --end > logs/verify_source_end_late.out 2>&1; \
      NCPC1_RESULTS=$CAMP/results timeout 600 python3 -s \$H/analyze.py $CAMP > logs/analyze_late.out 2>&1; \
      find results raw logs -type f | sort | xargs -d '\n' sha256sum > OUTPUT_SHA256SUMS.txt; \
      tar czf OUT_PARTIAL.tar.gz.tmp results raw logs REPORT.md OUTPUT_SHA256SUMS.txt IDENTITY_LOCAL.json SOURCE_LS_TREE.txt 2>/dev/null; \
      mv OUT_PARTIAL.tar.gz.tmp OUT_PARTIAL.tar.gz && sha256sum OUT_PARTIAL.tar.gz > OUT_PARTIAL.tar.gz.sha256" \
      || die 5 "remote partial packing failed"
    TAR=OUT_PARTIAL.tar.gz ;;
  *) die 5 "unexpected remote state: $STATE" ;;
esac
"${SCP[@]}" "$HOST:$CAMP/$TAR" "$HOST:$CAMP/$TAR.sha256" "$HOST:$CAMP/campaign.log" "$LOCAL/ncp/" || die 2 "scp failed"
( cd "$LOCAL/ncp" && sha256sum -c "$TAR.sha256" && tar xzf "$TAR" && sha256sum -c --quiet OUTPUT_SHA256SUMS.txt ) \
  || die 6 "verification of the copied results failed"
echo "[fetch] verified copy in $LOCAL/ncp ($TAR; verdict $(python3 -s -c "import json;print(json.load(open('$LOCAL/ncp/results/VERDICT.json'))['verdict'])" 2>/dev/null))"
echo "[fetch] next: python3 -s $(dirname "$0")/harness/import_results.py $LOCAL/ncp <repo>/research/bassrei_ncp_wp_c1_20261022"
