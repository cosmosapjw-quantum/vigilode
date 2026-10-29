# CMODE01: theory-first / coding-second execution and benchmark report (2026-10-29)

**Parent:** PR106 `c6cba2b62629e819dd163325c7c9fb9bb5e3080c`. Prereg: `0a8737e94319829c574fb5d954440b5de5931054`. Existing original DAG **7/23 inherited scoped, delta 0**, 34 edges unchanged. REI central **28/88 unchanged**. Auxiliary stages **8/10 scoped**; native/independent promotion **0/1**; new successor proposal **0/12 admitted**; production **HOLD**.

### Executed source scope
Standalone dependency-free Rust 1.94.1; `J(t)=(1+t/4)J0`, nonnormal 2x2 repeated blocks, fixed implicit midpoint, 1024 steps on [0,1], n=128/512. Identical physical norm, original-residual/stage budgets 1e-10 and identical stage arithmetic. Baseline computes outward mu on every step; candidate caches 8 whole-interval dyadic-cell upper bounds. Both recompute outward original residual and check physically converted error before transactional acceptance. The analytic model is `y(1)=exp((9/8)J0)y(0)`; it is not an actual BASS chemical client.

### Implementation validation, locally executed
`cargo test --locked --offline --all-targets`: 8 tests PASS.
`cargo clippy --locked --offline --all-targets -- -D warnings`: PASS.
`cargo fmt --all -- --check`: PASS.
Independent Python Fraction.from_float oracle: 560/560 exact inequalities across 40 cases PASS.
Independent SciPy analytic reference: 256 trajectories, maximum achieved endpoint absolute error 2.0699365877591136e-8; scan/cache output bit-identical. **No rigorous global error enclosure**.

### Light benchmark and algorithmic profile, one optimization iteration (1/3)
Seven paired alternating ABBA/BAAB blocks per dimension, same machine/source/compiler/controller/outputs and tolerance. Both arms run entire 1024-step IVP, inclusive of checks, construction, and drops.
- n=128: median scan/cache=**1.1823386213**; block range [1.147380,1.246677]; scan=14.9916 ms, cache=12.6004 ms; lognorm row scans 131072 -> 2048, eight cell builds/1016 cache hits.
- n=512: median scan/cache=**1.1787810814**; block range [1.168847,1.193995]; scan=59.3301 ms, cache=50.4266 ms; lognorm row scans 524288 -> 8192.
Reduced lognorm work 64x, net synthetic IVP time about 1.18x. Measurements are exploratory same-host paired times, not confidence intervals or native solver/physics gain. One benchmark loop; no post-measure algorithm tuning.

### Plot-based adversarial audit and failure ledger
Read `speed_ratio.png`: all 14 ratios >1, n128 more variable. `profile_counts.png` counts only row evals, not total CPU. `analytic_errors.png`: ~2.05–2.07e-8 achieved errors, same across arms. `lognorm.png`: weighted vs unweighted counterexample. No reference-ratio improvement claim beyond fixed synthetic model.

P0: external bound provenance/physical source ownership; JVP uncertainty; nonlinear whole-slab guard and global certified IVP error. P1: native adaptive GMRES integration, negative fallback costs and actual physical full30/physical33; unrelated Q-LEDGER/Q-API/Q-GIVENS/8 suites/10 interactions and authority CI remain OPEN. First compile work-counter mismatch and initial strict-Clippy diagnostics were fixed before benchmark, preserved locally. No main/root solver mutation, no merge, no force push, no fitting/release claim.

### Evidence and backup status
Local source/evidence/plots ZIP `VIGILODE_CMODE01_20261029_COMPLETE.zip`, SHA256 `00da49ecb37b0a316a40f35c4d62e3d43590240bcdd6ed07d6244a73ac89144a`. Repro source TAR `CMODE01_SOURCE_20261029.tar.gz`, SHA256 `b175eae4ade425923e18deff9acf1cee3c7ccccd31335d230a6614000db475ee`. Github files and any source archive require exact GitHub readback; new Drive/Dropbox ACK not asserted without upload.
