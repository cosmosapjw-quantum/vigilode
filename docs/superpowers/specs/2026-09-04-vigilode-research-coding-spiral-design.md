# VigilODE Research–Coding Spiral Design

Date: 2026-09-04
Status: approved design, durable planning checkpoint
Planning branch: `planning/vigilode-research-coding-spiral-20260904`
Scientific parent: Draft PR #42 head `b3e8165c8dc3b5016702821d280daea1a3f1feb7`

## 1. Purpose

VigilODE development shall alternate between mathematical/numerical research and implementation rather than treating theory, code, validation, and handoff as one-way phases.

The canonical cycle is

`V0 authority bind -> R research node -> C coding node -> J integrated adjudication -> D durable checkpoint -> next R/C node`.

The cycle is intended to convert mathematical obligations into executable contracts and to feed runtime counterexamples, residuals, constants, and failure receipts back into the next research node.

## 2. Current authority boundary

- Production/default reference remains repository `main`.
- Current scientific continuation authority is the open Draft PR #42 stack, not `main`.
- PR #42 is a control-only successor to PR #41 `STOP_INVALID`; it is not itself evidence that the scientific repair has executed.
- PR #42 currently permits only the observed F01/F03/F04 formal-scope repairs and the observed software-contract repairs for `max_arnoldi`, receipt round-trip, and directed binary64 rounding.
- Candidate execution, real-client execution, holdout access, claim promotion, merge, tag, and release remain outside PR #42.

No planning artifact created by this design changes that authority boundary.

## 3. Global invariants

### 3.1 Progress-first / anti-meta-loop

Each phase has one major objective and one verification bundle. A phase must produce at least one substantive delta: theorem/proof, executable implementation, numerical measurement, counterexample, or scientific decision.

Two consecutive process-only cycles without substantive progress terminate the planning branch with `STALL_PROCESS_ACCRETION`; no further governance layer is added.

Only observed failure classes may become mandatory guards. Review-of-review, recursive audit, and standalone gate proliferation are forbidden.

### 3.2 Durable continuation

Every new node resumes from `LATEST_VALID_DURABLE_CHECKPOINT`.

Before any mutation, bind:

- repository / branch / HEAD / tree;
- parent checkpoint and immutable predecessor receipts;
- worktree state and allowed dirty paths;
- plan/contract hash;
- claim ceiling.

Do not `reset`, `clean`, `stash`, `rebase`, overwrite a predecessor run, or checkout across a preserved dirty worktree. Non-overlap replay requires byte identity; overlaps require explicit three-way semantic merge with base/before/after identities.

### 3.3 Typed identity

Identity classes are non-substitutable:

- `BYTE`: exact SHA-256 equality;
- `CONTENT`: canonicalized structural equality with named normalizer/version;
- `NUMERICAL`: bitwise binary64, directed enclosure, or fixed norm/tolerance comparator;
- `SEMANTIC`: quantified obligations and invariant/policy conformance.

Byte equality does not prove numerical or semantic equivalence. Compiler/test exit zero does not prove quantified theorem coverage.

### 3.4 Claim discipline

Claims fail closed while development may continue. Result validity, provenance validity, packaging validity, and release authority remain separate.

A failed theorem, test, numerical gate, or independent review is preserved as a typed failure receipt rather than rewritten as success.

## 4. Node architecture

## V0 — Authority / resume node

Input: previous durable checkpoint plus exact active Git authority.

Output:

- exact source identity;
- immutable predecessor references;
- typed identity comparator table;
- allowed/forbidden mutation surface;
- one selected research or coding objective.

No scientific implementation begins until V0 is complete.

## R — Mathematical / numerical research node

Each R node answers one load-bearing question and ends in a computational contract.

Required structure:

1. definitions and assumptions;
2. theorem/claim and quantified domain;
3. derivation or proof obligation;
4. counterexamples / kill tests;
5. computational form;
6. observables or telemetry required from code;
7. acceptance comparator;
8. unresolved assumptions/constants;
9. exactly one next coding task.

