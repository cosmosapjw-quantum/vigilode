# Candidate reconsideration, 24 October 2026

The objective is to recover useful accuracy, robustness, stability, or execution-cost improvements from previously excluded VigilODE candidates. Previous selection decisions are evidence about their recorded assumptions and workloads, not universal exclusions. The source is the complete tree published at `14171ed0c4f7dc453f47870cf6c1399f41afc4f2`.

The investigation covers the 25 recorded portfolio families and additional algebraic constructions recovered from earlier pull requests. Existing proofs and raw runs remain valid for unchanged inputs; new assumptions, new implementations, and comparative claims require fresh derivation or execution. The current request explicitly authorizes modifications, reruns, lightweight benchmarking, and publication, superseding the preceding analysis-only request.

We separate three outcomes: mathematical usability under stated assumptions; a callable implementation with correctness and failure contracts; and measured benefit on specified workloads. A method can improve robustness without winning time. A component test is not a trajectory benchmark. No benchmark reference answer may be used to choose a runtime route.

## Revision budget

The first implementation and test is round 1. A diagnosis-driven modification and its affected reruns is round 2; at most one further such round is round 3. Independent candidate units record their own rounds. Documentation corrections that do not alter an algorithm are recorded separately. Failed builds and failed numerical cases are retained. After three algorithmic rounds, remaining issues are reported rather than hidden by additional tuning.

## Numerical comparisons

- Preserve original equations, physical output times, and configured tolerances across routes.
- Report achieved reference error separately from local error estimators and algebraic residuals.
- Include projection, rejected attempts, policy exploration, fallback, and setup in end-to-end costs.
- Use single-thread comparisons first. Parallel-stage work and independent-trajectory throughput have separate interpretations.
- Compare a rescued component with a strong applicable direct implementation as well as its predecessor; disclose any missing strong comparator.
- Keep failed cases in machine-readable outputs, with no success-only denominator.
- External solver comparisons, if run, are restricted to VODE/DVODE, diffsol, and Julia. This lightweight campaign may use internal controlled baselines without claiming a new external ranking.

## Implementation priorities

1. Extend quotient resolvents from near-exact algebraic identities to bounded residual correction using a supplied full-space defect witness. Preserve original residual checks and transactional output.
2. Add an opt-in causal trajectory controller that limits repeated failed projection work, bypasses directly to the same protected stage target, and permits bounded re-exploration. Keep fatal callback and model errors terminal.
3. Recover the distinctive algorithmic exposition in the external design: projected causal Rosenbrock elimination, parallel finite propagation, continuation clients, quotient algebra, and the exact roles of provided JVPs and auxiliary matrices.

The investigation is for general ODE algorithms. Attached application research supplies reusable mathematical or validation principles; it does not add application-specific development requirements. NCP is not used.
