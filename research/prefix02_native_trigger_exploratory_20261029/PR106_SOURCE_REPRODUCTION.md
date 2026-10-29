# Fresh PR106 source ZIP reproduction (2026-10-29)

**Scientific scope:** re-run of the original PREFIX02 standalone only, NOT actual PR104 native integration nor physical33 acceptance.

## Exact source identities
- Google Drive original [VIGILODE_PREFIX02_20261029_SOURCE.zip](https://drive.google.com/file/d/1Mh5nuvnex9hsXyDlr5Qk7PontXVPpBPZ/view), 227,923 bytes, **SHA-256 `543e0ba020f6f6f5effe3f1dc4c536068eb8440a60794744da69e03bb10b7e75`** verified on freshly fetched raw archive.
- ZIP has 76 members including `SOURCE_MANIFEST.json` (75 source entries).
- `git hash-object src/lib.rs` `f19639cebac252a44414f864515cc9a5d9b86349`, GitHub PR106 identical.
- `git hash-object src/preconditioned.rs` `5c7aabe290941b7c8a397449c9aed4030de5e687`, GitHub PR106 identical.
- `git hash-object tests/preconditioned_contracts.rs` `3cf46e17453260d3f1e01311191f5646c348f9b8`, GitHub PR106 identical.

## Fresh current-turn executable run
```bash
source /mnt/data/rust_1_94_1_env.sh
cd PR106_READBACK/VIGILODE_PREFIX02_20261029
rustc --version
# rustc 1.94.1 (e408947bf 2026-03-25)
cargo test --offline --all-targets
# exit 0; test binary groups 9+16 = 25 passed, 0 failed
cargo clippy --offline --all-targets -- -D warnings
# exit 0
python evidence/verify_bits.py
# exit 0; 40 source rows, 160 exact residual-component checks,
# 40 exact lognorm, 40 original residual max, 40 inverse bound,
# 40 forward-product, 40 true-stage-error; 360 inequalities PASS
```
All results confined to the original standalone and its saved bit-probe data, **not** a new 2520 IVP benchmark. This readback verifies ZIP bytes and three code blobs; it does not authorize physical33 source/rate norm transport, full native API claims or production promotion.

No newly acknowledged Google Drive/Dropbox *upload* was obtained for the new exploratory artifact. This retrieval is a historical original backup read, not a new backup receipt. Existing PR106 R/C 18/20, core 7/23, REI 28/88, production HOLD remain unchanged; independent-author R6 and independent-code C9 remain NOT_RUN.