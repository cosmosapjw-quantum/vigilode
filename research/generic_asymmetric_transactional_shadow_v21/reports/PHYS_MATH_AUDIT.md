# PHYS–MATH AUDIT — G4-S5B1

## PASS

1. Regret, speedup, overhead는 무차원이다.
2. Hard corpus에서 method economics는 raw wall이 아니라 동일 physical interval당 예상 비용으로 비교했다.
3. E-K admissibility 실패는 cost winner로 분류하지 않았다.
4. Flow defect는 explicit-time effect를 포함하는 G4-S1 time-aware quantity를 사용한다.
5. JVP quality가 threshold를 넘으면 E-K entry를 fail-closed한다.
6. Failed E-K full work와 discarded prefixes를 삭제하지 않는다.
7. Active switching이 없으므로 switching-induced global-order claim을 하지 않는다.

## P0 fixed

Raw current-step wall은 서로 다른 next-step proposal을 가진 methods의 economics를 비교할 authority가 아니다. 258개 hard rows 중 24개 winner가 unit-time correction 뒤 바뀌었다.

## P1 limitation

- Hard-corpus prefix 비용은 G4-S4 P95 fraction을 transfer한 conservative model이다.
- Atlas에는 G4-S1 flow/JVP feature가 없으므로 E admission은 local-error authority에 의존한다.
- 이번 result는 shadow replay이며 actual controller-state transfer를 검증하지 않는다.

## P2 limitation

- Calibration/holdout family 수가 작다.
- Endpoint/local-error authority가 event/phase/trajectory-wide observable을 완전히 대표하지 않는다.
- N=2048은 sealed 상태다.

## Verdict

```text
metric contract       PASS
read-only replay      PASS
active switching      NOT PROVED
```
