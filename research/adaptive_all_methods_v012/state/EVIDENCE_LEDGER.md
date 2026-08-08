# Evidence Ledger

| ID | Claim | Status | Source/evidence |
|---|---|---|---|
| E1 | CVODE BDF is variable-step/order 1–5 and uses history-dependent error/order logic | SUPPORTED | SUNDIALS CVODE mathematics |
| E2 | RADAU5 uses adaptive error control and Gustafsson prediction | SUPPORTED | Hairer–Wanner `radau5.f` |
| E3 | RODAS5P has a 5(4) embedded structure | SUPPORTED | Steinebach 2023 |
| E4 | Adaptive SDC can control both step size and sweeps | SUPPORTED | Saupe et al. 2024/2025 |
| E5 | Peer methods need variable-step theory/order conditions | SUPPORTED | Schmitt–Weiner; Schneider–Lang–Weiner |
| E6 | Exponential/iterative methods benefit from cost-aware control | SUPPORTED | Deka–Einkemmer 2022; Einkemmer 2018 |
| E7 | Step doubling is efficient for every family | CONTESTED | Asymptotically valid but 3-step work and history cost |
| E8 | Homotopy first correction is a bound | UNSUPPORTED | Prior false-accept corpus |
