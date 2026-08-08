# Phase 8 — External Decision Gate

| Candidate | Decision | Reason |
|---|---|---|
| Shared bounded controller contract | PROMOTE | Separates policy from estimator and gives common failure semantics. |
| Native RODAS embedded estimator | PROMOTE | Already method-consistent and cheap. |
| Step doubling as reference estimator | PROMOTE | Auditable and family-independent; no efficiency claim. |
| Variable-step BDF2 coefficients | PROMOTE | Required for any meaningful BDF2 adaptivity. |
| BDF/Radau step-doubling production default | HOLD | Correctness baseline first; work overhead may dominate. |
| Homotopy/SABR native adaptive fast path | HOLD | Requires false-acceptance and global-order gates. |
| PI controller default | HOLD | Must beat I-controller on holdout without destabilizing rejection patterns. |
| Cost-aware controller | HOLD | Requires reliable estimator and stable work measurements first. |
| Deferred-family adaptive implementations | HOLD | Their integrators are not yet implemented. |
