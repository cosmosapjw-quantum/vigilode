# Phase 9 — Survivor Formalization

## Definition 1 — Adaptive policy

An adaptive policy is the tuple

\[
\mathcal P=(\mathcal E,k,s,f_{\min},f_{\max},f_{\rm rej},h_{\min},h_{\max}),
\]

where \(\mathcal E\) is a dimensionless error estimator, \(k\) its asymptotic order, \(s\) a safety factor, and the remaining quantities bound step changes.

## Definition 2 — Transactional adaptive step

A trial is transactional if state, multistep/stage history, recycle space, and output-observer position are committed iff the candidate is finite and \(\mathcal E\le1\). Trial work is always retained in the work ledger.

## Definition 3 — Universal reference lane

For a method of global order \(p\), the universal lane uses one coarse step and two half steps, accepts the fine state, and estimates

\[
\mathcal E=
\left\|
\frac{y_{h/2,h/2}-y_h}{(2^p-1)s}
\right\|_{\rm RMS}.
\]

## Definition 4 — Native lane

A native lane may replace step doubling only when its estimator order and validity range are documented and verified by external global-error holdout.

## Implementation survivor

1. Common I/PI controller and adaptive result diagnostics.
2. Existing native RODAS controller refactored onto the common contract.
3. Variable-step BDF2 + step-doubling adaptive BDF1/2.
4. Step-doubling adaptive Radau IIA1/3.
5. Native adaptive homotopy integration with protected sequential fallback.
6. Unified common-grid adaptive benchmark.
