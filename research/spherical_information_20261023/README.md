# Spherical information geometry research package

2026-10-23. Theory and lightweight numerical verification based on the two
user-supplied Angular Calculus v13 drafts. No native solver edits or NCP execution.

- [한국어 연구 보고서](RESEARCH_REPORT_KO.md)
- [판정과 근거 상태](DECISIONS.json)
- [구체적인 다음 개발 단계](NEXT_STEPS.json)
- [재현 명령](REPRODUCE.md)
- [로컬 Codex 인계](HANDOFF_KO.md)
- `analysis/`: detailed derivations and independent review
- `src/`: standalone Python reference implementations and tests
- `results/`: measured results, receipts, preserved intermediate probes
- `figures/`: three PNG/PDF figure pairs
- `evidence/`: input identities and exact Wolfram calculations
- `reference/`: frozen existing FVM formula oracle with provenance
- Original draft source files are identified in `evidence/INPUTS.json`; retrieve them from the user-supplied archives. This repository import retains derived research, code and evidence.

Scope: finite-grid entropy compression, entropy moment inversion, fixed-shell
collision action, repeated positive-map remap budget, and spectral tools.
These are reusable research components, not an end-to-end production certification.

Dependencies used: Python 3.12.14, NumPy 2.3.5, SciPy 1.17.0, Matplotlib.
Individual execution receipts retain actual versions and command lines.
