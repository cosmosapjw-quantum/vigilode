# VigilODE PREFIX01: original-residual-gated GMRES prefix + signed lognorm

**2026-10-29 KST** · 새 standalone theory→coding 연구 · 기존 PR104 source locked `10854bc0bff81b6d3cf5cb0a44520ed9165c205f` · 생산 승격 **HOLD**.

## 핵심 결과

PR104는 quadratic directional relation으로 일부 Arnoldi/MGS/QR를 제거했고 실제 full-IVP 2,460 solves에서 회전16D 3.4253배, 회전4096D 1.9729배, damped 3.1744배, mixed16-rate 1.1863배를 보고했다. 잘못된 coefficient 대조군은 0.804배로 느려졌다. 기존 성과를 재실행하거나 새 성과에 합산하지 않았다.

이번에는 일반 16-rate 계열에서 **조기 종료한 GMRES prefix의 원래 잔차를 인증**하는 수학적·코딩 프로토타입을 만들었다. `A=I−σJ`, weighted infinity lognorm 상계 `m`에서 `σ≥0, 1−σm>0`이면 `||A^{-1}||≤(1−σm)^{-1}`다. 실제 저장된 binary64 점행렬의 outward original residual `ρ`로 `||xhat−x*||≤ρ/(1−σm)`를 보장한다. Projected Givens residual은 검사 trigger일 뿐 채택 근거가 아니다. 이론의 source assumptions, 단위, 증명, 반례는 `THEORY_KO.md`를 보라.

## 원천·DAG 대조

이번에 관련 8개 저장소(vigilode, bass, rei_bianchi, BASS_HE, bass_cr, rec_bianchi, WU088_HH, htt_base)의 branch metadata **1,831개**를 끝 페이지까지 열거했고, 각 repo 최신 PR 5–15개의 메타데이터와 주요 source PR 본문을 확인했다. 이는 모든 branch의 모든 commit/file 원문 전수 검증이 아니다. 이전 대화/백업의 근거는 GitHub 원문·업로드 ZIP 해시와 구분해 사용했다.

- VigilODE 최신 PR104 HEAD `10854bc0bff81b6d3cf5cb0a44520ed9165c205f` (Draft), PR103 HEAD `1845e1d8bdfda222bf77637e8d38d5076743f238` (R6/C9 pending), PR102 HEAD `8357faa5b42724a8be24609d4d2f16aec8a6eb3a` (scoped affine).
- PR104는 main 기반 누적 ancestry이며 PR103은 PR102 위의 **sibling**이다. 두 코드를 무비판적으로 cherry-pick하지 않았다. 새 독립 연구는 PR104 위에 research-only path로만 게시한다.
- BASS PR233 chemical6×23 crossblock, PR234 tau1×30 row는 partial physical input. BASS CE1-G와 REI UVB PR261 등 최신 변화는 본 연구의 rate/capsule/clock authority를 제공하지 않으므로 자동 import하지 않았다.
- 원본 VigilODE DAG **7/23 inherited scoped**, 34 edges; 중앙 REI **28/88** (working33/88과 별도). 이번 보조 PREFIX01 **0/1** (독립 R6/C9 대기), workflow **18/20**. Production **HOLD**.

## 실제 실행과 수치

- CURRENT-TURN runtime: Python `PYTHON_PASS`, C++ `CPP_PASS`; 초기 PATH의 Rust `COMPILER_MISSING`, 제공된 Rust1.94.1 tarball을 격리 prefix에 설치 후 `rustc 1.94.1`, `cargo 1.94.1` 실행 PASS. Git HTTPS DNS 실패, GitHub connector 별도 경로 사용.
- Standalone Rust no-dependency crate, `#![forbid(unsafe_code)]`; 2-pass MGS Arnoldi, incremental Givens QR, projected trigger, original-residual outward certificate, lognorm resolvent, transactional output, event dual-sign.
- `cargo test --offline --all-targets -- --test-threads=1`: **9/9 PASS**. Strict `cargo clippy --offline --all-targets -- -D warnings`: **PASS**. `cargo fmt --check`: **PASS**. 최초 Clippy 및 rustc PATH 오류 로그 보존.
- Rust가 출력한 60개 point-matrix binary64 record를 독립 Python `Fraction.from_float`로 복원해 μ, ρ, K, Kρ에 대한 **240개 정확 포함관계 PASS**. 최대 outward excess: μ 1.33e-15, ρ 4.44e-16, K 4.74e-16, forward 5.61e-16. 이는 해당 60개 record에 대한 검산이며 모든 FP 환경에 대한 완전 증명은 아니다.
- Wolfram `J=diag(-2,-3), σ=1/4`: μ=−2, K=2/3, exact inverse weighted∞ norm=2/3, Laplace resolvent identity difference zero.

