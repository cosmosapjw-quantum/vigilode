# G4-S5B2 Startup-Profile Architecture Preflight

이번 node는 G4-S5B2 계약의 실제 Arnoldi/GMRES bake-off를 구현하기 **전에**,
그 계약의 one-time startup-profile architecture 자체가 승격 gate를 통과할 수 있는지
기존 immutable G4-S1/G4-S5B0 campaign으로 상한 검사를 수행한 read-only P0 preflight다.

## 핵심 결과

- first-step perfect cost oracle도 calibration/holdout median regret가
  `1.407` / `1.386`로 실패했다.
- 더 강한 **complete-future trajectory profile oracle**도 dual-prefix 조건에서
  holdout median/P95 regret `1.322` / `2.895`로 실패했다.
- one-sided startup profile로 prefix 비용을 줄여도 holdout P95 regret는
  `2.510`로 실패했다.
- safety-triggered E re-entry를 허용한 optimistic upper bound는 regret gate를 통과하지만,
  short R-profile trajectory의 startup prefix 때문에 holdout P95 overhead가
  `5.18%`로 5% gate를 넘었다.

## 판정

```text
one-time startup profile architecture   REJECT
actual dual-prefix bake-off             NOT IMPLEMENTED
reusable prefix kernels                 PRESERVE
active switching                        HOLD
```

다음 node는 startup-only profile이 아니라 **short-horizon admission + event-triggered E re-entry**를
read-only로 검증해야 한다.
