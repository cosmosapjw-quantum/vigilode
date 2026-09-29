#!/bin/bash
# Build-provenance fixtures for crates/rodas5p-fair-ab/build.rs (audit F-067,
# external audit VIG-A03). Builds `rodas5p-fair-ab` from a source snapshot in
# several layouts and prints the revision and dirty flag the build script
# emits. Exit status is 0 only if every layout gets the expected identity.
#
#   tools/check_build_provenance_fixtures.sh [REV]   (default: HEAD)
#
# Layouts:
#   archive   `git archive` export, no git anywhere above it -> unknown/unknown
#   nested    the same export untracked inside an unrelated parent repository
#             -> unknown/unknown (must not inherit the parent commit)
#   mutated   `nested` after editing a source file -> unknown/unknown
#   subdir    the export committed as a subdirectory of the parent repository
#             -> unknown/unknown (only a checkout rooted at the workspace counts)
#   checkout  a clone of REV -> REV/false; with an untracked file -> REV/false;
#             after a tracked edit -> REV/true
#   worktree  a linked worktree of that clone (.git is a file) -> REV/false
#
# Uses a private target directory and a scratch directory under $TMPDIR;
# nothing in the repository is modified.
set -u
REPO=$(git -C "$(dirname "$0")/.." rev-parse --show-toplevel)
REV=$(git -C "$REPO" rev-parse "${1:-HEAD}")
WORK=$(mktemp -d "${TMPDIR:-/tmp}/vigilode-provenance.XXXXXX")
export CARGO_TARGET_DIR="$WORK/target" CARGO_PROFILE_DEV_DEBUG=0
export GIT_CEILING_DIRECTORIES="$WORK"
trap 'rm -rf "$WORK"' EXIT
failures=0

git_quiet() { git -c user.name=fixture -c user.email=fixture@invalid -c init.defaultBranch=main "$@" >/dev/null; }

identity() {
    local dir=$1
    rm -rf "$CARGO_TARGET_DIR"
    if ! (cd "$dir" && cargo check -p rodas5p-fair-ab --locked >"$WORK/build.log" 2>&1); then
        echo "build failed"; tail -5 "$WORK/build.log"; return
    fi
    local out
    out=$(cat "$CARGO_TARGET_DIR"/debug/build/rodas5p-fair-ab-*/output)
    local rev dirty
    rev=$(sed -n 's/^cargo:rustc-env=VIGILODE_DETECTED_GIT_REVISION=//p' <<<"$out")
    dirty=$(sed -n 's/^cargo:rustc-env=VIGILODE_SOURCE_DIRTY_AT_BUILD=//p' <<<"$out")
    echo "$rev/$dirty"
}

expect() {
    local label=$1 dir=$2 want=$3 got
    got=$(identity "$dir")
    if [ "$got" = "$want" ]; then
        echo "ok    $label: $got"
    else
        echo "FAIL  $label: got $got, want $want"
        failures=$((failures + 1))
    fi
}

export_to() { mkdir -p "$1" && git -C "$REPO" archive "$REV" | tar -x -C "$1"; }

# archive
export_to "$WORK/archive"
expect archive "$WORK/archive" unknown/unknown

# nested and mutated: parent repository with one commit, export untracked
mkdir -p "$WORK/parent"
git_quiet -C "$WORK/parent" init
echo parent >"$WORK/parent/README"
git_quiet -C "$WORK/parent" add README
git_quiet -C "$WORK/parent" commit -m parent
export_to "$WORK/parent/exports/vigilode"
expect nested "$WORK/parent/exports/vigilode" unknown/unknown
echo "// mutated" >>"$WORK/parent/exports/vigilode/crates/rodas5p-fair-ab/src/lib.rs"
expect mutated "$WORK/parent/exports/vigilode" unknown/unknown

# subdir: the export committed inside the parent repository
rm -rf "$WORK/parent/exports/vigilode"
export_to "$WORK/parent/exports/vigilode"
git_quiet -C "$WORK/parent" add exports
git_quiet -C "$WORK/parent" commit -m subdir
expect subdir "$WORK/parent/exports/vigilode" unknown/unknown

# checkout
git_quiet clone -q --no-checkout "$REPO" "$WORK/checkout"
git_quiet -C "$WORK/checkout" checkout -q --detach "$REV"
expect checkout "$WORK/checkout" "$REV/false"
touch "$WORK/checkout/untracked_scratch_file.txt"
expect checkout-untracked "$WORK/checkout" "$REV/false"
rm "$WORK/checkout/untracked_scratch_file.txt"
echo "// tracked edit" >>"$WORK/checkout/crates/rodas5p-fair-ab/src/lib.rs"
expect checkout-tracked-edit "$WORK/checkout" "$REV/true"

# worktree: a linked worktree has a .git file, not a directory
git_quiet -C "$WORK/checkout" worktree add -q --detach "$WORK/worktree" "$REV"
expect worktree "$WORK/worktree" "$REV/false"

echo "failures: $failures"
[ "$failures" -eq 0 ]
