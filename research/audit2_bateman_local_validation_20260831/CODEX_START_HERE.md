# HOST_CODEX_ONLY prompt: exact Bateman one-shot execution and readback

This prompt is for a local Codex executor after the pre-execution controller
PR has been published and read back. It authorizes one guarded candidate launch
attempt and a candidate-free independent adjudication. It does not authorize
threshold changes, case selection, reruns, holdout access, remote writes,
merge, tag, release, Jira/Confluence mutation, PM-7/K0 mutation, or local-LLM
use.

Current scientific pre-execution fact: **candidate executions = 0**. The
canonical state root now contains one sealed candidate-free preflight failure,
run `20260831T044826Z-bd0730fa`, and its independent inconclusive sidecar. It
contains no one-shot marker or candidate launch.

## 1. Stop until publication is concrete

Bind the controller to these already-published immutable values:

```text
PREEXECUTION_PROTOCOL_COMMIT_SHA = b5f553b5be24598c71bc7af15c97e67f503610a5
PREEXECUTION_PROTOCOL_TREE_SHA   = 7e96da6f612eade18acc77b700b6e847d68160c9
PREEXECUTION_DRAFT_PR_URL        = https://github.com/cosmosapjw-quantum/vigilode/pull/40
RUNNER_SHA256                    = f53f5bc2ea77721adc562c2640a58d24ae975f14795f7401c750c900c2980f29
ADJUDICATOR_SHA256               = 28697e81ea39532a2ffa86789a426c0f0a7107a1ea1a1a4269bc333cc8cf977d
```

Require PR #40 to be open, Draft, unmerged, stacked directly on PR #39, and
fully green for its required checks at the final documentation head. Verify
that the controller commit/tree above is its ancestor and read back the two
script hashes from that commit. Any mismatch means
`INCONCLUSIVE_PREEXECUTION_PROTOCOL_UNRESOLVED`; stop before any one-shot
marker or candidate process.

Known immutable chain:

```text
repository                  cosmosapjw-quantum/vigilode
PR #38 head                 f954e39130e5141256731d0745666a872c0267ea
PR #38 tree                 4314da2f9e1533737d4169526ebd2d84515ab19d
PR #39 state                OPEN / DRAFT / UNMERGED
PR #39 head                 6b00a886c4eb38d3fe199e3d77852cc1eb35eb39
PR #39 tree                 4a9ede5c442514f1ae86d018419a2afeee5b6d01
execution source head       6b00a886c4eb38d3fe199e3d77852cc1eb35eb39
execution source tree       4a9ede5c442514f1ae86d018419a2afeee5b6d01
execution source parent     cac7d1b7337a6dff25a60072009658f6ddf155d9
implementation origin       cac7d1b7337a6dff25a60072009658f6ddf155d9
implementation tree         c23abbee0d47e2dbe002e01516bf34e2481bc333
implementation parent       f954e39130e5141256731d0745666a872c0267ea
implementation parent tree  4314da2f9e1533737d4169526ebd2d84515ab19d
```

## 2. Prepare two isolated, clean checkouts

Use one control checkout at the exact repaired pre-execution protocol commit
and one source worktree at the exact PR #39 execution-source head
`6b00a886c4eb38d3fe199e3d77852cc1eb35eb39`. Preserve all existing checkouts,
worktrees, untracked files, the sealed initial preflight failure, and its
adjudication sidecar.

The execution source is the direct documentation-only child of the scientific
implementation origin. `cac7d1b..6b00a886` changes exactly 13 documentation
paths and no `crates/`, `tools/`, Cargo, toolchain, or workflow path. The two
compile-time documents absent at `cac7d1b...` are fixed source bytes at the
execution head. Do not use an untracked overlay or claim that a `6b00a886...`
checkout is the `cac7d1b...` implementation commit.

Before proceeding, independently confirm in the source worktree:

