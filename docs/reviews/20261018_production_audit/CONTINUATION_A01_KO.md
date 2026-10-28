# Production audit continuation: A01 identity boundary

기준은 게시된 감사 commit `6c067105f0b9e16cbfd397b2d5316673e698e00e`이며 tree는 `8a99f23d9dd855d4e4101c20f56eb1f94a73b63d`이다. 원본 `docs/diffsol-benchmark`의 `2fc61b944fe431d40ba455c373579bce00b0c39a`, Loop12 `bbb8ef42ad4e9b49704b165a54a26c435c575ae3`, 기존 리뷰 산출물은 수정하지 않았다. 이번 작업은 우선순위 A01의 descriptor identity 하위 계약만 구현했다.

## 구현

`CompiledPacket`은 생성 당시 모델, horizon/order, moments, free coefficients, kernel coefficients의 binary64 bit pattern을 private snapshot으로 보존한다. 기존 public 필드는 읽을 수 있으나 certificate, candidate generation, reconstruction, dense-output evaluation 경계에서 snapshot과 다르면 거절한다. `PacketCandidate`는 생성 packet의 private identity를 보유하며 다른 packet에 재사용할 수 없다. Reconstruction API는 이제 `CoreResult`를 반환하여 변조 입력을 빈/잘못된 출력으로 조용히 내보내지 않는다. trajectory는 reconstruction 거절을 rejected attempt로 처리하고 해당 시도에서 누적된 work를 유지한다.

이 안전망은 packet 생성과 candidate 생성에서 O(N+p) snapshot 복사/저장 비용, 각 검증에서 O(N+p) 비교 비용을 추가한다. 검증 시 임시 snapshot을 복제하지 않도록 직접 슬라이스 비교를 사용한다. 현재 `PacketWorkCounters`는 snapshot 복사와 비교 비용을 세지 않으므로 zero-allocation 또는 성능 개선 근거가 아니며 PK-09의 비용 회계는 열린 상태다.

중간 출력은 예외가 아니라 의도적인 descriptor 변환이다. `P(tau*s)`에 대해 계수를 `tau^k c_k`로 제한하고 horizon도 `tau*h`로 줄인 sub-packet을 만든 다음, 제한된 candidate identity를 그 sub-packet으로 전이한다. 이 처리는 이미 고정된 정규화 시간 수식에 대응하며, 이전 회귀의 중간 출력 정확도 조건을 유지한다.

## 회귀 및 독립 검토

새 회귀는 public kernel coefficient 변조가 certificate/output에서 거절되는지, 다른 horizon packet의 candidate가 거절되는지, `tau=0` cross-packet 요청도 거절되는지, model의 `carrier` +0.0을 -0.0으로 바꾼 byte-level identity 변화가 거절되는지 확인한다. 기존 exact one-mode dense restriction와 trajectory 회귀를 함께 실행했다. 두 차례 read-only 검토는 signed-zero 우회를 찾아 bitwise model 비교를 요구했고, 최종 검토는 source-origin identity slice를 scoped PASS했다. 이 검토는 outward 산술 정리의 독립 재증명이 아니다.

남은 API 경계도 명시한다. `PacketCandidate.mean`은 mutable이고 `PacketCertificate`에는 candidate waveform identity가 없으므로 certificate를 만든 뒤 mean을 바꾼 출력과 이전 certificate가 자동 결속되지는 않는다. Reconstruction errors는 `CoreError::InvalidInput(String)`로 변환되어 `PacketError` variant를 직접 구분할 수 없다. 두 점과 identity-work accounting은 A01 완료 조건으로 남는다.

## 검증 기록

| 명령 | 결과 |
|---|---|
| `cargo test --locked --offline -p rodas5p-integrators --test packet_window_contracts --test packet_trajectory_contracts --test production_audit_packet --test production_audit_split` | exit 0, 27 passed |
| `cargo check --locked --offline --workspace --all-targets` | exit 0 |
| 변경 대상 4개 Rust 파일 `rustfmt --edition 2024 --check ...` | exit 0 |
| `cargo fmt --all -- --check` | exit 1; 기존 `damped_moments.rs`, `damped_moment_enclosure_regressions.rs` 포맷 차이 포함. 이 무관 파일들은 수정하지 않음 |

원시 targeted test 출력과 실패/수정 내역은 `research/production_audit_20261018/evidence/continuation_a01/` 아래에 있다. 실행 환경은 Rust 1.94.1, locked offline Cargo dependency cache이다. `Cargo.toml`/`Cargo.lock`은 변경하지 않았다.

## finding별 이어받기와 판정

- PK-08: packet의 source model과 생성 배열 identity 변조는 거절된다. 완전 outward certificate 바인딩은 미완료이므로 `PARTIALLY_FIXED`.
- PK-10: public packet 변조와 cross-packet candidate 재사용은 닫혔다. softened admission 전제와 전체 산술은 남아 `PARTIALLY_FIXED`.
- PK-02: coefficient, nonlinear force, continuous residual, tail, reconstruction 반올림은 아직 모두 outward 처리되지 않아 `OPEN`.
- PK-09: setup/certificate/output 비용 계측은 미완료라 `OPEN`.
- A01: `IN_PROGRESS`; 현재 certificate는 계속 `roundoff_enclosed=false`이며 production packet decision은 `HOLD`.
- A02: A01의 실제 center/radius 인터페이스와 outward residual이 생길 때까지 dependency hold. T1은 조건부 수학 정리로 채택되었고 구현은 아직 계약에 머문다.
- 기존 전체 production, 전역 정확도, 동등 정확도 속도 결정은 계속 `HOLD`. generic RODAS5P의 이미 문제별 검증된 사용 범위는 바뀌지 않는다.

machine-readable continuation DAG는 `research/production_audit_20261018/CONTINUATION_DAG.json`이다. 다음 실행점은 A01의 interval moment/coefficient/force/residual/tail/reconstruction 계산을 구현해 반환 중심값까지 포함하는 반경을 만들고, exact one-mode 및 high-precision multi-mode 독립 reference로 이를 검증하는 것이다. 이 하위 계약이 통과되기 전에는 A02를 실행 완료로 표시하지 않는다.
