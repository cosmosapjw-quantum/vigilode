# Reproduce original REI F05 native Rust H/He photo/thermal source stage

All commands are scoped to previously locked source owners, without modifying original owner worker. The original source gzip is already committed within the VigilODE #183 ancestor.

```bash
ROOT=/home/cosmosapjw/worktrees/vigilode-rei-f05-native-photo-thermal-20261102
REI=/home/cosmosapjw/worktrees/rei-f05-rust-native-restoration-20261101
OWNER=/home/cosmosapjw/vigilode-rei-owner-f05-readonly-20261101
ARCH=/home/cosmosapjw/archives/vigilode_rei_f05_native_photo_thermal_20261102
P="$ROOT/exploratory/rei_igm_native_photo_thermal_20261102"
cd "$ROOT"
git rev-parse HEAD
git -C "$OWNER" rev-parse HEAD
git -C "$REI" rev-parse HEAD

# The source exporter is READ-ONLY. It does not evolve the ODE.
OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 python3 "$P/export_accepted_f05_hhe_thermal.py" \
  --owner "$OWNER" --old "$ROOT/exploratory/rei_igm_native_rank3_20261101/ORIGINAL_REI_F05_T160_PHOTON_SOURCE.bin.gz" \
  --out "$P/ORIGINAL_REI_F05_T160_PARENT5_THERMAL.bin.gz" \
  --manifest "$P/ORIGINAL_REI_F05_PARENT5_MANIFEST.json"
gzip -dc "$P/ORIGINAL_REI_F05_T160_PARENT5_THERMAL.bin.gz" > "$ARCH/parent5_accepted_f05.raw"

export CARGO_TARGET_DIR=/home/cosmosapjw/vigilode/target CARGO_BUILD_JOBS=4
cargo test --offline --locked -p rodas5p-integrators --example rei_original46k_native_rank3
cargo test --offline --locked -p rodas5p-integrators --example rei_original46k_native_photo_thermal
cargo test --offline --locked -p rodas5p-core --test low_rank_bordered_contracts
cargo build --offline --locked --release -p rodas5p-integrators --example rei_original46k_native_photo_thermal

gzip -dc "$ROOT/exploratory/rei_igm_native_rank3_20261101/ORIGINAL_REI_F05_T160_PHOTON_SOURCE.bin.gz" | \
  /home/cosmosapjw/vigilode/target/release/examples/rei_original46k_native_photo_thermal \
    "$ARCH/parent5_accepted_f05.raw" "$ARCH/stage_replay.raw" \
  > "$ARCH/stage_replay.json"
sha256sum "$ARCH/stage_replay.raw"
gzip -n -c "$ARCH/stage_replay.raw" > "$ARCH/stage_replay.bin.gz"

# Rust to Rust EOS link, without vendoring REI's original 117 historical files.
cd "$REI/rust/rei_microphysics"
CARGO_TARGET_DIR=/home/cosmosapjw/archives/rei_f05_rust_native_restoration_20261101/target cargo build --offline --locked --lib
LIB=$(find /home/cosmosapjw/archives/rei_f05_rust_native_restoration_20261101/target/debug/deps \
    -maxdepth 1 -name "librei_microphysics-*.rlib" | head -1)
rustc --edition=2021 -C opt-level=2 \
   -L dependency=/home/cosmosapjw/archives/rei_f05_rust_native_restoration_20261101/target/debug/deps \
   --extern rei_microphysics="$LIB" "$P/recovered_rei_native_eos_probe.rs" \
   -o "$ARCH/rei_rust_eos_probe_replay"
"$ARCH/rei_rust_eos_probe_replay" "$ARCH/parent5_accepted_f05.raw" "$ARCH/stage_replay.raw"
```

Independent physical preview comparison needs the original SHA-locked external owner file `/home/cosmosapjw/archives/rei_f05_tick161_owner_candidate_20261101/primary/ORIGINAL_REI_F05_TICK161_HHE_T_ELECTRON_PREVIEW_46080.csv.gz`, SHA256 `713b46a0da8931cc6f5a355912cdd2efa790945c2539ad049c086fd995f3441d`, and PREVIEW_RESULT.json beside it. This preview remains NONAUTHORITATIVE. Run `python3 audit_native_photo_thermal.py --help` for required exact sources. Offline automated stdlib tests: `python3 -m unittest discover -s "$P/tests" -v` and `python3 -O -m unittest discover -s "$P/tests" -v`.

Native raw stage format `REIF05D1` followed by u64(46080), then 10 f64 LE values per site: dHI, dHII, dHeI, dHeII, dHeIII, dNe, primary photoheat erg/s, photo-only dlnT/dt, original accepted gas U erg, photo binding gain erg/s. Compare raw SHA256 `c936d56b9a9c7805a365245c88dbd543bbad75c6d9ef33e49d7cf62a373c7556`; original accepted source parent raw SHA `eeb4aa50c56ed5437c26b1c7087d0f68884db3db331d4ef435657b0bceb22064`. Do not mix this source-only stage with the complete original 26-event coupled time map, full V1 error gate, or an authoritative tick161.
