# VigilODE v3.8-D High-Entropy Performance Tournament Roadmap

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this roadmap plan-by-plan. Each child plan is a separate reviewable unit.

**Goal:** Execute the approved v3.8-D design without mixing timing authority, exploratory benchmarking, candidate implementation, and promotion into one unbounded change.

**Architecture:** The roadmap is split into three independently reviewable plans. Plan A implements the already-sealed timing authority validator without producing new wall output. Plan B creates a common exploratory benchmark substrate that is explicitly non-authority. Plan C evaluates the first candidate wave in isolated worktrees and records `SURVIVE`, `HOLD`, or `KILL` outcomes before any combination.

**Tech Stack:** Rust 1.94.1, Cargo locked offline vendor, Python 3 standard library plus committed plotting dependencies, Git worktrees, JSON/CSV evidence, GitHub PRs, Jira PM project, Confluence SD space.

**Spec:** `docs/superpowers/specs/2026-08-23-v38d-high-entropy-performance-tournament-design.md`

## Global Constraints

- Scientific implementation baseline is `main@db51a9537a3f4898149cb463711eab0925387388`, tree `4a82e7b9196c383fdd9a9cae5ba566035ea420e0`.
- Integrated design parent is `main@c9df85a7f7c1fe2cf296b4d00da8799ac04e10f8`, tree `ad8bf3fbba710b3d5e7c4260e8a9b5b05268fc79`.
- R-JF remains the sole committed trajectory; E remains read-only shadow evidence.
- Preserve `k=3`, prefix cap 80 JVP vectors, cumulative prefix fraction 0.25, `tau_zeta=13.39706618860016`, and continuation cap 80 JVP vectors.
- Preserve 64 frozen recommendations, 62 bounded completions, two charged continuation-cap exhaustions, zero numerical continuation failures, zero unsafe recommendations, and zero budget breaches on consumed replay unless a candidate is explicitly numerical and its changed outcome is separately audited.
- Do not generate a new paired-wall timing-authority campaign in Plan A or Plan B.
- Exploratory wall measurements must carry `EXPLORATORY_NOT_TIMING_AUTHORITY` and retain every repetition and failure.
- No active switching, controller/cache transfer, BDF/Radau tuning, physical-client tuning, N=2048 execution, new production dependency, tag, or release.
- One initial attempt plus at most one diagnostic correction per materially equivalent candidate failure.
- One primary verification and at most one independent diff review per coherent artifact.

---

## Plan Boundaries

### Plan A — v3.7 Timing Authority Validator

File: `docs/superpowers/plans/2026-08-23-v37-timing-authority-validator-implementation.md`

Deliverable:

- deterministic host/identity attestation capture;
- fail-closed whole-campaign validation;
- exact all-pair retention;
- three-passing-within-five-attempt summary;
- retrospective v3.6 diagnostic proving the whole historical campaign is non-authority because of N=384 host-quality failure;
- no new paired-wall output.

Remote integration boundary:

- branch `research/v37-timing-authority-validator`;
- one implementation PR after focused verification;
- merge requires explicit user approval.

### Plan B — Common Exploratory Benchmark Substrate

File: `docs/superpowers/plans/2026-08-23-v38d-baseline-benchmark-substrate.md`

Deliverable:

- stable Rust report schema and dedicated research binary;
- deterministic synthetic operator cases;
- current full-MGS authority probe;
- allocation counting in the dedicated probe binary only;
- event adapters for normal and two known semilinear tail events;
- Python analyzer and plots;
- explicit non-authority status.

Remote integration boundary:

- branch `research/v38d-exploratory-benchmark-substrate`;
- proposed only after Plan A is merged;
- no performance candidate is included.

### Plan C — First Candidate Wave

File: `docs/superpowers/plans/2026-08-23-v38d-first-candidate-wave.md`

Initial isolated evaluations:

- K1 projected exp/phi1 oracle fusion;
- K2 reusable fused-phi workspace;
- K3 contiguous basis/Hessenberg layout;
- C1 checkpoint schedule sweep;
- C2 selective reorthogonalization;
- C4 adaptive Krylov-dimension/substep controller spike.

Deferred from the first wave unless a prerequisite result opens them:

- K4 reduced-space backend;
- M1/M2/M3 multi-action and block paths;
- A1/A2 alternative matrix-function backends;
- A3 cross-step recycling.

Promotion boundary:

- failed and held candidates stay local or in durable bundles;
- do not open a remote PR for every candidate;
- only a coherent survivor or smallest compatible combination receives a PR;
- merge requires explicit user approval.

---

## Jira / Confluence Control Model

- Jira PM owns execution status and task dependency.
- Confluence page `VigilODE v3.8-D Performance Tournament — Canonical DAG and Claim Boundaries` owns current DAG, claim boundary, and links.
- GitHub owns source bytes, design, plans, commits, and review diffs.
- Durable local bundles own failed/held candidate custody when no remote PR is opened.
- Rovo retrieval keys are `VigilODE`, `PM`, `v3.8-D`, `Timing Authority Validator`, `High-Entropy Performance Tournament`, `K1`, `K2`, `K3`, `C1`, `C2`, and `C4`.

## Completion Definition

The roadmap is complete when:

1. Plan A is implemented and reviewed without generating a new timing campaign.
2. Plan B produces deterministic non-authority benchmark reports and plots.
3. K1, K2/K3, C1/C2, and C4 receive bounded isolated evaluations.
4. Every evaluated candidate has hard-gate evidence and one durable disposition.
5. At most one or two survivors are nominated for a genuinely fresh holdout.
6. The closeout explicitly permits a no-survivor result and makes no active-speedup claim.
