# Homotopy 실제 수치실험 브랜치 handoff

## 출발 조건

새 브랜치는 이 문서를 포함하는 검증 완료 commit에서 분기한다. 동결 기준선은 `rust-pre-homotopy-v0.2.0-alpha1`; 새 reference API는 `rodas5p-integrators::homotopy`다.

권장 branch 이름:

```text
homotopy-numerical-experiments-v0.5
```

## 첫 과학 질문

다음 partial-coupling family

\[
H_\theta(K,\lambda)
=(D-\theta C)K-g
-\lambda\left[(1-\theta)CK+hN(K)\right]
\]

에서 low-depth finite action `q=0,1,2`와 소수의 batched path rounds가, equal-work SABR/Picard보다 원래 RODAS output defect를 더 빠르게 줄이는 regime이 존재하는가?

## 첫 구현 범위

1. 기존 `StructuredBlockSystem::nonlinear_rhs`와 동일한 frozen-linear remainder 정의를 별도 read-only adapter로 노출한다.
2. 한 path point에서 `H`, original target residual, `E_out`, function/JVP/common-`W` work를 동시에 기록한다.
3. 고정된 `theta`, `lambda` schedule에 대한 reference Euler/secant path step만 구현한다.
4. `q=0,1,2,7`를 모두 유지하되 `q=7`은 correctness oracle, `q<=2`만 fast-path 후보로 취급한다.
5. 모든 실패는 같은 timestep의 protected sequential RODAS5P로 transactional fallback한다.
6. Anderson, Chebyshev/Padé, adaptive schedule은 첫 kill screen 이후에만 추가한다.

## 최소 문제군

### A. affine exact-oracle

- scalar nearest-neighbor chain;
- noncommuting `M,J` two-vector system;
- determinant-one nonnormal lower shift;
- fixed and abruptly changing `W`.

### B. nonlinear ODE

- Prothero–Robinson with independent stiffness and nonlinear-strength controls;
- manufactured vector problem with noncommuting mass matrix;
- semilinear diffusion–reaction;
- advection–diffusion–reaction for nonnormality.

### C. later gate

- regular index-1 DAE only after ODE fifth-order and stiff-decay gates pass.

## 공정 대조군

동일 Rust executable/backend에서:

```text
protected sequential RODAS5P
SABR5P fixed-point/block correction
partial-coupling homotopy q=0,1,2
partial-coupling homotopy q=7 oracle
homotopy fast path + protected sequential fallback
```

## 필수 장부

\[
N_f,\quad N_{Jv},\quad N_{W^{-1}v},\quad
N_{\rm path},\quad N_{\rm correction},\quad N_{\rm fallback},
\]

\[
E_{\rm emb},\quad E_{\rm out},\quad E_{\rm alg},\quad
T_{\rm RHS},\quad T_W,\quad T_{\rm certificate},\quad T_{\rm total}.
\]

Iteration count alone is not a comparison metric.

## Kill gates

즉시 HOLD 또는 재설계:

- false acceptance가 한 번이라도 발생;
- original-output defect가 certificate budget을 초과;
- fixed-step global order 5 회귀;
- stiff limit에서 damping 회귀;
- intended smooth regime의 median path/correction rounds가 4 초과;
- rounds가 8 이상인데 재사용 가능한 work가 없음;
- fallback 포함 기대비용이 sequential보다 큼;
- equal-work SABR와 구별되는 성공 regime이 없음.

## 승격 조건

첫 fast-path PROMOTE에는 최소한 다음이 필요하다.

- false accept 0;
- fifth-order global convergence;
- Prothero–Robinson/stiff-decay regression 없음;
- holdout median wall speedup `>=1.15x`;
- lower quartile에서 지속적인 손실 없음;
- validity range와 fallback frequency 명시.

## 현재 reference artifact

```text
rodas5p homotopy-design-check --output design-check.json
```

Expected schema: `rodas5p-homotopy-design-check-v1`.
