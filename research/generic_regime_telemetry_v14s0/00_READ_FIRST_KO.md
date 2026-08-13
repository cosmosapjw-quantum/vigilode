# G4-S0 — Regime Telemetry + Offline Oracle Map

## 목적

이 checkpoint는 LSODA식 active switching을 구현하지 않는다. 동결된 G3 후보들을 수정하지 않은 채,
동일 external-error target에서 어느 method가 가장 싼지를 offline oracle로 재구성하고,
현재 이미 존재하는 저비용 telemetry가 그 winner를 일반화해 예측할 수 있는지 검사한다.

## 결과

- Strict-JF portfolio에서 12개 target 중 E-K가 6개, protected R-JF가 6개를 이겼다.
- Oracle aggregate cost는 protected R-JF 고정 대비 1.534배 감소 가능성을 보였다.
- Auto portfolio에서는 frozen Radau IIA3 8개, E-K 3개, protected R-JF 1개가 winner였다.
- 그러나 current zero-extra-work feature의 leave-one-problem-out 정확도는 최고 평균 0.75에 그쳤다.
- fresh JVP-quality probe 1회는 protected RODAS RHS work의 중앙값 5.36%, 최대 12.5%였다.
- 따라서 active switching은 아직 승인되지 않는다.

## 판정

- Offline oracle complementarity: PROMOTE
- Read-only regime telemetry: PROMOTE
- Active selector / method switching: HOLD
- 다음 node: accepted-step-local shadow telemetry + sampled nonlinear-defect/JVP-quality probes
