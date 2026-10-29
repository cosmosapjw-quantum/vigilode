# Integrated VigilODE research — 2026-10-28 KST

## Objective and boundary

Synthesize this conversation's PR90--93 into one constructive solver design,
extend the mathematics where it connects existing paths, and provide an executable
handoff for a separate PR that validates the complete cumulative implementation.
Selected physmath-research-loop 1.3.0 stages: R3 (align claims/assumptions),
R5 (constructive derivation), R9 (formal results and implementation connection),
with supporting source retrieval/checks and the requested C4/C10 handoff design.
This is not a claim that unexecuted native validation has passed.

## Source anchor

- Source PR93: 384dba091a3d9e1aac0ecddec264958298239253.
- PR92: 7eb2dd0ffda39afb2871d76700a720986ff09fe0.
- PR91: 206298b4068d923bb47c7160b111b7439098660c.
- PR90: 68a28357d960c4c2743c299770f0a33ef63245b2.
- Pre-conversation baseline: 3064d453091af1dec7f99f0c8afabd98b447ea34.
- Observed main: 8d0c79184e09efb5bdadc24a6315c60a71a44264.
- Remote survey at start found no PR after #93. All four remain open drafts/PRs;
  exact remote metadata, ancestry and CI will be recorded rather than assumed.

## Scientific invariants

Preserve original target, full-state physical error and original residual;
derivative-free first, selective explicitly supplied JVP/AD only; distinguish
matrix-free from derivative-free. Derive numerical constants, preserve old
negative results, failed CI and existing correction budgets. Keep expensive
native validation in the requested later independent PR. Checkpoints and
supporting exact computations must not become a new solver-performance claim.

## Progress

PR93 checkout was restored from the remote after scratch maintenance. The four
conversation PRs and current main were inspected; exact ancestry is recorded.
Three parallel agents completed error, algebra and cumulative-validation lanes.
One complementary mathematical review is closed; its four local corrections
were made before source registration. No recursive review was started.

Registered mathematical source:
`af7f3e9f55d652554042d2386bf07abb7d857d75`.
Both exact support programs ran once with unchanged source inputs and exit 0:
six error groups, nine compiler fixtures and refinement/refusal controls PASS.
Raw JSON and receipts are in evidence; L-0177/L-0178 append the admitted scope.
Native solver tests, benchmarks and profiles for this new theory remain NOT_RUN.

REPORT_KO.md, EXECUTION_CONTRACT.md and the two THEORY.md files supply the
integrated constructions. The REVTeX report is in report/. The independent-PR
handoff, safe preparation helper and INDEPENDENT_VALIDATION.json preserve the
old 23-node/34-edge/eight-suite plan and its correction counts. All 73 changed
crate/workflow paths map to semantic gates; ten cross-PR interactions are planned.

PR93's actual CI failures are preserved: 23 distinct Clippy sites and skipped
offline follow-through; full authority identity failures despite full-depth
checkout. No Q/N native gate has been marked passed by this research package.

## Next action

Use the final delivered full commit SHA from the PR/delivery to materialize a
fresh independent validation branch targeting main. Follow
INDEPENDENT_PR_HANDOFF_KO.md. Replay historical source-sensitive artifacts in
disposable worktrees; run cumulative native/physical interactions at the actual
final implementation tree. Preserve every old failure and correction budget.
The worktree helper is dry-run by default and never performs solver validation.
