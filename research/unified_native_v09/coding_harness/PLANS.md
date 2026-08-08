# PLANS.md

## Current task

TASK: Native BDF1/BDF2 and Radau IIA1/3 anchors under a common Rust nonlinear/work contract.

OUTCOME: Executable candidate registry entries and global-order/stiff gates; no production adaptivity claim.

IN_SCOPE: work counters, dense Newton, fixed BDF history/startup, fixed Radau stages, registry/gate tests, reports.

OUT_OF_SCOPE: variable-order BDF, embedded Radau controller, SDC/Peer/ROK/BOROK/Leja implementation, singular-mass DAE.

## Milestones

| ID | Milestone | Files | Validation | Status |
|---|---|---|---|---|
| P-001 | Common nonlinear work contract | core `work.rs`, integrator `nonlinear.rs` | targeted Newton tests | Planned |
| P-002 | BDF1/BDF2 anchors | `bdf.rs` | order/startup/stiff/mass tests | Planned |
| P-003 | Radau IIA1/3 anchors | `radau.rs` | tableau/order/stiff/mass tests | Planned |
| P-004 | Registry and integration dispatch | `candidates.rs`, `integrate.rs` | catalog and trajectory tests | Planned |
| P-005 | Unified native gate | `native_gates.rs`, CLI/report | deterministic report | Planned |
| P-006 | Full validation and independent review | harness/reports | fmt/clippy/test/build/review | Planned |

Status: `Planned / In Progress / Done / Blocked / Cancelled`
