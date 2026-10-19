# Banded exact-reuse improvement loop, 2026-10-20 KST

Parent: b56bb19ce85bc28a021138ee29689701a81bc4f0 on claude/nice-dijkstra-208xq0. Historical comparison PR71 remains unchanged. That parent is15commits ahead of the measured numerical source a21d573. Already-landed triage fixes and the new Diffsol comparator must not be overwritten.

Selected bounded theory/coding work: distinguish exact shifted-matrix reuse from stale-Jacobian approximations; repair failed-base retry validity and standalone band-solve false success; add opt-in bandwidth2 specialization and exact factor cache; test actual production code and measure same-output cost; preserve failures and publish. No default kernel change, no production router, no unvalidated Jacobian freezing, no promise to close the full DVODE spatial performance gap.

## Claims and invariants fixed before measurement

W=I/(gamma*h)-J. Equal J alone is insufficient when the represented shift changes. Cache reuse requires equality of every finite binary64 Jacobian entry and shift bit pattern; failed factor construction invalidates the key. Factor arrays must be finite before recording. No hash-only equality. This skips LU computation only: each new accepted state still evaluates its own Jacobian, RHS and partial-time derivative.

The bandwidth2 kernel performs exactly the same floating-point operations, pivot tie-breaking and interchange order as Slices with l=u=2; other widths explicitly use Slices. Tests compare factors, pivots and solve/output bits, not merely nearby solutions. Existing Indexed/Slices defaults remain selected by their existing entrypoints.

New correctness targets: no successful standalone solve containing NaN/Inf, dimension arithmetic overflow returns a typed error rather than a panic, and a failed Jacobian/RHS/base derivative callback must not make the next retry reuse a partially prepared state. Source inspection found that lu_chosen was incorrectly serving as the base-validity flag; baseline behavioral tests precede the fix.

Performance study: fresh same-host measurements only, legacy Slices vs FixedWidth2 vs FixedWidth2ExactCache, positive nonlinear Brusselator and negative/holdout sizes, plus exact linear diffusion controls with repeated dyadic steps. Input rtol={1e-4,1e-6,1e-8}, h0 scales={0.5,1,2}; no timing-based tolerance selection. Only bit-identical complete finite outputs at the same tolerance are admitted for an old/new speed ratio. Correctness/reference computations and builds do not overlap timing. Twelve balanced blocks, discarded warmup, independent process as statistical unit; retain per-repeat times and canonical full-state hashes. Shared-host timing stays diagnostic. Exact cache may lose on changing nonlinear Jacobians and that outcome must be reported, not hidden.

## Evidence policy

New branch research/banded-exact-reuse-20261020. All writes confined to this authorized improvement. Final changed source is committed before its benchmark build and its SHA is recorded in each measurement. Historical source/measurement identities are not relabelled. Preserve RED failures, full workspace test results (including unrelated failures), scoped parity and numerical experiments separately. No native subagent tool exists in this session: native_subagent NOT_RUN; serial execution and separately implemented numerical checks are not independent-personnel review.

The sandbox can execute the uploaded Rust compiler, but direct GitHub clone failed at DNS resolution. Connector reads/writes and bounded GitHub Actions are used for the actual repository build. A local dependency-free exact-key test recorded2expected RED failures and8GREEN passes before integration. No remote desktop quota workaround or external paid compute is requested.
