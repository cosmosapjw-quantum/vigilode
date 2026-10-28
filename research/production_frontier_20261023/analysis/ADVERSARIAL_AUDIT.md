# Independent adversarial review

This review begins from published source `9b210c76dffb8909902f733e80d6cfa32405006e`. It does not repeat the old full campaign. The prior package already shows sufficient negative controls to reject a broad production claim; the question is which specific implementation contracts the new work closes. New-code review and the final 59 targeted native regression receipts have now been inspected; the bounded campaign/figure review is tracked below.

## Acceptance logic

An algorithmic identity, an implementation contract, a completed trajectory, and a competitive production solver are four different claims. Exact affine action can establish the first two under a declared constant operator without validating arbitrary nonlinear forcing. A graph heat equation can establish conservative spatial scaling without validating radiative transfer, adaptive remapping, stiff chemistry, or geometry-dependent transport. A causal projected root can eliminate current-stage derivative work while retaining frozen-basis JVPs and small matrices. Reports must preserve these distinctions.

The machine-readable counterpart lists each inherited blocker, including general annihilator providers and broad external comparisons that are not automatically closed by the four main implementation branches.

## Mathematical checks and constructive counterexamples

1. **Work pressure and stiffness are different.** On `y'=ωRy`, with `R²=-I`, the exact semigroup is a norm-preserving rotation. Explicit phase accuracy can require many steps even though the real part of both eigenvalues is zero. An implicit method can pay Jacobian and solve costs without allowing a larger phase-accurate step. A pilot must compare physical progress and achieved error and charge its own failed work. A cheap local pilot supplies local evidence, not a uniform future-performance guarantee.

2. **A stable polynomial need not preserve nonnegativity.** Bounding `|p(hλ)|≤1` on a negative real interval controls a spectral norm for a symmetric graph generator. It does not imply every entry of `p(hQ)` is nonnegative. Positivity is a separate contract: use a positive-coefficient Markov expansion, a proven absolute-monotonicity interval, or test and report a limiter/projection with its conservation/error consequences.

3. **Finite propagation need not be well conditioned.** For a strictly lower triangular matrix `T`, `(I−T)⁻¹=Σ_{k=0}^{s−1}T^k` is finite. If the subdiagonal entries equal `M`, the bottom-left inverse entry is `M^(s−1)`. Nilpotence removes an infinite convergence condition but not amplification of residual or roundoff. Full-space residual acceptance plus a caller-justified inverse/Lipschitz bound is the existing conditional route; an unconditioned small residual is not a global solution-error certificate.

4. **Exact affine forcing has a stable singular chart.** For `A=−dI+ωR`, the increment is `hφ₁(hA)b`, which tends to `hb` at `(d,ω)=(0,0)`. Computing `A⁻¹(exp(hA)−I)b` directly can divide by zero or lose relative accuracy; forming an equilibrium `−A⁻¹b` can overflow even when the affine increment is representable. Near-zero series/expm1 and scaled large-argument formulas need independent oracle tests.

5. **Conservation uses the physical measure.** With cell densities `u_i`, weights `w_i>0`, and symmetric face conductances `c_ij≥0`, `(Qu)_i=Σ_j c_ij(u_j−u_i)/w_i` satisfies `wᵀQ=0`, not generally `1ᵀQ=0`. The symmetric similarity `W^(1/2) Q W^(−1/2)` is negative semidefinite, yielding a weighted energy contract. On a sphere, latitude-dependent weights and boundary fluxes must enter this statement.

6. **Positive uniformization gives a constructive fallback.** Let `ν≥max_i(−Q_ii)`, `P=I+Q/ν`, and `μ=νt`. Then `exp(tQ)=Σ_k exp(−μ)μ^kP^k/k!`, where `P` preserves nonnegative states and weighted mass. Replacing the omitted tail by its total mass times `P^K` preserves both properties. For the max norm, the difference from the exact semigroup is at most `2 tail·||u||∞`; a contracted-data bound using `P^k u−P^K u` can be sharper. The implemented candidate instead normalizes the retained positive coefficients; the same 2·tail bound follows by separately bounding normalization and omitted-tail differences. The tail-lumped expression is an alternative, not a description of that code. Such fallbacks cost operator actions proportional to μ and must not be advertised as mesh-independent acceleration.

## Search and evidence review

The attachment is used as a research selection discipline: diverge by at most three alternatives per blocker, evaluate semantically independent oracles, retain counterexamples, then prune by correctness/support and multiobjective evidence. A finite manually directed study is **MCTS-inspired**, not a production MCTS engine or a proof of global optimality. Timing must be coordinated on a quiet single-thread build and must not mix earlier hosts, profiles, or rejected fixed steps with accepted trajectories. Repeated points on a shared host are descriptive samples, not automatically independent confidence intervals.

