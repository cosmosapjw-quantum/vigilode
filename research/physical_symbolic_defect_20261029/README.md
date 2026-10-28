# VigilODE-SYM-01 research capsule (2026-10-29)

Reviewable Draft PR: https://github.com/cosmosapjw-quantum/vigilode/pull/100
Base: PR98 source bf308589e21b524d0ddc6e571b260f5e94a853f0.

Use `THEORY_KO.md` for mathematical contract, `symbolic_oracle.py` for standalone Fraction oracle, `test_symbolic_oracle.py` for 8 test groups (including 918 diagnostic samples), `RESULTS.json` and `CORRECTION_LEDGER.json` for evidence, `STAGE_LEDGER.json` for 7/23 core DAG vs 1/1 scoped auxiliary, `REPORT_KO.md` for synopsis, `NEXT_THREAD_HANDOFF_KO.md` for next-step Codex instructions. The PNG figures are diagnostics only; a rejected mixed-unit plot is preserved.

Run: `cd /mnt/data/vigilode_symbolic_20261029 && python3 -m unittest -v test_symbolic_oracle.py`.

Limits: no native Rust production path, no actual chemical or photon coupled client, no outward floating point guarantee, no whole-path general nonlinear certificate, no production release or speedup. `production=HOLD`.
