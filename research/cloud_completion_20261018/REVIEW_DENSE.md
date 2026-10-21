# Independent review: sealed PR101 continuous verification

Reviewer: structured-driver agent; implementation author: dense-completion agent. Reviewed source is bound in `evidence/review_dense.json`. The review accepts the stated scalar, identity-mass PR101 target on [0,1], its immutable provider and verified transactional driver. The evidence does not expand this result to arbitrary `OdeProblem` callbacks.

## Mathematical checks

1. The stored quartic is exactly `q(theta)=(1-theta)y0+theta[y1+(1-theta)(d0+theta(d1+theta d2))]`. The power representation `[y0,y1-y0+d0,d1-d0,d2-d1,-d2]` is interval-enclosed. Its endpoints are exactly the stored binary64 y0 and y1. Consequently propagating the verified endpoint radius to the next endpoint center is sound; no interior evaluation-roundoff term must be added at this endpoint copy.
2. `trig_at` sums Taylor coefficients through degree24 using interval division and encloses the remainder by t^25/25! on [0,1]. Shifted degree12 sine and cosine use the four-cycle derivative pattern, with remainder h^13/13!. Both constructions cover the exact mathematical trig functions independently of candidate libm evaluations.
3. With `z_p=q-sin_p`, the residual polynomial is `q'/h - cos_p - lambda z_p - eta z_p^2`. The coefficient convolution is formed before coefficient magnitudes are summed, preserving cancellation. If each trig tail is at most r, the omitted residual is bounded by `r + |lambda|r + |eta|(2||z_p||+r)r`, precisely the implemented tails.
4. Over the full tube `|y-q|<=T`, `f_y=lambda+2 eta (y-sin t)` is bounded above by `mu=lambda+2|eta|(||q-sin||+T)`. The mean-value theorem gives the scalar error differential inequality `D+e<=mu e+R`. The signed transport `exp(mu h)e0 + R integral_0^h exp(mu s)ds` is evaluated outward, retaining negative-mu contraction. For constant mu/R the comparison solution is monotone, so `max(e0,endpoint_upper)` encloses the complete segment. Closed-tube continuation establishes the accepted bound whenever this uniform value is <=T.
5. Evaluation at a requested represented time adds the interval displacement between the exact stored polynomial and the computed output center. Initial/endpoint ownership and private fields prevent edits from separating an accepted curve from its radii. No exact-solution callback is attached to the production candidate problem.
6. The driver's new progressive local tube is at most the fixed global atol/2 budget. It is a numerical budget policy, not a premise weakening. Accepting only requests with returned radius<=atol preserves the original absolute tolerance, and thus also atol+rtol|y|.

## Code review finding and closure

DR01: candidate step disposition counters were sampled after `sequential_step`, and the outer rejection path could classify an already rejected native candidate again. The author changed the code to snapshot accepted/rejected counts before the candidate, retain all execution work counters, restore only the two disposition counters, then add exactly one outer decision. The added native-overflow regression checks one attempt, one rejection, zero accepted segments and zero outputs. This is a bookkeeping correction; the continuous error proof is unchanged.

A separate author/reviewer check of a nonzero nonlinear expanding orbit exposed future error-budget starvation. The progressive tube policy and exact orbit `sin(t)+1/(16-t)` regression were examined. No promotion is conditioned on this heuristic guaranteeing all expanding orbits finish; unsuccessful runs retain only committed verified output and report failure.

## Independent execution and scope

`tests/independent_pr101_review.rs` was written by this reviewer. It tests nine signed/expanding combinations lambda in {-10,0,2}, eta in {-3,0,3}, a prescribed linear quartic on [0,1/8], and a nonzero inherited uncertainty disk. At33 points per case it compares the returned centers/radii with the exact sine member. The focused test passed. The mathematical argument above, rather than finite sampling, supports the continuous claim.

The author tests include the original stiff witness lambda=-1e6, eta=10, atol=1e-11, rtol=1e-8 and101 requested outputs, rejection rollback, adjacent represented outputs, native failure accounting, and a genuinely nonzero nonlinear orbit. Their final source and execution logs are referenced in the JSON. Existing directed arithmetic primitive contracts are reused; their implementations were not changed in this unit.

Decision: accept the conditional continuous-enclosure theorem and opt-in supported-model implementation. Generic solver global accuracy, arbitrary callbacks, wall-clock speed and allocation claims retain their separate decisions.

## Final symmetry delta

After the initial review, the author replaced the full13x13 square convolution by its upper triangle, doubling off-diagonal interval products. This reviewer read the final loop directly. For exact scalar coefficients, `z_i z_j=z_j z_i`; interval multiplication encloses that shared product, and outward multiplication by2 encloses the two identical contributions. Diagonals remain single. Thus the polynomial residual coverage is unchanged while the convolution uses91 products instead of169. Final test accounting expects639 provider operations for the linear fixture. The narrower independent transport review is also recorded in `REVIEW_DENSE_TRANSPORT.md`. The refreshed JSON binds the final symmetry source and seven-test file, rather than the earlier bytes.
