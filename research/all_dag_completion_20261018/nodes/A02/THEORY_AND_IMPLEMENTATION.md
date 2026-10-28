# A02 — transactional adaptive weighted trajectory

The new `VerifiedPacketState`, `advance_verified_packet`, and
`integrate_verified_packet_adaptive` connect the immutable A01 output owner to a
bounded adaptive trajectory. A refused candidate leaves the accepted physical
state unchanged. Its arithmetic work and error remain in caller-owned
`VerifiedPacketDiagnostics`, including when the complete solve returns an error.
This implements the original A02 `advance_verified(state,window)` contract.

## Source and reuse

The original node is `research/production_audit_20261018/NEXT_STEPS.json`, A02.
The weighted T1 error transport and the elimination of repeated ball-to-box
inflation are reused from `research/cloud_completion_20261018/PACKET.md`.
The current `PacketFrozenOutput::from_weighted_radius` owns the local polynomial,
source and incoming uncertainty. This work changes orchestration and ownership,
not the existing T1 logarithmic-norm proof or residual polynomial.

The equation is the native co-rotating undamped collective model

\[
 \dot a_j=i\,\mathrm{width}\,\xi_j a_j+i\chi |m|^2m,
 \qquad m=\sum_jw_ja_j,\qquad w_j>0.
\]

Both `kappa` and `carrier` must be zero. Frequencies, positive stored weights,
width and coupling remain those of the original admitted `CollectiveModel`.
Use the actual stored weight sum; it need not equal one exactly. Requested times
are finite and strictly increasing; each adjacent difference is checked to be
exactly representable in binary64. This node keeps these existing domain
conditions. Damping is the separate A03 implementation.

## AT1 — rejection-invariant global inclusion

Let the initial uncertainty be the product of complex disks
\( |a_j(t_0)-c_{0j}|\le\eta_j \). Define the outward initial radius
\(e_0\ge\sqrt{\sum_jw_j\eta_j^2}\). Assume the local frozen-owner bound of A01:
for every admitted incoming center \(c_k\), radius \(e_k\), and exact duration
\(h_k>0\), it returns a point \(c_{k+1}\), component and mean disks, and radius
\(e_{k+1}\) containing the **same original exact flow** for every incoming state
in \(\|a(t_k)-c_k\|_w\le e_k\). The returned radius includes the displacement
between the exact stored polynomial and the returned floating-point center.

A transactional step computes the entire candidate, freezes the incoming ball,
queries the endpoint, tests both original absolute budgets, copies the returned
point, and constructs the next model before mutating the state. Consequently:

1. If preparation refuses, the state tuple \((t_k,c_k,e_k)\), original source,
   and physical budgets are unchanged. Work and diagnostics may increase.
2. If preparation succeeds, the state is replaced once by
   \((t_k+h_k,c_{k+1},e_{k+1})\), and all stated local inclusions hold.
3. Induction on accepted steps proves inclusion of the original nonlinear
   trajectory after every accepted step, independently of the number and
   placement of refused candidates between accepted steps.

For the induction, the base follows by summing the input disk squares with the
positive exact weights. At the inductive step, the hypothesis supplies precisely
the frozen owner's incoming weighted-ball premise. The accepted endpoint's
centered radius supplies the next premise. A rejected attempt does not change
that premise, proving independence from rejection history. There is no model
linearization or substitution of a changed initial condition in this argument.

All accepted endpoints satisfy `mean_radius <= mean_tolerance` and every
`mode_radius <= mode_tolerance`. These tolerances are private state data and
have no mutation API. Subdivision introduces a stricter requirement at internal
endpoints; it never weakens a physical output budget. The API returns a complete
successful trajectory only after every original requested output has passed.

## AT2 — exact mesh and bounded refinement

For each requested endpoint the adaptive driver first attempts the entire
remaining interval. On an eligible budget, domain, or numerical arithmetic
refusal, it bisects the interval and processes the left child before the right
child. Identity, shape, configuration and allocation errors are returned
immediately. The numerical refusal itself remains in the diagnostics.

For endpoints \(a<b\), let the representable midpoint candidate be
\(m=\operatorname{fl}(a+\operatorname{fl}((b-a)/2))\). Admission requires
\(a<m<b\) and independently verifies
\(\operatorname{sub}_{\downarrow}(m,a)=
  \operatorname{sub}_{\uparrow}(m,a)>0\), with the same test for \(b-m\).
Thus each accepted duration equals the exact difference of its stored endpoints.
Real arithmetic gives \((m-a)+(b-m)=b-a\); there is no accumulated clock drift.
The driver returns the original requested endpoint values bit-for-bit.

The controller additionally reserves capacity for later windows. With an incoming
initial radius \(e_0\), it computes the planning radius
\[
 B=\tfrac12\min\!\left(\epsilon_{\rm mode}\sqrt{\min_jw_j},
                       \epsilon_{\rm mean}/\sqrt{\sum_jw_j}\right).
\]
When \(B>e_0\), the accepted weighted radius must also satisfy
\(e(t)\le e_0+(B-e_0)(t-t_0)/(t_f-t_0)\). This is a step-selection heuristic
computed in ordinary floating point; the original outward mean/mode gates are
still mandatory for every candidate. When the planning reserve is unavailable,
the physical gates alone apply. Thus planner rounding affects efficiency or
conservative refusals, not validity of an accepted output.

