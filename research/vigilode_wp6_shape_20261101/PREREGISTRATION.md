# VigilODE WP6: research-only theory contract (retrospective, HOLD)

Date: 2026-11-01. This document is **retrospective** relative to the WP6 local 96-panel pilot and OpenEvolve/UCT search, not an ante-dated prospective registration. No performance, source, tolerance, or production setting changes are authorized.

## Immutable science inputs
- Parent: PR #163 head `a2290b3bff2bcd1a93c6d703c39fe8bc46446c59`, parent PR #160 head `cf690ad5ec723052e9eb6f71335f7cb3b43607a6`.
- Original cooperative cubic source blob `d78c5c5f6f476f2a5b1ea88628ed25daa6cfc482`.
- WP1 `SWEEP.log` SHA256 `9dae90079893170edddcf5cf8af193e53b43ab5bcf0c436cb81ce7aa0f8790b0`.
- WP1 `WP_SUMMARY.json` SHA256 `56746487ac5c5c8702b53ef0ffd47694c040ab4e0adcfea1ec25aa3b8e9ba909`.
- WP2 exact dyadic micro32/micro64 reference bits, original Rust full-IVP source. No new original IVP.
- Environment/graph: `tree_search_research_preimplementation_final_v1_0(8).zip` 236 CRC-valid files, SHA256 `bf78183ea353979fe6bf009e725fa28a7228ed55ca4593f3659eb2add26f03f3`.

## WP6 scientific question
Can a continuous degree-nine Bernstein defect envelope
`|r_i(u)| <= sum_l |b_{i,l}| C(9,l) u^l (1-u)^(9-l)`
be integrated against the positive Metzler semigroup without collapsing to a panelwise constant residual maximum?

For dyadic coefficients, existing WP5 source-specific cubic-damped Metzler comparison, exact rational uniformization with q<=1, even Taylor M and positive Poisson tail N, the answer is yes. Integrate each coefficient by the exact beta moment
`C(9,l)*l!*(9-l+j+k)!/(10+j+k)!`.
A whole-panel bounding rule remains available as a fallback; no sampling based proof.

## Frozen pilot measurements (before this registration)
4 original WP1/WP2 cohort references, micro32 first 96 panels each: 384 exactly recomputed panels with source SHA checks; 96/96 per-cohort vectorwise WP6<=WP5 original endpoint bounds. Local same-author 8/8 negative/positive tests PASS; OpenEvolve local real ProgramDatabase + UCT alternating 8+8 waves, 19 candidates, zero external LLM; no HOL WP6 theorem kernel run yet.

## Publication status / boundaries
Github scope: immutable original parents, separate Draft PR, original checker and its historical missing Git objects remain fail-closed. External origin checkout repair is separately testable but **not a claim of full global CI green**.
OFFICIAL_VIGILODE_DAG=7/23; SUCCESSOR_ADMITTED=0/12; DISTICT_AUTHOR_C9=0/1_NOT_RUN; PRODUCTION=HOLD; MERGE=false; AUTO_NEXT_NODE=false.