```text
HEAD                         6b00a886c4eb38d3fe199e3d77852cc1eb35eb39
HEAD tree                    4a9ede5c442514f1ae86d018419a2afeee5b6d01
HEAD parent                  cac7d1b7337a6dff25a60072009658f6ddf155d9
scientific implementation tree  c23abbee0d47e2dbe002e01516bf34e2481bc333
scientific implementation parent f954e39130e5141256731d0745666a872c0267ea
implementation parent tree      4314da2f9e1533737d4169526ebd2d84515ab19d
worktree status      clean, including untracked files
```

Use Python `3.12.13` with NumPy `2.3.5` and mpmath `1.3.0`, and Rust/Cargo
`1.94.1`. The controller will recheck all identities, versions, frozen source
hashes, the authority proof, candidate-free tests, readiness, clippy, format,
diff, and candidate binary binding. Do not pre-empt or replace its plan with a
manual candidate command.

The candidate build target is not a caller choice. The controller derives the
exact run-specific path
`<CANONICAL_STATE_ROOT>/build-cache/6b00a886c4eb38d3fe199e3d77852cc1eb35eb39/<run-id>`
and rejects an ambient build override.

The controller must operate offline. Dependency fetch or toolchain repair, if
needed, must finish before this protocol begins and must not create candidate
output. Stop if the fixed offline preflight cannot be satisfied.

## 3. Understand the irreversible boundary

The controller accepts no state-root argument. It derives the canonical
machine-account-global state root from the operating-system passwd entry,
independently of a caller-supplied `HOME`:

```text
<PASSWD_ACCOUNT_HOME>/.local/state/vigilode/bateman-local-six-case-v1
```

The one-shot marker is durably created before the candidate spawn. Once the
marker exists, the attempt is consumed even if spawn fails, the process exits
nonzero, output is partial or malformed, validation fails, or sealing later
becomes inconclusive. Do not delete or relocate the marker, choose another
account or state root to evade it, or rerun any scenario.

Inspecting whether the marker already exists is allowed. If it exists, do not
invoke the controller; preserve and return the existing state as
`INCONCLUSIVE_ONE_SHOT_ALREADY_CONSUMED` unless an already sealed package is
being independently adjudicated.

The state root itself is expected to exist because it retains the sealed
candidate-free run `20260831T044826Z-bd0730fa` and sidecar
`20260831T044826Z-bd0730fa-preflight`. Their `SHA256SUMS` and adjudication
hashes are frozen in `evidence/VERIFICATION.md`. State-root existence is not a
candidate attempt; any `BATEMAN_CANDIDATE_ATTEMPT.*` marker is.

## 4. Run exactly one controller command

Set three operational absolute paths. They locate immutable bytes and do not
select a scientific value:

```text
PYTHON_ABS      = absolute Python 3.12.13 executable
CONTROL_ROOT    = clean checkout at PREEXECUTION_PROTOCOL_COMMIT_SHA
SOURCE_WORKTREE = clean checkout at 6b00a886c4eb38d3fe199e3d77852cc1eb35eb39
```

From `CONTROL_ROOT`, execute exactly:

```bash
"$PYTHON_ABS" tools/run_audit2_bateman_local_validation.py \
  --source-worktree "$SOURCE_WORKTREE"
```

There are no scientific knobs. Do not add flags or environment overrides,
change the six-case order, call the example directly, select individual cases,
or run Cargo separately after this point. Internally, the single candidate
argv is fixed to:

```text
cargo run --locked -p rodas5p-integrators --features audit2-bateman-authority --example audit2_bateman_local_six_case
```

Record the controller's exit code and JSON stdout. Exit `0` is a provisional
controller accept, exit `1` a provisional scientific reject, and exit `2` an
inconclusive controller outcome. None is final before independent package
adjudication. Do not rerun around any outcome.

## 5. Preserve and inspect the sealed package

The controller prints the new package path below the canonical state root.
Preserve it exactly. Depending on where a failure occurred, expected retained
files include:

- `execution_manifest.json`, `events.jsonl`, and `SHA256SUMS`;
- `authority_bundle_sha256.txt` and all available command logs;
- `attempt_lock.json` and `candidate_launch.json` when launch was committed;
- `result_summary.json` when the candidate was attempted, including partial or
  malformed bytes;
- `validator_attempt.json` and any `local_receipt_validation.json` produced by
  the frozen validator.