No unchanged prior proof or full benchmark is rerun solely to produce a new date. Dependency changes trigger bounded contract tests. The final review will list newly inspected files, actual failures, repairs, and residual limitations after implementations arrive.

## Implementation review and repairs

The reviewer inspected the switching pilot, declared polynomial resolvent, scaled affine action and affine Lawson extension, causal trajectory example/runner, Rust/Python graph kernels, and the second-level spherical longitude split. Mathematical signs, transaction boundaries and work definitions were checked against their equations. No broad solver certification follows from this inspection.

- A causal size fixture accidentally duplicated the 384-dimensional case. It was corrected to 96 versus 384 before timed source freeze.
- Polynomial margin1 was admitted as input but could never pass a strict threshold. The constructor now uses `(0,1)` and tests the boundary. Its quotient polynomial has `d−1` candidate actions, followed by one mandatory original residual action. The conditional inverse norm follows from a Neumann series only when the supplied full-space relation bound is truthful.
- Switching pilot records use disposition order, not execution chronology. The documentation now states this. Every implicit replay is discarded; the accepted explicit candidate is the only committed state. Native shortcircuited fatal retries are existing latency debt and are explicitly counted as driver attempts rather than claimed user reentries.
- The new harmonic action initially split forcing coefficients with ordinary binary64 halves. A least-subnormal cosine coefficient at `ν=1e−308`, `h=1e308`, `A=0` vanishes during that split although the integral is about `4.16e−16`. This is an actual numerical implementation defect, not an objection to the harmonic formula. Repair must retain scaled coefficients and add a regression before promotion.

The exact-rational audit script independently reproduced two constructive controls: a stable RKL2 diffusion polynomial creates `−5/512` from a nonnegative delta while preserving mass, and finite eight-stage causal propagation can amplify residuals by `10^7`. These controls are retained as raw evidence. They prune unconditional policies while preserving conditional useful algorithms.

## Second-level spherical split

The longitude Fourier multiplier equals the exact semidiscrete periodic longitude heat flow. The remaining latitude graph retains its geometric weights. A symmetric half-longitude/full-latitude/half-longitude composition with a second-order latitude polynomial is classically second order for each fixed finite graph. Its commutator constants depend on the mesh; reducing the latitude stability bound is not a uniform PDE error theorem.

At the common `1e−5` error cap, full-graph RKL2 uses 12, 48, 104 graph actions on the three tested grids. The split uses 16, 64, 64 latitude actions **plus**16, 64, 64 FFT pairs. These different operations cannot be added as equal units or converted to a speedup without timing. The current small-grid evidence does not support an automatic split default. Retaining it as a large-grid research branch is justified; longitudes with higher Fourier indices, anisotropy and commutator refinement are the next discriminating cases.

## Final audit conditions

The integration step owns the frozen-source campaigns. The final native regression receipts were subsequently read and the harmonic repair regression passed, as recorded below. Final-source identities, achieved-error eligibility for elapsed-time comparisons, and remaining production blockers are included in the stored evidence review below.

### Harmonic repair follow-up

The coefficient-underflow defect is repaired in the inspected source. Complex positive/negative frequency coefficients are summed and halved in `Scaled` form and passed to `forcing_scaled`; they are not converted back to ordinary binary64 before multiplication by the phi action. At zero forcing frequency the implementation calls the constant-affine action, so an arbitrarily large irrelevant sine coefficient cannot contaminate the cosine result. The new regression uses the exact least positive binary64 coefficient and checks the representable `~4.16e−16` result. This closes the code-review objection; the integration record must still show the targeted regression passing at the frozen source.

## Frozen-source regression and accuracy theorem follow-up

The reviewer read the final native logs at local source `55ad7ec6af8b127cb90849db56abff4f02b9920b`:28 integrator, 29 facade and 2 causal targeted tests passed, 59 in total. The harmonic subnormal regression is explicitly listed as passing. The integration publication maps that source to remote `df5c6c56da0eb93b58147d6630a0f094cf4af212` with identical tree `5430c642253cfc9f4b05f9531a6dccafce8eb6c8`. These are affected-contract regressions, not a whole-workspace or broad external solver qualification claim.

The new RKL accuracy theorem was independently checked. With `K=s(s+1)−2`, direct substitution gives `b ω³ P_s'''(1)=(2/3)(K−4)/K≤2/3`; for `s=2` the derivative is zero. The bound `|P_s'''(x)|≤P_s'''(1)` follows by repeatedly differentiating the positive Legendre expansion of `P_n'` and using `|P_k(x)|≤1` on `[−1,1]`. Separate Taylor remainders for the stability polynomial and exponential give `(1/9+1/6)x³=5x³/18`. The commuting contraction telescope then yields `(5/18) T h_max² ||Q³u0||_W`, also for varying step sizes on a fixed self-adjoint graph. This is a constructive conditional accuracy-and-work result.

