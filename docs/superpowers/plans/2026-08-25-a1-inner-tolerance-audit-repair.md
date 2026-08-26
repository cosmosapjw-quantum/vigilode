# A1 Inner-Tolerance Audit-Repair Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:test-driven-development for each behavioral change and superpowers:verification-before-completion before any success claim. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Repair PR #18 so the protected GMRES policy change is explicit, fallible, externally regression-gated, and backed by a committed old/new replay receipt before merge consideration.

**Architecture:** Preserve the pre-A1 phi arithmetic; introduce explicit legacy and outer-scaled numeric-parity GMRES arms; inject typed lane configurations; add a deterministic trace digest and fixture; execute a read-only two-arm N=320 replay; expand CI to the downstream workspace; commit the authority decision and claim boundary.

**Tech Stack:** Rust/Cargo 1.94.1, serde/serde_json, SHA-256 via `rodas5p_core`, Python 3 standard library, GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-08-25-a1-inner-tolerance-audit-repair-design.md`

## Global constraints

- Canonical audit base is PR #18 head `67ec3ad77d0a88f3ff9c096b309d3a12da72b600`.
- Exact starting authority for the receipt node is
  `7952bf96bfd9fb604e87bce41bd9b918cc9b93f4`.
- PR remains draft and unmerged throughout this plan.
- Do not retune `V36_FROZEN_ZETA34_TAU`, persistence, prefix budget, or continuation budget.
- Do not modify GMRES iteration/QR algorithms, G1/G3 policies, solver equations, coefficient fixtures, or timing protocol.
- No timing, speedup, ranking, active switching, tag, release, or inference claim.
- Equal numerical tolerances must never be described as a proved equal outer-error contribution.

---

### Task 1: Record claim scope and pre-replay invalidation

**Files:**
- Create: `research/a1_inner_tolerance_audit_20260825/CLAIM_SCOPE_AND_INVALIDATION.md`

- [ ] **Step 1: Enumerate affected authorities**

Record the v3.5 result summary, consumed calibration/holdout replays, v3.7 timing validator receipt, and CLI frozen-tau contract as conditional under the new GMRES arm until replay.

- [ ] **Step 2: Separate invalidation from deletion**

Do not alter or delete historical receipts. State that they remain valid for the exact legacy code identity but cannot be transplanted to a different protected trajectory generator.

- [ ] **Step 3: Record deferred findings**

List semantic forward/backward-error equivalence, G1/G3 scope, and A2/A3 as separate blocked nodes.

### Task 2: Encode RED contracts for explicit arms and typed lane wiring

**Files:**
- Modify: `crates/rodas5p-integrators/tests/a1_inner_tolerance_parity_contracts.rs`
- Create: `crates/rodas5p-integrators/tests/a1_committed_trace_regression.rs`

- [ ] **Step 1: Replace source scanning with value-level contracts**

Require public arm and lane enums, all six lane identities, separate linear/phi tolerance accessors, fallible constructors, exact legacy values, exact parity-arm values, unchanged phi bits, and unchanged structural solver settings.

- [ ] **Step 2: Add the frozen trace contract with a sentinel digest**

Run one focused committed-arm R-JF trace and compare its canonical digest to an intentionally invalid sentinel.

- [ ] **Step 3: Verify RED in GitHub Actions**

Expected: compile failure for absent arm/lane/digest APIs, or a digest mismatch after those APIs exist. Preserve the failing run ID in the PR ledger.

### Task 3: Implement explicit arms and remove abort/source-scan wiring

**Files:**
- Modify: `crates/rodas5p-integrators/src/g4_s5b0_inner_tolerance.rs`
- Modify: `crates/rodas5p-integrators/src/g4_s5b0_regime_atlas.rs`
- Modify: `crates/rodas5p-integrators/src/lib.rs`

- [ ] **Step 1: Add the arm and lane enums**

Provide:

```text
G4S5B0LinearToleranceArm::{LegacyFixed, OuterScaledNumericParity}
G4S5B0InnerToleranceLane::{RegimeAtlas, AttemptTrace, ActualLevel1Prefix,
  ActualLevel2Prefix, StageGrowthSafety, FrozenFullEShadow}
