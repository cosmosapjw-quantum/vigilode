# Reproduce actual REI F05 46,080-cell original-source native Rust H/He/OTS/thermal rate calculation

Exact original-source frozen accepted tick160, 26-event kernel reference, byte hashes and scope in PREREGISTRATION.md / SCIENCE_AND_HOSTILE_REVIEW.md. The original REI owner MUST remain read-only. This is not a production worker or original full [160,161] IVP admission.

## Rust actual source execution

    ROOT=/home/cosmosapjw/worktrees/vigilode-rei-f05-native-nonphoto-20261102
    OWNER=/home/cosmosapjw/vigilode-rei-owner-f05-readonly-20261101
    OUT=/home/cosmosapjw/archives/vigilode_rei_f05_native_nonphoto_20261102
    cd "$ROOT"
    git rev-parse HEAD
    git -C "$OWNER" rev-parse HEAD

    # Source-owner Python is a read-only oracle/fixture exporter only.
    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 python3 exploratory/rei_igm_native_nonphoto_20261102/export_original_f05_event_oracle.py \
      --owner "$OWNER" \
      --parent exploratory/rei_igm_native_photo_thermal_20261102/ORIGINAL_REI_F05_T160_PARENT5_THERMAL.bin.gz \
      --photon exploratory/rei_igm_native_rank3_20261101/ORIGINAL_REI_F05_T160_PHOTON_SOURCE.bin.gz \
      --out exploratory/rei_igm_native_nonphoto_20261102/ORIGINAL_F05_T160_NONPHOTO_EVENT_ORACLE.bin.gz \
      --metadata exploratory/rei_igm_native_nonphoto_20261102/ORIGINAL_NONPHOTO_ORACLE.json

    gzip -dc exploratory/rei_igm_native_nonphoto_20261102/ORIGINAL_F05_T160_NONPHOTO_EVENT_ORACLE.bin.gz > "$OUT/original_nonphoto_event_oracle.raw"
    gzip -dc exploratory/rei_igm_native_photo_thermal_20261102/ORIGINAL_REI_F05_T160_PARENT5_THERMAL.bin.gz > "$OUT/accepted_parent5.raw"

    export CARGO_TARGET_DIR=/home/cosmosapjw/vigilode/target CARGO_BUILD_JOBS=4
    cargo test --offline --locked -p rodas5p-integrators --example rei_original46k_native_nonphoto
    cargo test --offline --locked -p rodas5p-integrators --example rei_original46k_native_photo_thermal
    cargo test --offline --locked -p rodas5p-core --test low_rank_bordered_contracts
    cargo build --offline --locked --release -p rodas5p-integrators --example rei_original46k_native_nonphoto

    gzip -dc exploratory/rei_igm_native_rank3_20261101/ORIGINAL_REI_F05_T160_PHOTON_SOURCE.bin.gz | \
       taskset -c 4 /home/cosmosapjw/vigilode/target/release/examples/rei_original46k_native_nonphoto \
       "$OUT/accepted_parent5.raw" "$OUT/original_nonphoto_event_oracle.raw" \
       "$OUT/replayed_native_nonphoto.raw" > "$OUT/replayed_native_nonphoto.json"

    sha256sum "$OUT/replayed_native_nonphoto.raw"

Expected original native derived raw SHA256: 79344b098d441710c61f099c3daeec8acd6eebb2417d31ddbe2e30455252f8ca. Event oracle RAW SHA256: 28ea21ea9e5b4210b3e8971313af6aa6a8c4a07fad10c984bfd29424967ebf0b. Existing parent and photon gzip SHA identities are locked in fixture manifests. The source interval oracle is 46,080x25 flux boxes and 46,080x7 energy boxes. The new Rust data is 46,080x40 point source rows. Exact parent clock t=779495799866.9248 seconds and step=4871848749.16828 seconds, branch midpoint v and f=0.55.

## Original nonauthoritative preview independent audit

Original source-owner preview source must already exist at
/home/cosmosapjw/archives/rei_f05_tick161_owner_candidate_20261101/primary/ORIGINAL_REI_F05_TICK161_HHE_T_ELECTRON_PREVIEW_46080.csv.gz
with original SHA256 713b46a0da8931cc6f5a355912cdd2efa790945c2539ad049c086fd995f3441d and sibling PREVIEW_RESULT.json. The external preview is NOT a new accepted original state.

    python3 exploratory/rei_igm_native_nonphoto_20261102/audit_rust_original_event.py \
      --native-json "$OUT/replayed_native_nonphoto.json" \
      --native-raw "$OUT/replayed_native_nonphoto.raw" \
      --event-meta exploratory/rei_igm_native_nonphoto_20261102/ORIGINAL_NONPHOTO_ORACLE.json \
      --event-gzip exploratory/rei_igm_native_nonphoto_20261102/ORIGINAL_F05_T160_NONPHOTO_EVENT_ORACLE.bin.gz \
      --parent-meta exploratory/rei_igm_native_photo_thermal_20261102/ORIGINAL_REI_F05_PARENT5_MANIFEST.json \
      --parent-gzip exploratory/rei_igm_native_photo_thermal_20261102/ORIGINAL_REI_F05_T160_PARENT5_THERMAL.bin.gz \
      --preview-meta /home/cosmosapjw/archives/rei_f05_tick161_owner_candidate_20261101/primary/PREVIEW_RESULT.json \
      --preview-gzip /home/cosmosapjw/archives/rei_f05_tick161_owner_candidate_20261101/primary/ORIGINAL_REI_F05_TICK161_HHE_T_ELECTRON_PREVIEW_46080.csv.gz \
      --out "$OUT/redo_independent_audit.json"

    python3 -m unittest discover -s exploratory/rei_igm_native_nonphoto_20261102/tests -v
    python3 -O -m unittest discover -s exploratory/rei_igm_native_nonphoto_20261102/tests -v

Expected source-scoped verification: all 25 photon+nonphoto transition channels and all 7 energy/OTS/thermal original interval checks 46080/46080, original H/He/OTS energy conservation, first-order frozen native electron+logT estimates contained in source-owner tick161 NONAUTHORITATIVE preview intervals for all cells. Do NOT assert full original IVP convergence, 14 final ledgers, owner V1/V2 admission or generic native speedup.
