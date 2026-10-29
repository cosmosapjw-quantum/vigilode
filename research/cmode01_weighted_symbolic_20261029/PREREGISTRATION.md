# CMODE01 preregistration · 2026-10-29

Repository: cosmosapjw-quantum/vigilode
Immutable source baseline: PR106 c6cba2b62629e819dd163325c7c9fb9bb5e3080c
PR104 10854bc0bff81b6d3cf5cb0a44520ed9165c205f; PR103 1845e1d8bdfda222bf77637e8d38d5076743f238; PR102 8357faa5b42724a8be24609d4d2f16aec8a6eb3a (sibling inputs, NOT merged)
Status: prospective isolated algorithmic research; not native production admission.

Hypothesis H1: for an exact represented real point-matrix J, positive fixed w, mu_w(J)=max_i[J_ii+sum_{j!=i}|J_ij|w_j/w_i] and sigma>=0 with sigma*mu_w<1 yield K_w=(1-sigma*mu_w)^-1. For physical weights p, error bound ||e||_(p,infinity) <= max_i(w_i/p_i)*K_w*max_i(|r_i|/w_i). This *cannot* be substituted for an unweighted norm without conversion.
Hypothesis H2: for affine J(theta)=J0+theta E, theta on a certified interval [a,b], mu_w(J(theta)) <= max(mu_w(J(a)),mu_w(J(b))). Whole-interval guard plus immutable model/source/clock/norm/epoch supports reusing this upper bound across steps. Endpoint membership alone without an enclosure of theta(t) is not enough.
Symbolic role: validated finite-state cell itinerary with HOLD/REBUILD/REFUSE guards, not chaotic symbolic conjugacy. Any source/epoch invalidation requires refuse or rebuilding before output commit.

Experiments, frozen *before new execution*:
- Exact rational 2x2 nonnormal J=[[-2,10],[1/10,-3]], w=(10,1), sigma=1/4. Expected mu_unweighted=8, mu_weighted=-1, K_weighted=4/5, original-residual physical norm conversion explicitly audited.
- Synthetic nonautonomous commuting J(t)=(1+t/4)J0, t in [0,1], 2x2-block diagonal n=128 and n=512. Fixed-step implicit midpoint with identical stage algebra and accepted requested times across naive scan and 8-cell guarded cache. Cached certificate must upper bound pointwise original stage error; output expected bit-identical if fast/slow branches differ only in certificate calculation. Achieved error vs analytic matrix exponential independently checked.
- Seven alternating ABBA/BAAB paired timing blocks (or fewer if runtime limits); report median and block range, RHS/apply, row scans, cache builds/hits, end-to-end wall-time, and raw seconds. No speedup claim absent identical outputs/physical error, no full-IVP native claim.
- Negative tests: bad weight, altered source/epoch, t beyond certified interval, negative inverse margin, wrongly used left projected residual, nonlinear theta guard violation, nonfinite arithmetic; refuse without caller-output mutation.

Optimization loop limit: at most 3 measurement/repair cycles, stop earlier if no verified net improvement.
Milestones: R1-4 theory/math counterexamples; C1-4 standalone implementation; V1 tests/plots/2 audits; B1-3 paired bench; P publish evidence and copyable handoff. Existing original DAG 7/23 scoped; auxiliary CMODE01 0/1 until native + independent review; production HOLD.

Hard bans: no root solver change, no tolerance/control budget relaxation, no backdating existing PREFIX01 prereg, no release or fitting claims, no force push/merge. Record exact code/source identity and GitHub delivery; Drive/Dropbox completion requires separate verified ACK.