```

- [ ] **Step 2: Separate linear and phi thresholds**

The policy stores arm, outer rtol, linear rtol/atol, and phi relative/absolute tolerances separately. Document that numeric parity is not semantic error equivalence.

- [ ] **Step 3: Make construction fallible end-to-end**

Remove `expect` from tolerance construction. Convert affected private trajectory/execution boundaries to `CoreResult` or construct once in an existing `CoreResult` caller and inject the typed configuration.

- [ ] **Step 4: Wire every lane by value**

Each of the six lanes requests its declared lane configuration. Remove all `include_str!`/source-text wiring assertions.

- [ ] **Step 5: Preserve committed authority during evidence generation**

Set the committed arm to `LegacyFixed` until Task 6 classifies the new arm.

### Task 4: Add canonical trace digest and freeze a real fixture

**Files:**
- Modify: `crates/rodas5p-integrators/src/g4_s5b0_regime_atlas.rs`
- Modify: `crates/rodas5p-integrators/tests/a1_committed_trace_regression.rs`

- [ ] **Step 1: Implement canonical serialization**

Hash deterministic load-bearing fields only; exclude wall-clock values. Include an explicit schema/version prefix and option tags.

- [ ] **Step 2: Run the sentinel fixture and capture the actual digest**

Use exact-head Actions logs. Confirm the failure is only the expected digest mismatch.

- [ ] **Step 3: Replace the sentinel and verify GREEN**

Commit the observed digest with the exact profile/family/arm provenance.

### Task 5: Add the two-arm receipt runner and aggregation workflow

**Files:**
- Modify: `crates/rodas5p-integrators/src/g4_s5b0_regime_atlas.rs`
- Modify: `crates/rodas5p-integrators/src/lib.rs`
- Modify: `crates/rodas5p-cli/src/main.rs`
- Modify: `crates/rodas5p-cli/tests/cli_contracts.rs` or add a focused CLI contract
- Create: `tools/summarize-a1-tolerance-arms.py`
- Create: `.github/workflows/a1-two-arm-receipt.yml`

- [ ] **Step 1: Add an explicit read-only API/CLI**

Accept profile, family and arm. Reject unsupported profiles for the authority receipt. Output a deterministic summary, not timing authority.

- [ ] **Step 2: Add CLI RED/GREEN coverage**

Verify accepted arm names, output schema, profile/family provenance, and non-authoritative limitations.

- [ ] **Step 3: Add twelve-cell Actions replay**

Run two arms by six families, upload JSON results, aggregate them with a standard-library Python validator, and upload final JSON/Markdown artifacts.

- [ ] **Step 4: Validate aggregate invariants**

Require complete arm/family coverage, unique keys, consistent tau, hard-gate reporting, work/event/recommendation deltas, zeta margins, and Hires discriminating-event classification.

### Task 6: Execute the replay and commit the authority decision cycle-free

The earlier v1 execution (`32906175896`) is diagnostic-only. Before freezing a
new `H_exec`, add and locally validate the v2 independent `audit_full_e_*`
channel. Reuse the enforced stage-growth full-E solver with receipt-only arm
injection, preserve nullable unknown audit safety, and reject incomplete
eligible evidence before any decision. Do not derive audit safety from runtime
recommendation shadows.

**Files:**
- Create: `research/a1_inner_tolerance_audit_20260825/A1_TWO_ARM_AUTHORITY_RECEIPT.json`
- Create: `research/a1_inner_tolerance_audit_20260825/A1_TWO_ARM_AUTHORITY_RECEIPT.md`
- Update: `research/a1_inner_tolerance_audit_20260825/CLAIM_SCOPE_AND_INVALIDATION.md`

- [ ] **Step 1: Freeze and publish the scientific execution head**

Commit all load-bearing runner, schema, aggregation, and workflow semantics as
`H_exec`, without final generated receipt files and without an arm switch.

- [ ] **Step 2: Execute the exact-head replay**

Record `H_exec` SHA/tree, tested execution merge SHA/tree, workflow run/attempt,
Rust/Cargo versions, and all deterministic artifact content hashes.

- [ ] **Step 3: Download and independently inspect artifacts**

Check family coverage, totals, event/recommendation sets, zeta margins on both sides of tau, unsafe recommendations, and the Hires positive control.

- [ ] **Step 4: Classify the parity arm**

Apply the predeclared `ADMISSIBLE_AND_DISCRIMINATING / ADMISSIBLE_BUT_NONDISCRIMINATING / NOT_ADMISSIBLE` rule without threshold retuning.

- [ ] **Step 5: Preserve the committed arm**

Retain `LegacyFixed` for every result class. An
`ADMISSIBLE_AND_DISCRIMINATING` result opens only a separate, explicitly
approved activation commit.

- [ ] **Step 6: Commit receipt and decision**

Create `H_receipt` as a descendant of `H_exec`. The receipt binds the earlier
scientific execution identity and content manifest, but never embeds its own
commit/tree or later verification run IDs. Do not rewrite historical v3.5/v3.7
files. Link them by exact identity and state whether they are preserved,
superseded for current code, or remain legacy-only.

### Task 7: Expand CI and downstream closure

**Files:**
- Modify: `.github/workflows/a1-inner-tolerance-parity.yml`

- [ ] **Step 1: Expand triggers**

Run on relevant pull requests, relevant pushes to `main`, and manual dispatch. Include docs/spec/research/CLI paths that carry the authority contract.

- [ ] **Step 2: Run load-bearing tests**

Run A1 contracts, frozen trace regression, G4/S5B0 behavioral contracts, and `frozen_full_e_shadow_cli_contracts`.

- [ ] **Step 3: Run workspace closure**

Run workspace all-target tests/compilation and workspace Clippy with warnings denied, followed by rustfmt/diff/clean-tree checks.

### Task 8: Fresh exact-head verification and stop

**Files:**
- Update PR body only after exact-head verification.

- [ ] **Step 1: Verify commit identity and changed paths**

Confirm ordinary fast-forward history, draft/open/unmerged state, exact head/tree, and no undeclared scientific mutation.

- [ ] **Step 2: Verify all workflows**

Require A1 workspace CI, E4 fresh-clone CI, and receipt validation success at
the exact `H_receipt`. Record `H_receipt` SHA/tree and these post-receipt run IDs
externally; do not amend the receipt to insert them. Confirm the earlier
two-arm execution artifact corresponds byte-for-byte to the committed receipt.

- [ ] **Step 3: Perform fresh-context review**

Review P0/P1/P2/P3 against the static audit. No self-certification based only on the implementation agent's tests.

- [ ] **Step 4: Stop at merge gate**

Do not mark ready or merge. Report the authority decision, remaining deferred nodes, and the single next action requiring user approval.
