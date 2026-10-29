# Native coupled-step research: source-bound results

Start with [CRITICAL_REVIEW_KO.md](CRITICAL_REVIEW_KO.md), [companion API](../../crates/vigilode-coupled/README.md), and [REPRODUCE.md](REPRODUCE.md).

|Artifact|Purpose|
|---|---|
|[COST.csv](runs/39349031422/COST.csv)|All80case/arm configurations, including declined costs|
|[Generated report](runs/39349031422/REPORT.md)|Actual same-binary fixed-step latency and64paired comparisons|
|[Raw observation run](runs/39349031422)|Full per-repeat times, content-addressed states, independent references and hashes|
|[Numerical audit](runs/39349031422/AUDIT.json)|847metrics,770timing identities,30worker-bit comparisons,61paired medians|
|[Test log](runs/39349031422/TESTS.log)|19actual Rust tests passed; root workspace not claimed tested|
|[Root vs time error](closeout_runs/39349959195/ROOT_VS_TIME_ERROR.csv)|77root-reference admissions but only20also pass embedded LTE at the fixed h|
|[Archive/source validation](closeout_runs/39349959195/VALIDATION.json)|112hashes verified,1large reference reconstructed, existing numerical crates unchanged|
|[CodeRabbit status](closeout_runs/39349959195/CODERABBIT_STATUS.json)|CLIinstalled; authentication/login blocked; no completed review and issues=null|

Base numerical source: `c03ca90f19a9e64ba7751c069325a48b54b39c2e`.
Measured native companion and harness source: `1e4a993874d6930ea2bb41bb0ff7392f21c33df7`.
Measurement run39349031422. Frozen closeout39349959195. Later report commits do not relabel the measured binary.

The original supplied prototype's SHA256 is recorded in PLAN.md. No full trajectory, global optimality, universal speedup, strongest-baseline, or certified-output claim. The new crate is standalone and opt-in. PR71, PR73 and existing defaults remain untouched.
