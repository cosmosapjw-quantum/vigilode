# Publication package review

## Scope and review identity

This is a read-only review of the package builder, the research node's ignore
rules, and its archive verifier. No native test, benchmark, profiler, archive
build, extraction, Git index mutation, or source/configuration edit was run.
The only file written by this reviewer is this report. The five-package native
gate was still active; its outcome is outside this review.

The reviewed builder is the publication staging script
`/workspace/scratch/06cfffc081e2/tmp/production_package_builder.py`. Its SHA-256
was `17980430f37b2d0be5f6c0f236c1127490adde83af64fd0cd8db60bf6d227c10`.
The reviewed [verifier](../verify_package.py) SHA-256 was
`b7bf3d0be2a58aaeb36cb1d4a196fcf47989c718249a08bad9f59293a7f8824b`,
and the reviewed [ignore rules](../.gitignore) SHA-256 was
`82fa494a74f46c5521c6ab40d04cb0f0d337b73659189ec7365f2a86c50f314a`.
Later corrected versions need their own final package execution receipt.

**Decision:** the current pre-staging file selection includes the intended
changed solver code and excludes the identified fetched external text. The
inventory and verifier need the closure fixes below before their results are
used as a repeatable final publication gate. No claim of a completed clean
checkout verification is made here: the final manifests/archive did not yet
exist when this review began.

## Findings and concrete corrections

### P1 — Git selection loses staged code

The builder obtains tracked paths with `git diff --name-only -z` and new crate
paths with `git ls-files --others --exclude-standard -z crates`. The first
command compares the working tree to the index; the second omits indexed files.
After the intended runtime changes are staged, neither command identifies those
paths. Re-running the builder at that ordinary publication step can therefore
remove the runtime files from both `SOURCE_MANIFEST.json` and the explicit Git
inventory. A clean checkout after publication has the same omission because
the changes are then committed, rather than dirty.

Use the pinned baseline-to-current tree/index/worktree difference, plus the
intended new-file scopes, or preserve an explicit reviewed publication
inventory. Account for removed paths separately instead of attempting to hash
a nonexistent file. Verify that the same code inventory is obtained before and
after staging. The current working tree was not staged or modified to produce
this finding; it follows directly from the two Git command contracts.

### P1 — Final selection bypasses generated/external exclusions

The builder applies `generated_exclusions` and the curated external-text
exclusions during the research-directory walk, but then unions unfiltered
`tracked_changes` into `selected`. On a later run, a modified tracked
`SOURCE_MANIFEST.json` can be hashed into its own replacement. A modified
`MANIFEST.json` can similarly enter the source manifest even though it contains
the new source-manifest hash. These are self-referential hash dependencies.
An accidentally tracked excluded receipt would also bypass the external-text
exclusion at this same union.

Apply the exclusion policy to the final selected set as well. Keep the intended
acyclic order explicit: ordinary files/archive members, raw archive, raw
manifest, source manifest, outer manifest, then a verification receipt outside
that hash graph. `RAW_MANIFEST.json` can be deliberately hashed by the source
manifest because it is already complete and is excluded from archive members.
The outer manifest and verification receipt must not become their own inputs.

At this review's initial snapshot, the generated manifests were untracked and
absent, so the *first* construction did not yet contain this cycle. The finding
concerns the normal staging/rebuild lifecycle, not a fabricated current hash
mismatch.

### P2 — Build input coverage is narrower than complete source identity

The prospective source manifest contains changed/new runtime files, but it
omits unchanged `Cargo.lock`, the root `Cargo.toml`, and both changed packages'
`Cargo.toml`. The current inclusion reconstruction confirmed all four omissions.
They determine the actual rebuild despite being unchanged relative to PR91.
A subsequent local edit to one of those files would not be detected by this
source-manifest loop. Archived execution-time `SOURCE_INPUTS.json` records are
useful evidence, but the verifier does not compare their entries with the
current checkout.

Explicitly include the locked dependency/build inputs and the source scope
needed for the supported rebuild, or verify a pinned complete Git tree and
clearly state that the narrower manifest checks only the changed files. Avoid
interpreting a selected-change hash check as complete rebuild identity.

### P2 — Outer manifest and overlapping snapshots are not verified

`verify_package.py` never reads `MANIFEST.json`. Consequently it does not check
the declared source-manifest hash, raw-manifest hash, or report PDF hash. The
PDF is omitted by the source-manifest extension filter. In default verification
mode, changing the current report PDF alone does not change any checked source
entry; the archive's original PDF still verifies against its own member hash.
Extraction mode would discover the differing destination, but default mode
does not make that comparison.

The archive and source manifest are also read from disk at different times.
If an ordinary document/source changes between those reads, each snapshot can
match its own manifest while the two versions disagree. Default verification
does not compare the overlapping member/source hashes. Extraction will then
refuse to overwrite the conflicting checkout file. This matters while final
reports or test-produced evidence are still being written.

