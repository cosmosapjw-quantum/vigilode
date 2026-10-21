# Native adapter checkpoint (2026-10-18)

Base HEAD: `26f0b3a187ab41f7d68922491cc489b268c0caa1`. All adapter changes are untracked in the shared worktree; no commit or production solver change.

Implemented true native RODAS fast/small/banded/matrix-free/Gmres-into/LS-workspace/routed-dense, fixed BDF1/2 and Radau1/3, EXPRB2/43, adaptive PEXPRB54S4, and source-fixture SABR. Diffsol 0.16.1 isolated autobin implements BDF/TR-BDF2/ESDIRK34/Tsit45. Both accept timing/allocation requests and model probes. DAE solve is a typed adapter capability gap; DAE original RHS probe works.

First failed native SABR source step under mismatched earlier case tolerance is preserved in `native-sabr_sabr.smoke.json`; case fixture was corrected to exact source Smoke tolerance/config by case owner, with successful subsequent smoke. First Diffsol compile failure (`VectorHost` trait import) is preserved in `cargo_check_diffsol_01.log`.

`probe_parity_all_01.log` and `probe_parity_summary.json` report 34/34 model probe statuses OK against Python, worst normalized residual 2.1334e-16. This is model parity evidence, not admission. Prior targeted builds/checks and two native unit tests succeeded in numbered logs. Separate allocation-mode process positive control was observed at one allocation; entire-solve allocations are measured, no zero-hot-allocation claim.

Next bounded actions: expose existing controller and LU policies, inspect public routed/batch/parallel APIs for actual route eligibility, compile selected component tests without executing historical campaigns, and write build artifact manifest with source/test/binary identities. No new numerical kernel or scientific policy will be created.

Final adapter extension: `solver_config.controller` exposes Integral, PI, Predictive, PredictiveCapped, PredictiveCapped2 on adaptive routes. `solver_config.lu_policy` exposes Legacy, ColumnExtents, ColumnExtentsAbove64 on the dense fast route and records the actual LU selected by the library. Faer is an automatic library density choice and has no forced public option. `native-rodas5p-routed-matrix-free` uses the public JVP-only clone and structure router. `solver_config.threads` 1/2/4/8 instantiates a local Rayon execution context for PEXPRB54S4. All five controller fixtures and four thread fixtures returned OK on scalar decay. Unknown or inapplicable config fields return typed failure.

The isolated Diffsol 0.16.1 release build completed offline/locked with exit 0 in `cargo_build_diffsol_release_01.log`; a release BDF scalar request returned OK and 101 outputs in `diffsol_release_scalar_smoke_02.json`. The first attempted release smoke used a native solver ID and returned expected `UNSUPPORTED_CAPABILITY`; preserved in `diffsol_release_scalar_smoke.json`.

`build_artifacts.json` records 30 hashed binaries (native debug adapter, Diffsol debug and release adapters, 27 named component contract test binaries), exact source/test function provenance, and ten successful build/compile command logs. The 27 component contract binaries are compile-only, to run under the frozen execution commit. No historical campaign or holdout run was performed. Small ensemble batching needs an ensemble request and remains component-only rather than being relabelled as a single-case solve.
