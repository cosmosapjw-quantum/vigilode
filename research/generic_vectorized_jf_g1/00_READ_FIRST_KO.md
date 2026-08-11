# Generic Vectorized/Jacobian-Free Solver — G1 먼저 읽기

G1은 `q=1,c=2` RODAS stage-coupling fast attempt가 실패할 때 그 endpoint와 이미 지불한
common-W work를 보존한 채 `q=2` correction 하나를 추가하고, 그래도 실패하면 protected
sequential matrix-free RODAS5P로 되돌아가는 transactional engine을 구현·검증한 단계다.

결론은 명확하다.

- 구조·실패 의미론·work accounting은 승격한다.
- generic acceleration method로서의 q1→q2는 HOLD한다.
- accepted-step 수는 protected RODAS5P와 같았지만, adaptive corpus에서 q1 accept는 0회,
  fallback은 약 50.25%, median/P95 critical depth는 16, same-error speedup은 0.226(t1)과
  0.123(t4)에 불과했다.
- 다음 core node는 BDF/Radau 튜닝이 아니라 parallel fifth-order exponential
  Rosenbrock/EPIRK sibling이다.

Authoritative evidence는 `raw/`, `results/G1_ANALYSIS.json`, `plots/`, `reports/`에 있다.
`attempts/`의 archive는 final corpus를 잠그기 전의 superseded pre-expansion run이다.
