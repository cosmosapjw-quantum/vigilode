# WP9B hostile self-review and preserved failures

## Directly verified in this run
- Original frozen RHS and Rust case fixtures preserved by exact Git blob SHA-1, rechecked before and after the physical experiment.
- Source parameters parsed from actual registered Rust fixture, not from manually invented model coefficients.
- Radau/DOP853 true perturbed IVPs over theta=0,+0.03,-0.03 and 65 sampled times; raw trajectories, physical evidence and search traces written as JSONL.
- Genuine offline OpenEvolve ProgramDatabase.add and ProgramDatabase.sample called. No LLM calls. A separate uniform-prior OpenEvolve/MCTS study uses the same source and a shared physical-measurement cache.
- 16 alternating waves per arm, true before/after prior and mutation records; no acceptance gate or production default modifications.
- First-variational sensitivity reproduction at t=.1 matches frozen WP9 source conditional numerical evidence.
- Full forced weighted L1 convolution inequality derived for parameter changes and numerically sampled on physical source outputs.
- Normal, negative and null controls checked; original asymmetric_2_refined is identical physics.

## Found and corrected after initial trial
The first 114-solve exploratory run keyed the refined source separately, allowing a duplicated physical model to contribute posterior evidence twice. This is scientifically invalid. The uncorrected trial has a preserved terminal receipt in the WP9B archive, not claimed as final science. The final implementation canonicalizes both source variants to one original model, uses physical-keyed numerical cache and posterior deduplication, then reruns all 32 selection waves afresh. Final physical IVPs: 14 independent training source-parameter cases plus one holdout; both independent integrators on theta=0,+h,-h. A real ProgramDatabase may prune prior nodes according to its island/archive strategy: 16 successful insertions do not imply retaining exactly 17 nodes at the end. This was corrected from an invalid assertion and recorded in raw trace.

## Failures and limits retained
1. No real REI Bianchi IGM owner-IVP has been run by this unit. The measured physics belongs to frozen WP1-WP9 cooperative cubic source only.
2. The adaptive policy and uniform control select the same holdout champion. Their in-sample reward sums cannot demonstrate out-of-sample improvement. Furthermore the refined control and its original are equivalent physical sources.
3. This PUCT is one root expansion per wave, not a full rollout MCTS, and fitness heuristic is source-specific. It is not a learned neural prior, and robustness across random seeds is untested.
4. Numerical inequality witnesses use finite-grid Simpson convolution, not guaranteed outward interval integration of the forcing. Analytical contraction theorem holds for stated Metzler/positive-sink assumptions, but its numerical comparison is not an interval proof.
5. No HOL full proof, distinct-author C9, true same-runner BE-v-RODAS5P matched achieved-error benchmark, speed superiority, observational likelihood, or cosmological claim.
6. Original history bundle SHA256 9512aa3d596315ce952482214b4ae26edb47a50b3c857301485d228efa1d2862 independently located, verified and historically anchored on GitHub without changing original commit SHA. Other postimport local/CI-only objects remain absent and strict global authority CI may still fail. No fake allowlists or replacements.
7. A repaired E4 formatting-only commit covers exactly the existing WP3 original native RODAS5P example and old regression test. Formatting check local PASS, actual remote fresh-clone job must be read from new PR before claiming global success.

## Status and stop condition
The present scope ends with saved raw physics, reproducible scripts, source locks, hostile controls, origin-history restoration and a draft-only PR. Do not inflate the official DAG count: 7/23 and admitted successor 0/12; production HOLD; no merge, default change or tolerance relaxation.
