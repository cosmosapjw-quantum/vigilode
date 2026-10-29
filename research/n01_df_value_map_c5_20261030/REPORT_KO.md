# VigilODE N01 derivative-free C5: 소스 귀속 value-only RK stage

2026-10-30 KST. Source authority: PR117 head `0032b7bd7a0d5092eac26ad9408b9a261df2916e`; new Draft PR119, branch `research/n01-df-value-map-c5-20261030`. Original source is unchanged on `main`. This is a post-execution report, not prospective preregistration. First durable PR119 contract was published after first local experiments.

## 핵심 구현

원래 모토인 causal, parallel, derivative/matrix-free augmented Rosenbrock/RK and accelerated homotopy Newton-free root finding을 보존한다. PR91의 `rhs_homotopy::RhsOnlyMap`과 `solve_rhs_homotopy`를 **그대로 사용**하며 새로운 algorithm implementation을 대체하지 않는다.

`crates/rodas5p-core/src/n01_mass_action_owner.rs`의 새 read-only accessor가 PR115의 opaque trial을 재검사한 뒤 정확한 represented `k`, frozen live carrier `y`, physical `w,p`를 공급한다. 이 accessor는 `advance()`를 호출하지 않는다. 새 `crates/vigilode-coupled/src/n01_derivative_free.rs`는 모델 generation epoch를 고정하고, 

\[
 c(b)=c_0+\tau k_3b^2,\quad a(b)=\frac{a_0+\tau k_2bc(b)}{1+\tau k_1},\quad F(b)=a(b)+b+c(b)-M
\]

에서 `G(b)=clamp(b-F(b)/L_up,[0,M_up])`를 계산한다. `L_up>=1+La+Lc>=sup F'`는 native outward interval arithmetic으로 구성한다. Source stage model은 단일 비음수 diagonal implicit-RK/BE root이고 **원래 8-stage RODAS5P raw-U와 구분**한다. 따라서 N01의 최종 목표는 해당 BE stage를 일반 solver로 승격하는 것이 아니라, source-owned value-only homotopy + original RODAS residual authority의 연결을 실증하는 데 있다.

실제 output 후보의 3성분 full original stage residual `r=z-y-tau*f(z)`는 outward interval로 다시 계산한다. 정확한 scalar F와 a,c 재구성 roundoff에서 physical p-error 상계를 계산한다. 두 budget을 모두 만족해야 3성분 output을 한 번에 갱신하며, `certificate_error`를 gate decline과 구분한다. Source/epoch stale, invalid carrier, budget exhaustion, overflow 등은 output mutation 없이 거절한다. No JVP/Jacobian/Newton/fd derivative is invoked inside new stage path.

## 실행 증거와 실패 보존

- Current-turn runtime: `PYTHON_PASS`, Rust `1.94.1` installed from user's original tarball, compiler `rustc 1.94.1`, cargo `1.94.1`. C++ was not required.
- Local **compatibility mirror** (not full native workspace): `cargo test --offline --all-targets` 10/10, strict Clippy exit0, cargo fmt exit0. Mirror includes a simple standalone homotopy interface and directed interval shim and therefore does not prove the upstream integrated crate builds.
- Native new test source is published on GitHub but itself not executed in a full GitHub checkout in this container.
- `cargo run --offline --release --bin oracle` emits 256 records of original native f64 represented rates, weights, scalar candidates, full returned vector and certificate upper bounds. Independent Python `Fraction.from_float` recomputes exact three-component residual, exact original F and candidate defect `(0,F,0)`, an independent 112-round rational root bracket and physical error enclosure: **1,280/1,280 PASS**, zero failures. Maximum exact-enclosed physical error / f64 certificate is `0.9999771161217686` on generated cases.
- First failed mirror compile Rust syntax `.25`, Clippy errors, initial swallowed certificate-error report bug retained; one scoped repair episode passed tests after typed error propagation.
- Wolfram symbolic stage defect and G derivative-factor identities were checked; literature sources on safeguarded AA are background, not independent native validation.
- GitHub Actions first research-process check failed missing node prerequisites; the retrospective node file, numeric results and append-only ledger row L-0192 were posted later without falsifying their chronology. Existing Q-API, Q-LEDGER, Q-GIVENS, 8 suites/10 interactions and production authority are still not closed.
- Full native Rust repo build, original RODAS8, parallel same-depth work, full-IVP matched-error benchmark and independent-author R6/C9: **NOT_RUN**, not PASS. No timed speedup loop used.

## Source/publication identity

- Source blob `7c2cd365bb830b778910e98fb29be4c36f4cfd8e`, verified by GitHub readback == tested local 10,494-byte source.
- Native test blob `ef0d575ec40ee0e35016ad91e6522eaf6c769691`, verified by GitHub readback == local 3,102-byte test.
- Core source accessor update commit `c26885979abc0f36316832b0e35ada40a2b54038`, new blob `de603d54cee2d7434174b8a133964249ee4c3579`.
- Default target, GMRES, existing RHS homotopy, budget and work accounting, warm starts and tolerances unchanged. PR119 Draft against PR117; no merge or force push.

## DAG

| Object | Completed/total | Change |
|---|---:|---:|
| Original VigilODE | 7/23 | 0 |
| REI central | 28/88 | 0 |
| Proposed successor | 0/12 admitted | 0 |
| N00F scoped audit | 1/1 | 0 |
| R5 exact Robertson auxiliary | 1/1 scoped | 0 |
| N01 derivative-free C5 interface-mirror implementation | 1/1 scoped | 1 auxiliary, not official |
| Native source/root RODAS8 admission | 0/1 | 0 |

Production HOLD. The next actual selected stage is C6 full native integration verification, followed only when evidence supports it by C7 original eight-stage authority. N02 remains blocked. Parallel batching of **independent** physical cells/trajectories is future work; never parallelize strict causal predecessors out of order.

## Verification packaging correction (2026-10-30)

The first clean-archive `VERIFY.sh` accidentally invoked Cargo's automatic integration-test discovery in this **standalone interface mirror**, where the full repository-only `rodas5p_core` and `vigilode_coupled` crates do not exist. The `tests/n01_df_value_stage_contract.rs` file is source for the actual repo, NOT a standalone mirror test. The resulting E0432/E0433/E0689 are **mirror packaging/environment failures**, not evidence of native test failure or success. Preserve `evidence/verify_clean.log`. Correction: `Cargo.toml` sets `autotests=false`; `VERIFY.sh` runs the library and oracle binary while keeping the native test file for later full-workspace execution. Corrected replay `evidence/verify_clean_fixed.log`: exit 0, Rust 10/10, Clippy `-D warnings` PASS, rustfmt PASS, released oracle CSV byte-identical to archived original, independent Fraction 1280/1280 PASS. No numerical source or tolerance changed.

## Remote CI status and evidence boundary

First PR119 research-process run at `3ed27db...` failed because prereg (retrospective), numeric results and append-only ledger row were not yet present. They were subsequently committed. On run `40903002603` at `1d683309...`, research-node input scan advanced to full `check-authority-refs`, which failed with historical remote identity defects: `11297` citations, `1464` distinct identities, `invalid-repository=19`, `missing=42`, `missing+allowlisted=26`, `repository-unavailable=16`, `non-commit=261`, `reachable=1089`, `tree-equivalent=11`. Those failures are not to be corrected by inventing source identities, and do not provide a native C5 PASS. Other native CI runs remained running/queued at the last checked snapshot.

No full native workspace tests or 8-stage full-IVP tests have been verified. The new native tests are published but *their execution status remains NOT_RUN* in this work unit.
