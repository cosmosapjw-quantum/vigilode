Locked offline release, remote x86_64, polynomial-events. RED at RED_SHA intentionally fails; initial actual portfolio at EXEC_SHA, final at FINAL_EXEC_SHA. Input hashes bound separately to each commit; initial and gather compile-failure source files subsequently changed. Use isolated checkout.
```sh
export CARGO_TARGET_DIR="$HOME/.cargo-targets/vigilode-stage-lattice-20261124/kschain"
taskset -c 0 cargo test --locked --offline --release -p vigilode --features polynomial-events --test spatial_adjoint_contracts -- --nocapture --test-threads=1
cargo test --locked --offline --release -p vigilode --features polynomial-events --test spatial_diffusion --test spatial_separable --test spatial_upwind --test spatial_portfolio --test longitude_preconditioner --test explicit_contracts --test explicit_hardening_contracts -- --nocapture --test-threads=1
```
Capture initial campaign portfolio.log at EXEC_SHA and final_portfolio.log at FINAL_EXEC_SHA; regression.log final revision. verify.py independent Fraction on initial portfolio; verify_gather.py checks final replay and exact independent gather; guide.py generate/select and gather_guide.py generate/select validate package and select finite measured rows. Raw or gzip accepted; timing not expected bitwise. Use logs at exact recorded commits. No new production default or continuum claim.
