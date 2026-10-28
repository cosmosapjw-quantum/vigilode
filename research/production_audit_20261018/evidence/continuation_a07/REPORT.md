# A07 Robertson comparison

- Scientific accuracy overlap gate: **False** (empirical reference stability only).
- Speed measurement accuracy gate: **False**; no speedup claim is made and timing authority remains `diagnostic_only`.
- Rows retained: 182; source HEAD `c3036c63da02ca9b04cd39ccf580cd67d00ccb0c`.
- Timing binary SHA-256: `f1af08047816ca91f7ae69ea2e98349c52bbf2ea59d5c74eed5cfafb8cf749ee`.
- Allocator binary SHA-256: `7c487dd5cf0ecde75be860139caf55e88cd0290a617c23be699d0391a6113d64`.

## Accuracy

- Reference spreads (max-grid normalized WRMS): `{"fine_vs_lsoda_max_grid_wrms": 0.0010781557032515283, "fine_vs_previous_radau_max_grid_wrms": 1.0253688066238993e-05}`.
- native: 91/91 complete successful; max distance=0.33753007149457903; interval=[0.3364519157913275, 0.33860822719783057]; gate=True.
- diffsol: 91/91 complete successful; max distance=3.3026992500504733; interval=[3.301621094347222, 3.3037774057537246]; gate=False.

## Timing diagnostics

- single_cold_process: paired units=10, Diffsol/native median ratio=0.9195676453507838, bootstrap interval=[0.6677305256409933, 1.327452564133345].
- warm_abba: paired units=20, Diffsol/native median ratio=1.4194023547437313, bootstrap interval=[1.402230958217182, 1.4501800614107307].

The report is diagnostic. The empirical reference spread is not a rigorous enclosure; no production or universal speed claim follows.


## Allocation audit

- native: status=ok, allocations=443, allocated_bytes=49211.
- diffsol: status=ok, allocations=1039, allocated_bytes=116634.

Campaign schema/count/order validation: **True** ({"allocator": 2, "cold": 20, "warm": 160}).