### Standalone timing, 7 paired blocks per fixture

**End-only / selective prefix** (same acceptance budget 1e-7; end-only may achieve smaller actual error):

| n | Prefix columns | End-only columns | Median time ratio | Observed min–max |
|---:|---:|---:|---:|---:|
| 16 | 7 | 16 | 1.2559 | 1.0341–1.3223 |
| 64 | 7 | 16 | 1.1730 | 1.1171–1.2277 |
| 128 | 7 | 16 | 1.1902 | 1.0813–1.2517 |

**Always-check / selective prefix**, *bit-identical returned state*, both 7 columns:

| n | Selective checks | Always checks | Median time ratio | Observed min–max |
|---:|---:|---:|---:|---:|
| 16 | 2 | 8 | 3.4654 | 3.2993–3.8570 |
| 64 | 2 | 8 | 3.4729 | 3.3423–3.7482 |
| 128 | 2 | 8 | 3.5835 | 3.2491–3.7794 |

Negative control at 16D with a much stricter budget 1e-12: always-check 11 columns/12 checks versus end-only 16 columns/2 checks, yet **end-only / always-check median 0.2430** (always-check slower). All timing includes per-solve allocation and interval certification; CPU/OS noise and small synthetic sizes apply. This is not PR104 full-IVP or production benchmark, and cannot be added to PR104 ratios.

### Error admission

The synthetic 16-rate diagonal matrix has `J_ii=−2−(i mod16)/8`, `σ=1/4`, unit weights and `x0=0`. Certified `K` is outward near `2/3`. Prefix 16D: original residual upper `2.1431e-8`, forward upper `1.4287e-8` (budget `1e-7`); end-only upper `1.4063e-15`. Both satisfy the same budget, but their achieved accuracy differs. Matching-output comparison above isolates only check-selection overhead.

## 실패, 독립 심사 및 보존

- First Cargo run failed because `rustc` was not in PATH (`runtime_environment`); repaired by explicit toolchain PATH. The Rust tarball was not silently assumed installed.
- First strict Clippy had 8 diagnostics and first rustfmt check failed. Repair renamed outward arithmetic helpers, preserved sum order, introduced a config struct and iterator form; first failures remain in evidence. A second Clippy test-only range warning was repaired; final tests/Clippy passed.
- Incorrect/singular/residual-gap cases cannot be promoted: `σμ≥1`, negative σ, nonfinite/invalid weights, nonfinite operator and exhausted prefix fail closed. Forced projected triggers cannot bypass original-residual certification; caller output stays unchanged on refusal.
- Native subagent delegation unavailable this turn: **R6 independent-author and C9 independent-code reviews NOT_RUN**. Same-context adversarial checks and Python/Wolfram crosschecks are not independent reviewer receipts.
- Actual BASS30/physical33 source, vector accepted dense capsule, physical clock/rate/epoch, FP callback evaluation uncertainty, original 8 suites/10 interactions, Q-API, Q-LEDGER authority, Q-GIVENS and full CI remain open. PR104's 181 authority diagnostics and prior negative work/cost evidence remain retained.

## Reproduction

```bash
cd prefix01
export PATH=/path/to/rust-1.94.1/bin:$PATH
cargo test --offline --all-targets -- --test-threads=1
cargo clippy --offline --all-targets -- -D warnings
cargo fmt --all -- --check
cargo run --offline --release --bin bit_probe > evidence/bit_probe.csv
python3 evidence/verify_bits.py
cargo run --offline --release --bin prefix_bench > evidence/benchmark.csv
cargo run --offline --release --bin matched_bench > evidence/matched_benchmark.csv
cargo run --offline --release --bin negative_bench > evidence/negative_benchmark.csv
python3 evidence/analyze_bench.py
python3 evidence/analyze_matched.py
```

Root VigilODE crate was **not** cloned/compiled in this session; native integration is a distinct unverified obligation. The standalone tests above do not prove production integration or full-IVP speedup.

## 다음 노드

First integrate as an opt-in original-residual prefix into the actual PR104 `gmres_into.rs` while preserving both left/right preconditioning semantics, workspace counters, partial failures, warm starts, and default route. Avoid duplicating existing `solve_gmres_into_right_monitored`. Native test and whole-IVP matched-accuracy ABBA/BAAB then measure end-to-end cost, including extra certification. Separately, owner-accepted chemical6 rate/clock capsule and physical33 norm transport must precede physical-source promotion. No force-push, main merge, budget reset or production admission.