Validate the outer manifest's links and report identity. For every path present
in both the source manifest and raw members, require the same SHA-256. Build
only after the final input freeze and fail if any input changes during packing.
Keep the default verifier's scope explicit if it intentionally leaves restored
local evidence unchecked.

### P2 — Extraction rejects textual traversal but not symlink ancestors

The verifier correctly rejects absolute member names, `..` components and
non-regular tar entries. It writes with `root / name`, however, without checking
resolved path confinement or existing symlink ancestors. If a checkout parent
directory is a symlink, an otherwise acceptable relative member can write
outside the intended repository. No symlink existed anywhere in the reviewed
research node, so this is a concrete extraction-contract gap rather than an
observed escape in this package.

Resolve and confine destinations to the repository, reject symlink traversal,
and reject duplicate tar member names before converting the list into a
dictionary. Apply relative-path validation to manifest-specified source/archive
paths as well. These checks belong to the packaging helper and do not require
new solver experiments.

### P3 — Inventory omits its own Git entry

The builder serializes `GIT_FILE_INVENTORY.json` and only then adds that path to
the final staging list. Thus the JSON's `files` list and the generated
`production_git_files.txt` differ by that one entry. Add the inventory path
before writing its path list, or state explicitly that the inventory excludes
itself. Listing its own *path*, without hashing its contents, creates no hash
cycle.

### Publication freeze — preserve and restore both historic oracle outputs

At review time, `git status` showed two historical tracked evidence files
modified by native tests:

- `research/production_audit_20261018/evidence/continuation_a01_lcert/oracle_stdout.json`
- `research/production_audit_20261018/evidence/continuation_a01_unequal_reference/oracle_stdout.json`

The current builder would include both because it accepts every tracked change.
The integrator already intends to retain these new outputs in the current
campaign and restore the historical originals. Both paths must be included in
that operation before the final inventory is frozen. This review did not alter
either file.

## Checks that passed in the current content inspection

- A read-only reconstruction of the current selection included
  `production.yml`, all three changed solver lanes, the new production API,
  crate-local tests/examples, and the frozen PR91 structural control. No
  currently required changed runtime file was found missing before staging.
- The five fetched-text receipts named in
  [LITERATURE_MAP.json](../evidence/LITERATURE_MAP.json) were absent from both
  the prospective raw archive and the individual selected Git entries.
  The source receipts remain local; the published mapping retains metadata,
  provenance hashes and applicability notes.
- [STRUCTURAL_WOLFRAM_OWN_MATH.json](../evidence/STRUCTURAL_WOLFRAM_OWN_MATH.json)
  contains only the two project-math subrecords. Each `value` equals its
  corresponding original subrecord, and the curated canonical hashes agree.
  The mixed original's `exa` record is not included. Its exact Exa input query
  was not retained and was not reconstructed from returned titles.
- The include/exclude scan found no additional fetched-public-text receipt
  outside the five exclusions. Other matching structured `content` records
  were project-derived Wolfram work or compiler diagnostics. This is a scoped
  content inspection, not a general secret scanner certification.
- Raw member names are relative to the repository; `node.parents[1]` in the
  verifier points to the repository root. The report build script locates its
  own directory and restores archived TeX support through `--extract-evidence`.
  The current archive layout and verifier destination layout agree.
- Raw receipts, source-input hashes, profiling traces, failures and TeX support
  remain archive inputs despite their `.gitignore` rules. The candidate set
  contained 500 regular files and 32,338,341 input bytes at the inspection
  snapshot; these are not final archive counts or compressed sizes.
- The archive uses sorted paths, fixed tar timestamps/owner metadata and fixed
  regular-file modes. First-run hash dependencies are acyclic while generated
  manifests stay excluded. No symlinks were present in the inspected node.

## Clean-checkout closure still required

After the native gate stops and the findings above are resolved, create the
final archive/inventory from frozen inputs. Verify both the normal package and
`--extract-evidence` from an isolated checkout containing only the exact
published files. Verify a second extraction is unchanged, inspect the code and
build-input hashes, and confirm the fetched-text receipts are not archive or
Git entries. The same check should confirm that the root README, DAG, report,
manifest links and all eight execution-suite definitions are available.

The staging builder itself is outside the repository and hardcodes this
workspace. The documented *experiment* reproduction helpers are repository
relative, but recreating the publication archive from a clean checkout is not
currently a published helper contract. If archive rebuilding is intended as a
supported operation, publish a portable builder or document its exact process
and inputs. This limitation does not by itself prevent extraction of a valid
published archive.

No execution status, performance number or solver correction budget was changed
by this review. The final integration receipt should record which findings
were corrected and the final verifier/build identities.
