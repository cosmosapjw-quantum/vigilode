# VigilODE R2 + R4 + C8 prospective research, 2026-11-02

BASE=569ce02629498fec133db9ce368eb303ba943b69 (published C5 head).
TRACK=standalone original VigilODE generic ODE solver; applied BASS/REI work excluded.
SELECTED_STAGES=R2,R4,C8 only.
OFFICIAL_G0=7/23 unchanged; SUCCESSOR_ADMITTED=0/12; PRODUCTION=HOLD.
RESEARCH_DAG=3 nodes; do not rewrite or complete original 23-node DAG.

## Mission

Use ODE dynamical-systems theory, symbolic dynamics, graph algebra and sparse numerical linear algebra to design and numerically assess a structured solver portfolio following C5. Preserve C5's original RODAS5P ROW stage equation and source contract. Distinguish stage-linear-solver substitution (same W and same RODAS stages) from changing the whole-IVP integrator (e.g. BDF/EXPRB), which is a separate numerical method.

## Locked inputs and reuse

C5 source commit 569ce02629498fec133db9ce368eb303ba943b69; C5 result SHA256 88b7126b3016806f37a27d4de588bf3e2efc7dac37a865791df2318afaf53bff; R10 result SHA256 a95e69a9a42e18de608dab52f6c2f9b4054005e5f377ac384c350ba04e7c6535; original RODAS raw-U source blob d68239d53977e36f98b186b627aa0c4c5c7de7cb; original tableau SHA256 211255f6125b648c6955833c3da537d9c031ec78168b2918563c29fa88c8bce6.

Preserve existing StageLinearSystem, source-owned graph-Schur, path-forest validator, original full-band BandedStageLu and matrix-free fallback. No production router/default/Cargo lock/original physical input changes.

## Hypotheses

H1: the current serial Hillis-Steele affine scan has O(n log n) work and avoidable allocation/copy overhead; work-efficient Blelloch scan has O(n) work and O(log n) ideal span, but is worthwhile only beyond a measured chain-length/thread-count crossover.

H2: for a path forest with many independent chains, parallelizing chains while keeping each chain's O(m) sequential recurrence may dominate intra-chain parallel scan because it avoids scan barriers and extra work.

H3: graph solver eligibility and economic value depend on source sign/diagonal-margin, feedback-core size, path coverage, factorization fill, repeated RHS count and residual verification cost. A graph-valid candidate can be mathematically sound yet uneconomic.

H4: eigenvalues alone are insufficient for nonnormal transient classification. A cheap regime observer combining logarithmic norm/numerical abscissa, nonnormality/transient-gain proxies, stiffness time-scale and graph features can feed a hysteretic finite-state mode selector. The selector is a heuristic until error-transition stability and matched-work validation are supplied.

## Predeclared measurements

On the Ryzen 9 5900X / Rust 1.94.1 host, compare sequential recurrence, current allocating Hillis-Steele, buffered Hillis-Steele, work-efficient Blelloch scan (serial and Rayon parallel), and chain-parallel sequential recurrence. Sweep chain length and 1/2/4/8 threads. Record wall ns/scan, pair compositions, allocation calls/bytes, path residual, max difference from sequential, q-margin and deterministic replay. Use an identical input per paired method. This is a kernel benchmark, not a whole-IVP speed claim.

Adversarial controls: n=1; noncommutative affine maps; odd and non-power-of-two lengths; q near 1; q>=1 refusal; non-finite input; one long chain vs many short chains. A separate analytic 2x2 stable-nonnormal matrix demonstrates eigenvalue/logarithmic-norm disagreement.

## Acceptance / stop

R2 complete only with primary-source synthesis and explicit reuse map.
R4 complete only with concrete equations, a finite-state routing proposal, one checked nonnormal example and a work/span comparison.
C8 scoped PASS requires exact algebraic scan contracts, deterministic numerical agreement within stated residual, measured crossover data, honest kernel-only scope and adversarial review. Any unstable/nonfinite/incorrect candidate is REFUTED and preserved. No tolerance relaxation.
Do not promote the graph solver, route selector or whole-IVP integrator. No main merge, no PR #187 mutation, no automatic next cycle. Publish a new non-force research branch/PR and preserve a copyable handoff.
