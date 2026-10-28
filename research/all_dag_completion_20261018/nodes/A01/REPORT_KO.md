# A01: 단일 구간 인증과 weighted-tube owner의 통합

최신 local 연구의 `PacketFrozenOutput`/`PacketCertifiedView`와 cloud의 `enclose_at_weighted`/다중 구간 경로를 함께 보존했다. 충돌의 원인은 서로 독립적으로 추가된 export 및 radius 설명이었다. 잔차 다항식, 초기 에너지 상한, 반환 중심의 interval displacement 계산을 바꾸지 않고 두 API를 합쳤다.

이번 추가 구현은 `PacketFrozenOutput::from_weighted_radius(curve,r,work)`다. 입력 weighted ball의 반경을 owner 안에 복사하므로 후속 query에서 초기조건을 바꿔 붙일 수 없다. 기존 component-disk 생성자는 그대로 유지한다. 반환 `PacketCertifiedView::centered_weighted_radius()`는 weighted owner에서만 `Some`이며 반환 binary64 중심에 대한 반경이다. `weighted_radius()`는 정확한 저장 다항식 주위의 반경이므로 둘을 구별한다.

## 모델, 증명 재사용과 새로운 연결

가중 내적은 `||a||_w² = Σ_j w_j |a_j|²`, 저장된 양의 가중치 합은 `S = Σ_j w_j`다. 모델은 `a'_j = i ω_j a_j + i χ |m|²m`, `m = Σ_j w_j a_j`, `ω_j = width·xi_j`, `κ=0`이다. carrier는 rotating-frame metadata로 유지한다. 시간과 계수는 기존 모델의 단위이며 `λt`는 무차원이다.

기존 `research/cloud_completion_20261018/PACKET.md`의 에너지 보존 및 weighted logarithmic-norm 증명을 재사용한다. incoming true state가 `||a(0)-Y(0)||_w ≤ r₀`이면 삼각부등식으로

`E+ = (||Y(0)||_w + r₀)²`

가 모든 허용 초기조건의 에너지 상한이다. 정확한 저장 다항식 `Y`의 평균과 연속 잔차를 `M_Y`, `ρ_w`로 둘 때

`λ = S |χ| max(S E+, M_Y²)`,

`B_w(t) = exp(λt) (r₀ + tρ_w)`

는 원래 흐름과 그 다항식 사이의 weighted 거리 상한이다. interval Horner는 반환 중심 `c_j`에 대해 `|Y_j-c_j| ≤ δ_j`를 보장한다. 따라서

`||a-c||_w ≤ B_w + sqrt(Σ_j w_j δ_j²)`.

새 owner는 `r₀`를 변경할 수 없는 scalar로 저장한 뒤 **기존 동일 계산 함수**에 전달한다. 따라서 원래 theorem의 전제가 owner의 모든 query에 고정되고, returned-center weighted ball을 다음 owner의 입력으로 전달할 수 있다. component disk로 변환했다가 다시 합칠 필요가 없어 기존에 증명한 인위적 `sqrt(N)` inflation 제거가 유지된다. 추가 setup은 scalar 1개 저장, O(1) work/storage이며 초기 tube를 위한 heap buffer가 없다. 이 구조적 사실과 실측 allocator 결과를 혼동하지 않는다.

`κ=χ=width=0`에서는 정지 흐름이며 새 owner의 동일 연산 경로는 원래 가중 반경을 보존하는 쪽으로 outward rounding한다. `χ=0`은 기존 정확 trig oracle이 검사한다. 음·양의 χ, `S≠1`, support endpoint 밖의 허용된 binary64 입력도 원래 방정식 그대로 유지한다.

## 검증 분리 및 실패 보존

incoming local의 `production_audit_packet_local_certificate` 테스트는 첫 scientific reference보다 앞에서 allocator 양성 대조군을 검사한다. 이 환경에서 양성 대조군 실패가 이미 관찰되어, 해당 실패가 수학적 포함 검사를 가리지 않도록 새로운 `all_dag_a01_packet` 테스트를 추가했다.

기존 allocator-gated 테스트와 `verify(payload)`의 `clone allocations >= 3` 기준은 그대로다. Python oracle에는 `verify_numeric(payload)`를 분리하고 명시적 `--numeric-only` 경로를 추가했다. 수학적 recurrence, `epsilon40≤2^-120`, `epsilon48≤2^-160`, 교차 기준, 80개 disk 포함, 변조한 중심/반경 negative control은 그대로이며, 이 경로는 `allocation_authority=NOT_EVALUATED`를 반환한다. 기존 과거 결과를 새 결과로 덮어쓰지 않는다.

새 Rust 테스트는 다음을 검사한다.

- frozen weighted owner와 기존 weighted entry의 반환 중심·mode/mean radius·centered weighted radius가 비트 단위 동일하다.
- NaN/Inf/음의 초기반경을 typed-refuse하고 잘못된 query 후 owner의 초기 tube를 유지한다.
- weighted setup scalar 1개 복사, tube buffer 예약 0회, query/refusal 누적 계수를 검증한다.
- component owner 생성 후 caller의 원래 반경 배열을 NaN으로 바꿔도 복사된 정상 초기 tube가 유지된다.
- 4개 arm·3개 unequal-frequency mode·5개 시간에서 독립 Fraction reference의 80개 mode/mean 포함 검사를 수행한다. 기존 성공 query의 110 certificate-operation 계약이 유지된다.

실제 실행 상태는 `RESULT.json`과 로그를 참조한다. 새 proof는 기존 인증 theorem을 owner에 연결하는 직접 유도이며, 광범위 모델의 production 승격이나 elapsed-time speedup을 주장하지 않는다.

## local 실행

```sh
cargo test -p rodas5p-integrators --test all_dag_a01_packet --locked --offline -- --nocapture
```

Allocator를 보정한 local toolchain에서는 기존 `production_audit_packet_local_certificate` 전체를 실행한다. 보정 실패는 환경/측정의 실패로 보존하며 수치 포함의 실패와 분리한다. 이 노드의 API를 A02/A03/A09에서 사용할 수 있고, 원래 출력 tolerance는 해당 trajectory client가 유지한다.

## Integrated execution update

Root executed both native tests successfully, including 80 independent Fraction disk checks. See evidence/native.log. Allocation authority was deliberately not inferred from this numerical oracle.
