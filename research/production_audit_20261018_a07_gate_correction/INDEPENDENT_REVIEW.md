# Independent review: A07 threshold semantics

Reviewer assignment: read-only, requested `gpt-6.1-sol/xhigh`; observed runtime identity was not independently verified. Review based on the retained source/evidence and exact validator code.

Finding: MAJOR reporting/schema error in the parent follow-up report. The prior text treated 0.05 as the solver-arm WRMS gate. In `tools/benchmarks/diffsol/a07/src/validate_report.py`, the 0.05 constant gates empirical reference spreads; each arm separately passes only when `max_distance + uncertainty <= 1.0` and all rows succeed; combined arm comparison also requires empirical interval overlap.

Disposition: corrected in this node with no campaign rerun, no tolerance change, no row/reference/timing modification. Retained values show reference spread gate PASS; native arm accuracy PASS; Diffsol arm accuracy FAIL; empirical arm intervals disjoint; combined accuracy and matched-speed HOLD. The reference remains empirical rather than rigorous. This review admits only the reporting correction.
