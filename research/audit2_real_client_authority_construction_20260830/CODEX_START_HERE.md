# HOST_CODEX_ONLY local prompt — frozen Bateman six-case execution

Operate as `HOST_CODEX_ONLY`. Do not use a local LLM for generation, review,
classification, threshold selection, claim admission, or evidence
interpretation. This prompt authorizes local execution of the exact frozen
Bateman six-case package only. It does not authorize any push, PR mutation,
merge, issue comment, Jira/Confluence update, tag, release, or other remote
write.

## 1. Bind the exact published source

Repository: `cosmosapjw-quantum/vigilode`

```text
required branch              research/audit2-real-client-authority-construction-20260830
required implementation head cac7d1b7337a6dff25a60072009658f6ddf155d9
required implementation tree c23abbee0d47e2dbe002e01516bf34e2481bc333
stack base PR                #38 OPEN / DRAFT / UNMERGED
stack base head              f954e39130e5141256731d0745666a872c0267ea
stack base tree              4314da2f9e1533737d4169526ebd2d84515ab19d
```

Do not execute unless both implementation identities above are concrete 40-hex
Git objects and match the final PR receipt. Check out that exact implementation
head in a new isolated worktree, and verify `git rev-parse HEAD` and
`git rev-parse HEAD^{tree}` byte-for-byte. Require a clean worktree. Stop as
`SOURCE_IDENTITY_UNRESOLVED` on any mismatch. Preserve every existing checkout,
worktree, untracked file, and prior log.

Confirm that the implementation head descends from remote base `f954e391...`
and that the base tree is `4314da2...`. The locally equivalent construction
parent `5cf4189...` is provenance only and is not an acceptable checkout for
the published run.

The exact checked-in authority bundle must hash to:

```text
authority_manifest.json
673045bf6b9e723fceb6a3b8df8e9e9e9075c942cf1c438f0ebd03574dbac360

verify_authority_manifest.py
542715ca749efbf2060d608f2089ee8457e32f9c61fd0d35f613d5ecec26487d

evidence/AUTHORITY_VERIFICATION_RECEIPT.json
057cceba92fed0d707db1d586b53adebee5aed00583b224811d091f1d453ab12
```

## 2. Enforce boundaries before any candidate

- Do not open, enumerate, copy, hash, or execute any Oregonator holdout file,
  fixture, test case, or protected corpus.
- Do not edit the manifest, verifier, source, example, thresholds, references,
  digests, PC identities, trial stages, scenario order, or feature list.
- Do not derive a threshold from a candidate output. Do not widen a threshold
  after a miss.
- Do not run an arbitrary caller-built invocation of the raw PR #38 API. Only
  the canonical six-case example named below is authority-eligible.
- If any source or authority check fails, stop before the example.

Create a fresh directory outside the repository. One safe pattern is:

```bash
VIGILODE_BATEMAN_RUN_DIR=$(mktemp -d /tmp/vigilode-bateman-six-case.XXXXXX)
mkdir -p "$VIGILODE_BATEMAN_RUN_DIR/logs" \
  "$VIGILODE_BATEMAN_RUN_DIR/readiness-output"
```

Never reuse or overwrite an earlier execution directory.

## 3. Verify the frozen authority before execution

Run from the exact clean worktree with the repository-pinned Rust toolchain:

```bash
sha256sum \
  research/audit2_real_client_authority_construction_20260830/authority_manifest.json \
  research/audit2_real_client_authority_construction_20260830/verify_authority_manifest.py \
  research/audit2_real_client_authority_construction_20260830/evidence/AUTHORITY_VERIFICATION_RECEIPT.json \
  > "$VIGILODE_BATEMAN_RUN_DIR/authority_bundle_sha256.txt"

python3 research/audit2_real_client_authority_construction_20260830/verify_authority_manifest.py \
  > "$VIGILODE_BATEMAN_RUN_DIR/authority_verification.json" \
  2> "$VIGILODE_BATEMAN_RUN_DIR/logs/authority_verification.stderr.log"

python3 tools/test_audit2_real_client_authority.py -v \
  > "$VIGILODE_BATEMAN_RUN_DIR/logs/python_authority_tests.log" 2>&1

python3 tools/test_audit2_bateman_local_receipt.py -v \
  > "$VIGILODE_BATEMAN_RUN_DIR/logs/python_local_receipt_tests.log" 2>&1

cargo test --locked -p rodas5p-integrators \
  --features audit2-bateman-authority \
  --test audit2_real_client_authority_contracts \
  -- --nocapture --test-threads=1 \
  > "$VIGILODE_BATEMAN_RUN_DIR/logs/rust_authority_contracts.log" 2>&1

AUDIT2_OUTPUT_DIR="$VIGILODE_BATEMAN_RUN_DIR/readiness-output" \
  bash tools/check-audit2-readiness.sh \
  > "$VIGILODE_BATEMAN_RUN_DIR/logs/readiness.log" 2>&1

cargo clippy --locked -p rodas5p-integrators -p rodas5p-fair-ab \
  --all-targets --features rodas5p-integrators/audit2-bateman-authority \
  -- -D warnings \
  > "$VIGILODE_BATEMAN_RUN_DIR/logs/clippy.log" 2>&1

cargo fmt --all -- --check \
  > "$VIGILODE_BATEMAN_RUN_DIR/logs/fmt.log" 2>&1

git diff --check \
  > "$VIGILODE_BATEMAN_RUN_DIR/logs/diff-check.log" 2>&1
```

