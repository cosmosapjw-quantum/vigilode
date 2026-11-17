# Node H1: Hysteresis Band in `route_registry` after Log-Work-Ratio Bias Measurement
- written: 2026-11-04 (committed and pushed before the first run)
- question: Does an open integer hysteresis band $(\rho_e, \rho_r)$ with width $w = -\ln\rho_e - \ln\rho_r > 0$ satisfy the Theorem-H unbiasedness prerequisite $|b| < w/2$ on the `transition-39` and `transition-79` regimes (where $b = \mathbb{E}[\ln\hat\Phi - \ln\Phi]$ is the log-work-ratio prefix bias) and strictly reduce switches on break-even regimes without losing any registered entry/return decision?
- arms: `strict_1_1` (`entry_margin = None, return_margin = None`, width $w = 0$), `band_9_10` ($\rho_e = \rho_r = 9/10$, $w = -2\ln(0.9) \approx 0.2107$), `band_4_5` ($\rho_e = \rho_r = 4/5$, $w = -2\ln(0.8) \approx 0.4463$), `band_3_4` ($\rho_e = \rho_r = 3/4$, $w = -2\ln(0.75) \approx 0.5754$); everything else identical
- cells: `transition-39`, `transition-79`, `stiff-forced`, `rotation`, and a break-even relaxation family ($\lambda \in \{-350, -400, -450\}$)
- command: `python3 research/h1_hysteresis_band_20261104/eval_h1.py`
- inputs: `research/h1_hysteresis_band_20261104/PREREGISTRATION.md`, `research/h1_hysteresis_band_20261104/eval_h1.py`, `crates/vigilode/src/margin.rs`, `crates/vigilode/src/switching.rs`
- metrics: deterministic `WorkCounters` (`rhs_calls`, `jvp_vectors`, `jacobian_builds`, `switches`), log-work-ratio prefix bias $b = \ln(W_{\text{challenger,prefix}} / W_{\text{incumbent,prefix}}) - \ln(W_{\text{challenger,window}} / W_{\text{incumbent,window}})$, and Theorem-H switch bound $1 + \text{TV}(\ln\hat\Phi)/w$; wall time diagnostic only
- PASS gate: (1) $|b| < w/2$ on `transition-39` and `transition-79` for a candidate band; (2) switches on the break-even family $\le 1 + \text{TV}(\ln\hat\Phi)/w$ with a strict reduction vs `strict_1_1`; (3) zero registered entry/return decisions lost across `stiff-forced`, `rotation`, `transition-39`, and `transition-79`
- KILL gate: $|b| \ge w/2$ for every candidate band on `transition-39`/`transition-79`, or no reduction of switches on the break-even family, or any registered decision lost
- holdout: none (sealed holdout profiles `holdout-512`, `enforced-budget-holdout-320`, `stage-growth-holdout-384` and `tools/reference_v2/artifacts/*-holdout-v2.json` are not executed or inspected)
- analysis: `research/h1_hysteresis_band_20261104/eval_h1.py`; no threshold chosen after seeing results

## Results (appended after the run; nothing above this line changes)

- **Execution Commit (`EXEC_SHA`)**: `4339facc62258b5fb9a2469f16a3af8b93763e24`
- **Verdict**: `FAIL` (`entry_margin = None` and `return_margin = None` retained as defaults in `route_registry`; hysteresis band remains opt-in)
- **Summary**: On `transition-39`, the log-work-ratio prefix bias is $|b| = 1.2069 \ge w/2$ across all candidate bands ($\rho \in \{9/10, 4/5, 3/4\}$), and any entry margin $\rho_e \le 9/10$ blocks the `transition-39` entry (`185 * 10 = 1850 >= 192 * 9 = 1728`). Therefore `entry_margin` and `return_margin` remain `None` by default in `SwitchingConfig::route_registry`.

