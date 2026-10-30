# N4B verified research package, Rust 1.99.1

**결과:** 원본 N4A first-window 358개 모델의 binary64 accepted carrier와 첫 물리오차 상계를 불변 source receipt에 고정하고, 동일 frozen source의 second window 358개 실제 N3H/Picard 후보를 outward interval enclosure로 감쌌다. 이때 기존 N3H Session 내부 error metadata도 physical+float gap으로 갱신한다. **전체 716개 source-specific mode-window physical conditional bounds**; true full native PR131 original RODAS8 integration not run.

- `rust/`: original N4A research Rust source (42406-byte identical prefix in n3h_source_bounded_copy.rs) plus new appended N4B source-owned receipt and new bin.
- `evidence/N4B_TWO_WINDOW_NATIVE.csv`: 716 actual native outputs and source-exact interval/physical conditional bounds.
- `independent_second_window.py`, `quick_fraction_oracle.py`, `independent_physical_bdf.py`: independently implemented numerical and exact rational tests.
- `THEORY_AND_REVIEW_KO.md`, `STAGE_LEDGER.json`, `FAILURE_REGISTRY.json`, `HANDOFF_NEXT_THREAD_KO.md`: proven scope and blockers.

Quick replay: `bash REPLAY.sh` (no network needed, Rust1.99.1). Optional costly 180/220-digit check: `python independent_second_window.py`. Use `python verify_manifest.py .` to authenticate package.

Official: VigilODE 7/23; RVJ 3/8 + N4 partial; C9 0/1; C10 not started; production HOLD. The record of full 180/220-digit check is preserved; do not claim final joint rerun if it was interrupted.
