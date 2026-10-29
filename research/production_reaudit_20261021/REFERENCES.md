# References and independent symbolic checks

Audit date: 2026-10-21. These sources inform the corrections; none certifies the Rust implementation or establishes a new speed result.

1. Jens Lang and Jan G. Verwer, *On Global Error Estimation and Control for Initial Value Problems*, SIAM Journal on Scientific Computing (2007), DOI [10.1137/050646950](https://doi.org/10.1137/050646950). Exa retrieved the original article's residual/logarithmic-norm bound and global-error re-integration rule. Direct DOI access through the web reader failed. The current report derives its own stated inequalities; it does not treat the search excerpt as proof of implementation correctness.
2. Yang Cao and Linda R. Petzold, *A Posteriori Error Estimation and Global Error Control for Ordinary Differential Equations by the Adjoint Method* (2004), DOI [10.1137/S1064827503420969](https://doi.org/10.1137/S1064827503420969). Exa retrieved the primary article abstract. It motivates a future adjoint error-control option, not a claim that this option has been implemented here.
3. Benjamin Kehlet and Anders Logg, *A posteriori error analysis of round-off errors in the numerical solution of ordinary differential equations*, [arXiv:1504.06765](https://arxiv.org/abs/1504.06765), Numerical Algorithms DOI [10.1007/s11075-016-0250-4](https://doi.org/10.1007/s11075-016-0250-4). Retrieved through SciSpace and independently opened on arXiv. It distinguishes discretization, data and round-off contributions. Its round-off scaling is not used as a worst-case floating-point enclosure in this audit.
4. Calvo, Higham, Montijano and Randez, *Global error estimation with adaptive explicit Runge–Kutta methods*, [author PDF](https://webhomes.maths.ed.ac.uk/~dhigham/Publications/P23.pdf). Exa retrieved the adaptive-tolerance analysis. Explicit Runge–Kutta assumptions are not transferred to RODAS5P without a new proof.

## Wolfram check

Wolfram Language evaluator, 2026-10-21, actual query:

```wolfram
{FullSimplify[(Exp[z/2]-1)/z /. z -> 4 Pi I],
 FullSimplify[1/(2(1-gamma z)) /. z -> 4 Pi I],
 Exponent[Expand[(a0+a1 t+a2 t^2+a3 t^3+a4 t^4)^3],t],
 FullSimplify[Abs[1-4/5]]}
```

Actual output: `{0, I/(2*I + 8*gamma*Pi), 12, 1/5}`.

This checks (i) the oscillatory counterexample to the claimed extension of the negative-real-axis T3 ratio, (ii) degree 12 rather than degree 8 for a cubic RHS composed with a quartic curve, and (iii) the coefficient 0.2 in the proposed B1d fixture. The counterexample has a real two-dimensional representation with a skew-symmetric Jacobian, so it applies to the real ODE setting. Proofs and corrected constructive statements are in the node reviews. CAS output is a cross-check, not a replacement for those proofs.

## Evidence reuse

Published A01–A12 results are identified by their original commit and files. Unchanged proofs are reused. Ported code is tested at the new integrated source; old source-bound execution receipts are not relabeled as executions of this branch. The 2026-10-21 benchmark campaign is independently re-summarized without repeating its expensive measurements or changing its original INCONCLUSIVE verdict.
