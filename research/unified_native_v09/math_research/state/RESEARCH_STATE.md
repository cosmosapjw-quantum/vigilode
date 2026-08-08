# RESEARCH_STATE

PROJECT: Unified Stiff Integrator Expansion v0.9
VERSION: 0.2
CURRENT_PHASE: closed-math-research
LAST_UPDATED: 2026-08-08

## Primary question

PRIMARY_RQ: Build a legitimate common comparison and certification architecture, then add native BDF/Radau anchors before broader solver families.

## Hypothesis status

ACTIVE_HYPOTHESES: H-001 two-level certificate; H-002 stage-recursive output bound

PROMOTED: H-001 design; H-003 BDF anchors; H-004 Radau anchors; H-005 external-error Pareto

ON_HOLD: variable-order BDF; SDC; Peer/W; ROK/BOROK; Leja; DAE lane

REJECTED: universal one-correction certificate; equal-tolerance authoritative ranking

## Blockers

KEY_BLOCKERS: native nonlinear contract; derivative-bound source for rigorous certificate; adaptive/controller parity

MISSING_EVIDENCE: target derivative bounds; large sparse traces; index-1 DAE residual suite

## Gate

CURRENT_COMPLETION_BAR: math/research gate passed for bounded BDF/Radau anchor coding cycle

NEXT_GATE: coding harness Phase 0–10

NEXT_MINIMAL_ACTION: implement common work counters and dense Newton contract test-first
