# A01/A02: represented-curve weighted-ball trajectory

`packet_window::integrate_verified_packet` now computes an immutable trajectory at every requested mesh point after the initial point, with outward mean disks, mode disks and a single weighted ball about the returned binary64 centers. It uses the previously reviewed frozen-curve residual directly. The old `PacketCertificate` and old packet trajectory remain diagnostic and retain their original authority flags.

## Model and contract

The solver implements the native co-rotating equation

\[
 a'_j=i\omega_j a_j+i\chi |m|^2m,\qquad
 m=\sum_jw_ja_j,\quad\omega_j=\mathrm{width}\,\xi_j,\quad w_j>0.
\]

It accepts exactly `kappa=carrier=0`, and uses the actual stored weights and their actual sum S. The current `CollectiveModel` constructor's initial-energy/domain restrictions remain in effect. The finite increasing mesh must have exactly representable binary64 adjacent differences; `sub_down == sub_up` verifies this before solving. A suitably scaled uniform power-of-two mesh with exactly representable differences is a convenient choice; every supplied mesh is checked. This time contract prevents certifying a rounded duration in place of the requested time.

Input component disks are converted once to the weighted ball radius e0=sqrt(sum(w_j eta_j^2)), using outward arithmetic. The returned object privately stores the original model and initial disks. Each window uses the original weights, frequencies and coupling with the previous returned centers. `RepresentedPacketCurve` seals its rounded coefficients and source identity. External models and candidates cannot be rebound after sealing.

Every requested output must satisfy both supplied absolute mean and per-mode tolerances. Failure returns a typed error and does not relax either tolerance or substitute a partial successful result. The API is mesh driven and does not hide a retry policy.

## Weighted-ball transport theorem

Let ||v||_w^2=sum(w_j|v_j|^2), and suppose the incoming true state satisfies ||a(0)-Y(0)||_w <= e0. The represented curve begins exactly at the stored center. The triangle inequality gives

\[
 E(a(0))\le E_+=(\|Y(0)\|_w+e_0)^2.
\]

The exact equation preserves E=||a||_w^2, so |m(a(t))|^2 <= S E+. The reviewed same-curve coefficient enclosure supplies

\[
 M_Y\ge\sup|\sum_j w_jY_j(t)|,\qquad
 \rho\ge\sup\|Y'-f(Y)\|_w.
\]

On every segment joining a(t) to Y(t), the mean modulus is at most max(sqrt(S E+), M_Y). The previously proved weighted logarithmic norm identity gives

\[
 \lambda=S|\chi|\max(SE_+,M_Y^2),\qquad
 \|a(t)-Y(t)\|_w\le e^{\lambda t}(e_0+t\rho)=r(t).
\]

This is the reviewed local T1 estimate with a weighted-ball initial-energy premise. `enclose_at_weighted` computes every upper quantity outward and uses the existing validated exponential kernel. The original `enclose_at` path retains its existing operation-count contract.

Let c_j be the rounded Horner centers, with interval evaluation of the exact same polynomial giving |Y_j-c_j| <= delta_j. Then

\[
 |a_j-c_j|\le r/\sqrt{w_j}+\delta_j,\qquad
 \|a-c\|_w\le r+\sqrt{\sum_jw_j\delta_j^2}=r_c.
\]

The weighted entry point returns `centered_weighted_radius=Some(r_c)`. The older `weighted_radius` accessor still denotes r, excluding output evaluation displacement. The trajectory carries r_c to the next window. The mean output uses its independently outward-evaluated exact weighted polynomial and the returned center's displacement, giving sqrt(S) r + delta_m.

Induction over exact mesh durations proves every returned disk and weighted ball contains the same original exact flow. The induction preserves the source equation and weights, and does not replace the weighted-ball carry by independent component disks.

## Guaranteed reduction in artificial radius growth

Converting a weighted ball of radius r to the sufficient component disks r/sqrt(w_j), then back to a weighted ball, produces

\[
 \sqrt{\sum_jw_j(r/\sqrt{w_j})^2}=\sqrt N\,r.
\]

Repeating this conversion across K windows introduces the avoidable factor N^(K/2), including for the identity flow. Direct scalar weighted-ball carry eliminates that factor exactly in real arithmetic. The added evaluation-displacement reduction costs O(N) operations and O(1) scalar storage beyond the existing buffers. The native witness uses 64 modes and 32 identity windows; repeated component-box conversion would multiply by 8^32. The new radius remains the initial radius to within the tested 1e-7 relative margin. This is a guaranteed error-budget improvement, with no measured wall-time speedup claim.

## Independent unequal-frequency exact-rational reference

`tests/packet_unequal_frequency_oracle.py` directly constructs Taylor coefficients of the real/imaginary ODE in Python `Fraction`. It does not use packet kernels, Picard candidate polynomials, binary64 interval primitives or T1. Binary64 input values become exact rationals. Frequencies are unequal with both signs; chi takes negative, zero and positive values; weights have a sum slightly different from one; perturbed initial conditions are checked too.

For a rigorous independent remainder, complexify the real coordinates as independent analytic variables, using max_j(|u_j|+|v_j|). Define

\[
 B=\max_j(|u_j(0)|+|v_j(0)|),\quad
 \Omega=\max_j|\omega_j|,\quad K=|\chi|S^3,\quad
 R=(4\Omega+32KB^2)^{-1}.
\]

On the radius-2B ball, ||f|| <= 2 Omega B+8 K B^3 and Lip(f) <= Omega+12 K B^2. The integral map on analytic functions on |t|<=R has displacement <=B/2 and contraction factor <=3/8. Thus it supplies an analytic solution bounded by 2B. Cauchy's inequality gives the degree-p coordinate remainder

\[
 \epsilon_p(t)\le2B\frac{(|t|/R)^{p+1}}{1-|t|/R},\quad |t|<R.
\]

Zero-field and zero-state cases are exact. The oracle uses degree 28 and rationally verifies mode disks, mean disks and the returned weighted ball at four successive requested times. It checks containment of its whole remainder box rather than a rounded reference point. Six initial-condition/coupling cases produced 120 containment checks; the largest coordinate remainder was 3.3000315220941086e-17.

## Execution and scope

`evidence/packet_tests.log`: five original local-output regressions and three new trajectory tests passed. The new tests cover all three coupling signs, initial perturbations, 64-mode weighted carry, wrong frame, inexact duration, repeated mesh time, and an unattainable original tolerance that correctly refuses. Commands and source hashes are recorded in `evidence/packet_completion.json`.

Follow-up units: adaptive mesh/order selection with preserved global budgets; damping and physical-carrier reconstruction; complete allocation/cost instrumentation and matched-accuracy timing; wider scientific campaigns. The current constructor can safely refuse a rounded center near its unit-energy boundary. These restrictions are documented in the API and returned errors.

## Ready-to-run local command

```sh
cargo run --release -p rodas5p-integrators --example packet_verified_run --locked --offline -- 16 32 > packet_verified_16_32.json
```

The debug cloud run already passed with 16 modes, 32 windows, and both requested tolerances 1e-8. Its maximum mean/mode radii were 1.1268822212542885e-15 and 3.891194857018851e-15. The JSON retains every output center and radius, exact source arrays, and selected structural counters. Local release execution is useful for environment validation; it is not required to complete the proof or native implementation.
