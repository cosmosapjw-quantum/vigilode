#!/usr/bin/env bash
# Run this extracted file, not an absent command in the preserved branch.
set -euo pipefail
REPO_ROOT= PACKAGE_SHA=
while (($#)); do
  case "$1" in
    --repo-root|--package-sha)
      (($# >= 2)) || { echo 'missing argument value' >&2; exit 2; }
      if [[ $1 == --repo-root ]]; then REPO_ROOT=$2; else PACKAGE_SHA=$2; fi
      shift 2 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done
[[ -n $REPO_ROOT && $PACKAGE_SHA =~ ^[0-9a-f]{40}$ ]] || {
  echo 'usage: bootstrap --repo-root PATH --package-sha 40HEX' >&2; exit 2;
}
REPO_ROOT=$(cd "$REPO_ROOT" && pwd -P)
PYTHON=$(command -v python3 || command -v python) || exit 2
TMP=$(mktemp -d "${TMPDIR:-/tmp}/k0-bootstrap-entry.XXXXXX")
trap 'rm -rf -- "$TMP"' EXIT
export PYTHONDONTWRITEBYTECODE=1 GIT_TERMINAL_PROMPT=0
git -C "$REPO_ROOT" cat-file -e "$PACKAGE_SHA^{commit}"
git -C "$REPO_ROOT" show \
  "$PACKAGE_SHA:tools/verify-k0-wu05-bootstrap-v2.py" > "$TMP/validator.py"
"$PYTHON" -B "$TMP/validator.py" --repo-root "$REPO_ROOT" \
  --package-sha "$PACKAGE_SHA" --apply
