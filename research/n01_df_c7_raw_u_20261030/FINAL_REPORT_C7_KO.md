# VigilODE N01 C7 과학 검증 완료: 정확한 BE root와 RODAS5P raw-U의 차이

**판정: PASS_SCOPED_C7_PHYSICAL_CORRESPONDENCE.** 기존 C6 native 35/35와 이번 C7 native 3/3을 구별하여 총 38개의 실제 지정 테스트가 통과했다. 원래 8-stage RODAS5P 의 native-K baseline을 출발점으로 raw-U stage 식과 보존량을 비교했으며, derivative-free BE bridge를 원래 RODAS5P 해법으로 오인하는 문제를 정확한 유리수 반례로 제어했다.

## 이론·실험 요약

- Exact source: `f=(-k1*a+k2*b*c, k1*a-k2*b*c-k3*b², k3*b²)`. BE `z=y+tau*f(z)`의 exact root는 original RODAS first raw-U `(I-tau*J0)U0=tau*f(y)`의 해와 일반적으로 다르다.
- Exact defect: `r_U0=tau*(k2*d_b*d_c, -k2*d_b*d_c-k3*d_b², k3*d_b²)`, `d=z-y`. 이차 source의 실제 부호와 질량 중성 원리를 보존한다.
- Fixed dyadic counterexample: 정확 BE 잔차 0, 원래 U0 잔차 `(5/65536,-35/131072,25/131072)`, max=35/131072.
- Independent exact oracle: seed20261030, positive dyadic sources 256/256, 1,799 등록 assertion, false-BE→RODAS promotion cases 256/256.
- Fixed manufactured endpoint z에 대한 tau family는 정확히 `r_U0~tau³`, 정확한 τ 두배 잔차비 8, 10/10. 이는 source family y(tau)=z-tau*f(z) 조건에서만 주장한다.
- Original RODAS snapshot SHA256 `211255f6125b648c6955833c3da537d9c031ec78168b2918563c29fa88c8bce6`의 native 8-stage 직접 comparator, n=3, y=(0.75,0.125,0.125), k=(0.5,1.0,0.5), h=0.005: max raw-U stage residual `7.69783542464708148e-18`, stage mass `2.18026280623256907e-18`, endpoint mass recorded0, raw/K endpoint difference `2.77555756156289135e-17`, reference8 linear solves. 이는 **하나의 작고 온건한 물리 source** 기준이다.
- Source/Wolfram: exact polynomial remainder equality, `1^T r=0`; 원래 strictly-causal 8-stage mass 보존은 각 stage에 source mass identity를 대입하면 귀납적으로 성립한다.

## 실제 C7 native 검산

[GitHub Actions run 40914675878](https://github.com/cosmosapjw-quantum/vigilode/actions/runs/40914675878), job 123591615978, 실행 source HEAD `b80617f6eb0d38d42dbf8ee5552b3d94b5a0a785`:

- `cargo test --locked --offline -p vigilode-coupled --test n01_df_c7_original_raw_u_contract -- --nocapture`: 3/3 PASS.
- `cargo test --locked --offline -p vigilode-coupled --test n01_df_value_stage_contract`: 6/6 PASS.
- `cargo test --locked --offline -p vigilode-coupled --test rhs_homotopy_contracts --test causal_contracts --test causal_reuse_contracts`: 6+6+17=29/29 PASS.
- `cargo clippy --locked --offline -p vigilode-coupled --test n01_df_c7_original_raw_u_contract -- -D warnings`: PASS.
- `rustfmt --check --edition 2024 crates/vigilode-coupled/tests/n01_df_c7_original_raw_u_contract.rs`: PASS.
- Python Fraction 및 τ^3 oracle 재생: 256 cases, 1,799 assertions, 10 tau rows PASS; 두 machine-readable JSON은 Git 원본과 동일.

초기 run40913125750의 test-only Clippy 5건과 run40913805580의 Python `research/analysis/` 잘못된 상대경로 실패를 **그대로 보존**했다. 같은 알고리즘·률·tableau·허용오차를 유지한 상태에서 코드 스타일과 경로만 교정했다. 부정 결과를 사후에 PASS로 바꾸지 않았다.

## 안전한 다음 과학적 연결

C7 결과는 Newton-free auxiliary BE를 값 기반 예측기로 사용할 수 있다는 가능성을 보여주지만, 오직 **원래 8-stage 잔차와 물리 오차가 별도로 허용될 때만** adoption할 수 있다. n=3 dense direct reference를 derivative-free production implementation이라고 부르지 않는다. 실제 큰 source, BASS physical33, event/clock/epoch query, 엄밀한 stage-error resolvent/Lipschitz 및 full-IVP matched-error 비교는 아직 불완전하다.

원래 VigilODE **7/23**, REI 중앙 **28/88**, successor **0/12 admitted** 그대로. 보조 C7 **1/1 scoped**. Production **HOLD**, native independent-author R6/C9 **NOT_RUN**.
