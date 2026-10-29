# PREFIX02 native trigger bridge: scoped exploratory checkpoint

**After-experiment documentation, NOT a preregistration and NOT production integration.** Date 2026-10-29. Status `EXPLORATORY`; promotion `HOLD`.

## Sources and scientific scope
- Baseline PR106 SHA `c6cba2b62629e819dd163325c7c9fb9bb5e3080c`.
- PR104 native `crates/rodas5p-krylov/src/gmres_into.rs` source commit `10854bc0bff81b6d3cf5cb0a44520ed9165c205f`, blob `90bfaaa62afeda9dec27b3dfd594e67cd3034da1` (identical at PR106).
- PR103 chemical6 source `1845e1d8bdfda222bf77637e8d38d5076743f238` is a separate signed-lognorm synthetic rational box, not physical33/full-IVP.
- Inherited core DAG **7/23**, REI **28/88**, PREFIX01 auxiliary **0/1**, PREFIX02 auxiliary **0/1**, prior PREFIX02 R/C scoped **18/20** with native-independent R6,C9 `NOT_RUN`. No DAG completion advanced.
- The existing PR105 missing preregistration and root authority failures remain, as do Q-LEDGER, Q-API, Q-GIVENS, 8 suites and 10 interactions.

## Source audit (read native GitHub code)
`solve_gmres_into_right_monitored` calculates a least-squares prefix correction, transforms the correction with the right preconditioner, creates a candidate and calls its monitor **at each Arnoldi column**. A wrapper over that entry point cannot implement cost-preserving trigger-before-backsolve. Native `selected_residual_norm` uses WRMS with `residual_scale` and L2 otherwise; PREFIX02 stage error theorem uses weighted infinity norm. If weights match, `WRMS <= max <= sqrt(n)*WRMS`; do not silently identify these norms. The current `Preconditioner` trait does not assert linearity, invertibility or stationarity: fixed preconditioner assumptions must be separately enforced, or use flexible GMRES with stored Z.

## Conditional original-residual contract
For fixed `A=I-sigma J` with `sigma>=0`, positive `w`, `mu_w(J)=max_i(Jii + sum_{j!=i}|Jij|w_j/w_i)` and `sigma*mu_w(J)<1`, `K=1/(1-sigma*mu_w(J))` bounds the inverse in weighted infinity norm. Right and left preconditioned projected residuals are triggers, never admission. Accept only when outward original `rho>=||b-A xhat||_{w,inf}`, outward `K_up>=K`, and `rho<=tau_orig` AND `K_up*rho<=epsilon_stage`. Operator callback evaluation error, physical source/clock/epoch/norm, current PR104 WRMS tolerance and output transaction have independent owners.

## Local experiment (source NOT in GitHub; downloadable separately in chat artifact)
Rust 1.94.1 standalone, no external dependencies, dense diagonal preconditioner. It implements opt-in-style incremental Givens projected trigger before prefix backsolve, original floating-residual/externally supplied stage gate, warm starts, restart, side left/right, epoch callback simulation, transactional output. **It is not patched into PR104 native code and is not mathematically outward-certified.**

Fresh `cargo test --locked --offline --all-targets --quiet` => exit0, 13 PASS; `cargo fmt --all -- --check` exit0; `cargo clippy --locked --offline --all-targets -- -D warnings` exit0. Synthetic dense linear solves, seven paired ABBA/BAAB-style blocks, n=16/64/128, total 630 solves. Median time ratios identity/right: 3.0472/5.7997/7.7570 (not achieved-error matched). Strict matched-output **negative-cost** control ratios every-prefix/trigger-only: **1.4963/1.8253/1.8245**; identical Arnoldi column counts, output and achieved errors; backsolves 9/16/21 versus 1/1/1. No full-IVP claim.

Independent Fraction-from-binary64 2x2 rational oracle first found `2.0/3.0` rounded **below** exact `K=2/3`, 7/8 PASS, exit1; replacing the supplied bound in probe with `(2.0_f64/3.0).next_up()` led to 8/8 PASS, exit0. No tolerance expansion. This does not certify the standalone solve's floating residual.

## Next actual native work (not yet executed)
Isolated worktree from audited SHA, separate opt-in path inside `gmres_into.rs` retaining default dispatcher and public API unchanged; perform incremental Givens update **before** candidate construction, reconstruct candidate and original residual only on trigger/happy breakdown/restart as required. Preserve source identity and `WorkCounters` / `GmresMonitoredProgress` failure accounting. Require independently justified physical33 norm transport, operator error enclosure, zero-output-mutation on rejected check, callback failure, stale source, singular/nonstationary PC, restart and warm start. Run native Rust tests, independent original-residual oracle and matched-achieved-error full-IVP ABBA/BAAB with negative-cost control.

**Never main merge, force push or production promotion.** Native tests, native source implementation, full-IVP benchmark, independent R6/C9 review and newly acknowledged Drive/Dropbox backups: `NOT_RUN`. This commit is only a durable research checkpoint.