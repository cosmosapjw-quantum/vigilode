# VigilODE solver portfolio consolidation plan

**Plan:** VIGILODE-SOLVER-PORTFOLIO-20261108  
**Audit source:** 569ce02629498fec133db9ce368eb303ba943b69  
**Branch:** docs/solver-consolidation-plan-20261108  
**Verdict:** HOLD_NOT_RELEASE_QUALIFIED

This is a documentation-only audit and implementation plan. It changes no Rust source, tableau, default, tolerance, benchmark threshold, historic receipt or release gate.

Start with PLAN_BUNDLE.json. It contains source-pinned audit findings, production verdict, layered architecture, method capability matrix, 19-task execution queue and dependency graph, acceptance gates, mutation policy, anti-drift rules, benchmark protocol, and literature/source references. DESIGN_CONTRACT.md and VALIDATION_PROTOCOL.md expand implementation and review requirements. AGENT_PROMPT_KO.txt is the full model-execution prompt.

Architecture decision: extend the existing crates/vigilode frontend with distinct method plugins and common lifecycle/session contracts. DOPRI5 and protected RODAS5P are the first adapters. Internal BDF/Radau remain reference-only until independently qualified. Rosenbrock-K is a new method, not a RODAS5P refactor. Algebraic fixed-point/homotopy components are not generic time steppers.

Remote Desktop device was offline, so /home/cosmosapjw/vigilode was not modified. After fetching this branch, materialize the package with:

```bash
cd /home/cosmosapjw/vigilode
git fetch origin docs/solver-consolidation-plan-20261108
git worktree add /tmp/vigilode-plan-materialize docs/solver-consolidation-plan-20261108
mkdir -p docs/agent
cp -a /tmp/vigilode-plan-materialize/docs/agent/solver_consolidation_20261108 docs/agent/
python3 docs/agent/solver_consolidation_20261108/validate_package.py
```

The branch is based on PR #187 research snapshot, not main. Inspect local branch/worktree state before any merge. No solver source was changed.