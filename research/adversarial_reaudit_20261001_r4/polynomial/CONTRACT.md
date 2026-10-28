# R4 polynomial / arithmetic contract

Audited source: `1c54194123ee6abc6daa512e8574922f510b4e2c`. Prior package: R3 at `6598abd0db790162ab26f50bc13943f24a0c7e4a`.

Read-only production audit and standalone probes. No production edits. Scientific execution waits for root's `PREREG_COMMITTED` message. Claims remain exploratory until independent decision review.

Questions and fixed invariants:
1. Do original R3 lost-weight witnesses reject or carry a non-authoritative report? Replay positive/negative h nilpotent witness and an A=0 control.
2. Does public `bound_transform_error(A,h,b,stored)` bound the actual transformation error for exact scalar/zero/nilpotent inputs, including stored sign mismatch at order zero and high phi order? Fraction/Decimal independently evaluate represented inputs; zero tolerance must never admit a nonzero exact error.
3. Do Chebyshev `Certified` outputs enclose independent exact-input scalar/diagonal/symmetric 2x2 Decimal-oracle errors? Inputs use h in {0,1e-12,.1,1,10}, matrices zero/scalar/diag(-1,-2)/[[-2,1],[1,-2]], ordinary and cancellation vectors; Laguerre must disclose EstimateOnly outside scalar branches. At most 100 native rows.
4. Are unsupported/non-normal/nonpositive/spectral-domain inputs rejected or explicitly EstimateOnly? Cache reuse must bind operator/enclosure/h/basis/degree, and total certificate is an absolute error bound, distinct from truncation-only budget.
5. Construct and verify a scale-safe upper 2-norm primitive using exact powers-of-two normalization and outward arithmetic, or a corrected scalar transform-error expression if a real counterexample is found. Candidate remains standalone.

Commands after preregistration: `cargo run --offline --manifest-path r4/polynomial/probe/Cargo.toml` under the shared Cargo lock; `python3 r4/polynomial/oracle.py`. Final reproducibility runner uses relative repository dependency paths once copied into the audit node. Record stdout/stderr, exit code, exact source HEAD, binary64 input bits, and JSON raw results.

Acceptance: each production Certified total bound encloses oracle error; every claimed transform bound encloses exact error. Original witness must no longer claim convergence. The report distinguishes safe rejection/known research limitation from a new false bound or false convergence. Numerical oracle uses Decimal >=120 digits (Fractions for algebraic values); native tested Rust and analytical derivation are separate evidence.

Budget: <=100 native probe rows, one native build/run plus a bounded corrective rerun only for harness errors, one independent oracle run; no wall-time speed promotion. Stop after concrete findings and candidate checks or explicit runtime blocker.

Pre-run source-review addition, authorized by root before preregistration commit: A=[[0,0.25],[0,0]], h=1000.1, b2=[0,1] and other b_k zero, stored weights produced by weight_phi_vectors. Exact nilpotent Fraction oracle checks the matrix-norm max treatment of zero vs positive subunit ExpBound. No scientific outcomes were observed before adding this case. Planned native rows: 78.
