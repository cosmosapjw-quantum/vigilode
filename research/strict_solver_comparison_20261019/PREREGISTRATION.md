# Strict solver comparison: frozen design before timing

Parent numerical source: a21d573980f66fe02891bd216153443b5352f921. The earlier comparison used SciPy 1.17.0's C port of VODE, not original Fortran DVODE. Its dense-Jacobian transpose regression makes it an invalid algorithm-ranking baseline. Preserve those rows; classify them as wrapper-regression diagnostics, not performance evidence.

This coding loop covers C1-C10 in bounded form: definitions and contract tests; common native model callbacks; independent reference and derivative checks; same-host accuracy screen; frozen accuracy-only selection; separately replicated timing; critical interpretation; durable publication. No full 142-component triage or production promotion is claimed. Native subagent dispatch is unavailable; no independent-agent review is claimed.

## Planned comparison

- Compile native VigilODE RODAS5P fast, Diffsol =0.16.1 BDF/FaerLU, and Netlib original Fortran DVODE into one driver. Common Rust RHS and analytic Jacobian entries are shared. Original Fortran source and LINPACK routines are downloaded and SHA-256 recorded; Rust versions/lockfile are recorded.
- Cases: Robertson, HIRES, van der Pol mu=1000, Brusselator 50 and 200 cells, plus an exact two-state nonsymmetric stiff pivot fixture. All six have identity mass and autonomous RHS.
- Main grids: endpoint and uniform101. Input rtol: 1e-4 through 1e-10 by decades. Scalar atol = case_scale * rtol. Initial step = 1e-6 * {0.5,1,2}. DVODE max order 5. Success and reached outputs are mandatory; missing outputs, wrong shapes, nonfinite states, invalid reference radii must not produce finite accuracy scores.
- Native fast lands on output times; Diffsol and DVODE may interpolate. These are equal requested-output contracts, not identical internal output algorithms. Record this distinction; no continuous-time error theorem.
- Independent Python Radau tolerance ladder and Fortran LSODA cross-check on SciPy 1.16.3; exact solution for pivot fixture. Derivative/model parity required before using references. Empirical radius, not certified enclosure.
- E = max-grid WRMS using fixed case_scale + abs(reference); E_up adds radius in the same norm. R_up uses each row's input atol+rtol*abs(reference). R_up>1 is an observed global/grid-tolerance miss, not proof that a local-tolerance solver violates its documented local contract.
- Targets: E_up <= {1e-4,1e-6,1e-8}; reference radius <= 0.1 target. Select the largest input tolerance passing all three h0 scales, based on accuracy only, before timing. No cheapest-of-noisy-times selection.
- Timing: same compiled process code, common host, CPU affinity and one thread; separate fresh processes in 12 balanced blocks, each with discarded warmup and fixed repetitions. Reference calculation/build never overlap timed execution. Time includes per-solve setup and requested-output storage, not JSON serialization/process startup. Full paired block ratios retained. Shared runner timings are diagnostic only.
- Preserve failed attempts, censored timeouts and all host/toolchain identities. No speed ranking where any admission/reference gate fails. No cross-host ratios to historical timings.

## Stop conditions

Correctness-contract or common-model parity failure stops ranking. Build failure blocks only the affected lane, never becomes a fast result. At most one evidence-directed repair pass for a given failure class. No changes to production solver implementation in this node. Heavy/reconstructible binaries and dependencies are not committed. Essential data, scripts, failures and hashes are committed under this node.
