# Exact normalization correction: A_E[16,18] (2026-10-30)

Object: original BASS21 rank-nine-conditioned fixed-screen E_l0+ high multipole E shear operator, Draft PR #145, scope source-frozen 50 original cases ×193 saved epochs, original owner `cosmosapjw-quantum/bass@cc602e209e7deeb511ab2a1f3b2268d55206c43c`.

## Observed defect (human-readable analytical simplification only)

The first science report and PR body incorrectly displayed
`A_E[16,18] = 360 sqrt(7315)/2849 ≈ 10.8073002373`.

The correct source-defined normalization is `k_l = sqrt[4pi/(2l+1) * (l+2)!/(l-2)!]` and `A_E[l,m]=(k_l/k_m)*A_c[l,m]`. Since `A_c[16,18]=3420/259` exactly,

```
k16/k18 = 2 sqrt(37/209)
A_E[16,18] = (3420/259)*2 sqrt(37/209)
           = (360/7) sqrt(19/407)
           = 360 sqrt(7733)/2849
           = 11.1117908052872525736...
```

These expressions were checked symbolically by independent Wolfram Language algebra and with SymPy's **exact factorial ratio** in `tests/test_highrank.py`, including a negative test asserting that the old radicand `7315` is not equivalent. The actual original `code/highrank_atlas.py` already constructs `k` from correct exact factorials and computes `A_E=diag(k) A_c diag(k)^(-1)` numerically. The computed `A_E[16,18] = 11.111790805287251` and resulting 50×193 science NPZ, high-l coefficients and physical shear flux data were **not** affected by the wrong explanatory expression. This is not evidence of newly recomputed original IVPs.

## Scope, archival repair and classification

Kind: `theory/presentation_equation_error` in a human-written coefficient reduction, **not an implementation or a numerical physics error**. Correction applied in `reports/PHYSICS_AND_HOSTILE_AUDIT_KO.md` and the original report generator `code/build_report.py`; additionally an exact negative/positive test is now included. All previous unmodified source-full archive bytes remain immutable historical evidence and are superseded for human-readable coefficient reporting by the *separately named* corrected source-full ZIP. The Git branch is amended only by a new fast-forward descendant commit with append-only ledger L-0201. Old ledger L-0200 and preregistered source remains untouched. Formal external different-author review still NOT_RUN, global authority CI still separately gated, official VigilODE DAG 7/23 and successor 0/12 unchanged, production HOLD.
