# Integer-only physical-norm admission

Target: E²=(1/33)sum_i((Le)_i/s_i)² for the exact real values encoded by finite binary64 errors and positive finite supplied scales. Fixed PR79 compact30/full33 +/-1 map only. Dependent rows23/25/28 are sums of1/2/3inputs; other rows copy. Offset b absent. Units cancel. No new ODE method, scale constructor, general chart or global-error claim.

Every finite binary64 is an integer in units2^-1074. Separate signed accumulation yields exact row magnitude A<2^2100; scale B is a positive integer in the same units. Validate ALL input bit patterns before early rejection. If A>=8B for a row, E²>=64/33>1 proves Reject without division.

Otherwise use p=56, q=floor(2^p*A/B), remainder r, u=q+[r!=0]. Then q/2^p<=A/B<=u/2^p, hence L/(33*2^112)<=E²<=U/(33*2^112), L=sum(q²), U=sum(u²). Accept only U<=33*2^112, Reject only L>33*2^112, otherwise Indeterminate. An exact representable boundary may resolve because zero remainder gives u=q. An unresolved tiny positive term on E²=1 can still require Indeterminate. No threshold relaxation.

Capacity: A*2^56<2^2156 fits34u64words; shifted denominator also fits. Since A/B<8, q<2^59 and u<=2^59, and33*2^118<2^128. Integer division needs at most59quotient bits/row. Outward square width per inexact ratio is(2q+1)*2^-112; aggregate absolute E² width<16*2^-56+2^-112, about2.22044605e-16. This is not a relative tiny-value bound. Carry, borrow and capacity remain implementation-review obligations.

The new primitive consumes u64 encodings and has no floating arithmetic. Its argument interpretation and proof do not depend on FP rounding/FTZ/DAZ; this does NOT repair upstream f64 values computed under a wrong environment, authorize all Rust floating code in nondefault modes, or certify compiler/hardware correctness. A separate isolated mode experiment and integer-source/IR checks will be evidence, not a whole-platform proof.

Alternatives: FP-mode guard preserves old floating intervals but is platform-specific; exact rational final comparison removes ambiguity but has larger denominator products. Fixed integer brackets are the bounded selected route, not a claimed universal speed optimum. Superaccumulation is established (R.M.Neal, arXiv1505.05571), not newly invented here. Wolfram checked capacities,64/33>1 and width identity. Native V1 owner/hook remains protected and unreconciled.
