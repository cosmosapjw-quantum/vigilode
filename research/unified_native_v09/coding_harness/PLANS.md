# PLANS.md

## Closed task

TASK: Native fixed-step BDF1/BDF2 and Radau IIA1/3 anchors under a common Rust nonlinear/work contract.

OUTCOME: Four executable correctness anchors, deterministic native gates, unified registry integration, no production adaptivity or ranking claim.

## Milestones

| ID | Milestone | Status |
|---|---|---|
| P-001 | Common nonlinear work contract | Done |
| P-002 | BDF1/BDF2 anchors | Done |
| P-003 | Radau IIA1/3 anchors | Done |
| P-004 | Registry and domain-aware dispatch | Done |
| P-005 | Native gate and unified CLI integration | Done |
| P-006 | Full validation and independent review | Done |

## Next task

Build the method-independent global-error/total-cost Pareto authority, then implement variable-order BDF and adaptive Radau. Parallel SDC, Peer/W, ROK/BOROK and Leja remain explicitly sequenced behind that authority.
