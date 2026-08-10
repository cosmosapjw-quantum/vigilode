# PHYS–MATH AUDIT — Path Controller v0.17

## 1. 정의와 단위

- \(\lambda,\theta,\omega\)는 무차원이다.
- \(q\)와 correction count는 정수형 algorithmic depth다.
- \(W=M-h\gamma J_n\)에서 \(hJ_n\)은 mass operator와 같은 차원을 갖는다.
- stage increment \(K_i\)는 원래 RODAS5P convention과 동일하다.

판정: PASS.

## 2. 부호와 correction orientation

Endpoint correction은

\[
K\leftarrow K-Q_qR(K)
\]

형태로 구현된다. 기존 fixed-path equivalence test가 같은 convention을 기계적으로 확인한다.

판정: PASS.

## 3. Known-limit recovery

- affine noncommuting-mass problem에서 q=2/c2 candidate의 sequential-oracle output WRMS는 \(7.85\times10^{-11}\).
- q=7 correctness path는 non-hostile problems에서 oracle 수준으로 수렴.
- fixed schedule과 기존 fixed-path engine은 roundoff 내 동일.

판정: PASS in bounded reference regime.

## 4. 반례

### theta-ramp counterexample

Coupling split을 \(1\to0.5\to0\)으로 바꾸면 target residual이 크게 증가했다. Nonstationary split 변화가 자동으로 path curvature를 줄인다는 가설은 기각된다.

### stiff nonlinear PR counterexample

q=1/c2 candidate는 다른 네 non-hostile family에서 통과하지만 PR에서 실패한다. Mild-problem success를 stiff nonlinear generality로 확장할 수 없다.

### hostile nonnormal counterexample

모든 bounded low-q schedule과 q=7 correctness path가 hostile case의 algebraic budget을 넘었다. Nonnormal amplification과 nonlinear growth에 대한 protected fallback이 필수다.

## 5. 숨은 가정

- \(W\)는 invertible하다.
- frozen \(J_n\)가 한 step의 stiff linear part를 충분히 포착한다.
- current reference engine은 dense LU를 사용한다.
- offline target certificate는 dense target Jacobian을 조립한다.
- one-step success가 global order를 보장하지 않는다.

## 6. 최종 수학 판정

```text
bounded algebraic correctness: PASS
strict low-depth full coverage: FAIL
hostile robustness: FAIL
fifth-order trajectory preservation: NOT TESTED
strict Jacobian-free execution: NOT TESTED IN THIS CYCLE
```
