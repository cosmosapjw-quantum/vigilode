# Frozen preregistration: exact Bateman local six-case execution

This registration is a pre-execution control document. At the time of this
freeze, the Bateman candidate execution count is exactly **zero**. Nothing in
this document is a candidate result, and no result-dependent threshold,
scenario, operator, reference, or admission rule may be added after the first
candidate launch attempt.

An initial published controller invocation stopped in the candidate-free Rust
authority preflight because the implementation-origin commit did not yet
contain two documents required by `include_str!`. The sealed run
`20260831T044826Z-bd0730fa` records candidate and validator invocation counts
of zero, no `attempt_lock.json`, no `candidate_launch.json`, and no one-shot
marker. This candidate-independent amendment supersedes only the executable
source-checkout binding and the adjudicator's conditional-artifact ordering.
All client bytes, cases, thresholds, candidate argv, validator rules,
scientific one-shot key material, and the mathematical ledger remain frozen.

## Publication gate

The controller and independent adjudicator must be published in a new open,
Draft, unmerged PR stacked directly on PR #39, read back byte-for-byte, and
have their required remote checks green before the one-shot marker may be
created. The published controller bytes are bound to these concrete immutable
values:

```text
PREEXECUTION_PROTOCOL_COMMIT_SHA = b5f553b5be24598c71bc7af15c97e67f503610a5
PREEXECUTION_PROTOCOL_TREE_SHA   = 7e96da6f612eade18acc77b700b6e847d68160c9
PREEXECUTION_DRAFT_PR_URL        = https://github.com/cosmosapjw-quantum/vigilode/pull/40
RUNNER_SHA256                    = f53f5bc2ea77721adc562c2640a58d24ae975f14795f7401c750c900c2980f29
ADJUDICATOR_SHA256               = 28697e81ea39532a2ffa86789a426c0f0a7107a1ea1a1a4269bc333cc8cf977d
```

C1 is the immutable control commit above. The C2 documentation-binding commit
may advance the PR head without changing either C1 script byte. A non-Draft or
merged PR, a stack mismatch, a missing C1 ancestor, a controller commit/tree or
script-hash readback mismatch, or a failed/pending required check on the final
PR head is a stop condition.

## Immutable source chain

Repository: `cosmosapjw-quantum/vigilode`

| Object | Exact identity | State or relation |
|---|---|---|
| Preceding PR #38 | head `f954e39130e5141256731d0745666a872c0267ea`; tree `4314da2f9e1533737d4169526ebd2d84515ab19d` | Open, Draft, unmerged |
| Authority PR #39 | head `6b00a886c4eb38d3fe199e3d77852cc1eb35eb39`; tree `4a9ede5c442514f1ae86d018419a2afeee5b6d01` | Open, Draft, unmerged; head branch `research/audit2-real-client-authority-construction-20260830` |
| Executable source checkout | commit `6b00a886c4eb38d3fe199e3d77852cc1eb35eb39`; tree `4a9ede5c442514f1ae86d018419a2afeee5b6d01` | Direct parent is the scientific implementation origin; exact clean source checkout required |
| Scientific implementation origin | commit `cac7d1b7337a6dff25a60072009658f6ddf155d9`; tree `c23abbee0d47e2dbe002e01516bf34e2481bc333` | Direct parent `f954e39130e5141256731d0745666a872c0267ea`; retained as the scientific one-shot identity |
| Execution-control branch | `research/audit2-bateman-local-execution-orchestrator-20260831` | Must be stacked on the published PR #39 head |

The execution controller lives in the future execution-control commit. The
scientific source argument must point to a separate, clean worktree at the
exact executable source checkout above. `cac7d1b..6b00a886` changes exactly 13
top-level/authority documentation paths and changes no `crates/`, `tools/`,
Cargo, toolchain, or workflow path. The `crates` and `tools` Git trees and all
frozen authority/scientific blobs are identical. The controller therefore
records `6b00a886...` truthfully as the execution source while retaining
`cac7d1b...` separately as the scientific implementation origin.

## Frozen source-byte authorities

All paths are relative to the exact executable source worktree.

