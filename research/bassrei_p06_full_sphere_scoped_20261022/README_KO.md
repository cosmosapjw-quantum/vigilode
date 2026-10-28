# BASS/rei P06 (scoped): 삼축 Bianchi I 전구면(full sphere) 광자 수송 — 요약

작성일: 2026-10-22 KST. 기준 커밋: `031cd20b`. 이 노드의 산출물은 아래 세 가지다.

- `FULL_SPHERE_THEORY.md`: 유도, 증명, 이산 연산자, 상태 레이아웃, Rust 비트 일치(bit parity) 계약, HOLD 목록
- `FULL_SPHERE_FVM_REFERENCE.py`: 순수 Python float 공식 오라클 (검사 38개)
- `CHECKS.json`, `CHECKS.log`: 실행 결과

이 노드에서는 Rust 코드를 쓰거나 바꾸지 않는다. 이 문서는 BASS/rei 실제 통합이나 성능을 주장하지 않는다.

## 실행

```
cd research/bassrei_p06_full_sphere_scoped_20261022
python3 FULL_SPHERE_FVM_REFERENCE.py CHECKS.json
```

결과는 `pass_scoped`이다. 38개 검사가 실패 0, 오류 0으로 통과했다. 실행 시간은 약 24 s이며 진단용(`diagnostic_only`)이다. 실행 환경은 Python 3.12.3, numpy 2.4.2, scipy 1.17.0, mpmath 1.3.0, glibc 2.39다.

P01 오라클(`research/ncp_followup_20261021/BASS_REI_FVM_REFERENCE.py`)은 파일 경로로 읽기 전용 import했다. 바이트코드 생성을 끄고 읽었으므로 노드 디렉터리 밖의 파일은 바뀌지 않았다.

## 증명된 것

괄호 안은 해당 검사 ID다.

1. **드리프트 (`a1`)**: 보존되는 공변 운동량 `p_k`에서 다음을 유도했다. `Fraction` 위의 이중수(dual number) 미분으로 40개 유리점에서 정확히 확인했다.
   - `q̇ = −Σ H_k n_k²`
   - `μ̇ = −μ(1−μ²)(H_3 − H_φ)`
   - `φ̇ = (H_1 − H_2) sin φ cos φ`
2. **Liouville 항등식 (`a2`)**: `3H + 3q̇ + ∂_μ μ̇ + ∂_φ φ̇ = 0`.
   - 식은 `H`에 선형이고 4차원 공간 `span{1, μ²} ⊗ span{1, cos 2φ}`에 속한다.
   - 따라서 2×2 unisolvent 격자와 `H = e_1, e_2, e_3`에서 `Fraction` 계산으로 확인하면 항등식이 증명된다.
   - 중복 확인을 포함해 정확한 평가 414회를 수행했다.
3. **LRS 환원 (`a3`)**: `H_1 = H_2 = H − s`, `H_3 = H + 2s`이면 다음이 성립하며, 이는 P02 연산자와 정확히 같다.
   - `φ̇ = 0`
   - `μ̇ = −3sμ(1−μ²)`
   - `q̇ = −H − s(3μ² − 1)`
4. **정확한 특성곡선 (`g1`–`g3`)**: 다음 닫힌 형태를 시간 의존 `H_k(t)`에 대한 RK45 광선 적분(`rtol` 1e-12)과 비교했다.
   - 닫힌 형태: `U = (Π A_k)(E/E_0)³ U_0(q_0, μ_0, φ_0)`
   - 최대 오차는 2.2e-13으로 기준 1e-9 안이다.
   - 사상의 야코비안이 Liouville 인자와 일치한다(상대 오차 3.5e-10).
5. **이산 수송 (`b1`–`b6`)**:
   - 공유면 1차 upwind에서 수(number) 보존 `Σ rhs + out_lo + out_hi = 0`이 정확히 성립한다. float 잔차는 8.8e-17이다.
   - `dt · max λ ≤ 1`이면 Euler 양성이 유지된다.
   - 야코비안 대역폭은 `M + 1`이다.
   - 야코비안 열은 `state_rhs(e_c)`와 비트 단위로 같다(3178개 항목).
6. **이산 LRS 환원 (`c1`, `c2`)**: `H_1 == H_2`이고 축대칭 데이터이면, φ로 합한 3D rhs가 P01의 `lrs_number_rhs`와 같다.
   - 실수 산술에서는 정확히 같다.
   - float에서는 상대 오차 1.0e-15이다. φ 격자 6종(nphi = 1, 2, 3, 5, 7, 16, offset 포함)을 썼다.
7. **θ-only 축약의 비폐쇄성 (`c3`)**: 삼축 전단에서 φ 평균 방정식은 `m = 2` 방위 모멘트를 필요로 하므로 닫히지 않는다.
   - 1차 시간도함수는 θ-only 연산자와 일치한다.
   - 2차 도함수는 정확히 `h_d² C`만큼 다르다. `h_d`를 절반으로 줄이면 차이의 비가 3.99999999999995이다.
   - 따라서 θ-only 지름길은 정확도가 낮은 것이 아니라 틀린 것이다.
