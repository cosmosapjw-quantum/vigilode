# WU-05 host preparation and repair — bootstrap-v2 revision 2

## A. Authority and Scope

Preserve user-reported local review `e95ce1e58a603306cb665a6ab91cfe02d279972f`, tree `e3621a370297a76907e97730ebd18c5c1e0fb83e`, on `research/k0-stage-telemetry-integration-20260827`. WU-00–04 and their raw 12-cell data are not replayed here.

Use the exact NEW package SHA from the external publication receipt, not historical baseline `13aed8dabfbb5da4381d9d73d3cb0c0403ad5354` or a moving branch. GitHub owns source bytes; PM-7 owns progress; Confluence 15499267 is the control mirror.

The same host Codex session is authorized as HOST_CODEX_ORCHESTRATOR to run the pinned script, then as WU-05 implementer after readiness. The old separate-human-preparation prerequisite is superseded only for this script.

## B. P0/P1 Threat Catalogue

BR-ENTRY: absent helper called before merge. BR-MARKER: exit zero without required marker coverage. BR-SCOPE: non-control source hidden in package. BR-REENTRY: completed preparation cannot be retried. BR-COVERAGE: manifest excludes executable payload/bootstrap additions. BR-INHERITANCE: approved review-side validator repair replaced by stale package bytes. These are P1 preparation defects; the five original source-review findings retain their classifications.

## C. Invariant/Test Matrix

`python3 -B tools/test_k0_bootstrap_v2.py` is the primary regression suite: eleven tests, including actual package/supplement validator integration. Histories and metadata are synthetic; eight lifecycle cases also use explicit dependency stand-ins. This is not an actual e95 replay or solver evidence.

Existing manifests are still hash-checked. The exact pinned Git tree supplies exhaustive additional control-file coverage, including the compressed payload. The existing loader applies only the inventory compatibility repair; `--dump-source` exposes all effective code. Evidence, schema, numerical and signed-residual validation functions are unchanged.

## D. Ordered Work Units

PREPARE first. Locate the known worktree; if its path moved, inspect `git -C ~/vigilode worktree list --porcelain`, without creating/replacing branches. Run:

```bash
set -euo pipefail
: "${K0_PACKAGE_SHA:?exact externally supplied package SHA required}"
REPO=/tmp/vigilode-k0-stage-telemetry.kAguIL/tree
cd "$REPO"
test "$(git branch --show-current)" = research/k0-stage-telemetry-integration-20260827
test -z "$(git status --porcelain=v1)"
git fetch --prune origin docs/k0-codex-execution-package-20260827
test "$(git rev-parse origin/docs/k0-codex-execution-package-20260827)" = "$K0_PACKAGE_SHA"
BOOT=$(mktemp "${TMPDIR:-/tmp}/k0-bootstrap-entry.XXXXXX")
trap 'rm -f -- "$BOOT"' EXIT
git show "$K0_PACKAGE_SHA:tools/k0-wu05-bootstrap-v2.sh" > "$BOOT"
bash "$BOOT" --repo-root "$REPO" --package-sha "$K0_PACKAGE_SHA"
```

Do not invoke `verify-k0-fresh-review-repair.py` from the unprepared tree. The runner validates real dependencies in a detached worktree before merging. It requires exit codes AND structured PASS fields, including existing `legacy_marker` and `pin_marker` aliases.

It accepts the original clean review or its exact existing clean `[review, package]` merge. A retry creates no second commit. If a required pre-existing path was untouched on the package side since merge-base, exact review-side bytes are retained; otherwise package bytes are required. Conflict means abort, never guessed resolution. Post-merge failure retains the merge and logs. `BOOTSTRAP_RECEIPT=...` identifies logs under the Git common directory, outside tracked source.

REPAIR only after `LOCAL_WU05_AUTHORITY_READY`. Read `WU05_LOCAL_CODEX_PROMPT.md` and its existing bridge/evidence-v3/fresh-review contracts. Reproduce and repair the five original findings; do not rerun WU-00–04 or invent missing historical evidence. Retain raw campaign bytes; wrapper migration is not a new solver run.

## E. Fresh-Context Review Contract

Reuse one read-only fresh repair review with exact base/final SHA, diff, controlling contracts, actual logs and unresolved findings. Do not treat implementer reasoning as evidence. P0/P1 must close; retain P2/P3 without inflation.

## F. Final Differential Audit Contract

Audit only the repaired delta, evidence validity, scope and missed failure classes. No whole-project restart or review-of-review. Publish a draft stacked implementation PR only after existing closure conditions pass; no PR merge or activation.

## G. Unresolved Specification Boundaries

Bootstrap behavior is specified and tested. Actual unpushed e95 implementation, five exact reproducers, raw campaign files and source repairs require local access. Their completion is not claimed. Stop on genuinely contradictory raw evidence; do not fabricate a wrapper value.

## H. Process-Cost Assessment

Keep the same bootstrap-v2 entry and existing gates; no new review tier, evidence schema or solver abstraction. This package change requires no Cargo build or campaign replay. After actual implementation publication, synchronize GitHub, PM-7 and Confluence 15499267; report ATLAS_SYNC_PENDING for an outage rather than claiming Done.

No Cargo graph, equation, tolerance, convergence, output, production route/signature, homotopy certificate, timing/ranking/speedup, tag, release or source/control PR merge is authorized.
