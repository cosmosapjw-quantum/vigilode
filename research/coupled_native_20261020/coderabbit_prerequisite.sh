#!/usr/bin/env bash
# Never publish authentication output or credentials. Only prerequisite exit codes.
set +e
OUT=$(realpath "$1")
TMP=$(mktemp -d)
export PATH="$HOME/.local/bin:$PATH"
coderabbit --version > "$TMP/version" 2>&1
VERSION=$?
INSTALL=0
if [[ "$VERSION" != 0 ]]; then
  curl -fsSL --connect-timeout 10 --max-time 30 https://cli.coderabbit.ai/install.sh -o "$TMP/install.sh"
  INSTALL=$?
  if [[ "$INSTALL" == 0 ]]; then timeout 90 bash "$TMP/install.sh" > "$TMP/install.log" 2>&1; INSTALL=$?; fi
  coderabbit --version > "$TMP/version" 2>&1; VERSION=$?
fi
AUTH=127; LOGIN=127; RECHECK=127
if [[ "$VERSION" == 0 ]]; then
  timeout 20 coderabbit auth status --agent > "$TMP/auth" 2>&1; AUTH=$?
  if [[ "$AUTH" != 0 ]]; then
    timeout 30 coderabbit auth login --agent > "$TMP/login" 2>&1; LOGIN=$?
    timeout 20 coderabbit auth status --agent > "$TMP/recheck" 2>&1; RECHECK=$?
  else RECHECK=0; fi
fi
printf '{"cli_version_exit":%s,"installer_exit":%s,"auth_status_exit":%s,"login_attempt_exit":%s,"auth_recheck_exit":%s,"review_executed":false,"review_issues":null,"github_review_requested_on_pr":74,"authentication_output_retained":false}\n' "$VERSION" "$INSTALL" "$AUTH" "$LOGIN" "$RECHECK" > "$OUT/CODERABBIT_PREREQUISITE.json"
if [[ "$VERSION" == 0 ]]; then head -c 300 "$TMP/version" > "$OUT/coderabbit_version.txt"; fi
rm -rf "$TMP"
exit 0
