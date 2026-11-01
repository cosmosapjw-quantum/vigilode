# REI IGM 실제 원본 source에 기반한 VigilODE solver/partial-Jacobian 연구

이 작업은 기존 VigilODE PR175의 원본 source 15-state 경로에 대한 **기존 네 solver의 동일 소스 비교**와, 별도 읽기 전용 REI PR348 원본 26-event 커널 및 accepted parent 46,080-cell 입력에 대한 **photon 부분 Jacobian 저랭크 검증**의 두 단위이다. 이 두 단위를 원래 전 26-event IVP라고 합치거나 진척을 이중계상하지 않는다.

- `PREREGISTRATION.md`: 첫 실행 전 게시된 기존 RODAS5P 4-route 계약. 실행 Git SHA `e08a125895451b6aafd5af6541f1ba84a1e05a05`.
- `ORIGINAL26_LOWRANK_PREREGISTRATION.md`: 원래 photon suboperator rank<=3 계약. 실행 Git SHA `53217a58eb2d53c1b72283b174f5e99c6d81f5cd`.
- `WORK_PRECISION.json`: frozen/timevarying 두 물리 arm, 네 기존 native RODAS5P 경로, JVP, RHS, WorkCounters, 별도 출력 reference discrepancy. Time diagnostic only.
- `ORIGINAL26_INITIAL_INTERVAL_FAIL.json`: 원래 H I source의 float64 재배열이 원본 enclosure를 벗어난 실패, 결코 덮어쓰지 않음.
- `ORIGINAL26_PHOTON_JVP_RESULT.json`: source global high-precision (3 moments), local float64 46k-cell owner interval checks, finite difference JVP and photon ledger. 원래 26-event 커널은 읽기 전용.

Rust: `cargo test --offline --locked --release -p rodas5p-integrators --example rei_igm_native_solver_comparison` 및 `cargo run --offline --locked --release -p rodas5p-integrators --example rei_igm_native_solver_comparison`.

Python source: `python3 original26_photon_lowrank_check.py --owner-root /PATH/TO/EXACT/rei_bianchi_PR348_CHECKOUT --lane LOCAL_NEUTRAL_HAZARD_PRIMARY --epsilon 0.0002 --out ORIGINAL26_PHOTON_JVP_RESULT.json`. 정확한 original frozen source archive가 REI checkout에 존재해야 함. `python3 -m unittest discover -s tests -v`로 독립 algebra와 저장된 실제 artifact 검증.

**광학/열역학의 원본 전체 진화와 original tick161은 NOT_RUN/HOLD.** 이 서브시스템 모델을 원래 26-event solver로 포장하지 않는다.

추가 확인: 46,080개 He II photo-rate는 확장정밀도에서 양수이나 float64에서는 모두 underflow zero. 본 작업은 original full 26-event IVP가 아니며 true original acceleration은 미검증. ADVERSARIAL_REVIEW_KO.md 참조. Rust example 5 tests, Python 5 tests normal and optimized PASS.

Scientific publication: Draft PR #178 (VigilODE), base original PR #175, original REI source PR #348, all with immutable identifiers in HANDOFF_NEXT_THREAD.md.
