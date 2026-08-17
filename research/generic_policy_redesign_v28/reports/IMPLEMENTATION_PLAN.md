# v2.8 staged level-2 safety probe implementation plan

1. TDD RED: add a level1->level2->full resume parity contract that references the missing level-2 API.
2. GREEN: factor U3/D3 and U4/D4 into a retained `Pexprb54s4Level2Prefix`; keep existing one-shot behavior/work identical.
3. TDD RED/GREEN: add a read-only k3 family runner that computes level 2 only and never endpoints.
4. Freeze implementation before canonical solver output.
5. Run 12 family×dimension shards (N=128,512), preserving R-JF as the committed trajectory.
6. Join only to the already-produced historical full-E labels after exact R-JF parity checks.
7. Analyze predeclared level-2 features descriptively; no final threshold selection.
8. PHYS-MATH, PHYS-MATH-CODE, plot/CRAG audit; package durable evidence.
