# Scientific PR Review

현재 branch와 base의 diff를 독립적으로 검토한다.

우선순위:

1. correctness
2. scientific consistency
3. numerical stability
4. regression
5. missing tests
6. error swallowing
7. hidden scope expansion
8. reproducibility

각 finding에 severity, 파일/위치, 영향, 근거/재현, 권장 수정이 있어야 한다. 문제가 없으면 잔여 위험과 test gap을 보고한다.
