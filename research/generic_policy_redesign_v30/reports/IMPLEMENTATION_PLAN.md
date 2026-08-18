# v3.0 Stage-Trajectory Geometry Discovery — Reconstruction Plan

1. Preserve the exact v2.9 R-JF/event/audit path.
2. Add read-only `rho2`, `rho3`, `rho4`, `s23`, `s34`, `kappa234` fields to the existing stage-growth safety row.
3. Derive all three rho values from already-computed level-1/2 reports; add no numerical work.
4. TDD the shape formula and a v2.9 smoke runner identity/work-parity check.
5. Commit implementation before discovery replay.
6. Replay only the already-consumed N=96/256 discovery profiles, family-sharded.
7. Analyze low-complexity witnesses with no threshold selection; keep N=192/384/2048 sealed.
