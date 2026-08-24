# Codex launcher — VigilODE PM-4 Task-1 R5 CWD-independent recovery

This launcher is intentionally stored inside the fetched handoff branch. It
has no dependency on a ChatGPT sandbox download or on a file under
`~/vigilode/` that is not tracked by Git.

## 1. Resolve the fetched handoff root

```bash
HANDOFF_ROOT="$HOME/vigilode-pm4-cwd-surgical-handoff/handoff/pm4-task1-r5-cwd-surgical-repair-20260824"
test -d "$HANDOFF_ROOT"
```

## 2. Re-run the control-plane gate

```bash
export PM4_R4_ARCHIVE="$HOME/vigilode/VIGILODE_PM4_TASK1_SCHEMA_BOUNDARY_KIT_R4_20260824.tar.gz"
export PM4_R4_SIDECAR="$HOME/vigilode/VIGILODE_PM4_TASK1_SCHEMA_BOUNDARY_KIT_R4_20260824.tar.gz.sha256"

cd /tmp
bash "$HANDOFF_ROOT/acceptance/run_control_plane_preflight.sh"
```

Required final marker:

```text
PASS: CWD-independent PM-4 control-plane preflight
```

## 3. Reconstruct the complete handoff package from the fetched Git branch

```bash
bash "$HANDOFF_ROOT/RECONSTRUCT_AND_VERIFY.sh" \
  --output-dir "$HOME/vigilode-pm4-cwd-surgical-runtime"
```

Required final marker:

```text
RECONSTRUCTION_COMPLETE
```

Resolve the extracted package:

```bash
PACKAGE_ROOT="$(cat "$HOME/vigilode-pm4-cwd-surgical-runtime/PACKAGE_ROOT.txt")"
test -d "$PACKAGE_ROOT"
test -f "$PACKAGE_ROOT/IMPLEMENTER_PROMPT.md"
```

## 4. Read the authority and executable prompt

```bash
cat "$HANDOFF_ROOT/CANONICAL_HANDOFF.md"
cat "$PACKAGE_ROOT/README_FIRST.md"
cat "$PACKAGE_ROOT/AGENTS.md"
cat "$PACKAGE_ROOT/AUDIT_COMPILED_EXEC_PLAN.yaml"
cat "$PACKAGE_ROOT/IMPLEMENTER_PROMPT.md"
```

Start a **new Codex session** in `~/vigilode` and provide the complete contents
of `IMPLEMENTER_PROMPT.md`. Tell Codex that the immutable handoff inputs are:

```text
repository
~/vigilode

repository-local control-plane handoff
$HANDOFF_ROOT

extracted executable package
$PACKAGE_ROOT

canonical main
140f6b5c078c3d8fcd5b6c52310c063ee233dc12

expected feature before publication
b2d5ec41cb147e01aadbc9c42928da8abfa75c58

PR #11
OPEN / DRAFT / UNMERGED
```

Codex must execute `IMPLEMENTER_PROMPT.md` completely. It must not ask user
questions, guess across a specification boundary, mutate `main`, force-push,
merge, run wall timing, rank candidates, or start Task 2.

On any failed gate it must emit:

```text
BLOCKED_BY_UNRESOLVED_SPEC
```

and stop before production mutation. On success it may publish only one normal
non-force fast-forward to PR #11's feature branch, validate the completion
evidence, and stop for a fresh-context review.

## 5. Fresh-context review after successful publication only

```bash
cat "$PACKAGE_ROOT/FRESH_REVIEW_PROMPT.md"
```

The reviewer must report findings first and must not repair on the first pass.
Pass requires `P0=0` and `P1=0`. Merge, wall timing, and Task 2 remain forbidden.
