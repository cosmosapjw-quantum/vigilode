# VigilODE REI Bianchi IGM | Original photon-source reduced native solver-client

Scope: **VigilODE-owned original-REI source-conditioned reduced IVP**, based on original 46,080 REI cells, 3 shared neutral photoabsorber moments, 4 original photon groups, accepted original tick160. New physically sourced one-tick evolution and matrix-free JVP + existing native RODAS5P output; NOT an original 26-event REI thermochemical solve, NOT full canonical first interval, NOT Bianchi geometry.

Read `THEORY_KO.md` for equations and nuclear/photon identity. Original source output from REI PR332 Actions artifact SHA256 `83d94b5383ac7c1f36d89486e83bf8b4a56dd1993a25af3ae1f8fed06a695ba9`. Input fixture is the full original group fraction/current/neutral total source snapshot, not guessed synthetic parameters. All original sources immutable.

## Reproduce

With the downloaded original PR332 ZIP at `/mnt/data/REI_PR332_GLOBAL_PHOTON_46080_SOURCE_20261101.zip` (or update only the input path within `build_fixture.py`), run:

```sh
python build_fixture.py
python run_reference.py
python -m unittest discover -s tests -v
python -O -m unittest discover -s tests -v
```

With the matching VigilODE repository checkout and included Rust example path, additionally run:

```sh
cargo run --locked -p rodas5p-integrators --no-default-features --example rei_igm_source_moments > native_physics.json
cargo test --locked -p rodas5p-integrators --no-default-features --example rei_igm_source_moments
```

The `frozen_pr332_source.json` and `reference_trajectory.json` are included in the Git branch for independent Rust offline replay. GitHub also carries 9 physical time rows, source-identified statistics and a bounded source-node witness, but NOT the full original 46k REI owner archive. Full spatial physical CSV and source ZIP are provided in the dedicated full backup ZIP.

## Scientific status

`VIGILODE_DAG=7/23`, `SUCCESSOR=0/12`, `REI_CENTRAL=28/88`, `REI_WORKING=33/88`, `BASS_CE1=9/15`, `CANONICAL_DELTA=0`, `NEW_ORIGINAL_OWNER_IVPS=0`, `PRODUCTION=HOLD`, `NO_MERGE`, `NO_FORCE_PUSH`, `AUTO_NEXT_NODE=false`. All reports refer to independent owner DAGs, not equivalent percentages.
