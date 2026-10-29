# R4 adversarial re-audit — preregistered execution contract

Date: 2026-10-01 UTC. Audited production source: `1c54194123ee6abc6daa512e8574922f510b4e2c`, tree `e50f33fd2b336ed146b25f68e38bb29b19c44d0b`.
Publication branch: existing `claude/jolly-wozniak-7wl15h-wu23-reaudit-r3`. No new branch, production edits, or force push.
Previous audit publication: `6598abd0db790162ab26f50bc13943f24a0c7e4a`; previous audited source: `cc2cd041737e7ff543624d1b59893a3b4397369f`.

## Registration and evidence boundary

This file, the four lane contracts, native probe source, exact oracles, runtime plan and source/input hashes will be committed and remotely verified BEFORE scientific execution. Source inspection, reading inherited campaign outputs, Rust installation, dependency metadata/lock generation and hello-world preflight occurred before registration; no current audit numerical experiment has run. The commit that first contains this file is the preregistration identity and is recorded in subsequent execution receipts. An unexecuted preregistration commit is intentionally incomplete as a research node until the append-only numeric ledger/result commit follows.

This is a source-informed adversarial audit, not a blinded efficacy trial. Fixed witness/control inputs are disclosed, not held-out observations. There is no untouched statistical performance holdout and no speed promotion claim. Any new scientific input expansion requires a separate committed addendum before its execution; otherwise it must be marked exploratory. A single bounded harness/build correction is allowed per lane, preserving first failure, patch and reason; tolerances and success gates cannot be relaxed after results.

## Parallel math and coding loops

Workflow: the user-selected GPT-6 Astra v4 mathematics and coding harnesses, with an independent decision reviewer; this is a workflow selection, not a claim about runtime model identity. Owner proposes a contract, exact/independent oracle and standalone candidate; numerical evidence is then challenged by an independent reviewer. No recursive review campaign.

- Time/output: `time/CONTRACT.md`, represented intervals, BDF scale covariance, unequal Radau1 half-step estimator and a ratio-aware candidate.
- Polynomial/arithmetic: `polynomial/CONTRACT.md`, generated phi transformation bounds, Chebyshev/Laguerre certificates and outward arithmetic candidates. At most 100 rows.
- Homotopy: `homotopy/CONTRACT.md`, inverse witness trust boundary, stage-count generalization, finite nilpotent doubling and diagonal component factorization. No throughput inference from operation counts.
- Statistics: `statistics/CONTRACT.md`, documented coverage hold versus executable decisions, complete session receipts, inherited campaign replay, exact simultaneous order-statistic intervals with an explicit candidate estimand.
- Native regression: `evidence/runtime/FROZEN_PLAN.json`, all-target/all-feature offline build capped at 600 seconds, ordered native run budget 300 seconds. Record every test as PASS, FAIL, ignored, timed out or not run. No aggregate full-suite PASS unless all discovered nonignored tests actually finish.

Acceptance and rejection gates are those in the frozen lane contracts. A zero process exit is execution success only; observed invariant violations make the relevant scientific gate FAIL. Analytical identity, finite exact check, production native behavior, numerical oracle evidence, and environmental execution status remain separate fields. Proposed candidates may pass finite checks without production approval. No broad performance or universal numerical correctness statement follows.

## Inputs, commands, and portability

`evidence/source/SOURCE_MANIFEST.json` binds the production source bytes. `INPUTS_SHA256.json` binds the registered scripts and contracts. The source commit is an immutable baseline even if HEAD later moves through report-only publication commits.

Commands in each lane contract are executed from a workspace with `vigilode/`, `r4/`, and `runtime_r4/` siblings after sourcing `runtime_r4/env.sh`. Cargo calls share `runtime_r4/cargo-build.lock`, single build job and no incremental compilation. For reproduction, copy the published lane directories into `<work>/r4`; point `<work>/vigilode` to the specified audited checkout. Time's runner mechanically binds its temporary manifest to `--repo`; other manifests use the sibling repository path. Only dependency location rebinding is a portability operation; preserve the frozen original and log changes. Attachments provide Rust 1.94.1 and offline vendor packages. Ordinary installations with the locked dependencies may also reproduce, with environment differences recorded.

The runtime command is:
`python3 r4/evidence/runtime/run_bounded_suite.py --repo vigilode --target-dir runtime_r4/target --evidence-dir r4/evidence/runtime --build-lock runtime_r4/cargo-build.lock --plan r4/evidence/runtime/FROZEN_PLAN.json --prereg-commit <this-file-first-commit>`.

Lane owners retain raw stdout/stderr, process status, input bits where relevant and numerical oracle outputs. Initial setup/build errors remain evidence, not scientific failure. Published historical POLY03/HOM06 and coverage data are inspected/replayed and labeled inherited; no fresh speed timing campaign is planned.

## Independent decision and stopping rule

After lane results, one independent reviewer replays representative failures through the frozen native binaries and recomputes the exact binomial-tail theorem using separate standard-library integer arithmetic. Review covers scope/authority, source identity and the integrated report once. No candidate tuning or new timing study by the reviewer. A blocked claim is retained as blocked/inconclusive, not repaired by a looser gate. Stop after bounded native runs, exact checks, and this one review.

## Deliverables and publication

Detailed Korean REVIEW_KO.md; machine-readable findings, source closure matrix, claim ledger, dependency-ordered development tasks with acceptance/kill criteria; raw results and reproducible inputs; runtime coverage and limitations; append-only research/LEDGER.jsonl row(s) covering numerical results. Hash and validator results are published with the report on the same existing branch. No new exemption or authority-reference bypass is permitted. Source or statistical findings do not authorize production code changes in this task.

## Results

Not executed at registration. Results are published separately after the preregistration commit; the registered contract and input bytes remain immutable.
