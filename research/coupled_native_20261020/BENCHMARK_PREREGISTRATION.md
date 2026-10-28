# Native complete-step experiment, frozen before timing

This implements the supplied integration report's first bounded native step. It is not the proposed whole adaptive dispatcher. Current scientific code is an opt-in standalone companion crate; no defaults or existing numerical crate sources change.

16 cases: few-mode n96/384/12288 at eps1/10,h0.01 and eps1,h0.001; broad-spectrum n64,eps0,h0.02; chain n96/384,eps1,h0.01; periodic diffusion n96/384,eps1,h0.001/0.01. Case equations live in the benchmark adapter; all solvers receive supplied analytical JVPs. Candidate never receives an explicit Jacobian. Small independently assembled matrices are permitted only in the untimed reference. All cases are autonomous identity-mass.

Arms: coupled with1/2/4workers, existing native-K sequential GMRES, and explicit coupled4→sequential fallback. Whole candidate cost includes basis, reduced dense root solve, all stage callbacks, original-space validation and any declined work. Pool creation and result destruction are excluded and pool setup cost is separately recorded. A sequential loop is allowed in the small reduced system and untimed reference; the candidate has no8large-space serial W solves.

Input scale1e-10+1e-6*abs(state). Generic Config defaults fixed before timing. Reference target is raw-U stage root, not the exact continuous solution. Cross-check independent Rust raw recurrence and Python equations, empirical discrepancy radius; maximum relative difference <=1e-10. A result qualifies for a fixed-step comparison only if complete, finite, deterministic, output and embedded difference WRMS<=0.1. Native-K versus raw-U comparisons are threshold-only, NOT an exact target-identity claim. Worker-count equality is checked bitwise in independent readback.

80 accuracy cells. Ten fresh-process paired blocks, five balanced arm orders;3repeats for n12288 and5otherwise, discarded warmup1. Failed attempts still have timed costs retained but no speedup verdict. There are64predeclared comparisons; bootstrap100000samples using process/block units, pointwise and Bonferroni-family intervals. Shared-runner conclusions remain conditional diagnostics. No best-time tolerance selection, no pooling old hosts, no trajectory or strongest-external-solver ranking.

Validation: original5contract tests,12edge/failure/accounting tests, exact Riccati local-error refinement and failed fallback counters. Python readback independently recomputes saved metrics and all ratios from raw repeats. Different computations by one author do not constitute independent-personnel review. CodeRabbit prerequisite and requested PR review are recorded separately from numerical validation.

Stop: bounded native step and this campaign, preserve any declined regimes and blockers. No auto-merge or default promotion.
