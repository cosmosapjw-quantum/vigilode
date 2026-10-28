# R3 homotopy bounded research contract

Owner: `/root/r3_homotopy`. Source commit: `cc2cd041737e7ff543624d1b59893a3b4397369f`.

Continue R2's componentwise majorant by constructing an outward enclosure for the actual eight-stage RODAS5P coefficient target on one/two-component affine and quadratic systems. Preserve the sequential stage target and account for residual evaluation, inverse bound, candidate projection, output and embedded WRMS. The standalone Python prototype may use an exact rational inverse witness; it is not a production matrix-free implementation. Candidate construction is a toy implementation of R2 q1/q2, not an invocation of the Rust transactional controller.

Acceptance: every finite computed certificate bounds an independently computed exact rational sequential root componentwise, both output projections (including floating output rounding), and the fixed-scale WRMS; failed bounds/overflow reject explicitly. Certificate work and serial stage dependencies must appear in the cost discussion. All inputs and tableau semantics must be stated. No claim of ODE discretization error certification, stiff order, general nonlinear admission, or wall-clock speedup.

Read: root preregistration, GPT-6 Astra v4.0.0 research/coding core and phase prompt, R2 homotopy code/report and independent decision, current coefficient builder, homotopy and transactional code. User's existing selection of the harness is preserved without identifying or changing model runtime. Independent decision approval is delegated by root; this lane cannot promote itself.

Stop after bounded execution, evidence, theorem derivation and concrete next implementation steps. Do not edit solver production code or perform git actions.

Additional pre-execution discriminator: for a preset stage-state radius `D=1e-4` (not tuned to reference roots), freeze the quadratic Lipschitz majorant into a strict-lower 8-by-8 block matrix. Evaluate its finite inverse with three doubling levels, then independently require `|alpha| E <= D`. Rejection is informative; acceptance must imply the same stage enclosure. This tests a parallelizable certificate formulation without using the serial exact root or serial majorant to choose the radius. Dense toy arithmetic is not a scalable matrix-free implementation.
