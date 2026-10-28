#!/usr/bin/env bash
set -euo pipefail
if [[ $# -ne 4 ]]; then
  echo 'Usage: prepare_worktree.sh EXISTING_REPO EXPECTED_COMMIT EXPECTED_TREE NEW_CHECKOUT' >&2
  exit 2
fi
source_repo=$1
expected_commit=$2
expected_tree=$3
new_checkout=$4
if [[ ! $expected_commit =~ ^[0-9a-f]{40}$ || ! $expected_tree =~ ^[0-9a-f]{40}$ ]]; then
  echo 'Exact 40-character commit/tree hashes are required.' >&2
  exit 2
fi
if [[ -e $new_checkout ]]; then
  echo 'NEW_CHECKOUT must not already exist. Existing checkouts are preserved.' >&2
  exit 2
fi
git -C "$source_repo" fetch origin research/local-frontier-20261024
git -C "$source_repo" cat-file -e "${expected_commit}^{commit}"
actual_tree=$(git -C "$source_repo" rev-parse "${expected_commit}^{tree}")
[[ $actual_tree == "$expected_tree" ]] || { echo 'Commit tree mismatch.' >&2; exit 2; }
git -C "$source_repo" worktree add --detach "$new_checkout" "$expected_commit"
[[ $(git -C "$new_checkout" rev-parse HEAD) == "$expected_commit" ]]
[[ $(git -C "$new_checkout" rev-parse 'HEAD^{tree}') == "$expected_tree" ]]
[[ -z $(git -C "$new_checkout" status --porcelain --untracked-files=all) ]]
printf 'Prepared exact clean checkout: %s\n' "$new_checkout"