Two presentation corrections were requested: retain the minimum-stage cost `O(ET/h)` alongside the square-root stability cost; and distinguish periodic center-sampled Fourier comparison from exact cell-average projection. The sphere operation-count panel uses the axisymmetric mu mode while its spatial-error panel uses the nonaxisymmetric x mode. The figure needs this distinction visible. None of these changes alter the native algorithm or the valid discrete temporal reference.

### PDE presentation and identity repair closure

The final PDE figure was visually inspected after correction. It identifies the relative weighted norm, time0.1, operation counts, the center-sampled periodic comparison, and the different spherical modes. Labels are readable and do not overlap. The report now expressly excludes initial cell-average projection/reconstruction error from the periodic Fourier-symbol study; the spherical nonaxisymmetric oracle genuinely uses exact cell averages. The complete RKL cost sum is present in the synthesis.

A further prototype defect was identified: at `Q=0` or `t=0`, the redundant RKL recurrence can overflow an intermediate multiple of a finite state near `1.5e308`. The authorized narrow repair returns the validated input directly for these identity cases, after validating a positive requested step count. The reviewer read the nine passing prototype test results, including both new extreme-amplitude regressions. The repair does not imply arbitrary nonzero-operator extreme-amplitude safety. Its source identity follows the first native freeze and must be distinguished in the timing/publication record.

## Completed-trajectory and final prototype evidence review

The independent `src/adversarial_evidence_audit.py` reads the stored raw records and checks summary counts, physical observation endpoints and PDE timing medians. It executes no solver and collects no new timing. Its result is `results/FINAL_EVIDENCE_AUDIT.json`.

| Cohort | Completed runs | Runs satisfying the sampled error cap | Scope |
|---|---:|---:|---|
| Switching, timed | 114 of 120 | 105 of 120 | Absolute error at common observations; six failures and nine additional cap failures retained |
| Causal, one and two threads | 210 of 210 | 195 of 210 | Seventeen physical observations per trajectory; fifteen loose oscillatory cases exceed the cap |
| Affine, including warmups | 192 of 192 | 192 of 192 | 144 timed rows; direct declared action distinguished from adaptive integration |
| Native diffusion, final cohort | 135 of 135 | 135 of 135 | Relative weighted L2 error of fixed linear graph propagation |

The causal same-configuration thread comparison preserves all 84 paired observations bitwise and all 42 shared work configurations. No tested method reaches the proposed 1.2-fold two-thread speed improvement; extra scheduling is generally slower in this small-machine cohort. Causal forward propagation has useful single-thread improvements on several analytic fixtures, but the broad-spectrum case incurs 33 fallbacks and its approximately 0.999 speed ratio supplies no gain. These results support scoped selection rather than universal replacement.

Switching screening strongly reduces the previous rapid-oscillation JVP loss, yet the finite-difference variant can be slower, and the one-pilot policy misses later stiffness in the transition fixture. A pilot is a budgeted local observation, not a persistent session or a guarantee of future optimality. The legacy default remains unchanged. Typed immediate fatal termination and authorized-session reuse remain open implementation work.

Affine Lawson improves the stiff constant-forcing configurations but loses on zero/slow carriers and the generic time-forcing negative control. Exact declared harmonic propagation resolves that manufactured harmonic structure, not arbitrary time-dependent forcing. Direct formulas and adaptive wrappers must stay distinct in both dispatch rules and performance tables. Two nominal tolerance labels duplicate the direct-action configuration; its six timed samples are pooled rather than counted as separate algorithm settings.

All final native PDE medians agree with the raw five-repeat groups. All 135 rows have zero measured hot-loop allocations and maximum relative mass drift `7.7272e−14`; setup is excluded. The normalized Poisson method becomes slower than Euler at the largest tested spherical case, retaining a useful scaling counterexample. Final PDE measurements use the separately recorded post-identity-fix source. The Fourier/RKL split still has no wall-time qualification. The plotted gains compare internal fixed linear prototypes and must not be described as gains over VODE/DVODE, diffsol or Julia.

The four-panel final performance figure was visually inspected: methods, ratio orientation, separate baselines, retained losses and the distinction between a common error cap and identical achieved errors are readable. Medians are descriptive exploratory measurements. Shared-host timing uncertainty, few repeats, calibrated controls and the absence of external solver runs preclude a broad production conclusion.

The strengthened continuous-residual proof was checked after clarification: both event roots lie in the same transversality interval; any amplification divisor is finite and positive; simultaneous remap and flow-jump ordering is explicit. These are conditional mathematical guarantees and a concrete implementation specification. They do not turn sampled residuals or sampled trajectory errors into certified continuous error bounds.
