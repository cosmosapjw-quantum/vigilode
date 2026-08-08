# Task 2 root-cause receipt

The first attempted frozen false-accept case used a v0.6 replay row whose oracle WRMS was
`6.54e-11` and therefore represented floating-point projection noise rather than a robust
nonlinear underestimation case. In the current debug build the candidate and protected
sequential output were bit-identical, so the expected ratio was not reproducible.

The replacement case is the deterministic manufactured-vector configuration
`mv-s10000-m1000-eta0.9-h1e-3`, `theta=0`, `q=0`, three path rounds, AB2 predictor, one
correction. It reproduces a scientifically material first-correction underestimate:

- first projected correction WRMS: about `2.39672e5`;
- protected sequential-oracle output WRMS: about `3.38646e5`;
- ratio: about `1.413`;
- residual after first correction: about `1.40315e-2`;
- residual after second refreshed-Jacobian correction: about `4.06e-14`.

The refined-root default correction tolerance was initially `1e-11` WRMS. The target residual
already satisfied the strict relative tolerance, but roundoff-level correction noise stabilized
near `7.3e-9` WRMS, causing a false nonconvergence after ten iterations. The default reference
correction tolerance was therefore changed to `1e-8`, still far below a unit WRMS error budget.
