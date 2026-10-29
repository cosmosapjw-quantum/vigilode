# Production frontier research — 2026-10-23

**Broad production: HOLD.** This package includes implemented numerical improvements, constructive conditional theorems, negative controls, source-pinned native measurements, and a concrete local development plan. NCP was not used.

- [한국어 연구·감사 보고서](REPORT_KO.md) — motivation, methods, proofs, figures, limitations.
- [수학적 상세 유도](analysis/SYNTHESIS_KO.md) and [independent review](analysis/ADVERSARIAL_AUDIT.md).
- [Decisions](DECISIONS.json), [theorem contracts](THEOREMS.json), [research tree](RESEARCH_TREE.json), [next development steps](NEXT_STEPS.json), [loop record](RESEARCH_RECORD.json).
- [Results](SUMMARY.json), [broader local benchmark plan](BENCHMARK_PLAN.json), [source equivalence](PUBLISH_MAP.json).
- [Reproduction](REPRODUCE.md), [copyable local handoff](HANDOFF_PROMPT_KO.md), [references](REFERENCES.md).

`results/*/RAW.tar.gz` preserves all raw switching/causal JSON and stderr streams with SHA256 inventories. The read-only analysis supports these archives directly. The main manifest covers all package files except itself. Run `python3 research/production_frontier_20261023/src/verify_package.py` from the repository root.

The new public affine/polynomial/pilot interfaces remain explicitly scoped. The PDE kernels are standalone research prototypes. Timing cohorts use separate sources/builds; external VODE/DVODE, diffsol and Julia comparisons for these changes are **NOT_RUN**.
