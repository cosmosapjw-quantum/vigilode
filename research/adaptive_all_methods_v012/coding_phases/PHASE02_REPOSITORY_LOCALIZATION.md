# Phase 2 — Repository Localization

| Responsibility | Files/symbols |
|---|---|
| Existing Sequential/SABR adaptivity | `integrate.rs::{integrate_adaptive, integrate_adaptive_observed}` |
| Requested-output transaction | `output.rs::{OutputSchedule, OutputCollector}` |
| Embedded RODAS error | `sequential.rs::finish_step`, `StepResult::error_norm` |
| SABR certificate/fallback | `sabr.rs::sabr_step` |
| Homotopy candidate/fallback | `homotopy.rs::homotopy_step` |
| BDF history and fixed equations | `bdf.rs::{BdfHistory,bdf_step}` |
| Radau coupled stage solve | `radau.rs::{radau_step,RadauConfig}` |
| Work ledger | `rodas5p-core/src/work.rs::WorkCounters` |
| External global-error authority | `rodas5p-fair-ab/src/global_error.rs` |
| CLI/report serialization | `rodas5p-cli/src/main.rs` |

The narrow modification boundary is one new controller module, additive adaptive APIs in `bdf.rs`,
`radau.rs`, and `integrate.rs`, and one new fair-ab adaptive screen.  Fixed kernels remain protected.
