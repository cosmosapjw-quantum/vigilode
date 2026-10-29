# Native N01 represented-point research slice: 2026-10-29

VERDICT = PASS_SCOPED_POINT_SLICE; N01_PHYSICAL_ADMISSION = OPEN; PRODUCTION = HOLD.

## Exact source and change scope
Parent PR111 HEAD 23571352200b3919a008327c294dc3bc0f371156.
Native GMRES file crates/rodas5p-krylov/src/gmres_into.rs unchanged, pinned blob 90bfaaa62afeda9dec27b3dfd594e67cd3034da1.
Added opt-in module crates/rodas5p-core/src/n01_source_owner.rs Git blob 60166cbb8edea54d71f38cc14ea5c8976c650c7e, identical to locally tested file. Only other Rust modification is public mod declaration in core/src/lib.rs. No GMRES route or default behavior changed.

## Mathematical scope and implementation
Exact represented dense J0, b0, nonnegative sigma and strictly positive w,p. A0=I-sigma J0. Weighted infinity norm |v|_w=max_i |v_i|/w_i. Lognorm mu_w(J0)=max_i[J_ii + sum_(j!=i) |J_ij|w_j/w_i].
If 1-sigma*mu_w(J0)>0, stage bound is |xhat-x0|_p <= Cwp*rho0 / (1-sigma mu_w(J0)), where rho0=|b0-A0*xhat|_w and Cwp=max_i(w_i/p_i). All calculated upper bounds use the native crate directed Interval/add_up/div_up/mul_up/sub_down operations. Both original-residual and physical-stage-error upper bounds must pass before output is committed.
Source ID, clock bit pattern, epoch, J0, b0, sigma, w,p are snapshotted; fresh inputs compared by represented bits. A forged matching source ID is still possible because the physical provider does not own/authenticate it. Generic JVP uncertainty returns N01_JVP_UNCERTAINTY_OWNER_NOT_VERIFIED.

## Fresh evidence
Current session Rust1.94.1 interface mirror: cargo test --locked --offline --all-targets 7/7 PASS, cargo clippy --locked --offline --all-targets -- -D warnings PASS, cargo fmt --all -- --check PASS. Earlier fixture failure (incorrect positive-margin assumption) and initial Clippy style errors corrected without tolerance weakening.
256 exact binary64 2x2 nonnormal candidate records; independent Python Fraction.from_float oracle verifies eight inequalities per record, 2048/2048 PASS. Negative stale source/clock/epoch/J0/b0/norm, nonfinite forged clock, NaN, zero weights, nonpositive resolvent margin, unverified JVP, physical gate rejection and unchanged output pass scoped tests.
Residual plot shows near-coincident exact vs outward upper; physical p-bound conservative because w=(100,1), p=(1,1), Cwp=100.

## Unverified
This is NOT an actual full native repository build or native cross-crate integration test: container cannot resolve github.com, so connector source publishing/readback and standalone interface-mirror compilation are distinct. Real physical JVP error deltaJ, RHS delta_b and callback eta_apply are NOT certified; whole-slab, full-IVP, source-owned epoch and stage budget remain OPEN. Full native IVP, independent R6/C9, Q-API/Q-GIVENS/Q-LEDGER and eight suites/ten interactions NOT_RUN. No N01 timing loop used; original N00F budget 1/3 remains. N02 opt-in Givens not attempted.

## DAG
Original VigilODE 7/23 unchanged; central REI 28/88 unchanged; successor 0/12 admitted; N00F 1/1 scoped audit; native N01 physical admission OPEN. Production HOLD.

## Single next action
Connect a real owner to point J0,b0, source, clock, epoch, norm and outward deltaJ/db/eta_apply; implement full robust physical bound with original/stage gate and transactional failure. Run native tests, independent exact oracle, R6/C9, and only then N02. No main merge, force push, tolerance drift or production promotion.
