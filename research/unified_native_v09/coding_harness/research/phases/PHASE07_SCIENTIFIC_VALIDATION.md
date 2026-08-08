# Phase 7 — Scientific Validation

| Check | Status | Evidence |
|---|---|---|
| BDF1 definition and order 1 | PASS | observed order `0.986790599249055`; exact scalar amplification test |
| BDF2 startup and order 2 | PASS | explicit BDF1 startup; observed order `2.02189344651616` |
| BDF zero-stability/stiff limit | PASS within fixed-step anchor | symbolic roots `1,1/3`; numerical stiff amplification `0.00032128634084745` |
| Radau IIA1 = backward Euler | PASS | cross-method one-step test |
| Radau IIA3 tableau identities | PASS | `A 1=c`, last row=`b` |
| Radau IIA3 fifth order | PASS | observed order `4.99283017895308` |
| Radau stiff decay | PASS on scalar negative-real gate | amplification `7.15769262853094e-06` |
| Nonsingular mass-matrix path | PASS | Radau3 L2 error `5.55111512312578e-17`; BDF2 `1.96408446209025e-06` |
| Units/dimensions | PASS in declared regime | every BDF/Radau residual term has units of `M y`; solver algebra otherwise unit-agnostic |
| Singular-mass/index-1 DAE | NOT_TESTED / OUT OF SCOPE | no production DAE claim |
| Adaptive global error control | NOT_TESTED / OUT OF SCOPE | fixed-step anchors only |
| Rigorous nonlinear output certificate | CONCERN / DEFERRED | common Newton residual is operational convergence, not a Krawczyk/Kantorovich output bound |
| Protected RODAS/SABR/homotopy regression | PASS | full release workspace suite |

## Interpretation

The four native methods are correctness anchors, not production competitors. Their scientific eligibility in the unified document means only that the fixed-step order, scalar stiff-decay and nonsingular mass-matrix gates passed. They remain `HOLD` because no matched global-error/total-cost performance assessment exists yet.
