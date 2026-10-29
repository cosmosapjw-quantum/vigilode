# VigilODE N02 closeout + N03 theory/code research progress (2026-10-29)

## Previous interrupted unit actually closed

PR110 Draft native right FGMRES latest parent source `c15c34d52f09689bd37400443c1f7e328fc51729` had a later verified focused native test 3/3 PASS and strict all-target Clippy PASS (runs 40836690956, 40837168008 and 40837408483). The final current-view receipt was newly GitHub-published at `6c99864b9826070adfaf4da759efa5328f54f45a`; the former source ZIP is 477185 bytes SHA256 d07af6e280a6d23a91abde8719bdddfc8ecc6c84544d6698005c16f2b3675e39, ZIP CRC verified in this run. Prior full PR110 Production CI and authority remain FAIL: native focused PASS does not imply production PASS. N02 16/20 scoped; original VigilODE 7/23, REI central 28/88, successor 0/12 admitted.

## New independent branch

PR112 Draft from N02 terminal commit, isolated `research/n03-physical-interval-stage-certificate-20261029`. True prereg before all N03 scientific experiments: `d90f28dc3f6465882e8ce294a78adfdf9cfb96f4`. New additive native `stage_interval_certificate.rs`, tests `n03_stage_interval_contracts.rs`, bridge tests `n03_n02_guard_integration.rs`, plus explicit `lib.rs` export and dedicated focused CI. No main/default routing modified. The supported claim is source-conditional across the supplied full input box, NOT that the box contains a BASS/REI physical source.

## Actually executed (local)

- Rust1.94.1 restored from provided source archive, locally test `cargo test --locked --offline --all-targets`: 11/11 PASS exit0, `cargo fmt --all -- --check`: exit0, strict `cargo clippy --locked --offline --all-targets -- -D warnings`: exit0.
- Exact Python rational-from-binary64 independent check `python3 oracle_fraction.py`: 768/768 over 64 cases × 4 corners, exit0. Maximum exact corner physical error 0.0060357995215090205, maximum certificate upper 0.006929561638300846 (global maxima, not paired ratio). Interval bound was never below any tested exact case. Maximum represented original residual upper ~0.004325054767655297.
- Wolfram exact 2x2: mu_up=-31/32, Cwp=2, K=128/159, max exact residual=235/20992, max exact physical error=1645/107502 and upper=235/13038, inequality True.
- Explicit first local errors preserved: `cargo test` lexical `-.5/.5` 2 errors (exit101) and strict Clippy `should_implement_trait` 4 errors; repaired by explicit 0.5 and `add_out/sub_out/mul_out/negated`, not tolerance adjustment. An initial repair shell Python syntax error was execution-environment/user-script error, not a physical defect.

## Native CI and unclosed checks

- First PR112 native run `40853674030` Rustfmt FAIL. Three native paths were repaired using the exact observed rustfmt diffs, Git commits `b80205fd`, `86dbadd8`, `5e204c09`; each full blob readback matches content. This is formatting, no algorithm changes.
- Second PR112 native focused run `40854021844`: Rustfmt PASS, native interval and N02-monitor bridge contract test step PASS; strict Clippy and overall conclusion must be read at final HEAD before being promoted. Existing broad CI failures remain separate.

## Source audit and non-import

Sibling PR111 (`2357135220...`) proves nonlinear preconditioner reapplication in monitored GMRES can produce wrong candidate; PR110 stored flexible Z. PR108 affine-cell theorem is conditional on whole-slab containment; PR265 REI/BASS finds source inventory mismatches, so no source-owner physical certificate is imported. BASS CE1L PR251 also retains source-identity repair requirements. Current unit is bounded point/interval stage, not full30/physical33/accepted quartic, nonlinear whole-slab, continuous trajectory, observational inference or production speedup.

## DAG and gates

Original VigilODE 7/23, central REI 28/88, 12-node successor 0/12 admitted; PREFIX01/02 auxiliary 0/1. N03 scoped 17/20 only IF focused native CI + source readback pass; otherwise completed count remains lower. Independent R6/C9 absent and C7 actual physical source membership BLOCKED_SOURCE_MISMATCH. Original Q-LEDGER, Q-API, Q-GIVENS, 8 suites/10 interactions, negative evidence, no main merge, no force push, no production promotion.
