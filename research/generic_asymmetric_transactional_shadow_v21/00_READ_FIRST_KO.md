# G4-S5B1 — Asymmetric Transactional Shadow Closeout

이 node는 G4-S1 hard corpus와 G4-S5B0 transition atlas를 결합해 다음 세 정책을 **read-only shadow replay**로 비교한다.

1. protected R-JF static control
2. R-JF-default + event-triggered reusable E-K entry
3. E-K-first + immediate protected R-JF fallback

추가로 `safe-e-diagnostic`을 실행해, flow/JVP/admissibility safety만으로 어느 정도까지 갈 수 있는지 분리했다.

## 핵심 결과

- 두 contract policy 모두 unsafe recommendation은 0이다.
- `R-JF-default`는 trial overhead는 낮지만, 최소 residence 6 때문에 3-step advection–diffusion holdout을 놓쳐 holdout median/P95 regret가 `1.316 / 5.458`이다.
- `E-K-first`는 holdout median regret `1.072`와 aggregate speedup `2.538x`를 얻지만, P95 regret `1.570`과 P95 trial overhead `7.67%`로 gate를 넘는다.
- `safe-e-diagnostic`은 holdout에서 median/P95 regret `1.000 / 1.137`이지만, calibration quadratic에서 E-K가 안전하면서도 약 2.4–2.6배 비경제적이어서 calibration P95 regret가 `2.506`이다.
- 따라서 남은 blocker는 broad safety detection이 아니라 **E-K-admissible 영역의 startup economic admission**이다.

## P0 수정

초기 분석에서 raw current-step wall을 method economics로 비교했으나, E-K와 R-JF는 서로 다른 next-step size를 제안한다. 따라서 hard corpus에서는

\[
\Gamma_m=\frac{T_m}{\widehat h_m}
\]

을 사용하고 같은 RODAS accepted interval에 대한 비용으로 환산했다. 이 수정으로 258개 hard accepted-step row 중 24개(9.30%)의 winner label이 바뀌었다.

## 판정

```text
G4_S5B1_R_DEFAULT       HOLD
G4_S5B1_E_FIRST         HOLD
G4_S5B1_SAFE_E          DIAGNOSTIC SURVIVOR
ACTIVE_SWITCHING        HOLD
```

다음 node는 `G4-S5B2 Startup Economic Admission Gate`다. 첫 accepted state에서만 실제 reusable Arnoldi/GMRES prefix를 사용해 quadratic-like `E-admissible/R-cheaper` 상태와 complex/advection-like `E-cheaper` 상태를 구별하고, 이후 routine review는 하지 않는다.
