# D2 pre-RHS stage-domain consumer

Read REPORT_KO.md, THEORY.md, S2_RESULT.json, DAG.json, FINAL_DELIVERY.json and HANDOFF_KO.md. New code is in source/stage_safe.rs, with bounded pending-attempt abort and proposal cap in source/admission.rs. No native production/actual BASS integration is claimed.

Exact scalar-stage witness: at h*lambda=4, stage2 is negative despite a positive endpoint. Exact Bernstein certificate proves all8stage and endpoint multipliers in[0,1] on[0,2] for scalar linear decay. Runtime stage checks remain authoritative. Nonlinear frozen-rate cap is only a proposal heuristic.

Actual15newtests+31regressions,12trajectories; forbidden RHS calls507 in diagnostic baseline and0 in both guarded paths. Guard+cap reduces work from1032attempt/5323RHS to602attempt/4816RHS, not a measured wall-time/whole-solver speedup. Source identity and full raw proofs are preserved.

[Full S3 companion on Drive](https://drive.google.com/file/d/1qY8byW4TTBgJM1wmRjidmq7RKKBNfLFP/view?usp=drivesdk). Dropbox/stage backup IDs in FINAL_DELIVERY.json. Original archived DAG is prepublication; final Git record is additive. No sharing changes, merge or default promotion. Scoped S0–S3 complete; global percentage null, production HOLD.
