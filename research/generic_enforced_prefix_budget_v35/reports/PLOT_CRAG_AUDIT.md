# v3.5 Plot-Driven CRAG Audit

## C — Correctness
The all-event cap plot shows two exhausted events exactly on the 80-JVP boundary and no point above the cap. The N=320 zeta/error plot shows the only audit-E-inadmissible event above the frozen zeta threshold, so it is abstained.

## R — Retrieval
The project uses Krylov evaluations of matrix-function actions in the exponential lane. Residual-based stopping/restarting is established practice for Krylov evaluation of phi-functions, which supports enforcing a stopping condition before additional operator applications. This literature does not determine the project-specific numerical cap of 80 or cumulative fraction 0.25; those remain frozen empirical policy values.

## A — Augmented checks
The transactional semantics were replayed on N=96, 192, 256, and the consumed N=384 profile, then tested on a new N=320 profile. No cap violation appeared. The cumulative observed prefix fraction remained below about five percent on these profiles, far below the frozen 25% ceiling; the absolute 80-JVP cap, not the cumulative fraction, is the active constraint in the observed high-cost events.

## G — Generation
The next likely cost tail is the **full E continuation after a successful zeta recommendation**, not the prefix. Existing audit rows already contain full-E work, so the next node should first reconstruct incremental continuation work from durable ledgers before adding new solver runs.

## Claim status
- Transactional prefix cap: **SURVIVES**.
- Frozen zeta safety rule on fresh N=320: **SURVIVES this discriminating holdout**.
- Generic safety theorem: **NOT CLAIMED**.
- Active polyalgorithm economics: **NOT TESTED / NOT CLAIMABLE**.