Authority roles are explicit. Formal theorem provers may serve as universal proof authority where declared; CAS tools are exact cross-checks unless the node explicitly proves otherwise.

## C — Coding research node

A C node implements only the contract exported by the immediately relevant R node or an already-observed software defect.

Workflow:

1. RED test or executable failing contract;
2. smallest repair;
3. focused GREEN suite;
4. feature/default isolation and regression checks;
5. runtime telemetry and failure-preservation checks.

Unrelated refactoring and performance-first optimization are deferred.

## J — Integrated adjudication node

J connects

`definition -> theorem -> computational form -> code path -> runtime evidence`.

It verifies that tests exercise the claimed path and that numerical evidence uses the declared comparator.

For the current PR #42 closeout, exactly one fresh integrated review is allowed. It uses the formal-scope and software-contract lenses. P0/P1 produces `STOP_INVALID` with no post-review repair in that node.

## D — Durable checkpoint node

A durable checkpoint records at minimum:

- node id and parent checkpoint;
- Git HEAD/tree;
- theorem/claim IDs;
- implementation delta;
- tests/executions actually run;
- failure tombstones;
- typed identity evidence;
- result/provenance/packaging validity;
- progress delta;
- terminal disposition;
- exactly one next executable action.

Preferred terminal classes for new spiral nodes are `COMPLETE`, `STOP_INVALID`, `BLOCKED_EXTERNAL`, and `DEFERRED`, while inherited PR #42 terminal vocabulary is preserved unchanged inside that closeout.

## 5. Milestone M0 — Close PR #42 without scope expansion

M0 is not a new theory project. It closes the exact predecessor defects already admitted by PR #42.

### R01 formal scope

F01, arbitrary finite `n >= 1`, arbitrary strict-lower `T` indexed by `Fin n`:

- `T^n = 0`;
- with `S = sum_{k=0}^{n-1} T^k`, prove `(I-T)S=I` and `S(I-T)=I`.

F03, arbitrary two-sided inverses `W,V`, arbitrary `b,x`, `rho=b-Wx`, `kappa>=0`, and `||Vv|| <= kappa ||v||` for every `v`:

- `Vb = x + V rho`;
- `||Vb-x|| <= kappa ||rho||`;
- `||Vb|| <= ||x|| + kappa ||rho||`.

The RHS `b` and residual `rho` must never be conflated.

F04, arbitrary nonnegative strict-lower `A`, `q>=0`, admissible error `d`, and nonnegative endpoint/estimator weights:

`z_i = q_i + sum_{j<i} A_ij z_j`

with required conclusions:

- `z>=0`;
- finite-Neumann representation;
- componentwise domination `d_i <= z_i`;
- nonnegative weighted contamination bounds.

Lean/mathlib and Rocq remain universal authorities for F01/F03/F04. Wolfram/Sage/Singular remain role-limited exact cross-checks as already frozen by PR #42.

### C01 software scope

Close only:

1. `max_arnoldi` as `TOTAL_ARNOLDI_VECTOR_CAP_PER_COMPLETED_GMRES_LINEAR_SOLVE_TRACE_ROW`, independent of iteration limit and restart-cycle length;
2. concrete `serde_json` receipt serialize -> deserialize same type -> structural equality -> canonical reserialization;
3. correctly upward-rounded nonnegative binary64 add/multiply, preserving exact zero and exact representable values, with overflow -> `+inf` -> typed nonfinite rejection and no decision.

### J01 / D01

Run the exact integrated bundle required by PR #42. Freeze one snapshot, run exactly one integrated review, preserve failure if P0/P1 occurs, and publish only compact allowed evidence to the PR #42 branch.

M0 exit is either inherited `REPAIR_CLOSEOUT_VERIFIED` or one of PR #42's fail-closed terminal dispositions. No candidate execution is part of M0.

## 6. Milestone M1 — First real-client scientific execution

M1 opens only if M0 is verified. It must not be replaced by another planning/audit node.

Primary target: the already-constructed Bateman client lane.

Research questions:

