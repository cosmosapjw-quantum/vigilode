# VigilODE N03: source-conditional interval Jacobian original-residual stage certificate

2026-10-29. Parent PR110 `6c99864b9826070adfaf4da759efa5328f54f45a`; N03 genuine prereg `d90f28dc3f6465882e8ce294a78adfdf9cfb96f4`. Source PR109 N01, sibling PR111 N00F. This is not a claim that the physical BASS/REI state is within the supplied box.

## Definition / mathematical theorem (derived)

Fix one stage, units, time coordinate, source, epoch and two positive dimensionless normalization vectors `w,p`. In the correct physical clock let `A_*=I-sigma J_*`, `sigma=h gamma>=0` with time dimension, `J_*` reciprocal time, and `b_*` with stage-state units. Suppose independently justified componentwise exact-real enclosures `Lij<=J_*ij<=Uij` and `li<=b_*i<=ui` hold throughout the stage, and `xhat` is represented binary64. A sufficiently broad outer interval can be used for coupled entries, but independent interval endpoints may not be treated as an exact physical witness when rate constraints couple them.

Define

`mu_up=max_i (Uii + sum_{j!=i} max(|Lij|,|Uij|) (w_j/w_i))`.

This dominates the signed weighted-infinity matrix measure for *every* J in the interval family. If `sigma mu_up < 1`, the resolvent integral gives `||(I-sigma J_*)^-1||_{w,inf} <= 1/(1-sigma mu_up) = K`. This applies to nonnormal J; for sigma=0 K=1. With `C_wp=max_i(w_i/p_i)`, define componentwise residual interval

`R_i = [li,ui] - xhat_i + sigma sum_j ([Lij,Uij] xhat_j)`.

Then `rho=max_i sup_{r in R_i}|r|/w_i` is an upper bound for the *original* equation residual, and

`||xhat - x_*||_{p,inf} <= C_wp K rho`.

Acceptance of the input-box proposition requires both `rho <= tau_orig` and `C_wp K rho <= epsilon_stage`. Actual implementation computes upper enclosures with outward IEEE-754 binary64 arithmetic and rejects invalid/nonfinite/overflow, nonpositive margin, malformed model, changes in the supplied binding token, and failed certificate. The source membership and token ownership are EXTERNAL premises. Same-value string or epoch equality does not prove physical membership or protect against concurrent mutation unless the owner serializes source change with output commit.

## Exactly checked representative

`w=(8,1)`, `p=(4,2)`, `sigma=1/4`, `J11=-2`, `J12 in [8,33/4]`, `J21=1/8`, `J22=-3`, `b1 in [1,65/64]`, `b2=2`; candidate `xhat=(92/41,97/82)`.

Exact symbolic arithmetic: `mu_up=-31/32`, `K=128/159`, `C_wp=2`. Four independent input-box corners give largest weighted residual `235/20992`, largest physical solution error `1645/107502`, whereas conditional formula gives `235/13038`. Wolfram returned True. Python `Fraction` from represented binary64 verifies 64 generated models, 4 exact corners/model, 768 distinct inequalities, zero failures. Rust test separately checks output transaction and wrong norm factor (w=100,p=1) false-positive control.

## Scope map

- Established/literature-supported: matrix-measure/semigroup resolvent and interval verified residual lineage; Saad & Schultz 1986, Wathen 2007; Nguyen/Revol *Solving and Certifying the Solution of a Linear System*, Reliable Computing 15 (2011). Not new general GMRES or interval arithmetic.
- Derived: whole-box signed measure and physical norm transport, conditional stage-error inequality.
- Symbolically checked: exact 2x2 nonnormal example with Wolfram.
- Implementation-verified (local): no-dependency Rust 1.94.1 interval evaluator, 11/11 tests, fmt and strict Clippy pass; independent Fraction 768/768.
- Unresolved: real physical33 source/inventory/clock/epoch/weight membership, nonlinear Jacobian over whole slab, matrix-free JVP error, actual source owner output transaction, global IVP error, independent R6/C9, full CI/authority. BASS/REI PR265 explicitly identifies incompatible isotope inventory and nonzero T+ initial states; its source cannot be silently mapped into a shared Jacobian box.

## Algorithmic interpretation

The Givens projected residual remains only a trigger in native PR110's opt-in right FGMRES. N03 computes the candidate's original residual from source interval data (not from the projection). A native test composes N02 monitor with N03 conditional box gate, leaving default dispatcher unchanged. General physical acceptance requires a *source-owner signed membership witness* plus stable atomic/serialized epoch read, actual time coordinate, actual JVP/source uncertainty, and norm units; the optional user supplied `SourceStamp` does not provide any of these by itself.
