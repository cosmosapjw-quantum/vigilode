# RUN_STATE

Date: 2026-08-07
Branch: `homotopy-numerical-experiments-v0.5`
Base: `ce1cc51034186958da3e27f641f29aae777ceaf1`
Layer: cycle-1 closeout

Implemented fixed-schedule nonlinear partial-coupling experiments and completed the canonical
scientific screen. Canonical result SHA-256:
`a665562e3088950e3a7d64e985f220708cac6cad7356f096baf378e4ee27026e`.

Current evidence:

- 57 cases, 8,208 candidate rows, 114 controls;
- 0 numerical failures and 0 one-step false accepts;
- 401 low-depth fast rows across 16 cases;
- global-order screen rejects the fixed `E_out <= 0.1` low-depth policy;
- no candidate dominates sequential or SABR even in RHS and JVP scalar-work metrics.

Independent base-to-head review and the full exact-tree/clean-archive gate are complete. The
reference layer is promoted; the low-depth fixed-budget method and every speedup claim remain HOLD.
The next active research cycle is order-aware output-certificate policy screening.