Verify the package file set and every digest in `SHA256SUMS` without editing a
package byte. Never insert the later adjudication output into the sealed
package.

## 6. Run the independent, candidate-free adjudicator once

Use the adjudicator from the same exact control commit and the same exact
execution source worktree. Choose a new sidecar output path outside the
sealed package. From `CONTROL_ROOT`, execute:

```bash
"$PYTHON_ABS" tools/adjudicate_audit2_bateman_local_validation.py \
  --package "<ABSOLUTE_SEALED_PACKAGE_PATH>" \
  --source-worktree "$SOURCE_WORKTREE" \
  --out "<ABSOLUTE_NEW_SIDECAR_DIRECTORY>"
```

This command may rerun the frozen receipt validator but must not build, launch,
or otherwise execute the candidate. Preserve its stdout/stderr, exit code, and
sidecar JSON separately. Do not adjudicate with modified scripts or a different
source checkout.

The final evidence class is tri-state:

- **ACCEPT** only for an eligible sealed package whose six frozen contracts
  all pass. This is not a formal `Ehat + Theta <= 1`, `sum_i a_i q_i`, and
  per-solve-GMRES-telemetry certificate;
- **REJECT** only for an eligible sealed package carrying a valid scientific
  failure under the frozen rules;
- **INCONCLUSIVE** for any unresolved source, environment, spawn, controller,
  validator, integrity, sealing, provenance, or adjudication condition.

An infrastructure failure is not a scientific rejection. A scientific failure
is not a rerun opportunity.

## 7. Interpret the mathematics without expanding admission

The bound ledger verdict is
`COMPLETE_WITH_CLIENT_DEPENDENT_CONSTANTS`. Apply this frozen mapping only
after adjudication:

| Ledger IDs | Allowed result |
|---|---|
| `M01` | `PREEXISTING_MATHEMATICAL_AUTHORITY` |
| `X01` | Evaluated only for an eligible ACCEPT or REJECT package; otherwise not established |
| `M02`, `M03` | At most partial if the eligible receipt contains the necessary telemetry; otherwise `NOT_EVALUATED` |
| `M04`–`M12`, `X02` | `NOT_EVALUATED` for every outcome |

Do not treat the changed-W cache probe as changed-W output accuracy. Do not
infer speed, scalability, general preconditioner behavior, general real-client
accuracy, fifth-order preservation, cross-step reuse safety, root-distance
certification, observable accuracy, or performance from this package.

The authority manifest's IEEE-754 bits prevail over ledger decimal rendering;
the prechecked daughter discrepancy is below `5.1e-18` and inside the frozen
`1e-15` bound. This establishes no new numerical admission tolerance.

Do not invoke Wolfram/xAct, SageMath/Singular, Lean/mathlib, or Rocq as an
admission substitute. Those tools are outside the frozen command plan and
cannot replace empirical execution or receipt evidence. Their availability
does not authorize a new threshold or a higher claim ceiling.

## 8. Return the complete readback

Return, without result fitting:

- the concrete protocol commit/tree, Draft PR URL/state/stack, remote check
  results, runner hash, and adjudicator hash;
- exact execution-source head/tree/parent, separate scientific implementation
  origin/tree/parent, and frozen source-hash comparison;
- canonical state root, one-shot marker path/hash, package path, and package
  `SHA256SUMS` hash;
- every command launch status and exit code;
- raw candidate and validator bytes/hashes when attempted;
- independent adjudication JSON/hash and its tri-state evidence class;
- candidate invocation count, complete failure history, and the frozen math
  coverage table;
- `local_llm_used: false`, `holdout_access: NOT_OPENED_OR_EXECUTED`, and the
  controller's remote-write declaration.

Do not call missing conditional files a scientific failure; report why they
were not produced. Do not call skipped/not-applicable jobs scientific passes.

For every outcome, retain the project ceiling exactly:

> `EXPLORATORY_NONAUTHORITATIVE_REUSABLE_PRECONDITIONER_TRANSACTIONAL_STEP_SUBSTRATE`

No merge, tag, release, holdout, PM-7/K0 closure, production dispatcher change,
or broader scientific claim is authorized by this prompt.
