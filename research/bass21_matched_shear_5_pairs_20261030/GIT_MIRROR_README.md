# BASS21 matched saved histories: scoped five-pair physics, not production

This Git branch is a **text mirror** of the source-owned, physically generated matched-shear optical-depth decomposition. It is based on immutable VigilODE PR121 HEAD `53933d370c4bcc020dc03a6fce276f7b1fa2950b` and the original frozen BASS source owner commit `cc602e209e7deeb511ab2a1f3b2268d55206c43c`.

Five paired sets of true BASS 193-epoch histories (10 original `.npz`) were fully restored and byte-hash checked against their same-owner `RESULT.json` records; original `AtomicNetwork.evaluate` called 1,930 times. All five contrasts have `geometry` ~92% and `ionization` ~8% within a fixed exact finite Shapley/midpoint decomposition. This is model-conditional scalar forward optical-depth data, **not** measured CMB τ, causal identification, or a physical REI/BASS owner alignment.

**Authoritative raw binary/full source**: [verified 5-pair ZIP, Google Drive](https://drive.google.com/file/d/1fSH625iATkLfPJy7Se_y2G0BRKVM4MUj/view), SHA-256 `93a305a9599cd7a87d5f574fc8105ea9096af24fc5763287ff1d4fbc190f5561`, size 5,922,554 bytes. Whole-archive remote readback size/hash and ZIP CRC verified. `bash VERIFY.sh` runs only after full ZIP extraction; this Git mirror excludes native original `.npz` source and nested parent ZIP by design. ChatGPT Library has the same complete ZIP under `/BASS`.

Fresh clean extraction: 15/15 Python tests, EXIT=0, 61/61 manifest SHA, 14/14 recomputed output bytes, no new IVP. Independent R6/C9 NOT_RUN; prior research-ledger CI FAIL retained. Original VigilODE DAG7/23, REI central28/88 (working33/88), successor0/12 admitted, unchanged; new auxiliary model-data node 1/1 scoped, review HOLD. Preserve all original Q-ledger contracts, PR117 RHS/value-only priority, original failures/budgets, no merge/force push/tolerance weakening/production.

Stage JSON snapshots are published retrospectively from actual completed work; sequential Git commit ACKs are **not** evidence of contemporaneous per-stage backup at calculation time.