The first native run exposed budget starvation in the nonlinear fixture: the
unreserved controller spent nearly all capacity before the second requested
output and reached depth 16. The raw failure is `native_attempt1.log`. Adding
this stricter elapsed-time reserve completed the **same model, original
1e-4 physical tolerance, and requested mesh** with 16 accepted and 14 rejected
candidates; no fixture or tolerance was weakened. This is a controller repair,
not a claim of general termination at arbitrary precision.

The driver attempts at most `max_attempts` candidates per invocation. A child
may have depth at most `max_bisections`; the explicit depth-first stack requires
at most `max_bisections + 1` entries. Size overflow and failed reservation are
typed refusals. Therefore every invocation either completes or returns a typed
refusal after bounded candidate work. This is a resource termination theorem;
it does not assert that every input can satisfy an arbitrarily small tolerance.
The existing initial uncertainty, floating-point enclosure floor and model
constructor domain can still require refusal.

## Accuracy and work consequence

At an accepted step the carried scalar is
\[
 e_{k+1}=B_{w,k}+
 \sqrt{\sum_jw_j\delta_{j,k}^2},
\]
with outward arithmetic and \(\delta_{j,k}\) the returned-center evaluation
radius. Replacing this by component disks and rebuilding a weighted ball would
insert \(\sqrt N\) at every handoff. The new adaptive and transactional paths
retain the scalar and preserve the existing removal of this spurious factor.
Freezing it copies one scalar; it does not allocate an N-component input tube.

With A accepted and R rejected candidates, total work is the sum of the actual
A+R packet constructions and certificates, plus output copying and orchestration.
The log stores both work snapshots per attempted candidate; it never resets the
counters during rejection. The explicit subdivision stack is O(D), current
state is O(N), and Q requested outputs require O(QN) storage. Diagnostics require
O(A+R) records. Packet work counters are selected structural arithmetic counts,
not measured allocations or a complete FLOP/time count. No wall-time speed claim
is made by this node.

## Code and acceptance

Implementation: `crates/rodas5p-integrators/src/packet_verified_trajectory.rs`.
The historical fixed-mesh `integrate_verified_packet` is retained and also uses
`PacketFrozenOutput::from_weighted_radius`; its public interface is unchanged.

Tests: `crates/rodas5p-integrators/tests/production_audit_packet_adaptive.rs`.
The optional A10 path uses `bernstein_max_depth: Some(depth)`; `None` retains the
coefficient-only controller. After a cheap output or headroom refusal, the same
frozen owner is consumed to produce a tighter Bernstein residual bound, then the
original mean/mode and headroom gates are queried again. The heuristic target is
half the previous residual upper bound. Failure to attain that target is allowed
because A10 returns the best valid monotone bound; only the original gates decide
acceptance. The initial refusal is retained as `pre_refinement_error`, even when
the refined candidate is accepted. All first-query, refinement and second-query
work remains in the attempt snapshots. A single replay is allowed per candidate;
its subdivision budget is explicit. This makes AT1 apply without changing any
physical premise: only the valid bound on the same continuous defect is reduced.

They check rejected-state bit preservation and subsequent recovery, unchanged
physical budgets, retained work, exact output times, independent rational Taylor
inclusion for a genuinely nonlinear unequal-frequency model with nonzero initial
radii, bounded refusal on unattainable budgets, and invalid/overflow input.
The reference is the existing independent `packet_unequal_frequency_oracle.py`:
exact rational real/imaginary ODE coefficients with an analytic Cauchy remainder,
not the packet generator or its T1 certificate. Original fixed-mesh tests are
rerun because the A01 ownership call site changed.

The trace of a refused complete adaptive solve is in the externally supplied
log and work counters. Partial accepted endpoints are diagnostic history, not a
returned complete trajectory. A direct transactional caller can retain the
accepted state and explicitly decide its next step. The new API concerns the
requested output mesh; it does not add a dense-output query interface or a new
uniform-between-query tolerance claim.

## Actual validation status

`native_attempt2.log`: the first four adaptive/transactional contracts and three
fixed-mesh regression contracts passed. The new nonlinear test obtained 30
independent exact-rational containments across three perturbed initial states
and two requested outputs; its largest reference coordinate remainder was
3.300031538479502e-17. The existing test obtained 120 further containments.
These checks support the implementation for the exercised inputs; AT1's general
accepted-output guarantee follows from the reused A01 premise and induction.

`native_with_bernstein_final.log`: all five final adaptive tests and three
fixed-mesh regressions passed after connecting A10. The nonlinear exact-rational
fixture actually executes Bernstein replay at depth 2 and preserves the first
refusal. It retains 16 accepted/14 rejected windows in both enabled and disabled
runs; this fixture supplies no measured rejection saving. `adaptive_final.json`
and `adaptive_bernstein.json` record the full independent runs, all outputs and
attempt counters. The enabled run records 14 preliminary refusals and 3808
Bernstein transform terms. `clippy_with_bernstein.log` passed with warnings denied.

Independent reviewer `/root/dag_a12_independent` inspected AT1/AT2 and the final
transaction, time arithmetic, immutable owner and A10 fallback, and reported no
blocker. The overall A12 report binds the final source and evidence hashes.

Local execution (repository root):

```sh
cargo test -p rodas5p-integrators --test production_audit_packet_adaptive --test production_audit_packet_verified --locked --offline -- --nocapture
cargo run --profile measurement -p rodas5p-integrators --example packet_adaptive_run --locked --offline
cargo run --profile measurement -p rodas5p-integrators --example packet_adaptive_run --locked --offline -- 2
```