8. **rank-6 Thomson (`d1`–`d6`, `e1`–`e3`, `f1`)**:
   - 형식: `Ṅ_c = Γ[ω_c(M + Σ w_m B_{m,c} Q_m) − N_c]`
   - 군(group) 수 보존이 대수적으로 정확하다. float 잔차는 2.9e-16이다.
   - 등방 상태가 정상(stationary)이다. 잔차는 5.1e-16이다.
   - 이산 핵은 연속 핵의 셀 평균이고 3/4 이상이므로, `Γ dt ≤ 1`에서 Euler 양성이 유지된다.
   - 일반 격자에서 rank(T + ΓI) = 6이다.
   - 대칭 격자에서 퇴화를 예측했고 실제로 확인했다: 균일 nphi = 4는 rank 5, nphi = 2는 3, nphi = 1은 2, 대칭 2셀 μ 격자는 5.
   - φ로 합하면 LRS rank-2 Thomson과 같다(2.2e-15).
9. **편광 계약 (`a6`, `h1`–`h6`)**:
   - 결맞음(coherency) 행렬의 양성 조건은 `I ≥ sqrt(Q² + U² + V²)`이다.
   - 축대칭 Chandrasekhar `(I, Q)` 핵이 셀 평균 `(I_l, I_r)` 핵과 정확히 같음을 `Fraction`으로 확인했다.
   - `Γ dt ≤ 1`인 Euler는 `I ≥ |Q|`를 유지한다(1200회 시행).
   - `Γ dt = 1.5`에서는 이 조건이 깨진다.
   - 성분별 클리핑(clipping)의 반례 네 가지를 보였다: PSD 위반 유지, 수 생성, 유효 상태 변형, 실제 Euler 출력에서의 실패.
   - 올바른 대안 `coherency_limit`은 `I`를 보존하면서 편광 벡터를 축소한다. `I < 0`이면 거부(refusal)한다.

## 구현된 것 (오라클)

- `sphere_number_rhs(q_edges, mu_edges, phi_edges, contents, H1, H2, H3, boundary) -> (rhs, outward_lo, outward_hi)`
  - `select=`를 주면 upwind 패턴을 고정한 `f_t` 계산에 쓸 수 있다.
- `thomson6_rhs(mu_edges, phi_edges, contents, rate)`, `thomson6_factors` (저랭크 인자 `U`, `V`)
- `grid_state_rhs`, `grid_state_partial_t`, `grid_state_jacobian_entries` (Rust `jacobian_entries`의 형판), `grid_transport_cfl`
- 셀 평균 닫힌 형태: `mu_cell_constants`, `phi_cell_constants` (상쇄 없는 형태; mpmath 대비 1.0e-16)
- 정확한 특성곡선: `characteristic_map`, `exact_U`, `exact_cell_integral`, `exact_cell_contents`
- 편광: `polarized_thomson_iq_rhs`, `polarized_thomson_lr_rhs`, `coherency_eigenvalues`, `componentwise_clip` (음성 대조군), `coherency_limit`
- 유형별 거부(typed refusal) 27종: `GridRefusal`, `StateRefusal`, `GeometryRefusal`, `RateRefusal`, `BoundaryRefusal`, `CharacteristicRefusal`, `PolarizationRefusal`
- 골든 픽스처 `parity_fixture()`
  - 입력, 격자 상수, 출력의 hex 값과 SHA-256 `1514967f…941a`를 `CHECKS.json`에 기록했다.

## 1차 수렴 (보고만 하고 차수 게이트는 두지 않음)

조건: `H = (0.3, −0.1, 0.05)`, `T = 1`. 오라클에서 탐침(probing)으로 조립한 정확한 행렬에 `expm_multiply`를 적용했다.

| 격자 | 상대 L1 오차 | 관측 차수 |
|---|---|---|
| 12×6×6 | 0.0670 | – |
| 24×12×12 | 0.0447 | 0.58 |
| 48×24×24 | 0.0270 | 0.73 |

오차는 단조 감소한다. 이는 1차 기법이 점근 영역에 들어가기 전의 거동이며, P02 pilot의 0.52 → 0.875와 같은 양상이다.

## 모델링 선택 (Rust 구현자용; 상세는 이론 문서 §14)

1. **φ 격자**
   - 첫 경계 `φ0`는 `0 ≤ φ0 < TAU`이고, 마지막 경계는 f64 계산으로 정확히 `φ0 + TAU`이다(`TAU = std::f64::consts::TAU`).
   - 주기면은 `nphi`개다. 면 `l`은 `phi_edges[l]`에 있고 셀 `(l−1) mod nphi`와 `l` 사이에 있다.
   - 래핑(wrap) 면의 속도는 `phi_edges[0]`에서 평가한다.
