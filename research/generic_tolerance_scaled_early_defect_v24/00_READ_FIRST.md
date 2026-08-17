# P1-00 tolerance-scaled early-defect preflight

This node tests a tolerance-aware, read-only stage-1 defect score only. It does not implement early abort or method switching.

The sealed score uses the repository controller's component scale *form* with the already-computed stage state `U2`:

`rho_c2_wrms = sqrt(mean_i((abs(h)*D2_i/(atol + rtol*max(abs(y_n_i),abs(U2_i))))^2))`.

`eta_c2` remains an unchanged control. No threshold may be selected or recommended in this node.