| Path | SHA-256 |
|---|---|
| `research/audit2_real_client_authority_construction_20260830/authority_manifest.json` | `673045bf6b9e723fceb6a3b8df8e9e9e9075c942cf1c438f0ebd03574dbac360` |
| `research/audit2_real_client_authority_construction_20260830/verify_authority_manifest.py` | `542715ca749efbf2060d608f2089ee8457e32f9c61fd0d35f613d5ecec26487d` |
| `research/audit2_real_client_authority_construction_20260830/evidence/AUTHORITY_VERIFICATION_RECEIPT.json` | `057cceba92fed0d707db1d586b53adebee5aed00583b224811d091f1d453ab12` |
| `research/audit2_real_client_authority_construction_20260830/verify_local_six_case_receipt.py` | `8391e03e6f94f305f2675799c923d787547cad662cef8d8f8384a8c1bbe94e67` |
| `research/audit2_real_client_authority_construction_20260830/CODEX_START_HERE.md` | `ce96761b5cd067fe21e8d01e52a74767a65a8d9eaaa8c2c18ed1db8ca47de776` |
| `research/audit2_real_client_authority_construction_20260830/handoff.json` | `391861375a01a772e918aad28cfee887600b929cb7ed6b00b555a8bbc2aadb91` |
| `crates/rodas5p-integrators/examples/audit2_bateman_local_six_case.rs` | `0873ed8189a7e0f77ebd4eef05ce6067f84958e7f118aa3e686654e7dc3c48f9` |
| `tools/check-audit2-readiness.sh` | `74dc27607ff1fc764e3ea89912b333418c86a1dfb5e3c14764481b94821b7521` |
| `Cargo.lock` | `d9255cd442dfbca2890152549ae7edc60e890aa062a1046d8f0b8e44678d678a` |
| `Cargo.toml` | `86e27546665f923265a8addd3c464ac6017fe35558ab95fe0af7248cd99fb73b` |
| `rust-toolchain.toml` | `f53198ae4fdecfd87da36fe431c771b54c51e975d01c0e99f653bc14d5d48211` |

The environment is also frozen to Python `3.12.13`, NumPy `2.3.5`, mpmath
`1.3.0`, and Rust/Cargo `1.94.1`. A version, executable, source hash, Git
identity, worktree-cleanliness, or offline preflight mismatch is
**inconclusive**, never a scientific rejection.

## Frozen mathematical inputs and scope

The three supplied pre-result inputs are bound as follows:

| Input | SHA-256 |
|---|---|
| `Pasted markdown(20260831-031734).md` | `f01889a4dd46d8d0e87c12bf983605ab1cf78089802a38fc919d6b743cdea016` |
| `VIGILODE_NONCODING_MATH_BLOCKERS_20260831(1).md` | `6da3065246669fc43ee57eec57aff6063081dded1d24c225087d2bddee6d02bb` |
| `VIGILODE_MATH_BLOCKER_LEDGER_20260831(1).json` | `f84753318c31f8c8e5d8a578eae1c7bf9c1f90c0ad6713e2581ad071566e8956` |
| Checked-in semantic JSON mirror, normalized bytes | `7c02043767b0e8d4be9e6b484df132962c359456d2c2e4aa44e2985503731f10` |

The mathematical verdict is
`COMPLETE_WITH_CLIENT_DEPENDENT_CONSTANTS`. The direct PR #39 blocker is the
non-mathematical local scientific execution and its sealed receipt.

The authority manifest's IEEE-754 bit patterns are the numerical admission
authority. The supplied ledger's daughter-value decimal transcription differs
from those bits by at most 94 ULP (less than `5.1e-18`), which is inside the
already frozen `1e-15` reference bound; this comparison adds no tolerance and
does not alter the manifest. A stronger numerical certificate would require
the predeclared `Ehat + Theta <= 1` test, a bound on `sum_i a_i q_i`, and
per-solve GMRES telemetry. The compact six-case receipt does not provide that
certificate.

Coverage is conditional on independent adjudication of an eligible sealed
package:

| Ledger IDs | Frozen treatment |
|---|---|
| `M01` | Pre-existing mathematical authority; this run does not re-prove it. |
| `X01` | Evaluated only if the package is eligible and its scientific outcome is adjudicated as accept or reject. An inconclusive run does not establish it. |
| `M02`, `M03` | At most partially evaluated, and only when an eligible receipt exposes the required finite-precision or per-solve telemetry. Otherwise `NOT_EVALUATED`. |
| `M04`–`M12`, `X02` | Always `NOT_EVALUATED` by this compact six-case protocol. |

In particular, the changed-W case is a binding/cache invalidation probe, not a
changed-W output-accuracy admission. The exact Bateman Jacobi GMRES degree
bound of at most two is a supporting kill test only if the receipt exposes the
needed per-solve telemetry. It is not a post-observation threshold.

Wolfram/xAct, SageMath/Singular, Lean/mathlib, and Rocq may support separate
symbolic or formal derivations, but none is part of this fixed command plan or
admission rule. They cannot substitute for the empirical candidate launch,
sealed receipt, or independent adjudication, and their availability cannot
raise this node's claim ceiling.

## Frozen client, budgets, and scenarios

The exact authority manifest remains the byte-level authority. Its admitted
client is the four-state two-timescale Bateman parent-to-stable-daughter model
with rates `1000` and `1`, initial state `(0.5, 0, 0.5, 0)`, nominal step
`0.001`, and changed-W step `0.0005`. The frozen numerical ceilings are:

