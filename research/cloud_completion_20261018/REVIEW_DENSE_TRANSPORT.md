# Independent dense-output and parallel-transport review

Baseline: `c513f478f952068b91e3da368fb2cb156d942b28`. Reviewer: `/root/independent_review/dense_transport_review`, 2026-10-18. This review covers new source/proofs only; previously published campaigns were reused rather than repeated. The reviewer did not modify author production source. Exact reviewed hashes are in the accompanying JSON.

## Decisions

- **PT1 native prefix kernel: PROMOTE within its arithmetic contract.** Nonnegative caller-certified affine maps are composed with directed products and addition. The implementation provides deterministic outward prefixes. Its reusable `evaluate` operation now has O(N) work and O(log N) ideal fork-join span. Constructor/convenience allocation remains O(N) serial setup.
- **PT2: PROMOTE conditional theory.** The invariant-product-ball and uniform-contraction assumptions imply the stated fixed points, continuation bound, iteration bound and explicit numerical-error floor. A native nonlinear homotopy client remains a separate implementation.
- **PR101: PROMOTE the sealed model-specific verifier and opt-in verified driver.** Its initial-ball premise, identity mass, represented clock and directed-arithmetic premises are explicit. Continuous residual, signed logarithmic norm, inherited endpoint error and output rounding establish global same-curve authority on accepted segments. This decision has concrete native execution evidence.
- General production qualification and measured speed require their own relevant model/driver and same-accuracy evidence. They are not decided by this narrow review.

## Independent derivations

Write q in the admitted exact clock theta=(t-t0)/h. Expanding its stored factored quartic gives coefficients y0, y1-y0+d0, d1-d0, d2-d1, -d2. The new extractor encloses each coefficient outward and preserves the physical h used in evaluation and differentiation.

The degree-24 initial trig series encloses sin(t0),cos(t0). Their local degree-12 expansions have remainder rho=h^13/13!. Let Z=q-S and P=q_theta/h-C-lambda Z-eta Z^2. If sin=S+es and cos=C+ec, the exact defect is P+lambda es+2 eta Z es-eta es^2-ec. Therefore sum mag(Pk)+|lambda|rho+|eta|(2 B rho+rho^2)+rho bounds the continuous defect, where B=sum mag(Zk). The implementation constructs P before taking coefficient magnitudes, preserving stiff cancellation.

For |y-q|<=R the derivative upper bound mu=lambda+2|eta|(B+rho+R) yields D+|y-q|<=mu|y-q|+r. The comparison envelope v(s)=exp(mu s)e0+r integral_0^s exp(mu u)du is monotone because its derivative has fixed sign. Thus max(e0,v(h))<=R closes the tube; the bounded tube supplies continuation to the end of the segment. Negative-mu signed integration uses a certified lower exponential in its numerator; both coarse bounds used by min are independently upper bounds. The previous exact endpoint is the next exact stored initial center, so endpoint-radius induction gives error relative to the original IVP. Output radius additionally includes distance from the emitted binary64 center to the interval value of that same curve.

For affine transport, monotonicity proves that replacing an exact subtree map by its outward larger nonnegative map preserves every upper bound. The repaired preorder recursion owns disjoint subtrees, computes right prefixes after the complete left block, and overwrites all leaves without a serial reset. There are N-1 build compositions, N-1 right-prefix compositions and N leaf compositions: 3N-2 total for N>0. Each sweep satisfies S(N)=S(ceil(N/2))+O(1); allocation is separate.

For PT2, the product max norm and component Lipschitz bounds yield ||T_lambda z-T_lambda w||<=q||z-w||. Subtracting fixed-point equations yields |delta lambda| Delta/(1-q). Approximate map error eta gives stopping error (||z-T_hat z||+eta)/(1-q), and k-step error q^k D+eta(1-q^k)/(1-q). The q=0 boundary now explicitly uses one exact map application when D>epsilon.

## Findings closed with concrete changes

1. **DTR01:** Initial levelwise Rayon/serial-reset code supported logarithmic arithmetic levels, not its claimed native fork-join span. Replaced by reusable recursive code, then retested. The corrected native complexity claim is supported.
2. **DTR02:** q=0 and approximate-map stopping formulas are now explicit.
3. **DTR03:** An unnecessary initial Identity.apply was replaced by a direct guard, aligning the exact N bound-evaluation count.
4. **DTR04:** The reviewer proposed lambda=0, eta=1, z(0)=1/16, with actual nonzero nonlinear solution y=sin(t)+1/(16-t). Its first run exposed full-tube budget starvation. The preserved failed execution was followed by elapsed-time headroom reservation and a passing 17-output run with unchanged tolerance. This closes the concrete usability finding; arbitrary-parameter completion is not claimed.
5. **DTR05:** A test initially labeled Fraction oracle now has a separate exact-rational sine/cosine enclosure script. It supplies 131 checks and a whole-interval q=t error bound from monotonicity.

## Evidence inspected and independently checked

The reviewer independently rederived all 214 exact affine prefixes and six initials from binary64 leaf values; the stored oracle matches. Repaired native affine tests pass 3/3, including ordering, non-power-of-two sizes, thread determinism, workspace reuse, subnormals and fail-closed overflow. Source and logs were inspected; no historical native campaign was rerun by this reviewer.

Final PR101 native logs pass 7/7 focused tests, 3/3 primitive ledger tests and 15/15 existing dense contracts. The stiff witness returns 101 outputs with unchanged atol=1e-11 and rtol=1e-8: 492 accepted, 292 rejected, 6272 RHS evaluations and 574094 structural verification operations. The maximum sampled reference discrepancy is 8.382183835919932e-15. The independent rational linear fixture has whole-interval curve error <=1.552204217010931e-10 within the certified radius 1.5569727370016978e-10. Native sampled comparisons and exact fixture checks supplement the continuous proof; they do not replace it.

The new schedule increases accepted steps from the earlier stiff-only run's 394 to 492 while making the expanding orbit complete. A concrete next efficiency improvement is a certified future nonexpansive envelope that permits full-tube allocation in stable regimes, retaining headroom only where future amplification needs it. That change needs its own precise premise and targeted test; current source makes no measured speed claim.

## Final symmetry delta and concrete cost theorem

The final author change groups the scalar square convolution by symmetry. There are 13 diagonal contributions and 78 unordered off-diagonal pairs. Each diagonal uses multiplication, eta scaling and subtraction; each off-diagonal adds one outward factor-of-two scaling. Hence the implementation uses 13*3+78*4=351 calls instead of 169*3=507. The full successful contracting verifier requires 639 rather than 795 provider arithmetic API calls, an exact saving of 156 under the stated accounting unit. This is a verified positive implementation result. Interval enclosure follows from scalar commutativity and the exact square identity; outward doubling preserves it.

The final 7/7 native run retains 492 accepted and 292 rejected attempts, the same 101 requested outputs and the same sampled reference discrepancy. Structural verification operations fall from 696398 to 574094: 122304 fewer, exactly 784*156 for this run. The exact Fraction fixture and primitive ledger remain passing. Neither universal interval-width improvement nor a wall-clock ratio is inferred. Reviewed hashes in JSON revision 2 bind this final code and its receipts.
