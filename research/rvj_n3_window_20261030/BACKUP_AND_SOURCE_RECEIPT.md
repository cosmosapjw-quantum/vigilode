# N3 complete source archive and two-provider backup receipt (2026-10-30 KST)

## Verified original ZIP
- Filename: `VIGILODE_RVJ_N3_RUST199_ADAPTIVE_WINDOWS_20261030_FULL.zip`
- Exact SHA-256: `34397ba242db4a0da1f4b2e69e79535e73902655cced21810f2722105b116784`
- Exact archive size: **66,036 bytes**, 36 ZIP entries, `ZIP_CRC_PASS`, manifest-listed payload SHA-256 **35/35 PASS**.
- Google Drive binary: https://drive.google.com/file/d/18AWcusVY8yz6Eggrdzzswt1lowIjA0YA/view ; **provider ACK received**, fresh materialized **raw-byte readback matches local source ZIP** using `cmp` and SHA-256. Readback ZIP CRC and manifest 35/35 repeat PASS.
- ChatGPT Library: `/VIGILODE_RESEARCH/VIGILODE_RVJ_N3_RUST199_ADAPTIVE_WINDOWS_20261030_FULL.zip`; upload ACK. Separate handoff: `/VIGILODE_RESEARCH/VIGILODE_RVJ_N3_HANDOFF_20261030.md`. A separate Library restore has **not** been run; do not mistake upload ACK for readback.
- Google Drive handoff ID: `1B7YCyYgViVNX7NPG3Bh7dsYoniOxWdtQ`.
- Dropbox binary backup: **NOT_RUN**.

## Exact executable source identity within ZIP
- `rust/src/lib.rs` SHA-256 `8fa0fc2a229050fbd986c67dee8ea9b6edebeadf3ca626a81abb76713559333c`, 28,627 bytes.
- `rust/src/main.rs` SHA-256 `7f07d06cf24e079fd395f88e7f993846b43f708e0fa4f063fe19fe1988714490`.
- `rust/src/bin/bench.rs` SHA-256 `08d47b4f74c49fdf10c323572ebdc1fa250921059db2c0b5c5b3a8e1a45c254a`.
- `STAGE_PROGRESS.json` SHA-256 `078d538e4f565c2e23ce667f20ff5cabacf68f48b663c590df98edd60ef350b2`.
- `REPLAY.sh` SHA-256 `98ac1b27d06137b5b9f4c6e4720bbc513abcfc87e5117b4bdb3715ace1f3f245`.
- SHA-256 of every source, test/oracle, original numerical CSV and historical RED log is in `MANIFEST.json` inside the ZIP.
- Note: GitHub branch is a **text/checkpoint publication**, not a byte-complete native source tree. To reproduce/alter source, download the ZIP and verify with `python VERIFY_ARCHIVE.py <archive.zip>`; do not assume this GitHub tree contains all Rust sources.

## Scientific stage boundaries
- RVJ loop12 successor N3 **3/8 scoped**; official VigilODE **7/23**, REI **28/88**, formal successor admitted **0/12**.
- Standalone Rust 1.99.1: **20/20 PASS**, strict Clippy/fmt PASS; independent Python math/reference checks PASS scoped; older offline-source `rodas5p-core` **27/27 PASS**, not PR131.
- **544/716** f64 bound entries under exact-rational ideal bound, so outward-rounded arithmetic certification **NOT_RUN**; no original RODAS5P 8-stage bridge or matched achieved-error speed claim.
- C9 real distinct author **0/1 NOT_RUN**, C10 NOT_STARTED, production HOLD, auto-next false, merge ready false.
- This extra receipt is backup/verification metadata only, not a new scientific DAG node.