| Quantity | Frozen value |
|---|---:|
| outer atol / rtol | `1e-4` / `1e-6` |
| output L2 atol / rtol | `1e-4` / `1e-6` |
| embedded L2 | `2e-4` |
| original-target residual L2 | `1e-10` |
| original-target contraction | `1e-8` |
| declared reference L2 upper bound | `1e-15` |

The six scenarios are fixed in this order:

1. `same-live-context-reuse`
2. `changed-w-invalidation`
3. `nominal-independent-budget`
4. `over-strict-budget-fallback`
5. `late-preconditioner-failure`
6. `terminal-rejection`

No case may be added, removed, reordered, individually rerun, or supplied with
a new threshold after the one-shot attempt.

## Fixed command and one-shot boundary

The controller exposes one operational locator only: the absolute path of the
clean exact executable source worktree. It exposes no state-root, threshold,
case, solver, feature, retry, or scientific-value argument. From the exact
published execution-control checkout, the only authorized controller argv is:

```text
<ABSOLUTE_PYTHON_3.12.13> tools/run_audit2_bateman_local_validation.py --source-worktree <ABSOLUTE_CLEAN_EXECUTION_SOURCE_WORKTREE_AT_6b00a886c4eb38d3fe199e3d77852cc1eb35eb39>
```

The operational path placeholders do not alter scientific inputs. The
controller's candidate argv is exactly:

```text
cargo run --locked -p rodas5p-integrators --features audit2-bateman-authority --example audit2_bateman_local_six_case
```

The state root is derived from the operating-system passwd entry for the
account and cannot be supplied through `HOME` or on the command line:

```text
<PASSWD_ACCOUNT_HOME>/.local/state/vigilode/bateman-local-six-case-v1
```

The candidate build target is also controller-owned and exact:

```text
<CANONICAL_STATE_ROOT>/build-cache/6b00a886c4eb38d3fe199e3d77852cc1eb35eb39/<run-id>
```

It is not a caller argument or ambient build override. The run-specific suffix
prevents a concurrent build-cache collision without changing the one-shot key.

The amendment does not create a new one-shot namespace. The scientific guard
key remains exactly:

```text
one-shot/BATEMAN_CANDIDATE_ATTEMPT.799d2f31e0fcd3e255a1be55c27d0387d798851a36bee75f04710929fa3c3852.json
```

This is a canonical machine-account-global one-shot boundary, not a per-run
temporary directory. Before the candidate spawn attempt, the controller must
durably create the keyed marker below that root. Creation of that marker
consumes the attempt even if process spawn fails, the candidate exits nonzero,
the report is malformed, validation fails, or later source checks fail. The
marker and sealed package must not be deleted, moved to evade the guard, or
recreated under another state root. There is no authorized scientific rerun.

The controller owns the complete order: source identity, source hashes,
environment checks, candidate-free authority/tests/readiness/lint/format/diff,
candidate build and binary binding, immediate prelaunch source recheck,
durable one-shot marker, one candidate attempt, one frozen-validator attempt,
post-run source recheck, execution manifest, event ledger, and `SHA256SUMS`.
Operators must not manually splice or replace any phase.

Candidate-free adjudication closes the package file set and validates any
failed-prefix artifact exactly. It resolves Git from the trusted host path and
uses the adjudicator's current frozen Python, never package-recorded executable
paths.

## Tri-state evidence rule

Only the independent adjudicator may assign the final evidence class:

- **ACCEPT**: the sealed package is eligible and all six frozen contracts pass;
  it is not the stronger `Ehat + Theta`/`sum_i a_i q_i`/GMRES-telemetry
  certificate described above.
- **REJECT**: the sealed package is eligible and supplies a valid scientific
  failure under the frozen rules.
- **INCONCLUSIVE**: source, environment, launch, controller, validator,
  sealing, integrity, provenance, or adjudication infrastructure is unresolved.

Infrastructure failure is never converted into scientific rejection. A
scientific miss is never converted into infrastructure failure merely to
permit a rerun. Both reject and inconclusive outcomes are retained.

## Stopping rules and nonclaims

- Do not open, enumerate, hash, copy, or execute Oregonator holdout content.
- Do not use a local LLM. Execution policy is `HOST_CODEX_ONLY`.
- Do not perform a network operation from the controller command plan.
- Do not merge, tag, release, change PM-7/K0, or alter a production/default
  dispatcher as part of this node.
- Do not claim speedup, scalability, Krylov-basis reuse, general real-client
  accuracy, general or production preconditioning, dense output, general
  events, whole-integration transactionality, or performance.

The claim ceiling remains, for every outcome:

> `EXPLORATORY_NONAUTHORITATIVE_REUSABLE_PRECONDITIONER_TRANSACTIONAL_STEP_SUBSTRATE`
