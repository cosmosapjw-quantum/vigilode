# N01 bounded native affine source ownership, 2026-10-29
Source parent: PR113 HEAD `beee9aaf4f83eaedb5e4a3167cd3f42059215ec8`.
Original VigilODE 7/23; central REI 28/88; successor admitted 0/12; N00F scoped 1/1. Production HOLD.

## Scientific source (time-affine class only)
At fixed represented reference clock t0, the owner constructs an exact-real model from binary64 coefficients:
`f(t,z)=[J0+(t-t0)K]z + [b0+(t-t0)q]`.
Here time, J0, K, b0, q, sigma >= 0, positive w and p are owned (private) by one source. Epoch increments when the live clock changes; freeze returns an opaque snapshot. There is no general nonlinear/JVP error proof and no BASS33 source adapter.

For t live, set D=|t-t0|, deltaJ <= D max_i sum_j |Kij| wj/wi and deltab <= D max_i |q_i|/wi with outward arithmetic. Native source computes y=fl[(I-sigma J0)xhat] and independently encloses eta=||y-(I-sigma J0)xhat||_w using interval arithmetic. Then:
rho <= ||b0-y||_w + deltab + eta + sigma deltaJ ||xhat||_w.
mu_w(J0)=max_i[Jii+sum_(j!=i)|Jij| wj/wi].
If margin=1-sigma(mu_w(J0)+deltaJ)>0:
||xhat-x*||_p <= max_i(wi/pi) rho/margin.
Enforce BOTH original-residual and physical-stage thresholds, no output mutation on refusal. A projected or left-preconditioned residual cannot admit the result.

## Verification scope / stage provenance
R5 derivation and C5 source implementation in progress. Current session uses Rust1.94.1 interface-mirror because github.com clone DNS is unavailable; published native crate integration must be checked separately. Local mirror currently 15/15 including seven prior N01 point tests (eight new affine tests), strict Clippy/fmt PASS after an exact-zero test expectation was corrected to acknowledge outward interval enclosure. New independent exact Fraction oracle and actual native workspace build are NOT_RUN at this preregistered checkpoint. Zero timed benchmark loops in this unit.

Node fractions remain CORE 7/23; REI 28/88; proposed successor 0/12 admitted. Do not promote N01 to physical BASS source or begin N02.
