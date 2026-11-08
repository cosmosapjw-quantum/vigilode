# VigilODE solver portfolio consolidation plan

**Plan:** VIGILODE-SOLVER-PORTFOLIO-20261108  
**Audit source:** 569ce02629498fec133db9ce368eb303ba943b69  
**Branch:** docs/solver-consolidation-plan-20261108  
**Verdict:** HOLD_NOT_RELEASE_QUALIFIED

This is a documentation-only audit and implementation plan. It changes no Rust source, tableau, default, tolerance, benchmark threshold, historic receipt or release gate.

Start with PLAN_BUNDLE.json. It contains source-pinned audit findings, production verdict, layered architecture, method capability matrix, 19-task execution queue and dependency graph, acceptance gates, mutation policy, anti-drift rules, benchmark protocol, and literature/source references. DESIGN_CONTRACT.md and VALIDATION_PROTOCOL.md expand implementation and review requirements. AGENT_PROMPT_KO.txt is the original model-execution prompt.

Architecture decision: extend the existing crates/vigilode frontend with distinct method plugins and common lifecycle/session contracts. DOPRI5 and protected RODAS5P are the first adapters. Internal BDF/Radau remain reference-only until independently qualified. Rosenbrock-K is a new method, not a RODAS5P refactor. Algebraic fixed-point/homotopy components are not generic time steppers.

## Reuse-first implementation gate

Read [REUSE_FIRST_INVENTORY.md](REUSE_FIRST_INVENTORY.md) before any code change. New work must extend the pinned local pool, stage-batch harness, DOPRI5 ensemble, Problem/operator API, homotopy/continuation, polynomial resolvent, Fourier path and source-specific certificate code. No second solver frontend, pool, benchmark framework, WorkLedger, homotopy engine or Fourier implementation is permitted. Greenfield code is restricted to missing mathematical cores such as peer-W coefficients/history/startup/estimator and finite resonant normal-form lift construction.

## CPU-only parallel Rosenbrock research addendum

The current research work is extended by the following machine-readable/design files:
- PARALLEL_RESEARCH_PLAN.json: execution nodes, source lock, separate common-shift TSW and multi-implicit peer-W candidates, acceptance/failure gates and explicit GPU prohibition.
- PEER_METHOD_FAMILY_REVIEW.md: source-bound equations and distinction between parallel W, multi-implicit peer-W, nonlinear parallel peer, and sequential peer controls.
- PEER_PARALLEL_STAGE_PROOFS_20261109.md: constructive explicit-peer order/stability/convergence proofs, DQC compatibility audit, common-W/rank-r algebra, finite resonant parent lift, and implementation gates.
- PEER_W_TABLEAU_CERTIFICATES_20261109.md: exact variable-step PTSW2C tableau and estimator, stiff-accuracy/order impossibility, explicit-peer coefficient generator, EPPEER source audit, and method-selection gates.
- STAGE_TRANSFORMATION_THEORY.md: parallel peer-W, finite resonant normal-form lift, linear parent stage-block identity, Fourier/involution findings and limits.
- HISTORICAL_RESEARCH_MAP.json: 14 source/PR/code-lineage findings with evidence boundaries.
- CPU_PARALLEL_NODES.md: stage-internal CPU, independent IVP ensembles, and sphere/2D/3D MOL CPU execution.
- PARALLEL_BENCHMARK_PROTOCOL.md: matched-achieved-error, setup-inclusive CPU benchmark contract.
- PARALLEL_AGENT_PROMPT_KO.txt: copyable execution prompt for the next model.

GPU is explicitly forbidden across all new nodes and fallbacks. The preferred general stage-parallel candidate is a new two-step Rosenbrock-W peer method, not a claimed parallelization of the protected lower-triangular RODAS5P tableau. A finite Poincare-Dulac resonant lift is a separate local/source-class-specific candidate. The Fourier-Volterra code is already implemented for a narrow source, but its PP05/PP06 registered failures and PP15 no-timing caveat remain preserved.

The branch is based on the PR #187 research snapshot, not main. Inspect local branch/worktree state before any merge. This is a documentation-only research update; no solver source was changed.

Remote Desktop device was offline at the prior check, so /home/cosmosapjw/vigilode was not modified. After fetching this branch, materialize the package with:

```bash
cd /home/cosmosapjw/vigilode
git fetch origin docs/solver-consolidation-plan-20261108
git worktree add /tmp/vigilode-plan-materialize docs/solver-consolidation-plan-20261108
mkdir -p docs/agent
cp -a /tmp/vigilode-plan-materialize/docs/agent/solver_consolidation_20261108 docs/agent/
python3 docs/agent/solver_consolidation_20261108/validate_package.py
```