- obtain or validate per-stage solve-error bounds such as `q_i >= ||W^{-1} r_i||` under the declared norm/certificate mechanism;
- propagate stage contamination to endpoint and embedded-estimator bounds;
- connect the bound to runtime accept/reject semantics without fitting thresholds to observed outcomes.

Runtime decision target:

- certified accept when the declared estimator plus contamination enclosure is within budget;
- certified reject when the lower side of the enclosure exceeds budget;
- otherwise typed inconclusive.

Bateman-specific algebraic properties may be used as oracle/kill tests only on the Bateman client and must not be promoted to general GMRES claims.

## 7. Later research–coding nodes

The default order after real-client evidence is:

1. stage-residual -> state/embedded contamination runtime certificate;
2. fixed-step/order-preservation experiments and theory;
3. approximate/stale Jacobian effects and Rosenbrock/W/Krylov order conditions;
4. nonnormal GMRES, restart, field-of-values/residual-history diagnostics, and fallback policy;
5. whole-step transactional retry / event / dense-output semantics;
6. additional real clients and observable-budget validation;
7. only then same-physics / same-tolerance / same-output performance study and production polyalgorithm selection.

The order may change only when newly observed evidence creates a higher-priority blocker.

## 8. Literature anchors

The later approximate-Jacobian/Krylov-order lane should explicitly compare against Rosenbrock-Krylov order theory, especially Tranquilli & Sandu, *SIAM Journal on Scientific Computing* 36 (2014), DOI `10.1137/130923336`.

Matrix-free finite-difference/Jacobian approximation work and verified residual/error-bound literature should be used as background and adversarial comparison, not silently imported as proof of VigilODE's own contracts.

## 9. Tool-role contract

- GitHub: exact source/PR/CI/provenance authority and mutation surface.
- Local Codex: execution requiring the preserved local worktree, full Rust/formal toolchains, and source-bound candidate-free closeout.
- Lean/mathlib + Rocq: current universal formal authority where PR #42 declares them.
- Wolfram: exact symbolic cross-check only in the current formal scope; availability failure is `BLOCKED_EXTERNAL`, not a theorem failure.
- SciSpace: literature discovery/triage; papers are background until their claims are independently mapped to a current obligation.
- Research harness and coding harness: process guidance, not repository scientific evidence.

The repository currently records canonical harness hashes `9adde688f8020e7feb2c1c0304b3204dbe70dd01e2d87e64a5c4eb357c019934` and `6e67e999a0c19f6ed9de7c339067cc11691d5cf5cb662a11756d8fc393c849b4`. A fresh execution must verify the actual supplied archives against the required authority before treating them as byte-identical; this design does not infer identity from filenames.

## 10. Mutation strategy

This planning document is deliberately not committed into PR #42 because #42 has a narrow publication whitelist and a control-only scope. It lives on a separate planning branch rooted exactly at the inspected PR #42 head.

No planning PR is required. The next code/formal mutations remain on the exact successor authority required by PR #42 or in a local source-bound worktree according to its execution contract.

This avoids converting architecture documentation into another process-only stacked scientific PR.

## 11. Acceptance criteria for the spiral design

The design is successful when:

- every implementation claim has a traceable research/code/runtime chain;
- every R node emits an actionable computational contract;
- every C node begins with a failing executable contract and ends with focused evidence;
- runtime counterexamples can reopen a bounded research question without reopening completed unrelated work;
- failures are durable and typed;
- identity comparisons are explicit and non-substitutable;
- no two process-only cycles occur in succession;
- performance work does not start before scientific/semantic equivalence is established;
- M0 leads directly to M1 real-client execution when verified.

## 12. Immediate next transition

After this design document is reviewed, the next planning artifact is a detailed execution plan for exactly:

`V0/M0 bind -> R01 -> C01 -> J01 -> D01 -> M1 Bateman execution preflight`.

That implementation plan must identify exact files/functions/tests/tool invocations, RED/green criteria, formal obligations, durable receipts, delegation boundaries, and stop conditions. It must not broaden PR #42's scope or authorize candidate execution before D01 verifies M0.
