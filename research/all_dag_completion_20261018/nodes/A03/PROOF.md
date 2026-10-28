# A03: common damping through an exact frame and controlled coefficient averaging

The native source is `crates/rodas5p-integrators/src/damped_packet.rs`. It reuses the existing outward frozen polynomial packet; it does not silently replace the packet algorithm with a splitting solver. Native verification status is recorded separately in RESULTS.json.

Let positive weights have their actual exact sum S, let `||a||_w²=sum w_j|a_j|²`, and let `m=sum w_j a_j`. The admitted source is the immutable rotating-frame CollectiveModel, with common damping kappa>=0 and stored carrier=0. Its equation is

`a'_j=(-kappa+i omega_j)a_j+i chi |m|²m`.

The exact frame `a(t)=exp(-kappa t)b(t)` gives

`b'_j=i omega_j b_j+i chi exp(-2kappa t)|m(b)|²m(b)`.

Both b and every real-coefficient frozen model preserve the weighted norm. For an initial weighted ball of radius eta about the represented center z, use the outward upper radius `R=sqrt(sum w_j |z_j|²)+eta`. No assumption that the uncertain initial state itself lies in the constructor's nominal unit ball is needed.

On a subwindow of duration h, the exact mean coefficient is

`chi_bar=chi d_kappa(h)/h`, `d_kappa(h)=integral_0^h exp(-2kappa t)dt`.

The implementation evaluates the physical moment Q00 at rate 2kappa with the pre-existing outward damped-moment kernel. A checked interval gives a represented midpoint chi_hat and an outward bound r_chi on `|chi_hat-chi_bar|`. If the primary kernel refuses at large argument, the stable equivalent formula `Q00=(1-exp(-2kappa h))/(2kappa)` is attempted and subject to the same absolute-width gate. Small arguments retain the confluent Taylor kernel. At kappa=0, chi_hat=chi exactly and r_chi=0.

The packet reconstructs the complete frozen equation with chi_hat and returns a weighted-ball radius about its actual binary64 center. The following model error is added, not absorbed into the packet's tolerance.

For any real bounded random variable X in [l,u] with its true mean EX,

`E|X-EX| <= 2(u-EX)(EX-l)/(u-l) <= (u-l)/2`.

The first inequality follows by bounding the convex function `|x-EX|` by the chord between l and u and integrating. Apply it to `X=exp(-2kappa t)` under uniform time measure. Therefore

`integral_0^h |chi exp(-2kappa t)-chi_hat|dt`

`<=h[|chi|(1-exp(-2kappa h))/2+r_chi]`.

For the cubic `B0(a)=i|m|²m 1`, `||B0(a)||_w<=S²R³`, while the logarithmic norm of the frozen Jacobian is at most `c=S²R² |chi_hat|` (the implementation safely uses max(|chi|,|chi_hat|)). Gronwall applied to the difference between the true frame solution and the frozen one, starting from the same point in the initial ball, yields the uniform endpoint model-error bound

`delta_model <= exp(c h) S² R³ h [|chi|(1-exp(-2kappa h))/2+r_chi]`.

Every quantity in this computational formula is evaluated outward. For small h this conservative bound is O(kappa*h²), and adaptive dyadic subdivision reduces it without changing the original requested tolerance. This is an admitted first-order model-approximation bound, not a claim of second-order convergence or of optimality. Improved commutator-based averaging can be developed later without invalidating this client.

There is a useful exact special case. If all omega_j are equal, the linear common rotation commutes with the cubic flow. The exact endpoint depends on the time-dependent cubic coefficient only through its integral. Thus coefficient averaging is exact and only represented-coefficient displacement remains:

`delta_model <= S²R³ h r_chi`.

The implementation admits this special case only when width=0 or all stored xi values are identical. This includes every one-mode problem. If chi=0, the model error is identically zero.

Finally scale the frozen packet result by the enclosed positive damping factor d=exp(-kappa h). If r_packet is its weighted ball around its returned center, then the physical radius is

`r_physical = d_upper(r_packet+delta_model)+r_scale`,

where r_scale is the outward weighted norm of the difference between exact damping times the returned packet center and the new represented scaled center. The output mean adds its own summation displacement; component radii divide by the downward square root of each positive stored weight. Between subwindows and requested windows, the weighted radius is carried directly, avoiding repeated ball-to-component-box inflation.

All trial subwindows finish before a requested endpoint is committed. A complete result escapes only if the requested mean and all mode budgets hold. Failed trials charge counters; subdivision count is finite and configured. Every physical mesh difference and halving must be exactly represented. Nonzero carrier is a typed refusal: the source code currently ignores its stored carrier, so claiming a lab-frame solution would change the equation.

Work is O(subwindows * packet_cost) plus O(N) per scaling/output. The existing packet's polynomial order/degree/sweeps and all frozen residual checks remain authoritative. The new counters track setup attempts, scalar operations, moment attempts, copies and rejected windows, but are not allocation measurements or a complete wall-time model. No measured performance promotion follows from this implementation.

The public local verifier is `verify_damped_packet_window(model,h,initial_weighted_radius,config,work)`. It returns an immutable endpoint enclosure and can independently certify a damped homotopy candidate by enclosing the actual source flow from the same window input. The homotopy client must additionally account for the difference between its candidate and this verified reference center; this packet verifier alone does not certify the candidate.