2. **upwind 동률 규칙**: `v >= 0.0`이면(−0.0 포함) 낮은 인덱스 셀을 택한다. φ에서는 이전 주기 셀이다.
3. **면 속도**: 법선 속도의 정확한 면 평균을 쓴다.
   - q면: `−(H3·⟨μ²⟩ + ⟨1−μ²⟩·hp_k)`
   - μ면: `−(H3 − hp_k)·μ(1−μ)(1+μ)`
   - φ면: `h_d sin 2φ`
   - 플럭스는 `v · N_up / h_up`이다.
4. **연산 순서**
   - Stage Q: 열 `j` 우선, 그다음 `k`, 면 `0..=nq`
   - Stage MU: `i`, `k`, 면 `1..nm−1`
   - Stage PHI: `i`, `j`, 면 `0..nphi−1`
   - 각 면에서는 "아래 셀 `−= flux`, 그다음 위 셀 `+= flux`" 순서다.
   - 이후 군별 Thomson(모멘트를 셀 순서로 누적, 그다음 이득), 마지막으로 흡수 항이다.
5. **실수 조화함수 정규화**
   - 비정규화 `b_m = (P_2, xz, yz, x² − y², xy)`, 가중치 `w = (1/2, 3/2, 3/2, 3/8, 3/2)`
   - 정규직교 `Y_2m = sqrt(10 w_m/4π) b_m`
   - `ω_c = (d_j/2)(Δφ_k/TAU)`
6. **상태 레이아웃**: P02의 `[L_lo, (셀들, A_i) 군별, L_hi]`이다.
   - `cell(i, j, k) = 1 + i(M+1) + j·nphi + k`, `M = nm·nphi`
   - `absorbed(i) = 1 + i(M+1) + M`
   - 대역폭은 `M + 1`이다.
7. **부동소수점 규칙**
   - FMA 금지, `powf` 금지(곱으로 쓴다), 괄호대로 결합한다.
   - `sin`/`cos`는 플랫폼 libm에 의존한다. libm이 다르면 φ 상수에서 ulp 단위 차이가 날 수 있으므로, 골든 상수 hex를 먼저 비교한다.

## HOLD (이 노드에서 해결하지 않은 장애물)

- **틸트(tilt)와 수차(aberration)**: 도플러 이동이 에너지 군을 결합하므로 "대각 + 군별 rank 6" 구조가 깨진다. 셀 측도 변환, 군 간 보존적 재배치(remap), 물질 에너지 원장이 필요하다.
- **반조(recoil)**
  - Compton 반조와 Kompaneets 항은 군 간 에너지를 교환하며, 유도 산란(stimulated scattering) 때문에 비선형이다.
  - 보존적이고 양성인 군 간 이산화가 필요하다.
- **class-B 기하**
  - `a_b ≠ 0`이면 발산 항 `−2a_b N^b`가 남아, 위상공간 발산이 아닌 수 변화가 생긴다.
  - 새 Liouville 항등식과 기하 원천 원장이 필요하다.
  - class-A non-I 형도 드리프트에 곡률·회전 항이 추가된다.
- **전구면 편광 수송**
  - 유도 개요만 있고 검증하지 않았다.
  - 스크린 기저 회전률이 `±μφ̇`이어서 삼축에서 Q와 U가 섞인다.
  - spin-2 핵이 필요하고, 결맞음 원뿔 제약과 양립하는 이산화가 필요하다.
- **θ-only 지름길이 무효인 이유**
  - φ 평균이 닫히지 않는다(`c3`).
  - Thomson rank 6 중 4개 성분을 버린다.
  - 틸트 방향은 일반적으로 주축이 아니다.
  - 편광 회전은 `φ̇ = 0`일 때만 사라진다.
- **이 노드 범위 밖**: Rust 어댑터, RODAS5P 통합, 단계 해법(군당 2D 주기 스텐실 + 6×6 커패시턴스), VG-1 무할당 측정, 성능

## 개발 중 공개 사항

최종 실행 전에 테스트 픽스처 결함 5건을 고쳤다. 허용오차와 오라클 공식은 하나도 바꾸지 않았다.

- `b3`: 유한차분 픽스처에서 흡수율이 음수가 되었다. 오라클이 이를 정상적으로 거부했고, 픽스처 값을 바꿨다.
- `h5`: 이진수로 정확하지 않은 값을 썼다. 정확히 표현되는 값으로 바꿨다.
- `h6`: 허용오차가 1 ulp보다 작았다. 리미터를 반올림 인지형으로 고치고 단언을 정확히 `≥ 0`으로 강화했다.
- `g3`: 구적 차수가 부족했다. 6점과 5점을 각각 10점과 8점(16셀)으로 올렸다.
