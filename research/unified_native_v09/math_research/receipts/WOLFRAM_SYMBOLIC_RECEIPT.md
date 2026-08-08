# WOLFRAM SYMBOLIC RECEIPT

DATE: 2026-08-08

Exact Wolfram Language evaluation used the three-stage fifth-order Radau IIA tableau and verified:

```text
A . 1 == c                                      True
last row(A) == b                                True
series R(z)-exp(z), coefficients z^0..z^5       all zero
first nonzero coefficient at z^6                1/7200
limit R(z), z->infinity                         0
BDF1 R(z)                                       1/(1-z)
BDF2 root polynomial                            (3-2z) xi^2 - 4 xi + 1
BDF2 roots at z=0                               {1, 1/3}
BDF2 roots as z->-infinity                      {0,0}
```

Radau stability function:

\[
R(z)=\frac{60+24z+3z^2}{60-36z+9z^2-z^3}.
\]

The kernel emitted nonfatal precision warnings while simplifying radical row sums, but all exact Boolean identities and rational series coefficients were returned. No numerical approximation was used for the reported identities.