Require every exit to be zero. Require the authority verifier to report
`AUTHORITY_CONSTRUCTION_VERIFIED`, `candidate_executions: 0`, two operator
cases, six scenarios, and `NOT_OPENED_OR_EXECUTED`. Stop as
`AUTHORITY_PREFLIGHT_FAILED` if any condition or exact SHA-256 differs.

## 4. Execute the exact six-case example once

The final frozen example target and command are:

```bash
cargo run --locked -p rodas5p-integrators --features audit2-bateman-authority \
  --example audit2_bateman_local_six_case \
  > "$VIGILODE_BATEMAN_RUN_DIR/result_summary.json" \
  2> "$VIGILODE_BATEMAN_RUN_DIR/logs/bateman_six_case.stderr.log"
```

Do not add flags, rerun selected cases, or substitute a custom executable. Run
it at most once after preflight. Preserve a nonzero exit and any partial JSON
report as the result; do not rerun around a scientific or structural failure.

Stdout must be only pretty JSON with schema
`vigilode-audit2-bateman-local-six-case-report/v1`. Cargo diagnostics belong on
stderr. The report must retain its six-case plan, per-scenario receipts,
`all_six_executed`, `all_contracts_satisfied`, a nullable terminal failure, and
the three authority hashes.

The receipt must identify exactly these six scenarios in frozen order:

```text
same-live-context-reuse
changed-w-invalidation
nominal-independent-budget
over-strict-budget-fallback
late-preconditioner-failure
terminal-rejection
```

Require source/manifest/proof identities, cache transitions, selections,
candidate/fallback dispositions, state hashes, and complete monotone
`WorkCounters`. Missing, duplicated, reordered, or extra cases are a fail-closed
result. Do not call an observed nominal selection a predeclared PASS unless the
receipt's independent output, embedded, residual, contraction, and outer gates
all establish it under the frozen rules.

If the example exits zero, independently validate the emitted schema and
scenario invariants without rerunning the candidate:

```bash
python3 research/audit2_real_client_authority_construction_20260830/verify_local_six_case_receipt.py \
  "$VIGILODE_BATEMAN_RUN_DIR/result_summary.json" \
  > "$VIGILODE_BATEMAN_RUN_DIR/local_receipt_validation.json"
```

Treat a validator failure as the final result and preserve both files. Never
repair or rerun a scientific case around a failed receipt.

The validator independently recomputes the committed state digest, the
candidate-to-reference conservative output error and bound, exact cache
transitions, all 49 `WorkCounters` fields, and the late-failure apply ledger.
The compact report does not contain every raw embedded-error or
original-target residual vector, so their stored scalar values can be checked
for finiteness, frozen-threshold consistency, and boolean composition but not
fully recomputed from this report alone. The unkeyed state SHA-256 is an
integrity check, not host attestation against coordinated fabrication. Preserve
these limitations in any later review and do not raise the claim ceiling from
validator success alone.

## 5. Seal and return the local package

Create an execution manifest and result summary that record, without changing
the repository:

- UTC start/end, host/toolchain versions, exact head/tree, branch, and base;
- every exact command, exit, and log path;
- hashes of the authority manifest, verifier, executable source, report, and
  all retained logs;
- the six raw scenario dispositions and complete failure history;
- `local_llm_used: false`, `holdout_access: NOT_OPENED_OR_EXECUTED`, and
  `remote_writes: NONE`;
- whether each frozen invariant was proved, failed, or could not be evaluated.

Then produce a SHA-256 manifest over the package files. Do not edit, commit, or
push local results. Return the execution directory, exact source identities,
commands/exits, raw receipt, artifact hashes, and any failure.

The claim ceiling remains unchanged regardless of the local result:

```text
EXPLORATORY_NONAUTHORITATIVE_REUSABLE_PRECONDITIONER_TRANSACTIONAL_STEP_SUBSTRATE
```

A later independent review may consider a bounded Bateman-specific claim. This
prompt never authorizes speed, scalability, Krylov-basis reuse, a general or
production preconditioner, production dispatch, dense output, general event
handling, whole-integration transactionality, holdout access, merge, tag, or
release.
