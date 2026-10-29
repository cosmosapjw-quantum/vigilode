# VigilODE N01 다음 루프: source-owned affine C5 제한 구현 보고

DATE=2026-10-29. Source parent PR113 `beee9aaf4f83eaedb5e4a3167cd3f42059215ec8`, Draft PR114 `https://github.com/cosmosapjw-quantum/vigilode/pull/114` on `research/native-n01-affine-source-20261029`.

## 실제 산출

`crates/rodas5p-core/src/n01_affine_owner.rs` 신규 opt-in, `lib.rs` module export만 추가. 실제 시간 의존 affine RHS/JVP `f(t,z)=[J0+(t-t0)K]z+b0+(t-t0)q`에서 외부 공급된 추정 δJ/δb/eta를 받지 않고 source가 자체로 계산한다. clock 변화 시 epoch 증가, opaque freeze snapshot, shape/norm/finite validation, outward residual, weighted lognorm, 양의 resolvent margin, physical p-norm gate, output transactional commit. 기존 solver/GMRES default routing, restarts, warm start, counters, stage tolerance, Q-API 변경 없음.

## 검산 증거

- Current container Python probe `PYTHON_PASS`, Rust1.94.1 toolchain installed from attached official archive.
- github.com direct git clone `EXECUTION_FAILED` (DNS lookup), so full native workspace build NOT_RUN; Rust API mirror explicitly classified, not native build.
- `cargo test --locked --offline --all-targets`: 15/15 PASS (point baseline 7, affine new 8), `cargo fmt --all -- --check`: PASS, `cargo clippy --locked --offline --all-targets -- -D warnings`: PASS.
- First new test run: 14 PASS/1 FAIL because outward eta on an exact point was `2e-323` not mathematical zero, fixed overly strong test assertion to `eta<1e-300`; no scientific threshold weakened. Initial interface mirror compile failed due absent SHA-256 import; mirror-only helper added, native module uses real `rodas5p-core::sha256_f64_slice_bytes`.
- Rust `affine_oracle.rs` generated 192 deterministic binary64 rows, independent Python Fraction oracle checked 10 inequalities/row = 1920/1920 PASS; reported true maximum original weighted residual 1.1580010595386054 and actual physical error 1.6130637170270992 (synthetic units).
- Wolfram exact nonnormal example `rho_actual=3/320 <=13/320`, `stage_actual=1/135 <=13/387`.
- Two actual synthetic plots generated and visually inspected; both show upper ≥ exact. No claim of independent author review.
- 0 new timed performance loops; N00F inherited one of 3 benchmark loops remains separate.

## Audit

**PHYS-MATH:** time-affine source δJ,δb and residual-gap identity proven under exact represented coefficient law; lognorm resolvent conditional positive margin. Callbacks for actual BASS source have no verified remainder, so full physical extension not admitted.
**PHYS-MATH-CODE:** owner encapsulates clock/epoch, fingerprint, affine data and w/p; no caller-supplied uncertainty bound. Wrong/source/epoch, invalid norm/tolerance, nonpositive margin and callback overflow refuse without output mutation. True full native integrator wiring and distinct-author R6/C9 are NOT_RUN. Mirror hash helper is not the native SHA implementation.
**CRAG:** 192 sampled points show sound bounds, no below-diagonal violations; diagnostic scatter does not certify all inputs. A zero-eta test initially failed for outward arithmetic and was repaired by correctly recognizing a tiny upper enclosure. No increase of numerical admission tolerance.

## DAG and claims

Original core **7/23**, REI central **28/88**, proposed successor **0/12 admitted**, N00F **1/1 scoped**. This new C5/source-only unit: **1/1 implemented scoped**, native/physical admission **0/1**. Formal C6-C9 overall gates remain NOT_RUN, production HOLD; N02 not started. 

Next one actual action: connect this verified affine-source pattern to `OdeProblem`'s real source/clock/epoch/JVP lifecycle or prove a physical nonlinear enclosure for the actual BASS owner, followed by native offline workspace test and independent R6/C9. Do not claim actual physical33 or generalized runtime speedup.
